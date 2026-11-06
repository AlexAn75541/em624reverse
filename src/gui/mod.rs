pub mod advanced_tab;
pub mod buttons_tab;
pub mod config;
pub mod io_worker;
pub mod led_tab;
pub mod macro_tab;
pub mod mode_controller;
pub mod rebind_modal;
pub mod settings_tab;
pub mod status_bar;
pub mod telemetry_tab;

pub use config::{
    chrono_or_simple_time, configure_fonts, format_time_date_log, icon_label, icon_label_colored,
    icon_label_spaced, sanitize_font_filename, trim_working_set, AppConfig, Icons, Tab, ThemeMode,
    ThemePalette,
};
pub use io_worker::IoProgressMsg;

use crate::i18n::{Language, Strings};
use crate::macro_engine::{MacroAction, MacroBank, MacroEntry, MouseButton};
use crate::profile::MouseProfile;
use eframe::egui::{self, Color32, Vec2};
use std::fs;
use std::path::PathBuf;

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
    pub io_rx: Option<std::sync::mpsc::Receiver<IoProgressMsg>>,
    pub io_progress: Option<f32>,
    pub pending_led_sync: Option<(std::time::Instant, String)>,
    // Telemetry & Diagnostics Cockpit
    pub telemetry_samples: Vec<f32>,
    pub telemetry_last_cursor_pos: Option<egui::Pos2>,
    pub telemetry_last_motion_time: Option<std::time::Instant>,
    pub telemetry_current_hz: f32,
    pub telemetry_peak_hz: f32,
    pub telemetry_avg_hz: f32,
    pub telemetry_min_hz: f32,
    pub telemetry_total_events: u64,
    pub telemetry_clicks: u32,
    pub telemetry_click_down_instant: Option<std::time::Instant>,
    pub telemetry_last_hold_ms: f32,
    pub telemetry_min_hold_ms: f32,
    pub telemetry_max_hold_ms: f32,
    pub telemetry_cps: f32,
    pub telemetry_recent_clicks: Vec<std::time::Instant>,
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
            let baseline = include_bytes!("../../exampleProfiles/DailyUse.bin");
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
            io_rx: None,
            io_progress: None,
            pending_led_sync: None,
            telemetry_samples: Vec::with_capacity(128),
            telemetry_last_cursor_pos: None,
            telemetry_last_motion_time: None,
            telemetry_current_hz: 0.0,
            telemetry_peak_hz: 0.0,
            telemetry_avg_hz: 0.0,
            telemetry_min_hz: 0.0,
            telemetry_total_events: 0,
            telemetry_clicks: 0,
            telemetry_click_down_instant: None,
            telemetry_last_hold_ms: 0.0,
            telemetry_min_hold_ms: 0.0,
            telemetry_max_hold_ms: 0.0,
            telemetry_cps: 0.0,
            telemetry_recent_clicks: Vec::with_capacity(64),
        };

        app.sync_from_profile();
        app.log_debug("[STARTUP] E-DRA EM624 Configurator GUI session started.");
        app
    }
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

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if self.mouse_texture.is_none() {
            let png_bytes = include_bytes!("../../damouse.png");
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
}

impl Em624App {
    fn start_font_verification(&mut self) {
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
        self.poll_io_worker(ctx);

        if let Some((instant, desc)) = self.pending_led_sync.clone() {
            if instant.elapsed() >= std::time::Duration::from_millis(250) && !self.is_io_busy() {
                self.pending_led_sync = None;
                self.dispatch_led_sync(&desc);
            } else if !self.is_io_busy() {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
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
                self.render_mode_tab_item(ui, true);
                ui.add_space(6.0);
                self.render_mode_tab_item(ui, false);
                ui.add_space(6.0);
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
                    Tab::Telemetry,
                    icon_label_spaced(Icons::CHART_BAR, Strings::tab_telemetry(self.config.language), 8.0),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Settings,
                    icon_label(Icons::SETTINGS, Strings::tab_settings(self.config.language)),
                );
            });
            ui.add_space(6.0);
        });

        // 1st Floor: Action Buttons Toolbar (Bottom-most bar)
        self.render_first_floor_action_bar(ctx);

        // 2nd Floor: Dedicated Status & Non-blocking IO Progress Bar (Stacked directly above action buttons)
        self.render_second_floor_status_bar(ctx);

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
                    Tab::Telemetry => self.render_telemetry_tab(ui),
                    Tab::Settings => self.render_settings_tab(ui),
                });
        });

        // Modal Button Remapper Dialog if open
        if let Some((slot_idx, is_gaming)) = self.editing_slot {
            self.render_rebind_modal(ctx, slot_idx, is_gaming);
        }
    }
}
