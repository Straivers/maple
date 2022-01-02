#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
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

    pub fn random_rgb() -> Self {
        use rand::random;
        Color {
            r: random(),
            g: random(),
            b: random(),
            a: 255,
        }
    }

    pub fn unpack(packed: u32) -> Self {
        Self {
            r: packed as u8,
            g: (packed >> 8) as u8,
            b: (packed >> 16) as u8,
            a: (packed >> 24) as u8,
        }
    }

    pub fn pack(self) -> u32 {
        let mut packed = self.r as u32;
        packed |= (self.g as u32) << 8;
        packed |= (self.b as u32) << 16;
        packed |= (self.a as u32) << 24;
        packed
    }
}
