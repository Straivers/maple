use crate::traits::Number;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point<T>
where
    T: Number + Default,
{
    pub x: T,
    pub y: T,
}

impl<T> Point<T>
where
    T: Number + Default,
{
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn is_in(&self, rect: Rect<T>) -> bool {
        rect.contains_point(*self)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extent<T>
where
    T: Number + Default,
{
    pub width: T,
    pub height: T,
}

impl<T> Extent<T>
where
    T: Number + Default,
{
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }

    pub fn max_value() -> Self {
        Self {
            width: T::max_value(),
            height: T::max_value(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect<T>
where
    T: Number + Default,
{
    pub lower_left_corner: Point<T>,
    pub extent: Extent<T>,
}

impl<T> Rect<T>
where
    T: Number + Default,
{
    pub const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            lower_left_corner: Point::new(x, y),
            extent: Extent::new(width, height),
        }
    }

    pub fn x(&self) -> T {
        self.lower_left_corner.x
    }

    pub fn y(&self) -> T {
        self.lower_left_corner.y
    }

    pub fn width(&self) -> T {
        self.extent.width
    }

    pub fn height(&self) -> T {
        self.extent.height
    }

    pub fn points(&self) -> [Point<T>; 4] {
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

    pub fn contains_point(&self, point: Point<T>) -> bool {
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
