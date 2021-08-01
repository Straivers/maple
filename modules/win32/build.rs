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
                    LoadLibraryA
                },
                Diagnostics::Debug::{
                    GetLastError,
                    SetErrorMode,
                }
            },
            UI::{
                WindowsAndMessaging::{
                    WNDCLASSW, CREATESTRUCTW, DefWindowProcW, CW_USEDEFAULT,
                    WM_CREATE, WM_CLOSE, WM_QUIT, WM_DESTROY, WM_SIZE,
                    RegisterClassW, CreateWindowExW, DestroyWindow, ShowWindow,
                    PeekMessageW, TranslateMessage, DispatchMessageW,
                    SetWindowLongPtrW, GetWindowLongPtrW, GetClientRect,
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
