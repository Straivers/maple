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
                }
            },
            UI::{
                WindowsAndMessaging::{
                    // Constants
                    CW_USEDEFAULT, IDC_ARROW, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_QUIT, WM_SIZE,
                    // Structs
                    CREATESTRUCTW, MSG, WINDOW_EX_STYLE, WNDCLASSW,
                    // Functions
                    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetWindowLongPtrW, LoadCursorW,
                    PeekMessageW, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
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
