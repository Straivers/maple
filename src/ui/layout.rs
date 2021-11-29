//! Code for defining and calculating layouts.
//!
//! All units are held in device-independent and DPI-scaled pixels.

#![allow(dead_code)]

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

enum VerticalAlignment {
    /// ```
    /// ﹇
    /// A
    /// B
    /// C
    /// 
    /// 
    /// ﹈
    /// ```
    Top,
    /// ```
    /// ﹇
    /// 
    /// 
    /// A
    /// B
    /// C
    /// ﹈
    /// ```
    Bottom,
    /// ```
    /// ﹇
    /// 
    /// A
    /// B
    /// C
    /// 
    /// ﹈
    /// ```
    Center,
    /// ```
    /// ﹇
    /// 
    /// A
    /// 
    /// B
    /// 
    /// C
    /// 
    /// ﹈
    /// ```
    Even,
}

enum HorizontalAlignment {
    /// `[A B C        ]`
    Left,
    /// `[        A B C]`
    Right,
    /// `[    A B C    ]`
    Center,
    /// `[  A   B   C  ]`
    Justified,
}

enum Layout {
    Row {
        alignment: VerticalAlignment,
        vertical_margin: Px
    },
    Column {
        alignment: HorizontalAlignment,
        horizontal_margin: Px
    },
    Flex {
        alignment: HorizontalAlignment,
        vertical_margin: Px,
        horizontal_margin: Px,
    }
}

struct TreeNode {
    widget_id: u16,
    min_extent: Extent<Px>,
    max_extent: Extent<Px>,
    first_child: u16,
    num_children: u16,
    layout: Layout,
}

#[test]
fn t() {
    println!("{}", std::mem::size_of::<TreeNode>());
}

struct Tree {
    nodes: Vec<TreeNode>,
    children: Vec<u16>,
}
