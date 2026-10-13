use super::DeviceStatus;
use anyhow::{bail, Result};
use hidapi::HidApi;
use std::os::windows::process::CommandExt;
use std::ptr::null_mut;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HANDLE, HWND, LPARAM};
use windows_sys::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, RegisterRawInputDevices, HRAWINPUT, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK, RIDI_DEVICENAME, RID_INPUT, RIM_TYPEKEYBOARD,
    RIM_TYPEMOUSE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW,
    TranslateMessage, HWND_MESSAGE, MSG, PM_REMOVE, QS_ALLINPUT, WM_INPUT,
};

use crate::device::Em624Device;
use crate::gui::chrono_or_simple_time;

// Constants for Raw Input
const RI_MOUSE_LEFT_BUTTON_DOWN: u16 = 0x0001;
const RI_MOUSE_LEFT_BUTTON_UP: u16 = 0x0002;
const RI_MOUSE_RIGHT_BUTTON_DOWN: u16 = 0x0004;
const RI_MOUSE_RIGHT_BUTTON_UP: u16 = 0x0008;
const RI_MOUSE_MIDDLE_BUTTON_DOWN: u16 = 0x0010;
const RI_MOUSE_MIDDLE_BUTTON_UP: u16 = 0x0020;
const RI_MOUSE_BUTTON_4_DOWN: u16 = 0x0040;
const RI_MOUSE_BUTTON_4_UP: u16 = 0x0080;
const RI_MOUSE_BUTTON_5_DOWN: u16 = 0x0100;
const RI_MOUSE_BUTTON_5_UP: u16 = 0x0200;
const RI_MOUSE_WHEEL: u16 = 0x0400;

pub fn trim_working_set() {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::GetCurrentProcess();
        let _ = windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet(handle);
        let _ = windows_sys::Win32::System::Threading::SetProcessWorkingSetSize(
            handle,
            usize::MAX,
            usize::MAX,
        );
    }
}

pub fn get_process_memory_mb() -> (f32, f32) {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::GetCurrentProcess();
        let mut pmc: windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS =
            std::mem::zeroed();
        let cb = std::mem::size_of::<
            windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS,
        >() as u32;
        if windows_sys::Win32::System::ProcessStatus::K32GetProcessMemoryInfo(handle, &mut pmc, cb)
            != 0
        {
            (
                pmc.WorkingSetSize as f32 / (1024.0 * 1024.0),
                pmc.PagefileUsage as f32 / (1024.0 * 1024.0),
            )
        } else {
            (0.0, 0.0)
        }
    }
}

pub fn spawn_debug_terminal() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .arg("log-viewer")
            .creation_flags(0x00000010) // CREATE_NEW_CONSOLE
            .spawn();
    }
}

pub fn set_console_title(title: &str) {
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        extern "system" {
            fn SetConsoleTitleW(title: *const u16) -> i32;
        }
        let _ = SetConsoleTitleW(wide.as_ptr());
    }
}

pub fn start_device_monitor() -> Receiver<DeviceStatus> {
    let (tx, rx) = channel();
    thread::Builder::new()
        .name("em624-device-monitor-win32".to_string())
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

            let class_name = [0x53, 0x54, 0x41, 0x54, 0x49, 0x43, 0]; // "STATIC\0"
            let hwnd: HWND = unsafe {
                CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    null_mut(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    0,
                    0,
                    null_mut(),
                )
            };

            let mut msg: MSG = unsafe { std::mem::zeroed() };

            loop {
                // Wait for OS message or 1000ms heartbeat
                unsafe {
                    MsgWaitForMultipleObjectsEx(0, null_mut(), 1000, QS_ALLINPUT, 0);
                }

                while unsafe { PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) } != 0 {
                    unsafe {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

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

            if hwnd != 0 {
                unsafe { DestroyWindow(hwnd) };
            }
        })
        .expect("Failed to spawn Windows device monitor thread");

    rx
}

pub fn listen_raw_input(timeout: Option<u64>) -> Result<()> {
    let class_name = [0x53, 0x54, 0x41, 0x54, 0x49, 0x43, 0]; // "STATIC\0"
    let hwnd: HWND = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            null_mut(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            0,
            0,
            null_mut(),
        )
    };

    if hwnd == 0 {
        bail!("Failed to create Windows Raw Input message receiver window.");
    }

    let rid = [
        RAWINPUTDEVICE {
            usUsagePage: 0x01, // Generic Desktop
            usUsage: 0x02,     // Mouse
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x06, // Keyboard
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        },
    ];

    let registered = unsafe {
        RegisterRawInputDevices(
            rid.as_ptr(),
            rid.len() as u32,
            std::mem::size_of::<RAWINPUTDEVICE>() as u32,
        )
    };

    if registered == 0 {
        unsafe { DestroyWindow(hwnd) };
        bail!("Failed to register Raw Input devices with Windows OS.");
    }

    println!("========================================================================");
    println!("  E-DRA EM624 RAW HARDWARE INPUT SNIFFER & PACKET MONITOR (WINDOWS)");
    println!("  Connected via Windows Raw Input Subsystem (bypassing mouhid exclusive lock)");
    println!("  Active Listeners: Physical Buttons, Scroll Wheel, Optical Sensor & Hotkeys");
    println!("  Click buttons, move mouse, roll wheel. Press Ctrl+C to stop.");
    println!("========================================================================\n");

    let start = Instant::now();
    let mut msg: MSG = unsafe { std::mem::zeroed() };

    loop {
        if let Some(t) = timeout {
            if start.elapsed().as_secs() >= t {
                println!("Listening timeout reached ({} seconds). Exiting.", t);
                break;
            }
        }

        let has_msg = unsafe { PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) };

        if has_msg != 0 {
            if msg.message == WM_INPUT {
                process_raw_input(msg.lParam);
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }

    unsafe { DestroyWindow(hwnd) };
    Ok(())
}

