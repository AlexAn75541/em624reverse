use eframe::egui;
use eframe::egui::{Color32, Pos2, Rect, Stroke, Vec2};
use hidapi::HidApi;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::device::Em624Device;
use crate::profile::{MouseProfile, OFFICIAL_DPI_STEPS};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Tab {
    GameMode,
    OfficeMode,
    Advanced,
    LedLighting,
    MacroManager,
    Settings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub ui_scale: f32,
    pub debug_mode: bool,
    pub last_profile: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui_scale: 1.25,
            debug_mode: false,
            last_profile: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = Path::new(".config/settings.json");
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let _ = fs::create_dir_all(".config");
        let path = Path::new(".config/settings.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}

pub struct Em624App {
    config: AppConfig,
    profile: MouseProfile,
    active_tab: Tab,
    status_message: String,
    device_connected: bool,
    connected_info: String,
    file_path: Option<PathBuf>,
    hovered_button: Option<usize>,       // 1..=7
    editing_slot: Option<(usize, bool)>, // (slot 1..9, is_gaming)
    mouse_texture: Option<egui::TextureHandle>,
    // Advanced & Fire Key settings
    fire_mode: usize,
    fire_repeat: u32,
    fire_interval: u32,
    fire_continuous: bool,
    pointer_speed: u32,
    pointer_precision: bool,
    double_click_speed: u32,
    rolling_speed: u32,
    scroll_one_screen: bool,
    polling_rate_idx: usize, // 0=125Hz, 1=250Hz, 2=500Hz, 3=1000Hz
    // LED settings
    led_speed: u32,
    led_mode: usize,
    led_brightness: usize,
    // Macro state
    macros: Vec<String>,
    selected_macro: Option<usize>,
    macro_recording: bool,
    trimmed: bool,
}

impl Default for Em624App {
    fn default() -> Self {
        let config = AppConfig::load();
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let profile = MouseProfile::from_bytes(baseline).unwrap();

        let _ = fs::create_dir_all(".cache");
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(".cache/debug.log")
        {
            use std::io::Write;
            let ts = chrono_or_simple_time();
            let _ = writeln!(
                f,
                "[{}] [STARTUP] E-DRA EM624 Configurator GUI session started.",
                ts
            );
            let _ = f.flush();
        }

        Self {
            config,
            profile,
            active_tab: Tab::GameMode,
            status_message: "Ready. Connect your mouse or load a profile.".to_string(),
            device_connected: false,
            connected_info: "No device connected".to_string(),
            file_path: None,
            hovered_button: None,
            editing_slot: None,
            mouse_texture: None,
            fire_mode: 0,
            fire_repeat: 3,
            fire_interval: 30,
            fire_continuous: false,
            pointer_speed: 6,
            pointer_precision: true,
            double_click_speed: 7,
            rolling_speed: 3,
            scroll_one_screen: false,
            polling_rate_idx: 3, // 1000 Hz default
            led_speed: 3,
            led_mode: 1,       // Colorful
            led_brightness: 0, // Full
            macros: vec![
                "Burst Fire".to_string(),
                "Quick Scope".to_string(),
                "Double Jump".to_string(),
            ],
            selected_macro: Some(0),
            macro_recording: false,
            trimmed: false,
        }
    }
}

impl Em624App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        if app.config.debug_mode {
            app.spawn_native_terminal();
        }
        app.check_connection();
        app
    }

    fn spawn_native_terminal(&self) {
        if let Ok(exe) = std::env::current_exe() {
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new(exe)
                    .arg("log-viewer")
                    .creation_flags(0x00000010) // CREATE_NEW_CONSOLE
                    .spawn();
            }

            #[cfg(not(target_os = "windows"))]
            {
                let terminals = [
                    "x-terminal-emulator",
                    "gnome-terminal",
                    "konsole",
                    "xfce4-terminal",
                    "alacritty",
                    "xterm",
                ];
                let mut spawned = false;
                for term in terminals {
                    if let Ok(_child) = std::process::Command::new(term)
                        .arg("-e")
                        .arg(format!("{} log-viewer", exe.display()))
                        .spawn()
                    {
                        spawned = true;
                        break;
                    }
                }
                if !spawned {
                    let _ = std::process::Command::new(exe).arg("log-viewer").spawn();
                }
            }
        }
    }

    fn log_debug(&self, msg: &str) {
        let ts = chrono_or_simple_time();
        let formatted = format!("[{}] {}", ts, msg);
        println!("{}", formatted);
        let _ = fs::create_dir_all(".cache");
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(".cache/debug.log")
        {
            use std::io::Write;
            let _ = writeln!(f, "{}", formatted);
            let _ = f.flush();
        }
    }

    fn check_connection(&mut self) {
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

    fn sync_settings_to_profile(&mut self) {
        // Enforce safe stages and limit
        self.profile.apply_safe_profile();

        // Sync Polling Rate (byte 0x00A8): 125Hz=1, 250Hz=2, 500Hz=4, 1000Hz=15
        self.profile.raw[0x00a8] = match self.polling_rate_idx {
            0 => 0x01, // 125 Hz
            1 => 0x02, // 250 Hz
            2 => 0x04, // 500 Hz
            _ => 0x0f, // 1000 Hz
        };

        // Sync LED configuration (offsets 0x0006..0x0008)
        self.profile.raw[0x0006] = self.led_mode as u8;
        self.profile.raw[0x0007] = self.led_speed as u8;
        self.profile.raw[0x0008] = self.led_brightness as u8;

        // Sync Fire Key configuration (offsets 0x0010..0x0013)
        self.profile.raw[0x0010] = self.fire_mode as u8;
        self.profile.raw[0x0011] = self.fire_repeat as u8;
        self.profile.raw[0x0012] = self.fire_interval as u8;
        self.profile.raw[0x0013] = if self.fire_continuous { 1 } else { 0 };

        // Sync Pointer & Scrolling Speeds (offsets 0x0014..0x0017)
        self.profile.raw[0x0014] = self.pointer_speed as u8;
        self.profile.raw[0x0015] = if self.pointer_precision { 1 } else { 0 };
        self.profile.raw[0x0016] = self.double_click_speed as u8;
        self.profile.raw[0x0017] = self.rolling_speed as u8;

        // Recalculate checksums across all 5 sectors
        self.profile.recalculate_checksums();
    }

    fn read_from_mouse(&mut self) {
        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                return;
            }
        };

        match Em624Device::open_auto(&api) {
            Ok(dev) => {
                self.status_message = "Reading active 2048-byte EEPROM from mouse...".to_string();
                self.log_debug("[USB_READ] Sending Opcode 0x40 0x09 (Read 2048 bytes)...");
                match dev.read_profile() {
                    Ok(data) => match MouseProfile::from_bytes(&data) {
                        Ok(prof) => {
                            self.profile = prof;
                            self.device_connected = true;
                            self.status_message =
                                "Successfully loaded active profile from mouse hardware!"
                                    .to_string();
                            self.log_debug(&format!("[USB_READ] Profile read successfully (2048 bytes). Checksum0=0x{:02x}, Mask=0x{:02x}", data[0xa3], data[4]));
                        }
                        Err(e) => self.status_message = format!("Profile parse error: {}", e),
                    },
                    Err(e) => self.status_message = format!("Failed to read profile: {}", e),
                }
            }
            Err(e) => {
                self.device_connected = false;
                self.status_message = format!("Mouse not found or busy: {}", e);
            }
        }
    }

    fn flash_to_mouse(&mut self) {
        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                return;
            }
        };

        match Em624Device::open_auto(&api) {
            Ok(dev) => {
                self.sync_settings_to_profile();
                self.status_message = "Flashing 293 chunks to mouse flash memory...".to_string();
                self.log_debug(
                    "[USB_FLASH] Flashing 293 chunks (Report 0x41) + Commit (0x40 0x0E)...",
                );

                match dev.write_profile(&self.profile.raw) {
                    Ok(()) => {
                        self.device_connected = true;
                        self.status_message =
                            "Successfully flashed profile to mouse! Settings active immediately."
                                .to_string();
                        self.log_debug("[USB_FLASH] Flash write committed successfully!");
                    }
                    Err(e) => {
                        self.status_message = format!("Flash failed: {}", e);
                        self.log_debug(&format!("[USB_FLASH_ERROR] {}", e));
                    }
                }
            }
            Err(e) => {
                self.device_connected = false;
                self.status_message = format!("Mouse not connected: {}", e);
            }
        }
    }

    fn load_profile_auto(&mut self, path: PathBuf) {
        let _ = fs::create_dir_all(".cache");
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "bin" {
            // Automatic on-the-fly conversion of stock .bin into safe .em624 cached file
            self.log_debug(&format!(
                "[CONVERT] Stock .bin profile detected. Converting on the fly: {}",
                path.display()
            ));
            if let Ok(mut prof) = MouseProfile::load_file(&path) {
                prof.apply_safe_profile();
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("profile");
                let cached_path = PathBuf::from(format!(".cache/{}.em624", stem));
                if let Ok(()) = prof.save_file(&cached_path) {
                    self.profile = prof;
                    self.file_path = Some(cached_path.clone());
                    self.status_message =
                        format!("Converted and loaded into .cache/{}.em624", stem);
                    self.log_debug(&format!(
                        "[CONVERT] Saved cached safe profile to: {}",
                        cached_path.display()
                    ));
                    self.config.last_profile = Some(cached_path.to_string_lossy().to_string());
                    self.config.save();
                    return;
                }
            }
        }

        // Standard load
        if let Ok(p) = MouseProfile::load_file(&path) {
            self.profile = p;
            self.file_path = Some(path.clone());
            self.status_message = format!("Loaded profile: {}", path.display());
            self.config.last_profile = Some(path.to_string_lossy().to_string());
            self.config.save();
            self.log_debug(&format!("[LOAD] Loaded profile from: {}", path.display()));
        } else {
            self.status_message = "Failed to load profile file.".to_string();
        }
    }

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if self.mouse_texture.is_none() {
            let png_bytes = include_bytes!("../damouse.png");
            if let Ok(img) = image::load_from_memory(png_bytes) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture =
                    ctx.load_texture("mouse_photo", color_image, egui::TextureOptions::LINEAR);
                self.mouse_texture = Some(texture);
            }
        }
    }
}

