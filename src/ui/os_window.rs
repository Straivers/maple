use pal::win32::{
    Foundation::*, System::Diagnostics::Debug::GetLastError,
    System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*,
};
use std::ffi::c_void;
use std::marker::PhantomPinned;

const WNDCLASS_NAME: &str = "maple_wndclass";
const MAX_TITLE_LENGTH: usize = 256;

#[derive(Default, Debug)]
pub struct OsWindow {
    pub hwnd: HWND,
    pub was_close_requested: bool,
    _pin: PhantomPinned,
}

impl OsWindow {
    /// Creates a new, immovable OS window.
    pub fn new(title: &str) -> Box<Self> {
        let mut window = Box::new(Self::default());
        create_window(title, window.as_mut() as *mut _);
        window
    }

    /// Polls the operating system for new events
    pub fn poll(&mut self) {
        // Note: Performance here is probably not great, as you have to call
        // `poll()` for every window that you have. If you want to reduce
        // latency, you may have to call this several times for every event
        // loop, which exacerbates the issue.
        let mut msg = MSG::default();
        unsafe {
            while PeekMessageW(&mut msg, self.hwnd, 0, 0, PM_REMOVE).into() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                if msg.message == WM_QUIT {
                    break;
                }
            }
        }
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

pub fn create_window(title: &str, window_data: *mut OsWindow) {
    let mut class_name = TitleConv::new(WNDCLASS_NAME);
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

    let mut w_title = TitleConv::new(title);
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
            window_data.cast::<c_void>()
        )
    };

    assert_ne!(hwnd, HWND::NULL, "Window creation failed: {:?}", unsafe {
        GetLastError()
    });

    unsafe { ShowWindow(hwnd, SW_SHOW) };
}

/// Safety:
///
/// The `wndproc` is interpreted to be a member function of `OsWindow` because
/// of the way this callback is called. That is, it is only called when
/// functions within `OsWindow` have been called.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs: &CREATESTRUCTW = std::mem::transmute(lparam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);

        let window = cs.lpCreateParams.cast::<OsWindow>();
        (*window).hwnd = hwnd;

        return LRESULT(1);
    }

    let window = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OsWindow;

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

struct TitleConv {
    buffer: [u16; MAX_TITLE_LENGTH],
}

impl TitleConv {
    fn new(s: &str) -> Self {
        let mut buffer = [0; MAX_TITLE_LENGTH];
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
