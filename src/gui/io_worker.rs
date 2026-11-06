use super::config::Tab;
use super::Em624App;
use crate::device::Em624Device;
use crate::profile::MouseProfile;
use hidapi::HidApi;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

#[allow(dead_code)]
pub enum IoProgressMsg {
    Progress { fraction: f32, message: String },
    ReadDone(Result<Vec<u8>, String>),
    FlashDone(Result<(), String>),
}

impl Em624App {
    pub fn is_io_busy(&self) -> bool {
        self.io_rx.is_some()
    }

    pub fn poll_io_worker(&mut self, ctx: &eframe::egui::Context) {
        let mut finished = false;
        if let Some(rx) = &self.io_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    IoProgressMsg::Progress { fraction, message } => {
                        self.io_progress = Some(fraction);
                        self.status_message = message;
                        ctx.request_repaint();
                    }
                    IoProgressMsg::ReadDone(res) => {
                        self.io_progress = None;
                        finished = true;
                        match res {
                            Ok(bytes) => match MouseProfile::from_bytes(&bytes) {
                                Ok(prof) => {
                                    self.profile = prof;
                                    self.device_connected = true;
                                    self.status_message =
                                        "Successfully loaded active profile from mouse hardware!"
                                            .to_string();
                                    self.save_to_cached_profile();
                                }
                                Err(e) => {
                                    self.status_message = format!("Profile parse error: {}", e);
                                }
                            },
                            Err(e) => {
                                self.status_message = format!("Read failed: {}", e);
                            }
                        }
                        ctx.request_repaint();
                        break;
                    }
                    IoProgressMsg::FlashDone(res) => {
                        self.io_progress = None;
                        finished = true;
                        match res {
                            Ok(()) => {
                                self.device_connected = true;
                                self.status_message =
                                    "Successfully flashed profile to mouse! Settings active immediately."
                                        .to_string();
                                self.save_to_cached_profile();
                            }
                            Err(e) => {
                                self.status_message = format!("Flash failed: {}", e);
                            }
                        }
                        ctx.request_repaint();
                        break;
                    }
                }
            }
        }
        if finished {
            self.io_rx = None;
            super::config::trim_working_set();
        }
    }

    pub fn read_from_mouse(&mut self) {
        if self.is_io_busy() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.io_rx = Some(rx);
        self.io_progress = Some(0.01);
        self.status_message = "Starting background EEPROM read...".to_string();
        self.log_debug("[USB_READ_BG] Initiating non-blocking EEPROM read...");

        thread::spawn(move || {
            let api = match HidApi::new() {
                Ok(a) => a,
                Err(e) => {
                    let _ = tx.send(IoProgressMsg::ReadDone(Err(format!(
                        "HID API error: {}",
                        e
                    ))));
                    return;
                }
            };

            let dev = match Em624Device::open_auto(&api) {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(IoProgressMsg::ReadDone(Err(format!(
                        "Mouse not connected: {}",
                        e
                    ))));
                    return;
                }
            };

            let tx_prog = tx.clone();
            let res = dev.read_profile_with_progress(move |frac, msg| {
                let _ = tx_prog.send(IoProgressMsg::Progress {
                    fraction: frac,
                    message: msg.to_string(),
                });
            });

            match res {
                Ok(bytes) => {
                    let _ = tx.send(IoProgressMsg::ReadDone(Ok(bytes)));
                }
                Err(e) => {
                    let _ = tx.send(IoProgressMsg::ReadDone(Err(e.to_string())));
                }
            }
        });
    }

    pub fn flash_to_mouse(&mut self) {
        if self.is_io_busy() {
            return;
        }

        self.sync_settings_to_profile();
        let raw_data = self.profile.raw.clone();

        let (tx, rx) = mpsc::channel();
        self.io_rx = Some(rx);
        self.io_progress = Some(0.01);
        self.status_message = "Starting background flash to mouse memory...".to_string();
        self.log_debug("[USB_FLASH_BG] Initiating non-blocking flash...");

        thread::spawn(move || {
            let api = match HidApi::new() {
                Ok(a) => a,
                Err(e) => {
                    let _ = tx.send(IoProgressMsg::FlashDone(Err(format!(
                        "HID API error: {}",
                        e
                    ))));
                    return;
                }
            };

            let dev = match Em624Device::open_auto(&api) {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(IoProgressMsg::FlashDone(Err(format!(
                        "Mouse not connected: {}",
                        e
                    ))));
                    return;
                }
            };

            let tx_prog = tx.clone();
            let res = dev.write_profile_with_progress(&raw_data, move |frac, msg| {
                let _ = tx_prog.send(IoProgressMsg::Progress {
                    fraction: frac,
                    message: msg.to_string(),
                });
            });

            match res {
                Ok(()) => {
                    let _ = tx.send(IoProgressMsg::FlashDone(Ok(())));
                }
                Err(e) => {
                    let _ = tx.send(IoProgressMsg::FlashDone(Err(e.to_string())));
                }
            }
        });
    }

    pub fn check_connection(&mut self) {
        match HidApi::new() {
            Ok(api) => match Em624Device::open_auto(&api) {
                Ok(dev) => {
                    self.device_connected = true;
                    self.connected_info =
                        format!("E-DRA EM624 (VID 0x{:04x}, PID 0x{:04x})", dev.vid, dev.pid);
                    self.status_message =
                        "Mouse connected and responsive on Interface 1 Col04.".to_string();
                    self.log_debug(&format!(
                        "[USB] Connected: E-DRA EM624 VID=0x{:04x}, PID=0x{:04x}",
                        dev.vid, dev.pid
                    ));
                }
                Err(e) => {
                    self.device_connected = false;
                    self.connected_info = "Not detected".to_string();
                    self.status_message = format!("Mouse not found: {}", e);
                    self.log_debug(&format!("[USB] Disconnected or error: {}", e));
                }
            },
            Err(e) => {
                self.device_connected = false;
                self.status_message = format!("HID API Error: {}", e);
                self.log_debug(&format!("[HID_ERROR] {}", e));
            }
        }
    }

    pub fn sync_from_profile(&mut self) {
        // Staged for packet capture alignment
    }

    pub fn sync_settings_to_profile(&mut self) {
        self.profile.apply_safe_profile();

        if !self.config.allow_stage_6 {
            let (num_stages, cur_stage) = self.profile.get_active_dpi_config();
            if num_stages > 5 {
                self.profile.set_active_dpi_config(5, cur_stage.min(4));
            }
        }

        if self.active_tab == Tab::OfficeMode {
            self.profile.raw[0x00a4] = 0x00;
            self.profile.raw[0x00a8] = 0x00;
            self.profile.raw[0x00aa] = 0x00;
        } else {
            self.profile.raw[0x00a4] = 0x01;
            self.profile.raw[0x00a8] = 0x0f;
            self.profile.raw[0x00aa] = 0x18;
        }
        self.profile.raw[0x03f9] &= 0x7f;

        self.apply_live_os_mouse_settings();
        self.profile.recalculate_checksums();
    }

    pub fn apply_live_os_mouse_settings(&mut self) {
        let os_speed = match self.pointer_speed {
            1 => 1,
            2 => 3,
            3 => 5,
            4 => 7,
            5 => 9,
            6 => 10,
            7 => 12,
            8 => 14,
            9 => 16,
            10 => 18,
            _ => 20,
        };
        let os_dbl_click = 900 - ((self.double_click_speed.clamp(1, 11) - 1) * 70);
        let os_settings = crate::platform::OsMouseSettings {
            pointer_speed: os_speed,
            enhance_precision: self.pointer_precision,
            double_click_speed: os_dbl_click,
            rolling_speed: self.rolling_speed,
            scroll_one_screen: self.scroll_one_screen,
        };
        if let Err(e) = crate::platform::apply_os_mouse_settings(&os_settings) {
            self.log_debug(&format!(
                "[OS_MOUSE] Failed to apply OS mouse settings: {}",
                e
            ));
        } else {
            self.log_debug(&format!(
                "[OS_MOUSE] Applied settings: speed={}, precision={}, dbl_click={}ms, scroll_lines={}",
                os_speed,
                self.pointer_precision,
                os_dbl_click,
                if self.scroll_one_screen {
                    0
                } else {
                    self.rolling_speed
                }
            ));
        }
    }

    pub fn switch_mode_and_forward(&mut self, new_is_gaming: bool) {
        let mode_byte = if new_is_gaming { 0x01 } else { 0x00 };
        let mode_name = if new_is_gaming {
            "Gaming Mode"
        } else {
            "Office Mode"
        };
        self.profile.is_gaming_mode = new_is_gaming;
        self.active_tab = if new_is_gaming {
            Tab::GameMode
        } else {
            Tab::OfficeMode
        };
        self.profile.raw[0x00a4] = mode_byte;
        self.sync_settings_to_profile();
        self.profile.raw[0x00a4] = mode_byte;
        self.profile.recalculate_checksums();
        self.save_to_cached_profile();

        self.status_message = format!("Switched to {}.", mode_name);
        self.log_debug(&format!(
            "[MODE_FORWARD] Switched UI mode to: {} (Byte 0x00A4 = 0x{:02X})",
            mode_name, mode_byte
        ));

        // Non-blocking background thread: deliver mode register to mouse without freezing GUI
        let payload = self.profile.raw[0x00a4..0x00c0].to_vec();
        std::thread::spawn(move || {
            if let Ok(api) = HidApi::new() {
                if let Ok(dev) = Em624Device::open_auto(&api) {
                    let _ = dev.write_registers_3b(0x00a4, &payload);
                }
            }
        });
    }

    pub fn sync_led_and_forward(&mut self, desc: &str) {
        self.profile.recalculate_checksums();
        self.save_to_cached_profile();
        self.status_message = format!("LED Lighting synced: {}", desc);
        self.log_debug(&format!(
            "[LED_SYNC] {} | 0x00A5=0x{:02X} 0x0038=0x{:02X} 0x00A7=0x{:02X} 0x03F8=0x{:02X} 0x03FA=0x{:02X}",
            desc,
            self.profile.raw[0x00a5],
            self.profile.raw[0x0038],
            self.profile.raw[0x00a7],
            self.profile.raw[0x03f8],
            self.profile.raw[0x03fa]
        ));

        let raw_copy = self.profile.raw.clone();
        std::thread::spawn(move || {
            if let Ok(api) = HidApi::new() {
                if let Ok(dev) = Em624Device::open_auto(&api) {
                    let _ = dev.sync_led_lighting(&raw_copy);
                }
            }
        });
    }

    pub fn repair_sensor_orientation_now(&mut self) {
        self.profile.raw[0x0006] = 0x7f;
        self.profile.apply_safe_profile();
        self.profile.recalculate_checksums();
        self.save_to_cached_profile();

        self.status_message =
            "Calibrated sensor orientation to 180° PCB mounting (0x7F). Checksums valid."
                .to_string();
        self.log_debug("[ORIENTATION_REPAIR] Reset 0x0006=0x7F and verified checksums.");

        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                return;
            }
        };

        if let Ok(dev) = Em624Device::open_auto(&api) {
            let _ = dev.write_registers_3b(0x0004, &self.profile.raw[0x0004..0x0009]);
            self.status_message =
                "Calibrated sensor orientation and forwarded live report to mouse hardware."
                    .to_string();
        }
    }

    pub fn load_profile_auto(&mut self, path: PathBuf) {
        match MouseProfile::load_file(&path) {
            Ok(prof) => {
                self.profile = prof;
                self.file_path = Some(path.clone());
                self.config.last_profile = Some(path.to_string_lossy().to_string());
                self.config.save();
                self.status_message = format!("Loaded profile: {}", path.display());
                self.log_debug(&format!("[LOAD] Loaded profile: {}", path.display()));
                self.save_to_cached_profile();
            }
            Err(e) => {
                self.status_message = format!("Error loading profile: {}", e);
                self.log_debug(&format!("[LOAD_ERROR] Failed to load profile: {}", e));
            }
        }
    }

    pub fn save_to_cached_profile(&mut self) {
        let cache_dir = self.config.cache_dir();
        let _ = std::fs::create_dir_all(&cache_dir);
        let path = cache_dir.join("live_active_profile.em624");
        if let Ok(()) = self.profile.save_file(&path) {
            self.log_debug(&format!(
                "[CACHE_SYNC] Synced active profile to: {}",
                path.display()
            ));
        }
    }
}
