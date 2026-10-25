pub use anyhow::Result;
use std::path::Path;
pub use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatus {
    Connected { info: String, vid: u16, pid: u16 },
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsMouseSettings {
    pub pointer_speed: u32,      // 1..=20 (Windows scale, 10 is default 1:1)
    pub enhance_precision: bool, // Acceleration toggle
    pub double_click_speed: u32, // Double-click threshold in ms (default ~500ms)
    pub rolling_speed: u32,      // Lines per wheel notch (default 3)
    pub scroll_one_screen: bool, // Scroll one full page per notch
}

impl Default for OsMouseSettings {
    fn default() -> Self {
        Self {
            pointer_speed: 10,
            enhance_precision: true,
            double_click_speed: 500,
            rolling_speed: 3,
            scroll_one_screen: false,
        }
    }
}

pub fn get_os_mouse_settings() -> OsMouseSettings {
    #[cfg(target_os = "windows")]
    return windows::get_os_mouse_settings();
    #[cfg(target_os = "linux")]
    return linux::get_os_mouse_settings();
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    return fallback::get_os_mouse_settings();
}

pub fn apply_os_mouse_settings(settings: &OsMouseSettings) -> Result<()> {
    #[cfg(target_os = "windows")]
    return windows::apply_os_mouse_settings(settings);
    #[cfg(target_os = "linux")]
    return linux::apply_os_mouse_settings(settings);
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    return fallback::apply_os_mouse_settings(settings);
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
