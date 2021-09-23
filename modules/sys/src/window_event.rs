use crate::{dpi::PhysicalSize, window_handle::WindowHandle};

pub enum WindowEvent {
    Created { window: WindowHandle, size: PhysicalSize },
    CloseRequested { window: WindowHandle },
    Resized { window: WindowHandle, size: PhysicalSize },
    Redraw {},
    Update {},
}
