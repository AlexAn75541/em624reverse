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
                if ui
                    .add(egui::Slider::new(&mut cur_speed, 1..=8).show_value(true))
                    .changed()
                {
                    self.profile.set_led_speed(cur_speed);
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
                }
                if ui
                    .radio_value(&mut bright_sel, 1, "Half Brightness (50%)")
                    .clicked()
                {
                    self.profile.set_led_brightness(true);
                }
            });
        });
    }
}
