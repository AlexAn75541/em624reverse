use super::Em624App;
use crate::i18n::Strings;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use std::time::Instant;

impl Em624App {
    pub fn render_telemetry_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;
        let now = Instant::now();

        ui.add_space(6.0);
        ui.heading(Strings::tab_telemetry(lang));
        ui.label(
            RichText::new(
                "Real-time sensor polling rate telemetry, packet jitter analysis, and switch click latency diagnostics.",
            )
            .small()
            .color(pal.text_secondary),
        );
        ui.add_space(10.0);

        // Hardware Polling Rate from EEPROM profile
        let configured_rate = self.profile.get_polling_rate();

        // Update motion polling rate samples
        let current_pos = ui.input(|i| i.pointer.latest_pos());
        if let Some(pos) = current_pos {
            if let Some(last_pos) = self.telemetry_last_cursor_pos {
                let delta = pos - last_pos;
                if delta.length_sq() > 0.5 {
                    if let Some(last_time) = self.telemetry_last_motion_time {
                        let dt = (now - last_time).as_secs_f32();
                        if dt > 0.0005 && dt < 0.2 {
                            let inst_hz = 1.0 / dt;
                            self.telemetry_current_hz = inst_hz;
                            self.telemetry_total_events += 1;

                            if inst_hz > self.telemetry_peak_hz {
                                self.telemetry_peak_hz = inst_hz;
                            }
                            if self.telemetry_min_hz == 0.0 || inst_hz < self.telemetry_min_hz {
                                self.telemetry_min_hz = inst_hz;
                            }

                            // Exponential moving average for smooth display
                            if self.telemetry_avg_hz == 0.0 {
                                self.telemetry_avg_hz = inst_hz;
                            } else {
                                self.telemetry_avg_hz =
                                    self.telemetry_avg_hz * 0.92 + inst_hz * 0.08;
                            }

                            self.telemetry_samples.push(inst_hz);
                            if self.telemetry_samples.len() > 120 {
                                self.telemetry_samples.remove(0);
                            }
                        }
                    }
                    self.telemetry_last_motion_time = Some(now);
                }
            }
            self.telemetry_last_cursor_pos = Some(pos);
        }

        // Update CPS (Clicks Per Second) rolling window
        self.telemetry_recent_clicks
            .retain(|&t| (now - t).as_secs_f32() <= 1.0);
        self.telemetry_cps = self.telemetry_recent_clicks.len() as f32;

        let avail_w = ui.available_width();
        let col1_w = (avail_w * 0.55).max(340.0);
        let col2_w = (avail_w - col1_w - 20.0).max(280.0);

