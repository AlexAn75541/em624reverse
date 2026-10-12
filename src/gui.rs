use eframe::egui;
use eframe::egui::{Color32, Pos2, Rect, Stroke, Vec2};
use hidapi::HidApi;
use std::path::PathBuf;

use crate::device::Em624Device;
use crate::profile::{MouseProfile, OFFICIAL_DPI_STEPS};

#[derive(Debug, PartialEq, Clone, Copy)]
enum Tab {
    GameMode,
    OfficeMode,
    Advanced,
    Converter,
}

pub struct Em624App {
    profile: MouseProfile,
    active_tab: Tab,
    status_message: String,
    device_connected: bool,
    connected_info: String,
    file_path: Option<PathBuf>,
    hovered_button: Option<usize>, // 1..=7
    // Converter state
    convert_input_path: Option<PathBuf>,
    convert_log: String,
}

impl Default for Em624App {
    fn default() -> Self {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let profile = MouseProfile::from_bytes(baseline).unwrap();

        Self {
            profile,
            active_tab: Tab::GameMode,
            status_message: "Ready. Connect your mouse or load a profile.".to_string(),
            device_connected: false,
            connected_info: "No device connected".to_string(),
            file_path: None,
            hovered_button: None,
            convert_input_path: None,
            convert_log: "Select a stock .bin profile file to convert into safe .em624 format."
                .to_string(),
        }
    }
}

impl Em624App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.try_connect_silent();
        app
    }

    fn try_connect_silent(&mut self) {
        if let Ok(api) = HidApi::new() {
            if let Ok(dev) = Em624Device::open_auto(&api) {
                self.device_connected = true;
                self.connected_info =
                    format!("E-DRA EM624 (VID 0x{:04x}, PID 0x{:04x})", dev.vid, dev.pid);
                self.status_message = "Connected to mouse via USB HID (MI_01&Col04).".to_string();
            }
        }
    }

    fn read_from_mouse(&mut self) {
        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                return;
            }
        };

        match Em624Device::open_auto(&api) {
            Ok(dev) => {
                self.status_message = "Reading active 2048-byte EEPROM from mouse...".to_string();
                match dev.read_profile() {
                    Ok(data) => match MouseProfile::from_bytes(&data) {
                        Ok(prof) => {
                            self.profile = prof;
                            self.device_connected = true;
                            self.status_message =
                                "Successfully loaded active profile from mouse hardware!"
                                    .to_string();
                        }
                        Err(e) => self.status_message = format!("Profile parse error: {}", e),
                    },
                    Err(e) => self.status_message = format!("Failed to read profile: {}", e),
                }
            }
            Err(e) => {
                self.device_connected = false;
                self.status_message = format!("Mouse not found or busy: {}", e);
            }
        }
    }

    fn flash_to_mouse(&mut self) {
        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                self.status_message = format!("HID API Error: {}", e);
                return;
            }
        };

        match Em624Device::open_auto(&api) {
            Ok(dev) => {
                self.profile.apply_safe_profile();
                self.status_message = "Flashing 293 chunks to mouse flash memory...".to_string();

                match dev.write_profile(&self.profile.raw) {
                    Ok(()) => {
                        self.device_connected = true;
                        self.status_message =
                            "Successfully flashed profile to mouse! Settings active immediately."
                                .to_string();
                    }
                    Err(e) => self.status_message = format!("Flash failed: {}", e),
                }
            }
            Err(e) => {
                self.device_connected = false;
                self.status_message = format!("Mouse not connected: {}", e);
            }
        }
    }
}

