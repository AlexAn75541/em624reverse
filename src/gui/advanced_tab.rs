use super::Em624App;
use crate::i18n::Strings;
use crate::profile::OFFICIAL_DPI_STEPS;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

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
                    ui.add_space(6.0);

                    // Visual DPI Spectrum Bars
                    let (num_stages, current_stage_idx) = self.profile.get_active_dpi_config();
                    let spectrum_h = 32.0_f32;
                    let (spec_rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width() - 8.0, spectrum_h),
                        egui::Sense::hover(),
                    );
                    let spec_painter = ui.painter_at(spec_rect);
                    spec_painter.rect(
                        spec_rect,
                        4.0,
                        Color32::from_rgb(14, 18, 26),
                        Stroke::new(1.0_f32, Color32::from_rgb(35, 45, 65)),
                    );

                    let stage_w = (spec_rect.width() - 8.0) / (num_stages as f32).max(1.0);
                    for s in 0..num_stages as usize {
                        let step_idx = self.profile.dpi_stages[s].level_index.min(14) as usize;
                        let dpi_val = OFFICIAL_DPI_STEPS[step_idx];
                        let c = &self.profile.dpi_stages[s].color;
                        let bar_color = Color32::from_rgb(c.r, c.g, c.b);
                        let is_cur = s == current_stage_idx as usize;

                        let x0 = spec_rect.min.x + 4.0 + s as f32 * stage_w;
                        let bar_rect = Rect::from_min_max(
                            Pos2::new(x0 + 2.0, spec_rect.min.y + 3.0),
                            Pos2::new(x0 + stage_w - 2.0, spec_rect.max.y - 3.0),
                        );

                        spec_painter.rect(
                            bar_rect,
                            3.0,
                            if is_cur { bar_color } else { Color32::from_rgba_premultiplied(c.r / 3, c.g / 3, c.b / 3, 200) },
                            Stroke::new(if is_cur { 1.5_f32 } else { 1.0_f32 }, if is_cur { Color32::WHITE } else { bar_color }),
                        );

                        spec_painter.text(
                            bar_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("S{}: {}", s + 1, dpi_val),
                            egui::FontId::monospace(9.5),
                            if is_cur { Color32::BLACK } else { Color32::from_rgb(220, 230, 245) },
                        );
                    }
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
                    let rates = [
                        (125, "125 Hz", "8 ms"),
                        (250, "250 Hz", "4 ms"),
                        (500, "500 Hz", "2 ms"),
                        (1000, "1000 Hz", "1 ms"),
                    ];

                    let mut changed_rate = None;
                    ui.horizontal(|ui| {
                        for (rate, label, latency) in &rates {
                            let is_active = current_rate == *rate;
                            let (bg_col, stroke_col, text_col) = if is_active {
                                (
                                    Color32::from_rgb(0, 180, 230),
                                    Color32::from_rgb(0, 242, 254),
                                    Color32::BLACK,
                                )
                            } else {
                                (
                                    Color32::from_rgb(20, 26, 38),
                                    Color32::from_rgb(45, 60, 85),
                                    Color32::from_rgb(180, 200, 230),
                                )
                            };

                            let btn_text = egui::RichText::new(format!("{} ({})", label, latency))
                                .size(11.0)
                                .strong()
                                .color(text_col);

                            let btn = egui::Button::new(btn_text)
                                .fill(bg_col)
                                .stroke(Stroke::new(1.0_f32, stroke_col))
                                .rounding(4.0);

                            if ui.add(btn).clicked() {
                                changed_rate = Some(*rate);
                            }
                        }
                    });

                    if let Some(rate) = changed_rate {
                        self.profile.set_polling_rate(rate);
                        self.status_message = format!(
                            "USB Polling Rate set to {} Hz. Click 'Apply Settings' to burn to mouse.",
                            rate
                        );
                    }

                    if current_rate == 1000 {
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

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Sensor Optical DSP Axis Matrix");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("[LOCKED 180°]")
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::from_rgb(46, 213, 115)),
                            );
                        });
                    });
                    ui.label(
                        egui::RichText::new(
                            "SPCP6662 Optical DSP Coordinate Transformation (Register Byte 0x0006 = 0x7F).",
                        )
                        .small()
                        .color(pal.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Safety Guard: Enforces 180° rotation matching physical PCB sensor chassis orientation. Prevents 90° tracking inversion.",
                        )
                        .small()
                        .color(Color32::from_rgb(140, 180, 230)),
                    );
                });
            });
        });
    }
}
