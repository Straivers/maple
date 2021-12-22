use super::px::Px;

use std::ops::Add;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: Px,
    pub y: Px,
}

impl Point {
    pub fn new(x: Px, y: Px) -> Self {
        Self { x, y }
    }
}

impl Add<Extent> for Point {
    type Output = Rect;

    fn add(self, rhs: Extent) -> Self::Output {
        Rect {
            point: self,
            extent: rhs,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Offset {
    x: Px,
    y: Px,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extent {
    width: Px,
    height: Px,
}

impl Extent {
    pub fn new(width: Px, height: Px) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub point: Point,
    pub extent: Extent,
}

impl Rect {
    pub const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    pub fn new(x: Px, y: Px, width: Px, height: Px) -> Self {
        Self {
            point: Point { x, y },
            extent: Extent { width, height },
        }
    }

    pub fn x(&self) -> Px {
        self.point.x
    }

    pub fn y(&self) -> Px {
        self.point.y
    }

    pub fn width(&self) -> Px {
        self.extent.width
    }

    pub fn height(&self) -> Px {
        self.extent.height
    }

    pub fn points(&self) -> [Point; 4] {
        [
            self.point,
            Point {
                x: self.x(),
                y: self.y() + self.height(),
            },
            Point {
                x: self.x() + self.width(),
                y: self.y() + self.height(),
            },
            Point {
                x: self.x() + self.width(),
                y: self.y(),
            },
        ]
    }
}
