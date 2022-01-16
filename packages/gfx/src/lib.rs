#![allow(unused_variables)]

mod image;
pub use image::*;

mod shapes;
pub use shapes::*;

pub struct Canvas<F: PixelFormat> {
    target: Image<F>,
}

impl <F: PixelFormat> Canvas<F> {
    pub fn new(size: Extent) -> Self {
        Self {
            target: Image::new(size),
        }
    }

    pub fn with_target(target: Image<F>) -> Self {
        Self { target }
    }

    pub fn finish(self) -> Image<F> {
        self.target
    }

    pub fn draw_path(&mut self, path: &Path, style: &Style) {
        todo!()
    }
}

pub enum DrawMode {
    Fill,
    Stroke { line_width: f32 },
}

/// A [`Color`] represents the amount of each component of red, green, blue, and
/// alpha used to create a visual color, varying from 0 to 1 where 0 is the
/// minimum representable amount of that color, and 1 is the maximum. Thus,
/// `Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }` is as red as possible (the Red
/// primary), `Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }` is as white as possible
/// (the white point), and so on.
/// 
/// What the values actually mean in terms of visual color depends on the color
/// space in use.
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

pub struct Style {
    color: Color,
    mode: DrawMode,
}
