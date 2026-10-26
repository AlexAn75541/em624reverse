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
#[serde(default)]
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
    #[serde(default)]
    pub show_mode_switcher: bool,
    #[serde(default)]
    pub custom_font_name: Option<String>,
    #[serde(default)]
    pub free_window_resize: bool,
    #[serde(default)]
    pub disable_macro_guide: bool,
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
            show_mode_switcher: false,
            custom_font_name: None,
            free_window_resize: false,
            disable_macro_guide: false,
        }
    }
}

impl AppConfig {
    pub fn data_dir(&self) -> PathBuf {
        self.get_master_data_dir()
    }
    pub fn fonts_dir(&self) -> PathBuf {
        self.get_master_data_dir().join(".fonts")
    }
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
    // Macro timing & loop options (independent of fire_mode)
    macro_timing_mode: usize,
    macro_loop_mode: usize,
    macro_repeat_count: u32,
    // Font settings, catalog & verification
    font_input_name: String,
    font_source_option: crate::font_service::FontSourceOption,
    font_check_state: crate::font_service::FontCheckState,
    font_check_rx:
        Option<std::sync::mpsc::Receiver<Result<crate::font_service::VerifiedFont, String>>>,
    font_catalog_shown: Vec<String>,
    font_variants: Vec<crate::font_service::FontVariant>,
    selected_variant_idx: Option<usize>,
    font_variant_rx:
        Option<std::sync::mpsc::Receiver<Result<Vec<crate::font_service::FontVariant>, String>>>,
    font_variants_loading: bool,
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

        let mut initial_profile = None;
        let mut initial_path = None;
        if let Some(ref last_path_str) = config.last_profile {
            let last_path = PathBuf::from(last_path_str);
            if last_path.exists() {
                if let Ok(mut p) = MouseProfile::load_file(&last_path) {
                    p.refresh_stages();
                    initial_profile = Some(p);
                    initial_path = Some(last_path);
                }
            }
        }

        let profile = initial_profile.unwrap_or_else(|| {
            let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
            MouseProfile::from_bytes(baseline).unwrap()
        });
        let file_path = initial_path;

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
        let custom_font_name = config.custom_font_name.clone();

        let os_settings = crate::platform::get_os_mouse_settings();
        let pointer_speed = match os_settings.pointer_speed {
            1..=2 => 1,
            3..=4 => 2,
            5..=6 => 3,
            7..=8 => 4,
            9 => 5,
            10 => 6,
            11..=12 => 7,
            13..=14 => 8,
            15..=16 => 9,
            17..=18 => 10,
            _ => 11,
        };
        let double_click_speed =
            (11 - ((os_settings.double_click_speed.clamp(200, 900) - 200) * 10 / 700)).clamp(1, 11);
        let rolling_speed = os_settings.rolling_speed.clamp(1, 11);
        let scroll_one_screen = os_settings.scroll_one_screen;
        let pointer_precision = os_settings.enhance_precision;

        let mut app = Self {
            config,
            profile,
            active_tab: Tab::GameMode,
            status_message: "Ready. Connect your mouse or load a profile.".to_string(),
            device_connected: false,
            connected_info: "No device connected".to_string(),
            file_path,
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
            pointer_speed,
            pointer_precision,
            double_click_speed,
            rolling_speed,
            scroll_one_screen,
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
            macro_timing_mode: 0,
            macro_loop_mode: 0,
            macro_repeat_count: 1,
            font_input_name: custom_font_name.unwrap_or_else(|| "Inter".to_string()),
            font_source_option: crate::font_service::FontSourceOption::Auto,
            font_check_state: crate::font_service::FontCheckState::Idle,
            font_check_rx: None,
            font_catalog_shown: {
                let mut list = Vec::new();
                let _ = crate::font_service::get_random_font_batch(10, &mut list);
                list
            },
            font_variants: Vec::new(),
            selected_variant_idx: None,
            font_variant_rx: None,
            font_variants_loading: false,
        };

        app.sync_from_profile();
        app.log_debug("[STARTUP] E-DRA EM624 Configurator GUI session started.");
        app
    }
}

pub struct Icons;
#[allow(dead_code)]
impl Icons {
    pub const GAMEPAD: &'static str = "\u{f11b}"; // Game Mode (nf-fa-gamepad)
    pub const BRIEFCASE: &'static str = "\u{f0f2}"; // Office Mode (nf-fa-briefcase)
    pub const LIGHTBULB: &'static str = "\u{f0eb}"; // LED Lighting (nf-fa-lightbulb_o)
    pub const CODE: &'static str = "\u{f121}"; // Macro Manager (nf-fa-code)
    pub const COGS: &'static str = "\u{f085}"; // Advanced Settings (nf-fa-cogs)
    pub const SETTINGS: &'static str = "\u{f013}"; // App Settings (nf-fa-cog)
    pub const CHECK: &'static str = "\u{f00c}"; // Connected (nf-fa-check)
    pub const TIMES: &'static str = "\u{f00d}"; // Disconnected (nf-fa-times)
    pub const REFRESH: &'static str = "\u{f021}"; // Check Connection (nf-fa-refresh)
    pub const DOWNLOAD: &'static str = "\u{f019}"; // Read from Mouse (nf-fa-download)
    pub const UPLOAD: &'static str = "\u{f0ee}"; // Flash to Mouse (nf-fa-upload)
    pub const FOLDER_OPEN: &'static str = "\u{f115}"; // Load Profile (nf-fa-folder_open)
    pub const SAVE: &'static str = "\u{f0c7}"; // Save Profile (nf-fa-save)
    pub const TERMINAL: &'static str = "\u{f120}"; // Debug Terminal (nf-fa-terminal)
    pub const KEYBOARD: &'static str = "\u{f11c}"; // Keybind Wizard (nf-fa-keyboard_o)
    pub const BOLT: &'static str = "\u{f0e7}"; // Fire Key (nf-fa-bolt)
    pub const SLIDERS: &'static str = "\u{f1de}"; // Sliders / DPI (nf-fa-sliders)
    pub const SEARCH: &'static str = "\u{f002}"; // Search (nf-fa-search)
    pub const FONT: &'static str = "\u{f031}"; // Font (nf-fa-font)
}

/// Creates a widget text with configurable spacing between Nerd Font icon and text
pub fn icon_label_spaced(icon: &str, text: &str, space_px: f32) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        icon,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    job.append(
        text,
        space_px,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    egui::WidgetText::from(job)
}

