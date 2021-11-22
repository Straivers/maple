#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Released = 0,
    Pressed  = 1,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Unknown = 0,
    Left    = 1,
    Middle  = 2,
    Right   = 3,
}

impl Default for MouseButton {
    fn default() -> Self {
        Self::Unknown
    }
}
