use std::{cell::RefCell, convert::TryInto, sync::Once};

use windows::Win32::{
    Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, PWSTR, RECT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        GetWindowLongPtrW, GetWindowRect, LoadCursorW, PeekMessageW, PostQuitMessage,
        RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage, CREATESTRUCTW,
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MINMAXINFO, MSG,
        PM_REMOVE, SW_SHOW, WHEEL_DELTA, WINDOW_EX_STYLE, WM_CHAR, WM_CLOSE, WM_CREATE,
        WM_ERASEBKGND, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
        WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_QUIT,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    },
};

use super::input::{ButtonState, Event as InputEvent, MouseButton};
use crate::{
    array_vec::ArrayVec,
    px::Px,
    shapes::{Extent, Point},
};

const WNDCLASS_NAME: &str = "maple_wndclass";

/// The maximum number of bytes that the window title can be, in UTF-8 code
/// points including the null character required for compatibility with C.
///
/// That is to say: at most 255 bytes, plus the '\0' character.
pub const MAX_TITLE_LENGTH: usize = 256;

static REGISTER_CLASS: Once = Once::new();

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Created { size: Extent },
    Destroyed {},
    CloseRequested {},
    Resized { size: Extent },
    Update {},
    Input(super::input::Event),
}

#[derive(Debug, PartialEq)]
pub enum EventLoopControl {
    Continue,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(target_os = "windows")]
pub struct Handle {
    pub hwnd: HWND,
    pub hinstance: HINSTANCE,
}

pub trait Control {
    fn handle(&self) -> &Handle;

    fn min_size(&self) -> Extent;

    fn set_min_size(&mut self, size: Extent);
}

pub fn window<Callback>(title: &str, callback: Callback)
where
    Callback: FnMut(&mut dyn Control, Event) -> EventLoopControl,
{
    let mut class_name = to_wstr::<16>(WNDCLASS_NAME);

    let hinstance = unsafe { GetModuleHandleW(None) };
    assert_ne!(hinstance, HINSTANCE::default());

    REGISTER_CLASS.call_once(|| {
        let cursor = unsafe { LoadCursorW(None, &IDC_ARROW) };

        let class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW, /*| CS_DBLCLKS // for double clicks */
            hInstance: hinstance,
            lpfnWndProc: Some(wndproc_trampoline::<Callback>),
            lpszClassName: PWSTR(class_name.as_mut_ptr()),
            hCursor: cursor,
            ..WNDCLASSW::default()
        };

        let _ = unsafe { RegisterClassW(&class) };
    });

    let hwnd = {
        let mut w_title = to_wstr::<MAX_TITLE_LENGTH>(title);
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

    let window = RefCell::new(Window {
        callback,
        state: WindowState {
            high_surrogate: 0,
            handle: Handle { hwnd, hinstance },
            min_size: Extent::default(),
        },
    });

    {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect) };

        let width = (rect.right - rect.left)
            .try_into()
            .expect("Window width is negative or > 65535");
        let height = (rect.bottom - rect.top)
            .try_into()
            .expect("Window heigth is negative or > 65535");
        window.borrow_mut().dispatch(Event::Created {
            size: Extent {
                width: Px(width),
                height: Px(height),
            },
        });
    }

    let mut msg = MSG::default();

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &window as *const _ as _);
        ShowWindow(hwnd, SW_SHOW);
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

        DestroyWindow(window.borrow().state.handle.hwnd);
        PostQuitMessage(0);
    }

    window.borrow_mut().dispatch(Event::Destroyed {});
}

struct Window<Callback>
where
    Callback: FnMut(&mut dyn Control, Event) -> EventLoopControl,
{
    callback: Callback,
    state: WindowState,
}

struct WindowState {
    handle: Handle,
    high_surrogate: u16,
    min_size: Extent,
}

impl Control for WindowState {
    fn handle(&self) -> &Handle {
        &self.handle
    }

    fn min_size(&self) -> Extent {
        self.min_size
    }

    fn set_min_size(&mut self, size: Extent) {
        self.min_size = size;
    }
}

impl<Callback> Window<Callback>
where
    Callback: FnMut(&mut dyn Control, Event) -> EventLoopControl,
{
    fn dispatch(&mut self, event: Event) {
        let op = (self.callback)(&mut self.state, event);

        if op == EventLoopControl::Stop {
            unsafe { PostQuitMessage(0) };
        }
    }
}

