use std::{convert::TryInto, sync::Once};

use win32::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetLastError, GetMessageW, GetModuleHandleW,
    GetWindowLongPtrW, GetWindowRect, LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW,
    ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HINSTANCE, HWND,
    IDC_ARROW, LPARAM, LRESULT, MSG, PM_REMOVE, PWSTR, RECT, SW_SHOW, WHEEL_DELTA, WINDOW_EX_STYLE, WM_CHAR, WM_CLOSE,
    WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WNDCLASSW, WPARAM,
    WS_OVERLAPPEDWINDOW, WM_PAINT,
};

use super::{
    dpi::PhysicalSize,
    input::{ButtonState, MouseButton},
};
use crate::array_vec::ArrayVec;

const WNDCLASS_NAME: &str = "maple_wndclass";

/// The maximum number of bytes that the window title can be, in UTF-8 code
/// points including the null character required for compatibility with C.
///
/// That is to say: at most 255 bytes, plus the '\0' character.
pub const MAX_TITLE_LENGTH: usize = 256;

static REGISTER_CLASS: Once = Once::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(target_os = "windows")]
pub struct WindowHandle {
    pub hwnd: HWND,
    pub hinstance: HINSTANCE,
}

pub struct WindowControl {
    handle: WindowHandle,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowEvent {
    Created { size: PhysicalSize },
    Destroyed {},
    CloseRequested {},
    Resized { size: PhysicalSize },
    Update {},
    CursorMove { x_pos: i16, y_pos: i16 },
    MouseButton { button: MouseButton, state: ButtonState },
    ScrollWheel { scroll_x: f32, scroll_y: f32 },
    Char { codepoint: char },
}

impl WindowControl {
    pub fn destroy(&self) {
        unsafe {
            DestroyWindow(self.handle.hwnd);
        }
    }

    pub fn handle(&self) -> &WindowHandle {
        &self.handle
    }
}

#[derive(Debug, PartialEq)]
pub enum EventLoopControl {
    Continue,
    Stop,
}

pub fn window<Callback>(title: String, callback: Callback)
where
    Callback: FnMut(&WindowControl, WindowEvent) -> EventLoopControl,
{
    let mut class_name = to_wstr::<16>(WNDCLASS_NAME);

    let hinstance = unsafe { GetModuleHandleW(None) };
    assert_ne!(hinstance, HINSTANCE::default());

    REGISTER_CLASS.call_once(|| {
        let cursor = unsafe { LoadCursorW(None, &IDC_ARROW) };

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            hInstance: hinstance,
            lpfnWndProc: Some(wndproc_trampoline),
            lpszClassName: PWSTR(class_name.as_mut_ptr()),
            hCursor: cursor,
            ..WNDCLASSW::default()
        };

        let _ = unsafe { RegisterClassW(&class) };
    });

