use super::types::*;

#[derive(Debug)]
struct Slot {
    version: Version,
    payload: Payload,
}

#[derive(Debug, PartialEq, Eq)]
enum Payload {
    Active {
        item_type: Type,
        value_index: ObjectIndex,
    },
    Free {
        next_free: Option<SlotIndex>,
    },
    Dead,
}

pub struct Storage {
    slots: Vec<Slot>,
    freelist_head: Option<SlotIndex>,
    num_allocated: usize,
}

impl Storage {
    /// Initializes a new [`SlotStorage`] object.
    pub fn new() -> Self {
        Self {
            slots: vec![Slot {
                version: Version(1),
                payload: Payload::Free { next_free: None },
            }],
            freelist_head: Some(SlotIndex(0)),
            num_allocated: 0,
        }
    }

    /// Retrieves the [`ItemType`] and [`Index`] associated with `id`. If the
    /// `id` is invalid or the resource it pointed to was destroyed, this
    /// function will return `None`.
    pub fn get(&self, id: Id) -> Option<(Type, ObjectIndex)> {
        self.slots.get(id.index.0 as usize).map_or(None, |slot| {
            if let Payload::Active {
                item_type,
                value_index,
            } = &slot.payload
            {
                if slot.version == id.version {
                    return Some((*item_type, *value_index));
                }
            }
            None
        })
    }

    pub fn is_valid(&self, id: Id) -> bool {
        self.slots.len() > id.index.0 as usize
            && self.slots[id.index.0 as usize].version == id.version
    }

    pub fn num_active(&self) -> usize {
        self.num_allocated
    }

    /// Allocates a slot to store `item_type` and `value_index`, returning an
    /// [`ItemId`] on success. The `item_type` and `value_index` cannot be
    /// modified except to be freed.
    pub fn alloc(&mut self, item_type: Type, value_index: ObjectIndex) -> Option<Id> {
        if let Some(index) = self.freelist_head {
            let slot = unsafe { self.slots.get_unchecked_mut(index.0 as usize) };
            match slot.payload {
                Payload::Free { next_free } => {
                    self.freelist_head = next_free;
                    slot.payload = Payload::Active {
                        item_type,
                        value_index,
                    };
                    self.num_allocated += 1;
                    Some(Id {
                        index,
                        version: slot.version,
                    })
                }
                _ => unreachable!(),
            }
        } else if self.slots.len() < (u16::MAX as usize) {
            let index = self.slots.len() as u16;
            self.slots.push(Slot {
                version: Version(0),
                payload: Payload::Active {
                    item_type,
                    value_index,
                },
            });
            self.num_allocated += 1;
            Some(Id {
                index: SlotIndex(index),
                version: Version(0),
            })
        } else {
            None
        }
    }

    /// Returns a slot to the [`SlotStorage`] identified by `id`.
    pub fn free(&mut self, id: Id) -> (Type, ObjectIndex) {
        if let Some(slot) = self.slots.get_mut(id.index.0 as usize) {
            if id.version != slot.version {
                panic!("Attempted to free slot {:?} twice.", id);
            }

            match slot.payload {
                Payload::Active {
                    item_type,
                    value_index,
                } => {
                    if slot.version.0 < u16::MAX {
                        slot.version = Version(slot.version.0 + 1);
                        slot.payload = Payload::Free {
                            next_free: self.freelist_head,
                        };
                        self.freelist_head = Some(id.index);
                        self.num_allocated -= 1;
                    } else {
                        slot.payload = Payload::Dead;
                    }

                    return (item_type, value_index);
                }
                _ => unreachable!(),
            }
        }

        panic!("Attempted to free unallocated slot {:?}", id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_size() {
        assert_eq!(
            std::mem::size_of::<Id>(),
            4,
            "ItemId is not the expected 4 bytes!"
        );
        assert_eq!(
            std::mem::align_of::<Id>(),
            4,
            "ItemId is not aligned to 4 bytes!"
        );
        assert_eq!(
            std::mem::size_of::<Payload>(),
            6,
            "SlotPayload is not the expected 6 bytes!"
        );
    }

    #[test]
    fn slot_allocator_api() {
        let mut slots = {
            let init = Storage::new();
            assert_eq!(init.slots.len(), 1);
            assert_eq!(init.freelist_head, Some(SlotIndex(0)));
            init
        };
        {
            let slot1 = slots.alloc(Type::Unknown, ObjectIndex(0)).unwrap();
            assert_eq!(slots.is_valid(slot1), true);
            assert_eq!(
                slot1,
                Id {
                    index: SlotIndex(0),
                    version: Version(1)
                }
            );
            assert_eq!(slots.get(slot1), Some((Type::Unknown, ObjectIndex(0))));
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.freelist_head, None);

            slots.free(slot1);
            assert_eq!(slots.is_valid(slot1), false);
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.slots[0].payload, Payload::Free { next_free: None });
            assert_eq!(slots.freelist_head, Some(SlotIndex(0)));

            let slot2 = slots.alloc(Type::Unknown, ObjectIndex(100)).unwrap();
            assert_eq!(slots.is_valid(slot2), true);
            assert_eq!(slots.get(slot1), None);
            assert_eq!(slots.get(slot2), Some((Type::Unknown, ObjectIndex(100))));
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.freelist_head, None);

            slots.free(slot2);
        }
    }

    #[test]
    fn slot_allocator_dead_slot() {
        let mut slots = Storage::new();

        // Set up slots[0] to be near 2 allocations away from retirement.
        slots.slots[0].version = Version(u16::MAX - 1);

        let slot1 = slots.alloc(Type::Unknown, ObjectIndex(1)).unwrap();
        assert_eq!(slots.slots[0].version, Version(u16::MAX - 1));
        slots.free(slot1);
        assert_eq!(slots.slots[0].version, Version(u16::MAX));
        assert!(slots.freelist_head.is_some());

        // Test that we can allocate a saturated node.
        let slot2 = slots.alloc(Type::Unknown, ObjectIndex(3)).unwrap();
        assert_eq!(slots.slots[0].version, Version(u16::MAX));
        slots.free(slot2);
        assert_eq!(slots.slots[0].version, Version(u16::MAX)); // No change expected here

        // Test that the slot was correctly retired.
        assert!(slots.freelist_head.is_none());
        assert_eq!(slots.slots[0].payload, Payload::Dead);
    }
}
