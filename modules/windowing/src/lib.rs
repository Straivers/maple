use pal::win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM},
    System::{Diagnostics::Debug::GetLastError, LibraryLoader::GetModuleHandleW},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetWindowLongPtrW,
        PeekMessageW, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW,
        CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, MSG, PM_REMOVE, SW_SHOW, WINDOW_EX_STYLE, WM_CLOSE, WM_CREATE,
        WM_DESTROY, WM_QUIT, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    },
};
use std::{cell::RefCell, convert::TryInto, ffi::c_void, marker::PhantomPinned};
use utils::array_vec::ArrayVec;

const WNDCLASS_NAME: &str = "maple_wndclass";

/// The maximum number of characters that the window title can be, in UTF-8 code
/// points including the null character required for compatibility with C.
///
/// That is to say: at most 255 characters, plus the '\0' character.
pub const MAX_TITLE_LENGTH: usize = 256;

#[derive(Default, Debug)]
struct WindowData {
    hwnd: HWND,
    was_close_requested: bool,
    _pin: PhantomPinned,
}

pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

/// A window created by the operating system's window manager. The OS window will
/// be destroyed automatically when the structure is dropped.
///
/// Note, however, that some window activities such as processing the close
/// button, minimizing, or resizing require that the `EventLoop` be polled
/// frequently.
#[derive(Debug)]
pub struct Window {
    window_data: Box<RefCell<WindowData>>,
}

impl Window {
    /// Creates a new window and associates it with the event loop.
    #[must_use]
    pub fn new(_: &EventLoop, title: &str) -> Self {
        // let mut window = Box::new(Self::default());
        let mut window_data = Box::new(RefCell::new(WindowData {
            hwnd: HWND::NULL,
            was_close_requested: false,
            _pin: PhantomPinned,
        }));

        let mut class_name = to_wstr::<16>(WNDCLASS_NAME);

        let hmodule = unsafe { GetModuleHandleW(None) };
        assert_ne!(hmodule, HINSTANCE::NULL);

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            hInstance: hmodule,
            lpfnWndProc: Some(wndproc),
            lpszClassName: PWSTR(class_name.as_mut_ptr()),
            ..WNDCLASSW::default()
        };

        let _ = unsafe { RegisterClassW(&class) };

        let mut w_title = to_wstr::<MAX_TITLE_LENGTH>(title);

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PWSTR(class_name.as_mut_ptr()),
                PWSTR(w_title.as_mut_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                GetModuleHandleW(None),
                (window_data.as_mut() as *mut RefCell<_>).cast::<c_void>(),
            )
        };

        unsafe { ShowWindow(hwnd, SW_SHOW) };

        Window { window_data }
    }

    /// Checks if the user requested that the window be closed (by clicking the
    /// close button).
    #[must_use]
    pub fn was_close_requested(&self) -> bool {
        self.window_data.borrow().was_close_requested
    }

    #[cfg(target_os = "windows")]
    #[must_use]
    pub fn get_hwnd(&self) -> HWND {
        self.window_data.borrow().hwnd
    }

    #[must_use]
    pub fn framebuffer_size(&self) -> WindowSize {
        let mut rect = RECT::default();

        if unsafe { GetClientRect(self.window_data.borrow().hwnd, &mut rect) } == false {
            panic!("Unable to retrieve framebuffer size: {:?}", unsafe { GetLastError() });
        }

        WindowSize {
            width: (rect.right - rect.left)
                .try_into()
                .expect("Unrepresentable framebuffer size"),
            height: (rect.bottom - rect.top)
                .try_into()
                .expect("Unrepresentable framebuffer size"),
        }
    }
}

impl Drop for Window {
    /// Destroys the window
    fn drop(&mut self) {
        let hwnd = { self.window_data.borrow().hwnd };
        unsafe { DestroyWindow(hwnd) };
    }
}

/// Represents an event loop or message pump for retrieving input events from
/// the window manager. Events are automatically sent to the relevant window,
/// where they may be queried.
///
// Impl Note: This is a great place to stash anything that is shared between
// windows.
pub struct EventLoop {
    // So that we get /* fields omitted */ in the docs
    #[doc(hidden)]
    _empty: u8,
}

impl EventLoop {
    /// Creates a new event loop
    pub fn new() -> Self {
        Self { _empty: 0 }
    }

    /// Polls the operating system for input and window events. The events will
    /// be processed and reflected in their respective window objects when this
    /// call is complete. Call this at least once per frame to ensure responsiveness.
    ///
    /// Make sure to call this on the same thread that the OS windows were
    /// created.
    #[allow(clippy::unused_self)]
    pub fn poll(&mut self) {
        // Note: Performance here is probably not great, as you have to call
        // `poll()` for every window that you have. If you want to reduce
        // latency, you may have to call this several times for every event
        // loop, which exacerbates the issue.
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

#[allow(clippy::similar_names)]
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CREATE {
        let cs: &CREATESTRUCTW = std::mem::transmute(lparam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);

        let window = cs.lpCreateParams.cast::<RefCell<WindowData>>();
        (*window).borrow_mut().hwnd = hwnd;

        return LRESULT::default();
    }

    let window = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RefCell<WindowData>;

    if window.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_CLOSE => {
            (*window).borrow_mut().was_close_requested = true;
            LRESULT::default()
        }
        WM_DESTROY => {
            (*window).borrow_mut().hwnd = HWND::NULL;
            LRESULT::default()
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn to_wstr<const MAX_LENGTH: usize>(s: &str) -> ArrayVec<u16, MAX_LENGTH> {
    assert!(MAX_LENGTH > 0);

    let mut buffer = s.encode_utf16().collect::<ArrayVec<_, MAX_LENGTH>>();
    let len = buffer.len();

    if len == buffer.capacity() {
        buffer[len - 1] = 0;
    } else {
        buffer.push(0);
    }

    buffer
}
