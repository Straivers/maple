use std::num::NonZeroU16;

#[derive(PartialEq, Eq, Debug)]
struct SlotIndex(NonZeroU16);

#[derive(Default, Debug)]
struct SlotInfo {
    version: u16,
    is_free: bool,
    type_index: u8,
    value_index_or_next: u16,
}

struct SlotAllocator {
    slots: Vec<SlotInfo>,
    freelist_head: Option<NonZeroU16>,
}

impl SlotAllocator {
    fn new() -> Self {
        Self {
            slots: vec![SlotInfo::default()],
            freelist_head: None,
        }
    }

    fn alloc(&mut self) -> Option<SlotIndex> {
        if let Some(index) = self.freelist_head {
            self.freelist_head = NonZeroU16::new(self.slots[index.get() as usize].value_index_or_next);
            Some(SlotIndex(index))
        } else if self.slots.len() < (u16::MAX as usize) {
            let index = unsafe { NonZeroU16::new_unchecked(self.slots.len() as u16) };
            self.slots.push(SlotInfo::default());
            Some(SlotIndex(index))
        } else {
            None
        }
    }

    fn free(&mut self, index: SlotIndex) {
        let slot = &mut self.slots[index.0.get() as usize];
        slot.is_free = true;
        slot.type_index = 0;

        if slot.version < u16::MAX {
            slot.version += 1;
            slot.value_index_or_next = self.freelist_head.map_or(0, |v| v.get());
            self.freelist_head = Some(index.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_allocator() {
        let mut slots = {
            let init = SlotAllocator::new();
            assert_eq!(init.slots.len(), 1);
            assert_eq!(init.freelist_head, None);
            init
        };
        {
            let slot = slots.alloc().unwrap();
            assert_eq!(slot, SlotIndex(NonZeroU16::new(1).unwrap()));
            assert_eq!(slots.slots.len(), 2);
            assert_eq!(slots.freelist_head, None);

            slots.free(slot);
            assert_eq!(slots.slots.len(), 2);
            assert_eq!(slots.freelist_head, Some(NonZeroU16::new(1).unwrap()));

            let slot = slots.alloc().unwrap();
            assert_eq!(slot, SlotIndex(NonZeroU16::new(1).unwrap()));
            assert_eq!(slots.slots.len(), 2);
            assert_eq!(slots.freelist_head, None);
        }
    }
}
