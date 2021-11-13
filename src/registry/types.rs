use std::any::Any;

pub const ALLOCATIONS_NOT_FREED: &str =
    "All allocations must be freed before destroying the registry.";

/// A unique identifier associated with an item stored in a [`ItemStorage`]
/// object.
#[repr(align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Id {
    pub(crate) index: SlotIndex,
    pub(crate) version: Version,
}

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

pub trait RegistryObject {
    const TYPE: Type;
}

macro_rules! impl_registry_object {
    ($t:ty, $value:expr) => {
        impl RegistryObject for $t {
            const TYPE: Type = $value;
        }
    };
}

impl_registry_object!(u128, Type::U128);
impl_registry_object!(i128, Type::I128);
impl_registry_object!(&'static str, Type::StaticStr);
impl_registry_object!(Box<dyn Any>, Type::Any);
impl_registry_object!(u64, Type::U64);
impl_registry_object!(i64, Type::I64);
impl_registry_object!(f64, Type::F64);
impl_registry_object!(u32, Type::U32);
impl_registry_object!(i32, Type::I32);
impl_registry_object!(f32, Type::F32);
impl_registry_object!(char, Type::Char);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotIndex(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectIndex(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version(pub u16);