impl eframe::App for Em624App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // Top Navigation Bar
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🖱").size(22.0));
                ui.heading(
                    egui::RichText::new("E-DRA EM624")
                        .strong()
                        .color(Color32::from_rgb(0, 242, 254)),
                );
                ui.label(
                    egui::RichText::new("GAMING MOUSE CONFIGURATOR")
                        .small()
                        .color(Color32::from_rgb(140, 160, 190)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.device_connected {
                        ui.label(
                            egui::RichText::new("● CONNECTED")
                                .color(Color32::from_rgb(46, 213, 115))
                                .strong(),
                        );
                    } else {
                        if ui.button("Connect Mouse").clicked() {
                            self.try_connect_silent();
                        }
                        ui.label(
                            egui::RichText::new("○ DISCONNECTED")
                                .color(Color32::from_rgb(255, 71, 87)),
                        );
                    }
                });
            });
            ui.add_space(6.0);

            // Tab Buttons with icons
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::GameMode, "🎮 Game Mode");
                ui.selectable_value(&mut self.active_tab, Tab::OfficeMode, "💼 Office Mode");
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Advanced,
                    "⚡ Advanced (DPI & Sensor)",
                );
                ui.selectable_value(&mut self.active_tab, Tab::Converter, "🔄 Profile Converter");
            });
            ui.add_space(6.0);
        });

        // Bottom Action Toolbar & Status Bar
        egui::TopBottomPanel::bottom("bottom_toolbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("📥 Read from Mouse").clicked() {
                    self.read_from_mouse();
                }

                if ui
                    .button(
                        egui::RichText::new("⚡ Flash to Mouse")
                            .strong()
                            .color(Color32::from_rgb(0, 242, 254)),
                    )
                    .clicked()
                {
                    self.flash_to_mouse();
                }

                ui.separator();

                if ui.button("📂 Load Profile").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("EM624 Profiles", &["em624", "bin"])
                        .pick_file()
                    {
                        if let Ok(p) = MouseProfile::load_file(&path) {
                            self.profile = p;
                            self.file_path = Some(path.clone());
                            self.status_message = format!("Loaded profile: {}", path.display());
                        }
                    }
                }

                if ui.button("💾 Save Profile").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Safe EM624 Profile", &["em624"])
                        .set_file_name("custom_profile.em624")
                        .save_file()
                    {
                        self.profile.apply_safe_profile();
                        if let Ok(()) = self.profile.save_file(&path) {
                            self.file_path = Some(path.clone());
                            self.status_message = format!("Saved profile to: {}", path.display());
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(&self.status_message)
                            .small()
                            .italics()
                            .color(Color32::from_rgb(170, 190, 220)),
                    );
                });
            });
            ui.add_space(6.0);
        });

        // Central Content Area
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            Tab::GameMode => self.render_button_mode_tab(ui, true),
            Tab::OfficeMode => self.render_button_mode_tab(ui, false),
            Tab::Advanced => self.render_advanced_tab(ui),
            Tab::Converter => self.render_converter_tab(ui),
        });
    }
}

