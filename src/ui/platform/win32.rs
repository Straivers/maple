use pal::win32::Windows::Win32::{
    Foundation::{HINSTANCE, HWND, WPARAM, LPARAM, PWSTR, LRESULT},
    System::{Diagnostics::Debug::GetLastError, LibraryLoader::GetModuleHandleW},
    UI::WindowsAndMessaging::{CS_HREDRAW, CS_VREDRAW, DefWindowProcW, WNDCLASSW, RegisterClassW, },
};

const WNDCLASS_NAME: &str = "maple_wndclass";

#[derive(Clone, Copy)]
pub struct Window {
    hwnd: HWND,
    user_requested_close: bool,
}

pub struct Context {}

impl Context {
    pub fn new() -> Context {
        let hmodule = unsafe { GetModuleHandleW(None) };
        assert_ne!(hmodule, HINSTANCE::NULL);

        let mut name = TitleConv::new(WNDCLASS_NAME);

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            cbClsExtra: 8,
            cbWndExtra: 8,
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

        Context {}
    }

    pub fn create_window(&self, title: &str) -> Window {
        todo!()
    }

    pub fn destroy_window(&self, window: Window) {
        todo!()
    }
}

extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

struct TitleConv {
    buffer: [u16; 256]
}

impl TitleConv {
    fn new(s: &str) -> Self {
        let mut buffer = [0; 256];
        for (i, utf16) in s.encode_utf16().enumerate() {
            buffer[i] = utf16;
        }

        buffer[buffer.len() - 1] = 0;

        TitleConv {
            buffer
        }
    }

    fn as_pwstr(&mut self) -> PWSTR {
        PWSTR(self.buffer.as_mut_ptr())
    }
}
