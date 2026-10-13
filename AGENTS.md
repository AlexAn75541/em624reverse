# AGENTS.md — EM624 Mouse Reverse Engineering & Driver Architecture

## 1. Project Overview & Hardware Profile

- **Device Name**: E-DRA EM624 Gaming Mouse
- **Vendor ID (VID)**: `0x1BCF` (Sunplus Innovation Technology Inc.)
- **Product ID (PID)**: `0x08B8` (EM624 Gaming Mouse)
- **Controller / Sensor**: SunplusIT SPCP6662 (6662IC) Optical Gaming Sensor + SPCP6651B MCU
- **Switches**: Huano 50M (50 million clicks rated)
- **Buttons**: 7 physical switches + 1 scroll wheel
- **Calibrated DPI Table (14 discrete steps)**:
  `[200, 400, 600, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4800, 6400, 8000, 10000]`
- **Polling Rates**: 125Hz, 250Hz, 500Hz, 1000Hz
- **USB Interface Architecture**:
  - **Interface 0 (`MI_00`)**: Standard USB HID Mouse (Endpoint `0x81` IN, 1000Hz polling rate, buttons + X/Y relative motion + wheel).
  - **Interface 1 (`MI_01`)**: Multi-collection HID containing:
    - `Col01`: HID Keyboard/Consumer controls
    - `Col02`: HID Consumer controls
    - `Col03`: HID System controls
    - `Col04` (`Usage Page 0xFF00`, `Usage 0x01`): **Vendor Configuration & Flash Interface**. All configuration reports (`0x40`, `0x41`) must target this collection exclusively.

---

## 2. Decompiled Stock Application Analysis

- **Binary**: `extracted/EM624 Mouse.exe` (Delphi PE32 GUI application)
- **Image Base**: `0x00400000`, Entry Point: `0x005b9860`
- **Main Form**: `TForm1` (Caption: `SPCP665x 005A Gaming Mouse`, DFM offset `0x3c4458`)
- **Key Method Pointers (Decompiled)**:
  - `Btn_Confirm1_1Click` (Apply / Save Settings): Address `0x0059bc8c` (File offset `0x19b08c`)
  - `Btn_DefaultClick` (Factory Reset): Address `0x0057c93c` (File offset `0x17bd3c`)
  - `FlashMemoryWriteRoutine`: Address `0x005b7c8c` (File offset `0x1b7c8c`)
  - `SendControlReport_0x40`: Address `0x005b7218` (File offset `0x1b7218`)
  - `SendDataChunkReport_0x41`: Address `0x005b728c` (File offset `0x1b728c`)
  - Factory Defaults Buffer: In-memory table at address `0x005c7cdc` (2048 bytes)

### Flash Write Assembly Workflow (`0x005b7c8c`):
1. Allocates 8-byte buffer for Report `0x40`.
2. Sends `0x40 0x07` (Start Write): sets offset `0x0000`, length `0x0800` (2048 bytes).
3. Divides 2048 bytes into 7-byte slices (`idiv 7` -> 292 full chunks + 1 chunk of 4 bytes padded with `0xFF` to 7 bytes = 293 chunks total).
4. For each chunk, formats Report `0x41` [ReportID 0x41, Byte0, Byte1, ..., Byte6] and transmits via `HidD_SetFeature`.
5. Transmits Commit `0x40 0x0E`. On receiving this opcode, the Sunplus MCU commits RAM to EEPROM and momentarily power-cycles its USB bus.

---

## 3. USB HID Protocol Specification

All configuration occurs via USB HID Feature Reports on Interface 1 / Col04.

### Feature Report 0x40: Control Dispatcher (8 bytes)
```
Byte 0: 0x40 (Report ID)
Byte 1: Opcode
Byte 2: Address Offset LSB
Byte 3: Address Offset MSB
Byte 4: Data Length LSB
Byte 5: Data Length MSB
Byte 6: Reserved (0x00)
Byte 7: Reserved (0x00)
```
- **`Opcode 0x07` (Prepare Write)**: `40 07 00 00 08 00 00 00` (Prepare 2048-byte write starting at offset 0).
- **`Opcode 0x0E` (Commit to Flash)**: `40 0e 00 00 00 00 00 00` (Burns RAM buffer into persistent EEPROM).
- **`Opcode 0x0D` (Apply / Soft Reset)**: `40 0d 00 00 00 00 00 00` (Applies settings to sensor registers without reboot).
- **`Opcode 0x09` (Prepare Read)**: `40 09 00 00 08 00 00 00` (Requests 2048-byte read starting at offset 0).

