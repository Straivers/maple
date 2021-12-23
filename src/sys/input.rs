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

#[derive(Clone, Copy, Debug)]
pub enum Event {
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

#[derive(Default)]
pub struct State {
    pub cursor: Point,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub mouse_buttons: [ButtonState; 4],
    pub must_redraw: bool,
    pub codepoints: ArrayVec<char, 16>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = State::default();
    }

    pub fn process(&mut self, event: Event) {
        match event {
            Event::CursorMove { position } => {
                self.cursor = position;
            }
            Event::MouseButton { button, state } => {
                self.mouse_buttons[button as usize] = state;
            }
            Event::ScrollWheel { x, y } => {
                self.scroll_x += x;
                self.scroll_y += y;
            }
            Event::Char { codepoint } => {
                self.codepoints.push(codepoint);
            }
        }
    }

    pub fn is_pressed(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_buttons[0] == ButtonState::Pressed,
            MouseButton::Middle => self.mouse_buttons[1] == ButtonState::Pressed,
            MouseButton::Right => self.mouse_buttons[2] == ButtonState::Pressed,
        }
    }
}
