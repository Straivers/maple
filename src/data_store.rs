#[repr(align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemId {
    index: Index,
    version: Version,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ItemType {
    Unknown = 0,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Index(u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Version(u16);

#[derive(Debug)]
struct Slot {
    version: Version,
    payload: SlotPayload,
}

#[derive(Debug, PartialEq, Eq)]
enum SlotPayload {
    Active {
        item_type: ItemType,
        value_index: Index,
    },
    Free {
        next_free: Option<Index>,
    },
    Dead
}

struct SlotStorage {
    slots: Vec<Slot>,
    freelist_head: Option<Index>,
}

impl SlotStorage {
    fn new() -> Self {
        Self {
            slots: vec![Slot {
                version: Version(1),
                payload: SlotPayload::Free { next_free: None },
            }],
            freelist_head: Some(Index(0)),
        }
    }

    fn get(&self, id: ItemId) -> Option<(ItemType, Index)> {
        self.slots.get(id.index.0 as usize).map_or(None, |slot| {
            if let SlotPayload::Active { item_type, value_index } = &slot.payload {
                if slot.version == id.version {
                    return Some((*item_type, *value_index));
                }
            }
            None
        })
    }

    fn alloc(&mut self, item_type: ItemType, value_index: Index) -> Option<ItemId> {
        if let Some(index) = self.freelist_head {
            let slot = unsafe { self.slots.get_unchecked_mut(index.0 as usize) };
            match slot.payload {
                SlotPayload::Free { next_free } => {
                    self.freelist_head = next_free;
                    slot.payload = SlotPayload::Active {
                        item_type,
                        value_index,
                    };
                    Some(ItemId{ index, version: slot.version})
                }
                _ => unreachable!(),
            }
        } else if self.slots.len() < (u16::MAX as usize) {
            let index = self.slots.len() as u16;
            self.slots.push(Slot {
                version: Version(0),
                payload: SlotPayload::Active {
                    item_type,
                    value_index,
                },
            });
            Some(ItemId{ index: Index(index), version: Version(0)})
        } else {
            None
        }
    }

    fn free(&mut self, id: ItemId) {
        if let Some(slot) = self.slots.get_mut(id.index.0 as usize) {
            if id.version != slot.version {
                return;
            }

            match slot.payload {
                SlotPayload::Active {
                    item_type: _,
                    value_index: _,
                } => {
                    if slot.version.0 < u16::MAX {
                        slot.version = Version(slot.version.0 + 1);
                        slot.payload = SlotPayload::Free {
                            next_free: self.freelist_head,
                        };
                        self.freelist_head = Some(id.index);
                    } else {
                        slot.payload = SlotPayload::Dead;
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_size() {
        assert_eq!(std::mem::size_of::<ItemId>(), 4, "ItemId is not the expected 4 bytes!");
        assert_eq!(std::mem::align_of::<ItemId>(), 4, "ItemId is not aligned to 4 bytes!");
        assert_eq!(std::mem::size_of::<SlotPayload>(), 6, "SlotPayload is not the expected 6 bytes!");
    }

    #[test]
    fn slot_allocator_api() {
        let mut slots = {
            let init = SlotStorage::new();
            assert_eq!(init.slots.len(), 1);
            assert_eq!(init.freelist_head, Some(Index(0)));
            init
        };
        {
            let slot1 = slots.alloc(ItemType::Unknown, Index(0)).unwrap();
            assert_eq!(slot1, ItemId{ index: Index(0), version: Version(1) });
            assert_eq!(slots.get(slot1), Some((ItemType::Unknown, Index(0))));
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.freelist_head, None);

            slots.free(slot1);
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.slots[0].payload, SlotPayload::Free { next_free: None });
            assert_eq!(slots.freelist_head, Some(Index(0)));

            let slot2 = slots.alloc(ItemType::Unknown, Index(100)).unwrap();
            assert_eq!(slots.get(slot1), None);
            assert_eq!(slots.get(slot2), Some((ItemType::Unknown, Index(100))));
            assert_eq!(slots.slots.len(), 1);
            assert_eq!(slots.freelist_head, None);

            slots.free(slot2);
        }
    }

    #[test]
    fn slot_allocator_dead_slot() {
        let mut slots = SlotStorage::new();

        // Set up slots[0] to be near 2 allocations away from retirement.
        slots.slots[0].version = Version(u16::MAX - 1);

        let slot1 = slots.alloc(ItemType::Unknown, Index(1)).unwrap();
        assert_eq!(slots.slots[0].version, Version(u16::MAX - 1));
        slots.free(slot1);
        assert_eq!(slots.slots[0].version, Version(u16::MAX));
        assert!(slots.freelist_head.is_some());

        // Test that we can allocate a saturated node.
        let slot2 = slots.alloc(ItemType::Unknown, Index(3)).unwrap();
        assert_eq!(slots.slots[0].version, Version(u16::MAX));
        slots.free(slot2);
        assert_eq!(slots.slots[0].version, Version(u16::MAX)); // No change expected here

        // Test that the slot was correctly retired.
        assert!(slots.freelist_head.is_none());
        assert_eq!(slots.slots[0].payload, SlotPayload::Dead);
    }
}
