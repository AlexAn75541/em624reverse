use eframe::egui;
use eframe::egui::{Color32, Pos2, Rect, Stroke, Vec2};
use hidapi::HidApi;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::device::Em624Device;
use crate::i18n::{Language, Strings};
use crate::macro_engine::{MacroAction, MacroBank, MacroEntry, MouseButton};
use crate::profile::{format_hid_key_name, format_modifiers, MouseProfile, OFFICIAL_DPI_STEPS};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Tab {
    GameMode,
    OfficeMode,
    Advanced,
    LedLighting,
    MacroManager,
    Settings,
}

#[derive(Default, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    PitchBlackOled,
    Windows11Dark,
    LightWhite,
}

#[allow(dead_code)]
pub struct ThemePalette {
    pub bg_base: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub mouse_bg: Color32,
    pub badge_bg: Color32,
    pub badge_text: Color32,
    pub badge_border: Color32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub ui_scale: f32,
    pub debug_mode: bool,
    pub last_profile: Option<String>,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub custom_data_dir: Option<String>,
    #[serde(default)]
    pub allow_stage_6: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui_scale: 1.25,
            debug_mode: false,
            last_profile: None,
            theme: ThemeMode::PitchBlackOled,
            language: Language::English,
            custom_data_dir: None,
            allow_stage_6: false,
        }
    }
}

impl AppConfig {
    pub fn get_master_data_dir(&self) -> PathBuf {
        if let Some(ref custom) = self.custom_data_dir {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.file_name().map(|n| n == "em624").unwrap_or(false) {
                    return p;
                } else {
                    return p.join("em624");
                }
            }
        }
        PathBuf::from("em624")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.get_master_data_dir().join(".config")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.get_master_data_dir().join(".cache")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.get_master_data_dir().join(".logs")
    }

    pub fn pointer_file() -> PathBuf {
        PathBuf::from("em624/.config/settings_path.txt")
    }

    pub fn load() -> Self {
        let ptr_file = Self::pointer_file();
        if let Ok(ptr_content) = fs::read_to_string(&ptr_file) {
            let target_cfg = PathBuf::from(ptr_content.trim());
            if target_cfg.exists() {
                if let Ok(data) = fs::read_to_string(&target_cfg) {
                    if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                        return cfg;
                    }
                }
            }
        }

        let default_path = Path::new("em624/.config/settings.json");
        if let Ok(data) = fs::read_to_string(default_path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                return cfg;
            }
        }

        let legacy_path = Path::new(".config/settings.json");
        if let Ok(data) = fs::read_to_string(legacy_path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                cfg.save();
                return cfg;
            }
        }

        let cfg = Self::default();
        cfg.save();
        cfg
    }

    pub fn save(&self) {
        let dir = self.config_dir();
        let _ = fs::create_dir_all(&dir);
        let settings_path = dir.join("settings.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&settings_path, json);
        }

        if let Some(ref _custom) = self.custom_data_dir {
            let _ = fs::create_dir_all("em624/.config");
            let _ = fs::write(
                Self::pointer_file(),
                settings_path.to_string_lossy().as_bytes(),
            );
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
    selected_slot: usize,                // currently selected button (1..=9, default 1)
    editing_slot: Option<(usize, bool)>, // (slot 1..9, is_gaming)
    mouse_texture: Option<egui::TextureHandle>,
    // Rebind wizard modal state
    rebind_category: usize,
    rebind_subtab: usize, // 0=Letters, 1=Numbers, 2=Numpad, 3=Function, 4=Navigation, 5=Symbols
    rebind_key_code: u8,
    rebind_modifiers: u8,
    rebind_candidate: Option<[u8; 4]>,
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
    // Macro state with full MacroBank
    macro_bank: MacroBank,
    selected_macro: Option<usize>,
    #[allow(dead_code)]
    macro_recording: bool,
    trimmed: bool,
    frame_count: u64,
    last_trim_instant: std::time::Instant,
    device_rx: std::sync::mpsc::Receiver<crate::platform::DeviceStatus>,
    custom_dir_input: String,
    custom_dir_error: Option<String>,
}

pub fn format_time_date_log() -> String {
    let now = chrono::Local::now();
    now.format("%H-%M-%S_%Y-%m-%d.log").to_string()
}

impl Drop for Em624App {
    fn drop(&mut self) {
        let logs_dir = self.config.logs_dir();
        let active_log = logs_dir.join("debug.log");
        if active_log.exists() {
            self.log_debug("[SESSION_END] Closing session and archiving debug log.");
            let archive_name = format_time_date_log();
            let _ = fs::rename(&active_log, logs_dir.join(archive_name));
        }
    }
}

impl Default for Em624App {
    fn default() -> Self {
        let config = AppConfig::load();
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let profile = MouseProfile::from_bytes(baseline).unwrap();

        let logs_dir = config.logs_dir();
        let _ = fs::create_dir_all(&logs_dir);
        let active_log = logs_dir.join("debug.log");

        // Rotate leftover debug.log from an unclosed previous session
        if active_log.exists() {
            let archive_name = format_time_date_log();
            let _ = fs::rename(&active_log, logs_dir.join(archive_name));
        }

        let mut macro_bank = MacroBank::default();
        let mut m1 = MacroEntry::new("Burst Fire");
        m1.actions.push(MacroAction::MouseDown(MouseButton::Left));
        m1.actions.push(MacroAction::Delay(25));
        m1.actions.push(MacroAction::MouseUp(MouseButton::Left));
        m1.actions.push(MacroAction::Delay(35));
        m1.actions.push(MacroAction::MouseDown(MouseButton::Left));
        m1.actions.push(MacroAction::Delay(25));
        m1.actions.push(MacroAction::MouseUp(MouseButton::Left));

        let mut m2 = MacroEntry::new("Quick Scope");
        m2.actions.push(MacroAction::MouseDown(MouseButton::Right));
        m2.actions.push(MacroAction::Delay(150));
        m2.actions.push(MacroAction::MouseDown(MouseButton::Left));
        m2.actions.push(MacroAction::Delay(30));
        m2.actions.push(MacroAction::MouseUp(MouseButton::Left));
        m2.actions.push(MacroAction::MouseUp(MouseButton::Right));

        let mut m3 = MacroEntry::new("Double Jump");
        m3.actions.push(MacroAction::KeyDown(0x2c)); // Space
        m3.actions.push(MacroAction::Delay(30));
        m3.actions.push(MacroAction::KeyUp(0x2c));
        m3.actions.push(MacroAction::Delay(60));
        m3.actions.push(MacroAction::KeyDown(0x2c));
        m3.actions.push(MacroAction::Delay(30));
        m3.actions.push(MacroAction::KeyUp(0x2c));

        macro_bank.macros.push(m1);
        macro_bank.macros.push(m2);
        macro_bank.macros.push(m3);

        let (_tx, device_rx) = std::sync::mpsc::channel();
        let custom_dir_input = config.custom_data_dir.clone().unwrap_or_default();

        let app = Self {
            config,
            profile,
            active_tab: Tab::GameMode,
            status_message: "Ready. Connect your mouse or load a profile.".to_string(),
            device_connected: false,
            connected_info: "No device connected".to_string(),
            file_path: None,
            hovered_button: None,
            selected_slot: 1, // Default select Button 1 (Left Click)
            editing_slot: None,
            mouse_texture: None,
            rebind_category: 0,
            rebind_subtab: 0,
            rebind_key_code: 0x04,
            rebind_modifiers: 0,
            rebind_candidate: None,
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
            macro_bank,
            selected_macro: Some(0),
            macro_recording: false,
            trimmed: false,
            frame_count: 0,
            last_trim_instant: std::time::Instant::now(),
            device_rx,
            custom_dir_input,
            custom_dir_error: None,
        };

        app.log_debug("[STARTUP] E-DRA EM624 Configurator GUI session started.");
        app
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            candidates.push(PathBuf::from(&windir).join("Fonts").join("segoeui.ttf"));
            candidates.push(PathBuf::from(&windir).join("Fonts").join("arial.ttf"));
            candidates.push(PathBuf::from(&windir).join("Fonts").join("tahoma.ttf"));
        }
        if let Ok(sysroot) = std::env::var("SYSTEMROOT") {
            candidates.push(PathBuf::from(&sysroot).join("Fonts").join("segoeui.ttf"));
            candidates.push(PathBuf::from(&sysroot).join("Fonts").join("arial.ttf"));
        }
        candidates.push(PathBuf::from("C:\\Windows\\Fonts\\segoeui.ttf"));
        candidates.push(PathBuf::from("C:\\Windows\\Fonts\\arial.ttf"));
        candidates.push(PathBuf::from("C:\\Windows\\Fonts\\tahoma.ttf"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/opentype/noto/NotoSans-Regular.ttf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        ));
    }

    let mut font_loaded = false;
    for path in &candidates {
        if path.exists() {
            if let Ok(bytes) = fs::read(path) {
                fonts.font_data.insert(
                    "system_utf8_font".to_owned(),
                    egui::FontData::from_owned(bytes),
                );

                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "system_utf8_font".to_owned());

                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("system_utf8_font".to_owned());

                font_loaded = true;
                break;
            }
        }
    }

    if font_loaded {
        ctx.set_fonts(fonts);
    }
}

