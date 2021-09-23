//! Platform-abstracted window creation and management.

use crate::window_event::WindowEvent;
use crate::{platform::window as platform, window_handle::WindowHandle};

#[derive(Clone, Copy, PartialEq)]
pub enum EventLoopControl {
    /// Stops the event loop and causes it to return.
    Stop,
    /// Pauses the event loop until a user action or OS event occurs.
    Wait,
    /// Continues running the event loop in a polling fashion.
    Continue,
    /// Continues the event loop with a new update frequency.
    UpdateFreq { ticks_per_second: u32 },
    /// Continues the event loop with a new frame rate.
    RedrawFreq { frames_per_second: u32 },
}

/// A platform-dependent event loop for processing window events.
pub struct EventLoop {
    event_loop: platform::EventLoop,
}

impl EventLoop {
    pub fn new<Callback>(callback: Callback) -> Self
    where
        Callback: 'static + FnMut(&EventLoopProxy, WindowEvent) -> EventLoopControl,
    {
        Self {
            event_loop: platform::EventLoop::new(callback),
        }
    }

    pub fn num_windows(&self) -> u32 {
        self.event_loop.num_windows()
    }

    pub fn run(&mut self, updates_per_second: u32) {
        self.event_loop.run(updates_per_second);
    }

    pub fn create_window(&self, title: &str) -> WindowHandle {
        self.event_loop.create_window(title)
    }

    pub fn destroy_window(&self, window: WindowHandle) {
        self.event_loop.destroy_window(window)
    }
}

pub struct EventLoopProxy<'a> {
    pub(crate) event_loop: &'a platform::EventLoop,
}

impl<'a> EventLoopProxy<'a> {
    pub fn num_windows(&self) -> u32 {
        self.event_loop.num_windows()
    }

    pub fn create_window(&self, title: &str) -> WindowHandle {
        self.event_loop.create_window(title)
    }

    pub fn destroy_window(&self, window: WindowHandle) {
        self.event_loop.destroy_window(window)
    }
}
