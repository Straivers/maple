use std::marker::PhantomPinned;

use pal::win32::{
    Foundation::*, System::Diagnostics::Debug::GetLastError,
    System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*,
};

const WNDCLASS_NAME: &str = "maple_wndclass";
const MAX_TITLE_LENGTH: usize = 256;

#[derive(Default, Debug, Clone, Copy)]
pub struct WindowHandle {
    index: u32,
    generation: u32,
}

impl WindowHandle {
    pub fn is_null(&self) -> bool {
        self.index == 0 && self.generation == 0
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct WindowProperties {
    hwnd: HWND,
    handle: WindowHandle,
    pub was_close_requested: bool
}

struct CreateInfo {
    wm: *mut WindowManager,
    handle: WindowHandle
}

pub struct WindowManager {
    windows: [WindowProperties; 32],
    freelist_head: u32,
    num_windows: u32,
    _pin: PhantomPinned,
}

impl WindowManager {
    pub fn new() -> Box<Self> {
        let mut class_name = TitleConv::new(WNDCLASS_NAME);
        let hmodule = unsafe { GetModuleHandleW(None) };
        assert_ne!(hmodule, HINSTANCE::NULL);

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            hInstance: hmodule,
            lpfnWndProc: Some(Self::wndproc),
            lpszClassName: class_name.as_pwstr(),
            ..WNDCLASSW::default()
        };

        let _ = unsafe { RegisterClassW(&class) };

        let mut wm = Box::new(Self{
            windows: [WindowProperties::default(); 32],
            freelist_head: u32::MAX,
            num_windows: 0,
            _pin: PhantomPinned,
        });

        for (i, window) in wm.windows.iter_mut().enumerate() {
            window.handle.index = wm.freelist_head;
            wm.freelist_head = i as u32;
        }

        // reserve (0, 0) for Handle::null
        wm.windows[0].handle.generation = 1;

        wm
    }

    pub fn is_valid(&self, window: WindowHandle) -> bool {
        !window.is_null() && self.windows[window.index as usize].handle.generation == window.generation
    }

    pub fn has_windows(&self) -> bool {
        self.num_windows > 0
    }

    pub fn create_window(&mut self, title: &str) -> WindowHandle {
        let mut class_name = TitleConv::new(WNDCLASS_NAME);
        let mut w_title = TitleConv::new(title);

        let handle = if self.freelist_head < u32::MAX {
            let index = self.freelist_head;
            let slot = &mut self.windows[index as usize].handle;

            self.freelist_head = slot.index;
            slot.index = index;

            WindowHandle {
                index,
                generation: slot.generation
            }
        }
        else {
            panic!("Too many windows!");
        };

        let mut create_info = CreateInfo {
            wm: self as *mut _,
            handle
        };

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
                &mut create_info as *mut _ as _,
            )
        };

        assert_ne!(hwnd, HWND::NULL, "Window creation failed: {:?}", unsafe {
            GetLastError()
        });

        unsafe { ShowWindow(hwnd, SW_SHOW) };

        handle
    }

    pub fn destroy_window(&mut self, window: WindowHandle) {
        if self.is_valid(window) {
            unsafe { DestroyWindow(self.windows[window.index as usize].hwnd) };
        }
    }

    pub fn get(&self, window: WindowHandle) -> Option<&WindowProperties> {
        let index = window.index as usize;

        if index < self.windows.len() && self.windows[index].handle.generation == window.generation {
            Some(&self.windows[index])
        }
        else {
            None
        }
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

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_NCCREATE {
            let cs: &CREATESTRUCTW = std::mem::transmute(lparam);
    
            let create_info = cs.lpCreateParams.cast::<CreateInfo>();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, std::mem::transmute((*create_info).handle));
            SetClassLongPtrW(hwnd, GET_CLASS_LONG_INDEX(0), (*create_info).wm as _);

            return LRESULT(1);
        }

        let handle: WindowHandle = std::mem::transmute(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
        let manager: *mut WindowManager = std::mem::transmute(GetClassLongPtrW(hwnd, GET_CLASS_LONG_INDEX(0)));
    
        if handle.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }

        let window = &mut (*manager).windows[handle.index as usize];
    
        match msg {
            WM_CREATE => {
                window.hwnd = hwnd;
                LRESULT::default()
            }
            WM_CLOSE => {
                window.was_close_requested = true;
                LRESULT::default()
            }
            WM_DESTROY => {
                window.handle.generation += 1;
                window.handle.index = (*manager).freelist_head;
                (*manager).freelist_head = handle.index;

                window.hwnd = HWND::NULL;
                LRESULT::default()
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
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
