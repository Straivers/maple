//! This module defines the [`Px`] device-independent pixel unit type for UI
//! layout and sizing.
//!

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

/// A device-independent pixel.
///
/// Note: Multiplication of a pixel by another pixel is not defined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Px(pub i16);

impl Px {
    pub const MAX: Self = Px(i16::MAX);
}

macro_rules! impl_bin_op {
    ($trait:ident, $lhs: ty, $rhs:ty, $func:ident, $extract_left:expr, $extract_right:expr) => {
        impl $trait<$rhs> for $lhs {
            type Output = Px;

            fn $func(self, rhs: $rhs) -> Self::Output {
                Px($extract_left(self).$func($extract_right(rhs)))
            }
        }
    };
}

impl_bin_op!(Add, Px, Self, add, |v: Px| v.0, |v: Px| v.0);
impl_bin_op!(Sub, Px, Self, sub, |v: Px| v.0, |v: Px| v.0);
impl_bin_op!(Div, Px, Self, div, |v: Px| v.0, |v: Px| v.0);
impl_bin_op!(Rem, Px, Self, rem, |v: Px| v.0, |v: Px| v.0);

impl_bin_op!(Mul, Px, i16, mul, |v: Px| v.0, |v: i16| v);
impl_bin_op!(Div, Px, i16, div, |v: Px| v.0, |v: i16| v);
impl_bin_op!(Rem, Px, i16, rem, |v: Px| v.0, |v: i16| v);
impl_bin_op!(Mul, i16, Px, mul, |v: i16| v, |v: Px| v.0);
impl_bin_op!(Div, i16, Px, div, |v: i16| v, |v: Px| v.0);
impl_bin_op!(Rem, i16, Px, rem, |v: i16| v, |v: Px| v.0);

macro_rules! impl_bin_op_assign {
    ($trait:ident, $rhs:ty, $func:ident, $extract_right:expr) => {
        impl $trait<$rhs> for Px {
            fn $func(&mut self, rhs: $rhs) {
                self.0.$func($extract_right(rhs));
            }
        }
    };
}

impl_bin_op_assign!(AddAssign, Self, add_assign, |v: Px| v.0);
impl_bin_op_assign!(SubAssign, Self, sub_assign, |v: Px| v.0);
impl_bin_op_assign!(DivAssign, Self, div_assign, |v: Px| v.0);
impl_bin_op_assign!(RemAssign, Self, rem_assign, |v: Px| v.0);

impl_bin_op_assign!(MulAssign, i16, mul_assign, |v: i16| v);
impl_bin_op_assign!(DivAssign, i16, div_assign, |v: i16| v);
impl_bin_op_assign!(RemAssign, i16, rem_assign, |v: i16| v);

impl PartialEq<i16> for Px {
    fn eq(&self, other: &i16) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Px> for i16 {
    fn eq(&self, other: &Px) -> bool {
        *self == other.0
    }
}

impl PartialOrd<i16> for Px {
    fn partial_cmp(&self, other: &i16) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<Px> for i16 {
    fn partial_cmp(&self, other: &Px) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl From<Px> for f32 {
    fn from(p: Px) -> Self {
        p.0 as f32
    }
}

impl From<i16> for Px {
    fn from(v: i16) -> Self {
        Self(v)
    }
}