### Feature Report 0x41: Data Streaming (8 bytes)
```
Byte 0: 0x41 (Report ID)
Byte 1..7: 7-byte raw payload slice from profile memory
```
- Total chunks per 2048-byte profile: **293 chunks**.
- Inter-chunk transfer delay: 4ms to 6ms.

---

### Hardware Checksum Architecture
The Sunplus MCU validates 5 distinct checksum sectors before activating mouse button event dispatch:
1. **Sector 0 (`0x00A3`)**: Sum of bytes `0x0000..0x00A2` (163 bytes, covers Magic, Active Mask, DPI nibbles, RGB colors).
2. **Sector 1 (`0x00BF`)**: Sum of bytes `0x00A4..0x00BE` (27 bytes, mode & report control registers).
3. **Sector 2 (`0x00F0`)**: Sum of bytes `0x00C0..0x00EF` (48 bytes, Office Mode button table).
4. **Sector 3 (`0x0121`)**: Sum of bytes `0x00F1..0x0120` (48 bytes, Gaming Mode button table).
5. **Sector Footer (`0x03FF`)**: Sum of bytes `0x03F8..0x03FE` (7 bytes, hardware footer).

Algorithm for each sector:
```
checksum = (sum(sector_bytes) ^ 0x5A) & 0xFF
```
*If Sector 0 checksum at `0x00A3` fails, the MCU boots optical cursor tracking but completely shuts down physical button click dispatching.*

---

## 4. Profile Binary Format (`exampleProfiles/*.bin` and `*.em624`, 2048 Bytes)

| Offset Range | Length | Description |
|---|---|---|
| `0x0000..0x0003` | 4 bytes | Magic Hardware Signature: `1B CF 08 B8` (Little-endian VID `0x1BCF` + PID `0x08B8`) |
| `0x0004` | 1 byte | Active DPI Stages Bitmask (`0xDF` = 5 stages, `0xFF` = 6 stages) |
| `0x002C..0x002E` | 3 bytes | Packed DPI levels (Stage 1..6 nibbles: `[S1|S2], [S3|S4], [S5|S6]`) |
| `0x003C..0x0053` | 24 bytes | DPI Stage RGB Colors (4 bytes per stage: `[R, G, B, 0x00]`) |
| `0x006F..0x007F` | 17 bytes | Identifier string: `"USB Gaming Mouse\0"` |
| `0x00A0..0x0122` | 131 bytes | Button remap lookup tables & function triggers |
| `0x0400..0x07FF` | 1024 bytes | Macro sequence bytecode buffer & padding |

---

## 5. Button Mapping Architecture & Exact Checksum Formula

### Exact 12-Key Table Structure (4 Bytes per Key, 48 Bytes Table)
Decompilation of `EM624 Mouse.exe` (`0x0059bc8c`, `0x005a0000`, `0x005606a8`) revealed that button configurations are arranged as arrays of 12 keys (4 bytes each):
- **Office Mode Table**: `0x00C0..0x00EF` (48 bytes) followed by 1-byte checksum at `0x00F0`.
- **Gaming Mode Table**: `0x00F1..0x0120` (48 bytes) followed by 1-byte checksum at `0x0121`.
- **Active Mode Selector**: Offset `0x00A4` (`0x00` = Office Mode, `0x01` = Gaming Mode).

#### Key Record Layout: `[Type, Param1, Param2, Param3]`
- `0x01` (Mouse): Param1 `0x01` = Left, `0x02` = Right, `0x04` = Middle, `0x08` = Forward (Side 1), `0x10` = Backward (Side 2).
- `0x02` (Keyboard): Param1 = Modifier (`0x01`=Ctrl, `0x02`=Shift, `0x04`=Alt, `0x08`=Win), Param2 = Keycode.
- `0x03` (Media / Consumer): Param2 `0xb5` = Next Track, `0xcd` = Play/Pause, `0xe9` = Vol+, `0xea` = Vol-.
- `0x06` (CPI): Param1 `0x12` or `0x92` = CPI Loop.
- `0x09` (Fn): Param1 `0x00` = Office Mode, `0x01` = Gaming Mode, `0x02` = CPI+.