    let hwnd = {
        let mut w_title = to_wstr::<MAX_TITLE_LENGTH>(&title);
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PWSTR(class_name.as_ptr() as *mut _),
                PWSTR(w_title.as_mut_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                GetModuleHandleW(None),
                std::ptr::null_mut(),
            )
        }
    };

    let mut window = Window {
        callback,
        control: WindowControl {
            handle: WindowHandle { hwnd, hinstance },
        },
        high_surrogate: 0,
    };

    {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect) };

        let width = (rect.right - rect.left)
            .try_into()
            .expect("Window width is negative or > 65535");
        let height = (rect.bottom - rect.top)
            .try_into()
            .expect("Window heigth is negative or > 65535");
        window.dispatch(WindowEvent::Created {
            size: PhysicalSize { width, height },
        });
    }

    // Is there a way to make sure both window_trait and window_trait_ptr are on
    // the same cache line? Fat pointer
    let mut window_trait: &mut dyn WindowT = &mut window;
    // Pointer to fat pointer
    let window_trait_ptr: *mut &mut dyn WindowT = &mut window_trait;

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_trait_ptr as _);
        ShowWindow(hwnd, SW_SHOW);
    }

    let mut msg = MSG::default();
    unsafe {
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0).0;
            if ret == -1 {
                panic!("GetMessage failed. Error: {:?}", GetLastError());
            } else if ret == 0 {
                break;
            } else {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                if msg.message == WM_QUIT {
                    DestroyWindow(hwnd);
                    return;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

struct Window<Callback>
where
    Callback: FnMut(&WindowControl, WindowEvent) -> EventLoopControl,
{
    callback: Callback,
    control: WindowControl,
    high_surrogate: u16,
}

impl<Callback> WindowT for Window<Callback>
where
    Callback: FnMut(&WindowControl, WindowEvent) -> EventLoopControl,
{
    fn handle(&self) -> WindowHandle {
        self.control.handle
    }

    fn dispatch(&mut self, event: WindowEvent) {
        let op = (self.callback)(&self.control, event);

        if op == EventLoopControl::Stop {
            unsafe { PostQuitMessage(0) };
        }
    }

    fn save_high_surrogate(&mut self, value: u16) {
        self.high_surrogate = value;
    }

    fn take_high_surrogate(&mut self) -> u16 {
        std::mem::take(&mut self.high_surrogate)
    }
}

trait WindowT {
    fn handle(&self) -> WindowHandle;

    fn dispatch(&mut self, event: WindowEvent);

    fn save_high_surrogate(&mut self, value: u16);

    fn take_high_surrogate(&mut self) -> u16;
}

unsafe extern "system" fn wndproc_trampoline(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Pointer to fat pointer
    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut &mut dyn WindowT;

    if window_ptr.is_null() {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    } else {
        // Reference to fat pointer
        let window = &mut *window_ptr;

        match msg {
            WM_CREATE => {
                let createstruct = &(*(lparam.0 as *const CREATESTRUCTW));
                let width = createstruct.cx.try_into().expect("Window width out of bounds!");
                let height = createstruct.cy.try_into().expect("Window height out of bounds!");
                window.dispatch(WindowEvent::Created {
                    size: PhysicalSize { width, height },
                });
            }
            WM_CLOSE => {
                window.dispatch(WindowEvent::CloseRequested {});
            }
            WM_DESTROY => {
                window.dispatch(WindowEvent::Destroyed {});
            }
            WM_SIZE => {
                // LOWORD and HIWORD (i16s for historical reasons, I guess)
                let width = (lparam.0 as i16)
                    .try_into()
                    .expect("Window width is negative or > 65535");
                let height = (lparam.0 >> i16::BITS)
                    .try_into()
                    .expect("Window height is negative or > 65535");

                window.dispatch(WindowEvent::Resized {
                    size: PhysicalSize { width, height },
                });
            }
            WM_ERASEBKGND => {
                /* No op, as recommended here:
                  https://stackoverflow.com/questions/53000291/how-to-smooth-ugly-jitter-flicker-jumping-when-resizing-windows-especially-drag
                */
                return LRESULT(1);
            }
            WM_MOUSEMOVE => window.dispatch(WindowEvent::CursorMove {
                x_pos: lparam.0 as i16,
                y_pos: (lparam.0 >> 16) as i16,
            }),
            WM_LBUTTONDOWN => window.dispatch(WindowEvent::MouseButton {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
            }),
            WM_LBUTTONUP => window.dispatch(WindowEvent::MouseButton {
                button: MouseButton::Left,
                state: ButtonState::Released,
            }),
            WM_MBUTTONDOWN => window.dispatch(WindowEvent::MouseButton {
                button: MouseButton::Middle,
                state: ButtonState::Pressed,
            }),
            WM_MBUTTONUP => window.dispatch(WindowEvent::MouseButton {
                button: MouseButton::Middle,
                state: ButtonState::Released,
            }),
            WM_RBUTTONDOWN => window.dispatch(WindowEvent::MouseButton {
                button: MouseButton::Right,
                state: ButtonState::Pressed,
            }),
            WM_RBUTTONUP => window.dispatch(WindowEvent::MouseButton {
                button: MouseButton::Right,
                state: ButtonState::Released,
            }),
            WM_MOUSEWHEEL => window.dispatch(WindowEvent::ScrollWheel {
                scroll_x: 0.0,
                scroll_y: (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32),
            }),
            WM_MOUSEHWHEEL => window.dispatch(WindowEvent::ScrollWheel {
                scroll_x: (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32),
                scroll_y: 0.0,
            }),
            WM_CHAR => {
                if (wparam.0 & 0xD800) == 0xD800 {
                    window.save_high_surrogate(wparam.0 as u16);
                } else {
                    let codepoint = char::from_u32(if (wparam.0 & 0xDC00) == 0xDC00 {
                        (((window.take_high_surrogate() as u32 - 0xD800) << 10) | (wparam.0 as u32 - 0xDC00)) + 0x10000
                    } else {
                        wparam.0 as u32
                    })
                    .unwrap();

                    window.dispatch(WindowEvent::Char { codepoint });
                }
            }
            WM_PAINT => {
                window.dispatch(WindowEvent::Update {})
            }
            _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
        }

        LRESULT::default()
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
