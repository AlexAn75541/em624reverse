use super::Em624App;
use crate::i18n::Strings;
use crate::macro_engine::{MacroAction, MacroEntry, MouseButton};
use crate::profile::format_hid_key_name;
use eframe::egui::{self, RichText, Vec2};

impl Em624App {
    pub fn render_macro_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.palette();
        let lang = self.config.language;

        ui.add_space(6.0);
        ui.heading(Strings::tab_macro(lang));
        ui.label(
            RichText::new(
                "Create, edit, and assign keystroke & click sequences to mouse hardware macros.",
            )
            .small()
            .color(pal.text_secondary),
        );
        ui.add_space(10.0);

        let total_w = ui.available_width();
        let total_h = ui.available_height().max(420.0);

        let col1_w = (total_w * 0.28).clamp(180.0, 260.0);
        let col2_w = (total_w * 0.44).max(280.0);
        let col3_w = (total_w - col1_w - col2_w - 28.0).max(180.0);

        ui.horizontal(|ui| {
            // Column 1: Macro Library
            ui.allocate_ui_with_layout(
                Vec2::new(col1_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(RichText::new("Macro Library").strong());
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(total_h - 100.0)
                            .show(ui, |ui| {
                                for (i, m) in self.macro_bank.macros.iter().enumerate() {
                                    let selected = self.selected_macro == Some(i);
                                    if ui.selectable_label(selected, &m.name).clicked() {
                                        self.selected_macro = Some(i);
                                    }
                                }
                            });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("New Macro").clicked() {
                            let new_name = format!("Macro {}", self.macro_bank.macros.len() + 1);
                            self.macro_bank.macros.push(MacroEntry::new(&new_name));
                            self.selected_macro = Some(self.macro_bank.macros.len() - 1);
                        }
                        if ui.button("Delete").clicked() {
                            if let Some(idx) = self.selected_macro {
                                if idx < self.macro_bank.macros.len() {
                                    self.macro_bank.macros.remove(idx);
                                    self.selected_macro = if self.macro_bank.macros.is_empty() {
                                        None
                                    } else {
                                        Some(idx.saturating_sub(1))
                                    };
                                }
                            }
                        }
                    });
                },
            );

            ui.separator();

