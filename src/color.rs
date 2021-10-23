#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub fn normalized(r: f32, g: f32, b: f32, a: f32) -> Self {
        let r8 = (r * u8::MAX as f32).round() as u8;
        let g8 = (g * u8::MAX as f32).round() as u8;
        let b8 = (b * u8::MAX as f32).round() as u8;
        let a8 = (a * u8::MAX as f32).round() as u8;
        Color {
            r: r8,
            g: g8,
            b: b8,
            a: a8,
        }
    }
}