/// Creates a widget text with exact >= 8px spacing between Nerd Font icon and text
pub fn icon_label(icon: &str, text: &str) -> egui::WidgetText {
    icon_label_spaced(icon, text, 8.0)
}

/// Creates a widget text with configurable spacing and custom text color
pub fn icon_label_colored_spaced(
    icon: &str,
    text: &str,
    color: Color32,
    space_px: f32,
) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        icon,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            color,
            ..Default::default()
        },
    );
    job.append(
        text,
        space_px,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            color,
            ..Default::default()
        },
    );
    egui::WidgetText::from(job)
}

/// Creates a widget text with exact >= 8px spacing and custom text color
pub fn icon_label_colored(icon: &str, text: &str, color: Color32) -> egui::WidgetText {
    icon_label_colored_spaced(icon, text, color, 8.0)
}

pub fn sanitize_font_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn configure_fonts(ctx: &egui::Context, custom_font: Option<(&str, &[u8])>) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Primary Text Font: Dosis-Bold (All text, Latin + Vietnamese Extended)
    fonts.font_data.insert(
        "Dosis-Bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Dosis-Bold.ttf")),
    );

    // 2. Symbols Suite: CascadiaMono Nerd Font (Full Nerd Font icons)
    fonts.font_data.insert(
        "CascadiaMono-NF".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/CaskaydiaMonoNerdFont-Bold.ttf"
        )),
    );

    let mut prop = Vec::new();
    let mut mono = Vec::new();

    if let Some((custom_name, custom_bytes)) = custom_font {
        fonts.font_data.insert(
            custom_name.to_owned(),
            egui::FontData::from_owned(custom_bytes.to_vec()),
        );
        prop.push(custom_name.to_owned());
        mono.push(custom_name.to_owned());
    }

    // Fallbacks
    prop.push("Dosis-Bold".to_owned());
    prop.push("CascadiaMono-NF".to_owned());

    mono.push("CascadiaMono-NF".to_owned());
    mono.push("Dosis-Bold".to_owned());

    fonts.families.insert(egui::FontFamily::Proportional, prop);
    fonts.families.insert(egui::FontFamily::Monospace, mono);

    ctx.set_fonts(fonts);
}

