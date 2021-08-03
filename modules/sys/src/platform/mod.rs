// Pass through to platform implementation
pub use implementation::*;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod implementation;
