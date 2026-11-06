use crate::i18n::Language;
use eframe::egui::{self, Color32};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Tab {
    GameMode,
    OfficeMode,
    Advanced,
    LedLighting,
    MacroManager,
    Telemetry,
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

#[allow(dead_code)]
impl ThemePalette {
    #[allow(dead_code)]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::PitchBlackOled => Self {
                bg_base: Color32::from_rgb(0, 0, 0),
                panel_bg: Color32::from_rgb(10, 10, 12),
                card_bg: Color32::from_rgb(16, 16, 20),
                card_border: Color32::from_rgb(32, 32, 40),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_secondary: Color32::from_rgb(160, 160, 175),
                accent: Color32::from_rgb(0, 242, 254),
                accent_hover: Color32::from_rgb(79, 172, 254),
                mouse_bg: Color32::from_rgb(22, 22, 28),
                badge_bg: Color32::from_rgb(20, 20, 26),
                badge_text: Color32::from_rgb(0, 242, 254),
                badge_border: Color32::from_rgb(0, 242, 254),
            },
            ThemeMode::Windows11Dark => Self {
                bg_base: Color32::from_rgb(32, 32, 32),
                panel_bg: Color32::from_rgb(38, 38, 38),
                card_bg: Color32::from_rgb(45, 45, 45),
                card_border: Color32::from_rgb(60, 60, 60),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_secondary: Color32::from_rgb(200, 200, 200),
                accent: Color32::from_rgb(96, 205, 255),
                accent_hover: Color32::from_rgb(120, 215, 255),
                mouse_bg: Color32::from_rgb(48, 48, 48),
                badge_bg: Color32::from_rgb(45, 45, 45),
                badge_text: Color32::from_rgb(96, 205, 255),
                badge_border: Color32::from_rgb(96, 205, 255),
            },
            ThemeMode::LightWhite => Self {
                bg_base: Color32::from_rgb(243, 243, 243),
                panel_bg: Color32::from_rgb(255, 255, 255),
                card_bg: Color32::from_rgb(255, 255, 255),
                card_border: Color32::from_rgb(215, 215, 215),
                text_primary: Color32::from_rgb(10, 10, 10),
                text_secondary: Color32::from_rgb(90, 90, 90),
                accent: Color32::from_rgb(0, 103, 192),
                accent_hover: Color32::from_rgb(24, 120, 205),
                mouse_bg: Color32::from_rgb(235, 235, 235),
                badge_bg: Color32::from_rgb(255, 255, 255),
                badge_text: Color32::from_rgb(0, 103, 192),
                badge_border: Color32::from_rgb(0, 103, 192),
            },
        }
    }
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
            disable_macro_guide: true, // Guiding widgets removed per user instruction
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

pub struct Icons;

#[allow(dead_code)]
impl Icons {
    pub const GAMEPAD: &'static str = "\u{f11b}";
    pub const BRIEFCASE: &'static str = "\u{f0b1}";
    pub const COGS: &'static str = "\u{f085}";
    pub const LIGHTBULB: &'static str = "\u{f0eb}";
    pub const CODE: &'static str = "\u{f121}";
    pub const SETTINGS: &'static str = "\u{f013}";
    pub const DOWNLOAD: &'static str = "\u{f019}";
    pub const UPLOAD: &'static str = "\u{f093}";
    pub const FOLDER_OPEN: &'static str = "\u{f07c}";
    pub const SAVE: &'static str = "\u{f0c7}";
    pub const REFRESH: &'static str = "\u{f021}";
    pub const CHART_BAR: &'static str = "\u{f080}";
    pub const TACHOMETER: &'static str = "\u{f0e4}";
    pub const MOUSE: &'static str = "\u{f11b}";
    pub const KEYBOARD: &'static str = "\u{f11c}";
    pub const MEDIA: &'static str = "\u{f001}";
    pub const COMPASS: &'static str = "\u{f14e}";
    pub const INFO_CIRCLE: &'static str = "\u{f05a}";
    pub const CHECK: &'static str = "\u{f00c}";
    pub const TIMES: &'static str = "\u{f00d}";
    pub const BUG: &'static str = "\u{f188}";
    pub const TRASH: &'static str = "\u{f1f8}";
    pub const TERMINAL: &'static str = "\u{f120}";
    pub const COMPACT: &'static str = "\u{f0e2}";
    pub const SEARCH: &'static str = "\u{f002}";
}

pub fn icon_label_spaced(icon: &str, text: &str, space_px: f32) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        icon,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            ..Default::default()
        },
    );
    job.append(
        text,
        space_px,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            ..Default::default()
        },
    );
    egui::WidgetText::from(job)
}

pub fn icon_label(icon: &str, text: &str) -> egui::WidgetText {
    icon_label_spaced(icon, text, 8.0)
}

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

    fonts.font_data.insert(
        "Dosis-Bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Dosis-Bold.ttf")),
    );

    fonts.font_data.insert(
        "CascadiaMono-NF".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/CaskaydiaMonoNerdFont-Bold.ttf"
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

    prop.push("Dosis-Bold".to_owned());
    prop.push("CascadiaMono-NF".to_owned());

    mono.push("CascadiaMono-NF".to_owned());
    mono.push("Dosis-Bold".to_owned());

    fonts.families.insert(egui::FontFamily::Proportional, prop);
    fonts.families.insert(egui::FontFamily::Monospace, mono);

    ctx.set_fonts(fonts);
}

pub fn format_time_date_log() -> String {
    let now = chrono::Local::now();
    now.format("%H-%M-%S_%Y-%m-%d.log").to_string()
}

pub fn chrono_or_simple_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        (now / 3600) % 24,
        (now / 60) % 60,
        now % 60
    )
}

pub fn trim_working_set() {
    crate::platform::trim_working_set();
}
