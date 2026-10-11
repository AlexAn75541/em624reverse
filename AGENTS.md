# AGENTS.md — EM624 Mouse Reverse Engineering & Driver Architecture

## 1. Project Overview & Hardware Profile

- **Device Name**: Dareu / Sunplus EM624 Gaming Mouse
- **Vendor ID (VID)**: `0x1BCF` (Sunplus Innovation Technology Inc.)
- **Product ID (PID)**: `0x08B8` (EM624 Gaming Mouse)
- **Controller / Sensor**: SunplusIT SPCP6651B MCU & Optical Gaming Sensor
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

## 4. Profile Binary Format (`profiles/*.bin`, 2048 Bytes)

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

## 5. Button Mapping Architecture & Root Cause of Dead Buttons

### Root Cause Analysis
Binary differential analysis between working (`DailyUse.bin`) and non-working profiles (`Valorant+CS2.bin`, `Bodycam.bin`, `LN.bin`, etc.) revealed:
1. **Identical Hardware Payload**: All profiles share 2005 out of 2048 identical bytes. The macro bytecode bank (`0x0400..0x0800`, 1024 bytes) is 100% identical across all files.
2. **Button Table Scrambling**:
   - `0x00C0..0x00EF`: Office mode button table (Key1 to Key6, 8 bytes each).
   - `0x00F0..0x011F`: Gaming mode button table (Key1 to Key6, 8 bytes each).
   - `0x0120..0x0122`: Mode identifier and checksum.
   - In `DailyUse.bin`, Key1 (`0x00C0`) is mapped to standard left click (`01 01 00 00`), Key2 (`0x00C8`) to right click (`01 02 00 00`), Key3 (`0x00D0`) to middle click (`01 04 00 00`), and Key8 (`0x00E0`) to DPI loop (`08 00 00 00`).
   - In non-working profiles, custom key assignments or macro triggers (e.g. `01 08 00 02`, `01 10 00 00`, `09 01 00 00`) pointed to unpopulated macro sequences. The Sunplus MCU firmware dropped all physical click events because it was trying to evaluate non-existent macro tables.
3. **Sensor Motion Independence**: Optical tracking and USB cursor packets (`MI_00` / EP `0x81`) run directly off the sensor hardware DSP, so cursor movement continued working even while the MCU button handler was dead.

### The Fix: Invariant Button Sanitization
All valid custom profiles are differentiated **ONLY** by:
- Offset `0x0004`: Active DPI stages mask (e.g. `0xDF` = 5 stages, `0xFF` = 6 stages).
- Offsets `0x002C..0x002E`: Custom DPI level stages (e.g. 1200, 1600, 2000, 3600, 4800, 5200 DPI).
- Offsets `0x003C..0x0053`: Custom RGB colors per DPI stage.

The driver now enforces the **Verified Standard Button Template** (`VERIFIED_BUTTON_BLOCK` at `0x00A0..0x0128` and `VERIFIED_FOOTER` at `0x03F8..0x0400`), guaranteeing that:
- Left, Right, Middle, Scroll, Forward, Back, and DPI Loop always function.
- All user-configured DPI levels and RGB LED color stages are 100% preserved.

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
cargo run -- apply-profile profiles/DailyUse.bin
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

### Known Issue: Button & Side Button Mapping
- **Observed Behavior**: While profile flashing succeeds and DPI/RGB stages update, custom button mappings (particularly the two side thumb buttons: Forward/Back, and certain secondary buttons) are not properly mapped as expected when switching between different game profiles.
- **Root Cause & Technical Detail**: The stock Delphi application manages multiple button state tables simultaneously (`Office Mode` at `0x00C0..0x00EF`, `Gaming Mode` at `0x00F0..0x011F`, and action masks). Splicing with the `DailyUse.bin` baseline protects physical clicks from being disabled, but it sets default standard button actions rather than each profile's tailored side button bindings.
- **Future Milestone**: Reverse engineer the full button bytecode dispatch matrix from `Skin.ini` and `0x0059bc8c` to support arbitrary button remapping and macro key assignments without freezing physical input.

---

## 9. Next Milestones
- [ ] Refine 6-button mapping matrix (Left, Right, Middle, Forward, Back, DPI Cycle) for full custom keybinds.
- [ ] Build zero-install WebHID web UI (HTML5/WebHID) for browser-based configuration.
- [ ] Package cross-platform Tauri v2 desktop application with system tray profile switching.
