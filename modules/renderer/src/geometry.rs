use std::{fmt::Debug, ops::Add};

#[derive(PartialEq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct float2(pub f32, pub f32);

impl float2 {
    pub fn x(&self) -> f32 {
        self.0
    }

    pub fn y(&self) -> f32 {
        self.1
    }
}

impl Debug for float2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("float2")
            .field("x", &self.0)
            .field("y", &self.1)
            .finish()
    }
}

impl Add for float2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

pub struct Rect {
    pub position: float2,
    pub extent: float2,
}

impl Rect {
    pub fn x(&self) -> f32 {
        self.position.x()
    }

    pub fn y(&self) -> f32 {
        self.position.y()
    }

    pub fn width(&self) -> f32 {
        self.extent.x()
    }

    pub fn height(&self) -> f32 {
        self.extent.y()
    }
}
