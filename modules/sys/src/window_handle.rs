///! Window handles specific to a particular platform.
use std::ffi::c_void;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg(target_os = "windows")]
pub struct WindowHandle {
    pub hwnd: *mut c_void,
    pub hinstance: *mut c_void,
}
