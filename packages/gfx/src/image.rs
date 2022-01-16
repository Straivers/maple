use crate::Color;

/// Describes the way color information will be stored per pixel.
pub trait PixelFormat: Clone + From<Color> {
    const BYTES_PER_PIXEL: usize = std::mem::size_of::<Self>();
    const BLACK: Self;
    const WHITE: Self;
}

/// The standard SRGB color space with 8 bits per channel (0-255).
#[derive(Clone, Copy, Debug)]
pub struct RgbaU8Srgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PixelFormat for RgbaU8Srgb {
    const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    const WHITE: Self = Self { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };
}

impl From<Color> for RgbaU8Srgb {
    fn from(color: Color) -> Self {
        Self {
            r: (color.r * u8::MAX as f32) as u8,
            g: (color.g * u8::MAX as f32) as u8,
            b: (color.b * u8::MAX as f32) as u8,
            a: (color.a * u8::MAX as f32) as u8,
        }
    }
}

/// Describes the horizontal and vertical size of an image.
#[derive(Clone, Copy, Debug, Default)]
pub struct Extent {
    width: u32,
    height: u32,
}

/// An image with clone-on-write semantics.
#[derive(Clone)]
pub struct Image<F: PixelFormat> {
    size: Extent,
    bytes: Box<[F]>,
}

impl <F: PixelFormat> Image<F> {
    pub fn new(size: Extent) -> Self {
        let num_pixels = (size.width * size.height) as usize;
        Self {
            size,
            bytes: vec![F::BLACK; num_pixels * F::BYTES_PER_PIXEL].into_boxed_slice(),
        }
    }

    pub fn size(&self) -> Extent {
        self.size
    }

    pub fn bytes(&self) -> &[F] {
        &self.bytes
    }

    /// Clears the entire image to `color`.
    pub fn clear(&mut self, color: Color) {
        self.bytes.fill(color.into())
    }
}
