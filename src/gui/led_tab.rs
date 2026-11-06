use super::Em624App;
use crate::i18n::Strings;
use eframe::egui::{self, RichText};

impl Em624App {
    pub fn render_led_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;

        ui.add_space(6.0);
        ui.heading(Strings::tab_led(lang));
        ui.label(
            RichText::new(
                "Configure RGB lighting effects, breathing speed, brightness, and LED modes.",
            )
            .small()
            .color(pal.text_secondary),
        );
        ui.add_space(14.0);

        ui.group(|ui| {
            ui.heading("LED Breathing / Effect Speed");
            let mut cur_speed = self.profile.get_led_speed();
            ui.horizontal(|ui| {
                ui.label("Slow (1)");
                let resp = ui.add(egui::Slider::new(&mut cur_speed, 1..=8).show_value(true));
                if resp.changed() {
                    self.profile.set_led_speed(cur_speed);
                }
                if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                    self.sync_led_and_forward(&format!("Speed Step {}", cur_speed));
                }
                ui.label("Fast (8)");
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("LED Mode");
            let mut cur_mode = self.profile.get_led_mode();
            let mut mode_changed = false;
            ui.horizontal(|ui| {
                if ui.radio_value(&mut cur_mode, 0, "CPI Breathing").clicked() {
                    mode_changed = true;
                }
                if ui
                    .radio_value(&mut cur_mode, 1, "Colorful Breathing")
                    .clicked()
                {
                    mode_changed = true;
                }
                if ui
                    .radio_value(&mut cur_mode, 2, "Rainbow / Steady")
                    .clicked()
                {
                    mode_changed = true;
                }
                if ui.radio_value(&mut cur_mode, 3, "Floe Light").clicked() {
                    mode_changed = true;
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.radio_value(&mut cur_mode, 4, "Waltz").clicked() {
                    mode_changed = true;
                }
                if ui.radio_value(&mut cur_mode, 5, "Four Seasons").clicked() {
                    mode_changed = true;
                }
                if ui
                    .radio_value(&mut cur_mode, 6, "Follow / Reactive")
                    .clicked()
                {
                    mode_changed = true;
                }
                if ui.radio_value(&mut cur_mode, 7, "Off").clicked() {
                    mode_changed = true;
                }
            });
            if mode_changed {
                self.profile.set_led_mode(cur_mode);
                self.sync_led_and_forward(&format!("Mode {}", cur_mode));
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.heading("LED Brightness");
            let cur_bright = self.profile.get_led_brightness();
            let mut bright_sel = cur_bright as usize;
            ui.horizontal(|ui| {
                if ui
                    .radio_value(&mut bright_sel, 0, "Full Brightness (100%)")
                    .clicked()
                {
                    self.profile.set_led_brightness(false);
                    self.sync_led_and_forward("Brightness 100% (0x84)");
                }
                if ui
                    .radio_value(&mut bright_sel, 1, "Half Brightness (50%)")
                    .clicked()
                {
                    self.profile.set_led_brightness(true);
                    self.sync_led_and_forward("Brightness 50% (0x44)");
                }
            });
        });

        ui.add_space(12.0);
        if ui
            .button("Sync LED Lighting to Hardware Now")
            .on_hover_text(
                "Immediately sends LED speed (0x0038), RGB stage colors (0x0039..0x004D), LED mode (0x00A5), brightness (0x00A7), and MCULED sub-controller commands (0x40 0x01 0x07) to the mouse.",
            )
            .clicked()
        {
            self.sync_led_and_forward("Manual Sync");
        }
    }
}
