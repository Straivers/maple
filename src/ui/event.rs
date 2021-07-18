use super::window::WindowHandle;

pub enum Event {
    Draw,
    WindowEvent {
        window: WindowHandle,
        event: WindowEvent,
    },
}

pub enum WindowEvent {
    Resize,
    ModeChange { width: i16, height: i16 },
    Destroy,
}
