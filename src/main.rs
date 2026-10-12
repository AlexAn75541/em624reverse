mod device;
mod gui;
mod profile;

use anyhow::Result;
use clap::{Parser, Subcommand};
use hidapi::HidApi;
use std::fs;
use std::path::PathBuf;

use device::Em624Device;
use profile::MouseProfile;

#[derive(Parser)]
#[command(name = "em624")]
#[command(about = "Cross-platform driver & configuration GUI/CLI for E-DRA EM624 Gaming Mouse", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the native graphical desktop interface (default if no subcommand is given)
    Gui,
    /// List all connected USB HID devices
    List,
    /// Inspect and decode a 2048-byte .bin profile file
    InspectProfile {
        /// Path to the .bin profile file
        path: PathBuf,
    },
    /// Flash a 2048-byte configuration profile (.em624 or .bin) to the mouse
    ApplyProfile {
        /// Path to the .em624 or .bin profile file
        path: PathBuf,
        /// Button mapping mode:
        /// 'keep' (default): Preserves profile's own button mappings (e.g. Win+D, Shift+Win+S, hotkeys) with safe stage 6 mask
        /// 'standard': Enforces standard browser actions (Left, Right, Middle, Forward, Backward, CPI Loop)
        /// 'daily': Clone DailyUse baseline
        #[arg(long, default_value = "keep")]
        buttons: String,
        #[arg(long)]
        vid: Option<u16>,
        #[arg(long)]
        pid: Option<u16>,
    },
    /// Read the active 2048-byte configuration profile directly from mouse flash memory
    ReadProfile {
        /// Destination file to save the dumped .bin profile
        dest: PathBuf,
        #[arg(long)]
        vid: Option<u16>,
        #[arg(long)]
        pid: Option<u16>,
    },
    /// Live probe of the connected mouse hardware state over USB
    Probe {
        #[arg(long)]
        vid: Option<u16>,
        #[arg(long)]
        pid: Option<u16>,
    },
    /// Convert and fix a profile .bin file for full compatibility with mouse and modern driver
    #[command(alias = "fix-profile")]
    ConvertProfile {
        /// Path to input .bin profile file
        input: PathBuf,
        /// Path to output converted .bin profile file (defaults to overwriting input with backup)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Button mapping mode: 'standard' (default) or 'keep'
        #[arg(short, long, default_value = "standard")]
        buttons: String,
    },
    /// Convert and fix all .bin profiles in a directory
    #[command(alias = "repair-profiles")]
    ConvertAll {
        /// Directory containing .bin profiles
        #[arg(default_value = "exampleProfiles")]
        dir: PathBuf,
        /// Button mapping mode: 'standard' (default) or 'keep'
        #[arg(short, long, default_value = "standard")]
        buttons: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Gui) => {
            let native_options = eframe::NativeOptions {
                viewport: eframe::egui::ViewportBuilder::default()
                    .with_inner_size([880.0, 640.0])
                    .with_min_inner_size([700.0, 500.0])
                    .with_title("E-DRA EM624 Gaming Mouse Configurator"),
                ..Default::default()
            };
            eframe::run_native(
                "E-DRA EM624 Gaming Mouse Configurator",
                native_options,
                Box::new(|cc| Ok(Box::new(gui::Em624App::new(cc)))),
            )
            .map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;
        }
        Some(Commands::List) => {
            println!("Enumerating connected USB HID devices...\n");
            let api = HidApi::new()?;
            let mut count = 0;
            for dev in api.device_list() {
                let mfg = dev.manufacturer_string().unwrap_or("Unknown");
                let prod = dev.product_string().unwrap_or("Unknown");
                let vid = dev.vendor_id();
                let pid = dev.product_id();
                let usage_page = dev.usage_page();
                let usage = dev.usage();
                let path = dev.path().to_string_lossy();

                let is_match = vid == device::DEFAULT_VID && pid == device::DEFAULT_PID;
                let tag = if is_match { " [★ EM624 MATCH ★]" } else { "" };

                println!(
                    "VID: 0x{:04x} | PID: 0x{:04x} | IF: {:2} | Page: 0x{:04x} | Usage: 0x{:04x} | {}: {} | Path: {}{}",
                    vid, pid, dev.interface_number(), usage_page, usage, mfg, prod, path, tag
                );
                count += 1;
            }
            println!("\nTotal devices scanned: {}", count);
        }
        Some(Commands::InspectProfile { path }) => {
            println!("Decoding profile: {}", path.display());
            let profile = MouseProfile::load_file(&path)?;

            println!("Device Name: {}", profile.device_name);
            println!("Active Stages Mask: 0x{:02x} ({:06b})", profile.active_stages_mask, profile.active_stages_mask & 0x3f);
            let mode_str = if profile.is_gaming_mode { "Gaming Mode" } else { "Office Mode" };
            println!("Active Profile Mode: {}", mode_str);

            println!("\nOffice Mode Buttons (0x00C0):");
            for btn in &profile.office_buttons {
                if btn.description != "None" && !btn.physical_button.contains("Unused") {
                    println!("  {:28} -> {}", btn.physical_button, btn.description);
                }
            }

            println!("\nGaming Mode Buttons (0x00F1):");
            for btn in &profile.gaming_buttons {
                if btn.description != "None" && !btn.physical_button.contains("Unused") {
                    println!("  {:28} -> {}", btn.physical_button, btn.description);
                }
            }

            println!("\nDPI Stages & Color Mapping:");
            for (idx, stage) in profile.dpi_stages.iter().enumerate() {
                let status = if stage.enabled { "ENABLED" } else { "DISABLED" };
                println!(
                    "  Stage {}: Level={:2} (~{:4} DPI) | RGB=({:3}, {:3}, {:3}) | [{}]",
                    idx + 1,
                    stage.level_index,
                    stage.approx_dpi,
                    stage.color.r,
                    stage.color.g,
                    stage.color.b,
                    status
                );
            }
        }
        Some(Commands::ApplyProfile { path, buttons, vid, pid }) => {
            println!("Reading profile file: {}", path.display());
            let mut profile = MouseProfile::load_file(&path)?;
            println!("Loaded profile '{}' (Active mask: 0x{:02x})", profile.device_name, profile.active_stages_mask);

            match buttons.to_lowercase().as_str() {
                "standard" => {
                    println!("  -> [BUTTONS] Enforcing standard mouse buttons (Left, Right, Middle, Forward, Backward, CPI Loop)...");
                    profile.apply_standard_buttons();
                }
                "keep" => {
                    println!("  -> [BUTTONS] Preserving profile's own button mappings (safe stage 6 mask & verified checksums)...");
                    profile.apply_safe_profile();
                }
                "daily" => {
                    println!("  -> [BUTTONS] Synthesizing with DailyUse baseline...");
                    profile.apply_og_compat();
                }
                other => {
                    eprintln!("Unknown button mode '{}', falling back to 'standard'", other);
                    profile.apply_standard_buttons();
                }
            }

            let api = HidApi::new()?;
            let dev = match (vid, pid) {
                (Some(v), Some(p)) => Em624Device::open_vid_pid(&api, v, p)?,
                _ => Em624Device::open_auto(&api)?,
            };

            println!("Connected to EM624 Mouse (VID: 0x{:04x}, PID: 0x{:04x})", dev.vid, dev.pid);
            println!("Flashing profile to mouse memory (293 chunks)...");
            dev.write_profile(&profile.raw)?;
            println!("Successfully applied profile to mouse!");
        }
        Some(Commands::ReadProfile { dest, vid, pid }) => {
            let api = HidApi::new()?;
            let dev = match (vid, pid) {
                (Some(v), Some(p)) => Em624Device::open_vid_pid(&api, v, p)?,
                _ => Em624Device::open_auto(&api)?,
            };

            println!("Connected to EM624 Mouse (VID: 0x{:04x}, PID: 0x{:04x})", dev.vid, dev.pid);
            println!("Dumping active profile from mouse flash...");
            let data = dev.read_profile()?;
            fs::write(&dest, &data)?;
            println!("Dumped {} bytes to {}", data.len(), dest.display());

            if let Ok(profile) = MouseProfile::parse(&data) {
                println!("Active Profile Header: {}", profile.device_name);
                println!("Active Stages Mask: 0x{:02x}", profile.active_stages_mask);
            }
        }
        Some(Commands::Probe { vid, pid }) => {
            println!("Connecting to EM624 Mouse for hardware diagnostic probe...");
            let api = HidApi::new()?;
            let dev = match (vid, pid) {
                (Some(v), Some(p)) => Em624Device::open_vid_pid(&api, v, p)?,
                _ => Em624Device::open_auto(&api)?,
            };

            println!("  Connected: VID 0x{:04x}, PID 0x{:04x}", dev.vid, dev.pid);
            println!("  Reading active profile memory over USB HID (293 chunks)...");
            let data = dev.read_profile()?;
            let profile = MouseProfile::parse(&data)?;

            println!("\n=== EM624 LIVE HARDWARE PROBE REPORT ===");
            println!("Device Name String : {}", profile.device_name);
            println!("Active Profile Mode: {}", if profile.is_gaming_mode { "Gaming Mode" } else { "Office Mode" });
            println!("Active Stages Mask : 0x{:02x} ({:06b})", profile.active_stages_mask, profile.active_stages_mask & 0x3f);

            // Hardware register safety audit
            let reg_mode = data[0x00a4];
            let reg_lockout = data[0x03f9];
            let stage6_active = (profile.active_stages_mask & (1 << 5)) != 0;

            println!("\n--- Hardware Register Audit ---");
            println!("  Mode Register (0x00A4)   : 0x{:02x} -> {}", reg_mode, if reg_mode == 1 { "Gaming [OK]" } else { "Office" });
            println!("  Lockout Flag  (0x03F9)   : 0x{:02x} -> {}", reg_lockout, if (reg_lockout & 0x80) == 0 { "Buttons ACTIVE [OK]" } else { "⚠️ BUTTON INTERRUPT LOCKED (0x80 bit set!)" });
            println!("  Stage 6 Status           : {}", if !stage6_active { "Disabled [SAFE]" } else { "⚠️ ENABLED (May trigger MCU sensor hang!)" });

            println!("\n--- Active DPI Stages & RGB LEDs ---");
            for (idx, stage) in profile.dpi_stages.iter().enumerate() {
                let status = if stage.enabled { "ACTIVE" } else { "disabled" };
                println!(
                    "  Stage {}: ~{:4} DPI (Level {:2}) | RGB=({:3}, {:3}, {:3}) | [{}]",
                    idx + 1, stage.approx_dpi, stage.level_index,
                    stage.color.r, stage.color.g, stage.color.b, status
                );
            }

            println!("\n--- Current Button Mapping ({}) ---", if profile.is_gaming_mode { "Gaming Mode" } else { "Office Mode" });
            let active_buttons = if profile.is_gaming_mode { &profile.gaming_buttons } else { &profile.office_buttons };
            for btn in active_buttons {
                if btn.description != "None" && !btn.physical_button.contains("Unused") {
                    println!("  {:28} -> {}", btn.physical_button, btn.description);
                }
            }
            println!("========================================\n");
        }
        Some(Commands::ConvertProfile { input, output, buttons }) => {
            println!("Loading profile: {}", input.display());
            let mut profile = MouseProfile::load_file(&input)?;

            match buttons.to_lowercase().as_str() {
                "standard" => {
                    println!("  -> Enforcing standard mouse buttons (Left, Right, Middle, Forward, Back, CPI Loop)...");
                    profile.apply_standard_buttons();
                }
                "keep" => {
                    println!("  -> Preserving profile's custom buttons with safe stage 6 mask & recalculated checksums...");
                    profile.apply_safe_profile();
                }
                _ => {
                    profile.apply_standard_buttons();
                }
            }

            let target_path = output.unwrap_or_else(|| {
                let mut em624 = input.clone();
                em624.set_extension("em624");
                em624
            });

            profile.save_file(&target_path)?;
            println!("Successfully converted and saved profile to: {}", target_path.display());
        }
        Some(Commands::ConvertAll { dir, buttons }) => {
            println!("Scanning directory for .bin profiles: {}", dir.display());
            let mut converted_count = 0;
            let mut scanned_count = 0;

            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    scanned_count += 1;
                    if let Ok(mut profile) = MouseProfile::load_file(&path) {
                        match buttons.to_lowercase().as_str() {
                            "standard" => profile.apply_standard_buttons(),
                            _ => profile.apply_safe_profile(),
                        }

                        let mut out_path = path.clone();
                        out_path.set_extension("em624");
                        profile.save_file(&out_path)?;
                        println!("  [CONVERTED] {} -> {}", path.file_name().unwrap_or_default().to_string_lossy(), out_path.file_name().unwrap_or_default().to_string_lossy());
                        converted_count += 1;
                    }
                }
            }
            println!("\nConversion finished: {} scanned, {} converted to .em624.", scanned_count, converted_count);
        }
    }

    Ok(())
}