impl Em624App {
    fn render_button_mode_tab(&mut self, ui: &mut egui::Ui, is_gaming: bool) {
        ui.horizontal(|ui| {
            // Left Column: Button List (Fixed 340px width to prevent pushing center diagram)
            ui.allocate_ui_with_layout(
                Vec2::new(340.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(4.0);
                    ui.heading(if is_gaming {
                        "Gaming Mode Bindings"
                    } else {
                        "Office Mode Bindings"
                    });
                    ui.label(
                        egui::RichText::new("Hover buttons to highlight switch on mouse:")
                            .small()
                            .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(8.0);

                    let buttons = if is_gaming {
                        &self.profile.gaming_buttons
                    } else {
                        &self.profile.office_buttons
                    };

                    self.hovered_button = None;

                    // Display Buttons 1 through 7
                    let display_order = [
                        (1, "① Left Click"),
                        (4, "② Right Click"),
                        (7, "③ Middle Click"),
                        (2, "④ Forward (Side 1)"),
                        (5, "⑤ Backward (Side 2)"),
                        (9, "⑥ DPI + (Top)"),
                        (8, "⑦ DPI - (Back)"),
                    ];

                    for (slot_idx, label) in display_order {
                        if let Some(btn) = buttons.get(slot_idx - 1) {
                            let is_highlighted = self.hovered_button == Some(slot_idx);
                            let bg_color = if is_highlighted {
                                Color32::from_rgba_premultiplied(0, 242, 254, 25)
                            } else {
                                Color32::from_rgb(20, 26, 38)
                            };

                            let frame =
                                egui::Frame::group(ui.style())
                                    .fill(bg_color)
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if is_highlighted {
                                            Color32::from_rgb(0, 242, 254)
                                        } else {
                                            Color32::from_rgb(36, 48, 70)
                                        },
                                    ));

                            let response = frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(label).strong());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(&btn.description)
                                                    .color(Color32::from_rgb(0, 242, 254)),
                                            );
                                        },
                                    );
                                });
                            });

                            if response.response.hovered() {
                                self.hovered_button = Some(slot_idx);
                            }
                            ui.add_space(3.0);
                        }
                    }
                },
            );

            ui.separator();

            // Right Column: Centered Hardware Mouse Diagram
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), ui.available_height()),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("E-DRA EM624 HARDWARE SWITCH MATRIX")
                            .strong()
                            .color(Color32::from_rgb(140, 160, 190)),
                    );
                    ui.add_space(6.0);

                    // Draw custom ergonomic vector mouse with glowing badges
                    draw_mouse_diagram(ui, self.hovered_button);

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "SunplusIT SPCP6662 Optical Sensor • Huano 50M Switches",
                        )
                        .small()
                        .color(Color32::from_rgb(120, 140, 170)),
                    );
                },
            );
        });
    }

    fn render_advanced_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading("⚡ Optical Sensor & DPI Stages (SPCP6662)");
        ui.label(
            egui::RichText::new("14 calibrated hardware resolution steps (200 to 10,000 DPI)")
                .small()
                .color(Color32::from_rgb(140, 160, 190)),
        );
        ui.add_space(12.0);

        egui::Grid::new("dpi_grid")
            .num_columns(5)
            .spacing([18.0, 12.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Stage").strong());
                ui.label(egui::RichText::new("Active").strong());
                ui.label(egui::RichText::new("Resolution").strong());
                ui.label(egui::RichText::new("Calibrated Slider").strong());
                ui.label(egui::RichText::new("RGB LED").strong());
                ui.end_row();

                for i in 0..6 {
                    let is_active = (self.profile.active_stages_mask & (1 << i)) != 0;
                    let current_step = self.profile.dpi_stages[i].level_index.min(12) as usize;
                    let color = self.profile.dpi_stages[i].color.clone();

                    ui.label(format!("Stage {}", i + 1));

                    // Active checkbox
                    let mut active_checked = is_active;
                    if ui.checkbox(&mut active_checked, "").changed() {
                        if active_checked {
                            self.profile.active_stages_mask |= 1 << i;
                        } else {
                            self.profile.active_stages_mask &= !(1 << i);
                        }
                        self.profile.active_stages_mask &= 0xdf; // Safe stage 6 clamp
                    }

                    // DPI value display
                    let dpi_val = OFFICIAL_DPI_STEPS[current_step];
                    ui.label(
                        egui::RichText::new(format!("{:5} DPI", dpi_val))
                            .strong()
                            .color(Color32::from_rgb(0, 242, 254)),
                    );

                    // Step Slider (0..12)
                    let mut step_val = current_step;
                    if ui
                        .add(egui::Slider::new(&mut step_val, 0..=12).show_value(false))
                        .changed()
                    {
                        let byte_off = 0x2c + (i / 2);
                        if i % 2 == 0 {
                            self.profile.raw[byte_off] =
                                (self.profile.raw[byte_off] & 0xf0) | (step_val as u8 & 0x0f);
                        } else {
                            self.profile.raw[byte_off] = (self.profile.raw[byte_off] & 0x0f)
                                | ((step_val as u8 & 0x0f) << 4);
                        }
                        self.profile.refresh_stages();
                    }

                    // RGB Color Picker
                    let mut color_rgb = [color.r, color.g, color.b];
                    if ui.color_edit_button_srgb(&mut color_rgb).changed() {
                        let color_off = 0x3c + (i * 4);
                        self.profile.raw[color_off] = color_rgb[0];
                        self.profile.raw[color_off + 1] = color_rgb[1];
                        self.profile.raw[color_off + 2] = color_rgb[2];
                        self.profile.refresh_stages();
                    }

                    ui.end_row();
                }
            });

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading("⏱️ USB Polling Rate (Report Speed)");
        ui.horizontal(|ui| {
            ui.label("Selected Rate:");
            ui.label(
                egui::RichText::new("1000 Hz (1 ms)")
                    .strong()
                    .color(Color32::from_rgb(46, 213, 115)),
            );
            ui.label("• 125 Hz • 250 Hz • 500 Hz • 1000 Hz");
        });
    }

    fn render_converter_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading("🔄 Stock Profile (.bin) Migration Tool");
        ui.label(
            egui::RichText::new("Convert vendor exported profiles into safe modern .em624 files with full button preservation.")
                .small()
                .color(Color32::from_rgb(140, 160, 190)),
        );
        ui.add_space(12.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Select .bin File to Convert...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Vendor Profiles", &["bin"])
                        .pick_file()
                    {
                        self.convert_input_path = Some(path.clone());
                        if let Ok(mut prof) = MouseProfile::load_file(&path) {
                            prof.apply_safe_profile();
                            let mut out_path = path.clone();
                            out_path.set_extension("em624");
                            if let Ok(()) = prof.save_file(&out_path) {
                                self.convert_log = format!(
                                    "Successfully converted:\n  Source: {}\n  Target: {}\n\nAll 100% authentic button mappings, DPI steps, and colors preserved.",
                                    path.display(),
                                    out_path.display()
                                );
                            }
                        }
                    }
                }

                if let Some(p) = &self.convert_input_path {
                    ui.label(p.file_name().unwrap_or_default().to_string_lossy());
                }
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Conversion Log:").strong());
            ui.monospace(&self.convert_log);
        });
    }
}

