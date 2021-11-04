use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Unknown = 0,
    Left = 1,
    Middle = 2,
    Right = 3,
}

impl Default for MouseButton {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Index<MouseButton> for [MouseButtonState] {
    type Output = MouseButtonState;

    fn index(&self, index: MouseButton) -> &Self::Output {
        match index {
            MouseButton::Unknown => &self[0],
            MouseButton::Left => &self[1],
            MouseButton::Middle => &self[2],
            MouseButton::Right => &self[3],
        }
    }
}

impl IndexMut<MouseButton> for [MouseButtonState] {
    fn index_mut(&mut self, index: MouseButton) -> &mut Self::Output {
        match index {
            MouseButton::Unknown => &mut self[0],
            MouseButton::Left => &mut self[1],
            MouseButton::Middle => &mut self[2],
            MouseButton::Right => &mut self[3],
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Released = 0,
    Pressed = 1,
    Held = 2,
    DoublePress = 3,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Default, Clone, Copy)]
pub struct MouseButtonState {
    // bits 0-3 store MouseButton, bits 4-7 store MouseButtonState
    value: u8,
}

impl MouseButtonState {
    pub fn new(button: MouseButton) -> Self {
        let button = button as u8;
        debug_assert!(button < (1 << 4));

        Self { value: button }
    }

    pub fn button(&self) -> MouseButton {
        unsafe { std::mem::transmute(self.value & 0b1111) }
    }

    pub fn state(&self) -> ButtonState {
        unsafe { std::mem::transmute(self.value >> 4) }
    }

    pub fn set(&mut self, state: ButtonState) {
        debug_assert!((state as u8) < (1 << 4));
        self.value = (self.value & 0b1111) | ((state as u8) << 4);
    }
}

impl Debug for MouseButtonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MouseButtonState")
            .field("button", &self.button())
            .field("state", &self.state())
            .finish()
    }
}

#[derive(Debug)]
pub struct FrameInput {
    // State that's preserved between frames.
    pub cursor_x: i16,
    pub cursor_y: i16,
    pub mouse_buttons: [MouseButtonState; 4],

    // State that's reset every frame.
    pub cursor_wheel_x: f32,
    pub cursor_wheel_y: f32,
}

impl FrameInput {
    pub fn new() -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            mouse_buttons: [
                MouseButtonState::new(MouseButton::Unknown),
                MouseButtonState::new(MouseButton::Left),
                MouseButtonState::new(MouseButton::Middle),
                MouseButtonState::new(MouseButton::Right),
            ],
            cursor_wheel_x: 0.0,
            cursor_wheel_y: 0.0,
        }
    }

    pub fn advance(&mut self) {
        self.cursor_wheel_x = 0.0;
        self.cursor_wheel_y = 0.0;
    }
}
