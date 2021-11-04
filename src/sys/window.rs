use std::{convert::TryInto, sync::Once};

use win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM},
    System::{LibraryLoader::GetModuleHandleW, Threading::MsgWaitForMultipleObjects, WindowsProgramming::INFINITE},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetWindowLongPtrW, GetWindowRect,
        LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
        CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, PM_REMOVE, QS_ALLEVENTS,
        SW_SHOW, WHEEL_DELTA, WINDOW_EX_STYLE, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    },
};

use super::{
    dpi::PhysicalSize,
    input::{ButtonState, FrameInput, MouseButton},
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
pub enum WindowEvent<'a> {
    Created { size: PhysicalSize },
    Destroyed {},
    CloseRequested {},
    Resized { size: PhysicalSize },
    Update { input: &'a FrameInput },
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
    assert_ne!(hinstance, HINSTANCE::NULL);

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
        input: FrameInput::new(),
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

    // Is there a way to make sure both window_trait and window_trait_ptr are on the same cache line?
    // Fat pointer
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
            MsgWaitForMultipleObjects(0, std::ptr::null(), false, INFINITE, QS_ALLEVENTS);
            window.input.advance();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                if msg.message == WM_QUIT {
                    DestroyWindow(hwnd);
                    return;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            (window.callback)(&window.control, WindowEvent::Update { input: &window.input });
        }
    }
}

struct Window<Callback>
where
    Callback: FnMut(&WindowControl, WindowEvent) -> EventLoopControl,
{
    callback: Callback,
    control: WindowControl,
    input: FrameInput,
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

    fn input_mut(&mut self) -> &mut FrameInput {
        &mut self.input
    }
}

trait WindowT {
    fn handle(&self) -> WindowHandle;

    fn dispatch(&mut self, event: WindowEvent);

    fn input_mut(&mut self) -> &mut FrameInput;
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
            WM_MOUSEMOVE => {
                let input = window.input_mut();
                input.cursor_x = lparam.0 as i16;
                input.cursor_y = (lparam.0 >> 16) as i16;
            }
            WM_LBUTTONDOWN => window.input_mut().mouse_buttons[MouseButton::Left].set(ButtonState::Pressed),
            WM_LBUTTONUP => window.input_mut().mouse_buttons[MouseButton::Left].set(ButtonState::Released),
            WM_MBUTTONDOWN => window.input_mut().mouse_buttons[MouseButton::Middle].set(ButtonState::Pressed),
            WM_MBUTTONUP => window.input_mut().mouse_buttons[MouseButton::Middle].set(ButtonState::Released),
            WM_RBUTTONDOWN => window.input_mut().mouse_buttons[MouseButton::Right].set(ButtonState::Pressed),
            WM_RBUTTONUP => window.input_mut().mouse_buttons[MouseButton::Right].set(ButtonState::Released),
            WM_MOUSEWHEEL => window.input_mut().cursor_wheel_y += (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32),
            WM_MOUSEHWHEEL => window.input_mut().cursor_wheel_x += (wparam.0 >> 16) as i16 as f32 / (WHEEL_DELTA as f32),
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