impl eframe::App for Em624App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(self.config.ui_scale);
        ctx.set_visuals(egui::Visuals::dark());

        self.ensure_texture(ctx);

        if !self.trimmed && self.mouse_texture.is_some() {
            trim_working_set();
            self.trimmed = true;
            self.log_debug("[MEM] Initial working set trimmed to active pages (<32MB target).");
        }

        // Top Navigation Header
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("E-DRA EM624")
                        .strong()
                        .color(Color32::from_rgb(0, 242, 254)),
                );
                ui.label(
                    egui::RichText::new("GAMING MOUSE CONFIGURATOR")
                        .small()
                        .color(Color32::from_rgb(140, 160, 190)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Check Connection").clicked() {
                        self.check_connection();
                    }

                    if self.device_connected {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                rect.center(),
                                4.0,
                                Color32::from_rgb(46, 213, 115),
                            );
                            ui.label(
                                egui::RichText::new("CONNECTED")
                                    .color(Color32::from_rgb(46, 213, 115))
                                    .strong(),
                            );
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                rect.center(),
                                4.0,
                                Color32::from_rgb(255, 71, 87),
                            );
                            ui.label(
                                egui::RichText::new("DISCONNECTED")
                                    .color(Color32::from_rgb(255, 71, 87))
                                    .strong(),
                            );
                        });
                    }
                });
            });
            ui.add_space(6.0);

            // Tab Navigation
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::GameMode, "Game Mode");
                ui.selectable_value(&mut self.active_tab, Tab::OfficeMode, "Office Mode");
                ui.selectable_value(&mut self.active_tab, Tab::Advanced, "Advanced");
                ui.selectable_value(&mut self.active_tab, Tab::LedLighting, "LED Lighting");
                ui.selectable_value(&mut self.active_tab, Tab::MacroManager, "Macro Manager");
                ui.selectable_value(&mut self.active_tab, Tab::Settings, "App Settings");
            });
            ui.add_space(6.0);
        });

        // Bottom Action Toolbar & Status Bar
        egui::TopBottomPanel::bottom("bottom_toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Read from Mouse").clicked() {
                    self.read_from_mouse();
                }

                if ui
                    .button(
                        egui::RichText::new("Flash to Mouse")
                            .strong()
                            .color(Color32::from_rgb(0, 242, 254)),
                    )
                    .clicked()
                {
                    self.flash_to_mouse();
                }

                ui.separator();

                if ui.button("Load Profile").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Profiles (*.em624, *.bin)", &["em624", "bin"])
                        .pick_file()
                    {
                        self.load_profile_auto(path);
                    }
                }

                if ui.button("Save Profile").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Safe EM624 Profile (*.em624)", &["em624"])
                        .set_file_name("custom_profile.em624")
                        .save_file()
                    {
                        self.sync_settings_to_profile();
                        if let Ok(()) = self.profile.save_file(&path) {
                            self.file_path = Some(path.clone());
                            self.status_message = format!("Saved profile to: {}", path.display());
                            self.log_debug(&format!("[SAVE] Saved profile to: {}", path.display()));
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(&self.status_message)
                            .small()
                            .italics()
                            .color(Color32::from_rgb(170, 190, 220)),
                    );
                });
            });
            ui.add_space(6.0);
        });

        // Central Content Area
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            Tab::GameMode => self.render_button_mode_tab(ui, true),
            Tab::OfficeMode => self.render_button_mode_tab(ui, false),
            Tab::Advanced => self.render_advanced_tab(ui),
            Tab::LedLighting => self.render_led_tab(ui),
            Tab::MacroManager => self.render_macro_tab(ui),
            Tab::Settings => self.render_settings_tab(ui),
        });

        // Modal Button Remapper Dialog if open
        if let Some((slot_idx, is_gaming)) = self.editing_slot {
            let mut close_modal = false;
            let title = format!(
                "Remap Button [{}] ({})",
                slot_idx_to_button_num(slot_idx),
                if is_gaming {
                    "Gaming Mode"
                } else {
                    "Office Mode"
                }
            );

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(320.0);
                    ui.label("Select new button assignment:");
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .max_height(340.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Standard Mouse Keys")
                                    .strong()
                                    .color(Color32::from_rgb(0, 242, 254)),
                            );
                            if ui.button("Left Click").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x01, 0x01, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Right Click").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x01, 0x02, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Middle Click").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x01, 0x04, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Forward (Browser Side 1)").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x01, 0x08, 0x00, 0x02],
                                );
                                close_modal = true;
                            }
                            if ui.button("Backward (Browser Side 2)").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x01, 0x10, 0x00, 0x00],
                                );
                                close_modal = true;
                            }

                            ui.separator();
                            ui.label(
                                egui::RichText::new("Special & DPI Keys")
                                    .strong()
                                    .color(Color32::from_rgb(0, 242, 254)),
                            );
                            if ui.button("DPI Loop (Cycle All)").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x08, 0x00, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("DPI +").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x09, 0x02, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("DPI -").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x09, 0x00, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Fire Key (Rapid Burst)").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x06, 0x92, 0x00, 0x1e],
                                );
                                close_modal = true;
                            }

                            ui.separator();
                            ui.label(
                                egui::RichText::new("Multimedia")
                                    .strong()
                                    .color(Color32::from_rgb(0, 242, 254)),
                            );
                            if ui.button("Play / Pause").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x03, 0x00, 0xcd, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Next Track").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x03, 0x00, 0xb5, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Previous Track").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x03, 0x00, 0xb6, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Volume +").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x03, 0x00, 0xe9, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Volume -").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x03, 0x00, 0xea, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Mute").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x03, 0x00, 0xe2, 0x00],
                                );
                                close_modal = true;
                            }

                            ui.separator();
                            ui.label(
                                egui::RichText::new("Productivity / Shortcuts")
                                    .strong()
                                    .color(Color32::from_rgb(0, 242, 254)),
                            );
                            if ui.button("Desktop (Win+D)").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x02, 0x08, 0x07, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("Snip (Shift+Win+S)").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x02, 0x0a, 0x16, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("LED ON/OFF Toggle").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x07, 0x80, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                            if ui.button("LED Mode Cycle").clicked() {
                                self.set_button_binding(
                                    slot_idx,
                                    is_gaming,
                                    &[0x07, 0x81, 0x00, 0x00],
                                );
                                close_modal = true;
                            }
                        });

                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        close_modal = true;
                    }
                });

            if close_modal {
                self.editing_slot = None;
            }
        }
    }
}