#### Exact Firmware Checksum Algorithm
The MCU validates four distinct profile sectors using the formula:
```
Checksum = (Sum_of_bytes ^ 0x5A) & 0xFF
```
1. **Sector A (`0x00A4..0x00BF`)**: Checksum stored at `0x00BF`.
2. **Office Table (`0x00C0..0x00F0`)**: Checksum stored at `0x00F0`.
3. **Gaming Table (`0x00F1..0x0121`)**: Checksum stored at `0x0121`.
4. **Footer Table (`0x03F8..0x03FF`)**: Checksum stored at `0x03FF`.

### Why Side Buttons Were Previously Mismapped
1. In `DailyUse.bin`, the side buttons were custom-bound to keyboard keys `0x4E` ('N') and `0x4B` ('K').
2. When the initial compatibility patch cloned the `DailyUse.bin` button block onto all profiles, it forced those 'N' and 'K' bindings onto every game profile, breaking standard Forward and Back.
3. The driver now provides two clean modes:
   - `--buttons standard` (default): Applies factory standard mouse actions (Left, Right, Middle, Forward Side 1, Back Side 2, CPI Loop) to both Office and Gaming modes.
   - `--buttons keep`: Preserves the profile's own authentic keybindings (e.g. Spacebar and Q for Valorant) while safely masking Stage 6 (`active_stages_mask &= 0xDF`) and verifying all four checksums.

---

## 6. Mouse Recovery Procedure (Dead Buttons / Unresponsive Input)

If the mouse buttons or scroll wheel become unresponsive after flashing a custom profile (e.g. `Valorant+CS2.bin` had macro keybind overrides at offsets `0xA0..0x122`):

### Option A: Physical Hard Reset
1. Unplug the mouse USB cable.
2. Wait 5 seconds.
3. Plug the mouse back in. The Sunplus MCU reboots from persistent flash.

### Option B: Restore Default Working Profile via CLI
Open PowerShell in the workspace and execute:
```powershell
cargo run -- apply-profile exampleProfiles/DailyUse.em624
```
*`DailyUse.bin` contains standard 1:1 mouse button mappings (Left, Right, Middle, Back, Forward, DPI Switch).*

### Option C: Restore via Official Vendor Utility
1. Launch `extracted/EM624 Mouse.exe`.
2. Click the **"Default"** button (triggers `Btn_DefaultClick` at `0x0057c93c`, writing factory defaults from `0x005c7cdc`).
3. Click **"Apply"** / **"Confirm"** (`Btn_Confirm1_1Click` at `0x0059bc8c`).

---

## 7. Rust Driver Workspace & Tooling Reference

- `Cargo.toml`: Package configuration with `hidapi`, `clap`, `serde`.
- `src/device.rs`: USB HID interface handling, endpoint discovery, feature report transfer.
- `src/profile.rs`: 2048-byte binary profile parser, button table validator (`is_button_table_valid`), and sanitization engine (`sanitize_buttons`).
- `src/main.rs`: CLI subcommands:
  - `list`: Enumerates HID devices and highlights candidate mice.
  - `inspect-profile <file.bin>`: Decodes DPI stages, colors, and validates button safety.
  - `apply-profile [--safe-buttons] <file.bin>`: Flashes profile to mouse memory (auto-repairs broken buttons by default).
  - `read-profile <out.bin>`: Reads current 2048-byte profile from mouse memory.
  - `fix-profile <file.bin> [-o out.bin]`: Repairs a profile file in place while preserving custom DPI and RGB colors.
  - `repair-profiles [dir]`: Scans an entire directory of profiles and repairs broken button tables in batch.
- `scripts/probe_windows.ps1`: Zero-dependency PowerShell script for device discovery.
- `em624.pcap`: USBPcap trace confirming USB protocol packet exchange.

---

## 8. Current Project State & Known Issues