impl Em624App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();

        if let Some(ref font_name) = app.config.custom_font_name {
            let font_path = app
                .config
                .fonts_dir()
                .join(format!("{}.ttf", sanitize_font_filename(font_name)));
            if let Ok(bytes) = fs::read(&font_path) {
                configure_fonts(&cc.egui_ctx, Some((font_name, &bytes)));
            } else {
                configure_fonts(&cc.egui_ctx, None);
            }
        } else {
            configure_fonts(&cc.egui_ctx, None);
        }

        if app.config.free_window_resize {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                    280.0, 200.0,
                )));
        } else {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                    700.0, 500.0,
                )));
        }

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

    fn sync_from_profile(&mut self) {
        self.polling_rate_idx = match self.profile.polling_rate() {
            125 => 0,
            250 => 1,
            500 => 2,
            _ => 3,
        };
        self.led_mode = self.profile.led_mode() as usize;
        self.led_speed = self.profile.led_speed() as u32;
        self.led_brightness = self.profile.led_brightness() as usize;
    }

    fn sync_settings_to_profile(&mut self) {
        // Enforce safe stages and limit unless experimental Stage 6 is unlocked
        if !self.config.allow_stage_6 {
            self.profile.apply_safe_profile();
        } else {
            self.profile.refresh_stages();
        }

        // Active Mode (byte 0x00A4): 0x01 for Gaming Mode, 0x00 for Office Mode
        self.profile.raw[0x00a4] = if self.active_tab == Tab::OfficeMode {
            0x00
        } else {
            0x01
        };
        self.profile.raw[0x00aa] = 0x18; // Feature report enable
        self.profile.raw[0x03f9] &= 0x7f; // Clear button lockout flag

        // Sync Polling Rate (byte 0x0004 bits 5..4 and 0x00A8):
        let rate_hz = match self.polling_rate_idx {
            0 => 125,
            1 => 250,
            2 => 500,
            _ => 1000,
        };
        self.profile.set_polling_rate(rate_hz);
        self.profile.raw[0x00a8] = match self.polling_rate_idx {
            0 => 0x01, // 125 Hz
            1 => 0x02, // 250 Hz
            2 => 0x04, // 500 Hz
            _ => 0x0f, // 1000 Hz
        };

        // LED lighting registers:
        // Byte 0x0018: LED lighting effect mode (0=Breathing, 1=Colorful Light, 2=Rainbow, 3=Flow Light, 4=Waltz, 5=4-Seasons, 6=Off)
        // Byte 0x0007: LED breathing/animation speed (1..5)
        // Byte 0x0008: LED brightness (0=Full, 1=Half)
        self.profile.set_led_mode(self.led_mode as u8);
        self.profile.set_led_speed(self.led_speed as u8);
        self.profile.set_led_brightness(self.led_brightness as u8);

        // Sync Fire Key configuration and compile rapid fire bytecode
        self.profile.set_fire_key(
            self.fire_mode as u8,
            self.fire_repeat as u8,
            self.fire_interval as u8,
            self.fire_continuous,
        );

        // Sync OS mouse dynamics (Pointer Speed, Double Click, Wheel Scroll Lines)
        self.apply_live_os_mouse_settings();

        // Recalculate checksums across all 5 sectors
        self.profile.recalculate_checksums();
    }

    fn apply_live_os_mouse_settings(&mut self) {
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
                        let _ = dev.apply_soft_reset();
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

    /// On-demand emergency hardware sensor orientation and axis calibration repair.
    /// Immediately restores verified hardware register values (0x0006 = 0x7F, 0x0007 = 0x7F),
    /// recalculates all checksums, saves cached profile, and flashes mouse memory.
    fn repair_sensor_orientation_now(&mut self) {
        self.log_debug("[REPAIR] Starting on-demand sensor orientation calibration...");
        self.profile.raw[0x0006] = 0x7f;
        self.profile.raw[0x0007] = 0x7f;
        self.profile.apply_safe_profile();
        self.save_to_cached_profile();

        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                return;
            }
        };

        match Em624Device::open_auto(&api) {
            Ok(dev) => {
                self.status_message =
                    "Calibrating sensor orientation and flashing to mouse...".to_string();
                match dev.write_profile(&self.profile.raw) {
                    Ok(()) => {
                        self.device_connected = true;
                        let _ = dev.apply_soft_reset();
                        self.status_message =
                            "Sensor orientation successfully restored to 180° PCB standard!"
                                .to_string();
                        self.log_debug(
                            "[REPAIR] Success! Sensor orientation locked to 0x7F (normal axes).",
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("Sensor calibration flash failed: {}", e);
                        self.log_debug(&format!("[REPAIR_ERROR] {}", e));
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
                    self.sync_from_profile();
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
            self.sync_from_profile();
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
                // Load native full-resolution texture (600x600) for pristine clarity across all display scales (100%-200%)
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture =
                    ctx.load_texture("mouse_photo", color_image, egui::TextureOptions::LINEAR);
                self.mouse_texture = Some(texture);

                // Immediately drop CPU-side decode buffers and trim working set
                drop(pixels);
                drop(img);
                trim_working_set();
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

            // Also synchronize to complementary mode table (Office / Gaming)
            // so physical hardware buttons ALWAYS trigger the rebound action regardless of
            // internal MCU mode switch state.
            let alt_base_off = if is_gaming { 0x00c0 } else { 0x00f1 };
            let alt_off = alt_base_off + (slot_idx - 1) * 4;
            self.profile.raw[alt_off..alt_off + 4].copy_from_slice(&bytes);

            // Set active mode byte 0x00a4 (0x01 Gaming, 0x00 Office)
            self.profile.raw[0x00a4] = if is_gaming { 0x01 } else { 0x00 };
            self.profile.raw[0x00aa] = 0x18; // Feature report enable
            self.profile.raw[0x03f9] &= 0x7f; // Clear button lockout flag

            self.profile.apply_safe_profile();
            self.profile.refresh_stages();

            // Auto-save to cached profile
            self.save_to_cached_profile();

            let btn_num = slot_idx_to_button_num(slot_idx);
            let desc = format_binding_desc(&bytes);
            self.status_message = format!(
                "Saved Button [{}] as '{}'! Click 'Flash to Mouse' to commit to hardware.",
                btn_num, desc
            );
            self.log_debug(&format!(
                "[REMAP] Updated slot {} ({}) to bytecode {:02x?} ('{}') (synced to both mode tables & cached)",
                slot_idx,
                if is_gaming { "Gaming" } else { "Office" },
                bytes,
                desc
            ));
        }
    }

    pub fn start_font_verification(&mut self) {
        let name = self.font_input_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        self.font_check_state = crate::font_service::FontCheckState::Checking(name.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        self.font_check_rx = Some(rx);

        let opt = self.font_source_option;
        let variant_url = self
            .selected_variant_idx
            .and_then(|idx| self.font_variants.get(idx))
            .map(|v| (v.label.clone(), v.download_url.clone()));

        std::thread::spawn(move || {
            let res = if let Some((var_label, url)) = variant_url {
                crate::font_service::fetch_direct_url(&url).map(|(family, _, bytes)| {
                    let size_kb = bytes.len() / 1024;
                    crate::font_service::VerifiedFont {
                        display_name: format!("{name} {var_label}"),
                        family_name: family,
                        source_name: "Direct Variant Download",
                        download_url: url,
                        font_bytes: bytes,
                        file_size_kb: size_kb,
                    }
                })
            } else {
                crate::font_service::verify_and_fetch_font(&name, opt)
            };
            let _ = tx.send(res);
        });
    }

    pub fn start_fetch_font_variants(&mut self) {
        let name = self.font_input_name.trim().to_string();
        if name.is_empty() {
            self.font_variants.clear();
            self.selected_variant_idx = None;
            self.font_variants_loading = false;
            return;
        }
        self.font_variants.clear();
        self.selected_variant_idx = None;
        self.font_variants_loading = true;
        let opt = self.font_source_option;
        let rx = crate::font_service::start_fetch_font_variants(name, opt);
        self.font_variant_rx = Some(rx);
    }

    pub fn poll_font_verification(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.font_check_rx {
            if let Ok(res) = rx.try_recv() {
                match res {
                    Ok(verified) => {
                        self.font_check_state =
                            crate::font_service::FontCheckState::Available(verified);
                    }
                    Err(err) => {
                        self.font_check_state = crate::font_service::FontCheckState::NotFound(err);
                    }
                }
                self.font_check_rx = None;
                ctx.request_repaint();
            }
        }

        if let Some(ref rx) = self.font_variant_rx {
            if let Ok(res) = rx.try_recv() {
                self.font_variants_loading = false;
                match res {
                    Ok(vars) => {
                        self.font_variants = vars;
                        // Select Regular 400 by default if available
                        self.selected_variant_idx = self
                            .font_variants
                            .iter()
                            .position(|v| v.weight == 400 && !v.italic)
                            .or(if self.font_variants.is_empty() {
                                None
                            } else {
                                Some(0)
                            });
                    }
                    Err(_) => {
                        self.font_variants.clear();
                        self.selected_variant_idx = None;
                    }
                }
                self.font_variant_rx = None;
                ctx.request_repaint();
            }
        }
    }

    pub fn apply_verified_font(
        &mut self,
        ctx: &egui::Context,
        verified: &crate::font_service::VerifiedFont,
    ) {
        let fonts_dir = self.config.fonts_dir();
        let _ = fs::create_dir_all(&fonts_dir);
        let filename = format!("{}.ttf", sanitize_font_filename(&verified.display_name));
        let path = fonts_dir.join(filename);
        if let Err(e) = fs::write(&path, &verified.font_bytes) {
            self.font_check_state =
                crate::font_service::FontCheckState::Error(format!("Failed to save font: {}", e));
            return;
        }

        self.config.custom_font_name = Some(verified.display_name.clone());
        self.config.save();

        configure_fonts(ctx, Some((&verified.display_name, &verified.font_bytes)));
        ctx.request_repaint();
        self.status_message = format!("Activated font '{}' globally.", verified.display_name);
        trim_working_set();
    }

    pub fn reset_to_default_font(&mut self, ctx: &egui::Context) {
        self.config.custom_font_name = None;
        self.config.save();
        configure_fonts(ctx, None);
        ctx.request_repaint();
        self.font_check_state = crate::font_service::FontCheckState::Idle;
        self.status_message = "Reset UI font to default (Dosis-Bold).".to_string();
        trim_working_set();
    }

    pub fn toggle_free_window_resize(&mut self, ctx: &egui::Context, free: bool) {
        self.config.free_window_resize = free;
        self.config.save();
        if free {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                280.0, 200.0,
            )));
            self.status_message = "Free window resize unlocked: constraints removed.".to_string();
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                700.0, 500.0,
            )));
            self.status_message =
                "Standard window boundaries and ratio constraints restored.".to_string();
        }
    }

    pub fn set_window_size_preset(&mut self, ctx: &egui::Context, width: f32, height: f32) {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
        self.status_message = format!("Resized window to {}x{}.", width as u32, height as u32);
    }
}

