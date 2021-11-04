fn main() {
    build_apis();
}

#[cfg(target_os = "windows")]
fn build_apis() {
    windows::build! {
        Windows::Win32::{
            Foundation::{HWND, HINSTANCE, LPARAM, WPARAM, PWSTR, PSTR, RECT},
            System::{
                LibraryLoader::{
                    GetModuleHandleW,
                    GetProcAddress,
                    LoadLibraryW,
                },
                Diagnostics::Debug::{
                    GetLastError,
                    SetErrorMode,
                },
                Threading::{
                    MsgWaitForMultipleObjects
                },
                WindowsProgramming::{
                    INFINITE
                }
            },
            UI::{
                WindowsAndMessaging::{
                    // Constants
                    CW_USEDEFAULT, IDC_ARROW, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_QUIT, WM_SIZE, WM_NULL,
                    WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN,
                    WM_RBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOUSEHWHEEL,
                    WHEEL_DELTA,
                    // Structs
                    CREATESTRUCTW, MSG, WINDOW_EX_STYLE, WNDCLASSW,
                    // Functions
                    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetWindowLongPtrW, LoadCursorW,
                    PeekMessageW, GetMessageW, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
                    PostQuitMessage, GetWindowRect,
                },
                HiDpi::GetProcessDpiAwareness,
            },
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn build_apis() {
    // no-op
}