impl Em624App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        let mut app = Self::default();
        app.device_rx = crate::platform::start_device_monitor();
        if app.config.debug_mode {
            app.spawn_native_terminal();
        }
        app.check_connection();
        trim_working_set();
        app
    }

    pub fn palette(&self) -> ThemePalette {
        match self.config.theme {
            ThemeMode::PitchBlackOled => ThemePalette {
                bg_base: Color32::from_rgb(0, 0, 0),
                panel_bg: Color32::from_rgb(8, 8, 8),
                card_bg: Color32::from_rgb(14, 18, 25),
                card_border: Color32::from_rgb(34, 44, 62),
                text_primary: Color32::from_rgb(245, 245, 245),
                text_secondary: Color32::from_rgb(150, 165, 185),
                accent: Color32::from_rgb(0, 242, 254),
                accent_hover: Color32::from_rgb(79, 172, 254),
                mouse_bg: Color32::from_rgb(10, 12, 18),
                badge_bg: Color32::from_rgba_premultiplied(10, 15, 25, 230),
                badge_text: Color32::from_rgb(0, 242, 254),
                badge_border: Color32::from_rgb(0, 242, 254),
            },
            ThemeMode::Windows11Dark => ThemePalette {
                bg_base: Color32::from_rgb(32, 32, 32),
                panel_bg: Color32::from_rgb(38, 38, 38),
                card_bg: Color32::from_rgb(46, 46, 46),
                card_border: Color32::from_rgb(64, 64, 64),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_secondary: Color32::from_rgb(180, 180, 180),
                accent: Color32::from_rgb(0, 242, 254),
                accent_hover: Color32::from_rgb(79, 172, 254),
                mouse_bg: Color32::from_rgb(28, 28, 28),
                badge_bg: Color32::from_rgba_premultiplied(30, 30, 30, 230),
                badge_text: Color32::from_rgb(0, 242, 254),
                badge_border: Color32::from_rgb(0, 242, 254),
            },
            ThemeMode::LightWhite => ThemePalette {
                bg_base: Color32::from_rgb(243, 244, 246),
                panel_bg: Color32::from_rgb(249, 250, 251),
                card_bg: Color32::from_rgb(255, 255, 255),
                card_border: Color32::from_rgb(209, 213, 219),
                text_primary: Color32::from_rgb(17, 24, 39),
                text_secondary: Color32::from_rgb(75, 85, 99),
                accent: Color32::from_rgb(2, 132, 199),
                accent_hover: Color32::from_rgb(3, 105, 161),
                mouse_bg: Color32::from_rgb(240, 242, 245),
                badge_bg: Color32::from_rgb(255, 255, 255),
                badge_text: Color32::from_rgb(2, 132, 199),
                badge_border: Color32::from_rgb(2, 132, 199),
            },
        }
    }

    fn change_custom_data_dir(&mut self, new_raw_path: &str) {
        let trimmed = new_raw_path.trim();
        if trimmed.is_empty() {
            self.config.custom_data_dir = None;
            self.config.save();
            self.custom_dir_input.clear();
            self.custom_dir_error = None;
            self.status_message = "Reset master data directory to default: ./em624".to_string();
            self.log_debug("[CONFIG] Master data directory reset to default: ./em624");
            return;
        }

        let target_path = PathBuf::from(trimmed);
        if let Err(e) = fs::create_dir_all(&target_path) {
            let err_msg = format!("Cannot create directory '{}': {}", target_path.display(), e);
            self.custom_dir_error = Some(err_msg.clone());
            self.log_debug(&format!("[WARNING] Unsuitable directory: {}", err_msg));
            return;
        }

        let test_file = target_path.join(".perm_test");
        if let Err(e) = fs::write(&test_file, b"test") {
            let err_msg = format!(
                "Directory not writable '{}': Permission denied ({})",
                target_path.display(),
                e
            );
            self.custom_dir_error = Some(err_msg.clone());
            self.log_debug(&format!("[WARNING] Unsuitable directory: {}", err_msg));
            return;
        }
        let _ = fs::remove_file(&test_file);

        let master_dir = if target_path
            .file_name()
            .map(|n| n == "em624")
            .unwrap_or(false)
        {
            target_path.clone()
        } else {
            target_path.join("em624")
        };

        if let Err(e) = fs::create_dir_all(master_dir.join(".config")) {
            let err_msg = format!("Failed to create .config in master directory: {}", e);
            self.custom_dir_error = Some(err_msg.clone());
            self.log_debug(&format!("[WARNING] {}", err_msg));
            return;
        }
        let _ = fs::create_dir_all(master_dir.join(".cache"));
        let _ = fs::create_dir_all(master_dir.join(".logs"));

        self.config.custom_data_dir = Some(trimmed.to_string());
        self.config.save();
        self.custom_dir_error = None;
        self.status_message = format!("Master data directory updated: {}", master_dir.display());
        self.log_debug(&format!(
            "[CONFIG] Successfully relocated master data directory to: {}",
            master_dir.display()
        ));
    }

    fn spawn_native_terminal(&self) {
        let log_path = self.config.logs_dir().join("debug.log");
        crate::platform::spawn_debug_terminal(Some(&log_path));
    }

    fn log_debug(&self, msg: &str) {
        let ts = chrono_or_simple_time();
        let formatted = format!("[{}] {}", ts, msg);
        println!("{}", formatted);
        let logs_dir = self.config.logs_dir();
        let _ = fs::create_dir_all(&logs_dir);
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs_dir.join("debug.log"))
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
        // Enforce safe stages and limit unless experimental Stage 6 is unlocked
        if !self.config.allow_stage_6 {
            self.profile.apply_safe_profile();
        } else {
            self.profile.refresh_stages();
        }

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
                            self.log_debug(&format!(
                                "[USB_READ] Profile read successfully (2048 bytes). Checksum0=0x{:02x}, Mask=0x{:02x}",
                                data[0xa3], data[4]
                            ));
                            self.save_to_cached_profile();
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
                        self.save_to_cached_profile();
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

    fn switch_mode_and_forward(&mut self, new_is_gaming: bool) {
        let mode_byte = if new_is_gaming { 0x01 } else { 0x00 };
        let mode_name = if new_is_gaming {
            "Gaming Mode"
        } else {
            "Office Mode"
        };
        self.profile.raw[0x00a4] = mode_byte;
        self.sync_settings_to_profile();
        self.save_to_cached_profile();

        self.log_debug(&format!(
            "[MODE_FORWARD] Switching mouse active mode to: {} (Byte 0x00A4 = 0x{:02X})",
            mode_name, mode_byte
        ));
        self.log_debug(&format!(
            "[MODE_FORWARD] Sector 1 Checksum (0x00BF) recalculated: 0x{:02X}",
            self.profile.raw[0x00bf]
        ));

        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                self.log_debug(&format!("[HID_ERROR] {}", e));
                return;
            }
        };

        match Em624Device::open_auto(&api) {
            Ok(dev) => {
                self.status_message =
                    format!("Forwarding {} signal to mouse hardware...", mode_name);
                self.log_debug(
                    "[MODE_FORWARD] Transmitting Report 0x40 Opcode 0x07 + 293 chunks + Commit 0x40 0x0E...",
                );
                match dev.write_profile(&self.profile.raw) {
                    Ok(()) => {
                        self.device_connected = true;
                        let _ = dev.apply_soft_reset();
                        self.status_message =
                            format!("Mouse switched to {} successfully!", mode_name);
                        self.log_debug(&format!(
                            "[MODE_FORWARD] Hardware acknowledgment received! Mouse is now operating in {}.",
                            mode_name
                        ));
                    }
                    Err(e) => {
                        self.status_message = format!("Mode switch signal failed: {}", e);
                        self.log_debug(&format!("[MODE_FORWARD_ERROR] {}", e));
                    }
                }
            }
            Err(e) => {
                self.device_connected = false;
                self.status_message = format!("Mouse not connected (Saved mode to profile): {}", e);
                self.log_debug(&format!(
                    "[MODE_OFFLINE] Mouse offline; updated active mode in profile to {}.",
                    mode_name
                ));
            }
        }

        // Switch active tab view to match new mode
        self.active_tab = if new_is_gaming {
            Tab::GameMode
        } else {
            Tab::OfficeMode
        };
    }

    fn load_profile_auto(&mut self, path: PathBuf) {
        let cache_dir = self.config.cache_dir();
        let _ = fs::create_dir_all(&cache_dir);
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
                let cached_path = cache_dir.join(format!("{}.em624", stem));
                if let Ok(()) = prof.save_file(&cached_path) {
                    self.profile = prof;
                    self.file_path = Some(cached_path.clone());
                    self.status_message =
                        format!("Converted and loaded into {}", cached_path.display());
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

    fn save_to_cached_profile(&mut self) {
        let cache_dir = self.config.cache_dir();
        let _ = fs::create_dir_all(&cache_dir);
        let stem = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("DailyUse");
        let cached_path = cache_dir.join(format!("{}.em624", stem));
        if let Ok(()) = self.profile.save_file(&cached_path) {
            self.file_path = Some(cached_path.clone());
            self.config.last_profile = Some(cached_path.to_string_lossy().to_string());
            self.config.save();
        }
    }

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if self.mouse_texture.is_none() {
            let png_bytes = include_bytes!("../damouse.png");
            if let Ok(img) = image::load_from_memory(png_bytes) {
                // Downsample to exact rendered dimensions (320x340) for aggressive RAM optimization
                let resized = img.resize_exact(320, 340, image::imageops::FilterType::Triangle);
                let rgba = resized.to_rgba8();
                let size = [320, 340];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture =
                    ctx.load_texture("mouse_photo", color_image, egui::TextureOptions::LINEAR);
                self.mouse_texture = Some(texture);
            }
        }
    }

    fn init_rebind_modal(&mut self, slot_idx: usize, is_gaming: bool) {
        let base_off = if is_gaming { 0x00f1 } else { 0x00c0 };
        let off = base_off + (slot_idx - 1) * 4;
        let mut cur = [0u8; 4];
        cur.copy_from_slice(&self.profile.raw[off..off + 4]);
        self.rebind_candidate = Some(cur);

        let ktype = cur[0];
        let p1 = cur[1];
        let p2 = cur[2];

        match ktype {
            0x01 => {
                self.rebind_category = 0; // Mouse
            }
            0x02 => {
                if p1 == 0 {
                    self.rebind_category = 1; // Single key
                    self.rebind_key_code = p2;
                    self.rebind_modifiers = 0;
                    self.rebind_subtab = match p2 {
                        0x04..=0x1d => 0, // Letters
                        0x1e..=0x27 => 1, // Numbers
                        0x54..=0x63 => 2, // Numpad
                        0x3a..=0x45 => 3, // Function
                        0x49..=0x53 => 4, // Navigation
                        _ => 5,           // Symbols
                    };
                } else {
                    self.rebind_category = 2; // Shortcut
                    self.rebind_key_code = p2;
                    self.rebind_modifiers = p1;
                }
            }
            0x06 | 0x08 | 0x09 => {
                self.rebind_category = 3; // Special / DPI
            }
            0x03 => {
                match p2 {
                    0x81 | 0x82 | 0x83 | 0x84 | 0x92 | 0x94 | 0x8a => {
                        self.rebind_category = 5; // Productivity
                    }
                    _ => {
                        self.rebind_category = 4; // Multimedia
                    }
                }
            }
            0x07 => {
                self.rebind_category = 5; // Productivity (LED)
            }
            _ => {
                self.rebind_category = 0;
            }
        }
    }

    fn apply_and_cache_binding(&mut self, slot_idx: usize, is_gaming: bool) {
        if let Some(bytes) = self.rebind_candidate {
            let base_off = if is_gaming { 0x00f1 } else { 0x00c0 };
            let off = base_off + (slot_idx - 1) * 4;
            self.profile.raw[off..off + 4].copy_from_slice(&bytes);
            self.profile.apply_safe_profile();
            self.profile.refresh_stages();

            // Auto-save to cached profile
            self.save_to_cached_profile();

            let btn_num = slot_idx_to_button_num(slot_idx);
            let desc = format_binding_desc(&bytes);
            self.status_message = format!(
                "Saved Button [{}] as '{}' to cached profile!",
                btn_num, desc
            );
            self.log_debug(&format!(
                "[REMAP] Updated slot {} ({}) to bytecode {:02x?} ('{}') and cached to disk",
                slot_idx,
                if is_gaming { "Gaming" } else { "Office" },
                bytes,
                desc
            ));
        }
    }
}

