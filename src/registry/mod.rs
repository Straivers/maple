mod object;
mod slot;
mod types;

use self::object::{Object128, Object32, Object64};
use std::{any::Any, marker::PhantomData, mem::ManuallyDrop};

pub use types::*;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct TypedId<T>(Id, PhantomData<T>);

impl<T> TypedId<T> {
    pub fn get(&self) -> Id {
        self.0
    }
}

impl<T> Clone for TypedId<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T> Copy for TypedId<T> {}

pub struct Registry {
    slots: slot::Storage,
    objects_128: object::Storage<Object128>,
    objects_64: object::Storage<Object64>,
    objects_32: object::Storage<Object32>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            slots: slot::Storage::new(),
            objects_128: object::Storage::new(),
            objects_64: object::Storage::new(),
            objects_32: object::Storage::new(),
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
            U128 | I128 | StaticStr => unsafe { self.objects_128.delete(object_index, |_| {}) },
            Any => unsafe {
                self.objects_128
                    .delete(object_index, |value| ManuallyDrop::drop(&mut value.any))
            },
            U64 | I64 | F64 => unsafe { self.objects_64.delete(object_index, |_| {}) },
            U32 | I32 | F32 | Char => unsafe { self.objects_32.delete(object_index, |_| {}) },
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

macro_rules! impl_simple_ops {
    ($api_type:ty, $api_name:ident, $kind:expr, $storage:ident, $object_type:ident) => {
        impl Ops<$api_type> for Registry {
            fn get(&self, id: Id) -> Option<&$api_type> {
                let (object_type, object_index) = self.slots.get(id)?;

                if object_type == $kind {
                    Some(unsafe { &self.$storage.get(object_index).$api_name })
                } else {
                    None
                }
            }

            fn get_typed(&self, id: TypedId<$api_type>) -> Option<&$api_type> {
                let (_, object_index) = self.slots.get(id.get())?;
                Some(unsafe { &self.$storage.get(object_index).$api_name })
            }

            fn insert(&mut self, value: $api_type) -> Option<TypedId<$api_type>> {
                let object_index = self.$storage.store($object_type { $api_name: value })?;
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

impl_simple_ops!(u128, u128, Type::U128, objects_128, Object128);
impl_simple_ops!(i128, i128, Type::I128, objects_128, Object128);
impl_simple_ops!(
    &'static str,
    static_str,
    Type::StaticStr,
    objects_128,
    Object128
);
impl_simple_ops!(u64, u64, Type::U64, objects_64, Object64);
impl_simple_ops!(i64, i64, Type::I64, objects_64, Object64);
impl_simple_ops!(f64, f64, Type::F64, objects_64, Object64);
impl_simple_ops!(u32, u32, Type::U32, objects_32, Object32);
impl_simple_ops!(i32, i32, Type::I32, objects_32, Object32);
impl_simple_ops!(f32, f32, Type::F32, objects_32, Object32);
impl_simple_ops!(char, char, Type::Char, objects_32, Object32);

impl Ops<Box<dyn Any>> for Registry {
    fn get(&self, id: Id) -> Option<&Box<dyn Any>> {
        let (object_type, object_index) = self.slots.get(id)?;

        if object_type == Type::Any {
            Some(unsafe { &self.objects_128.get(object_index).any })
        } else {
            None
        }
    }

    fn get_typed(&self, id: TypedId<Box<dyn Any>>) -> Option<&Box<dyn Any>> {
        let (_, object_index) = self.slots.get(id.get())?;
        Some(unsafe { &self.objects_128.get(object_index).any })
    }

    fn insert(&mut self, value: Box<dyn Any>) -> Option<TypedId<Box<dyn Any>>> {
        let object_index = self.objects_128.store(Object128 {
            any: ManuallyDrop::new(value),
        })?;
        self.slots
            .alloc(Type::Any, object_index)
            .map(|id| TypedId(id, PhantomData))
    }

    fn remove_typed(&mut self, id: TypedId<Box<dyn Any>>) {
        let (_, index) = self.slots.free(id.get());
        unsafe { self.objects_128.delete(index, |_| {}) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{seq::SliceRandom, thread_rng, Rng};

    macro_rules! test_simple_type_ops {
        ($t:ident) => {
            mod $t {
                use super::*;

                #[test]
                fn basic_ops() {
                    const COUNT: usize = 100;

                    let mut registry = Registry::new();

                    let mut ids = Vec::<(usize, TypedId<$t>)>::with_capacity(COUNT);
                    let values = {
                        let mut arr: [$t; COUNT] = [Default::default(); COUNT];
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
                }
            }
        };
    }

    test_simple_type_ops!(u128);
    test_simple_type_ops!(i128);
    test_simple_type_ops!(u64);
    test_simple_type_ops!(i64);
    test_simple_type_ops!(f64);
    test_simple_type_ops!(u32);
    test_simple_type_ops!(i32);
    test_simple_type_ops!(f32);
    test_simple_type_ops!(char);

    mod static_str {
        use super::*;
        #[test]
        fn basic_ops() {
            let mut registry = Registry::new();
            let id = registry.insert("Hello").unwrap();
            assert_eq!(registry.get_typed(id).unwrap(), &"Hello");
            let raw: &'static str = *registry.get(id.get()).unwrap();
            assert_eq!(raw, "Hello");
            registry.remove_typed(id);
        }
    }

    mod any {
        use super::*;

        struct Test {
            value: u32,
        }

        impl Test {
            fn echo(&self) -> u32 {
                self.value
            }
        }

        #[test]
        fn basic_ops() {
            let mut registry = Registry::new();
            let id = registry
                .insert(Box::new(Test { value: 100 }) as Box<dyn Any>)
                .unwrap();
            assert_eq!(
                registry
                    .get_typed(id)
                    .unwrap()
                    .downcast_ref::<Test>()
                    .unwrap()
                    .echo(),
                100
            );
            let raw: &Box<dyn Any> = registry.get(id.get()).unwrap();
            assert_eq!(raw.downcast_ref::<Test>().unwrap().echo(), 100);
            registry.remove_typed(id);
        }
    }

    #[test]
    #[should_panic(expected = "All allocations must be freed before destroying the registry.")]
    fn bad_cleanup() {
        let mut registry = Registry::new();
        let _ = registry.insert(1u128).unwrap();
        // registry destructor should fail.
    }
}
