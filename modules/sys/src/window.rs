use crate::dpi::PhysicalSize;
use crate::{platform::window as platform, window_handle::WindowHandle};

pub struct Window {
    window: platform::Window,
}

impl Window {
    #[must_use]
    pub fn new(event_loop: &EventLoop, title: &str) -> Self {
        Self {
            window: platform::Window::new(&event_loop.event_loop, title),
        }
    }

    /// Checks if the user requested that the window be closed (by clicking the
    /// close button).
    #[must_use]
    pub fn was_close_requested(&self) -> bool {
        self.window.was_close_requested()
    }

    #[must_use]
    pub fn handle(&self) -> WindowHandle {
        self.window.handle()
    }

    #[must_use]
    pub fn framebuffer_size(&self) -> PhysicalSize {
        self.window.framebuffer_size()
    }
}

pub struct EventLoop {
    event_loop: platform::EventLoop,
}

impl EventLoop {
    #[must_use]
    pub fn new() -> Self {
        Self {
            event_loop: platform::EventLoop::new(),
        }
    }

    pub fn poll(&mut self) {
        self.event_loop.poll();
    }
}