impl eframe::App for Em624App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(status) = self.device_rx.try_recv() {
            match status {
                crate::platform::DeviceStatus::Connected { info, vid, pid } => {
                    self.device_connected = true;
                    self.connected_info = info;
                    self.status_message = format!(
                        "E-DRA EM624 connected (VID 0x{:04x}, PID 0x{:04x}).",
                        vid, pid
                    );
                }
                crate::platform::DeviceStatus::Disconnected => {
                    self.device_connected = false;
                    self.connected_info = "Not detected".to_string();
                    self.status_message = "Mouse disconnected.".to_string();
                }
            }
        }

        ctx.set_zoom_factor(self.config.ui_scale);
        let pal = self.palette();

        let visuals = match self.config.theme {
            ThemeMode::PitchBlackOled => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(0, 0, 0);
                v.window_fill = Color32::from_rgb(6, 8, 12);
                v
            }
            ThemeMode::Windows11Dark => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(32, 32, 32);
                v.window_fill = Color32::from_rgb(40, 40, 40);
                v
            }
            ThemeMode::LightWhite => {
                let mut v = egui::Visuals::light();
                v.panel_fill = Color32::from_rgb(243, 244, 246);
                v.window_fill = Color32::from_rgb(255, 255, 255);
                v
            }
        };
        ctx.set_visuals(visuals);

        self.ensure_texture(ctx);

        self.frame_count = self.frame_count.wrapping_add(1);
        let now = std::time::Instant::now();
        // Aggressive RAM optimization:
        // Flush memory during initial frame setup (frames 1..10) and then every 3 seconds
        if self.frame_count <= 10
            || now.duration_since(self.last_trim_instant).as_secs() >= 3
            || !self.trimmed
        {
            trim_working_set();
            self.last_trim_instant = now;
            self.trimmed = true;
        }

        // Top Navigation Header
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("E-DRA EM624")
                        .strong()
                        .color(pal.accent),
                );
                ui.label(
                    egui::RichText::new(Strings::app_subtitle(self.config.language))
                        .small()
                        .color(pal.text_secondary),
                );

                // Distinct unfinished badge next to window title
                match self.active_tab {
                    Tab::Advanced => {
                        ui.label(
                            egui::RichText::new(Strings::unfinished_adv(self.config.language))
                                .small()
                                .strong()
                                .color(Color32::from_rgb(255, 170, 0)),
                        );
                    }
                    Tab::LedLighting => {
                        ui.label(
                            egui::RichText::new(Strings::unfinished_led(self.config.language))
                                .small()
                                .strong()
                                .color(Color32::from_rgb(255, 170, 0)),
                        );
                    }
                    Tab::MacroManager => {
                        ui.label(
                            egui::RichText::new(Strings::unfinished_macro(self.config.language))
                                .small()
                                .strong()
                                .color(Color32::from_rgb(255, 170, 0)),
                        );
                    }
                    _ => {}
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(Strings::check_connection(self.config.language))
                        .clicked()
                    {
                        self.check_connection();
                    }

                    let (theme_label, next_theme) = match self.config.theme {
                        ThemeMode::PitchBlackOled => ("[OLED]", ThemeMode::Windows11Dark),
                        ThemeMode::Windows11Dark => ("[Win11 Dark]", ThemeMode::LightWhite),
                        ThemeMode::LightWhite => ("[Light White]", ThemeMode::PitchBlackOled),
                    };
                    if ui
                        .button(theme_label)
                        .on_hover_text("Click to toggle theme: Pitch Black OLED / Windows 11 Dark / High-Contrast White")
                        .clicked()
                    {
                        self.config.theme = next_theme;
                        self.config.save();
                    }

                    let (lang_label, next_lang) = match self.config.language {
                        Language::English => ("[EN]", Language::Vietnamese),
                        Language::Vietnamese => ("[VI]", Language::English),
                    };
                    if ui
                        .button(lang_label)
                        .on_hover_text("Click to toggle language / Chuyển đổi ngôn ngữ: English / Tiếng Việt")
                        .clicked()
                    {
                        self.config.language = next_lang;
                        self.config.save();
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
                                egui::RichText::new(Strings::connected(self.config.language))
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
                                egui::RichText::new(Strings::disconnected(self.config.language))
                                    .color(Color32::from_rgb(255, 71, 87))
                                    .strong(),
                            );
                        });
                    }
                });
            });
            ui.add_space(6.0);

            // Tab Navigation: Advanced setting is placed immediately to the left of App Settings
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::GameMode,
                    Strings::tab_game(self.config.language),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::OfficeMode,
                    Strings::tab_office(self.config.language),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::LedLighting,
                    Strings::tab_led(self.config.language),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::MacroManager,
                    Strings::tab_macro(self.config.language),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Advanced,
                    Strings::tab_advanced(self.config.language),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Settings,
                    Strings::tab_settings(self.config.language),
                );
            });
            ui.add_space(6.0);
        });

        // Bottom Action Toolbar & Status Bar
        egui::TopBottomPanel::bottom("bottom_toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button(Strings::btn_read(self.config.language))
                    .clicked()
                {
                    self.read_from_mouse();
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(Strings::btn_flash(self.config.language))
                                .strong()
                                .color(if self.config.theme == ThemeMode::LightWhite {
                                    Color32::WHITE
                                } else {
                                    Color32::BLACK
                                }),
                        )
                        .fill(pal.accent),
                    )
                    .clicked()
                {
                    self.flash_to_mouse();
                }

                ui.separator();

                let is_gaming = self.profile.raw[0x00a4] == 0x01;
                let mode_btn_text = if is_gaming {
                    Strings::btn_mode_gaming(self.config.language)
                } else {
                    Strings::btn_mode_office(self.config.language)
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(mode_btn_text)
                                .strong()
                                .color(if is_gaming {
                                    Color32::from_rgb(0, 242, 254)
                                } else {
                                    Color32::from_rgb(46, 213, 115)
                                }),
                        )
                        .stroke(Stroke::new(1.0_f32, pal.card_border)),
                    )
                    .on_hover_text(
                        "Click to toggle between Gaming Mode (0x01) and Office Mode (0x00) and forward the reversed configuration report to mouse hardware immediately.",
                    )
                    .clicked()
                {
                    self.switch_mode_and_forward(!is_gaming);
                }

                ui.separator();

                if ui
                    .button(Strings::btn_load(self.config.language))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Profiles (*.em624, *.bin)", &["em624", "bin"])
                        .pick_file()
                    {
                        self.load_profile_auto(path);
                    }
                }

                if ui
                    .button(Strings::btn_save(self.config.language))
                    .clicked()
                {
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
                            .color(pal.text_secondary),
                    );
                });
            });
            ui.add_space(6.0);
        });

        // Central Content Area (Fully responsive ScrollArea for all tabs)
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.active_tab {
                    Tab::GameMode => self.render_button_mode_tab(ui, true),
                    Tab::OfficeMode => self.render_button_mode_tab(ui, false),
                    Tab::Advanced => self.render_advanced_tab(ui),
                    Tab::LedLighting => self.render_led_tab(ui),
                    Tab::MacroManager => self.render_macro_tab(ui),
                    Tab::Settings => self.render_settings_tab(ui),
                });
        });

        // Modal Button Remapper Dialog if open
        if let Some((slot_idx, is_gaming)) = self.editing_slot {
            self.render_rebind_modal(ctx, slot_idx, is_gaming);
        }
    }
}