/// Draw a centered, proportioned vector mouse with numbered badges matching physical switch routing
fn draw_mouse_diagram(ui: &mut egui::Ui, hovered_slot: Option<usize>) {
    let desired_size = Vec2::new(260.0, 320.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let cx = rect.center().x;
    let cy = rect.center().y;

    // Mouse Body: Ergonomic rounded shape
    let body_rect = Rect::from_center_size(Pos2::new(cx, cy + 10.0), Vec2::new(140.0, 240.0));
    painter.rect_filled(body_rect, 60.0, Color32::from_rgb(22, 28, 40));
    painter.rect_stroke(
        body_rect,
        60.0,
        Stroke::new(2.0_f32, Color32::from_rgb(0, 242, 254)),
    );

    // Center divider between Left & Right clicks
    painter.line_segment(
        [Pos2::new(cx, cy - 110.0), Pos2::new(cx, cy - 30.0)],
        Stroke::new(2.0_f32, Color32::from_rgb(36, 48, 70)),
    );

    // Scroll Wheel Slot (Button 3)
    let wheel_rect = Rect::from_center_size(Pos2::new(cx, cy - 65.0), Vec2::new(18.0, 42.0));
    let wheel_active = hovered_slot == Some(7);
    painter.rect_filled(
        wheel_rect,
        6.0,
        if wheel_active {
            Color32::from_rgb(0, 242, 254)
        } else {
            Color32::from_rgb(46, 58, 80)
        },
    );
    painter.rect_stroke(
        wheel_rect,
        6.0,
        Stroke::new(1.5_f32, Color32::from_rgb(0, 242, 254)),
    );

    // Top DPI Buttons (Buttons 6 & 7)
    let dpi_top_rect = Rect::from_center_size(Pos2::new(cx, cy - 25.0), Vec2::new(16.0, 18.0));
    let dpi_bot_rect = Rect::from_center_size(Pos2::new(cx, cy - 2.0), Vec2::new(16.0, 18.0));
    painter.rect_filled(dpi_top_rect, 3.0, Color32::from_rgb(36, 48, 70));
    painter.rect_filled(dpi_bot_rect, 3.0, Color32::from_rgb(36, 48, 70));

    // Left Side Thumb Buttons (Buttons 4 & 5)
    let side_fwd_rect =
        Rect::from_center_size(Pos2::new(cx - 72.0, cy - 25.0), Vec2::new(8.0, 28.0));
    let side_back_rect =
        Rect::from_center_size(Pos2::new(cx - 72.0, cy + 10.0), Vec2::new(8.0, 28.0));
    painter.rect_filled(side_fwd_rect, 3.0, Color32::from_rgb(0, 242, 254));
    painter.rect_filled(side_back_rect, 3.0, Color32::from_rgb(0, 242, 254));

    // Palm Rest Ambient RGB Glow
    let palm_logo = Rect::from_center_size(Pos2::new(cx, cy + 65.0), Vec2::new(32.0, 20.0));
    painter.rect_filled(
        palm_logo,
        10.0,
        Color32::from_rgba_premultiplied(0, 242, 254, 40),
    );
    painter.text(
        Pos2::new(cx, cy + 65.0),
        egui::Align2::CENTER_CENTER,
        "E-DRA",
        egui::FontId::proportional(11.0),
        Color32::from_rgb(0, 242, 254),
    );

    // Numbered Badge Draw Helper: ① Left, ② Right, ③ Wheel, ④ Forward, ⑤ Backward, ⑥ DPI+, ⑦ DPI-
    let badges = [
        (1, Pos2::new(cx - 40.0, cy - 65.0), "①"),
        (4, Pos2::new(cx + 40.0, cy - 65.0), "②"),
        (7, Pos2::new(cx, cy - 92.0), "③"),
        (2, Pos2::new(cx - 96.0, cy - 25.0), "④"),
        (5, Pos2::new(cx - 96.0, cy + 10.0), "⑤"),
        (9, Pos2::new(cx + 28.0, cy - 25.0), "⑥"),
        (8, Pos2::new(cx + 28.0, cy - 2.0), "⑦"),
    ];

    for (slot_idx, pos, text) in badges {
        let is_hovered = hovered_slot == Some(slot_idx);
        let badge_bg = if is_hovered {
            Color32::from_rgb(0, 242, 254)
        } else {
            Color32::from_rgb(14, 20, 32)
        };
        let text_color = if is_hovered {
            Color32::BLACK
        } else {
            Color32::from_rgb(0, 242, 254)
        };

        painter.circle_filled(pos, 11.0, badge_bg);
        painter.circle_stroke(
            pos,
            11.0,
            Stroke::new(1.5_f32, Color32::from_rgb(0, 242, 254)),
        );
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(13.0),
            text_color,
        );
    }
}
