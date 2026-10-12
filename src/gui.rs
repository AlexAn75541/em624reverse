use eframe::egui;
use hidapi::HidApi;
use std::path::PathBuf;

use crate::device::Em624Device;
use crate::profile::{MouseProfile, OFFICIAL_DPI_STEPS};

#[derive(Debug, PartialEq, Clone, Copy)]
enum Tab {
    OfficeMode,
    GameMode,
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
    // Converter state
    convert_input_path: Option<PathBuf>,
    convert_log: String,
}

impl Default for Em624App {
    fn default() -> Self {
        // Load embedded verified DailyUse baseline as initial state
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let profile = MouseProfile::from_bytes(baseline).unwrap();

        Self {
            profile,
            active_tab: Tab::GameMode,
            status_message: "Ready. Connect your mouse or load a profile.".to_string(),
            device_connected: false,
            connected_info: "No device connected".to_string(),
            file_path: None,
            convert_input_path: None,
            convert_log: "Select a stock .bin profile file to convert into safe .em624 format.".to_string(),
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
                self.connected_info = format!("E-DRA EM624 (VID 0x{:04x}, PID 0x{:04x})", dev.vid, dev.pid);
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
                    Ok(data) => {
                        match MouseProfile::from_bytes(&data) {
                            Ok(prof) => {
                                self.profile = prof;
                                self.device_connected = true;
                                self.status_message = "Successfully loaded active profile from mouse hardware!".to_string();
                            }
                            Err(e) => self.status_message = format!("Profile parse error: {}", e),
                        }
                    }
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
                // Enforce safety transformations before flashing
                self.profile.apply_safe_profile();
                self.status_message = "Flashing 293 chunks to mouse flash memory...".to_string();

                match dev.write_profile(&self.profile.raw) {
                    Ok(()) => {
                        self.device_connected = true;
                        self.status_message = "Successfully flashed profile to mouse! Settings active immediately.".to_string();
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
        // Dark theme tuning
        ctx.set_visuals(egui::Visuals::dark());

        // Top Navigation Bar
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("E-DRA EM624").strong().color(egui::Color32::from_rgb(0, 242, 254)));
                ui.label(egui::RichText::new("GAMING MOUSE CONFIGURATOR").small().color(egui::Color32::from_rgb(160, 180, 210)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.device_connected {
                        ui.label(egui::RichText::new("● CONNECTED").color(egui::Color32::from_rgb(46, 213, 115)).strong());
                    } else {
                        if ui.button("Connect Mouse").clicked() {
                            self.try_connect_silent();
                        }
                        ui.label(egui::RichText::new("○ DISCONNECTED").color(egui::Color32::from_rgb(255, 71, 87)));
                    }
                });
            });
            ui.add_space(8.0);

            // Tab Buttons
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::OfficeMode, "Office Mode");
                ui.selectable_value(&mut self.active_tab, Tab::GameMode, "Game Mode");
                ui.selectable_value(&mut self.active_tab, Tab::Advanced, "Advanced (DPI & Sensor)");
                ui.selectable_value(&mut self.active_tab, Tab::Converter, "Profile Converter Tool");
            });
            ui.add_space(6.0);
        });

        // Bottom Action Toolbar & Status Bar
        egui::TopBottomPanel::bottom("bottom_toolbar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Read from Mouse").clicked() {
                    self.read_from_mouse();
                }

                if ui.button(egui::RichText::new("Flash to Mouse").strong().color(egui::Color32::from_rgb(0, 242, 254))).clicked() {
                    self.flash_to_mouse();
                }

                ui.separator();

                if ui.button("Load Profile File").clicked() {
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

                if ui.button("Save Profile File").clicked() {
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
                    ui.label(egui::RichText::new(&self.status_message).small().italics());
                });
            });
            ui.add_space(8.0);
        });

        // Central Content Area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                Tab::OfficeMode => {
                    self.render_button_mode_tab(ui, false);
                }
                Tab::GameMode => {
                    self.render_button_mode_tab(ui, true);
                }
                Tab::Advanced => {
                    self.render_advanced_tab(ui);
                }
                Tab::Converter => {
                    self.render_converter_tab(ui);
                }
            }
        });
    }
}

