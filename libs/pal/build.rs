fn main() {
    build_apis();
}

#[cfg(target_os = "windows")]
fn build_apis() {
    windows::build! {
        Windows::Win32::{
            Foundation::{HWND, HINSTANCE, LPARAM, WPARAM, PWSTR, PSTR},
            System::{
                LibraryLoader::{
                    GetModuleHandleW,
                    GetProcAddress,
                    LoadLibraryA
                },
                Diagnostics::Debug::{
                    SetErrorMode,
                }
            },
            UI::WindowsAndMessaging::{
                WNDCLASSW, CREATESTRUCTW, DefWindowProcW, CW_USEDEFAULT,
                WM_CREATE, WM_CLOSE, WM_QUIT, WM_DESTROY,
                RegisterClassW, CreateWindowExW, DestroyWindow, ShowWindow,
                PeekMessageW, TranslateMessage, DispatchMessageW,
                SetWindowLongPtrW, GetWindowLongPtrW,
            },
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn build_apis() {
    // no-op
}
