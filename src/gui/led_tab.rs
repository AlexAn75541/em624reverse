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
                "Visual LED Studio: Real-time lighting simulator, 7 hardware illumination modes, and stage color tuning.",
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
            _ => {
                // LED Off (Mode 6)
                Color32::from_rgb(35, 42, 55)
            }
        };

        if cur_mode != 6 {
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

                    let sim_h = 240.0_f32;
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

                    // Mouse aspect ratio from authentic chassis photo (296 x 522 px)
                    let mouse_aspect = 296.0 / 522.0; // 0.56705
                    let img_h = (sim_rect.height() - 48.0).max(120.0);
                    let img_w = img_h * mouse_aspect;
                    let img_center = Pos2::new(sim_rect.center().x, sim_rect.min.y + 16.0 + img_h * 0.5);
                    let img_rect = Rect::from_center_size(img_center, Vec2::new(img_w, img_h));

                    // Paint actual mouse photo reference
                    if let Some(ref tex) = self.mouse_texture {
                        let u_min = 148.0 / 600.0;
                        let u_max = 444.0 / 600.0;
                        let v_min = 40.0 / 600.0;
                        let v_max = 562.0 / 600.0;
                        painter.image(
                            tex.id(),
                            img_rect,
                            Rect::from_min_max(Pos2::new(u_min, v_min), Pos2::new(u_max, v_max)),
                            Color32::WHITE,
                        );
                    }

                    let img_min = img_rect.min;
                    let w = img_rect.width();
                    let h = img_rect.height();
                    let cx = img_min.x + 0.500 * w;

                    // 1. Lower E-DRA Palm Logo (Covering authentic logo at y_frac 0.69..0.81)
                    let logo_cy = img_min.y + 0.751 * h;
                    let r = w * 0.16;
                    let crest_pts = vec![
                        Pos2::new(cx, logo_cy - r * 0.82),
                        Pos2::new(cx + r * 0.50, logo_cy - r * 0.48),
                        Pos2::new(cx + r * 0.85, logo_cy - r * 0.12),
                        Pos2::new(cx + r * 0.55, logo_cy + r * 0.42),
                        Pos2::new(cx, logo_cy + r * 0.82),
                        Pos2::new(cx - r * 0.55, logo_cy + r * 0.42),
                        Pos2::new(cx - r * 0.85, logo_cy - r * 0.12),
                        Pos2::new(cx - r * 0.50, logo_cy - r * 0.48),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        crest_pts,
                        Color32::from_rgba_premultiplied(
                            preview_rgb.r(),
                            preview_rgb.g(),
                            preview_rgb.b(),
                            160,
                        ),
                        Stroke::new(1.6_f32, preview_rgb),
                    ));

                    // Inner dragon diamond core
                    let core_pts = vec![
                        Pos2::new(cx, logo_cy - r * 0.28),
                        Pos2::new(cx + r * 0.25, logo_cy + r * 0.08),
                        Pos2::new(cx, logo_cy + r * 0.42),
                        Pos2::new(cx - r * 0.25, logo_cy + r * 0.08),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        core_pts,
                        Color32::from_rgba_premultiplied(
                            preview_rgb.r(),
                            preview_rgb.g(),
                            preview_rgb.b(),
                            230,
                        ),
                        Stroke::new(1.0_f32, Color32::WHITE),
                    ));

                    // 2. Side LED Light Strips & Rear Skirt (Contoured along chassis flanks)
                    let left_side_pts = [
                        Pos2::new(img_min.x + 0.075 * w, img_min.y + 0.420 * h),
                        Pos2::new(img_min.x + 0.065 * w, img_min.y + 0.500 * h),
                        Pos2::new(img_min.x + 0.080 * w, img_min.y + 0.600 * h),
                        Pos2::new(img_min.x + 0.070 * w, img_min.y + 0.700 * h),
                        Pos2::new(img_min.x + 0.080 * w, img_min.y + 0.780 * h),
                        Pos2::new(img_min.x + 0.120 * w, img_min.y + 0.850 * h),
                        Pos2::new(img_min.x + 0.200 * w, img_min.y + 0.890 * h),
                    ];

                    let right_side_pts = [
                        Pos2::new(img_min.x + 0.925 * w, img_min.y + 0.420 * h),
                        Pos2::new(img_min.x + 0.935 * w, img_min.y + 0.500 * h),
                        Pos2::new(img_min.x + 0.920 * w, img_min.y + 0.600 * h),
                        Pos2::new(img_min.x + 0.930 * w, img_min.y + 0.700 * h),
                        Pos2::new(img_min.x + 0.920 * w, img_min.y + 0.780 * h),
                        Pos2::new(img_min.x + 0.880 * w, img_min.y + 0.850 * h),
                        Pos2::new(img_min.x + 0.800 * w, img_min.y + 0.890 * h),
                    ];

                    let rear_skirt_pts = [
                        Pos2::new(img_min.x + 0.200 * w, img_min.y + 0.890 * h),
                        Pos2::new(img_min.x + 0.320 * w, img_min.y + 0.930 * h),
                        Pos2::new(img_min.x + 0.500 * w, img_min.y + 0.945 * h),
                        Pos2::new(img_min.x + 0.680 * w, img_min.y + 0.930 * h),
                        Pos2::new(img_min.x + 0.800 * w, img_min.y + 0.890 * h),
                    ];

                    // Outer ambient diffused glow
                    let halo_col = Color32::from_rgba_premultiplied(
                        preview_rgb.r() / 3,
                        preview_rgb.g() / 3,
                        preview_rgb.b() / 3,
                        40,
                    );
                    let mid_col = Color32::from_rgba_premultiplied(
                        preview_rgb.r() / 2,
                        preview_rgb.g() / 2,
                        preview_rgb.b() / 2,
                        95,
                    );

                    for strip in [&left_side_pts[..], &right_side_pts[..], &rear_skirt_pts[..]] {
                        for win in strip.windows(2) {
                            painter.line_segment([win[0], win[1]], Stroke::new(9.0_f32, halo_col));
                        }
                        for win in strip.windows(2) {
                            painter.line_segment([win[0], win[1]], Stroke::new(4.5_f32, mid_col));
                        }
                        for win in strip.windows(2) {
                            painter.line_segment([win[0], win[1]], Stroke::new(2.0_f32, preview_rgb));
                        }
                    }

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

            // Right Column: 7 Mode Cards, Speed Slider, Brightness Pills, Manual Sync
            ui.vertical(|ui| {
                ui.set_width(col2_w);

                ui.group(|ui| {
                    ui.heading("Hardware Illumination Modes (Register 0x00A5 / 0x03F8)");
                    ui.add_space(4.0);

                    let modes: [(u8, &str, &str); 7] = [
                        (0, "CPI Breathing", "Breathing pulse matching active DPI stage color"),
                        (1, "Multicolor Breathing", "Continuous breathing with automatic color shift"),
                        (2, "Rainbow / Steady", "Vibrant fixed multi-spectrum illumination"),
                        (3, "Floe Light (Flow)", "Dynamic streaming light wave across chassis rim"),
                        (4, "Waltz", "Rhythmic dual-step color dancing wave"),
                        (5, "Four Seasons", "Continuous seasonal transition (Green/Gold/Red/Blue)"),
                        (6, "LED OFF", "Disable all lighting (ultra stealth mode)"),
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
