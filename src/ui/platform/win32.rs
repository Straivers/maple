use crate::ui::window::WindowHandle;
use crate::ui::event::*;
use pal::win32::Windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, WPARAM},
    System::{Diagnostics::Debug::GetLastError, LibraryLoader::GetModuleHandleW},
    UI::WindowsAndMessaging::*,
};
use std::convert::TryInto;
use std::marker::PhantomPinned;

const WNDCLASS_NAME: &str = "maple_wndclass";

type Callback = dyn FnMut(&mut WindowControl, Event);

#[derive(Default, Clone, Copy)]
pub struct Window {
    hwnd: HWND,
    pub user_requested_close: bool,
    _pin: PhantomPinned,
}

pub struct WindowManager {
    callback: Option<Box<Callback>>,
    control: WindowControl,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            callback: None,
            control: WindowControl::new(),
        }
    }

    pub fn create_window(&mut self, title: &str) -> WindowHandle {
        let (hwnd, handle) = self.control.create_window(title);

        unsafe {
            SetClassLongPtrW(hwnd, GET_CLASS_LONG_INDEX(0), self as *mut _ as _)
        };

        handle
    }

    pub fn destroy_window(&mut self, handle: WindowHandle) {
        self.control.destroy_window(handle)
    }

    pub fn run<F>(&mut self, callback: F)
    where
        F: 'static + FnMut(&mut WindowControl, Event),
    {
        self.callback = Some(Box::new(callback));
        self.control.run(self.callback.unwrap().as_mut());
    }
}

pub struct WindowControl {
    handles: [WindowHandle; 16],
    windows: [Window; 16],
    num_windows: u32,
    _pin: PhantomPinned,
}

impl WindowControl {
    pub fn new() -> Self {
        let mut name = TitleConv::new(WNDCLASS_NAME);
        let hmodule = unsafe { GetModuleHandleW(None) };
        assert_ne!(hmodule, HINSTANCE::NULL);

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            cbClsExtra: std::mem::size_of::<*const i32>().try_into().unwrap(),
            cbWndExtra: std::mem::size_of::<*const i32>().try_into().unwrap(),
            hInstance: hmodule,
            lpfnWndProc: Some(wndproc),
            lpszClassName: name.as_pwstr(),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&class) };
        assert_ne!(
            atom,
            0,
            "Registration of window class failed: {:?}",
            unsafe { GetLastError() }
        );

        Self {
            handles: [Default::default(); 16],
            windows: [Default::default(); 16],
            num_windows: 0,
            _pin: PhantomPinned,
        }
    }

    pub fn create_window(&mut self, title: &str) -> (HWND, WindowHandle) {
        let mut name = TitleConv::new(title);
        let mut class_name = TitleConv::new(WNDCLASS_NAME);
        let hmodule = unsafe { GetModuleHandleW(None) };
        assert_ne!(hmodule, HINSTANCE::NULL);

        let handle = {
            let mut index = None;
            for (i, window) in self.windows.iter().enumerate() {
                if window.hwnd == HWND::NULL {
                    index = Some(i);
                    break;
                }
            }

            if let Some(i) = index {
                let h = &mut self.handles[i];
                if h.is_null() {
                    *h = WindowHandle::new_index(i).unwrap()
                }
                *h
            } else {
                WindowHandle::null()
            }
        };

        self.windows[handle.index()] = Window {
            hwnd: HWND::NULL,
            user_requested_close: false,
            _pin: PhantomPinned,
        };

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name.as_pwstr(),
                name.as_pwstr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                hmodule,
                handle.raw_value() as usize as _,
            )
        };

        self.num_windows += 1;

        assert_ne!(hwnd, HWND::NULL, "Failed  to create window: {:?}", unsafe {
            GetLastError()
        });

        unsafe { ShowWindow(hwnd, SW_SHOW) };

        (hwnd, handle)
    }

    pub fn destroy_window(&mut self, handle: WindowHandle) {
        let h = &mut self.handles[handle.index()];
        if h.generation() == handle.generation() {
            h.inc_generation().unwrap();

            let w = &mut self.windows[handle.index()];
            unsafe { DestroyWindow(w.hwnd) };
            *w = Default::default();
        }
    }

    pub fn run(&mut self, callback: &mut Callback) {
        // Note: callback needs to be a call type to be safe
        let mut msg = MSG::default();
        loop {
            unsafe {
                if !WaitMessage().as_bool() {
                    panic!("Win32 API Waitmessage() failed: {:?}", GetLastError());
                }

                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                    if msg.message == WM_QUIT {
                        return;
                    }

                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                callback(self, Event::Draw);
            }
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam.0 as *const CREATESTRUCTW;
        unsafe {
            let raw_handle = (*cs).lpCreateParams as usize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_handle as _);
        }

        return LRESULT::default();
    }

    let handle =
    unsafe { WindowHandle::from_raw_value(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u32) };
    let manager = unsafe { GetClassLongPtrW(hwnd, GET_CLASS_LONG_INDEX(0)) as *mut WindowManager };

    if unsafe { (*manager).callback.is_none() } {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let callback = unsafe { (*manager).callback.as_ref().unwrap() };

    match msg {
        WM_CLOSE => {
            unsafe { (*manager).control.windows[handle.index()].user_requested_close = true };
            LRESULT::default()
        }
        WM_DESTROY => {
            unsafe {
                (*manager).control.num_windows -= 1;
                if (*manager).control.num_windows == 0 {
                    PostQuitMessage(0);
                }
            }

            LRESULT::default()
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

struct TitleConv {
    buffer: [u16; 256],
}

impl TitleConv {
    fn new(s: &str) -> Self {
        let mut buffer = [0; 256];
        for (i, utf16) in s.encode_utf16().enumerate() {
            buffer[i] = utf16;
        }

        buffer[buffer.len() - 1] = 0;

        TitleConv { buffer }
    }

    fn as_pwstr(&mut self) -> PWSTR {
        PWSTR(self.buffer.as_mut_ptr())
    }
}
