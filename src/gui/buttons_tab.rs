use super::Em624App;
use crate::i18n::{Language, Strings};
use crate::profile::{format_hid_key_name, format_modifiers};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

impl Em624App {
    pub fn render_button_mode_tab(&mut self, ui: &mut egui::Ui, is_gaming: bool) {
        let pal = self.palette();
        let lang = self.config.language;
        let avail_w = ui.available_width();
        let avail_h = ui.available_height();

        // Responsive proportional column widths based on available width
        let col1_w = (avail_w * 0.36).clamp(330.0, 460.0);
        let col2_w = (avail_w * 0.38).clamp(330.0, 480.0);
        let total_content_w = col1_w + col2_w + 32.0;

        let pad_x = ((avail_w - total_content_w) * 0.5).max(0.0);
        let pad_y = ((avail_h - 520.0) * 0.40).clamp(8.0, 80.0);

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
                    self.profile.gaming_buttons.clone()
                } else {
                    self.profile.office_buttons.clone()
                };

                self.hovered_button = None;

                let display_order: [(usize, &'static str); 7] = match lang {
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
                        (1, "[1] Chuột Trái"),
                        (4, "[2] Chuột Phải"),
                        (7, "[3] Con Lăn (Giữa)"),
                        (2, "[4] Tiến (Hông 1)"),
                        (5, "[5] Lùi (Hông 2)"),
                        (9, "[6] DPI + (Nút trên 1)"),
                        (8, "[7] DPI - (Nút trên 2)"),
                    ],
                };

