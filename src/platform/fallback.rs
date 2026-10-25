use super::{DeviceStatus, OsMouseSettings};
use anyhow::{bail, Result};
use hidapi::HidApi;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use crate::device::Em624Device;

pub fn trim_working_set() {}

pub fn spawn_debug_terminal(log_path: Option<&std::path::Path>) {
    if let Ok(exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("log-viewer");
        if let Some(p) = log_path {
            cmd.arg("--log-file").arg(p);
        }
        let _ = cmd.spawn();
    }
}

pub fn set_console_title(_title: &str) {}

pub fn get_os_mouse_settings() -> OsMouseSettings {
    OsMouseSettings {
        pointer_speed: 10,
        enhance_precision: true,
        double_click_speed: 500,
        rolling_speed: 3,
        scroll_one_screen: false,
    }
}

pub fn apply_os_mouse_settings(_settings: &OsMouseSettings) -> Result<()> {
    Ok(())
}

pub fn start_device_monitor() -> Receiver<DeviceStatus> {
    let (tx, rx) = channel();
    thread::Builder::new()
        .name("em624-device-monitor".to_string())
        .spawn(move || {
            let mut api = match HidApi::new() {
                Ok(a) => a,
                Err(_) => return,
            };

            let mut was_connected = false;
            if let Ok(dev) = Em624Device::find_candidate(&api) {
                was_connected = true;
                let _ = tx.send(DeviceStatus::Connected {
                    info: format!(
                        "E-DRA EM624 (VID 0x{:04x}, PID 0x{:04x})",
                        dev.vendor_id(),
                        dev.product_id()
                    ),
                    vid: dev.vendor_id(),
                    pid: dev.product_id(),
                });
            } else {
                let _ = tx.send(DeviceStatus::Disconnected);
            }

            loop {
                thread::sleep(Duration::from_millis(1000));
                let _ = api.refresh_devices();
                let is_connected = Em624Device::find_candidate(&api).is_ok();
                if is_connected != was_connected {
                    was_connected = is_connected;
                    if is_connected {
                        if let Ok(dev) = Em624Device::find_candidate(&api) {
                            if tx
                                .send(DeviceStatus::Connected {
                                    info: format!(
                                        "E-DRA EM624 (VID 0x{:04x}, PID 0x{:04x})",
                                        dev.vendor_id(),
                                        dev.product_id()
                                    ),
                                    vid: dev.vendor_id(),
                                    pid: dev.product_id(),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    } else if tx.send(DeviceStatus::Disconnected).is_err() {
                        break;
                    }
                }
            }
        })
        .expect("Failed to spawn device monitor thread");

    rx
}

pub fn listen_raw_input(_timeout: Option<u64>) -> Result<()> {
    bail!("Hardware packet sniffer not supported on this operating system");
}