impl eframe::App for Em624App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_font_verification(ctx);
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
                        .button(icon_label(Icons::REFRESH, Strings::check_connection(self.config.language)))
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
                            ui.label(icon_label_colored(
                                Icons::CHECK,
                                Strings::connected(self.config.language),
                                Color32::from_rgb(46, 213, 115),
                            ));
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
                            ui.label(icon_label_colored(
                                Icons::TIMES,
                                Strings::disconnected(self.config.language),
                                Color32::from_rgb(255, 71, 87),
                            ));
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
                    icon_label_spaced(Icons::GAMEPAD, Strings::tab_game(self.config.language), 12.0),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::OfficeMode,
                    icon_label(Icons::BRIEFCASE, Strings::tab_office(self.config.language)),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::LedLighting,
                    icon_label_spaced(Icons::LIGHTBULB, Strings::tab_led(self.config.language), 5.0),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::MacroManager,
                    icon_label_spaced(Icons::CODE, Strings::tab_macro(self.config.language), 12.0),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Advanced,
                    icon_label_spaced(Icons::COGS, Strings::tab_advanced(self.config.language), 12.0),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Settings,
                    icon_label(Icons::SETTINGS, Strings::tab_settings(self.config.language)),
                );
            });
            ui.add_space(6.0);
        });

        // Bottom Action Toolbar & Status Bar
        egui::TopBottomPanel::bottom("bottom_toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            let available_w = ui.available_width();
            let is_compact = available_w < 950.0;

            // When window is half-scaled or narrow (<950px), move status message one section up from the action buttons
            if is_compact {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(&self.status_message)
                                .color(pal.text_secondary),
                        );
                    });
                });
                ui.add_space(2.0);
            }

            ui.horizontal(|ui| {
                if ui
                    .button(icon_label(Icons::DOWNLOAD, Strings::btn_read(self.config.language)))
                    .clicked()
                {
                    self.read_from_mouse();
                }

                let flash_btn_color = if self.config.theme == ThemeMode::LightWhite {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                };
                if ui
                    .add(
                        egui::Button::new(icon_label_colored_spaced(
                            Icons::UPLOAD,
                            Strings::btn_flash(self.config.language),
                            flash_btn_color,
                            12.0, // +4px separation
                        ))
                        .fill(pal.accent),
                    )
                    .clicked()
                {
                    self.flash_to_mouse();
                }

                if self.config.show_mode_switcher {
                    ui.separator();

                    let is_gaming = self.profile.raw[0x00a4] == 0x01;
                    let mode_btn_text = if is_gaming {
                        format!("{} [EXP]", Strings::btn_mode_gaming(self.config.language))
                    } else {
                        format!("{} [EXP]", Strings::btn_mode_office(self.config.language))
                    };
                    let mode_icon = if is_gaming {
                        Icons::GAMEPAD
                    } else {
                        Icons::BRIEFCASE
                    };
                    let mode_color = if is_gaming {
                        Color32::from_rgb(0, 242, 254)
                    } else {
                        Color32::from_rgb(46, 213, 115)
                    };
                    if ui
                        .add(
                            egui::Button::new(icon_label_colored(mode_icon, &mode_btn_text, mode_color))
                                .stroke(Stroke::new(1.0_f32, pal.card_border)),
                        )
                        .on_hover_text(
                            "Experimental Mode Switcher: Click to toggle between Gaming Mode (0x01) and Office Mode (0x00) and forward the reversed configuration report to mouse hardware immediately.",
                        )
                        .clicked()
                    {
                        self.switch_mode_and_forward(!is_gaming);
                    }
                }

                ui.separator();

                if ui
                    .button(icon_label(Icons::FOLDER_OPEN, Strings::btn_load(self.config.language)))
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
                    .button(icon_label(Icons::SAVE, Strings::btn_save(self.config.language)))
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

                if !is_compact {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(&self.status_message)
                                .color(pal.text_secondary),
                        );
                    });
                }
            });
            ui.add_space(4.0);
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
        let mode_label = if is_gaming {
            Strings::game_mode_title(self.config.language)
        } else {
            Strings::office_mode_title(self.config.language)
        };
        let title = Strings::rebind_title(self.config.language, btn_num, mode_label);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(460.0);

                // Step 1: Category Selection
                ui.label(
                    egui::RichText::new(Strings::step1_cat(self.config.language))
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
                    ui.selectable_value(&mut self.rebind_category, 7, "Macro [EXPERIMENTAL]");
                    ui.selectable_value(&mut self.rebind_category, 6, "Disable");
                });

                ui.separator();

                // Step 2: Pick Key
                ui.label(
                    egui::RichText::new(Strings::step2_key(self.config.language))
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
                        7 => self.render_category_macro(ui),
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
                                egui::RichText::new(Strings::done_btn(self.config.language))
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
                        .add_sized(
                            [100.0, 30.0],
                            egui::Button::new(Strings::cancel_btn(self.config.language)),
                        )
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

    fn render_category_macro(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(
                "Select a recorded hardware macro to bind to this physical button [EXPERIMENTAL]:",
            )
            .small(),
        );
        ui.add_space(6.0);
        if self.macro_bank.macros.is_empty() {
            ui.label(
                egui::RichText::new(
                    "No macros recorded in Macro Manager yet. Go to Macro tab to create one.",
                )
                .color(Color32::from_rgb(255, 180, 0)),
            );
        } else {
            for (idx, macro_item) in self.macro_bank.macros.iter().enumerate() {
                let is_selected = match self.rebind_candidate {
                    Some([0x05, m_idx, _, _]) => m_idx as usize == idx,
                    _ => false,
                };
                let label = format!(
                    "Macro #{}: {} ({} steps)",
                    idx + 1,
                    macro_item.name,
                    macro_item.actions.len()
                );
                if ui.selectable_label(is_selected, label).clicked() {
                    self.rebind_candidate = Some([0x05, idx as u8, 0x00, 0x00]);
                }
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

                let mut open_rebind_slot = None;

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
                        if response.double_clicked() {
                            self.selected_slot = slot_idx;
                            open_rebind_slot = Some(slot_idx);
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

                if let Some(slot) = open_rebind_slot {
                    self.init_rebind_modal(slot, is_gaming);
                    self.editing_slot = Some((slot, is_gaming));
                }

                ui.add_space(10.0);
                let sel_button_num = slot_idx_to_button_num(self.selected_slot);
                let change_btn_text = Strings::change_key_bind_btn(lang, sel_button_num);
                let change_resp = ui.add_sized(
                    [col1_w, 36.0],
                    egui::Button::new(icon_label_colored(
                        Icons::KEYBOARD,
                        &change_btn_text,
                        Color32::BLACK,
                    ))
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
                self.render_mouse_photo(ui, mouse_scale, is_gaming);
            });

            if pad_x > 0.0 {
                ui.add_space(pad_x);
            }
        });
        ui.add_space(pad_y);
    }

    fn render_mouse_photo(&mut self, ui: &mut egui::Ui, scale: f32, is_gaming: bool) {
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
            if badge_resp.double_clicked() {
                self.selected_slot = slot_idx;
                self.init_rebind_modal(slot_idx, is_gaming);
                self.editing_slot = Some((slot_idx, is_gaming));
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
                    let mut fire_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Repeat Count:");
                        if ui
                            .add(egui::Slider::new(&mut self.fire_repeat, 1..=20))
                            .changed()
                        {
                            fire_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Interval (ms):");
                        if ui
                            .add(egui::Slider::new(&mut self.fire_interval, 10..=100))
                            .changed()
                        {
                            fire_changed = true;
                        }
                    });
                    if ui.checkbox(&mut self.fire_continuous, "Continuous Fire").changed() {
                        fire_changed = true;
                    }
                    if fire_changed {
                        self.profile.set_fire_key(
                            self.fire_mode as u8,
                            self.fire_repeat as u8,
                            self.fire_interval as u8,
                            self.fire_continuous,
                        );
                        self.save_to_cached_profile();
                    }
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("Pointer & Wheel Speeds");
                    let mut os_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Moving Speed (1..11):");
                        if ui
                            .add(egui::Slider::new(&mut self.pointer_speed, 1..=11))
                            .changed()
                        {
                            os_changed = true;
                        }
                    });
                    if ui
                        .checkbox(&mut self.pointer_precision, "Enhance pointer precision")
                        .changed()
                    {
                        os_changed = true;
                    }

                    ui.horizontal(|ui| {
                        ui.label("Double Click Speed:");
                        if ui
                            .add(egui::Slider::new(&mut self.double_click_speed, 1..=11))
                            .changed()
                        {
                            os_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Rolling Speed:");
                        if ui
                            .add(egui::Slider::new(&mut self.rolling_speed, 1..=11))
                            .changed()
                        {
                            os_changed = true;
                        }
                    });
                    if ui
                        .checkbox(&mut self.scroll_one_screen, "Scroll one screen at a time")
                        .changed()
                    {
                        os_changed = true;
                    }

                    if os_changed {
                        self.apply_live_os_mouse_settings();
                    }
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

                    let mut rate_changed = false;
                    ui.horizontal(|ui| {
                        if ui.radio_value(&mut self.polling_rate_idx, 0, "125 Hz (8 ms)").changed() {
                            rate_changed = true;
                        }
                        if ui.radio_value(&mut self.polling_rate_idx, 1, "250 Hz (4 ms)").changed() {
                            rate_changed = true;
                        }
                        if ui.radio_value(&mut self.polling_rate_idx, 2, "500 Hz (2 ms)").changed() {
                            rate_changed = true;
                        }
                        if ui.radio_value(&mut self.polling_rate_idx, 3, "1000 Hz (1 ms)").changed() {
                            rate_changed = true;
                        }
                    });
                    if rate_changed {
                        let hz = match self.polling_rate_idx {
                            0 => 125,
                            1 => 250,
                            2 => 500,
                            _ => 1000,
                        };
                        self.profile.set_polling_rate(hz);
                        self.save_to_cached_profile();
                        self.status_message = format!(
                            "Polling rate set to {} Hz. Click 'Flash to Mouse' to commit to hardware.",
                            hz
                        );
                    }
                });
            });
        });
    }

    fn render_led_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        ui.add_space(6.0);
        ui.heading("LED Backlight & Lighting Effects (Unfinished)");
        ui.label(
            egui::RichText::new("Configure RGB breathing, speed, mode, and brightness:")
                .small()
                .color(pal.text_secondary),
        );
        ui.add_space(14.0);

        let mut led_changed = false;

        ui.group(|ui| {
            ui.heading("LED Breathing Speed");
            ui.horizontal(|ui| {
                ui.label("Slow (1)");
                if ui
                    .add(egui::Slider::new(&mut self.led_speed, 1..=5).show_value(true))
                    .changed()
                {
                    led_changed = true;
                }
                ui.label("Fast (5)");
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("LED Mode");
            ui.horizontal(|ui| {
                if ui.radio_value(&mut self.led_mode, 0, "Breathing").changed() {
                    led_changed = true;
                }
                if ui
                    .radio_value(&mut self.led_mode, 1, "Colorful Light")
                    .changed()
                {
                    led_changed = true;
                }
                if ui.radio_value(&mut self.led_mode, 2, "Rainbow").changed() {
                    led_changed = true;
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .radio_value(&mut self.led_mode, 3, "Flow Light")
                    .changed()
                {
                    led_changed = true;
                }
                if ui.radio_value(&mut self.led_mode, 4, "Waltz").changed() {
                    led_changed = true;
                }
                if ui.radio_value(&mut self.led_mode, 5, "4-Seasons").changed() {
                    led_changed = true;
                }
                if ui.radio_value(&mut self.led_mode, 6, "Off").changed() {
                    led_changed = true;
                }
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("LED Brightness");
            ui.horizontal(|ui| {
                if ui
                    .radio_value(&mut self.led_brightness, 0, "Full Brightness (100%)")
                    .changed()
                {
                    led_changed = true;
                }
                if ui
                    .radio_value(&mut self.led_brightness, 1, "Half Brightness (50%)")
                    .changed()
                {
                    led_changed = true;
                }
            });
        });

        if led_changed {
            self.profile.set_led_mode(self.led_mode as u8);
            self.profile.set_led_speed(self.led_speed as u8);
            self.profile.set_led_brightness(self.led_brightness as u8);
            self.save_to_cached_profile();
            self.status_message = format!(
                "LED Settings updated: Mode {}, Speed {}, Brightness {}. Click 'Flash to Mouse' to commit to hardware.",
                self.led_mode,
                self.led_speed,
                if self.led_brightness == 0 {
                    "Full"
                } else {
                    "Half"
                }
            );
        }
    }

    fn render_macro_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.heading("Macro Manager (Unfinished)");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut disable = self.config.disable_macro_guide;
                if ui
                    .checkbox(&mut disable, Strings::macro_guide_toggle(lang))
                    .on_hover_text(
                        "Toggle off or on the step-by-step guidance panel and helper hints in Macro Manager.",
                    )
                    .changed()
                {
                    self.config.disable_macro_guide = disable;
                    self.config.save();
                }
            });
        });

        if !self.config.disable_macro_guide {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(
                    "Record, assign, and store keystroke sequences to mouse EEPROM:",
                )
                .small()
                .color(pal.text_secondary),
            );
            ui.add_space(6.0);

            // Guiding UI Widget Card
            egui::Frame::none()
                .fill(pal.card_bg)
                .stroke(Stroke::new(1.0_f32, pal.card_border))
                .rounding(6.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("[i]").strong().color(pal.accent));
                        ui.label(
                            egui::RichText::new(Strings::macro_guide_title(lang))
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(Strings::macro_guide_dismiss_btn(lang))
                                .clicked()
                            {
                                self.config.disable_macro_guide = true;
                                self.config.save();
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(Strings::macro_guide_step1(lang))
                                .small()
                                .color(pal.text_secondary),
                        );
                        ui.label(egui::RichText::new("->").small().color(pal.accent));
                        ui.label(
                            egui::RichText::new(Strings::macro_guide_step2(lang))
                                .small()
                                .color(pal.text_secondary),
                        );
                        ui.label(egui::RichText::new("->").small().color(pal.accent));
                        ui.label(
                            egui::RichText::new(Strings::macro_guide_step3(lang))
                                .small()
                                .color(pal.text_secondary),
                        );
                        ui.label(egui::RichText::new("->").small().color(pal.accent));
                        ui.label(
                            egui::RichText::new(Strings::macro_guide_step4(lang))
                                .small()
                                .color(pal.text_secondary),
                        );
                    });
                });
            ui.add_space(6.0);
        } else {
            ui.add_space(6.0);
        }

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
                                            if !self.config.disable_macro_guide {
                                                ui.label(
                                                    "No actions in sequence. Click buttons below to add steps.",
                                                );
                                            } else {
                                                ui.label("(Empty sequence)");
                                            }
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
                                    if !self.config.disable_macro_guide {
                                        ui.label("Select or create a macro from the library.");
                                    } else {
                                        ui.label("(No macro selected)");
                                    }
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
                                ui.radio_value(&mut self.macro_timing_mode, 0, "Real record");
                                ui.radio_value(&mut self.macro_timing_mode, 1, "Fixed Delay (50ms)");
                                ui.radio_value(&mut self.macro_timing_mode, 2, "No Delay");

                                ui.add_space(10.0);
                                ui.label("Playback Loop:");
                                ui.radio_value(&mut self.macro_loop_mode, 0, "Play once");
                                ui.radio_value(&mut self.macro_loop_mode, 1, "Hold until released");
                                ui.radio_value(&mut self.macro_loop_mode, 2, "Repeat N times");
                                if self.macro_loop_mode == 2 {
                                    ui.horizontal(|ui| {
                                        ui.label("Times:");
                                        ui.add(egui::DragValue::new(&mut self.macro_repeat_count).range(1..=100));
                                    });
                                }

                                ui.add_space(12.0);
                                ui.separator();
                                ui.label("EEPROM Hardware Bank:");
                                if ui.button("Write Macro to Profile").clicked() {
                                    if let Some(idx) = self.selected_macro {
                                        if let Some(macro_item) = self.macro_bank.macros.get(idx) {
                                            let res = macro_item
                                                .compile_bytecode()
                                                .map(|code| (macro_item.name.clone(), macro_item.actions.len(), code));
                                            match res {
                                                Ok((name, count, bytecode)) => {
                                                    self.profile.write_macro_bytecode(0, &bytecode);
                                                    self.save_to_cached_profile();
                                                    self.status_message = format!(
                                                        "Macro '{}' compiled ({} actions) into Profile Slot 0 (0x01F0 / 0x0400). Click 'Apply to Mouse' to flash hardware.",
                                                        name, count
                                                    );
                                                    self.log_debug(&format!(
                                                        "[MACRO] Compiled '{}' bytecode (256B) into profile slots 0x01F0 and 0x0400",
                                                        name
                                                    ));
                                                }
                                                Err(e) => {
                                                    self.status_message = format!("Macro compilation error: {}", e);
                                                    self.log_debug(&format!("[MACRO_ERR] {}", e));
                                                }
                                            }
                                        }
                                    } else {
                                        self.status_message = "Please select a macro from the library first.".to_string();
                                    }
                                }

                                ui.add_space(6.0);
                                if ui.button("Save Macro Bank to Disk").clicked() {
                                    let macro_path = self.config.cache_dir().join("macros.bin");
                                    if let Err(e) = self.macro_bank.save_file(&macro_path) {
                                        self.status_message = format!("Failed to save macro bank: {}", e);
                                    } else {
                                        self.status_message = format!("Macro bank saved to disk ({} macros).", self.macro_bank.macros.len());
                                    }
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

        // Active Development & Hardware Safety Warning Callout
        egui::Frame::none()
            .fill(Color32::from_rgb(32, 22, 8))
            .stroke(Stroke::new(1.5_f32, Color32::from_rgb(255, 165, 0)))
            .rounding(6.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("[!] ")
                            .strong()
                            .size(16.0)
                            .color(Color32::from_rgb(255, 165, 0)),
                    );
                    ui.label(
                        egui::RichText::new(Strings::active_dev_warning_title(lang))
                            .strong()
                            .color(Color32::from_rgb(255, 185, 40)),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(Strings::active_dev_warning_body(lang))
                        .color(Color32::from_rgb(240, 220, 190)),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(Strings::active_dev_mitigation_title(lang))
                        .strong()
                        .color(Color32::from_rgb(255, 200, 80)),
                );
                ui.label(
                    egui::RichText::new(Strings::active_dev_mitigation_axes(lang))
                        .small()
                        .color(Color32::from_rgb(210, 200, 180)),
                );
                ui.label(
                    egui::RichText::new(Strings::active_dev_mitigation_buttons(lang))
                        .small()
                        .color(Color32::from_rgb(210, 200, 180)),
                );
                ui.label(
                    egui::RichText::new(Strings::active_dev_mitigation_s6(lang))
                        .small()
                        .color(Color32::from_rgb(210, 200, 180)),
                );
            });
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
            ui.heading(format!("{} Quick Access Folders", Icons::FOLDER_OPEN));
            ui.label(
                egui::RichText::new(
                    "Open application folders directly in your system file explorer:",
                )
                .small()
                .color(pal.text_secondary),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui
                    .button(icon_label_spaced(
                        Icons::FOLDER_OPEN,
                        "Open App Folder",
                        6.0,
                    ))
                    .on_hover_text("Open folder containing em624-reversed executable")
                    .clicked()
                {
                    let app_dir = std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| PathBuf::from("."));
                    let _ = crate::platform::open_folder_in_file_manager(&app_dir);
                }

                if ui
                    .button(icon_label_spaced(
                        Icons::FOLDER_OPEN,
                        "Open Data Folder",
                        6.0,
                    ))
                    .on_hover_text(
                        "Open master data directory containing custom profiles and macros",
                    )
                    .clicked()
                {
                    let data_dir = self.config.data_dir();
                    let _ = fs::create_dir_all(&data_dir);
                    let _ = crate::platform::open_folder_in_file_manager(&data_dir);
                }

                if ui
                    .button(icon_label_spaced(
                        Icons::FOLDER_OPEN,
                        "Open Cache Folder",
                        6.0,
                    ))
                    .on_hover_text("Open temporary active profile and downloaded font cache")
                    .clicked()
                {
                    let cache_dir = self.config.cache_dir();
                    let _ = fs::create_dir_all(&cache_dir);
                    let _ = crate::platform::open_folder_in_file_manager(&cache_dir);
                }
            });
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
            ui.heading(Strings::window_resize_title(lang));
            ui.label(
                egui::RichText::new(Strings::window_resize_desc(lang)).color(pal.text_secondary),
            );
            ui.add_space(8.0);

            let mut free_res = self.config.free_window_resize;
            if ui
                .checkbox(&mut free_res, Strings::window_resize_toggle(lang))
                .on_hover_text(
                    "When checked, unlocks window constraints and removes minimum boundary clamping, allowing free arbitrary resizing without aspect-ratio locks.",
                )
                .changed()
            {
                self.toggle_free_window_resize(ui.ctx(), free_res);
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Quick Sizing Presets:")
                        .small()
                        .color(pal.text_secondary),
                );
                if ui.button(Strings::window_size_default_btn(lang)).clicked() {
                    self.set_window_size_preset(ui.ctx(), 880.0, 640.0);
                }
                if ui.button(Strings::window_size_compact_btn(lang)).clicked() {
                    self.set_window_size_preset(ui.ctx(), 720.0, 520.0);
                }
                if ui.button(Strings::window_size_wide_btn(lang)).clicked() {
                    self.set_window_size_preset(ui.ctx(), 1120.0, 740.0);
                }
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::font_section_title(lang));
            ui.label(
                egui::RichText::new(Strings::font_section_desc(lang)).color(pal.text_secondary),
            );
            ui.add_space(8.0);

            let active_font = self
                .config
                .custom_font_name
                .as_deref()
                .unwrap_or("Dosis-Bold (Default)");
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(Strings::current_font_label(lang)).strong());
                ui.label(
                    egui::RichText::new(active_font)
                        .strong()
                        .color(pal.accent),
                );
            });
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(Strings::font_input_label(lang));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.font_input_name)
                        .desired_width(200.0)
                        .hint_text("e.g. Inter, Roboto, Montserrat..."),
                );
                if resp.changed() {
                    self.font_check_state = crate::font_service::FontCheckState::Idle;
                }

                ui.label(Strings::font_source_label(lang));
                egui::ComboBox::from_id_source("cloud_font_source_selector")
                    .selected_text(self.font_source_option.display_name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.font_source_option,
                            crate::font_service::FontSourceOption::Auto,
                            "Auto (Google Fonts + Fontshare)",
                        );
                        ui.selectable_value(
                            &mut self.font_source_option,
                            crate::font_service::FontSourceOption::GoogleFonts,
                            "Google Fonts (1500+ fonts)",
                        );
                        ui.selectable_value(
                            &mut self.font_source_option,
                            crate::font_service::FontSourceOption::Fontshare,
                            "Fontshare (Free Quality Fonts)",
                        );
                    });
            });

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Random Cloud Fonts (Google Fonts & Fontshare):")
                    .small()
                    .color(pal.text_secondary),
            );
            ui.horizontal_wrapped(|ui| {
                let mut clicked_font = None;
                for name in &self.font_catalog_shown {
                    let is_active = self.font_input_name.eq_ignore_ascii_case(name);
                    if ui.selectable_label(is_active, name).clicked() {
                        clicked_font = Some(name.clone());
                    }
                }
                if let Some(font) = clicked_font {
                    self.font_input_name = font;
                    self.start_fetch_font_variants();
                    self.start_font_verification();
                }

                if ui
                    .button(egui::RichText::new("+ See More").strong().color(pal.accent))
                    .on_hover_text("Fetch another 10 random fonts from the catalog")
                    .clicked()
                {
                    crate::font_service::get_random_font_batch(10, &mut self.font_catalog_shown);
                }
            });

            ui.add_space(10.0);
            // Variant list downward to that box, only fetch available variants of the selected font, otherwise greyed out
            let font_selected = !self.font_input_name.trim().is_empty();
            ui.group(|ui| {
                ui.add_enabled_ui(font_selected, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Font Variant / Weight [EXPERIMENTAL]:").strong(),
                        );
                        if !font_selected {
                            ui.label(
                                egui::RichText::new(
                                    "(No font selected. Select a font above to view available variants)",
                                )
                                .italics()
                                .color(pal.text_secondary),
                            );
                        } else if self.font_variants_loading {
                            ui.spinner();
                            ui.label("Fetching available variants from cloud...");
                        } else if self.font_variants.is_empty() {
                            ui.label(
                                egui::RichText::new("Standard / Default Weight").color(pal.accent),
                            );
                        } else {
                            let current_label = self
                                .selected_variant_idx
                                .and_then(|idx| self.font_variants.get(idx))
                                .map(|v| v.label.as_str())
                                .unwrap_or("Select variant");

                            egui::ComboBox::from_id_source("font_variant_selector_dropdown")
                                .selected_text(current_label)
                                .show_ui(ui, |ui| {
                                    let mut variant_changed = false;
                                    for (idx, var) in self.font_variants.iter().enumerate() {
                                        let is_sel = self.selected_variant_idx == Some(idx);
                                        if ui.selectable_label(is_sel, &var.label).clicked() {
                                            self.selected_variant_idx = Some(idx);
                                            variant_changed = true;
                                        }
                                    }
                                    if variant_changed {
                                        self.start_font_verification();
                                    }
                                });
                        }
                    });
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let is_checking = matches!(
                    self.font_check_state,
                    crate::font_service::FontCheckState::Checking(_)
                );
                let can_verify = !is_checking && !self.font_input_name.trim().is_empty();

                if ui
                    .add_enabled(
                        can_verify,
                        egui::Button::new(icon_label_spaced(
                            Icons::SEARCH,
                            Strings::verify_font_btn(lang),
                            6.0,
                        )),
                    )
                    .clicked()
                {
                    self.start_font_verification();
                }

                let is_available = matches!(
                    self.font_check_state,
                    crate::font_service::FontCheckState::Available(_)
                );

                let apply_btn = egui::Button::new(icon_label_spaced(
                    Icons::CHECK,
                    Strings::apply_font_btn(lang),
                    6.0,
                ))
                .fill(if is_available {
                    pal.accent
                } else {
                    Color32::from_rgb(45, 45, 45)
                });

                if ui.add_enabled(is_available, apply_btn).clicked() {
                    if let crate::font_service::FontCheckState::Available(ref verified) =
                        self.font_check_state.clone()
                    {
                        self.apply_verified_font(ui.ctx(), verified);
                    }
                }

                if ui.button(Strings::reset_font_btn(lang)).clicked() {
                    self.reset_to_default_font(ui.ctx());
                }
            });

            ui.add_space(6.0);
            match &self.font_check_state {
                crate::font_service::FontCheckState::Idle => {
                    ui.label(
                        egui::RichText::new(
                            "Enter a font name and click 'Verify Font' to check remote availability.",
                        )
                        .small()
                        .color(pal.text_secondary),
                    );
                }
                crate::font_service::FontCheckState::Checking(name) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            egui::RichText::new(format!(
                                "Querying public font repositories for '{}'...",
                                name
                            ))
                            .color(Color32::from_rgb(255, 200, 80)),
                        );
                    });
                }
                crate::font_service::FontCheckState::Available(verified) => {
                    ui.label(
                        egui::RichText::new(format!(
                            "[OK] Verified valid TrueType font '{}' from {} ({} KB). Click 'Apply Font' to activate.",
                            verified.display_name, verified.source_name, verified.file_size_kb
                        ))
                        .strong()
                        .color(Color32::from_rgb(46, 213, 115)),
                    );
                }
                crate::font_service::FontCheckState::NotFound(msg) => {
                    ui.label(
                        egui::RichText::new(format!("[!] Font unavailable: {}", msg))
                            .color(Color32::from_rgb(255, 71, 87))
                            .strong(),
                    );
                }
                crate::font_service::FontCheckState::Error(err) => {
                    ui.label(
                        egui::RichText::new(format!("[!] Error: {}", err))
                            .color(Color32::from_rgb(255, 71, 87))
                            .strong(),
                    );
                }
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading(Strings::dev_debug_title(lang));
            ui.label(egui::RichText::new(Strings::dev_debug_desc(lang)).color(pal.text_secondary));
            ui.add_space(8.0);

            let term_btn_color = if self.config.theme == ThemeMode::LightWhite {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            if ui
                .add_sized(
                    [280.0, 32.0],
                    egui::Button::new(icon_label_colored(
                        Icons::TERMINAL,
                        Strings::launch_terminal_btn(lang),
                        term_btn_color,
                    ))
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

            ui.add_space(4.0);
            let mut show_mode = self.config.show_mode_switcher;
            let mode_toggle_label = if lang == Language::Vietnamese {
                "Hiển thị nút chuyển chế độ ở thanh dưới (Gaming / Office) [Chưa hoàn thiện / Thử nghiệm]"
            } else {
                "Show Mode Switcher in Bottom Bar (Gaming / Office) [Unfinished / Experimental]"
            };
            if ui
                .checkbox(&mut show_mode, mode_toggle_label)
                .on_hover_text(
                    "Enables live on-the-fly mode toggle (0x01 Gaming / 0x00 Office) in the bottom toolbar. Note: live mode switching is an unfinished feature and may require a profile reload.",
                )
                .changed()
            {
                self.config.show_mode_switcher = show_mode;
                self.config.save();
            }

            ui.add_space(4.0);
            let mut disable_guide = self.config.disable_macro_guide;
            if ui
                .checkbox(
                    &mut disable_guide,
                    Strings::macro_guide_settings_toggle(lang),
                )
                .on_hover_text(
                    "When checked, hides the quick guide walkthrough panel, workflow steps, and helper hints in the Macro Manager tab.",
                )
                .changed()
            {
                self.config.disable_macro_guide = disable_guide;
                self.config.save();
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // On-demand Emergency Hardware Orientation & Axis Calibration Button
            let repair_btn_text = if lang == Language::Vietnamese {
                "Khôi phục hướng di chuột cảm biến (Sửa lỗi đảo trục / Lộn hướng)"
            } else {
                "Calibrate & Fix Sensor Tracking Orientation (Emergency Axis Repair)"
            };
            if ui
                .add_sized(
                    [360.0, 32.0],
                    egui::Button::new(icon_label_colored(
                        Icons::REFRESH,
                        repair_btn_text,
                        pal.accent,
                    ))
                    .stroke(Stroke::new(1.0_f32, pal.card_border)),
                )
                .on_hover_text(
                    "Immediately restores the verified hardware sensor orientation registers (0x0006 = 0x7F, 0x0007 = 0x7F) and flashes to mouse memory to eliminate inverted/swapped X/Y axes.",
                )
                .clicked()
            {
                self.repair_sensor_orientation_now();
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
        0x05 => format!("Macro #{} [EXPERIMENTAL]", p1 + 1),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_disable_macro_guide_default_and_serde() {
        let default_cfg = AppConfig::default();
        assert!(
            !default_cfg.disable_macro_guide,
            "disable_macro_guide must default to false"
        );

        // Deserializing empty JSON should default disable_macro_guide to false
        let json_empty = "{}";
        let parsed: AppConfig = serde_json::from_str(json_empty).unwrap();
        assert!(!parsed.disable_macro_guide);

        // Deserializing with disable_macro_guide = true
        let json_true = r#"{"disable_macro_guide": true}"#;
        let parsed_true: AppConfig = serde_json::from_str(json_true).unwrap();
        assert!(parsed_true.disable_macro_guide);

        // Roundtrip serialization
        let custom = AppConfig {
            disable_macro_guide: true,
            ..Default::default()
        };
        let serialized = serde_json::to_string(&custom).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.disable_macro_guide);
    }
}
