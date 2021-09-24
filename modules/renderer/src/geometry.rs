use std::fmt::Debug;

#[derive(PartialEq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct float2(pub f32, pub f32);

impl float2 {
    pub fn x(&self) -> f32 {
        self.0
    }

    pub fn y(&self) -> f32 {
        self.1
    }
}

impl Debug for float2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("float2")
            .field("x", &self.0)
            .field("y", &self.1)
            .finish()
    }
}
