use std::slice::SliceIndex;

use super::types::*;

union Object<T> {
    object: std::mem::ManuallyDrop<T>,
    next_free: Option<ObjectIndex>,
}

/// Stores objects in a flat array addressed by [`ObjectIndex`]es. Freed objects
/// are placed on a free list and made available for future allocations. In
/// order to ensure that objects are correctly dropped, all objects must be
/// deleted (with `delete()`) before the [`Storage`] is dropped.
///
/// This design was made under the following assumptions:
///
/// - Memory efficiency is very important.
/// - The position of an object should not move once allocated.
/// - It is more likely for an application to add items than to remove them.
/// - Programs typically enter a steady-state in terms of the number of objects.
/// - Minimal work should be done when `delete()` _is_ called.
///
/// This has the benefit of adding no memory overhead to storing freed items.
///
/// Note:
///
/// - All objects must be deleted before the storage object can be dropped!
pub struct Storage<T> {
    values: Vec<Object<T>>,
    free_list: Option<ObjectIndex>,
    num_free_objects: usize,
}

impl<T> Storage<T> {
    pub fn new() -> Self {
        Self {
            values: vec![],
            free_list: None,
            num_free_objects: 0,
        }
    }

    /// # Safety
    /// 
    /// Make sure that `index` points to a live object. Pointing to an
    /// freed object produces undefined garbage.
    pub unsafe fn get(&self, index: ObjectIndex) -> &T {
        &self.values[index.0 as usize].object
    }

    pub fn store(&mut self, value: T) -> Option<ObjectIndex> {
        if let Some(index) = self.free_list {
            let object = &mut self.values[index.0 as usize];
            unsafe {
                self.free_list = object.next_free;
                object.object = std::mem::ManuallyDrop::new(value);
            }
            self.num_free_objects -= 1;
            Some(index)
        } else if let Ok(index) = self.values.len().try_into() {
            self.values.push(Object::<T> {
                object: std::mem::ManuallyDrop::new(value),
            });
            Some(ObjectIndex(index))
        } else {
            None
        }
    }

    /// # Safety
    /// 
    /// 1. The object must not have been previously deleted.
    pub unsafe fn delete(
        &mut self,
        index: ObjectIndex,
        destructor: &mut dyn FnMut(&mut T),
    ) {
        if let Some(object) = self.values.get_mut(index.0 as usize) {
            (destructor)(&mut object.object);
            object.next_free = self.free_list;
            self.free_list = Some(index);
            self.num_free_objects += 1;
        }
    }
}

impl <T> Drop for Storage<T> {
    fn drop(&mut self) {
        assert_eq!(self.num_free_objects, self.values.len(), "Storage must not have any allocated objects when being dropped!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_storage_test() {
        let mut storage = Storage::new();

        unsafe {
            let i0 = storage.store(0u128).unwrap();
            assert_eq!(*storage.get(i0), 0);
            let i1 = storage.store(1u128).unwrap();
            assert_eq!(*storage.get(i1), 1);
            let i2 = storage.store(2u128).unwrap();
            assert_eq!(*storage.get(i2), 2);
            let i3 = storage.store(3u128).unwrap();
            assert_eq!(*storage.get(i3), 3);
            let i4 = storage.store(4u128).unwrap();
            assert_eq!(*storage.get(i4), 4);
    
            storage.delete(i1, &mut |_| {});
    
            let i5 = storage.store(5u128).unwrap();
            assert_eq!(i5, i1);
            assert_eq!(*storage.get(i5), 5);

            storage.delete(i0, &mut |_| {});
            // i1 was previously deleted to make room for i5
            storage.delete(i2, &mut |_| {});
            storage.delete(i3, &mut |_| {});
            storage.delete(i4, &mut |_| {});
            storage.delete(i5, &mut |_| {});
        }
    }
}
