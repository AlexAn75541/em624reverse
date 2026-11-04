use super::config::{icon_label, icon_label_colored, icon_label_colored_spaced, Icons, ThemeMode};
use super::Em624App;
use crate::i18n::Strings;
use eframe::egui::{self, Color32, ProgressBar, RichText, Stroke};

impl Em624App {
    /// 1st Floor: Bottom Action Button Bar (At the very bottom of the window)
    pub fn render_first_floor_action_bar(&mut self, ctx: &egui::Context) {
        let pal = self.palette();
        let busy = self.is_io_busy();

        egui::TopBottomPanel::bottom("bottom_action_floor_1")
            .exact_height(38.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // Read from Mouse button
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(icon_label(
                                Icons::DOWNLOAD,
                                Strings::btn_read(self.config.language),
                            )),
                        )
                        .clicked()
                    {
                        self.read_from_mouse();
                    }

                    // Flash to Mouse button
                    let flash_btn_color = if self.config.theme == ThemeMode::LightWhite {
                        Color32::WHITE
                    } else {
                        Color32::BLACK
                    };
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(icon_label_colored_spaced(
                                Icons::UPLOAD,
                                Strings::btn_flash(self.config.language),
                                flash_btn_color,
                                12.0,
                            ))
                            .fill(pal.accent),
                        )
                        .clicked()
                    {
                        self.flash_to_mouse();
                    }

                    // Optional Mode Switcher button
                    if self.config.show_mode_switcher {
                        ui.separator();

                        let is_gaming = (self.profile.raw[0x00a4] & 0x0f) != 0;
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
                            .add_enabled(
                                !busy,
                                egui::Button::new(icon_label_colored(
                                    mode_icon,
                                    &mode_btn_text,
                                    mode_color,
                                ))
                                .stroke(Stroke::new(1.0_f32, pal.card_border)),
                            )
                            .on_hover_text(
                                "Experimental Mode Switcher: Click to toggle between Gaming Mode (0x01) and Office Mode (0x00).",
                            )
                            .clicked()
                        {
                            self.switch_mode_and_forward(!is_gaming);
                        }
                    }

                    ui.separator();

                    // Load Profile button
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(icon_label(
                                Icons::FOLDER_OPEN,
                                Strings::btn_load(self.config.language),
                            )),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Profiles (*.em624, *.bin)", &["em624", "bin"])
                            .pick_file()
                        {
                            self.load_profile_auto(path);
                        }
                    }

                    // Save Profile button
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(icon_label(
                                Icons::SAVE,
                                Strings::btn_save(self.config.language),
                            )),
                        )
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
                                self.status_message =
                                    format!("Saved profile to: {}", path.display());
                                self.log_debug(&format!(
                                    "[SAVE] Saved profile to: {}",
                                    path.display()
                                ));
                            }
                        }
                    }
                });
            });
    }

    /// 2nd Floor: Dedicated Status Report & Progress Bar (Directly on top of 1st Floor Action Bar)
    /// Has the exact same width and height as the 1st floor action bar, creating a clean stacked 2-story bar.
    pub fn render_second_floor_status_bar(&mut self, ctx: &egui::Context) {
        let pal = self.palette();

        egui::TopBottomPanel::bottom("bottom_status_floor_2")
            .exact_height(38.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Left side: Status message
                    ui.label(
                        RichText::new(&self.status_message)
                            .color(pal.text_secondary)
                            .size(13.0)
                            .strong(),
                    );

                    // Right side: Progress Bar during Read/Flash, or USB Connection status badge
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(frac) = self.io_progress {
                            ui.add(
                                ProgressBar::new(frac)
                                    .desired_width(220.0)
                                    .desired_height(18.0)
                                    .show_percentage()
                                    .animate(true),
                            );
                        } else {
                            let badge_text = if self.device_connected {
                                "USB: 1BCF:08B8 ONLINE"
                            } else {
                                "USB: OFFLINE"
                            };
                            let badge_col = if self.device_connected {
                                Color32::from_rgb(46, 213, 115)
                            } else {
                                Color32::from_rgb(255, 170, 0)
                            };
                            ui.label(
                                RichText::new(badge_text)
                                    .monospace()
                                    .small()
                                    .color(badge_col),
                            );
                        }
                    });
                });
            });
    }
}
