use super::Em624App;
use crate::i18n::Strings;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};

impl Em624App {
    pub fn render_led_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;
        let time = ui.input(|i| i.time);

        ui.add_space(6.0);
        ui.heading(Strings::tab_led(lang));
        ui.label(
            RichText::new(
                "Visual LED Studio: Real-time lighting simulator, 8 hardware illumination modes, and stage color tuning.",
            )
            .small()
            .color(pal.text_secondary),
        );
        ui.add_space(10.0);

        let cur_mode = self.profile.get_led_mode();
        let cur_speed = self.profile.get_led_speed();
        let cur_brightness_half = self.profile.get_led_brightness() == 1;
        let (_num_stages, cur_stage_idx) = self.profile.get_active_dpi_config();
        let cur_stage_color = self.profile.dpi_stages[cur_stage_idx as usize % 6]
            .color
            .clone();

        // Calculate animated preview color based on mode and speed
        let speed_factor = 0.5 + (cur_speed as f64) * 0.45;
        let bright_mult = if cur_brightness_half {
            0.55_f32
        } else {
            1.0_f32
        };

        let preview_rgb: Color32 = match cur_mode {
            0 => {
                // CPI Breathing: Breathes with current stage RGB color
                let wave = ((time * speed_factor).sin() * 0.5 + 0.5) as f32;
                let intensity = (0.2 + 0.8 * wave) * bright_mult;
                Color32::from_rgb(
                    (cur_stage_color.r as f32 * intensity) as u8,
                    (cur_stage_color.g as f32 * intensity) as u8,
                    (cur_stage_color.b as f32 * intensity) as u8,
                )
            }
            1 => {
                // Multicolor Breathing: Smooth HSL hue sweep
                let wave = ((time * speed_factor).sin() * 0.5 + 0.5) as f32;
                let intensity = (0.25 + 0.75 * wave) * bright_mult;
                let hue = (time * speed_factor * 0.4) % 1.0;
                let rgb = hsl_to_rgb(hue as f32, 1.0, 0.5 * intensity);
                Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            }
            2 => {
                // Rainbow / Steady: Steady multi-spectrum color
                let hue = (time * speed_factor * 0.25) % 1.0;
                let rgb = hsl_to_rgb(hue as f32, 1.0, 0.5 * bright_mult);
                Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            }
            3 => {
                // Floe Light (Flow): Rapid spectral flow
                let hue = (time * speed_factor * 0.8) % 1.0;
                let rgb = hsl_to_rgb(hue as f32, 1.0, 0.55 * bright_mult);
                Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            }
            4 => {
                // Waltz: Dancing rhythmic pulse
                let wave = ((time * speed_factor * 1.5).sin().abs() as f32) * bright_mult;
                Color32::from_rgb(
                    (120.0 * wave) as u8,
                    (220.0 * wave) as u8,
                    (255.0 * wave) as u8,
                )
            }
            5 => {
                // Four Seasons: Cycles through Green, Gold, Red, Blue
                let season_phase = ((time * speed_factor * 0.2) % 4.0) as usize;
                let base = match season_phase {
                    0 => [46, 213, 115], // Spring Green
                    1 => [255, 177, 66], // Summer Gold
                    2 => [255, 71, 87],  // Autumn Red
                    _ => [30, 144, 255], // Winter Blue
                };
                Color32::from_rgb(
                    (base[0] as f32 * bright_mult) as u8,
                    (base[1] as f32 * bright_mult) as u8,
                    (base[2] as f32 * bright_mult) as u8,
                )
            }
            6 => {
                // Follow / Reactive
                let wave = ((time * speed_factor * 2.0).sin().max(0.0) as f32) * bright_mult;
                Color32::from_rgb(
                    (255.0 * wave) as u8,
                    (255.0 * wave) as u8,
                    (255.0 * wave) as u8,
                )
            }
            _ => {
                // LED Off
                Color32::from_rgb(35, 42, 55)
            }
        };

        if cur_mode != 7 {
            ui.ctx().request_repaint();
        }

        let avail_w = ui.available_width();
        let col1_w = (avail_w * 0.36).clamp(240.0, 320.0);
        let col2_w = (avail_w - col1_w - 20.0).max(340.0);

