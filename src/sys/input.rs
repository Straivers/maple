use crate::{array_vec::ArrayVec, shapes::Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Released,
    Pressed,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left   = 0,
    Middle = 1,
    Right  = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    None,
    CursorMove {
        position: Point,
    },
    MouseButton {
        button: MouseButton,
        state: ButtonState,
    },
    ScrollWheel {
        x: f32,
        y: f32,
    },
    Char {
        codepoint: char,
    },
}
