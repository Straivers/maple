use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub enum Format {
    RGBA8
}

impl Format {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Format::RGBA8 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ColorSpace {
    SRGB
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Extent {
    width: u32,
    height: u32,
}

/// An image with clone-on-write semantics.
#[derive(Clone)]
pub struct Image {
    format: Format,
    color_space: ColorSpace,
    size: Extent,
    bytes: Rc::<[u8]>,
}

impl Image {
    pub fn new(format: Format, color_space: ColorSpace, size: Extent) -> Self {
        let num_pixels = (size.width * size.height) as usize;
        Self {
            format,
            color_space,
            size,
            bytes: Rc::from(vec![0; num_pixels * format.bytes_per_pixel()])
        }
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn color_space(&self) -> ColorSpace {
        self.color_space
    }

    pub fn size(&self) -> Extent {
        self.size
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
