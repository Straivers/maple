pub trait OutputIter<T> {
    fn push(&mut self, value: T);
}

impl<T> OutputIter<T> for Vec<T> {
    fn push(&mut self, value: T) {
        Vec::push(self, value);
    }
}

pub trait CountingOutputIter<T>: OutputIter<T> {
    fn len(&self) -> usize;
}

impl<T> CountingOutputIter<T> for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

/// A marker trait for plain-old-data types.
pub trait PoD: 'static + Sized + Copy + Send + Sync {}

macro_rules! impl_trait {
    ($trait:ident for $($i:ty)+) => {
        $(impl $trait for $i {})+
    };
}

impl_trait!(PoD for usize u8 u16 u32 u64 isize i8 i16 i32 i64);
impl_trait!(PoD for f64 f32);

/// The Number trait represents a type that can be treated as a simple number.
/// It is sized, cheaply copyable, printable, and supports the basic numerical
/// operations.
///
/// This trait must be manually defined.
pub trait Number: PoD + NumberOps {}

impl_trait!(Number for usize u8 u16 u32 u64 isize i8 i16 i32 i64);
impl_trait!(Number for f64 f32);

/// Trait for the basic numerical operations.
pub trait NumberOps<Rhs = Self, Output = Self>:
    PartialEq + PartialOrd + 
    std::ops::Add<Rhs, Output = Output>
    + std::ops::Sub<Rhs, Output = Output>
    + std::ops::Mul<Rhs, Output = Output>
    + std::ops::Div<Rhs, Output = Output>
    + std::ops::Rem<Rhs, Output = Output>
{
}

/// Automatically applied implementation for any type that implements the
/// required operations. This is so that custom number types (like an f16 or
/// f128 type) will be automatically supported.
impl<T, Rhs, Output> NumberOps<Rhs, Output> for T where
    T: PartialEq + PartialOrd + std::ops::Add<Rhs, Output = Output>
        + std::ops::Sub<Rhs, Output = Output>
        + std::ops::Mul<Rhs, Output = Output>
        + std::ops::Div<Rhs, Output = Output>
        + std::ops::Rem<Rhs, Output = Output>
{
}
