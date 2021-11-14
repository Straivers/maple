#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extent {
    pub width: f32,
    pub height: f32,
}

impl Extent {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub lower_left_corner: Point,
    pub extent: Extent,
}

impl Rect {
    pub const INDICES: [u16; 6] = [0, 3, 2, 0, 2, 1];

    pub fn points(&self) -> [Point; 4] {
        [
            self.lower_left_corner,
            Point {
                x: self.lower_left_corner.x,
                y: self.lower_left_corner.y + self.extent.height,
            },
            Point {
                x: self.lower_left_corner.x + self.extent.width,
                y: self.lower_left_corner.y + self.extent.height,
            },
            Point {
                x: self.lower_left_corner.x + self.extent.width,
                y: self.lower_left_corner.y,
            },
        ]
    }
}
