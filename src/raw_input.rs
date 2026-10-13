#[cfg(target_os = "windows")]
pub mod listener {
    use anyhow::{bail, Result};
    use std::ptr::null_mut;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{HANDLE, HWND, LPARAM};
    use windows_sys::Win32::UI::Input::{
        GetRawInputData, GetRawInputDeviceInfoW, RegisterRawInputDevices, HRAWINPUT, RAWINPUT,
        RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK, RIDI_DEVICENAME, RID_INPUT,
        RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, PeekMessageW, TranslateMessage,
        HWND_MESSAGE, MSG, PM_REMOVE, WM_INPUT,
    };

    use crate::gui::chrono_or_simple_time;

    // Standard Win32 Raw Input Mouse Flags
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

    pub fn listen_windows(timeout: Option<u64>) -> Result<()> {
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

        // Register both Mouse (1, 2) and Keyboard (1, 6)
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
                std::thread::sleep(std::time::Duration::from_millis(1));
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

        // Get device name to check if this is our EM624 mouse
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

            // Print on button action, wheel, or motion from our mouse
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
}
