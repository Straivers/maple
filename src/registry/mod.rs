mod object;
mod slot;

pub use slot::Id;
use std::{any::Any, marker::PhantomData, mem::ManuallyDrop};

pub const ALLOCATIONS_NOT_FREED: &str =
    "All allocations must be freed before destroying the registry.";

/// Enumerates the types of data that can be stored in an [`ItemStorage`]
/// object. No guarantee is made that storing an object if type `T` will occupy
/// only `std::mem::size_of::<T>()` bytes.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Unknown   = 0,

    // 128-bit types
    U128      = 1,
    I128      = 2,
    Any       = 3, // Box<dyn Any>
    StaticStr = 4, // &'static str

    // 64-bit types
    U64       = 11,
    I64       = 12,
    F64       = 13,

    // 32-bit types
    U32       = 21,
    I32       = 22,
    F32       = 23,
    Char      = 24,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    OutOfIds,
    InvalidId,
    TypeMismatch { expected: Type, actual: Type },
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct TypedId<T>(Id, PhantomData<T>);

impl<T> TypedId<T> {
    pub fn get(self) -> Id {
        self.0
    }
}

impl<T> Clone for TypedId<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T> Copy for TypedId<T> {}

union Object128 {
    u128: u128,
    i128: i128,
    any: ManuallyDrop<Box<dyn Any>>,
    static_str: &'static str,
}

union Object64 {
    u64: u64,
    i64: i64,
    f64: f64,
}

union Object32 {
    char: char,
    u32: u32,
    i32: i32,
    f32: f32,
}

pub struct Registry {
    slots: slot::Storage<(Type, object::Index)>,
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

    /// Returns `true` if the `id` refers to a value.
    pub fn is_valid(&self, id: Id) -> bool {
        self.slots.is_valid(id)
    }

    /// Returns the type of the value referred to by `id`, or [`None`]
    /// otherwise.
    pub fn type_of(&self, id: Id) -> Result<Type, Error> {
        self.slots
            .get(id)
            .map_or(Err(Error::InvalidId), |(object_type, _)| Ok(object_type))
    }

    /// Removes the value referred to by `id`.
    pub fn remove(&mut self, id: Id) -> Result<(), Error> {
        use Type::*;
        
        let (object_type, object_index) = self.slots.free(id).ok_or(Error::InvalidId)?;
        match object_type {
            U128 | I128 | StaticStr => unsafe { self.objects_128.delete(object_index, |_| {}) },
            Any => unsafe {
                self.objects_128
                    .delete(object_index, |value| ManuallyDrop::drop(&mut value.any));
            },
            U64 | I64 | F64 => unsafe { self.objects_64.delete(object_index, |_| {}) },
            U32 | I32 | F32 | Char => unsafe { self.objects_32.delete(object_index, |_| {}) },
            _ => unimplemented!(),
        }

        Ok(())
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        assert_eq!(self.slots.num_active(), 0, "{}", ALLOCATIONS_NOT_FREED);
    }
}

pub trait Ops<T> {
    /// Retrieves a reference to the object referred to by `id`. Returns
    /// [`None`] if `!is_valid(id)` or if `type_of(id) != T`.
    fn get(&self, id: Id) -> Result<&T, Error>;

    /// Retrieves a mutable reference to the object referred to by `id`. Returns
    /// [`None`] if `!is_valid(id)` or if `type_of(id) != T`.
    fn get_mut(&mut self, id: Id) -> Result<&mut T, Error>;

    /// Retrieves a reference to the object referred to by `id`. Using the typed
    /// version of `get()` elides the type check that would otherwise be needed.
    /// Returns [`None`] if `!is_valid(id)`.
    fn get_typed(&self, id: TypedId<T>) -> Result<&T, Error>;

    /// Retrieves a mutable reference to the object referred to by `id`. Using
    /// the typed version of `get()` elides the type check that would otherwise
    /// be needed. Returns [`None`] if `!is_valid(id)`.
    fn get_typed_mut(&mut self, id: TypedId<T>) -> Result<&mut T, Error>;

    /// Inserts a new value into the [`Registry`], returning an ID that can be
    /// used to retrieve it at a later time.
    fn insert(&mut self, value: T) -> Result<TypedId<T>, Error>;

    /// Destroys the value identified by `id` if `is_valid(id)`.
    fn remove_typed(&mut self, id: TypedId<T>) -> Result<(), Error>;
}