fn process_raw_input(lparam: LPARAM) {
    let mut size: u32 = 0;
    let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;

    unsafe {
        GetRawInputData(
            lparam as HRAWINPUT,
            RID_INPUT,
            null_mut(),
            &mut size,
            header_size,
        );
    }

    if size == 0 {
        return;
    }

    let mut buffer = vec![0u8; size as usize];
    let bytes_read = unsafe {
        GetRawInputData(
            lparam as HRAWINPUT,
            RID_INPUT,
            buffer.as_mut_ptr() as *mut _,
            &mut size,
            header_size,
        )
    };

    if bytes_read == u32::MAX {
        return;
    }

    let raw = unsafe { &*(buffer.as_ptr() as *const RAWINPUT) };
    let ts = chrono_or_simple_time();

    let dev_name = get_device_name(raw.header.hDevice);
    let is_em624 =
        dev_name.to_lowercase().contains("1bcf") && dev_name.to_lowercase().contains("08b8");
    let dev_tag = if is_em624 {
        "[E-DRA EM624]"
    } else {
        "[INPUT_EVENT]"
    };

    if raw.header.dwType == RIM_TYPEMOUSE {
        let mouse = unsafe { &raw.data.mouse };
        let flags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags };
        let button_data = unsafe { mouse.Anonymous.Anonymous.usButtonData as i16 };
        let dx = mouse.lLastX;
        let dy = mouse.lLastY;

        let mut events = Vec::new();
        if (flags & RI_MOUSE_LEFT_BUTTON_DOWN) != 0 {
            events.push("LEFT [DOWN]");
        }
        if (flags & RI_MOUSE_LEFT_BUTTON_UP) != 0 {
            events.push("LEFT [UP]");
        }
        if (flags & RI_MOUSE_RIGHT_BUTTON_DOWN) != 0 {
            events.push("RIGHT [DOWN]");
        }
        if (flags & RI_MOUSE_RIGHT_BUTTON_UP) != 0 {
            events.push("RIGHT [UP]");
        }
        if (flags & RI_MOUSE_MIDDLE_BUTTON_DOWN) != 0 {
            events.push("MIDDLE [DOWN]");
        }
        if (flags & RI_MOUSE_MIDDLE_BUTTON_UP) != 0 {
            events.push("MIDDLE [UP]");
        }
        if (flags & RI_MOUSE_BUTTON_4_DOWN) != 0 {
            events.push("SIDE 1 (FORWARD) [DOWN]");
        }
        if (flags & RI_MOUSE_BUTTON_4_UP) != 0 {
            events.push("SIDE 1 (FORWARD) [UP]");
        }
        if (flags & RI_MOUSE_BUTTON_5_DOWN) != 0 {
            events.push("SIDE 2 (BACKWARD) [DOWN]");
        }
        if (flags & RI_MOUSE_BUTTON_5_UP) != 0 {
            events.push("SIDE 2 (BACKWARD) [UP]");
        }
        if (flags & RI_MOUSE_WHEEL) != 0 {
            events.push("WHEEL");
        }

        if !events.is_empty() {
            let wheel_str = if (flags & RI_MOUSE_WHEEL) != 0 {
                format!(" | Wheel: {:+}", button_data / 120)
            } else {
                String::new()
            };

            println!(
                "[{}] {} {:<32} | dX={:+5}, dY={:+5}{}",
                ts,
                dev_tag,
                events.join(" + "),
                dx,
                dy,
                wheel_str
            );
        }
    } else if raw.header.dwType == RIM_TYPEKEYBOARD {
        let kbd = unsafe { &raw.data.keyboard };
        let is_up = (kbd.Flags & 1) != 0;
        let action = if is_up { "UP" } else { "DOWN" };
        println!(
            "[{}] {} KEYBOARD HOTKEY: VKey=0x{:02X}, MakeCode=0x{:02X} [{}]",
            ts, dev_tag, kbd.VKey, kbd.MakeCode, action
        );
    }
}

fn get_device_name(h_device: HANDLE) -> String {
    let mut size: u32 = 0;
    unsafe {
        GetRawInputDeviceInfoW(h_device, RIDI_DEVICENAME, null_mut(), &mut size);
    }
    if size == 0 {
        return String::new();
    }
    let mut name = vec![0u16; size as usize];
    let res = unsafe {
        GetRawInputDeviceInfoW(
            h_device,
            RIDI_DEVICENAME,
            name.as_mut_ptr() as *mut _,
            &mut size,
        )
    };
    if res == u32::MAX {
        return String::new();
    }
    String::from_utf16_lossy(&name)
}
