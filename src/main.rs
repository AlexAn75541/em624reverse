mod device;
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
#[command(about = "Cross-platform driver & configuration CLI for EM624 Mouse", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all connected USB HID devices
    List,
    /// Inspect and decode a 2048-byte .bin profile file
    InspectProfile {
        /// Path to the .bin profile file
        path: PathBuf,
    },
    /// Flash a 2048-byte configuration profile to the mouse
    ApplyProfile {
        /// Path to the .bin profile file
        path: PathBuf,
        /// Full compatibility mode matching the original Delphi application profile logic
        #[arg(long, default_value_t = true)]
        og_compat: bool,
        /// Automatically sanitize button mapping to ensure clicks/scroll remain 100% active
        #[arg(long, default_value_t = true)]
        safe_buttons: bool,
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
    /// Repair a .bin profile's button table while preserving custom DPI and RGB colors
    FixProfile {
        /// Path to input .bin profile file
        input: PathBuf,
        /// Path to output fixed .bin profile file (defaults to overwriting input with backup)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Scan and repair all .bin profiles in a directory
    RepairProfiles {
        /// Directory containing .bin profiles
        #[arg(default_value = "profiles")]
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
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
        Commands::InspectProfile { path } => {
            println!("Decoding profile: {}", path.display());
            let profile = MouseProfile::load_file(&path)?;

            println!("Device Name: {}", profile.device_name);
            println!("Active Stages Mask: 0x{:02x} ({:06b})", profile.active_stages_mask, profile.active_stages_mask & 0x3f);
            let btn_status = if profile.is_button_table_valid() {
                "[VALID] Standard 1:1 Click Bindings"
            } else {
                "[WARNING] Contains non-standard / broken macro bindings (clicks may be non-responsive)"
            };
            println!("Button Table Status: {}", btn_status);

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
        Commands::ApplyProfile { path, og_compat, safe_buttons, vid, pid } => {
            println!("Reading profile file: {}", path.display());
            let mut profile = MouseProfile::load_file(&path)?;
            println!("Loaded profile '{}' (Active mask: 0x{:02x})", profile.device_name, profile.active_stages_mask);

            if og_compat {
                println!("  -> [OG-COMPAT] Synthesizing configuration with 100% Delphi hardware baseline...");
                profile.apply_og_compat();
            } else if safe_buttons {
                if profile.sanitize_buttons() {
                    println!("  -> [AUTO-REPAIR] Applied standard working button table (Left/Right/Middle clicks restored).");
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
        Commands::ReadProfile { dest, vid, pid } => {
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
        Commands::FixProfile { input, output } => {
            println!("Loading profile: {}", input.display());
            let mut profile = MouseProfile::load_file(&input)?;
            let modified = profile.sanitize_buttons();
            let target_path = output.unwrap_or_else(|| {
                let mut bak = input.clone();
                bak.set_extension("bin.bak");
                let _ = fs::copy(&input, &bak);
                println!("  Backed up original to {}", bak.display());
                input.clone()
            });

            profile.save_file(&target_path)?;
            if modified {
                println!("Repaired button table! Custom DPI/LED preserved. Saved to: {}", target_path.display());
            } else {
                println!("Button table was already valid. Saved to: {}", target_path.display());
            }
        }
        Commands::RepairProfiles { dir } => {
            println!("Scanning directory for profiles: {}", dir.display());
            let mut repaired_count = 0;
            let mut scanned_count = 0;

            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    scanned_count += 1;
                    if let Ok(mut profile) = MouseProfile::load_file(&path) {
                        if !profile.is_button_table_valid() {
                            let mut bak = path.clone();
                            bak.set_extension("bin.bak");
                            let _ = fs::copy(&path, &bak);
                            profile.sanitize_buttons();
                            profile.save_file(&path)?;
                            println!("  [REPAIRED] {} (backed up to .bak)", path.file_name().unwrap_or_default().to_string_lossy());
                            repaired_count += 1;
                        } else {
                            println!("  [OK]       {}", path.file_name().unwrap_or_default().to_string_lossy());
                        }
                    }
                }
            }
            println!("\nScan finished: {} scanned, {} repaired with standard working buttons.", scanned_count, repaired_count);
        }
    }

    Ok(())
}
