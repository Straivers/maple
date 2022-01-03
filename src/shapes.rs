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
    pub x: Px,
    pub y: Px,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Extent {
    pub width: Px,
    pub height: Px,
}

impl Extent {
    pub const MAX: Self = Self::new(Px::MAX, Px::MAX);

    pub const fn new(width: Px, height: Px) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub point: Point,
    pub extent: Extent,
}

impl Rect {
    pub const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    pub const fn new(x: Px, y: Px, width: Px, height: Px) -> Self {
        Self {
            point: Point { x, y },
            extent: Extent { width, height },
        }
    }

    pub const fn from_extent(x: Px, y: Px, extent: Extent) -> Self {
        Self {
            point: Point { x, y },
            extent,
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

    pub fn left(&self) -> Px {
        self.point.x
    }

    pub fn right(&self) -> Px {
        self.point.x + self.extent.width
    }

    pub fn top(&self) -> Px {
        self.point.y
    }

    pub fn bottom(&self) -> Px {
        self.point.y + self.extent.height
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

    pub fn contains_point(&self, point: Point) -> bool {
        (self.left() <= point.x)
            & (self.right() >= point.x)
            & (self.top() <= point.y)
            & (self.bottom() >= point.y)
    }

    pub fn contains_rect(&self, rect: Self) -> bool {
        (self.left() <= rect.left())
            & (self.right() >= rect.right())
            & (self.top() <= rect.top())
            & (self.bottom() >= rect.bottom())
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rect")
            .field("x", &self.point.x)
            .field("y", &self.point.y)
            .field("width", &self.extent.width)
            .field("height", &self.extent.height)
            .finish()
    }
}
