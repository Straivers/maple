fn main() {
    build_apis();
}

#[cfg(target_os = "windows")]
fn build_apis() {
    windows::build! {
        Windows::Win32::{
        Foundation::{HWND, LPARAM, WPARAM}
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn build_apis() {
    // no-op
}
