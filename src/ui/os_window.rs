
use pal::win32::{
    Foundation::*,
    System::Diagnostics::Debug::GetLastError,
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};
use std::marker::PhantomPinned;

const WNDCLASS_NAME: &str = "maple_wndclass";
const MAX_TITLE_LENGTH: usize = 256;

#[derive(Default)]
pub struct OsWindow {
    pub hwnd: HWND,
    pub was_close_requested: bool,
    _pin: PhantomPinned
}

impl OsWindow {
    pub fn new(title: &str) -> Box<Self> {
        let mut window = Box::new(Self::default());
        create_window(title, window.as_mut() as *mut _);
        window
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
        ..Default::default()
    };

    let atom = unsafe { RegisterClassW(&class) };

    let mut w_title = TitleConv::new(title);
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
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
            window_data as _,
        )
    };

    assert_ne!(hwnd, HWND::NULL, "Window creation failed: {:?}", unsafe { GetLastError() });

    unsafe { ShowWindow(hwnd, SW_SHOW) };
}

pub fn poll_events() {
    let mut msg = MSG::default();
    unsafe  {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
    
            if msg.message == WM_QUIT {
                break;
            }
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs: &CREATESTRUCTW = std::mem::transmute(lparam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);

        let window = cs.lpCreateParams as *mut OsWindow;
        (*window).hwnd = hwnd;

        return LRESULT(1);
    }

    let window = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OsWindow;

    if window == std::ptr::null_mut() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_CLOSE => {
            (*window).was_close_requested = true;
            LRESULT::default()
        },
        WM_DESTROY => {
            (*window).hwnd = HWND::NULL;
            LRESULT::default()
        },
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
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
