mod object;
mod slot;
mod types;

use std::marker::PhantomData;

pub use types::*;

use self::object::{Object128, Object64};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypedId<T>(Id, PhantomData<T>);

impl<T> TypedId<T> {
    pub fn get(&self) -> Id {
        self.0
    }
}

pub struct Registry {
    slots: slot::Storage,
    objects_128: object::Storage<Object128>,
    objects_64: object::Storage<Object64>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            slots: slot::Storage::new(),
            objects_128: object::Storage::new(),
            objects_64: object::Storage::new(),
        }
    }

    pub fn is_valid(&self, id: Id) -> bool {
        self.slots.is_valid(id)
    }

    pub fn type_of(&self, id: Id) -> Option<Type> {
        self.slots.get(id).map(|(object_type, _)| object_type)
    }

    pub fn remove(&mut self, id: Id) {
        let (object_type, object_index) = self.slots.free(id);

        use Type::*;
        match object_type {
            U128 | I128 => unsafe { self.objects_128.delete(object_index, |_| {}) },
            U64 | I64 => unsafe { self.objects_64.delete(object_index, |_| {}) },
            _ => unimplemented!(),
        }
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        assert_eq!(self.slots.num_active(), 0, "{}", ALLOCATIONS_NOT_FREED);
    }
}

pub trait Ops<T: RegistryObject> {
    /// Retrieves the object referred to by `id`. Returns [`None`] if
    /// `!is_valid(id)` or if `type_of(id) != T`.
    fn get(&self, id: Id) -> Option<&T>;

    /// Retrieves the object referred to by `id`. Returns [`None`] if
    /// `!is_valid(id)`.
    fn get_typed(&self, id: TypedId<T>) -> Option<&T>;

    fn insert(&mut self, value: T) -> Option<TypedId<T>>;

    fn remove_typed(&mut self, id: TypedId<T>);
}

macro_rules! impl_int_ops {
    ($api_type:ident, $storage_type:ident, $kind:expr, $storage:ident, $object_type:ident) => {
        impl Ops<$api_type> for Registry {
            fn get(&self, id: Id) -> Option<&$api_type> {
                let (object_type, object_index) = self.slots.get(id)?;

                if object_type == $kind {
                    Some(unsafe { &self.$storage.get(object_index).$api_type })
                } else {
                    None
                }
            }

            fn get_typed(&self, id: TypedId<$api_type>) -> Option<&$api_type> {
                let (_, object_index) = self.slots.get(id.get())?;
                Some(unsafe { &self.$storage.get(object_index).$api_type })
            }

            fn insert(&mut self, value: $api_type) -> Option<TypedId<$api_type>> {
                let object_index = self.$storage.store($object_type { $api_type: value })?;
                self.slots
                    .alloc($kind, object_index)
                    .map(|id| TypedId(id, PhantomData))
            }

            fn remove_typed(&mut self, id: TypedId<$api_type>) {
                let (_, index) = self.slots.free(id.get());
                unsafe { self.$storage.delete(index, |_| {}) };
            }
        }
    };
}

impl_int_ops!(u128, u128, Type::U128, objects_128, Object128);
impl_int_ops!(i128, u128, Type::I128, objects_128, Object128);
impl_int_ops!(u64, u64, Type::U64, objects_64, Object64);
impl_int_ops!(i64, u64, Type::I64, objects_64, Object64);

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{seq::SliceRandom, thread_rng, Rng};

    macro_rules! basic_ops_int_test {
        ($t:ty) => {
            const COUNT: usize = 100;

            let mut registry = Registry::new();

            let mut ids = Vec::<(usize, TypedId<$t>)>::with_capacity(COUNT);
            let values = {
                let mut arr: [$t; COUNT]= [0; COUNT];
                thread_rng().fill(&mut arr[..]);
                arr
            };

            for i in 0..COUNT {
                ids.push((i, registry.insert(values[i]).unwrap()));
            }

            ids.shuffle(&mut thread_rng());

            for (index, id) in &ids {
                let typed = *registry.get_typed(*id).unwrap();
                let raw: $t = *registry.get(id.get()).unwrap();
                assert_eq!(typed, raw);
                assert_eq!(raw, values[*index]);
            }

            for (index, id) in &ids {
                if index & 1 == 0 {
                    registry.remove_typed(*id);
                } else {
                    registry.remove(id.get());
                }
            }

            for (_, id) in &ids {
                let typed = registry.get_typed(*id);
                let raw: Option<&$t> = registry.get(id.get());

                assert_eq!(typed, raw);
                assert_eq!(raw, None);
            }
        };
    }

    #[test]
    fn basic_ops_u128() {
        basic_ops_int_test!(u128);
    }

    #[test]
    fn basic_ops_i128() {
        basic_ops_int_test!(i128);
    }

    #[test]
    fn basic_ops_u64() {
        basic_ops_int_test!(u64);
    }

    #[test]
    fn basic_ops_i64() {
        basic_ops_int_test!(i64);
    }

    #[test]
    #[should_panic(expected = "All allocations must be freed before destroying the registry.")]
    fn bad_cleanup() {
        let mut registry = Registry::new();
        let _ = registry.insert(1u128).unwrap();
        // registry destructor should fail.
    }
}
