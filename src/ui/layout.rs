//! Code for defining and calculating layouts.
//!
//! All units are held in device-independent and DPI-scaled pixels.

use std::num::NonZeroU16;

use crate::gfx::{Extent, Rect};

/// Unit representing device-independent and DPI-scaled pixels.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
struct Px(u16);

impl crate::traits::PoD for Px {}
impl crate::traits::Number for Px {}

macro_rules! impl_ops {
    ($op:path, $func:ident) => {
        impl $op for Px {
            type Output = Px;

            fn $func(self, rhs: Self) -> Self::Output {
                Px(self.0.$func(rhs.0))
            }
        }
    };
}

impl_ops!(std::ops::Add, add);
impl_ops!(std::ops::Sub, sub);
impl_ops!(std::ops::Mul, mul);
impl_ops!(std::ops::Div, div);
impl_ops!(std::ops::Rem, rem);

trait AsPx {
    fn px(self) -> Px;
}

impl AsPx for u16 {
    fn px(self) -> Px {
        Px(self)
    }
}

#[repr(u8)]
enum Type {
    Row
}

struct RowItem {
    min_extent: Extent<Px>,
    max_extent: Extent<Px>,
}

struct RowContainer {
    items: RowItem,
    vertical_margin: Px,
}

struct TreeNode {
    widget_id: u16,
    bounds: Rect<Px>,
    first_child: u16,
    num_children: u16,
}
