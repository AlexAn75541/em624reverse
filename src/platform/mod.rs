pub use anyhow::Result;
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