### Verified Functionality
- [x] Official Delphi executable decompiled and analyzed (`TForm1` event handlers at `0x0059bc8c`, flash writer at `0x005b7c8c`).
- [x] Extracted 497 assets (`assets/` containing bitmaps, cursor groups, dialogs, INI, and RCDATA).
- [x] Identified Sunplus MCU VID (`0x1BCF`) and PID (`0x08B8`).
- [x] Fully mapped USB HID Feature Report protocol (`0x40` commands, `0x41` 7-byte chunk streaming).
- [x] Isolated vendor configuration interface on `Page 0xFF00, Usage 0x01` (`MI_01&Col04`).
- [x] Handled MCU EEPROM flash burn and bus reset response code (`0x0000001F`).
- [x] Decoded 2048-byte profile binary format (DPI stages, colors, active stage masks).
- [x] Successfully write custom DPI levels and custom RGB LED colors into mouse hardware.
- [x] Decoded exact 12-key button mapping tables (Office mode `0x00C0` and Gaming mode `0x00F1`).
- [x] Reverse-engineered exact Sunplus checksum algorithm: `(sum ^ 0x5a) & 0xff` across 5 sectors.
- [x] Implemented `--buttons standard` and `--buttons keep` with live profile verification.
- [x] Built Windows Raw Input hardware packet sniffer (`listen` subcommand).
- [x] Reverse-engineered EEPROM macro bytecode format and instruction set (`src/macro_engine.rs`).
- [x] Created Linux non-root udev rules installer (`scripts/install_udev.sh`).

---

## 9. Next Milestones
- [ ] Trace Lift-Off Distance (LOD) and hardware angle-snapping register offsets in SPCP6662 DSP.
- [ ] Package cross-platform AppImage bundle for Linux desktop distributions.

---

## 10. Mandatory Optimization & Resource Budget Guidelines (Every Feature Request)

Whenever new features, UI controls, hardware registers, or driver modules are added, agents and contributors must strictly enforce the following optimization standards:

### 1. Memory Budget (Strict <= 10MB Working Set Target)
- **Continuous Working Set Trimming**: The application must remain under the **10 MB Working Set** RAM limit during steady-state execution.
- **Heap Compaction**: Every major user interaction (tab switch, modal close, profile save/load, flash) and periodic background tick must invoke `crate::platform::trim_working_set()`, ensuring Windows `HeapCompact` + `K32EmptyWorkingSet` + `SetProcessWorkingSetSize` or Linux `malloc_trim(0)` actively purges unreferenced heap pages to OS standby.
- **Texture Downsampling**: Never store or load raw full-resolution image buffers into GPU/CPU memory if rendered smaller. Images must be resized to exact target pixel dimensions prior to `ColorImage` creation, and intermediate CPU vectors must be immediately dropped.
- **Zero Memory Leaks**: Prevent unbounded collections in logs, event sniffers, or macro banks. Rotate or prune buffers at strict boundaries.

### 2. Layout Scalability & Universal Responsiveness
- **Universal Scrolling**: Every tab, sub-view, and dialog must be wrapped in `egui::ScrollArea::both().auto_shrink([false, false])` or maintain natural vertical growth.
- **No Hardcoded Height Clamps**: Never clamp container panels with `allocate_ui_with_layout(..., ui.available_height())` as it causes elements (e.g. action buttons or chassis diagrams) to be clipped when display scaling (125% to 200%) or window cropping is active.
- **High-Contrast Themes**: All UI elements must maintain legible contrast across Pitch Black (OLED), Windows 11 Dark, and High-Contrast White palettes.
- **Typography Standards**: Never use decorative emoji glyphs or symbols that render as broken tofu boxes on non-standard Windows/Linux fonts. Use clean typographic labels and ASCII indicators.
- **Feature Status Transparency**: Any WIP or partially reverse-engineered feature must be explicitly badged in the window title and page heading with `[UNFINISHED]`.

### 3. Binary Size & Compiler Optimizations
- **Release Profile**: Retain `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, and `strip = true` in `Cargo.toml`. The compiled release binary must not exceed 5 MB.
- **Zero Webview Overhead**: Maintain 100% native offline Rust execution (`eframe`/`glow`). Do not add Electron, CEF, Webview2, or external JavaScript/Node runtimes to the desktop client.

### 4. Verification & Quality Gates
- **Format & Lint**: Every commit must pass `/mnt/c/Users/Aretzera/.cargo/bin/cargo.exe fmt --check` and `cargo clippy --all-targets -- -D warnings` with zero warnings.
- **Automated Tests**: Maintain unit tests covering checksum formulas, packet framing, and macro bytecodes.
- **Cross-Platform CI**: Ensure both Windows and Linux GitHub Actions runners complete with green build and packaging checks.
