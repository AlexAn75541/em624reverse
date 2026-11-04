use super::buttons_tab::{format_binding_desc, slot_idx_to_button_num};
use super::Em624App;
use crate::i18n::Strings;
use crate::profile::{format_hid_key_name, MouseProfile};
use eframe::egui::{self, Color32, Vec2};

impl Em624App {
    pub fn init_rebind_modal(&mut self, btn_num: u8, is_gaming: bool) {
        let base_off = if is_gaming { 0x00f1 } else { 0x00c0 };
        let slot = MouseProfile::button_num_to_slot(btn_num);
        let off = base_off + slot * 4;
        let mut cur = [0u8; 4];
        cur.copy_from_slice(&self.profile.raw[off..off + 4]);
        self.rebind_candidate = Some(cur);

        let ktype = cur[0];
        let p1 = cur[1];
        let p2 = cur[2];

        match ktype {
            0x01 => {
                self.rebind_category = 0; // Mouse
            }
            0x02 => {
                if p1 == 0 {
                    self.rebind_category = 1; // Single key
                    self.rebind_key_code = p2;
                    self.rebind_modifiers = 0;
                    self.rebind_subtab = match p2 {
                        0x04..=0x1d => 0, // Letters
                        0x1e..=0x27 => 1, // Numbers
                        0x54..=0x63 => 2, // Numpad
                        0x3a..=0x45 => 3, // Function
                        0x49..=0x53 => 4, // Navigation
                        _ => 5,           // Symbols
                    };
                } else {
                    self.rebind_category = 2; // Shortcut
                    self.rebind_key_code = p2;
                    self.rebind_modifiers = p1;
                }
            }
            0x06 | 0x08 | 0x09 => {
                self.rebind_category = 3; // Special / DPI
            }
            0x03 => {
                match (p1, p2) {
                    (0x23, 0x02)
                    | (0x27, 0x02)
                    | (0x24, 0x02)
                    | (0x25, 0x02)
                    | (0x8a, 0x01)
                    | (0x92, 0x01)
                    | (0x94, 0x01)
                    | (0x00, 0x81)
                    | (0x00, 0x82)
                    | (0x00, 0x83)
                    | (0x00, 0x84)
                    | (0x00, 0x92)
                    | (0x00, 0x94)
                    | (0x00, 0x8a) => {
                        self.rebind_category = 5; // Productivity
                    }
                    _ => {
                        self.rebind_category = 4; // Multimedia
                    }
                }
            }
            0x07 | 0x0a => {
                self.rebind_category = 5; // Productivity (LED)
            }
            0x05 => {
                self.rebind_category = 6; // Macro
            }
            _ => {
                self.rebind_category = 0;
            }
        }
    }

    #[allow(dead_code)]
    fn apply_and_cache_binding(&mut self, slot_idx: usize, is_gaming: bool) {
        if let Some(bytes) = self.rebind_candidate {
            let base_off = if is_gaming { 0x00f1 } else { 0x00c0 };
            let off = base_off + (slot_idx - 1) * 4;
            self.profile.raw[off..off + 4].copy_from_slice(&bytes);

            // Also synchronize to complementary mode table (Office / Gaming)
            // so physical hardware buttons ALWAYS trigger the rebound action regardless of
            // internal MCU mode switch state.
            let alt_base_off = if is_gaming { 0x00c0 } else { 0x00f1 };
            let alt_off = alt_base_off + (slot_idx - 1) * 4;
            self.profile.raw[alt_off..alt_off + 4].copy_from_slice(&bytes);

            // Set active mode byte 0x00a4 (0xF1 Gaming, 0x00 Office)
            self.profile.raw[0x00a4] = if is_gaming { 0xf1 } else { 0x00 };
            self.profile.raw[0x00aa] = 0x18; // Feature report enable
            self.profile.raw[0x03f9] &= 0x7f; // Clear button lockout flag

            self.profile.apply_safe_profile();
            self.profile.refresh_stages();

            // Auto-save to cached profile
            self.save_to_cached_profile();

            let btn_num = slot_idx_to_button_num(slot_idx);
            let desc = format_binding_desc(&bytes);
            self.status_message = format!(
                "Saved Button [{}] as '{}'! Click 'Flash to Mouse' to commit to hardware.",
                btn_num, desc
            );
            self.log_debug(&format!(
                "[REMAP] Updated slot {} ({}) to bytecode {:02x?} ('{}') (synced to both mode tables & cached)",
                slot_idx,
                if is_gaming { "Gaming" } else { "Office" },
                bytes,
                desc
            ));
        }
    }

