use crate::{dpi::PhysicalSize, window_handle::WindowHandle};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Unknown,
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Unknown,
    Pressed,
    Released,
    Repeated(u8),
}

#[test]
fn button_state_size() {
    assert_eq!(std::mem::size_of::<ButtonState>(), std::mem::size_of::<u8>() * 2);
}

#[derive(Debug, Clone, Copy)]
pub enum WindowEvent {
    Created {
        window: WindowHandle,
        size: PhysicalSize,
    },
    Destroyed {
        window: WindowHandle,
    },
    CloseRequested {
        window: WindowHandle,
    },
    Resized {
        window: WindowHandle,
        size: PhysicalSize,
    },
    MouseButton {
        window: WindowHandle,
        button: MouseButton,
        state: ButtonState,
    },
    MouseMove {
        window: WindowHandle,
        x: i16,
        y: i16,
    },
    MouseWheel {
        window: WindowHandle,
        /// The number of lines scrolled. Negative for towards user, positive
        /// for away from user; may be less than `abs(1)`.
        delta: f32,
    },
    Update {},
}
