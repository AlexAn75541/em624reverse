use super::DeviceStatus;
use anyhow::Result;
use hidapi::HidApi;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use crate::device::Em624Device;
use crate::gui::chrono_or_simple_time;

pub fn trim_working_set() {
    extern "C" {
        fn malloc_trim(pad: usize) -> i32;
    }
    unsafe {
        let _ = malloc_trim(0);
    }
}

pub fn get_process_memory_mb() -> (f32, f32) {
    if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = statm.split_whitespace().collect();
        if parts.len() >= 2 {
            let page_size_kb = 4.0;
            let rss_pages: f32 = parts[1].parse().unwrap_or(0.0);
            let rss_mb = (rss_pages * page_size_kb) / 1024.0;
            return (rss_mb, rss_mb);
        }
    }
    (0.0, 0.0)
}

pub fn spawn_debug_terminal(log_path: Option<&std::path::Path>) {
    if let Ok(exe) = std::env::current_exe() {
        let terminals = [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "xterm",
        ];
        let arg_str = if let Some(p) = log_path {
            format!(
                "{} log-viewer --log-file \"{}\"",
                exe.display(),
                p.display()
            )
        } else {
            format!("{} log-viewer", exe.display())
        };

        let mut spawned = false;
        for term in terminals {
            if let Ok(_child) = std::process::Command::new(term)
                .arg("-e")
                .arg(&arg_str)
                .spawn()
            {
                spawned = true;
                break;
            }
        }
        if !spawned {
            let mut cmd = std::process::Command::new(exe);
            cmd.arg("log-viewer");
            if let Some(p) = log_path {
                cmd.arg("--log-file").arg(p);
            }
            let _ = cmd.spawn();
        }
    }
}

pub fn set_console_title(title: &str) {
    print!("\x1b]0;{}\x07", title);
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub fn start_device_monitor() -> Receiver<DeviceStatus> {
    let (tx, rx) = channel();
    thread::Builder::new()
        .name("em624-device-monitor-linux".to_string())
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
        .expect("Failed to spawn Linux device monitor thread");

    rx
}

pub fn listen_raw_input(timeout: Option<u64>) -> Result<()> {
    let api = HidApi::new()?;
    let endpoints = Em624Device::open_input_endpoints(&api)?;

    println!("========================================================================");
    println!("  E-DRA EM624 HARDWARE INPUT EVENT SNIFFER & PROBED PACKET LISTENER (LINUX)");
    println!("  Listening to live USB HID Input Reports across all mouse interfaces...");
    println!("  Click physical buttons, roll wheel, or trigger macros. Press Ctrl+C to stop.");
    println!("========================================================================\n");

    for (idx, ep) in endpoints.iter().enumerate() {
        println!("  Endpoint [{}]: {}", idx + 1, ep.interface_name);
    }
    println!();

    let start = std::time::Instant::now();
    let mut buf = [0u8; 64];

    loop {
        if let Some(t) = timeout {
            if start.elapsed().as_secs() >= t {
                println!("Listening timeout reached ({} seconds). Exiting.", t);
                break;
            }
        }

        let mut received = false;
        for ep in &endpoints {
            if let Ok(len) = ep.device.read_timeout(&mut buf, 10) {
                if len > 0 {
                    received = true;
                    let ts = chrono_or_simple_time();
                    let hex_slice = &buf[..len];
                    print!(
                        "[{}] [{}] RAW [{}B]: {:02X?}",
                        ts, ep.interface_name, len, hex_slice
                    );

                    if ep.interface_name.contains("MI_00") && len >= 3 {
                        let btn_mask = buf[0];
                        let mut pressed = Vec::new();
                        if (btn_mask & 0x01) != 0 {
                            pressed.push("Left");
                        }
                        if (btn_mask & 0x02) != 0 {
                            pressed.push("Right");
                        }
                        if (btn_mask & 0x04) != 0 {
                            pressed.push("Middle");
                        }
                        if (btn_mask & 0x08) != 0 {
                            pressed.push("Side1(Fwd)");
                        }
                        if (btn_mask & 0x10) != 0 {
                            pressed.push("Side2(Back)");
                        }

                        let dx = buf[1] as i8;
                        let dy = buf[2] as i8;
                        let wheel = if len >= 4 { buf[3] as i8 } else { 0 };

                        print!(
                            " -> Buttons: [{}] | dX: {:4}, dY: {:4}, Wheel: {:2}",
                            if pressed.is_empty() {
                                "None".to_string()
                            } else {
                                pressed.join("+")
                            },
                            dx,
                            dy,
                            wheel
                        );
                    } else if ep.interface_name.contains("Col01") && len >= 2 {
                        print!(
                            " -> Keyboard Modifier: 0x{:02x}, Key: 0x{:02x}",
                            buf[0], buf[1]
                        );
                    } else if ep.interface_name.contains("Col02") && len >= 2 {
                        print!(" -> Consumer/Media Usage: 0x{:02x}{:02x}", buf[1], buf[0]);
                    }

                    println!();
                }
            }
        }

        if !received {
            thread::sleep(Duration::from_millis(5));
        }
    }

    Ok(())
}
