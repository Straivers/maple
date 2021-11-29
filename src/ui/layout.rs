//! Code for defining and calculating layouts.
//!
//! All units are held in device-independent and DPI-scaled pixels.

use crate::gfx::{Extent, Rect};

use super::tree::{self, Tree};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not add a new layout node to the tree.")]
    TooManyNodes,
}

crate::traits::newtype_number!(Px, u16);

trait AsPx {
    fn px(self) -> Px;
}

impl AsPx for u16 {
    fn px(self) -> Px {
        Px(self)
    }
}

#[derive(Clone, Copy)]
pub enum YAlignment {
    None,
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

#[derive(Clone, Copy)]
pub enum XAlignment {
    None,
    /// `[A B C        ]`
    Left,
    /// `[        A B C]`
    Right,
    /// `[    A B C    ]`
    Center,
    /// `[  A   B   C  ]`
    Even,
}

#[derive(Clone, Copy)]
pub struct Alignment(u8);

impl Alignment {
    pub fn left() -> Self {
        Self(XAlignment::Left as u8)
    }

    pub fn x_center() -> Self {
        Self(XAlignment::Center as u8)
    }

    pub fn right() -> Self {
        Self(XAlignment::Right as u8)
    }

    pub fn x_even() -> Self {
        Self(XAlignment::Even as u8)
    }

    pub fn top() -> Self {
        Self((YAlignment::Top as u8) << 4)
    }

    pub fn y_center() -> Self {
        Self((YAlignment::Center as u8) << 4)
    }

    pub fn bottom() -> Self {
        Self((YAlignment::Bottom as u8) << 4)
    }

    pub fn y_even() -> Self {
        Self((YAlignment::Even as u8) << 4)
    }

    pub fn new(x: XAlignment, y: YAlignment) -> Self {
        Self((x as u8) | (y as u8) << 4)
    }

    pub fn is_x(self) -> bool {
        (self.0 & 0x0F) != 0
    }

    pub fn is_y(self) -> bool {
        (self.0 & 0xF0) != 0
    }

    pub fn x(self) -> Option<XAlignment> {
        match self.0 & 0x0F {
            1 => Some(XAlignment::Left),
            2 => Some(XAlignment::Center),
            3 => Some(XAlignment::Right),
            4 => Some(XAlignment::Even),
            _ => None,
        }
    }

    pub fn y(self) -> Option<YAlignment> {
        match self.0 & 0x0F {
            0x10 /*0x0F + 1*/ => Some(YAlignment::Top),
            0x11 /*0x0F + 2*/ => Some(YAlignment::Center),
            0x12 /*0x0F + 3*/ => Some(YAlignment::Bottom),
            0x13 /*0x0F + 4*/ => Some(YAlignment::Even),
            _ => None
        }
    }
}

#[derive(Clone, Copy)]
pub enum Kind {
    None,
    Row {
        alignment: YAlignment,
        margin: Px,
    },
    Column {
        alignment: XAlignment,
        margin: Px,
    },
    Flex {
        alignment: Alignment,
        vertical_margin: Px,
        horizontal_margin: Px,
    },
}

impl Default for Kind {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Copy)]
pub struct Layout {
    widget_id: u16,
    min_extent: Extent<Px>,
    max_extent: Extent<Px>,
    layout: Kind,
}

#[derive(Clone, Copy)]
pub struct Region {
    widget_id: u16,
    bounds: Rect<Px>,
}

pub fn compute_layout(soft_bounds: Rect<Px>, tree: &tree::Tree<Layout>, regions: &mut tree::Tree<Region>) {
    todo!()
}
