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
    Redraw {},
    Update {},
}
