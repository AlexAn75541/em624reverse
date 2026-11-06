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

        // 8-12px padding from both top header and bottom 2nd-floor status bar
        let pad_y = 10.0;
        ui.add_space(pad_y);

        // Proportional column layout: Left button list takes 330..420px, Right mouse map takes remainder
        let col1_w = (avail_w * 0.38).clamp(320.0, 420.0);
        let col2_w = (avail_w - col1_w - 24.0).max(280.0);

        ui.horizontal(|ui| {
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

            ui.add_space(16.0);

            // Right Column: Mouse Visual Map
            ui.vertical(|ui| {
                ui.set_width(col2_w);
                self.render_mouse_photo(ui, is_gaming, avail_h - pad_y);
            });
        });
    }

    pub fn render_mouse_photo(&mut self, ui: &mut egui::Ui, is_gaming: bool, avail_h: f32) {
        let pal = self.palette();
        let avail_w = ui.available_width();

        // 8-12px padding clearance from top and bottom bar
        let max_card_h = (avail_h - 16.0).clamp(240.0, 680.0);
        let max_card_w = (avail_w - 12.0).max(240.0);

        // Aspect ratio of mouse body in damouse.png (296 x 522 pixels)
        let mouse_aspect = 296.0 / 522.0; // 0.56705

        // Space inside card: margin for side badges and height padding
        let inner_avail_w = (max_card_w - 100.0).max(120.0);
        let inner_avail_h = (max_card_h - 40.0).max(180.0);

        let (base_w, base_h) = if inner_avail_w / inner_avail_h > mouse_aspect {
            (inner_avail_h * mouse_aspect, inner_avail_h)
        } else {
            (inner_avail_w, inner_avail_w / mouse_aspect)
        };

        // Scale down mouse illustration to ~80% of capacity
        let img_w = base_w * 0.80;
        let img_h = base_h * 0.80;

        let scale = (img_h / 380.0).clamp(0.65, 1.25);
        let top_pad = 36.0 * scale;
        let bottom_pad = 20.0 * scale;
        let card_w = (img_w + 110.0 * scale).min(max_card_w);
        let card_h = (img_h + top_pad + bottom_pad).min(max_card_h);

        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(card_w, card_h), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Background chassis card
        painter.rect(
            rect,
            8.0,
            pal.card_bg,
            Stroke::new(1.0_f32, Color32::from_rgb(40, 50, 68)),
        );

        // Mouse texture positioned with top margin so badge (3) is fully visible
        let img_center = Pos2::new(rect.center().x, rect.min.y + top_pad + img_h * 0.5);
        let img_rect = Rect::from_center_size(img_center, Vec2::new(img_w, img_h));

        // Paint Mouse Texture with cropped UV to remove dead transparent borders
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

        // Exact subpixel switch coordinates on EM624 chassis
        let pt_left = Pos2::new(img_min.x + 0.297 * w, img_min.y + 0.201 * h);
        let pt_right = Pos2::new(img_min.x + 0.703 * w, img_min.y + 0.201 * h);
        let pt_wheel = Pos2::new(img_min.x + 0.500 * w, img_min.y + 0.188 * h);
        let pt_fwd = Pos2::new(img_min.x + 0.095 * w, img_min.y + 0.431 * h);
        let pt_back = Pos2::new(img_min.x + 0.108 * w, img_min.y + 0.527 * h);
        let pt_dpi_plus = Pos2::new(img_min.x + 0.500 * w, img_min.y + 0.385 * h);
        let pt_dpi_minus = Pos2::new(img_min.x + 0.500 * w, img_min.y + 0.455 * h);

        // Callout badges positioned clearly with leader lines
        let badge_left = Pos2::new(pt_left.x - 28.0 * scale, pt_left.y - 20.0 * scale);
        let badge_right = Pos2::new(pt_right.x + 28.0 * scale, pt_right.y - 20.0 * scale);
        let badge_wheel = Pos2::new(pt_wheel.x, img_min.y - 14.0 * scale);
        let badge_fwd = Pos2::new(img_min.x - 22.0 * scale, pt_fwd.y);
        let badge_back = Pos2::new(img_min.x - 22.0 * scale, pt_back.y);
        let badge_dpi_plus = Pos2::new(pt_dpi_plus.x + 36.0 * scale, pt_dpi_plus.y - 8.0 * scale);
        let badge_dpi_minus =
            Pos2::new(pt_dpi_minus.x + 36.0 * scale, pt_dpi_minus.y + 8.0 * scale);

        let anchors: [(u8, usize, Pos2, Pos2); 7] = [
            (1, 1, badge_left, pt_left),
            (2, 4, badge_right, pt_right),
            (3, 7, badge_wheel, pt_wheel),
            (4, 2, badge_fwd, pt_fwd),
            (5, 5, badge_back, pt_back),
            (6, 9, badge_dpi_plus, pt_dpi_plus),
            (7, 8, badge_dpi_minus, pt_dpi_minus),
        ];

        let mut clicked_slot = None;

        for (btn_num, slot_idx, badge_pos, physical_pos) in &anchors {
            let is_hovered = self.hovered_button == Some(*slot_idx);
            let is_selected = self.selected_slot == *slot_idx;

            let radius = if is_hovered || is_selected {
                14.0 * scale
            } else {
                11.0 * scale
            };

            let badge_rect = Rect::from_center_size(*badge_pos, Vec2::splat(radius * 2.0));
            let switch_rect = Rect::from_center_size(*physical_pos, Vec2::splat(18.0 * scale));

            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            let is_pointer_over =
                pointer_pos.is_some_and(|p| badge_rect.contains(p) || switch_rect.contains(p));

            if is_pointer_over {
                self.hovered_button = Some(*slot_idx);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                if ui.input(|i| i.pointer.primary_clicked()) {
                    clicked_slot = Some(*slot_idx);
                }

                let binding_text = if is_gaming {
                    &self.profile.gaming_buttons[*slot_idx].description
                } else {
                    &self.profile.office_buttons[*slot_idx].description
                };
                egui::show_tooltip(
                    ui.ctx(),
                    ui.layer_id(),
                    egui::Id::new(format!("switch_tip_{}", slot_idx)),
                    |ui| {
                        ui.label(
                            egui::RichText::new(format!("Switch {}: {}", btn_num, binding_text))
                                .strong()
                                .color(Color32::from_rgb(0, 242, 254)),
                        );
                        ui.label(
                            egui::RichText::new("Hardware: Huano Blue Shell 50M Click Rated")
                                .small()
                                .color(Color32::from_rgb(140, 160, 190)),
                        );
                        ui.label(
                            egui::RichText::new("Click to configure rebind")
                                .small()
                                .color(Color32::from_rgb(46, 213, 115)),
                        );
                    },
                );
            }

            let (fill_col, text_col, stroke_col, leader_col, dot_col) = if is_selected {
                (
                    Color32::from_rgb(0, 242, 254),
                    Color32::BLACK,
                    Color32::WHITE,
                    Color32::from_rgb(0, 242, 254),
                    Color32::from_rgb(0, 242, 254),
                )
            } else if is_hovered {
                (
                    Color32::from_rgb(0, 180, 230),
                    Color32::BLACK,
                    Color32::WHITE,
                    Color32::from_rgb(0, 180, 230),
                    Color32::from_rgb(0, 242, 254),
                )
            } else {
                (
                    Color32::from_rgb(20, 25, 35),
                    Color32::from_rgb(180, 200, 230),
                    Color32::from_rgb(60, 80, 110),
                    Color32::from_rgba_premultiplied(80, 100, 130, 160),
                    Color32::from_rgb(90, 110, 140),
                )
            };

            // Draw leader line from badge to physical switch
            painter.line_segment(
                [*badge_pos, *physical_pos],
                Stroke::new(1.2 * scale, leader_col),
            );
            painter.circle_filled(
                *physical_pos,
                (if is_selected || is_hovered { 4.5 } else { 3.0 }) * scale,
                dot_col,
            );

            // Glow ring on selection
            if is_selected {
                painter.circle_stroke(
                    *badge_pos,
                    radius + 4.0 * scale,
                    Stroke::new(2.0_f32, Color32::from_rgb(0, 180, 230)),
                );
            }

            painter.circle_filled(*badge_pos, radius, fill_col);
            painter.circle_stroke(*badge_pos, radius, Stroke::new(1.5_f32, stroke_col));

            painter.text(
                *badge_pos,
                egui::Align2::CENTER_CENTER,
                btn_num.to_string(),
                egui::FontId::proportional(12.0 * scale),
                text_col,
            );
        }

        if let Some(slot) = clicked_slot {
            self.selected_slot = slot;
            self.init_rebind_modal(slot_idx_to_button_num(slot), is_gaming);
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