unsafe extern "system" fn wndproc_trampoline<Callback>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT
where
    Callback: FnMut(&mut dyn Control, Event) -> EventLoopControl,
{
    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Window<Callback>>;

    if window_ptr.is_null() {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    } else {
        let window = &(*window_ptr);

        match msg {
            WM_CREATE => {
                let createstruct = &(*(lparam.0 as *const CREATESTRUCTW));
                let width = createstruct
                    .cx
                    .try_into()
                    .expect("Window width out of bounds!");
                let height = createstruct
                    .cy
                    .try_into()
                    .expect("Window height out of bounds!");
                window.borrow_mut().dispatch(Event::Created {
                    size: Extent {
                        width: Px(width),
                        height: Px(height),
                    },
                });
            }
            WM_CLOSE => {
                window.borrow_mut().dispatch(Event::CloseRequested {});
            }
            WM_GETMINMAXINFO => {
                let pointer = lparam.0 as *mut MINMAXINFO;
                let min = window.borrow().state.min_size;
                (*pointer).ptMinTrackSize = POINT {
                    x: min.width.0.into(),
                    y: min.height.0.into(),
                };
            }
            // WM_DESTROY is not handled. We send out the Event::Destroyed
            // message once we exit the event loop instead to avoid a re-entrant
            // call to window.borrow_mut();
            WM_SIZE => {
                // LOWORD and HIWORD (i16s for historical reasons, I guess)
                let width = (lparam.0 as i16)
                    .try_into()
                    .expect("Window width is negative or > 65535");
                let height = ((lparam.0 >> i16::BITS) as i16)
                    .try_into()
                    .expect("Window height is negative or > 65535");

                window.borrow_mut().dispatch(Event::Resized {
                    size: Extent { width, height },
                });
            }
            WM_ERASEBKGND => {
                /* No op, as recommended here:
                  https://stackoverflow.com/questions/53000291/how-to-smooth-ugly-jitter-flicker-jumping-when-resizing-windows-especially-drag
                */
                return LRESULT(1);
            }
            WM_MOUSEMOVE => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::CursorMove {
                    position: Point::new(Px(lparam.0 as i16), Px((lparam.0 >> 16) as i16)),
                })),
            WM_LBUTTONDOWN => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::MouseButton {
                    button: MouseButton::Left,
                    state: ButtonState::Pressed,
                })),
            WM_LBUTTONUP => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::MouseButton {
                    button: MouseButton::Left,
                    state: ButtonState::Released,
                })),
            WM_MBUTTONDOWN => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::MouseButton {
                    button: MouseButton::Middle,
                    state: ButtonState::Pressed,
                })),
            WM_MBUTTONUP => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::MouseButton {
                    button: MouseButton::Middle,
                    state: ButtonState::Released,
                })),
            WM_RBUTTONDOWN => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::MouseButton {
                    button: MouseButton::Right,
                    state: ButtonState::Pressed,
                })),
            WM_RBUTTONUP => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::MouseButton {
                    button: MouseButton::Right,
                    state: ButtonState::Released,
                })),
            WM_MOUSEWHEEL => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::ScrollWheel {
                    x: 0.0,
                    y: (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32),
                })),
            WM_MOUSEHWHEEL => window
                .borrow_mut()
                .dispatch(Event::Input(InputEvent::ScrollWheel {
                    x: (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32),
                    y: 0.0,
                })),
            WM_CHAR => {
                let mut window_mut = window.borrow_mut();
                if (wparam.0 & 0xD800) == 0xD800 {
                    window_mut.state.high_surrogate = wparam.0 as u16;
                } else {
                    let codepoint = char::from_u32(if (wparam.0 & 0xDC00) == 0xDC00 {
                        (((window_mut.state.high_surrogate as u32 - 0xD800) << 10)
                            | (wparam.0 as u32 - 0xDC00))
                            + 0x10000
                    } else {
                        wparam.0 as u32
                    })
                    .unwrap();

                    window_mut.dispatch(Event::Input(InputEvent::Char { codepoint }));
                }
            }
            WM_PAINT => window.borrow_mut().dispatch(Event::Update {}),
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
