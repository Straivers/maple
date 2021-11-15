#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn is_in(&self, rect: Rect) -> bool {
        rect.contains_point(*self)
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

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            lower_left_corner: Point::new(x, y),
            extent: Extent::new(width, height),
        }
    }

    pub fn x(&self) -> f32 {
        self.lower_left_corner.x
    }

    pub fn y(&self) -> f32 {
        self.lower_left_corner.y
    }

    pub fn width(&self) -> f32 {
        self.extent.width
    }

    pub fn height(&self) -> f32 {
        self.extent.height
    }

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

    pub fn contains_point(&self, point: Point) -> bool {
        let left = self.lower_left_corner.x;
        let right = self.lower_left_corner.x + self.extent.width;
        let top = self.lower_left_corner.y + self.extent.height;
        let bottom = self.lower_left_corner.y;

        let in_x = (left <= point.x) & (point.x <= right);
        let in_y = (bottom <= point.y) & (point.y <= top);

        in_x & in_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_rect_intersection() {
        let rect = Rect {
            lower_left_corner: Point { x: 100.0, y: 100.0 },
            extent: Extent {
                width: 200.0,
                height: 400.0,
            },
        };

        // 4 corners
        assert!(rect.contains_point(Point::new(100.0, 100.0)));
        assert!(rect.contains_point(Point::new(100.0, 500.0)));
        assert!(rect.contains_point(Point::new(300.0, 500.0)));
        assert!(rect.contains_point(Point::new(300.0, 100.0)));

        assert!(!rect.contains_point(Point::new(0.0, 0.0)));
    }
}
