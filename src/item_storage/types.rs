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
    Any       = 1,
    HeapStr   = 2, // Box<str>
    StaticStr = 3, // &'static str
    U128      = 4,
    I128      = 5,

    // 64-bit types
    VoidPtr   = 11,
    U64       = 12,
    I64       = 13,
    F64       = 14,

    // 32-bit types
    Char      = 21,
    U32       = 22,
    I32       = 23,
    F32       = 24,

    // 16-bit types
    U16       = 31,
    I16       = 32,
    F16       = 33,

    // 8-bit types
    I8        = 41,
    U8        = 42,
    Bool      = 43,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotIndex(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObjectIndex(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version(pub u16);

#[cfg(test)]
mod tests {
    #[test]
    fn type_size() {
        assert_eq!(std::mem::size_of::<Box<str>>(), 16);
        assert_eq!(std::mem::size_of::<&'static str>(), 16);
    }
}
