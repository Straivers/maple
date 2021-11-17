/// The size of a window in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhysicalSize {
    pub width: u16,
    pub height: u16,
}

impl PhysicalSize {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}
