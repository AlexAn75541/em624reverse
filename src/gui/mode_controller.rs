use super::config::{icon_label_spaced, Icons, Tab};
use super::Em624App;
use crate::i18n::Strings;
use eframe::egui::{self, Color32, RichText, Stroke};

impl Em624App {
    /// Check whether mouse profile is currently in Gaming mode (0x01) vs Office mode (0x00)
    pub fn is_hardware_gaming_mode(&self) -> bool {
        self.profile.raw.get(0x00a4).copied().unwrap_or(0x01) == 0x01
    }

    /// Renders a top navigation tab button with an adjacent sub-button indicator to activate that mode on hardware.
    pub fn render_mode_tab_item(&mut self, ui: &mut egui::Ui, target_mode_is_gaming: bool) {
        let is_hw_active = self.is_hardware_gaming_mode() == target_mode_is_gaming;
        let is_tab_active = self.active_tab
            == if target_mode_is_gaming {
                Tab::GameMode
            } else {
                Tab::OfficeMode
            };
        let lang = self.config.language;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            // Main Tab Selectable Button
            let (icon, title) = if target_mode_is_gaming {
                (Icons::GAMEPAD, Strings::tab_game(lang))
            } else {
                (Icons::BRIEFCASE, Strings::tab_office(lang))
            };

            if ui
                .selectable_label(is_tab_active, icon_label_spaced(icon, title, 8.0))
                .clicked()
            {
                self.active_tab = if target_mode_is_gaming {
                    Tab::GameMode
                } else {
                    Tab::OfficeMode
                };
            }

            // Sub-button: Active / Inactive hardware toggle beside the title button
            let (badge_text, badge_bg, badge_fg) = if is_hw_active {
                ("[Active]", Color32::from_rgb(0, 180, 230), Color32::BLACK)
            } else {
                (
                    "[Set Active]",
                    Color32::from_rgb(30, 40, 55),
                    Color32::from_rgb(140, 170, 210),
                )
            };

            let sub_btn = egui::Button::new(
                RichText::new(badge_text)
                    .monospace()
                    .size(10.0)
                    .color(badge_fg),
            )
            .fill(badge_bg)
            .stroke(Stroke::new(
                1.0_f32,
                if is_hw_active {
                    Color32::from_rgb(0, 242, 254)
                } else {
                    Color32::from_rgb(50, 70, 95)
                },
            ))
            .rounding(4.0);

            if ui
                .add(sub_btn)
                .on_hover_text(if is_hw_active {
                    "This mode is currently active on mouse hardware (Register 0x00A4)"
                } else {
                    "Click to switch mouse hardware to this mode (non-blocking)"
                })
                .clicked()
            {
                self.switch_mode_and_forward(target_mode_is_gaming);
            }
        });
    }
}