macro_rules! impl_ops {
    ($api_type:ty, $api_name:ident, $kind:expr, $storage:ident, $object_type:ident, $ctor:expr, $dtor:expr) => {
        impl Ops<$api_type> for Registry {
            fn get(&self, id: Id) -> Result<&$api_type, Error> {
                let (object_type, object_index) = self.slots.get(id).ok_or(Error::InvalidId)?;

                if object_type == $kind {
                    Ok(unsafe { &self.$storage.get(object_index).$api_name })
                } else {
                    Err(Error::TypeMismatch {
                        expected: $kind,
                        actual: object_type,
                    })
                }
            }

            fn get_mut(&mut self, id: Id) -> Result<&mut $api_type, Error> {
                let (object_type, object_index) = self.slots.get(id).ok_or(Error::InvalidId)?;

                if object_type == $kind {
                    Ok(unsafe { &mut self.$storage.get_mut(object_index).$api_name })
                } else {
                    Err(Error::TypeMismatch {
                        expected: $kind,
                        actual: object_type,
                    })
                }
            }

            fn get_typed(&self, id: TypedId<$api_type>) -> Result<&$api_type, Error> {
                let (_, object_index) = self.slots.get(id.get()).ok_or(Error::InvalidId)?;
                Ok(unsafe { &self.$storage.get(object_index).$api_name })
            }

            fn get_typed_mut(&mut self, id: TypedId<$api_type>) -> Result<&mut $api_type, Error> {
                let (_, object_index) = self.slots.get(id.get()).ok_or(Error::InvalidId)?;
                Ok(unsafe { &mut self.$storage.get_mut(object_index).$api_name })
            }

            fn insert(&mut self, value: $api_type) -> Result<TypedId<$api_type>, Error> {
                let object_index = self
                    .$storage
                    .store($object_type {
                        $api_name: $ctor(value),
                    })
                    .ok_or(Error::OutOfIds)?;
                self.slots
                    .alloc(($kind, object_index))
                    .map(|id| TypedId(id, PhantomData))
                    .ok_or(Error::OutOfIds)
            }

            fn remove_typed(&mut self, id: TypedId<$api_type>) -> Result<(), Error> {
                let (_, index) = self.slots.free(id.get()).ok_or(Error::InvalidId)?;
                unsafe { self.$storage.delete(index, $dtor) };
                Ok(())
            }
        }
    };
}

macro_rules! impl_ops_simple {
    ($api_name:ident, $kind:expr, $storage:ident, $object_type:ident) => {
        impl_ops!(
            $api_name,
            $api_name,
            $kind,
            $storage,
            $object_type,
            |v| v,
            |_| {}
        );
    };
}

impl_ops_simple!(u128, Type::U128, objects_128, Object128);
impl_ops_simple!(i128, Type::I128, objects_128, Object128);
impl_ops!(
    &'static str,
    static_str,
    Type::StaticStr,
    objects_128,
    Object128,
    |v| v,
    |_| {}
);
impl_ops!(
    Box<dyn Any>,
    any,
    Type::Any,
    objects_128,
    Object128,
    |v| ManuallyDrop::new(v),
    |v| ManuallyDrop::drop(&mut v.any)
);
impl_ops_simple!(u64, Type::U64, objects_64, Object64);
impl_ops_simple!(i64, Type::I64, objects_64, Object64);
impl_ops_simple!(f64, Type::F64, objects_64, Object64);
impl_ops_simple!(u32, Type::U32, objects_32, Object32);
impl_ops_simple!(i32, Type::I32, objects_32, Object32);
impl_ops_simple!(f32, Type::F32, objects_32, Object32);
impl_ops_simple!(char, Type::Char, objects_32, Object32);

#[cfg(test)]
mod tests {
    use super::{Any, Error, Ops, Registry, TypedId};
    use rand::{seq::SliceRandom, thread_rng, Rng};

    macro_rules! test_simple_type_ops {
        ($t:ident) => {
            mod $t {
                use super::*;

                #[test]
                fn basic_ops() {
                    const COUNT: usize = 1000;

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
                            registry.remove_typed(*id).unwrap();
                        } else {
                            registry.remove(id.get()).unwrap();
                        }
                    }

                    for (_, id) in &ids {
                        let typed = registry.get_typed(*id);
                        let raw: Result<&$t, Error> = registry.get(id.get());

                        assert_eq!(typed, raw);
                        assert_eq!(raw, Err(Error::InvalidId));
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
            registry.remove_typed(id).unwrap();
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
            registry.remove_typed(id).unwrap();
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