        ui.horizontal(|ui| {
            // Left Column: Live Illumination Silhouette & Stage Colors
            ui.vertical(|ui| {
                ui.set_width(col1_w);

                ui.group(|ui| {
                    ui.heading("Live Illumination Simulator");
                    ui.label(
                        RichText::new("Real-time preview of active hardware LED effect:")
                            .small()
                            .color(pal.text_secondary),
                    );
                    ui.add_space(8.0);

                    let sim_h = 190.0_f32;
                    let (sim_rect, _) = ui.allocate_exact_size(
                        Vec2::new(col1_w - 20.0, sim_h),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(sim_rect);

                    // Chassis Dark Card
                    painter.rect(
                        sim_rect,
                        8.0,
                        Color32::from_rgb(14, 18, 25),
                        Stroke::new(1.0_f32, Color32::from_rgb(35, 45, 65)),
                    );

                    let c = sim_rect.center();
                    let mw = 70.0_f32;
                    let mh = 120.0_f32;
                    let mouse_body = Rect::from_center_size(c, Vec2::new(mw, mh));

                    // Outer mouse body silhouette
                    painter.rect(
                        mouse_body,
                        28.0,
                        Color32::from_rgb(22, 28, 38),
                        Stroke::new(2.0_f32, Color32::from_rgb(50, 65, 88)),
                    );

                    // Bottom Rim Underglow Strip (Primary RGB Zone)
                    let glow_rect = Rect::from_min_max(
                        Pos2::new(c.x - mw * 0.42, c.y + mh * 0.36),
                        Pos2::new(c.x + mw * 0.42, c.y + mh * 0.46),
                    );
                    painter.rect(
                        glow_rect,
                        6.0,
                        preview_rgb,
                        Stroke::new(1.5_f32, preview_rgb),
                    );

                    // Underglow diffused halo
                    painter.circle_stroke(
                        Pos2::new(c.x, c.y + mh * 0.42),
                        mw * 0.55,
                        Stroke::new(
                            3.0_f32,
                            Color32::from_rgba_premultiplied(
                                preview_rgb.r() / 2,
                                preview_rgb.g() / 2,
                                preview_rgb.b() / 2,
                                70,
                            ),
                        ),
                    );

                    // Scroll Wheel LED Ring
                    let wheel_rect = Rect::from_center_size(
                        Pos2::new(c.x, c.y - mh * 0.28),
                        Vec2::new(12.0, 22.0),
                    );
                    painter.rect(
                        wheel_rect,
                        4.0,
                        Color32::from_rgb(10, 14, 20),
                        Stroke::new(1.5_f32, preview_rgb),
                    );

                    // Center Honeycomb / E-DRA Logo Glow Accent
                    painter.circle_filled(
                        Pos2::new(c.x, c.y + mh * 0.08),
                        7.0,
                        Color32::from_rgba_premultiplied(
                            preview_rgb.r(),
                            preview_rgb.g(),
                            preview_rgb.b(),
                            180,
                        ),
                    );

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "Current Mode: {} | Speed: {} | Bright: {}",
                            mode_name(cur_mode),
                            cur_speed,
                            if cur_brightness_half { "50%" } else { "100%" }
                        ))
                        .monospace()
                        .size(10.0)
                        .color(Color32::from_rgb(140, 170, 210)),
                    );
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("DPI Stage RGB Colors");
                    ui.label(
                        RichText::new("Colors active during CPI Breathing mode:")
                            .small()
                            .color(pal.text_secondary),
                    );
                    ui.add_space(6.0);

                    for i in 0..6 {
                        let c = self.profile.dpi_stages[i].color.clone();
                        let mut rgb = [c.r, c.g, c.b];
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Stage {}", i + 1)).strong());
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                self.profile.set_stage_color(
                                    i,
                                    crate::profile::RgbColor {
                                        r: rgb[0],
                                        g: rgb[1],
                                        b: rgb[2],
                                    },
                                );
                                self.sync_led_and_forward(&format!("Stage {} Color", i + 1));
                            }
                            ui.label(
                                RichText::new(format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]))
                                    .monospace()
                                    .size(10.0)
                                    .color(pal.text_secondary),
                            );
                        });
                    }
                });
            });

            // Right Column: 8 Mode Cards, Speed Slider, Brightness Pills, Manual Sync
            ui.vertical(|ui| {
                ui.set_width(col2_w);

                ui.group(|ui| {
                    ui.heading("Hardware Illumination Modes (Register 0x00A5 / 0x03F8)");
                    ui.add_space(4.0);

                    let modes: [(u8, &str, &str); 8] = [
                        (0, "CPI Breathing", "Breathing pulse matching active DPI stage color"),
                        (1, "Multicolor Breathing", "Continuous breathing with automatic color shift"),
                        (2, "Rainbow / Steady", "Vibrant fixed multi-spectrum illumination"),
                        (3, "Floe Light (Flow)", "Dynamic streaming light wave across chassis rim"),
                        (4, "Waltz", "Rhythmic dual-step color dancing wave"),
                        (5, "Four Seasons", "Continuous seasonal transition (Green/Gold/Red/Blue)"),
                        (6, "Follow / Reactive", "Illumination reacts dynamically to mouse clicks"),
                        (7, "LED OFF", "Disable all lighting (ultra stealth mode)"),
                    ];

                    let mut selected_mode = None;

                    // 2-Column Mode Card Grid
                    egui::Grid::new("mode_card_grid")
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            for (idx, (m_val, title, desc)) in modes.iter().enumerate() {
                                let is_active = cur_mode == *m_val;
                                let card_w = ((col2_w - 30.0) / 2.0).max(150.0);

                                let (bg_col, stroke_col) = if is_active {
                                    (Color32::from_rgb(18, 38, 55), Color32::from_rgb(0, 242, 254))
                                } else {
                                    (Color32::from_rgb(18, 22, 30), Color32::from_rgb(38, 48, 68))
                                };

                                let (card_rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(card_w, 46.0),
                                    egui::Sense::click(),
                                );
                                let card_painter = ui.painter_at(card_rect);

                                if resp.clicked() {
                                    selected_mode = Some(*m_val);
                                }

                                card_painter.rect(
                                    card_rect,
                                    6.0,
                                    bg_col,
                                    Stroke::new(if is_active { 1.5_f32 } else { 1.0_f32 }, stroke_col),
                                );

                                // Active indicator icon/dot
                                card_painter.text(
                                    Pos2::new(card_rect.min.x + 12.0, card_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    if is_active { "[*]" } else { "[ ]" },
                                    egui::FontId::monospace(11.0),
                                    if is_active {
                                        Color32::from_rgb(0, 242, 254)
                                    } else {
                                        Color32::from_rgb(100, 120, 150)
                                    },
                                );

                                card_painter.text(
                                    Pos2::new(card_rect.min.x + 36.0, card_rect.min.y + 13.0),
                                    egui::Align2::LEFT_CENTER,
                                    *title,
                                    egui::FontId::proportional(11.5),
                                    if is_active {
                                        Color32::from_rgb(0, 242, 254)
                                    } else {
                                        Color32::from_rgb(210, 225, 245)
                                    },
                                );

                                card_painter.text(
                                    Pos2::new(card_rect.min.x + 36.0, card_rect.min.y + 30.0),
                                    egui::Align2::LEFT_CENTER,
                                    *desc,
                                    egui::FontId::proportional(9.0),
                                    Color32::from_rgb(120, 140, 170),
                                );

                                if (idx + 1) % 2 == 0 {
                                    ui.end_row();
                                }
                            }
                        });

                    if let Some(m) = selected_mode {
                        self.profile.set_led_mode(m);
                        self.sync_led_and_forward(&format!("Mode {} ({})", m, mode_name(m)));
                    }
                });

                ui.add_space(8.0);

                // Speed & Brightness Group
                ui.group(|ui| {
                    ui.heading("Illumination Controls");

                    // Speed Slider
                    ui.horizontal(|ui| {
                        ui.label("Effect Speed:");
                        let mut s = cur_speed;
                        let resp = ui.add(egui::Slider::new(&mut s, 1..=8).show_value(true));
                        if resp.changed() {
                            self.profile.set_led_speed(s);
                        }
                        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                            self.sync_led_and_forward(&format!("Speed Step {}", s));
                        }
                        ui.label(
                            RichText::new(format!("(~{} ms cycle)", 7000 - (s as u32 * 750)))
                                .small()
                                .color(pal.text_secondary),
                        );
                    });

                    ui.add_space(6.0);

                    // Brightness Pills
                    ui.horizontal(|ui| {
                        ui.label("Brightness:");
                        let is_full = !cur_brightness_half;

                        let full_btn = egui::Button::new(
                            RichText::new("100% Full Brightness")
                                .size(11.0)
                                .color(if is_full { Color32::BLACK } else { Color32::from_rgb(180, 200, 230) }),
                        )
                        .fill(if is_full { Color32::from_rgb(0, 180, 230) } else { Color32::from_rgb(20, 26, 38) })
                        .stroke(Stroke::new(1.0_f32, if is_full { Color32::from_rgb(0, 242, 254) } else { Color32::from_rgb(45, 60, 85) }))
                        .rounding(4.0);

                        if ui.add(full_btn).clicked() && !is_full {
                            self.profile.set_led_brightness(false);
                            self.sync_led_and_forward("Brightness 100% (0x84)");
                        }

                        let half_btn = egui::Button::new(
                            RichText::new("50% Half Brightness")
                                .size(11.0)
                                .color(if !is_full { Color32::BLACK } else { Color32::from_rgb(180, 200, 230) }),
                        )
                        .fill(if !is_full { Color32::from_rgb(0, 180, 230) } else { Color32::from_rgb(20, 26, 38) })
                        .stroke(Stroke::new(1.0_f32, if !is_full { Color32::from_rgb(0, 242, 254) } else { Color32::from_rgb(45, 60, 85) }))
                        .rounding(4.0);

                        if ui.add(half_btn).clicked() && is_full {
                            self.profile.set_led_brightness(true);
                            self.sync_led_and_forward("Brightness 50% (0x44)");
                        }
                    });
                });

                ui.add_space(8.0);
                if ui
                    .button("Force Hardware Resync (Opcode 0x40 0x05 & 0x07)")
                    .on_hover_text(
                        "Sends all LED registers and MCULED sub-controller commands (0x40 0x01 0x07) to the mouse.",
                    )
                    .clicked()
                {
                    self.sync_led_and_forward("Manual Sync");
                }
            });
        });
    }
}

fn mode_name(mode: u8) -> &'static str {
    match mode {
        0 => "CPI Breathing",
        1 => "Multicolor Breathing",
        2 => "Rainbow / Steady",
        3 => "Floe Light",
        4 => "Waltz",
        5 => "Four Seasons",
        6 => "Follow / Reactive",
        _ => "LED OFF",
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).clamp(0.0, 255.0) as u8,
    ]
}