            // Column 2: Macro Content / Sequence Timeline
            ui.allocate_ui_with_layout(
                Vec2::new(col2_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(RichText::new("Macro Sequence Timeline").strong());
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(total_h - 100.0)
                            .show(ui, |ui| {
                                if let Some(idx) = self.selected_macro {
                                    if let Some(macro_item) = self.macro_bank.macros.get_mut(idx) {
                                        ui.horizontal(|ui| {
                                            ui.label("Name:");
                                            ui.text_edit_singleline(&mut macro_item.name);
                                        });
                                        ui.separator();

                                        if macro_item.actions.is_empty() {
                                            ui.label(
                                                RichText::new("(Empty sequence — click '+ Add' below to create steps)")
                                                    .small()
                                                    .color(pal.text_secondary),
                                            );
                                        } else {
                                            let mut to_remove = None;
                                            for (step_idx, action) in macro_item.actions.iter_mut().enumerate() {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(format!("{}.", step_idx + 1))
                                                            .monospace()
                                                            .color(pal.accent),
                                                    );
                                                    match action {
                                                        MacroAction::MouseDown(btn) => {
                                                            ui.label(format!("Mouse Down [{:?}]", btn));
                                                        }
                                                        MacroAction::MouseUp(btn) => {
                                                            ui.label(format!("Mouse Up [{:?}]", btn));
                                                        }
                                                        MacroAction::KeyDown(k) => {
                                                            let kname = format_hid_key_name(*k);
                                                            ui.label(format!("Key Down [{}]", kname));
                                                        }
                                                        MacroAction::KeyUp(k) => {
                                                            let kname = format_hid_key_name(*k);
                                                            ui.label(format!("Key Up [{}]", kname));
                                                        }
                                                        MacroAction::ModifierDown(m) => {
                                                            ui.label(format!("Modifier Down [{:?}]", m));
                                                        }
                                                        MacroAction::ModifierUp(m) => {
                                                            ui.label(format!("Modifier Up [{:?}]", m));
                                                        }
                                                        MacroAction::Delay(ms) => {
                                                            ui.label("Delay:");
                                                            ui.add(egui::DragValue::new(ms).suffix(" ms").range(5..=2000));
                                                        }
                                                        MacroAction::Loop(c) => {
                                                            ui.label("Loop repeat:");
                                                            ui.add(egui::DragValue::new(c).range(1..=127));
                                                        }
                                                    }
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.small_button("X").clicked() {
                                                            to_remove = Some(step_idx);
                                                        }
                                                    });
                                                });
                                            }
                                            if let Some(rem_idx) = to_remove {
                                                macro_item.actions.remove(rem_idx);
                                            }
                                        }
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("(Select or create a macro from the library on the left)")
                                            .color(pal.text_secondary),
                                    );
                                }
                            });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if let Some(idx) = self.selected_macro {
                            if let Some(macro_item) = self.macro_bank.macros.get_mut(idx) {
                                if ui.button("+ Add Key").clicked() {
                                    macro_item.actions.push(MacroAction::KeyDown(0x2c)); // Spacebar
                                    macro_item.actions.push(MacroAction::Delay(30));
                                    macro_item.actions.push(MacroAction::KeyUp(0x2c));
                                }
                                if ui.button("+ Add Click").clicked() {
                                    macro_item.actions.push(MacroAction::MouseDown(MouseButton::Left));
                                    macro_item.actions.push(MacroAction::Delay(25));
                                    macro_item.actions.push(MacroAction::MouseUp(MouseButton::Left));
                                }
                                if ui.button("+ Add Delay").clicked() {
                                    macro_item.actions.push(MacroAction::Delay(50));
                                }
                            }
                        }
                    });
                },
            );

            ui.separator();

            // Column 3: Storage & Execution Options
            ui.allocate_ui_with_layout(
                Vec2::new(col3_w, total_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(RichText::new("Execution & Storage").strong());
                    ui.add_space(4.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(total_h - 100.0)
                            .show(ui, |ui| {
                                ui.label("Playback Mode:");
                                ui.radio_value(&mut self.macro_loop_mode, 0, "Play Once");
                                ui.radio_value(&mut self.macro_loop_mode, 1, "Hold Until Released");
                                ui.radio_value(&mut self.macro_loop_mode, 2, "Repeat N Times");
                                if self.macro_loop_mode == 2 {
                                    ui.horizontal(|ui| {
                                        ui.label("Times:");
                                        ui.add(egui::DragValue::new(&mut self.macro_repeat_count).range(1..=100));
                                    });
                                }

                                ui.add_space(10.0);
                                ui.label("Timing Delay:");
                                ui.radio_value(&mut self.macro_timing_mode, 0, "Default (50ms)");
                                ui.radio_value(&mut self.macro_timing_mode, 1, "Real Record Timing");
                                ui.radio_value(&mut self.macro_timing_mode, 2, "No Delay (Fastest)");

                                ui.add_space(12.0);
                                ui.separator();
                                ui.add_space(6.0);

                                if ui.button("Sync to Mouse Profile").clicked() {
                                    if let Err(e) = crate::macro_engine::pack_macros_to_profile(&self.macro_bank, &mut self.profile.raw) {
                                        self.status_message = format!("Macro pack warning: {}", e);
                                    } else {
                                        self.profile.recalculate_checksums();
                                        self.save_to_cached_profile();
                                        self.status_message = format!("Synced {} macro(s) into active profile memory.", self.macro_bank.macros.len());
                                    }
                                }

                                ui.add_space(6.0);
                                if ui.button("Save Macro Bank to Disk").clicked() {
                                    let macro_path = self.config.cache_dir().join("macros.bin");
                                    if let Err(e) = self.macro_bank.save_file(&macro_path) {
                                        self.status_message = format!("Failed to save macro bank: {}", e);
                                    } else {
                                        self.status_message = format!("Macro bank saved to disk ({} macros).", self.macro_bank.macros.len());
                                    }
                                }
                            });
                    });
                },
            );
        });
    }
}
