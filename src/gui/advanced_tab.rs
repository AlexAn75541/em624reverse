use super::Em624App;
use crate::i18n::Strings;
use crate::profile::OFFICIAL_DPI_STEPS;
use eframe::egui::{self, Color32};

impl Em624App {
    pub fn render_advanced_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;
        ui.add_space(4.0);
        ui.heading(Strings::tab_advanced(lang));
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
                    ui.heading("Fire Key Setting [UNFINISHED - DEMO UI ONLY]");
                    ui.label(
                        egui::RichText::new(
                            "Demo UI only: hardware fire key bytecode failed in testing.",
                        )
                        .small()
                        .color(pal.text_secondary),
                    );
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
                                let current_step =
                                    self.profile.dpi_stages[i].level_index.min(14) as usize;
                                let color = self.profile.dpi_stages[i].color.clone();

                                let (num_stages, current_stage_idx) =
                                    self.profile.get_active_dpi_config();
                                let is_cur_stage = current_stage_idx == (i as u8);
                                let stage_label = if is_cur_stage {
                                    format!("* Stage {}", i + 1)
                                } else {
                                    format!("  Stage {}", i + 1)
                                };
                                if ui
                                    .selectable_label(is_cur_stage, stage_label)
                                    .on_hover_text("Click to set as current active DPI stage (0x00A6)")
                                    .clicked()
                                {
                                    let new_num = num_stages.max(i as u8 + 1);
                                    self.profile.set_active_dpi_config(new_num, i as u8);
                                }

                                let mut active_checked = (i as u8) < num_stages;
                                if i == 5 && !self.config.allow_stage_6 {
                                    if ui
                                        .checkbox(&mut active_checked, "")
                                        .on_hover_text("Stage 6 is locked by default. Click to unlock (Synchronized with App Settings).")
                                        .changed()
                                        && active_checked
                                    {
                                        self.config.allow_stage_6 = true;
                                        self.config.save();
                                        self.profile.set_active_dpi_config(6, current_stage_idx);
                                    }
                                } else if ui.checkbox(&mut active_checked, "").changed() {
                                    let max_allowed = if self.config.allow_stage_6 { 6 } else { 5 };
                                    let new_num = if active_checked {
                                        (i as u8 + 1).clamp(1, max_allowed)
                                    } else {
                                        (i as u8).clamp(1, max_allowed)
                                    };
                                    self.profile.set_active_dpi_config(new_num, current_stage_idx);
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
                                    self.profile.recalculate_checksums();
                                    self.profile.refresh_stages();
                                }

                                let mut color_rgb = [color.r, color.g, color.b];
                                if ui.color_edit_button_srgb(&mut color_rgb).changed() {
                                    self.profile.set_stage_color(
                                        i,
                                        crate::profile::RgbColor {
                                            r: color_rgb[0],
                                            g: color_rgb[1],
                                            b: color_rgb[2],
                                        },
                                    );
                                }

                                ui.end_row();
                            }
                        });

                    ui.add_space(8.0);
                    let mut allow_s6 = self.config.allow_stage_6;
                    if ui
                        .checkbox(&mut allow_s6, "Enable 6th DPI Stage (Synchronized with App Settings)")
                        .on_hover_text(
                            "When disabled, mouse DPI stages are safely capped to 5 steps (preventing Stage 6 firmware conflicts). When enabled, unlocks full 6 stages across all tabs.",
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
                    }
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("USB Report Rate (Polling Speed)");
                    ui.label(
                        egui::RichText::new(
                            "Hardware USB interrupt report frequency (125 Hz - 1000 Hz):",
                        )
                        .small()
                        .color(pal.text_secondary),
                    );
                    ui.add_space(6.0);

                    let current_rate = self.profile.get_polling_rate();
                    let mut rate_sel = match current_rate {
                        125 => 0,
                        250 => 1,
                        500 => 2,
                        1000 => 3,
                        _ => 3,
                    };

                    let mut changed_rate = None;
                    ui.horizontal(|ui| {
                        if ui.radio_value(&mut rate_sel, 0, "125 Hz (8 ms)").clicked() {
                            changed_rate = Some(125);
                        }
                        if ui.radio_value(&mut rate_sel, 1, "250 Hz (4 ms)").clicked() {
                            changed_rate = Some(250);
                        }
                        if ui.radio_value(&mut rate_sel, 2, "500 Hz (2 ms)").clicked() {
                            changed_rate = Some(500);
                        }
                        if ui.radio_value(&mut rate_sel, 3, "1000 Hz (1 ms)").clicked() {
                            changed_rate = Some(1000);
                        }
                    });

                    if let Some(rate) = changed_rate {
                        self.profile.set_polling_rate(rate);
                        self.status_message = format!(
                            "USB Polling Rate set to {} Hz. Click 'Apply Settings' to burn to mouse.",
                            rate
                        );
                    }

                    if rate_sel == 3 {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(
                                "Notice: 1000 Hz report rate requires high CPU interrupt budget; optimal for high-refresh gaming displays.",
                            )
                            .small()
                            .color(Color32::from_rgb(140, 180, 230)),
                        );
                    }
                });
            });
        });
    }
}