        ui.horizontal(|ui| {
            // Left Column: Real-Time Polling Rate Jitter Graph
            ui.vertical(|ui| {
                ui.set_width(col1_w);

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Sensor Polling Rate & Jitter Waveform");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Reset Stats").clicked() {
                                self.telemetry_samples.clear();
                                self.telemetry_current_hz = 0.0;
                                self.telemetry_peak_hz = 0.0;
                                self.telemetry_min_hz = 0.0;
                                self.telemetry_avg_hz = 0.0;
                                self.telemetry_total_events = 0;
                            }
                        });
                    });

                    ui.label(
                        RichText::new(
                            "Move mouse continuously across the window to feed telemetry stream:",
                        )
                        .small()
                        .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(8.0);

                    // Polling Rate Summary Metrics Cards
                    ui.horizontal(|ui| {
                        let card_width = ((col1_w - 30.0) / 4.0).max(70.0);

                        // Configured Target Rate
                        ui.group(|ui| {
                            ui.set_width(card_width);
                            ui.label(RichText::new("Target").small().color(pal.text_secondary));
                            ui.label(
                                RichText::new(format!("{} Hz", configured_rate))
                                    .strong()
                                    .color(Color32::from_rgb(0, 242, 254)),
                            );
                        });

                        // Current Instant Hz
                        ui.group(|ui| {
                            ui.set_width(card_width);
                            ui.label(RichText::new("Current").small().color(pal.text_secondary));
                            let col = if (self.telemetry_current_hz - configured_rate as f32).abs()
                                < 150.0
                            {
                                Color32::from_rgb(46, 213, 115)
                            } else {
                                Color32::from_rgb(255, 177, 66)
                            };
                            ui.label(
                                RichText::new(format!("{:.0} Hz", self.telemetry_current_hz))
                                    .strong()
                                    .color(col),
                            );
                        });

                        // Average Hz
                        ui.group(|ui| {
                            ui.set_width(card_width);
                            ui.label(RichText::new("Average").small().color(pal.text_secondary));
                            ui.label(
                                RichText::new(format!("{:.0} Hz", self.telemetry_avg_hz))
                                    .strong()
                                    .color(Color32::from_rgb(124, 77, 255)),
                            );
                        });

                        // Peak Hz
                        ui.group(|ui| {
                            ui.set_width(card_width);
                            ui.label(RichText::new("Peak").small().color(pal.text_secondary));
                            ui.label(
                                RichText::new(format!("{:.0} Hz", self.telemetry_peak_hz))
                                    .strong()
                                    .color(Color32::from_rgb(255, 71, 87)),
                            );
                        });
                    });

                    ui.add_space(10.0);

                    // Custom Vector Graph Canvas for Polling Rate Jitter
                    let graph_h = 180.0_f32;
                    let (graph_rect, _) = ui.allocate_exact_size(
                        Vec2::new(col1_w - 20.0, graph_h),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(graph_rect);

                    // Graph Background & Border
                    painter.rect(
                        graph_rect,
                        6.0,
                        Color32::from_rgb(14, 18, 26),
                        Stroke::new(1.0_f32, Color32::from_rgb(35, 45, 65)),
                    );

                    let max_scale_hz = 1200.0_f32;

                    // Reference Grid Lines
                    let ref_rates = [
                        (
                            1000.0,
                            "1000 Hz",
                            Color32::from_rgba_premultiplied(0, 242, 254, 70),
                        ),
                        (
                            500.0,
                            "500 Hz",
                            Color32::from_rgba_premultiplied(46, 213, 115, 60),
                        ),
                        (
                            250.0,
                            "250 Hz",
                            Color32::from_rgba_premultiplied(255, 177, 66, 50),
                        ),
                        (
                            125.0,
                            "125 Hz",
                            Color32::from_rgba_premultiplied(255, 71, 87, 40),
                        ),
                    ];

                    for (ref_hz, label, stroke_col) in &ref_rates {
                        let y_ratio = 1.0 - (ref_hz / max_scale_hz).clamp(0.0, 1.0);
                        let y_pos = graph_rect.min.y + y_ratio * graph_h;
                        painter.line_segment(
                            [
                                Pos2::new(graph_rect.min.x, y_pos),
                                Pos2::new(graph_rect.max.x, y_pos),
                            ],
                            Stroke::new(1.0_f32, *stroke_col),
                        );
                        painter.text(
                            Pos2::new(graph_rect.min.x + 6.0, y_pos - 8.0),
                            egui::Align2::LEFT_TOP,
                            *label,
                            egui::FontId::monospace(9.0),
                            Color32::from_rgba_premultiplied(140, 160, 190, 180),
                        );
                    }

                    // Plot Telemetry Samples Line & Fill
                    if self.telemetry_samples.len() > 1 {
                        let sample_count = self.telemetry_samples.len();
                        let step_x = (graph_rect.width() - 10.0) / 120.0;

                        let mut points: Vec<Pos2> = Vec::with_capacity(sample_count);
                        for (idx, &hz) in self.telemetry_samples.iter().enumerate() {
                            let x =
                                graph_rect.min.x + 5.0 + (120 - sample_count + idx) as f32 * step_x;
                            let y_ratio = 1.0 - (hz / max_scale_hz).clamp(0.0, 1.0);
                            let y = graph_rect.min.y + y_ratio * graph_h;
                            points.push(Pos2::new(x, y));
                        }

                        // Draw wave segments
                        for w in points.windows(2) {
                            painter.line_segment(
                                [w[0], w[1]],
                                Stroke::new(2.0_f32, Color32::from_rgb(0, 242, 254)),
                            );
                        }

                        // Draw glowing head dot
                        if let Some(&last_pt) = points.last() {
                            painter.circle_filled(last_pt, 4.0, Color32::from_rgb(0, 242, 254));
                            painter.circle_stroke(
                                last_pt,
                                7.0,
                                Stroke::new(
                                    1.5_f32,
                                    Color32::from_rgba_premultiplied(0, 242, 254, 120),
                                ),
                            );
                        }
                    } else {
                        painter.text(
                            graph_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Move mouse across window to visualize polling stream...",
                            egui::FontId::proportional(12.0),
                            Color32::from_rgb(80, 100, 130),
                        );
                    }

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "Stream Packets Observed: {} | Jitter Band: {:.0} Hz - {:.0} Hz",
                            self.telemetry_total_events,
                            self.telemetry_min_hz,
                            self.telemetry_peak_hz
                        ))
                        .monospace()
                        .size(10.0)
                        .color(pal.text_secondary),
                    );
                });
            });

            // Right Column: Switch Debounce & Click Latency Tester
            ui.vertical(|ui| {
                ui.set_width(col2_w);

                ui.group(|ui| {
                    ui.heading("Switch Debounce & Latency Tester");
                    ui.label(
                        RichText::new("Measure Huano 50M switch response and physical debounce:")
                            .small()
                            .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(8.0);

                    // Click Arena Button / Pad
                    let pad_h = 100.0_f32;
                    let (pad_rect, pad_resp) = ui.allocate_exact_size(
                        Vec2::new(col2_w - 20.0, pad_h),
                        egui::Sense::click_and_drag(),
                    );
                    let pad_painter = ui.painter_at(pad_rect);

                    let is_active = pad_resp.is_pointer_button_down_on();
                    let is_hover = pad_resp.hovered();

                    if pad_resp.clicked()
                        || (ui.input(|i| i.pointer.primary_down())
                            && is_hover
                            && self.telemetry_click_down_instant.is_none())
                    {
                        self.telemetry_click_down_instant = Some(now);
                    }

                    if ui.input(|i| i.pointer.primary_released())
                        && self.telemetry_click_down_instant.is_some()
                    {
                        if let Some(down_time) = self.telemetry_click_down_instant.take() {
                            let duration_ms = (now - down_time).as_secs_f32() * 1000.0;
                            self.telemetry_clicks += 1;
                            self.telemetry_last_hold_ms = duration_ms;
                            self.telemetry_recent_clicks.push(now);

                            if self.telemetry_min_hold_ms == 0.0
                                || duration_ms < self.telemetry_min_hold_ms
                            {
                                self.telemetry_min_hold_ms = duration_ms;
                            }
                            if duration_ms > self.telemetry_max_hold_ms {
                                self.telemetry_max_hold_ms = duration_ms;
                            }
                        }
                    }

                    let (pad_bg, pad_stroke) = if is_active {
                        (
                            Color32::from_rgb(0, 120, 160),
                            Color32::from_rgb(0, 242, 254),
                        )
                    } else if is_hover {
                        (
                            Color32::from_rgb(25, 35, 52),
                            Color32::from_rgb(0, 180, 230),
                        )
                    } else {
                        (Color32::from_rgb(18, 22, 32), Color32::from_rgb(45, 58, 82))
                    };

                    pad_painter.rect(pad_rect, 8.0, pad_bg, Stroke::new(1.5_f32, pad_stroke));

                    let center = pad_rect.center();
                    let title_text = if is_active {
                        "[SWITCH DOWN]"
                    } else {
                        "CLICK HERE TO TEST SWITCH LATENCY"
                    };

                    pad_painter.text(
                        Pos2::new(center.x, center.y - 10.0),
                        egui::Align2::CENTER_CENTER,
                        title_text,
                        egui::FontId::proportional(12.0),
                        if is_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0, 242, 254)
                        },
                    );

                    pad_painter.text(
                        Pos2::new(center.x, center.y + 12.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.1} Clicks / Sec (CPS)", self.telemetry_cps),
                        egui::FontId::monospace(11.0),
                        Color32::from_rgb(180, 200, 230),
                    );

                    ui.add_space(8.0);

                    // Switch Latency Stats Grid
                    egui::Grid::new("switch_stats_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Total Clicks Counted:");
                            ui.label(RichText::new(self.telemetry_clicks.to_string()).strong());
                            ui.end_row();

                            ui.label("Last Contact Duration:");
                            ui.label(
                                RichText::new(format!("{:.1} ms", self.telemetry_last_hold_ms))
                                    .strong()
                                    .color(Color32::from_rgb(0, 242, 254)),
                            );
                            ui.end_row();

                            ui.label("Min Contact Duration:");
                            ui.label(
                                RichText::new(format!("{:.1} ms", self.telemetry_min_hold_ms))
                                    .color(Color32::from_rgb(46, 213, 115)),
                            );
                            ui.end_row();

                            ui.label("Max Contact Duration:");
                            ui.label(
                                RichText::new(format!("{:.1} ms", self.telemetry_max_hold_ms))
                                    .color(Color32::from_rgb(255, 177, 66)),
                            );
                            ui.end_row();

                            ui.label("Switch Debounce Health:");
                            if self.telemetry_clicks > 0 && self.telemetry_min_hold_ms < 4.0 {
                                ui.label(
                                    RichText::new("[CHATTER WARNING]")
                                        .color(Color32::from_rgb(255, 71, 87))
                                        .strong(),
                                );
                            } else if self.telemetry_clicks > 0 {
                                ui.label(
                                    RichText::new("[HEALTHY - NO BOUNCE]")
                                        .color(Color32::from_rgb(46, 213, 115))
                                        .strong(),
                                );
                            } else {
                                ui.label(
                                    RichText::new("Awaiting clicks...").color(pal.text_secondary),
                                );
                            }
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    if ui.button("Reset Click Session").clicked() {
                        self.telemetry_clicks = 0;
                        self.telemetry_last_hold_ms = 0.0;
                        self.telemetry_min_hold_ms = 0.0;
                        self.telemetry_max_hold_ms = 0.0;
                        self.telemetry_cps = 0.0;
                        self.telemetry_recent_clicks.clear();
                    }
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("Hardware Sensor Specification");
                    ui.label(RichText::new("Controller : SunplusIT SPCP6662 / 6662IC").small());
                    ui.label(
                        RichText::new("Resolution : 14 discrete steps (200 - 10,000 DPI)").small(),
                    );
                    ui.label(RichText::new("Max Speed  : 60 IPS | Acceleration: 20G").small());
                    ui.label(RichText::new("Switches   : Huano Blue Shell 50M Clicks").small());
                    ui.label(
                        RichText::new("DSP Matrix : 180° Inverted (Locked Byte 0x0006=0x7F)")
                            .small()
                            .color(Color32::from_rgb(46, 213, 115)),
                    );
                });
            });
        });
    }
}
