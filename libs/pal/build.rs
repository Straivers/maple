fn main() {
    build_apis();
}

#[cfg(target_os = "windows")]
fn build_apis() {
    windows::build! {
        Windows::Win32::{
            Foundation::{HWND, HINSTANCE, LPARAM, WPARAM, PWSTR},
            System::{LibraryLoader::{GetModuleHandleW}},
            UI::WindowsAndMessaging::{
                WNDCLASSW, CREATESTRUCTW, DefWindowProcW, CW_USEDEFAULT,
                WM_NCCREATE, WM_CLOSE, WM_QUIT, WM_DESTROY,
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