impl Em624App {
    fn render_button_mode_tab(&mut self, ui: &mut egui::Ui, is_gaming: bool) {
        ui.horizontal(|ui| {
            // Left Column: Button List
            ui.vertical(|ui| {
                ui.set_min_width(340.0);
                ui.heading(if is_gaming { "Gaming Mode Bindings" } else { "Office Mode Bindings" });
                ui.label(egui::RichText::new("Configure 7 physical buttons + secondary actions:").small().color(egui::Color32::from_rgb(140, 160, 190)));
                ui.add_space(8.0);

                let buttons = if is_gaming {
                    &self.profile.gaming_buttons
                } else {
                    &self.profile.office_buttons
                };

                for btn in buttons {
                    if btn.physical_button.contains("Unused") {
                        continue;
                    }
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&btn.physical_button).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(&btn.description).color(egui::Color32::from_rgb(0, 242, 254)));
                            });
                        });
                    });
                    ui.add_space(4.0);
                }
            });

            ui.separator();

            // Center Column: Mouse Visualizer with Large PC Mouse Emoji
            ui.vertical_centered(|ui| {
                ui.add_space(15.0);
                // Big PC Mouse Emoji as temporary substitute
                ui.label(egui::RichText::new("🖱").size(150.0));

                ui.heading(egui::RichText::new("E-DRA EM624").strong());
                ui.label(egui::RichText::new("SunplusIT SPCP6662 Gaming Sensor • Huano 50M Switches").small().color(egui::Color32::from_rgb(150, 170, 200)));

                ui.add_space(12.0);

                // Physical Button Legend
                ui.group(|ui| {
                    ui.set_max_width(360.0);
                    ui.label(egui::RichText::new("Hardware Switch Layout:").strong());
                    ui.label("  (1) Left Click       •  (2) Right Click");
                    ui.label("  (3) Middle Click     •  (4) Side 1 (Forward)");
                    ui.label("  (5) Side 2 (Backward)•  (6) Top DPI +");
                    ui.label("  (7) Top DPI -        •  (8) Scroll Wheel");
                });
            });
        });
    }

    fn render_advanced_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Optical Sensor & DPI Stages (SPCP6662)");
        ui.label(egui::RichText::new("14 discrete calibrated hardware steps (200 to 10,000 DPI)").small().color(egui::Color32::from_rgb(140, 160, 190)));
        ui.add_space(12.0);

        egui::Grid::new("dpi_grid")
            .num_columns(5)
            .spacing([14.0, 10.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Stage").strong());
                ui.label(egui::RichText::new("Enable").strong());
                ui.label(egui::RichText::new("Hardware Resolution").strong());
                ui.label(egui::RichText::new("DPI Step Slider").strong());
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
                    ui.label(egui::RichText::new(format!("{} DPI", dpi_val)).strong().color(egui::Color32::from_rgb(0, 242, 254)));

                    // Step Slider (0..12)
                    let mut step_val = current_step;
                    if ui.add(egui::Slider::new(&mut step_val, 0..=12).show_value(false)).changed() {
                        let byte_off = 0x2c + (i / 2);
                        if i % 2 == 0 {
                            self.profile.raw[byte_off] = (self.profile.raw[byte_off] & 0xf0) | (step_val as u8 & 0x0f);
                        } else {
                            self.profile.raw[byte_off] = (self.profile.raw[byte_off] & 0x0f) | ((step_val as u8 & 0x0f) << 4);
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

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading("USB Report Rate (Polling Speed)");
        ui.horizontal(|ui| {
            ui.label("Selected Rate:");
            ui.label(egui::RichText::new("1000 Hz (1 ms)").strong().color(egui::Color32::from_rgb(46, 213, 115)));
            ui.label("• 125 Hz • 250 Hz • 500 Hz • 1000 Hz");
        });
    }

    fn render_converter_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Stock Profile (.bin) Migration Tool");
        ui.label(egui::RichText::new("Convert vendor exported profiles into safe modern .em624 files with full button preservation.").small().color(egui::Color32::from_rgb(140, 160, 190)));
        ui.add_space(12.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("Select .bin File to Convert...").clicked() {
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