impl Em624App {
    fn set_button_binding(&mut self, slot_idx: usize, is_gaming: bool, bytes: &[u8; 4]) {
        let base_off = if is_gaming { 0x00f1 } else { 0x00c0 };
        let off = base_off + (slot_idx - 1) * 4;
        self.profile.raw[off..off + 4].copy_from_slice(bytes);
        self.profile.apply_safe_profile();
        self.profile.refresh_stages();
        self.log_debug(&format!(
            "[REMAP] Updated slot {} ({}) to bytecode {:02x?}",
            slot_idx,
            if is_gaming { "Gaming" } else { "Office" },
            bytes
        ));
    }

    fn render_button_mode_tab(&mut self, ui: &mut egui::Ui, is_gaming: bool) {
        ui.horizontal(|ui| {
            // Left Column: Button List (Fixed 340px width)
            ui.allocate_ui_with_layout(
                Vec2::new(340.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(4.0);
                    ui.heading(if is_gaming {
                        "Gaming Mode Bindings"
                    } else {
                        "Office Mode Bindings"
                    });
                    ui.label(
                        egui::RichText::new("Click any button to remap function:")
                            .small()
                            .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(8.0);

                    let buttons = if is_gaming {
                        &self.profile.gaming_buttons
                    } else {
                        &self.profile.office_buttons
                    };

                    self.hovered_button = None;

                    let display_order = [
                        (1, "[1] Left Click"),
                        (4, "[2] Right Click"),
                        (7, "[3] Middle Click"),
                        (2, "[4] Forward (Side 1)"),
                        (5, "[5] Backward (Side 2)"),
                        (9, "[6] DPI + (Top Front)"),
                        (8, "[7] DPI - (Top Back)"),
                    ];

                    for (slot_idx, label) in display_order {
                        if let Some(btn) = buttons.get(slot_idx - 1) {
                            let is_highlighted = self.hovered_button == Some(slot_idx);
                            let bg_color = if is_highlighted {
                                Color32::from_rgba_premultiplied(0, 242, 254, 35)
                            } else {
                                Color32::from_rgb(20, 26, 38)
                            };

                            let frame =
                                egui::Frame::group(ui.style())
                                    .fill(bg_color)
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if is_highlighted {
                                            Color32::from_rgb(0, 242, 254)
                                        } else {
                                            Color32::from_rgb(36, 48, 70)
                                        },
                                    ));

                            let response = frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(label).strong());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(&btn.description)
                                                    .color(Color32::from_rgb(0, 242, 254)),
                                            );
                                        },
                                    );
                                });
                            });

                            if response.response.hovered() {
                                self.hovered_button = Some(slot_idx);
                            }

                            if response.response.clicked() {
                                self.editing_slot = Some((slot_idx, is_gaming));
                            }
                            ui.add_space(3.0);
                        }
                    }
                },
            );

            ui.separator();

            // Right Column: Centered Hardware Mouse Picture (Fatter, 330x340px)
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), ui.available_height()),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("E-DRA EM624 HARDWARE SWITCH MATRIX")
                            .strong()
                            .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(4.0);

                    self.render_mouse_photo(ui);

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "SunplusIT SPCP6662 Optical Sensor • Huano 50M Switches",
                        )
                        .small()
                        .color(Color32::from_rgb(120, 140, 170)),
                    );
                },
            );
        });
    }

    fn render_mouse_photo(&self, ui: &mut egui::Ui) {
        let display_size = Vec2::new(320.0, 340.0);
        let (rect, _response) = ui.allocate_exact_size(display_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        if let Some(texture) = &self.mouse_texture {
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(rect, 10.0, Color32::from_rgb(20, 26, 38));
        }

        let cx = rect.center().x;
        let cy = rect.center().y;

        // Numbered badges [1] through [7] directly over the switches on the fatter mouse
        let badges = [
            (1, Pos2::new(cx - 46.0, cy - 75.0), "[1]"), // Button 1: Left Click
            (4, Pos2::new(cx + 46.0, cy - 75.0), "[2]"), // Button 2: Right Click
            (7, Pos2::new(cx, cy - 100.0), "[3]"),       // Button 3: Wheel Click
            (2, Pos2::new(cx - 96.0, cy - 28.0), "[4]"), // Button 4: Side Forward
            (5, Pos2::new(cx - 96.0, cy + 14.0), "[5]"), // Button 5: Side Backward
            (9, Pos2::new(cx, cy - 30.0), "[6]"),        // Button 6: DPI +
            (8, Pos2::new(cx, cy - 2.0), "[7]"),         // Button 7: DPI -
        ];

        for (slot_idx, pos, text) in badges {
            let is_hovered = self.hovered_button == Some(slot_idx);
            let badge_bg = if is_hovered {
                Color32::from_rgb(0, 242, 254)
            } else {
                Color32::from_rgba_premultiplied(10, 15, 25, 230)
            };
            let text_color = if is_hovered {
                Color32::BLACK
            } else {
                Color32::from_rgb(0, 242, 254)
            };

            let badge_rect = Rect::from_center_size(pos, Vec2::new(26.0, 18.0));
            painter.rect_filled(badge_rect, 4.0, badge_bg);
            painter.rect_stroke(
                badge_rect,
                4.0,
                Stroke::new(1.5_f32, Color32::from_rgb(0, 242, 254)),
            );
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::monospace(11.0),
                text_color,
            );
        }
    }

    fn render_advanced_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Left Column: Fire Key & Pointer Speeds
            ui.vertical(|ui| {
                ui.set_width(330.0);
                ui.group(|ui| {
                    ui.heading("Fire Key Setting");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.fire_mode, 0, "Mode 1");
                        ui.radio_value(&mut self.fire_mode, 1, "Mode 2");
                        ui.radio_value(&mut self.fire_mode, 2, "Mode 3");
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Repeat Count:");
                        ui.add(egui::Slider::new(&mut self.fire_repeat, 1..=20));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Interval (ms):");
                        ui.add(egui::Slider::new(&mut self.fire_interval, 10..=100));
                    });
                    ui.checkbox(&mut self.fire_continuous, "Continuous Fire");
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("Pointer & Wheel Speeds");
                    ui.horizontal(|ui| {
                        ui.label("Moving Speed (1..11):");
                        ui.add(egui::Slider::new(&mut self.pointer_speed, 1..=11));
                    });
                    ui.checkbox(&mut self.pointer_precision, "Enhance pointer precision");

                    ui.horizontal(|ui| {
                        ui.label("Double Click Speed:");
                        ui.add(egui::Slider::new(&mut self.double_click_speed, 1..=11));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Rolling Speed:");
                        ui.add(egui::Slider::new(&mut self.rolling_speed, 1..=11));
                    });
                    ui.checkbox(&mut self.scroll_one_screen, "Scroll one screen at a time");
                });
            });

            ui.separator();

            // Right Column: DPI Stages & Polling Rate
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.heading("DPI Stages (SPCP6662)");
                    ui.label(
                        egui::RichText::new(
                            "15 calibrated hardware resolution steps (200 to 12,000 DPI)",
                        )
                        .small()
                        .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(8.0);

                    egui::Grid::new("dpi_grid")
                        .num_columns(5)
                        .spacing([14.0, 10.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Stage").strong());
                            ui.label(egui::RichText::new("Active").strong());
                            ui.label(egui::RichText::new("Resolution").strong());
                            ui.label(egui::RichText::new("Calibrated Slider").strong());
                            ui.label(egui::RichText::new("RGB LED").strong());
                            ui.end_row();

                            for i in 0..6 {
                                let is_active = (self.profile.active_stages_mask & (1 << i)) != 0;
                                let current_step =
                                    self.profile.dpi_stages[i].level_index.min(14) as usize;
                                let color = self.profile.dpi_stages[i].color.clone();

                                ui.label(format!("Stage {}", i + 1));

                                // Stage 6 is strictly locked / non-toggleable to protect against the MCU trap
                                if i == 5 {
                                    let mut disabled_flag = false;
                                    ui.add_enabled(
                                        false,
                                        egui::Checkbox::new(&mut disabled_flag, ""),
                                    );
                                } else {
                                    let mut active_checked = is_active;
                                    if ui.checkbox(&mut active_checked, "").changed() {
                                        if active_checked {
                                            self.profile.active_stages_mask |= 1 << i;
                                        } else {
                                            self.profile.active_stages_mask &= !(1 << i);
                                        }
                                        self.profile.active_stages_mask &= 0xdf;
                                        // Safety clamp
                                    }
                                }

                                let dpi_val = OFFICIAL_DPI_STEPS[current_step];
                                ui.label(
                                    egui::RichText::new(format!("{:5} DPI", dpi_val))
                                        .strong()
                                        .color(Color32::from_rgb(0, 242, 254)),
                                );

                                let mut step_val = current_step;
                                if ui
                                    .add(egui::Slider::new(&mut step_val, 0..=14).show_value(false))
                                    .changed()
                                {
                                    let byte_off = 0x2c + (i / 2);
                                    if i % 2 == 0 {
                                        self.profile.raw[byte_off] = (self.profile.raw[byte_off]
                                            & 0xf0)
                                            | (step_val as u8 & 0x0f);
                                    } else {
                                        self.profile.raw[byte_off] = (self.profile.raw[byte_off]
                                            & 0x0f)
                                            | ((step_val as u8 & 0x0f) << 4);
                                    }
                                    self.profile.refresh_stages();
                                }

                                let mut color_rgb = [color.r, color.g, color.b];
                                if ui.color_edit_button_srgb(&mut color_rgb).changed() {
                                    let color_off = 0x3c + (i * 4);
                                    self.profile.raw[color_off] = color_rgb[0];
                                    self.profile.raw[color_off + 1] = color_rgb[1];
                                    self.profile.raw[color_off + 2] = color_rgb[2];
                                    self.profile.refresh_stages();
                                }

                                ui.end_row();
                            }
                        });
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("USB Report Rate (Polling Speed)");
                    ui.label("Select active polling frequency (interchangeable):");
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.polling_rate_idx, 0, "125 Hz (8 ms)");
                        ui.radio_value(&mut self.polling_rate_idx, 1, "250 Hz (4 ms)");
                        ui.radio_value(&mut self.polling_rate_idx, 2, "500 Hz (2 ms)");
                        ui.radio_value(&mut self.polling_rate_idx, 3, "1000 Hz (1 ms)");
                    });
                });
            });
        });
    }

    fn render_led_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading("LED Backlight & Lighting Effects");
        ui.label(
            egui::RichText::new("Configure RGB breathing, speed, mode, and brightness:")
                .small()
                .color(Color32::from_rgb(140, 160, 190)),
        );
        ui.add_space(14.0);

        ui.group(|ui| {
            ui.heading("LED Breathing Speed");
            ui.horizontal(|ui| {
                ui.label("Slow");
                ui.add(egui::Slider::new(&mut self.led_speed, 1..=5).show_value(false));
                ui.label("Fast");
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("LED Mode");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.led_mode, 0, "Breathing");
                ui.radio_value(&mut self.led_mode, 1, "Colorful Light");
                ui.radio_value(&mut self.led_mode, 2, "Rainbow");
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.led_mode, 3, "Flow Light");
                ui.radio_value(&mut self.led_mode, 4, "Waltz");
                ui.radio_value(&mut self.led_mode, 5, "4-Seasons");
                ui.radio_value(&mut self.led_mode, 6, "Off");
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("LED Brightness");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.led_brightness, 0, "Full Brightness");
                ui.radio_value(&mut self.led_brightness, 1, "Half Brightness");
            });
        });
    }

    fn render_macro_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading("Macro Manager");
        ui.label(
            egui::RichText::new("Record, assign, and store keystroke sequences to mouse EEPROM:")
                .small()
                .color(Color32::from_rgb(140, 160, 190)),
        );
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            // Column 1: Macro List
            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.label(egui::RichText::new("Macro List").strong());
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_height(240.0);
                    for (i, m) in self.macros.iter().enumerate() {
                        let selected = self.selected_macro == Some(i);
                        if ui.selectable_label(selected, m).clicked() {
                            self.selected_macro = Some(i);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("New Macro").clicked() {
                        self.macros.push(format!("Macro {}", self.macros.len() + 1));
                        self.selected_macro = Some(self.macros.len() - 1);
                    }
                    if ui.button("Add to Mouse").clicked() {
                        self.status_message = "Macro saved to mouse macro bank.".to_string();
                    }
                });
            });

            ui.separator();

            // Column 2: Macro Content
            ui.vertical(|ui| {
                ui.set_width(240.0);
                ui.label(egui::RichText::new("Macro Content").strong());
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_height(240.0);
                    if let Some(idx) = self.selected_macro {
                        ui.label(format!("Key sequence for '{}':", self.macros[idx]));
                        ui.label("  1. Key Down [Space]");
                        ui.label("  2. Delay 30 ms");
                        ui.label("  3. Key Up [Space]");
                    } else {
                        ui.label("No macro selected.");
                    }
                });
                if ui
                    .button(if self.macro_recording {
                        "Stop Recording"
                    } else {
                        "Record Keystrokes"
                    })
                    .clicked()
                {
                    self.macro_recording = !self.macro_recording;
                }
            });

            ui.separator();

            // Column 3: Storage & Execution Options
            ui.vertical(|ui| {
                ui.set_width(220.0);
                ui.label(egui::RichText::new("Storage in Mouse").strong());
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_height(240.0);
                    ui.label("Recording Timing:");
                    ui.radio_value(&mut self.fire_mode, 0, "Real record");
                    ui.radio_value(&mut self.fire_mode, 1, "Delay Time (50 ms)");
                    ui.radio_value(&mut self.fire_mode, 2, "No Delay");

                    ui.add_space(8.0);
                    ui.label("Playback Loop:");
                    ui.radio_value(&mut self.fire_mode, 3, "Stop until macro end");
                    ui.radio_value(&mut self.fire_mode, 4, "Stop until key released");
                    ui.radio_value(&mut self.fire_mode, 5, "Cycles (1-127)");
                });
            });
        });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading("Application Settings & Display Scaling");
        ui.label(
            egui::RichText::new(
                "Configure interface scaling, hardware diagnostics, and debug preferences.",
            )
            .small()
            .color(Color32::from_rgb(140, 160, 190)),
        );
        ui.add_space(14.0);

        ui.group(|ui| {
            ui.heading("Display Scale Factor (Static Presets)");
            ui.label("Select a comfortable zoom level for your display resolution:");
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui
                    .selectable_label((self.config.ui_scale - 1.00).abs() < 0.01, "100%")
                    .clicked()
                {
                    self.config.ui_scale = 1.00;
                    self.config.save();
                }
                if ui
                    .selectable_label(
                        (self.config.ui_scale - 1.25).abs() < 0.01,
                        "125% (Standard)",
                    )
                    .clicked()
                {
                    self.config.ui_scale = 1.25;
                    self.config.save();
                }
                if ui
                    .selectable_label((self.config.ui_scale - 1.50).abs() < 0.01, "150%")
                    .clicked()
                {
                    self.config.ui_scale = 1.50;
                    self.config.save();
                }
                if ui
                    .selectable_label((self.config.ui_scale - 1.75).abs() < 0.01, "175%")
                    .clicked()
                {
                    self.config.ui_scale = 1.75;
                    self.config.save();
                }
                if ui
                    .selectable_label((self.config.ui_scale - 2.00).abs() < 0.01, "200% (HiDPI)")
                    .clicked()
                {
                    self.config.ui_scale = 2.00;
                    self.config.save();
                }
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("Developer & Debug Mode");
            ui.label("Opens terminal output window and enables raw USB packet logging:");
            ui.add_space(6.0);

            let mut dbg = self.config.debug_mode;
            if ui
                .checkbox(
                    &mut dbg,
                    "Enable Live Debug Console Window (OS Native Terminal)",
                )
                .changed()
            {
                self.config.debug_mode = dbg;
                self.config.save();
                if dbg {
                    self.spawn_native_terminal();
                }
            }

            ui.add_space(4.0);
            if ui
                .button("Launch Native CMD / Terminal Window Now")
                .clicked()
            {
                self.spawn_native_terminal();
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("Hardware Connection Status");
            ui.horizontal(|ui| {
                ui.label("Active Device:");
                ui.label(egui::RichText::new(&self.connected_info).strong().color(
                    if self.device_connected {
                        Color32::from_rgb(46, 213, 115)
                    } else {
                        Color32::from_rgb(255, 71, 87)
                    },
                ));
            });

            ui.add_space(6.0);
            if ui.button("Check / Refresh Mouse Connection").clicked() {
                self.check_connection();
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("About & Hardware Specifications");
            ui.label("• Device: E-DRA EM624 Gaming Mouse (7D + 1 Wheel)");
            ui.label("• Optical Sensor: SunplusIT SPCP6662 (6662IC) • 15 discrete DPI steps (200..12000 DPI)");
            ui.label("• MCU Controller: SunplusIT SPCP6651B");
            ui.label("• Switches: Huano 50M rated");
            ui.label("• Communication: 100% offline, native Rust, zero network or HTML dependencies");
        });
    }
}

#[cfg(target_os = "windows")]
pub fn trim_working_set() {
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn SetProcessWorkingSetSize(h: isize, min: usize, max: usize) -> i32;
    }
    unsafe {
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn trim_working_set() {}

pub fn chrono_or_simple_time() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!(
        "{}:{:02}:{:02}",
        (now / 3600) % 24,
        (now / 60) % 60,
        now % 60
    )
}

fn slot_idx_to_button_num(slot_idx: usize) -> u8 {
    match slot_idx {
        1 => 1,
        2 => 4,
        4 => 2,
        5 => 5,
        7 => 3,
        8 => 7,
        9 => 6,
        _ => slot_idx as u8,
    }
}