impl Em624App {
    fn render_rebind_modal(&mut self, ctx: &egui::Context, slot_idx: usize, is_gaming: bool) {
        let mut close_modal = false;
        let mut apply_binding = false;
        let btn_num = slot_idx_to_button_num(slot_idx);
        let title = format!(
            "Remap Button [{}] ({})",
            btn_num,
            if is_gaming {
                "Game Mode"
            } else {
                "Office Mode"
            }
        );

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(460.0);

                // Step 1: Category Selection
                ui.label(
                    egui::RichText::new("Step 1: Pick a Keybind Category")
                        .strong()
                        .color(Color32::from_rgb(0, 242, 254)),
                );
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.rebind_category, 0, "Mouse");
                    ui.selectable_value(&mut self.rebind_category, 1, "Keyboard (Single)");
                    ui.selectable_value(&mut self.rebind_category, 2, "Shortcut (Combo)");
                    ui.selectable_value(&mut self.rebind_category, 3, "DPI / Special");
                    ui.selectable_value(&mut self.rebind_category, 4, "Media");
                    ui.selectable_value(&mut self.rebind_category, 5, "Productivity");
                    ui.selectable_value(&mut self.rebind_category, 6, "Disable");
                });

                ui.separator();

                // Step 2: Pick Key
                ui.label(
                    egui::RichText::new("Step 2: Pick a Key or Action")
                        .strong()
                        .color(Color32::from_rgb(0, 242, 254)),
                );
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| match self.rebind_category {
                        0 => self.render_category_mouse(ui),
                        1 => self.render_category_single_key(ui),
                        2 => self.render_category_shortcut(ui),
                        3 => self.render_category_special(ui),
                        4 => self.render_category_media(ui),
                        5 => self.render_category_productivity(ui),
                        _ => {
                            self.rebind_candidate = Some([0x00, 0x00, 0x00, 0x00]);
                            ui.label("This button will be disabled (no action).");
                        }
                    });

                ui.separator();

                // Live Preview
                let preview_text = if let Some(bytes) = &self.rebind_candidate {
                    format_binding_desc(bytes)
                } else {
                    "None".to_string()
                };

                ui.horizontal(|ui| {
                    ui.label("New Assignment:");
                    ui.label(
                        egui::RichText::new(preview_text)
                            .strong()
                            .color(Color32::from_rgb(0, 242, 254)),
                    );
                });

                ui.add_space(8.0);

                // Step 3: Done Keybinding
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [160.0, 30.0],
                            egui::Button::new(
                                egui::RichText::new("Done Keybinding")
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .fill(Color32::from_rgb(0, 242, 254)),
                        )
                        .clicked()
                    {
                        apply_binding = true;
                    }

                    if ui
                        .add_sized([100.0, 30.0], egui::Button::new("Cancel"))
                        .clicked()
                    {
                        close_modal = true;
                    }
                });
            });

        if apply_binding {
            self.apply_and_cache_binding(slot_idx, is_gaming);
            self.editing_slot = None;
        } else if close_modal {
            self.editing_slot = None;
        }
    }

    fn render_category_mouse(&mut self, ui: &mut egui::Ui) {
        ui.label("Standard mouse buttons:");
        ui.add_space(4.0);

        let options = [
            ("Left Click", [0x01, 0x01, 0x00, 0x00]),
            ("Right Click", [0x01, 0x02, 0x00, 0x00]),
            ("Middle Click", [0x01, 0x04, 0x00, 0x00]),
            ("Forward (Browser Side 1)", [0x01, 0x08, 0x00, 0x02]),
            ("Backward (Browser Side 2)", [0x01, 0x10, 0x00, 0x00]),
        ];

        for (label, bytes) in options {
            let is_cur = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_cur, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_single_key(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.rebind_subtab, 0, "Letters");
            ui.selectable_value(&mut self.rebind_subtab, 1, "Numbers");
            ui.selectable_value(&mut self.rebind_subtab, 2, "Numpad");
            ui.selectable_value(&mut self.rebind_subtab, 3, "F1..F12");
            ui.selectable_value(&mut self.rebind_subtab, 4, "Navigation");
            ui.selectable_value(&mut self.rebind_subtab, 5, "Symbols");
        });
        ui.separator();

        match self.rebind_subtab {
            0 => {
                // Letters A..Z
                ui.horizontal_wrapped(|ui| {
                    for code in 0x04..=0x1d {
                        let name = format_hid_key_name(code);
                        let is_sel = self.rebind_candidate == Some([0x02, 0x00, code, 0x00]);
                        if ui
                            .add_sized([32.0, 26.0], egui::SelectableLabel::new(is_sel, name))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, 0x00]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            1 => {
                // Numbers 1..9, 0
                ui.horizontal_wrapped(|ui| {
                    for code in 0x1e..=0x27 {
                        let name = format_hid_key_name(code);
                        let is_sel = self.rebind_candidate == Some([0x02, 0x00, code, 0x00]);
                        if ui
                            .add_sized([36.0, 28.0], egui::SelectableLabel::new(is_sel, name))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, 0x00]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            2 => {
                // Full Numpad keys (17 discrete keys)
                ui.label("Full Numpad keys (dedicated scancodes):");
                ui.add_space(4.0);
                let numpad_keys = [
                    (0x59, "Num 1"),
                    (0x5a, "Num 2"),
                    (0x5b, "Num 3"),
                    (0x5c, "Num 4"),
                    (0x5d, "Num 5"),
                    (0x5e, "Num 6"),
                    (0x5f, "Num 7"),
                    (0x60, "Num 8"),
                    (0x61, "Num 9"),
                    (0x62, "Num 0"),
                    (0x57, "Num +"),
                    (0x56, "Num -"),
                    (0x55, "Num *"),
                    (0x54, "Num /"),
                    (0x58, "Num Enter"),
                    (0x63, "Num ."),
                    (0x53, "Num Lock"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in numpad_keys {
                        let is_sel = self.rebind_candidate == Some([0x02, 0x00, code, 0x00]);
                        if ui
                            .add_sized([72.0, 28.0], egui::SelectableLabel::new(is_sel, label))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, 0x00]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            3 => {
                // F1..F12
                ui.horizontal_wrapped(|ui| {
                    for code in 0x3a..=0x45 {
                        let name = format_hid_key_name(code);
                        let is_sel = self.rebind_candidate == Some([0x02, 0x00, code, 0x00]);
                        if ui
                            .add_sized([42.0, 28.0], egui::SelectableLabel::new(is_sel, name))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, 0x00]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            4 => {
                // Navigation & Editing
                let nav_keys = [
                    (0x4b, "Page Up"),
                    (0x4e, "Page Down"),
                    (0x4a, "Home"),
                    (0x4d, "End"),
                    (0x49, "Insert"),
                    (0x4c, "Delete"),
                    (0x52, "Up Arrow"),
                    (0x51, "Down Arrow"),
                    (0x50, "Left Arrow"),
                    (0x4f, "Right Arrow"),
                    (0x28, "Enter"),
                    (0x29, "Esc"),
                    (0x2c, "Space"),
                    (0x2b, "Tab"),
                    (0x2a, "Backspace"),
                    (0x39, "Caps Lock"),
                    (0x46, "Print Screen"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in nav_keys {
                        let is_sel = self.rebind_candidate == Some([0x02, 0x00, code, 0x00]);
                        if ui
                            .add_sized([80.0, 28.0], egui::SelectableLabel::new(is_sel, label))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, 0x00]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            _ => {
                // Symbols
                let sym_keys = [
                    (0x2d, "- Minus"),
                    (0x2e, "= Equals"),
                    (0x2f, "[ Left Bracket"),
                    (0x30, "] Right Bracket"),
                    (0x31, "\\ Backslash"),
                    (0x33, "; Semicolon"),
                    (0x34, "' Quote"),
                    (0x35, "` Grave"),
                    (0x36, ", Comma"),
                    (0x37, ". Period"),
                    (0x38, "/ Slash"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in sym_keys {
                        let is_sel = self.rebind_candidate == Some([0x02, 0x00, code, 0x00]);
                        if ui
                            .add_sized([110.0, 28.0], egui::SelectableLabel::new(is_sel, label))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, 0x00]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
        }
    }

    fn render_category_shortcut(&mut self, ui: &mut egui::Ui) {
        ui.label("Select modifier keys:");
        ui.horizontal(|ui| {
            let mut ctrl = (self.rebind_modifiers & 0x01) != 0;
            if ui.checkbox(&mut ctrl, "Ctrl").changed() {
                if ctrl {
                    self.rebind_modifiers |= 0x01;
                } else {
                    self.rebind_modifiers &= !0x01;
                }
            }

            let mut shift = (self.rebind_modifiers & 0x02) != 0;
            if ui.checkbox(&mut shift, "Shift").changed() {
                if shift {
                    self.rebind_modifiers |= 0x02;
                } else {
                    self.rebind_modifiers &= !0x02;
                }
            }

            let mut alt = (self.rebind_modifiers & 0x04) != 0;
            if ui.checkbox(&mut alt, "Alt").changed() {
                if alt {
                    self.rebind_modifiers |= 0x04;
                } else {
                    self.rebind_modifiers &= !0x04;
                }
            }

            let mut win = (self.rebind_modifiers & 0x08) != 0;
            if ui.checkbox(&mut win, "Win").changed() {
                if win {
                    self.rebind_modifiers |= 0x08;
                } else {
                    self.rebind_modifiers &= !0x08;
                }
            }
        });

        ui.add_space(6.0);
        ui.label("Select target key:");
        egui::ComboBox::from_label("")
            .selected_text(format_hid_key_name(self.rebind_key_code))
            .show_ui(ui, |ui| {
                for code in 0x04..=0x1d {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
                for code in 0x1e..=0x27 {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
                for code in 0x3a..=0x45 {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
                for code in [0x28, 0x29, 0x2c, 0x2b, 0x2a, 0x4b, 0x4e, 0x4a, 0x4d] {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
            });

        self.rebind_candidate = Some([0x02, self.rebind_modifiers, self.rebind_key_code, 0x00]);
    }

    fn render_category_special(&mut self, ui: &mut egui::Ui) {
        let specials = [
            ("DPI Loop (Cycle All)", [0x08, 0x00, 0x00, 0x00]),
            ("DPI +", [0x09, 0x02, 0x00, 0x00]),
            ("DPI -", [0x09, 0x00, 0x00, 0x00]),
            ("Fire Key (Rapid Burst)", [0x06, 0x92, 0x00, 0x1e]),
            ("Office Mode Switch", [0x09, 0x00, 0x00, 0x00]),
            ("Game Mode Switch", [0x09, 0x01, 0x00, 0x00]),
        ];

        for (label, bytes) in specials {
            let is_sel = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_sel, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_media(&mut self, ui: &mut egui::Ui) {
        let media = [
            ("Play / Pause", [0x03, 0x00, 0xcd, 0x00]),
            ("Next Track", [0x03, 0x00, 0xb5, 0x00]),
            ("Previous Track", [0x03, 0x00, 0xb6, 0x00]),
            ("Stop", [0x03, 0x00, 0xb7, 0x00]),
            ("Volume +", [0x03, 0x00, 0xe9, 0x00]),
            ("Volume -", [0x03, 0x00, 0xea, 0x00]),
            ("Mute", [0x03, 0x00, 0xe2, 0x00]),
        ];

        for (label, bytes) in media {
            let is_sel = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_sel, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_productivity(&mut self, ui: &mut egui::Ui) {
        let prod = [
            ("Desktop (Win+D)", [0x02, 0x08, 0x07, 0x00]),
            ("Snip (Shift+Win+S)", [0x02, 0x0a, 0x16, 0x00]),
            ("Calculator", [0x03, 0x00, 0x92, 0x00]),
            ("My Computer", [0x03, 0x00, 0x94, 0x00]),
            ("Web Back", [0x03, 0x00, 0x83, 0x00]),
            ("Web Forward", [0x03, 0x00, 0x84, 0x00]),
            ("LED ON/OFF Toggle", [0x07, 0x80, 0x00, 0x00]),
            ("LED Mode Cycle", [0x07, 0x81, 0x00, 0x00]),
        ];

        for (label, bytes) in prod {
            let is_sel = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_sel, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_button_mode_tab(&mut self, ui: &mut egui::Ui, is_gaming: bool) {
        let _pal = self.palette();
        let lang = self.config.language;
        let avail_w = ui.available_width();
        let avail_h = ui.available_height();

        // Responsive proportional column widths based on available width
        // On widescreen: expands up to 450px left, 460px right
        // On compact/scaled window: fits down to 330px each
        let col1_w = (avail_w * 0.36).clamp(330.0, 460.0);
        let col2_w = (avail_w * 0.38).clamp(330.0, 480.0);
        let total_content_w = col1_w + col2_w + 32.0;

        // Smart horizontal and vertical viewport centering
        let pad_x = ((avail_w - total_content_w) * 0.5).max(0.0);
        let pad_y = ((avail_h - 520.0) * 0.40).clamp(8.0, 80.0);

        // Smart scale factor for the hardware mouse chassis diagram
        let mouse_scale = (col2_w / 380.0).clamp(0.85, 1.30);

        ui.add_space(pad_y);
        ui.horizontal(|ui| {
            if pad_x > 0.0 {
                ui.add_space(pad_x);
            }

            // Left Column: Button List
            ui.vertical(|ui| {
                ui.set_width(col1_w);
                ui.add_space(4.0);
                ui.heading(if is_gaming {
                    Strings::game_mode_title(lang)
                } else {
                    Strings::office_mode_title(lang)
                });
                ui.label(
                    egui::RichText::new(Strings::select_key_hint(lang))
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

                let display_order = match lang {
                    Language::English => [
                        (1, "[1] Left Click"),
                        (4, "[2] Right Click"),
                        (7, "[3] Middle Click"),
                        (2, "[4] Forward (Side 1)"),
                        (5, "[5] Backward (Side 2)"),
                        (9, "[6] DPI + (Top Front)"),
                        (8, "[7] DPI - (Top Back)"),
                    ],
                    Language::Vietnamese => [
                        (1, "[1] Chuột trái"),
                        (4, "[2] Chuột phải"),
                        (7, "[3] Chuột giữa"),
                        (2, "[4] Tiến (Hông 1)"),
                        (5, "[5] Lùi (Hông 2)"),
                        (9, "[6] Tăng DPI (Trước)"),
                        (8, "[7] Giảm DPI (Sau)"),
                    ],
                };

                for (slot_idx, label) in display_order {
                    if let Some(btn) = buttons.get(slot_idx - 1) {
                        let is_selected = self.selected_slot == slot_idx;
                        let is_hovered = self.hovered_button == Some(slot_idx);

                        // Background remains clean dark slate - no blinding cyan washout
                        let bg_color = if is_selected {
                            Color32::from_rgb(22, 28, 42)
                        } else if is_hovered {
                            Color32::from_rgb(26, 34, 52)
                        } else {
                            Color32::from_rgb(18, 24, 36)
                        };

                        let border_stroke = if is_selected {
                            Stroke::new(1.5_f32, Color32::from_rgb(0, 242, 254))
                        } else if is_hovered {
                            Stroke::new(1.0_f32, Color32::from_rgb(70, 90, 120))
                        } else {
                            Stroke::new(1.0_f32, Color32::from_rgb(36, 48, 70))
                        };

                        // Flashing -> indicator towards selected key
                        let flash_on = if is_selected {
                            ui.ctx()
                                .request_repaint_after(std::time::Duration::from_millis(200));
                            (ui.ctx().input(|i| i.time) * 2.5).fract() < 0.5
                        } else {
                            false
                        };

                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(col1_w, 36.0), egui::Sense::click());

                        if response.clicked() {
                            self.selected_slot = slot_idx;
                        }

                        if response.hovered() {
                            self.hovered_button = Some(slot_idx);
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }

                        let painter = ui.painter();
                        painter.rect_filled(rect, 4.0, bg_color);
                        painter.rect_stroke(rect, 4.0, border_stroke);

                        let content_rect = rect.shrink2(Vec2::new(10.0, 0.0));
                        ui.allocate_ui_at_rect(content_rect, |content_ui| {
                            content_ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |content_ui| {
                                    // Flashing arrow or transparent placeholder
                                    let arrow_color = if is_selected && flash_on {
                                        Color32::from_rgb(0, 242, 254)
                                    } else {
                                        Color32::TRANSPARENT
                                    };

                                    content_ui.label(
                                        egui::RichText::new("->").strong().color(arrow_color),
                                    );
                                    content_ui.add_space(4.0);

                                    // Button Name: high contrast white
                                    content_ui.label(egui::RichText::new(label).strong().color(
                                        if is_selected {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_rgb(215, 225, 240)
                                        },
                                    ));

                                    // Right-aligned description: crisp cyan
                                    content_ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |r_ui| {
                                            r_ui.label(
                                                egui::RichText::new(&btn.description)
                                                    .strong()
                                                    .color(Color32::from_rgb(0, 242, 254)),
                                            );
                                        },
                                    );
                                },
                            );
                        });

                        ui.add_space(3.0);
                    }
                }

                ui.add_space(10.0);
                let sel_button_num = slot_idx_to_button_num(self.selected_slot);
                let change_btn_text = Strings::change_key_bind_btn(lang, sel_button_num);
                let change_resp = ui.add_sized(
                    [col1_w, 36.0],
                    egui::Button::new(
                        egui::RichText::new(change_btn_text)
                            .strong()
                            .color(Color32::BLACK),
                    )
                    .fill(Color32::from_rgb(0, 242, 254)),
                );

                if change_resp.clicked() {
                    self.init_rebind_modal(self.selected_slot, is_gaming);
                    self.editing_slot = Some((self.selected_slot, is_gaming));
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Right Column: Centered Hardware Mouse Picture with Smart Scaling
            ui.vertical(|ui| {
                ui.set_width(col2_w);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(Strings::switch_matrix_title(lang))
                        .strong()
                        .color(Color32::from_rgb(140, 160, 190)),
                );
                ui.add_space(4.0);

                self.render_mouse_photo(ui, mouse_scale);

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(Strings::switch_spec(lang))
                        .small()
                        .color(Color32::from_rgb(120, 140, 170)),
                );
            });

            if pad_x > 0.0 {
                ui.add_space(pad_x);
            }
        });
        ui.add_space(pad_y);
    }

    fn render_mouse_photo(&mut self, ui: &mut egui::Ui, scale: f32) {
        let display_size = Vec2::new(320.0 * scale, 340.0 * scale);
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

        // Numbered badges [1] through [7] scaled proportionally
        let badges = [
            (1, Pos2::new(cx - 46.0 * scale, cy - 75.0 * scale), "[1]"), // Button 1: Left Click
            (4, Pos2::new(cx + 46.0 * scale, cy - 75.0 * scale), "[2]"), // Button 2: Right Click
            (7, Pos2::new(cx, cy - 100.0 * scale), "[3]"),               // Button 3: Wheel Click
            (2, Pos2::new(cx - 96.0 * scale, cy - 28.0 * scale), "[4]"), // Button 4: Side Forward
            (5, Pos2::new(cx - 96.0 * scale, cy + 14.0 * scale), "[5]"), // Button 5: Side Backward
            (9, Pos2::new(cx, cy - 30.0 * scale), "[6]"),                // Button 6: DPI +
            (8, Pos2::new(cx, cy - 2.0 * scale), "[7]"),                 // Button 7: DPI -
        ];

        for (slot_idx, pos, text) in badges {
            let is_selected = self.selected_slot == slot_idx;
            let is_hovered = self.hovered_button == Some(slot_idx);
            let badge_bg = if is_selected {
                Color32::from_rgb(0, 242, 254)
            } else if is_hovered {
                Color32::from_rgb(0, 180, 210)
            } else {
                Color32::from_rgb(14, 20, 32)
            };
            let text_color = if is_selected || is_hovered {
                Color32::BLACK
            } else {
                Color32::from_rgb(0, 242, 254)
            };

            let badge_rect = Rect::from_center_size(pos, Vec2::new(28.0 * scale, 20.0 * scale));
            let badge_resp = ui.interact(
                badge_rect,
                ui.id().with(("badge", slot_idx)),
                egui::Sense::click(),
            );
            if badge_resp.clicked() {
                self.selected_slot = slot_idx;
            }
            if badge_resp.hovered() {
                self.hovered_button = Some(slot_idx);
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            }

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
                egui::FontId::monospace(11.0 * scale),
                text_color,
            );
        }
    }

    fn render_advanced_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        ui.add_space(4.0);
        ui.heading("Advanced Settings & Performance (Unfinished)");
        ui.label(
            egui::RichText::new(
                "Configure DPI stages, Fire Key, pointer speeds, and USB polling rate:",
            )
            .small()
            .color(pal.text_secondary),
        );
        ui.add_space(8.0);

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

                                if i == 5 && !self.config.allow_stage_6 {
                                    ui.label("Stage 6 [Locked]").on_hover_text(
                                        "Stage 6 is locked for hardware safety (causes Sunplus MCU freeze).\nYou can unlock it in App Settings.",
                                    );
                                } else {
                                    ui.label(format!("Stage {}", i + 1));
                                }

                                // Stage 6 is protected against MCU lockup unless experimental mode is active
                                if i == 5 {
                                    if !self.config.allow_stage_6 {
                                        let mut disabled_flag = false;
                                        ui.add_enabled(
                                            false,
                                            egui::Checkbox::new(&mut disabled_flag, ""),
                                        )
                                        .on_disabled_hover_text(
                                            "Locked for Hardware Safety:\nEnabling Stage 6 (bit 5 = 1) causes an internal MCU firmware interrupt deadlock on Sunplus SPCP6651B, freezing physical button clicks until USB replug.\n\nTo unlock for testing, enable 'Allow Experimental Stage 6' in App Settings.",
                                        );
                                    } else {
                                        let mut active_checked = is_active;
                                        if ui.checkbox(&mut active_checked, "").changed() {
                                            if active_checked {
                                                self.profile.active_stages_mask |= 1 << i;
                                            } else {
                                                self.profile.active_stages_mask &= !(1 << i);
                                            }
                                        }
                                    }
                                } else {
                                    let mut active_checked = is_active;
                                    if ui.checkbox(&mut active_checked, "").changed() {
                                        if active_checked {
                                            self.profile.active_stages_mask |= 1 << i;
                                        } else {
                                            self.profile.active_stages_mask &= !(1 << i);
                                        }
                                        if !self.config.allow_stage_6 {
                                            self.profile.active_stages_mask &= 0xdf;
                                            // Safety clamp
                                        }
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
        ui.heading("LED Backlight & Lighting Effects (Unfinished)");
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
        ui.heading("Macro Manager (Unfinished)");
        ui.label(
            egui::RichText::new("Record, assign, and store keystroke sequences to mouse EEPROM:")
                .small()
                .color(Color32::from_rgb(140, 160, 190)),
        );
        ui.add_space(10.0);

        // Fully responsive scaling based on available viewport
        let total_w = ui.available_width();
        let total_h = ui.available_height().max(380.0);

        let col1_w = (total_w * 0.28).clamp(180.0, 260.0);
        let col2_w = (total_w * 0.44).max(280.0);
        let col3_w = (total_w - col1_w - col2_w - 28.0).max(180.0);

        ui.horizontal(|ui| {
            // Column 1: Macro Library (Responsive)
            ui.allocate_ui_with_layout(
                Vec2::new(col1_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(egui::RichText::new("Macro Library").strong());
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(total_h - 90.0)
                            .show(ui, |ui| {
                                for (i, m) in self.macro_bank.macros.iter().enumerate() {
                                    let selected = self.selected_macro == Some(i);
                                    if ui.selectable_label(selected, &m.name).clicked() {
                                        self.selected_macro = Some(i);
                                    }
                                }
                            });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("New Macro").clicked() {
                            let new_name = format!("Macro {}", self.macro_bank.macros.len() + 1);
                            self.macro_bank.macros.push(MacroEntry::new(&new_name));
                            self.selected_macro = Some(self.macro_bank.macros.len() - 1);
                        }
                        if ui.button("Delete").clicked() {
                            if let Some(idx) = self.selected_macro {
                                if idx < self.macro_bank.macros.len() {
                                    self.macro_bank.macros.remove(idx);
                                    self.selected_macro = if self.macro_bank.macros.is_empty() {
                                        None
                                    } else {
                                        Some(idx.saturating_sub(1))
                                    };
                                }
                            }
                        }
                    });
                },
            );

            ui.separator();

            // Column 2: Macro Content / Sequence Timeline (Responsive)
            ui.allocate_ui_with_layout(
                Vec2::new(col2_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(egui::RichText::new("Macro Sequence Timeline").strong());
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(total_h - 90.0)
                            .show(ui, |ui| {
                                if let Some(idx) = self.selected_macro {
                                    if let Some(macro_item) = self.macro_bank.macros.get_mut(idx) {
                                        ui.horizontal(|ui| {
                                            ui.label("Name:");
                                            ui.text_edit_singleline(&mut macro_item.name);
                                        });
                                        ui.separator();

                                        if macro_item.actions.is_empty() {
                                            ui.label("No actions in sequence. Click buttons below to add steps.");
                                        } else {
                                            let mut to_remove = None;
                                            for (step_idx, action) in macro_item.actions.iter_mut().enumerate() {
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}.", step_idx + 1));
                                                    match action {
                                                        MacroAction::MouseDown(btn) => {
                                                            ui.label(format!("Mouse Down [{:?}]", btn));
                                                        }
                                                        MacroAction::MouseUp(btn) => {
                                                            ui.label(format!("Mouse Up [{:?}]", btn));
                                                        }
                                                        MacroAction::KeyDown(k) => {
                                                            let kname = format_hid_key_name(*k);
                                                            ui.label(format!("Key Down [{}]", kname));
                                                        }
                                                        MacroAction::KeyUp(k) => {
                                                            let kname = format_hid_key_name(*k);
                                                            ui.label(format!("Key Up [{}]", kname));
                                                        }
                                                        MacroAction::ModifierDown(m) => {
                                                            ui.label(format!("Modifier Down [{:?}]", m));
                                                        }
                                                        MacroAction::ModifierUp(m) => {
                                                            ui.label(format!("Modifier Up [{:?}]", m));
                                                        }
                                                        MacroAction::Delay(ms) => {
                                                            ui.label("Delay (ms):");
                                                            ui.add(egui::DragValue::new(ms).range(5..=2000));
                                                        }
                                                        MacroAction::Loop(c) => {
                                                            ui.label("Loop repeat:");
                                                            ui.add(egui::DragValue::new(c).range(1..=127));
                                                        }
                                                    }
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.small_button("X").clicked() {
                                                            to_remove = Some(step_idx);
                                                        }
                                                    });
                                                });
                                            }
                                            if let Some(rem_idx) = to_remove {
                                                macro_item.actions.remove(rem_idx);
                                            }
                                        }
                                    }
                                } else {
                                    ui.label("Select or create a macro from the library.");
                                }
                            });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if let Some(idx) = self.selected_macro {
                            if let Some(macro_item) = self.macro_bank.macros.get_mut(idx) {
                                if ui.button("+ Add Key").clicked() {
                                    macro_item.actions.push(MacroAction::KeyDown(0x2c)); // Space
                                    macro_item.actions.push(MacroAction::Delay(30));
                                    macro_item.actions.push(MacroAction::KeyUp(0x2c));
                                }
                                if ui.button("+ Add Click").clicked() {
                                    macro_item.actions.push(MacroAction::MouseDown(MouseButton::Left));
                                    macro_item.actions.push(MacroAction::Delay(25));
                                    macro_item.actions.push(MacroAction::MouseUp(MouseButton::Left));
                                }
                                if ui.button("+ Add Delay").clicked() {
                                    macro_item.actions.push(MacroAction::Delay(50));
                                }
                            }
                        }
                    });
                },
            );

            ui.separator();

            // Column 3: Storage & Execution Options (Responsive)
            ui.allocate_ui_with_layout(
                Vec2::new(col3_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(egui::RichText::new("Execution & Storage").strong());
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(total_h - 90.0)
                            .show(ui, |ui| {
                                ui.label("Recording Timing:");
                                ui.radio_value(&mut self.fire_mode, 0, "Real record");
                                ui.radio_value(&mut self.fire_mode, 1, "Fixed Delay (50ms)");
                                ui.radio_value(&mut self.fire_mode, 2, "No Delay");

                                ui.add_space(10.0);
                                ui.label("Playback Loop:");
                                ui.radio_value(&mut self.fire_mode, 3, "Play once");
                                ui.radio_value(&mut self.fire_mode, 4, "Hold until released");
                                ui.radio_value(&mut self.fire_mode, 5, "Repeat N times");

                                ui.add_space(12.0);
                                ui.separator();
                                ui.label("EEPROM Hardware Bank:");
                                if ui.button("Write Macro to Mouse").clicked() {
                                    self.status_message = "Macro written to mouse hardware bank.".to_string();
                                }
                            });
                    });
                },
            );
        });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;
        ui.add_space(6.0);
        ui.heading(Strings::settings_title(lang));
        ui.label(
            egui::RichText::new(Strings::settings_desc(lang))
                .small()
                .color(pal.text_secondary),
        );
        ui.add_space(14.0);

        ui.group(|ui| {
            ui.heading(Strings::lang_section_title(lang));
            ui.label(
                egui::RichText::new(Strings::lang_section_desc(lang)).color(pal.text_secondary),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.config.language == Language::English, "English")
                    .clicked()
                {
                    self.config.language = Language::English;
                    self.config.save();
                }
                if ui
                    .selectable_label(self.config.language == Language::Vietnamese, "Tiếng Việt")
                    .clicked()
                {
                    self.config.language = Language::Vietnamese;
                    self.config.save();
                }
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::theme_section_title(lang));
            ui.label(
                egui::RichText::new(Strings::theme_section_desc(lang)).color(pal.text_secondary),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        self.config.theme == ThemeMode::PitchBlackOled,
                        "Pitch Black (OLED)",
                    )
                    .clicked()
                {
                    self.config.theme = ThemeMode::PitchBlackOled;
                    self.config.save();
                }
                if ui
                    .selectable_label(
                        self.config.theme == ThemeMode::Windows11Dark,
                        "Windows 11 Dark",
                    )
                    .clicked()
                {
                    self.config.theme = ThemeMode::Windows11Dark;
                    self.config.save();
                }
                if ui
                    .selectable_label(
                        self.config.theme == ThemeMode::LightWhite,
                        "High-Contrast White",
                    )
                    .clicked()
                {
                    self.config.theme = ThemeMode::LightWhite;
                    self.config.save();
                }
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::master_dir_title(lang));
            ui.label(egui::RichText::new(Strings::master_dir_desc(lang)).color(pal.text_secondary));
            ui.add_space(6.0);

            let current_master = self.config.get_master_data_dir();
            ui.label(
                egui::RichText::new(format!(
                    "Active Master Folder: {}",
                    current_master.display()
                ))
                .strong()
                .color(pal.accent),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Custom Root Path:");
                ui.add(egui::TextEdit::singleline(&mut self.custom_dir_input).desired_width(320.0));
                if ui.button("Browse...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.custom_dir_input = folder.to_string_lossy().to_string();
                        let clone = self.custom_dir_input.clone();
                        self.change_custom_data_dir(&clone);
                    }
                }
                if ui.button("Apply").clicked() {
                    let clone = self.custom_dir_input.clone();
                    self.change_custom_data_dir(&clone);
                }
                if ui.button("Reset Default").clicked() {
                    self.change_custom_data_dir("");
                }
            });

            if let Some(ref err) = self.custom_dir_error {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("[!] {}", err))
                        .color(Color32::from_rgb(255, 71, 87))
                        .strong(),
                );
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::scale_section_title(lang));
            ui.label(
                egui::RichText::new(Strings::scale_section_desc(lang)).color(pal.text_secondary),
            );
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
            ui.heading(Strings::dev_debug_title(lang));
            ui.label(egui::RichText::new(Strings::dev_debug_desc(lang)).color(pal.text_secondary));
            ui.add_space(8.0);

            if ui
                .add_sized(
                    [280.0, 32.0],
                    egui::Button::new(
                        egui::RichText::new(Strings::launch_terminal_btn(lang))
                            .strong()
                            .color(if self.config.theme == ThemeMode::LightWhite {
                                Color32::WHITE
                            } else {
                                Color32::BLACK
                            }),
                    )
                    .fill(pal.accent),
                )
                .clicked()
            {
                self.spawn_native_terminal();
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::safety_title(lang));
            ui.label(
                egui::RichText::new(Strings::safety_desc(lang))
                    .color(pal.text_secondary),
            );
            ui.add_space(6.0);

            let mut allow_s6 = self.config.allow_stage_6;
            if ui
                .checkbox(&mut allow_s6, Strings::allow_s6_label(lang))
                .on_hover_text(
                    "Warning: In stock firmware, enabling Stage 6 (bit 5 in byte 0x04) triggers an internal MCU interrupt deadlock that suspends physical clicks until USB replug.\nOnly enable if you understand the risks and wish to test sensor registers.",
                )
                .changed()
            {
                self.config.allow_stage_6 = allow_s6;
                self.config.save();
                self.sync_settings_to_profile();
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(if lang == Language::Vietnamese {
                "Trạng thái kết nối phần cứng"
            } else {
                "Hardware Connection Status"
            });
            ui.horizontal(|ui| {
                ui.label(if lang == Language::Vietnamese {
                    "Thiết bị đang kết nối:"
                } else {
                    "Active Device:"
                });
                ui.label(egui::RichText::new(&self.connected_info).strong().color(
                    if self.device_connected {
                        Color32::from_rgb(46, 213, 115)
                    } else {
                        Color32::from_rgb(255, 71, 87)
                    },
                ));
            });

            ui.add_space(6.0);
            if ui.button(Strings::check_connection(lang)).clicked() {
                self.check_connection();
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::about_title(lang));
            ui.add_space(4.0);
            ui.label(Strings::about_desc(lang));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(Strings::author(lang)).strong());
                ui.hyperlink_to("AlexAn75541", "https://github.com/AlexAn75541");
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(Strings::github_repo(lang)).strong());
                ui.hyperlink_to(
                    "https://github.com/AlexAn75541/em624reverse",
                    "https://github.com/AlexAn75541/em624reverse",
                );
            });
            ui.add_space(4.0);
            let (ws_mb, priv_mb) = get_process_memory_mb();
            ui.label(format!(
                "• Live Memory Usage: {:.1} MB Working Set (Private: {:.1} MB) — budget <= 10MB",
                ws_mb, priv_mb
            ));
        });
    }
}

pub fn format_binding_desc(bytes: &[u8; 4]) -> String {
    let key_type = bytes[0];
    let p1 = bytes[1];
    let p2 = bytes[2];
    match key_type {
        0x01 => match p1 {
            0x01 => "Left Click".to_string(),
            0x02 => "Right Click".to_string(),
            0x04 => "Middle Click".to_string(),
            0x08 => "Forward (Side 1)".to_string(),
            0x10 => "Backward (Side 2)".to_string(),
            other => format!("Mouse Button 0x{:02x}", other),
        },
        0x02 => {
            let mod_str = format_modifiers(p1);
            let key_name = format_hid_key_name(p2);
            if key_name == "Unknown Key" {
                format!("Key: {}0x{:02x}", mod_str, p2)
            } else {
                format!("Key: {}{}", mod_str, key_name)
            }
        }
        0x03 => match p2 {
            0xb5 => "Media: Next Track".to_string(),
            0xb6 => "Media: Prev Track".to_string(),
            0xb7 => "Media: Stop".to_string(),
            0xcd => "Media: Play/Pause".to_string(),
            0xe2 => "Media: Mute".to_string(),
            0xe9 => "Media: Vol+".to_string(),
            0xea => "Media: Vol-".to_string(),
            0x81 => "Browser: Home".to_string(),
            0x82 => "Browser: Refresh".to_string(),
            0x83 => "Browser: Back".to_string(),
            0x84 => "Browser: Forward".to_string(),
            0x8a => "App: Mail".to_string(),
            0x92 => "App: Calculator".to_string(),
            0x94 => "App: My Computer".to_string(),
            other => format!("Consumer 0x{:02x}", other),
        },
        0x06 => "Fire Key (Rapid Burst)".to_string(),
        0x07 => match p1 {
            0x80 => "LED ON/OFF Toggle".to_string(),
            0x81 => "LED Mode Cycle".to_string(),
            other => format!("ACPI 0x{:02x}", other),
        },
        0x08 => "DPI Loop (Cycle All)".to_string(),
        0x09 => match p1 {
            0x00 => "DPI - / Office Mode".to_string(),
            0x01 => "Gaming Mode Switch".to_string(),
            0x02 => "DPI +".to_string(),
            other => format!("Fn 0x{:02x}", other),
        },
        0x00 => "Disabled (None)".to_string(),
        other => format!("Type 0x{:02x}", other),
    }
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

pub fn trim_working_set() {
    crate::platform::trim_working_set();
}

pub fn get_process_memory_mb() -> (f32, f32) {
    crate::platform::get_process_memory_mb()
}

pub fn chrono_or_simple_time() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        (now / 3600) % 24,
        (now / 60) % 60,
        now % 60
    )
}