    pub fn render_rebind_modal(&mut self, ctx: &egui::Context, slot_idx: usize, is_gaming: bool) {
        let mut close_modal = false;
        let mut apply_binding = false;
        let btn_num = slot_idx_to_button_num(slot_idx);
        let mode_label = if is_gaming {
            Strings::game_mode_title(self.config.language)
        } else {
            Strings::office_mode_title(self.config.language)
        };
        let title = Strings::rebind_title(self.config.language, btn_num, mode_label);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(460.0);

                ui.label(
                    egui::RichText::new(
                        "Hardware button remapping writes to both Office (0x00C0) & Gaming (0x00F1) tables with HID+VK translation.",
                    )
                    .small()
                    .color(Color32::from_rgb(140, 180, 230)),
                );
                ui.add_space(4.0);

                // Step 1: Category Selection
                ui.label(
                    egui::RichText::new(Strings::step1_cat(self.config.language))
                        .strong()
                        .color(Color32::from_rgb(0, 242, 254)),
                );
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.rebind_category, 0, "Mouse");
                    ui.selectable_value(&mut self.rebind_category, 1, "Keyboard (Single)");
                    ui.selectable_value(&mut self.rebind_category, 2, "Shortcut (Combo)");
                    ui.selectable_value(&mut self.rebind_category, 3, "DPI / Special");
                    ui.selectable_value(&mut self.rebind_category, 4, "Media");
                    ui.selectable_value(&mut self.rebind_category, 5, "Productivity");
                    ui.selectable_value(&mut self.rebind_category, 7, "Macro [EXPERIMENTAL]");
                    ui.selectable_value(&mut self.rebind_category, 6, "Disable");
                });

                ui.separator();

                // Step 2: Pick Key
                ui.label(
                    egui::RichText::new(Strings::step2_key(self.config.language))
                        .strong()
                        .color(Color32::from_rgb(0, 242, 254)),
                );
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| match self.rebind_category {
                        0 => self.render_category_mouse(ui),
                        1 => self.render_category_single_key(ui),
                        2 => self.render_category_shortcut(ui),
                        3 => self.render_category_special(ui),
                        4 => self.render_category_media(ui),
                        5 => self.render_category_productivity(ui),
                        7 => self.render_category_macro(ui),
                        _ => {
                            self.rebind_candidate = Some([0x00, 0x00, 0x00, 0x00]);
                            ui.label("This button will be disabled (no action).");
                        }
                    });

                ui.separator();

                // Live Preview
                let preview_text = if let Some(bytes) = &self.rebind_candidate {
                    format_binding_desc(bytes)
                } else {
                    "None".to_string()
                };

                ui.horizontal(|ui| {
                    ui.label("New Assignment:");
                    ui.label(
                        egui::RichText::new(preview_text)
                            .strong()
                            .color(Color32::from_rgb(0, 242, 254)),
                    );
                });

                ui.add_space(8.0);

                // Step 3: Done Keybinding
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [160.0, 30.0],
                            egui::Button::new(
                                egui::RichText::new(Strings::done_btn(self.config.language))
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .fill(Color32::from_rgb(0, 242, 254)),
                        )
                        .clicked()
                    {
                        apply_binding = true;
                    }

                    if ui
                        .add_sized(
                            [100.0, 30.0],
                            egui::Button::new(Strings::cancel_btn(self.config.language)),
                        )
                        .clicked()
                    {
                        close_modal = true;
                    }
                });
            });

        if apply_binding {
            if let Some(binding) = self.rebind_candidate {
                let slot = MouseProfile::button_num_to_slot(slot_idx as u8);
                self.profile.set_button_binding(slot, binding);
                self.profile.refresh_decoded_buttons();
                let desc = format_binding_desc(&binding);
                self.status_message = format!(
                    "Button {} configured to '{}'. Click 'Apply Settings' to burn to mouse.",
                    btn_num, desc
                );
            }
            self.editing_slot = None;
        } else if close_modal {
            self.editing_slot = None;
        }
    }

    fn render_category_mouse(&mut self, ui: &mut egui::Ui) {
        ui.label("Standard mouse buttons:");
        ui.add_space(4.0);

        let options = [
            ("Left Click", [0x01, 0x01, 0x00, 0x00]),
            ("Right Click", [0x01, 0x02, 0x00, 0x00]),
            ("Middle Click", [0x01, 0x04, 0x00, 0x00]),
            ("Forward (Browser Side 1)", [0x01, 0x08, 0x00, 0x00]),
            ("Backward (Browser Side 2)", [0x01, 0x10, 0x00, 0x00]),
            ("Double Click (Left x2)", [0x06, 0x01, 0x02, 0x1e]),
            ("Scroll Up", [0x01, 0x00, 0x01, 0x00]),
            ("Scroll Down", [0x01, 0x00, 0xff, 0x00]),
        ];

        for (label, bytes) in options {
            let is_cur = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_cur, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_single_key(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.rebind_subtab, 0, "Letters");
            ui.selectable_value(&mut self.rebind_subtab, 1, "Numbers");
            ui.selectable_value(&mut self.rebind_subtab, 2, "Numpad");
            ui.selectable_value(&mut self.rebind_subtab, 3, "F1..F12");
            ui.selectable_value(&mut self.rebind_subtab, 4, "Navigation");
            ui.selectable_value(&mut self.rebind_subtab, 5, "Symbols");
        });
        ui.separator();

        match self.rebind_subtab {
            0 => {
                // Letters A..Z
                ui.horizontal_wrapped(|ui| {
                    for code in 0x04..=0x1d {
                        let name = format_hid_key_name(code);
                        let vk = MouseProfile::hid_usage_to_vk(code);
                        let is_sel = self
                            .rebind_candidate
                            .map(|b| b[0] == 0x02 && b[1] == 0x00 && b[2] == code)
                            .unwrap_or(false);
                        if ui
                            .add_sized([32.0, 26.0], egui::SelectableLabel::new(is_sel, name))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, vk]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            1 => {
                // Numbers 1..9, 0
                ui.horizontal_wrapped(|ui| {
                    for code in 0x1e..=0x27 {
                        let name = format_hid_key_name(code);
                        let vk = MouseProfile::hid_usage_to_vk(code);
                        let is_sel = self
                            .rebind_candidate
                            .map(|b| b[0] == 0x02 && b[1] == 0x00 && b[2] == code)
                            .unwrap_or(false);
                        if ui
                            .add_sized([36.0, 28.0], egui::SelectableLabel::new(is_sel, name))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, vk]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            2 => {
                // Full Numpad keys (17 discrete keys)
                ui.label("Full Numpad keys (dedicated scancodes):");
                ui.add_space(4.0);
                let numpad_keys = [
                    (0x59, "Num 1"),
                    (0x5a, "Num 2"),
                    (0x5b, "Num 3"),
                    (0x5c, "Num 4"),
                    (0x5d, "Num 5"),
                    (0x5e, "Num 6"),
                    (0x5f, "Num 7"),
                    (0x60, "Num 8"),
                    (0x61, "Num 9"),
                    (0x62, "Num 0"),
                    (0x57, "Num +"),
                    (0x56, "Num -"),
                    (0x55, "Num *"),
                    (0x54, "Num /"),
                    (0x58, "Num Enter"),
                    (0x63, "Num ."),
                    (0x53, "Num Lock"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in numpad_keys {
                        let vk = MouseProfile::hid_usage_to_vk(code);
                        let is_sel = self
                            .rebind_candidate
                            .map(|b| b[0] == 0x02 && b[1] == 0x00 && b[2] == code)
                            .unwrap_or(false);
                        if ui
                            .add_sized([72.0, 28.0], egui::SelectableLabel::new(is_sel, label))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, vk]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            3 => {
                // F1..F12
                ui.horizontal_wrapped(|ui| {
                    for code in 0x3a..=0x45 {
                        let name = format_hid_key_name(code);
                        let vk = MouseProfile::hid_usage_to_vk(code);
                        let is_sel = self
                            .rebind_candidate
                            .map(|b| b[0] == 0x02 && b[1] == 0x00 && b[2] == code)
                            .unwrap_or(false);
                        if ui
                            .add_sized([42.0, 28.0], egui::SelectableLabel::new(is_sel, name))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, vk]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            4 => {
                // Navigation & Editing
                let nav_keys = [
                    (0x4b, "Page Up"),
                    (0x4e, "Page Down"),
                    (0x4a, "Home"),
                    (0x4d, "End"),
                    (0x49, "Insert"),
                    (0x4c, "Delete"),
                    (0x52, "Up Arrow"),
                    (0x51, "Down Arrow"),
                    (0x50, "Left Arrow"),
                    (0x4f, "Right Arrow"),
                    (0x28, "Enter"),
                    (0x29, "Esc"),
                    (0x2c, "Space"),
                    (0x2b, "Tab"),
                    (0x2a, "Backspace"),
                    (0x39, "Caps Lock"),
                    (0x46, "Print Screen"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in nav_keys {
                        let vk = MouseProfile::hid_usage_to_vk(code);
                        let is_sel = self
                            .rebind_candidate
                            .map(|b| b[0] == 0x02 && b[1] == 0x00 && b[2] == code)
                            .unwrap_or(false);
                        if ui
                            .add_sized([80.0, 28.0], egui::SelectableLabel::new(is_sel, label))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, vk]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
            _ => {
                // Symbols
                let sym_keys = [
                    (0x2d, "- Minus"),
                    (0x2e, "= Equals"),
                    (0x2f, "[ Left Bracket"),
                    (0x30, "] Right Bracket"),
                    (0x31, "\\ Backslash"),
                    (0x33, "; Semicolon"),
                    (0x34, "' Quote"),
                    (0x35, "` Grave"),
                    (0x36, ", Comma"),
                    (0x37, ". Period"),
                    (0x38, "/ Slash"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in sym_keys {
                        let vk = MouseProfile::hid_usage_to_vk(code);
                        let is_sel = self
                            .rebind_candidate
                            .map(|b| b[0] == 0x02 && b[1] == 0x00 && b[2] == code)
                            .unwrap_or(false);
                        if ui
                            .add_sized([110.0, 28.0], egui::SelectableLabel::new(is_sel, label))
                            .clicked()
                        {
                            self.rebind_candidate = Some([0x02, 0x00, code, vk]);
                            self.rebind_key_code = code;
                        }
                    }
                });
            }
        }
    }

    fn render_category_shortcut(&mut self, ui: &mut egui::Ui) {
        ui.label("Select modifier keys:");
        ui.horizontal(|ui| {
            let mut ctrl = (self.rebind_modifiers & 0x01) != 0;
            if ui.checkbox(&mut ctrl, "Ctrl").changed() {
                if ctrl {
                    self.rebind_modifiers |= 0x01;
                } else {
                    self.rebind_modifiers &= !0x01;
                }
            }

            let mut shift = (self.rebind_modifiers & 0x02) != 0;
            if ui.checkbox(&mut shift, "Shift").changed() {
                if shift {
                    self.rebind_modifiers |= 0x02;
                } else {
                    self.rebind_modifiers &= !0x02;
                }
            }

            let mut alt = (self.rebind_modifiers & 0x04) != 0;
            if ui.checkbox(&mut alt, "Alt").changed() {
                if alt {
                    self.rebind_modifiers |= 0x04;
                } else {
                    self.rebind_modifiers &= !0x04;
                }
            }

            let mut win = (self.rebind_modifiers & 0x08) != 0;
            if ui.checkbox(&mut win, "Win").changed() {
                if win {
                    self.rebind_modifiers |= 0x08;
                } else {
                    self.rebind_modifiers &= !0x08;
                }
            }
        });

        ui.add_space(6.0);
        ui.label("Select target key:");
        egui::ComboBox::from_label("")
            .selected_text(format_hid_key_name(self.rebind_key_code))
            .show_ui(ui, |ui| {
                for code in 0x04..=0x1d {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
                for code in 0x1e..=0x27 {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
                for code in 0x3a..=0x45 {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
                for code in [0x28, 0x29, 0x2c, 0x2b, 0x2a, 0x4b, 0x4e, 0x4a, 0x4d] {
                    ui.selectable_value(&mut self.rebind_key_code, code, format_hid_key_name(code));
                }
            });

        let vk = MouseProfile::hid_usage_to_vk(self.rebind_key_code);
        self.rebind_candidate = Some([0x02, self.rebind_modifiers, self.rebind_key_code, vk]);
    }

    fn render_category_special(&mut self, ui: &mut egui::Ui) {
        let specials = [
            ("DPI Loop (Cycle All)", [0x08, 0x00, 0x00, 0x00]),
            ("DPI +", [0x09, 0x02, 0x00, 0x00]),
            ("DPI -", [0x09, 0x03, 0x00, 0x00]),
            ("Fire Key (Rapid Burst)", [0x06, 0x92, 0x00, 0x00]),
            ("Office Mode Switch", [0x09, 0x00, 0x00, 0x00]),
            ("Game Mode Switch", [0x09, 0x01, 0x00, 0x00]),
            ("LED Mode Switch", [0x0a, 0x01, 0x00, 0x00]),
        ];

        for (label, bytes) in specials {
            let is_sel = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_sel, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_media(&mut self, ui: &mut egui::Ui) {
        // Verified from Key_MediaClick (0x005814b0): [0x03, usage_lsb, usage_msb, 0x00]
        let media = [
            ("Play / Pause", [0x03, 0xcd, 0x00, 0x00]),
            ("Next Track", [0x03, 0xb5, 0x00, 0x00]),
            ("Previous Track", [0x03, 0xb6, 0x00, 0x00]),
            ("Stop", [0x03, 0xb7, 0x00, 0x00]),
            ("Volume +", [0x03, 0xe9, 0x00, 0x00]),
            ("Volume -", [0x03, 0xea, 0x00, 0x00]),
            ("Mute", [0x03, 0xe2, 0x00, 0x00]),
            ("Media Player", [0x03, 0x83, 0x01, 0x00]),
        ];

        for (label, bytes) in media {
            let is_sel = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_sel, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_productivity(&mut self, ui: &mut egui::Ui) {
        // Verified from Key_MediaClick (0x005814b0) & Key_AssemblyClick (0x005828b0)
        let prod = [
            ("Desktop (Win+D)", [0x02, 0x08, 0x07, b'D']),
            ("Snip (Shift+Win+S)", [0x02, 0x0a, 0x16, b'S']),
            ("Calculator", [0x03, 0x92, 0x01, 0x00]),
            ("My Computer", [0x03, 0x94, 0x01, 0x00]),
            ("Email Client", [0x03, 0x8a, 0x01, 0x00]),
            ("Web Back", [0x03, 0x24, 0x02, 0x00]),
            ("Web Forward", [0x03, 0x25, 0x02, 0x00]),
            ("Web Home", [0x03, 0x23, 0x02, 0x00]),
            ("Web Refresh", [0x03, 0x27, 0x02, 0x00]),
            ("LED Mode Cycle", [0x0a, 0x01, 0x00, 0x00]),
        ];

        for (label, bytes) in prod {
            let is_sel = self.rebind_candidate == Some(bytes);
            if ui.selectable_label(is_sel, label).clicked() {
                self.rebind_candidate = Some(bytes);
            }
        }
    }

    fn render_category_macro(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(
                "Select a recorded hardware macro to bind to this physical button [EXPERIMENTAL]:",
            )
            .small(),
        );
        ui.add_space(6.0);
        if self.macro_bank.macros.is_empty() {
            ui.label(
                egui::RichText::new(
                    "No macros recorded in Macro Manager yet. Go to Macro tab to create one.",
                )
                .color(Color32::from_rgb(255, 180, 0)),
            );
        } else {
            for (idx, macro_item) in self.macro_bank.macros.iter().enumerate() {
                let is_selected = match self.rebind_candidate {
                    Some([0x05, _, m_idx, _]) => m_idx as usize == idx,
                    _ => false,
                };
                let label = format!(
                    "Macro #{}: {} ({} steps)",
                    idx + 1,
                    macro_item.name,
                    macro_item.actions.len()
                );
                if ui.selectable_label(is_selected, label).clicked() {
                    // Verified from Key_MacroClick (0x0058328c): [0x05, repeat_mode, macro_idx, 0x00]
                    self.rebind_candidate = Some([0x05, 0x01, idx as u8, 0x00]);
                }
            }
        }
    }
}
