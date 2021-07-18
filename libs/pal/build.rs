fn main() {
    build_apis();
}

#[cfg(target_os = "windows")]
fn build_apis() {
    windows::build! {
        Windows::Win32::{
            Foundation::{HWND, HINSTANCE, LPARAM, WPARAM, PWSTR},
            System::{Diagnostics::Debug::GetLastError, LibraryLoader::{GetModuleHandleW}},
            UI::WindowsAndMessaging::{
                WNDCLASSW, CREATESTRUCTW, DefWindowProcW, CW_USEDEFAULT,
                WM_NCCREATE, WM_CLOSE, WM_QUIT, WM_PAINT,
                RegisterClassW, CreateWindowExW, DestroyWindow, ShowWindow,
                WaitMessage, PeekMessageW, TranslateMessage, DispatchMessageW, PostQuitMessage,
                SetClassLongPtrW, GetClassLongPtrW, SetWindowLongPtrW, GetWindowLongPtrW,
            },
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn build_apis() {
    // no-op
}