                for (slot_idx, label) in display_order {
                    let desc = if let Some(b) = buttons.iter().find(|b| b.key_number == slot_idx as u8) {
                        format_binding_desc(&[b.key_type, b.param1, b.param2, b.param3])
                    } else {
                        "Disabled".to_string()
                    };

                    let is_selected = self.selected_slot == slot_idx;
                    let is_being_edited = self.editing_slot == Some((slot_idx, is_gaming));

                    let bg_color = if is_being_edited {
                        Color32::from_rgb(0, 50, 70)
                    } else if is_selected {
                        Color32::from_rgb(25, 35, 50)
                    } else {
                        pal.card_bg
                    };

                    let border_col = if is_being_edited {
                        pal.accent
                    } else if is_selected {
                        Color32::from_rgb(0, 180, 230)
                    } else {
                        pal.card_border
                    };

                    let frame = egui::Frame::none()
                        .fill(bg_color)
                        .stroke(Stroke::new(1.5_f32, border_col))
                        .rounding(6.0)
                        .inner_margin(egui::Margin::symmetric(10.0, 7.0));

                    let response = frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let label_col = if is_selected || is_being_edited {
                                pal.accent
                            } else {
                                pal.text_primary
                            };
                            ui.label(egui::RichText::new(label).strong().color(label_col));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let desc_col = if is_selected {
                                    Color32::from_rgb(220, 240, 255)
                                } else {
                                    pal.text_secondary
                                };
                                ui.label(egui::RichText::new(&desc).small().color(desc_col));
                            });
                        });
                    });

                    let interact_rect = response.response.rect;
                    let is_hovered = ui.rect_contains_pointer(interact_rect);

                    if is_hovered {
                        self.hovered_button = Some(slot_idx);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    if response.response.clicked() {
                        self.selected_slot = slot_idx;
                        self.init_rebind_modal(slot_idx_to_button_num(slot_idx), is_gaming);
                    }

                    ui.add_space(4.0);
                }

                ui.add_space(8.0);
                let sel_button_num = slot_idx_to_button_num(self.selected_slot);
                let change_btn_text = Strings::change_key_bind_btn(lang, sel_button_num);

                if ui
                    .add_sized(
                        [col1_w, 36.0],
                        egui::Button::new(
                            egui::RichText::new(change_btn_text)
                                .strong()
                                .color(Color32::BLACK),
                        )
                        .fill(pal.accent)
                        .rounding(6.0),
                    )
                    .on_hover_text(
                        "Click to remap this physical mouse button using standard, media, macro, or keyboard hotkeys.",
                    )
                    .clicked()
                {
                    self.init_rebind_modal(sel_button_num, is_gaming);
                }
            });

            ui.add_space(24.0);

            // Right Column: Mouse Visual Map
            ui.vertical(|ui| {
                ui.set_width(col2_w);
                self.render_mouse_photo(ui, mouse_scale);
            });
        });
    }

    pub fn render_mouse_photo(&mut self, ui: &mut egui::Ui, scale: f32) {
        let pal = self.palette();
        let target_size = Vec2::new(320.0 * scale, 480.0 * scale);

        let (rect, response) = ui.allocate_exact_size(target_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Background chassis card
        painter.rect(
            rect,
            8.0,
            pal.card_bg,
            Stroke::new(1.0_f32, Color32::from_rgb(40, 50, 68)),
        );

        // Paint Mouse Texture
        if let Some(ref tex) = self.mouse_texture {
            let img_margin = 12.0 * scale;
            let img_rect = rect.shrink(img_margin);
            painter.image(
                tex.id(),
                img_rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        // Physical switch anchor locations (calibrated to EM624 chassis)
        let cx = rect.center().x;
        let cy = rect.center().y;

        let anchors: [(u8, usize, Pos2); 7] = [
            (1, 1, Pos2::new(cx - 48.0 * scale, cy - 120.0 * scale)),
            (2, 4, Pos2::new(cx + 48.0 * scale, cy - 120.0 * scale)),
            (3, 7, Pos2::new(cx, cy - 110.0 * scale)),
            (4, 2, Pos2::new(cx - 86.0 * scale, cy - 15.0 * scale)),
            (5, 5, Pos2::new(cx - 86.0 * scale, cy + 30.0 * scale)),
            (6, 9, Pos2::new(cx, cy - 42.0 * scale)),
            (7, 8, Pos2::new(cx, cy - 8.0 * scale)),
        ];

        // Draw leader lines and interactive badges for hovered / selected buttons
        for (btn_num, slot_idx, pos) in &anchors {
            let is_hovered = self.hovered_button == Some(*slot_idx);
            let is_selected = self.selected_slot == *slot_idx;

            let radius = if is_hovered || is_selected {
                14.0 * scale
            } else {
                11.0 * scale
            };

            let (fill_col, text_col, stroke_col) = if is_selected {
                (
                    Color32::from_rgb(0, 242, 254),
                    Color32::BLACK,
                    Color32::WHITE,
                )
            } else if is_hovered {
                (
                    Color32::from_rgb(0, 180, 230),
                    Color32::BLACK,
                    Color32::WHITE,
                )
            } else {
                (
                    Color32::from_rgb(20, 25, 35),
                    Color32::from_rgb(180, 200, 230),
                    Color32::from_rgb(60, 80, 110),
                )
            };

            // Glow ring on selection
            if is_selected {
                painter.circle_stroke(
                    *pos,
                    radius + 4.0 * scale,
                    Stroke::new(2.0_f32, Color32::from_rgb(0, 180, 230)),
                );
            }

            let badge_rect = Rect::from_center_size(*pos, Vec2::splat(radius * 2.0));
            if response.hover_pos().is_some_and(|p| badge_rect.contains(p)) {
                self.hovered_button = Some(*slot_idx);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            painter.circle_filled(*pos, radius, fill_col);
            painter.circle_stroke(*pos, radius, Stroke::new(1.5_f32, stroke_col));

            painter.text(
                *pos,
                egui::Align2::CENTER_CENTER,
                btn_num.to_string(),
                egui::FontId::proportional(12.0 * scale),
                text_col,
            );
        }
    }
}

pub fn format_binding_desc(bytes: &[u8; 4]) -> String {
    let key_type = bytes[0];
    let p1 = bytes[1];
    let p2 = bytes[2];
    match key_type {
        0x01 => match (p1, p2) {
            (0x01, _) => "Left Click".to_string(),
            (0x02, _) => "Right Click".to_string(),
            (0x04, _) => "Middle Click".to_string(),
            (0x08, _) => "Forward (Side 1)".to_string(),
            (0x10, _) => "Backward (Side 2)".to_string(),
            (0x00, 0x01) => "Scroll Up".to_string(),
            (0x00, 0xff) => "Scroll Down".to_string(),
            (other, _) => format!("Mouse Button 0x{:02x}", other),
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
        0x03 => match (p1, p2) {
            (0xb5, 0x00) | (0x00, 0xb5) => "Media: Next Track".to_string(),
            (0xb6, 0x00) | (0x00, 0xb6) => "Media: Prev Track".to_string(),
            (0xb7, 0x00) | (0x00, 0xb7) => "Media: Stop".to_string(),
            (0xcd, 0x00) | (0x00, 0xcd) => "Media: Play/Pause".to_string(),
            (0xe2, 0x00) | (0x00, 0xe2) => "Media: Mute".to_string(),
            (0xe9, 0x00) | (0x00, 0xe9) => "Media: Vol+".to_string(),
            (0xea, 0x00) | (0x00, 0xea) => "Media: Vol-".to_string(),
            (0x83, 0x01) => "Media: Player".to_string(),
            (0x23, 0x02) | (0x00, 0x81) => "Browser: Home".to_string(),
            (0x27, 0x02) | (0x00, 0x82) => "Browser: Refresh".to_string(),
            (0x24, 0x02) | (0x00, 0x83) => "Browser: Back".to_string(),
            (0x25, 0x02) | (0x00, 0x84) => "Browser: Forward".to_string(),
            (0x8a, 0x01) | (0x00, 0x8a) => "App: Mail".to_string(),
            (0x92, 0x01) | (0x00, 0x92) => "App: Calculator".to_string(),
            (0x94, 0x01) | (0x00, 0x94) => "App: My Computer".to_string(),
            _ => format!("Consumer 0x{:02x}{:02x}", p2, p1),
        },
        0x06 => {
            if p1 == 0x01 && p2 == 0x02 {
                "Double Click (Left x2)".to_string()
            } else {
                "Fire Key (Rapid Burst)".to_string()
            }
        }
        0x07 => match p1 {
            0x80 => "LED ON/OFF Toggle".to_string(),
            0x81 => "LED Mode Cycle".to_string(),
            other => format!("ACPI 0x{:02x}", other),
        },
        0x08 => "DPI Loop (Cycle All)".to_string(),
        0x09 => match p1 {
            0x00 => "Office Mode Switch".to_string(),
            0x01 => "Gaming Mode Switch".to_string(),
            0x02 => "DPI +".to_string(),
            0x03 => "DPI -".to_string(),
            other => format!("Fn 0x{:02x}", other),
        },
        0x0a => "LED Mode Cycle".to_string(),
        0x05 => format!("Macro #{} [EXPERIMENTAL]", p2.saturating_add(1)),
        0x00 => "Disabled (None)".to_string(),
        other => format!("Type 0x{:02x}", other),
    }
}

pub fn slot_idx_to_button_num(slot_idx: usize) -> u8 {
    slot_idx as u8
}
