#![allow(dead_code)]

#[repr(align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Id {
    index: Index,
    version: Version,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Version(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Index(pub u16);

struct Slot<T: Copy> {
    version: Version,
    payload: Payload<T>,
}

#[derive(Debug, PartialEq, Eq)]
enum Payload<T: Copy> {
    Active(T),
    Free { next_free: Option<Index> },
    Dead,
}

pub struct Storage<T: Copy> {
    slots: Vec<Slot<T>>,
    freelist_head: Option<Index>,
    num_allocated: usize,
}

impl<T: Copy> Storage<T> {
    /// Initializes a new [`SlotStorage`] object.
    pub fn new() -> Self {
        Self {
            slots: vec![Slot {
                version: Version(1),
                payload: Payload::Free { next_free: None },
            }],
            freelist_head: Some(Index(0)),
            num_allocated: 0,
        }
    }

    /// Retrieves the [`ItemType`] and [`Index`] associated with `id`. If the
    /// `id` is invalid or the resource it pointed to was destroyed, this
    /// function will return `None`.
    pub fn get(&self, id: Id) -> Option<T> {
        self.slots.get(id.index.0 as usize).and_then(|slot| {
            if let Payload::Active(data) = &slot.payload {
                if slot.version == id.version {
                    return Some(*data);
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
    pub fn alloc(&mut self, data: T) -> Option<Id> {
        if let Some(index) = self.freelist_head {
            let slot = unsafe { self.slots.get_unchecked_mut(index.0 as usize) };
            match slot.payload {
                Payload::Free { next_free } => {
                    self.freelist_head = next_free;
                    slot.payload = Payload::Active(data);
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
                payload: Payload::Active(data),
            });
            self.num_allocated += 1;
            Some(Id {
                index: Index(index),
                version: Version(0),
            })
        } else {
            None
        }
    }

    /// Removes the value addressed by `id` and frees the slot for future use.
    pub fn take(&mut self, id: Id) -> Option<T> {
        if let Some(slot) = self.slots.get_mut(id.index.0 as usize) {
            if id.version != slot.version {
                return None;
            }

            match slot.payload {
                Payload::Active(data) => {
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

                    return Some(data);
                }
                _ => return None,
            }
        }

        None
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
    }

    #[test]
    fn slot_allocator_api() {
        let mut slots = {
            let init = Storage::new();
            assert_eq!(init.slots.len(), 1);
            assert_eq!(init.freelist_head, Some(Index(0)));
            init
        };
        {
            let slot1 = slots.alloc(10).unwrap();
            assert_eq!(slots.is_valid(slot1), true);
            assert_eq!(
                slot1,
                Id {
                    index: Index(0),
                    version: Version(1)
                }
            );
            assert_eq!(slots.get(slot1), Some(10));
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.freelist_head, None);

            assert_eq!(slots.take(slot1), Some(10));
            assert_eq!(slots.is_valid(slot1), false);
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.slots[0].payload, Payload::Free { next_free: None });
            assert_eq!(slots.freelist_head, Some(Index(0)));

            let slot2 = slots.alloc(11).unwrap();
            assert_eq!(slots.is_valid(slot2), true);
            assert_eq!(slots.get(slot1), None);
            assert_eq!(slots.get(slot2), Some(11));
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.freelist_head, None);

            slots.take(slot2);
        }
    }

    #[test]
    fn slot_allocator_dead_slot() {
        let mut slots = Storage::new();

        // Set up slots[0] to be near 2 allocations away from retirement.
        slots.slots[0].version = Version(u16::MAX - 1);

        let slot1 = slots.alloc(1).unwrap();
        assert_eq!(slots.slots[0].version, Version(u16::MAX - 1));
        slots.take(slot1);
        assert_eq!(slots.slots[0].version, Version(u16::MAX));
        assert!(slots.freelist_head.is_some());

        // Test that we can allocate a saturated node.
        let slot2 = slots.alloc(2).unwrap();
        assert_eq!(slots.slots[0].version, Version(u16::MAX));
        slots.take(slot2);
        assert_eq!(slots.slots[0].version, Version(u16::MAX)); // No change expected here

        // Test that the slot was correctly retired.
        assert!(slots.freelist_head.is_none());
        assert_eq!(slots.slots[0].payload, Payload::Dead);
    }
}
