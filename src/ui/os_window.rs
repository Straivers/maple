use pal::win32::{
    Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*,
};
use std::marker::PhantomPinned;

const WNDCLASS_NAME: &str = "maple_wndclass";

const MAX_TITLE_LENGTH: usize = 256;

#[derive(Default, Debug)]
pub struct Window {
    hwnd: HWND,
    was_close_requested: bool,
    _pin: PhantomPinned,
}

impl Window {
    pub fn new(_: &EventLoop, title: &str) -> Box<Self> {
        let mut window = Box::new(Self::default());

        // Should be TitleConv::<WNDCLASS_NAME.len() + 1>::new() but that's
        // not supported by the compiler yet.
        let mut class_name = TitleConv::<16>::new(WNDCLASS_NAME);

        let hmodule = unsafe { GetModuleHandleW(None) };
        assert_ne!(hmodule, HINSTANCE::NULL);

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            hInstance: hmodule,
            lpfnWndProc: Some(wndproc),
            lpszClassName: class_name.as_pwstr(),
            ..WNDCLASSW::default()
        };

        let _ = unsafe { RegisterClassW(&class) };

        let mut w_title = TitleConv::<MAX_TITLE_LENGTH>::new(title);
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name.as_pwstr(),
                w_title.as_pwstr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                GetModuleHandleW(None),
                window.as_mut() as *mut _ as _,
            )
        };

        unsafe { ShowWindow(hwnd, SW_SHOW) };

        window
    }

    pub fn was_close_requested(&self) -> bool {
        self.was_close_requested
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

pub struct EventLoop {}

impl EventLoop {
    pub fn new() -> Self {
        Self {}
    }

    pub fn poll(&mut self) {
        let mut msg = MSG::default();
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                if msg.message == WM_QUIT {
                    break;
                }
            }
        }
    }
}

/// Safety:
///
/// The `wndproc` is interpreted to be a member function of `Window` because
/// of the way this callback is called. That is, it is only called when
/// functions within `Window` have been called.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs: &CREATESTRUCTW = std::mem::transmute(lparam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);

        let window = cs.lpCreateParams.cast::<Window>();
        (*window).hwnd = hwnd;

        return LRESULT(1);
    }

    let window = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Window;

    if window.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_CLOSE => {
            (*window).was_close_requested = true;
            LRESULT::default()
        }
        WM_DESTROY => {
            (*window).hwnd = HWND::NULL;
            LRESULT::default()
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

struct TitleConv<const CAPACITY: usize> {
    buffer: [u16; CAPACITY],
}

impl<const CAPACITY: usize> TitleConv<CAPACITY> {
    fn new(s: &str) -> Self {
        let mut buffer = [0; CAPACITY];
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
