# E-DRA EM624 Mouse Configurator & Driver

[![CI & Continuous Pre-Release](https://github.com/AlexAn75541/em624reverse/actions/workflows/ci.yml/badge.svg)](https://github.com/AlexAn75541/em624reverse/actions/workflows/ci.yml)
[![GitHub Release](https://img.shields.io/github/v/release/AlexAn75541/em624reverse?color=10b981&label=Release)](https://github.com/AlexAn75541/em624reverse/releases/latest)
[![Continuous Builds](https://img.shields.io/badge/Continuous-Pre--Release-38bdf8)](https://github.com/AlexAn75541/em624reverse/releases/tag/pre-release)
[![License: MIT](https://img.shields.io/badge/License-MIT-amber.svg)](https://opensource.org/licenses/MIT)
[![Memory Footprint](https://img.shields.io/badge/RAM-≤_10MB_Working_Set-10b981)](#strict-resource-budget--memory-hygiene)

A 100% native Rust open-source companion application, driver, and reverse-engineering suite for the **E-DRA EM624 Gaming Mouse**. Built by decompiling and reverse-engineering the stock 2018 Delphi vendor utility (`EM624 Mouse.exe`) and analyzing real-time USB HID packets over USBPcap.

---

## Hardware Profile

| Component | Hardware Specification | Driver Implementation |
|---|---|---|
| **Device Model** | E-DRA EM624 RGB Gaming Mouse | Verified & Full Support |
| **USB Vendor ID (VID)** | `0x1BCF` (Sunplus Innovation Technology Inc.) | Auto-detected & Hotplug Monitored |
| **USB Product ID (PID)** | `0x08B8` (EM624 Gaming Mouse) | Auto-detected & Hotplug Monitored |
| **Optical Sensor** | SunplusIT SPCP6662 (6662IC) Optical Gaming Sensor | 15 Discrete Calibrated DPI Steps (`200` to `12,000 DPI`) |
| **Microcontroller (MCU)** | SunplusIT SPCP6651B High-Speed USB MCU | 2048-Byte EEPROM Read/Write via Feature Reports |
| **Microswitches** | Huano Blue Shell Pink Dot (50 Million rated clicks) | 7 Physical Switches + Scroll Wheel Encoder |
| **Target Interface** | Interface 1, Collection 04 (`Usage Page 0xFF00`, `Usage 0x01`) | Direct Vendor Configuration Pipe |
| **Memory Footprint** | Strict **&le; 10 MB Working Set RAM** | Continuous OS Heap Trimming & Compaction |

---

## Key Features & Modular UI Architecture

The desktop companion app adopts the modern modular architecture inspired by the [OpenMouse-Project](https://github.com/OpenMouse-Project), completely written in native Rust (`egui`/`eframe` and `glow`) with zero Electron, CEF, or web runtimes.

### 1. Buttons Tab (Switch Mapping Matrix)
- Interactive, responsive 1:1 mouse chassis diagram with subpixel switch coordinates and leader line callout badges.
- Remap all 7 physical switches across **Office Mode** (`0x00C0`) and **Gaming Mode** (`0x00F1`):
  - Standard Mouse Actions (Left, Right, Middle, Side Forward 1, Side Backward 2, DPI Loop, CPI+, CPI-).
  - Single Keyboard Keys & Full Modifiers (Ctrl, Shift, Alt, Windows).
  - Multimedia & Consumer Controls (Play/Pause, Next/Prev Track, Volume Up/Down, Mute).
  - Rapid Burst Fire Key with configurable click count and repeat delay.
  - Onboard Macro Triggers.

### 2. DPI Tuning Tab
- 15 discrete hardware-calibrated DPI steps (`200, 400, 600, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4800, 6400, 8000, 10000, 12000`).
- **Stage 6 Firmware Lockup Safeguard**: Clamps active stage mask (`active_mask & 0xDF`) by default to prevent physical button click lockups caused by stock vendor firmware bugs.
- Independent RGB indicator color picker for each active DPI stage.

### 3. Visual Live LED Studio
- Real-time illuminated mouse chassis simulator:
  - Accurately illuminates the lower E-DRA palm dragon logo (`0.690..0.812 * h`, centered at `0.751 * h`).
  - Contours the dual lateral side LED light strips running along the chassis flanks and rear skirt.
- Supports all 7 authentic hardware lighting modes:
  - `Mode 0`: DPI Breathing
  - `Mode 1`: Multicolor Breathing
  - `Mode 2`: Rainbow / Steady Neon
  - `Mode 3`: Floe Light (Flowing wave)
  - `Mode 4`: Waltz
  - `Mode 5`: Four Seasons
  - `Mode 6`: LED OFF
- Live breathing speed slider (1..8) and brightness toggle (100% / 50%).

### 4. Hardware Telemetry Cockpit
- Real-time sensor polling rate telemetry and jitter waveform (500Hz/1000Hz target baseline).
- Switch debounce and click latency diagnostic test pad with millisecond contact duration tracking.
- Rolling Clicks Per Second (CPS) counter.
- **Start / Stop Telemetry** controls with live status indicators (`[ACTIVE]` / `[PAUSED]`) to pause background polling.

### 5. Hardware Macro Manager
- 1:1 compiler, disassembler, and bank manager matching Delphi `0x0058eccc` / `0x0058ea38` routines.
- Directly compiles and stores macro sequences into the hardware EEPROM buffer (`0x01F0..0x03EF`).

### 6. Mode Controller
- Instant mode toggle between **Office Mode** (`0x00`) and **Gaming Mode** (`0x01`) with automatic dual button table synchronization to ensure buttons never become unresponsive.

### 7. App Settings & Customization
- High-contrast themes: **Pitch Black OLED**, **Windows 11 Dark**, and **High-Contrast White**.
- Bilingual localization: Full **English** and **Tiếng Việt** dictionary.
- Dynamic typography: Download and cache Google Fonts and Fontshare typefaces locally.
- Quick Access Folders: One-click shortcuts to open the executable folder, data directory, and cache directory.

---

## Reverse-Engineered Hardware Protocols

### 1. USB HID Feature Reports (Interface 1, Col 04)
- **Feature Report `0x40` (Control Dispatcher, 8 bytes)**:
  - `0x40 0x07 [offset_lsb, offset_msb, len_lsb, len_msb, 0, 0]`: Prepares 2048-byte EEPROM write.
  - `0x40 0x09 [offset_lsb, offset_msb, len_lsb, len_msb, 0, 0]`: Prepares direct register or profile read.
  - `0x40 0x0D [0...]`: Live soft-apply (applies RAM buffer to active registers without reboot) & Chip ID query.
  - `0x40 0x0E [0...]`: EEPROM flash burn (commits RAM to persistent flash and resets USB bus).
- **Feature Report `0x41` (Data Streaming, 8 bytes)**:
  - `[0x41, B0, B1, B2, B3, B4, B5, B6]`: Transmits payload memory in 7-byte slices across 293 chunks with 4–6ms inter-packet pacing.

### 2. 5-Sector Hardware Checksum Formula
The Sunplus SPCP6651B microcontroller enforces 5 separate checksum sectors:
```
Checksum = (SUM(Sector_Bytes) ^ 0x5A) & 0xFF
```
1. **Sector 0 (`0x00A3`)**: Sum of bytes `0x0000..0x00A2` (163 bytes).
2. **Sector 1 (`0x00BF`)**: Sum of bytes `0x00A4..0x00BE` (27 bytes).
3. **Sector 2 (`0x00F0`)**: Sum of bytes `0x00C0..0x00EF` (48 bytes, Office Mode button table).
4. **Sector 3 (`0x0121`)**: Sum of bytes `0x00F1..0x0120` (48 bytes, Gaming Mode button table).
5. **Sector Footer (`0x03FF`)**: Sum of bytes `0x03F8..0x03FE` (7 bytes, hardware footer).

### 3. Sensor Orientation Safeguard
Byte `0x0006` configures the SPCP6662 optical DSP coordinate transformation. The EM624 chassis mounts the sensor at a **180° rotation** (`0x7F` or `0x40`). Setting this byte to `0x00` inverts the physical tracking and swaps X/Y axes. The driver strictly enforces `0x7F` across all profiles.

---

## Strict Resource Budget & Memory Hygiene

Unlike bloated Electron/CEF companion utilities, this driver is engineered for extreme efficiency:
- **&le; 10 MB Working Set RAM**: Stays under 10 MB throughout steady-state execution.
- **Continuous Compaction**: Automatically trims heap memory via Windows `HeapCompact` + `K32EmptyWorkingSet` + `SetProcessWorkingSetSize` or Linux `malloc_trim(0)`.
- **Texture Hygiene**: CPU-side pixel vectors from the 600x600 mouse diagram are immediately dropped after GPU texture upload.
- **Zero Web Dependencies**: Pure native Rust compiled with `opt-level = 3`, `lto = true`, `codegen-units = 1`, and `strip = true`.

---

## Installation & Download

Pre-compiled standalone binaries are available on the [Releases Page](https://github.com/AlexAn75541/em624reverse/releases):
- **Windows (x64)**: `em624-windows-x64.zip`
- **Windows on ARM (ARM64)**: `em624-windows-arm64.zip`
- **Linux (x86_64)**: `em624-linux-x64.zip`
- **Continuous Pre-Release**: Updated automatically on every commit to `main` at [pre-release](https://github.com/AlexAn75541/em624reverse/releases/tag/pre-release).

### Linux Prerequisites & Udev Setup
To access the mouse USB HID configuration interface without running as `root`, install the provided udev rule:
```bash
sudo bash scripts/install_udev.sh
sudo udevadm control --reload-rules && sudo udevadm trigger
```

---

## CLI Reference

The binary doubles as a comprehensive CLI diagnostic and management tool:

```bash
# Launch the native graphical configurator
em624-reversed

# Probe connected mouse and dump live hardware register state
em624-reversed probe

# List all detected USB HID devices and highlight the EM624 mouse
em624-reversed list

# Inspect and validate an existing profile file
em624-reversed inspect-profile exampleProfiles/DailyUse.em624

# Flash a profile directly to mouse EEPROM (with automatic button safety repair)
em624-reversed apply-profile exampleProfiles/DailyUse.em624

# Dump current 2048-byte EEPROM profile from mouse memory to a file
em624-reversed read-profile current_backup.em624

# Repair a legacy or broken profile file
em624-reversed fix-profile broken.bin -o repaired.em624

# Batch scan and repair an entire directory of profiles
em624-reversed repair-profiles exampleProfiles/

# Windows Raw Input hardware event sniffer (clicks, wheel motion, timing)
em624-reversed listen
```

---

## Building from Source

Ensure you have Rust and Cargo installed:

```bash
# Clone the repository
git clone https://github.com/AlexAn75541/em624reverse.git
cd em624reverse

# Build and run the GUI
cargo run --release

# Run the automated unit test suite
cargo test --bin em624-reversed

# Run Clippy linter
cargo clippy --all-targets -- -D warnings
```

---

## Documentation Links

- **Documentation Portal & Web Visualizer**: https://alexan75541.github.io/em624reverse/
- **Hardware Reverse-Engineering Guide**: [`AGENTS.md`](./AGENTS.md)
- **Button Bytecode & Microswitch Matrix**: [`BUTTON_SPEC.md`](./BUTTON_SPEC.md)
- **Linux Setup & Architecture**: [`LINUX.md`](./LINUX.md)
- **Migration from Stock Vendor Utility**: [`MIGRATION.md`](./MIGRATION.md)

---

## License

This project is licensed under the [MIT License](./LICENSE).

*Disclaimer: This is an independent, non-commercial open-source reverse-engineering project created for hardware preservation, interoperability, and Linux support. Not affiliated with or endorsed by E-DRA or Sunplus Innovation Technology Inc.*
