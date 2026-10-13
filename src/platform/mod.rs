pub use anyhow::Result;
use std::path::Path;
pub use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatus {
    Connected { info: String, vid: u16, pid: u16 },
    Disconnected,
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod fallback;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub use fallback::*;

pub fn spawn_debug_terminal(log_path: Option<&Path>) {
    #[cfg(target_os = "windows")]
    windows::spawn_debug_terminal(log_path);
    #[cfg(target_os = "linux")]
    linux::spawn_debug_terminal(log_path);
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    fallback::spawn_debug_terminal(log_path);
}

pub fn ensure_console_allocated() {
    #[cfg(target_os = "windows")]
    windows::ensure_console_allocated();
    #[cfg(target_os = "linux")]
    linux::ensure_console_allocated();
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    fallback::ensure_console_allocated();
}
