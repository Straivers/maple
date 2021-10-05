use std::{
    cell::{Cell, RefCell},
    convert::TryInto,
    ffi::c_void,
    time::{Duration, Instant},
};
use utils::array_vec::ArrayVec;
use win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetWindowLongPtrW, GetWindowRect,
        LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
        CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, PM_REMOVE, SW_HIDE,
        SW_SHOW, WINDOW_EX_STYLE, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_QUIT, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
        WM_MOUSEMOVE, WM_MOUSEWHEEL, WHEEL_DELTA,
    },
};

use crate::{
    dpi::PhysicalSize,
    window::{EventLoopControl, EventLoopProxy},
    window_event::{ButtonState, MouseButton, WindowEvent},
    window_handle::WindowHandle,
};

const WNDCLASS_NAME: &str = "maple_wndclass";

/// The maximum number of characters that the window title can be, in UTF-8 code
/// points including the null character required for compatibility with C.
///
/// That is to say: at most 255 characters, plus the '\0' character.
pub const MAX_TITLE_LENGTH: usize = 256;

/// Represents an event loop or message pump for retrieving input events from
/// the window manager. Events are automatically sent to the relevant window,
/// where they may be queried.
///
// Impl Note: This is a great place to stash anything that is shared between
// windows.
pub(crate) struct EventLoop {
    // So that we get /* fields omitted */ in the docs
    callback: Box<RefCell<dyn FnMut(&EventLoopProxy, WindowEvent) -> EventLoopControl>>,
    hinstance: HINSTANCE,
    class_name: ArrayVec<u16, 16>,
    num_windows: Cell<u32>,
    control: Cell<EventLoopControl>,
    destroy_queue: RefCell<Vec<WindowHandle>>,
}

impl EventLoop {
    pub fn new<Callback>(callback: Callback) -> Self
    where
        Callback: 'static + FnMut(&EventLoopProxy, WindowEvent) -> EventLoopControl,
    {
        let mut class_name = to_wstr::<16>(WNDCLASS_NAME);

        let hinstance = unsafe { GetModuleHandleW(None) };
        assert_ne!(hinstance, HINSTANCE::NULL);
        let cursor = unsafe { LoadCursorW(None, &IDC_ARROW) };

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            hInstance: hinstance,
            lpfnWndProc: Some(Self::wndproc_trampoline),
            lpszClassName: PWSTR(class_name.as_mut_ptr()),
            hCursor: cursor,
            ..WNDCLASSW::default()
        };

        let _ = unsafe { RegisterClassW(&class) };

