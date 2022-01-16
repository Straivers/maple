#![allow(unused_variables)]

mod image;
pub use image::*;

mod shapes;
pub use shapes::*;

pub struct Canvas {
    target: Image,
}

impl Canvas {
    pub fn new(format: Format, color_space: ColorSpace, size: Extent) -> Self {
        Self {
            target: Image::new(format, color_space, size),
        }
    }

    pub fn with_target(target: Image) -> Self {
        Self { target }
    }

    pub fn finish(self) -> Image {
        self.target
    }

    pub fn draw_shape(&mut self, path: &Path, style: &Style) {
        todo!()
    }
}

pub enum DrawMode {
    Fill,
    Stroke { line_width: f32 },
}

// TODO: What color space is this in?
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
