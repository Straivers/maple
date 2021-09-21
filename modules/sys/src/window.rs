//! Platform-abstracted window creation and management.

use std::ops::{Deref, DerefMut};

use crate::dpi::PhysicalSize;
use crate::{platform::window as platform, window_handle::WindowHandle};

/// A platform-specific graphical window.
///
/// Processing window events depends on calling `EventLoop::poll()`. You should
/// poll for events at least once per frame.
pub struct Window<UserData> {
    window: platform::Window<UserData>,
}

impl<UserData> Window<UserData> {
    #[must_use]
    pub fn new(event_loop: &EventLoop<UserData>, title: &str, user_data: UserData) -> Self {
        Self {
            window: platform::Window::<UserData>::new(&event_loop.event_loop, title, user_data),
        }
    }

    /// Gets a non-owning const reference to the window.
    #[must_use]
    pub fn get(&self) -> WindowRef<UserData> {
        WindowRef {
            window: self.window.get(),
        }
    }

    /// Checks if the user requested that the window be closed (by clicking the
    /// close button).
    #[must_use]
    pub fn was_close_requested(&self) -> bool {
        self.window.was_close_requested()
    }

    /// Gets a platform-specific handle to the window.
    #[must_use]
    pub fn handle(&self) -> WindowHandle {
        self.window.handle()
    }

    #[must_use]
    pub fn data_mut(&self) -> UserDataRefMut<UserData> {
        UserDataRefMut {
            user_data_ref: self.window.data_mut(),
        }
    }

    /// Gets the size of the window in pixels.
    #[must_use]
    pub fn framebuffer_size(&self) -> PhysicalSize {
        self.window.framebuffer_size()
    }
}

pub struct UserDataRefMut<'a, UserData> {
    user_data_ref: platform::UserDataRefMut<'a, UserData>,
}

impl<'a, UserData> Deref for UserDataRefMut<'a, UserData> {
    type Target = UserData;

    fn deref(&self) -> &Self::Target {
        &self.user_data_ref
    }
}

impl<'a, UserData> DerefMut for UserDataRefMut<'a, UserData> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.user_data_ref
    }
}

#[derive(Debug, Clone)]
/// A non-owning const, possibly invalid const reference to a window.
pub struct WindowRef<UserData> {
    window: platform::WindowRef<UserData>,
}

impl<UserData> WindowRef<UserData> {
    /// Checks if this reference is still valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.window.is_valid()
    }

    /// Checks if the user requested that the window be closed (by Alt+F4,
    /// clicking the close button, etc.).
    ///
    /// Returns `None` if the window reference is no longer valid.
    #[must_use]
    pub fn was_close_requested(&self) -> Option<bool> {
        self.window.was_close_requested()
    }

    /// Gets the handle to the window.
    ///
    /// Returns `None` if the window reference is no longer valid.
    #[must_use]
    pub fn handle(&self) -> Option<WindowHandle> {
        self.window.handle()
    }

    /// Gets the size of the window's framebuffer in pixels.
    ///
    /// Returns `None` if the window reference is no longer valid.
    #[must_use]
    pub fn framebuffer_size(&self) -> Option<PhysicalSize> {
        self.window.framebuffer_size()
    }
}

/// A platform-dependent event loop for processing window events.
pub struct EventLoop<UserData> {
    event_loop: platform::EventLoop<UserData>,
}

impl<UserData> EventLoop<UserData> {
    /// Creates a new event loop
    #[must_use]
    pub fn new() -> Self {
        Self {
            event_loop: platform::EventLoop::new(),
        }
    }

    /// Polls the event loop for window events. These events will be reflected
    /// in the window they were sent to.
    pub fn poll(&mut self) {
        self.event_loop.poll();
    }
}
