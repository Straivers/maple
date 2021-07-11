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
                WNDCLASSW, DefWindowProcW,
                RegisterClassW, CreateWindowExW, DestroyWindow,
                GetMessageW, TranslateMessage, DispatchMessageW
            },
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn build_apis() {
    // no-op
}
