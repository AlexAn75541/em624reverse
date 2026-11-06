use super::config::{icon_label_colored, icon_label_spaced, Icons, ThemeMode};
use super::Em624App;
use crate::i18n::{Language, Strings};
use eframe::egui::{self, Color32, Stroke};
use std::fs;
use std::path::PathBuf;

impl Em624App {
    pub fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
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
                if !allow_s6 {
                    let (num, cur) = self.profile.get_active_dpi_config();
                    if num > 5 {
                        self.profile.set_active_dpi_config(5, cur.min(4));
                        self.profile.recalculate_checksums();
                        self.save_to_cached_profile();
                    }
                }
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
                ui.label(egui::RichText::new("Version:").strong());
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .color(Color32::from_rgb(16, 185, 129))
                        .strong(),
                );
            });
            ui.add_space(2.0);
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