        Self {
            callback: Box::new(RefCell::new(callback)),
            hinstance,
            class_name,
            num_windows: Cell::new(0),
            control: Cell::new(EventLoopControl::Continue),
            destroy_queue: RefCell::new(Vec::new()),
        }
    }

    pub fn num_windows(&self) -> u32 {
        self.num_windows.get()
    }

    /// Runs the event loop continuously.
    pub fn run(&mut self, updates_per_second: u32) {
        let msecs_per_tick = Duration::from_secs(1) / updates_per_second;

        let mut previous = Instant::now();
        let mut tick_lag = Duration::ZERO;

        let mut msg = MSG::default();
        while self.control.get() != EventLoopControl::Stop {
            let current = Instant::now();
            let elapsed = current - previous;
            previous = current;

            tick_lag += elapsed;

            for window in self.destroy_queue.borrow_mut().drain(0..) {
                unsafe { DestroyWindow(HWND(window.hwnd as _)) };
            }

            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);

                    if msg.message == WM_QUIT {
                        break;
                    }
                }
            }

            while tick_lag >= msecs_per_tick {
                self.callback.borrow_mut()(&self.proxy(), WindowEvent::Update {});
                tick_lag -= msecs_per_tick;
            }

            self.callback.borrow_mut()(&self.proxy(), WindowEvent::Redraw {});
        }
    }

    pub fn create_window(&self, title: &str) -> WindowHandle {
        let mut w_title = to_wstr::<MAX_TITLE_LENGTH>(title);

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PWSTR(self.class_name.as_ptr() as *mut _),
                PWSTR(w_title.as_mut_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                GetModuleHandleW(None),
                (self as *const Self) as *mut c_void,
            )
        };

        unsafe { ShowWindow(hwnd, SW_SHOW) };

        self.num_windows.set(self.num_windows.get() + 1);

        WindowHandle {
            hwnd: hwnd.0 as _,
            hinstance: self.hinstance.0 as _,
        }
    }

    pub fn destroy_window(&self, window: WindowHandle) {
        unsafe { ShowWindow(HWND(window.hwnd as _), SW_HIDE) };
        self.destroy_queue.borrow_mut().push(window);

        assert!(self.num_windows.get() > 0);
        self.num_windows.set(self.num_windows.get() - 1);
    }

    fn proxy(&self) -> EventLoopProxy {
        EventLoopProxy { event_loop: self }
    }

    fn dispatch(&self, event: WindowEvent) {
        self.control.set(self.callback.borrow_mut()(&self.proxy(), event));
    }

    /// The default-unsafe wndproc callback. Event handling is forwarded to the
    /// default-safe `wndproc()`.
    #[allow(clippy::similar_names)]
    unsafe extern "system" fn wndproc_trampoline(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_CREATE {
            let cs: &CREATESTRUCTW = std::mem::transmute(lparam);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);
        }

        let event_loop_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Self;

        if event_loop_ptr.is_null() {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        } else {
            Self::wndproc(&*event_loop_ptr, hwnd, msg, wparam, lparam)
        }
    }

    /// The default-safe wndproc. It can be assumed that the `window` parameter
    /// is valid and points to the same window as `hwnd`.
    fn wndproc(event_loop: &Self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let window_handle = WindowHandle {
            hwnd: hwnd.0 as _,
            hinstance: event_loop.hinstance.0 as _,
        };

        match msg {
            WM_CLOSE => {
                event_loop.dispatch(WindowEvent::CloseRequested { window: window_handle });
            }
            WM_SIZE => {
                // LOWORD and HIWORD (i16s for historical reasons, I guess)
                let width = (lparam.0 as i16)
                    .try_into()
                    .expect("Window width is negative or > 65535");
                let height = (lparam.0 >> i16::BITS)
                    .try_into()
                    .expect("Window height is negative or > 65535");

                // We need to guard against an empty callback
                event_loop.dispatch(WindowEvent::Resized {
                    window: window_handle,
                    size: PhysicalSize { width, height },
                });
            }
            WM_CREATE => {
                let mut rect = RECT::default();
                unsafe { GetWindowRect(hwnd, &mut rect) };

                let width = (rect.right - rect.left)
                    .try_into()
                    .expect("Window width is negative or > 65535");
                let height = (rect.bottom - rect.top)
                    .try_into()
                    .expect("Window heigth is negative or > 65535");

                event_loop.dispatch(WindowEvent::Created {
                    window: window_handle,
                    size: PhysicalSize { width, height },
                });
            }
            WM_DESTROY => {
                if event_loop.num_windows() == 0 {
                    unsafe { PostQuitMessage(0) };
                }
            }
            WM_ERASEBKGND => {
                /* No op, as recommended here:
                  https://stackoverflow.com/questions/53000291/how-to-smooth-ugly-jitter-flicker-jumping-when-resizing-windows-especially-drag
                */
                return LRESULT(1);
            }
            WM_LBUTTONDOWN => {
                event_loop.dispatch(WindowEvent::MouseButton {
                    window: window_handle,
                    button: MouseButton::Left,
                    state: ButtonState::Pressed,
                });
            }
            WM_LBUTTONUP => {
                event_loop.dispatch(WindowEvent::MouseButton {
                    window: window_handle,
                    button: MouseButton::Left,
                    state: ButtonState::Released,
                });
            }
            WM_MBUTTONDOWN => {
                event_loop.dispatch(WindowEvent::MouseButton {
                    window: window_handle,
                    button: MouseButton::Middle,
                    state: ButtonState::Pressed,
                });
            }
            WM_MBUTTONUP => {
                event_loop.dispatch(WindowEvent::MouseButton {
                    window: window_handle,
                    button: MouseButton::Middle,
                    state: ButtonState::Released,
                });
            }
            WM_RBUTTONDOWN => {
                event_loop.dispatch(WindowEvent::MouseButton {
                    window: window_handle,
                    button: MouseButton::Right,
                    state: ButtonState::Pressed,
                });
            }
            WM_RBUTTONUP => {
                event_loop.dispatch(WindowEvent::MouseButton {
                    window: window_handle,
                    button: MouseButton::Right,
                    state: ButtonState::Released,
                });
            }
            WM_MOUSEMOVE => {
                let x = lparam.0 as i16;
                let y = (lparam.0 >> 16) as i16;
                event_loop.dispatch(WindowEvent::MouseMove {
                    window: window_handle,
                    x,
                    y,
                })
            }
            WM_MOUSEWHEEL => {
                event_loop.dispatch(WindowEvent::MouseWheel {
                    window: window_handle,
                    delta: (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32)
                })
            }
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
        LRESULT::default()
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        for window in self.destroy_queue.get_mut().drain(0..) {
            unsafe { DestroyWindow(HWND(window.hwnd as _)) };
        }
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
