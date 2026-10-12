# EM624 Mouse Reverse Engineering — Session Notes & Technical Log

## 1. Initial Target Identification
- **Target File**: `EM624 Mouse.exe` (2.4 MB installer).
- **Format**: NSIS (Nullsoft Scriptable Install System) archive containing:
  - `EM624 Mouse.exe` (4.2 MB PE32 Delphi executable)
  - `SPCP6651B-Macro.bin` (27.2 KB uninitialized macro buffer)
  - `uninstall.exe`
- **Hardware Architecture**:
  - Controller: Sunplus Innovation Technology (SunplusIT) SPCP6651B mouse MCU.
  - Profiles format: 2048-byte `.bin` files (`0x800` bytes).

---

## 2. Hardware Discovery & USB Enumeration
Using `scripts/probe_windows.ps1` and Windows PnP inspection:
- **Vendor ID (VID)**: `0x1BCF`
- **Product ID (PID)**: `0x08B8`
- **Interfaces**:
  - `MI_00`: Primary mouse endpoint (`HID-compliant mouse`, Endpoint `0x81`, 1000 Hz report rate).
  - `MI_01`: Composite HID interface with 4 collections:
    - `Col01`: Consumer controls
    - `Col02`: Consumer controls
    - `Col03`: System controller
    - `Col04`: Vendor-defined configuration endpoint (`Usage Page 0xFF00`, `Usage 0x01`).
- **Signature Discovery**: The first 4 bytes of every valid profile `.bin` file are `1B CF 08 B8`, which is the little-endian representation of `VID 0x1BCF` + `PID 0x08B8`.

---

## 3. Protocol Reverse Engineering (`em624.pcap`)
Captured USB traffic using USBPcap and Wireshark while interacting with the official utility.

### Protocol Summary:
- Transfers occur strictly over **HID Feature Reports** on `MI_01` / `Col04`.
- **Control Report `0x40`** (8 bytes): `[0x40, Opcode, Offset_LSB, Offset_MSB, Len_LSB, Len_MSB, 0x00, 0x00]`
  - `0x07`: Prepare Write (sets target memory offset `0x0000`, length `0x0800` = 2048 bytes).
  - `0x0E`: Commit Write (signals MCU to burn RAM into internal EEPROM).
  - `0x0D`: Soft Apply / Reset.
  - `0x09`: Prepare Read (requests 2048 bytes from offset `0x0000`).
- **Data Report `0x41`** (8 bytes): `[0x41, Byte0, Byte1, ..., Byte6]`
  - Streams 2048 bytes across 293 chunks (7 bytes payload per chunk).
  - Total: 292 chunks * 7 bytes = 2044 bytes + 1 chunk of 4 bytes padded with `0xFF`.
  - Required inter-chunk delay: ~4ms to 6ms.

---

## 4. Decompilation of `EM624 Mouse.exe`
Decompiled the stock application (`extracted/EM624 Mouse.exe`) using PE disassembler and Delphi RTTI analysis:
- **Framework**: Borland Delphi PE32 GUI.
- **Main Window**: `TForm1` (`SPCP665x 005A Gaming Mouse`, DFM offset `0x3c4458`).
- **Key Method Addresses**:
  - `Btn_Confirm1_1Click` (Apply Settings): VMA `0x0059bc8c` (File offset `0x19b08c`)
  - `Btn_DefaultClick` (Factory Reset): VMA `0x0057c93c` (File offset `0x17bd3c`)
  - `FlashMemoryWriteRoutine`: VMA `0x005b7c8c` (File offset `0x1b7c8c`)
  - `SendControlReport_0x40`: VMA `0x005b7218` (File offset `0x1b7218`)
  - `SendDataChunkReport_0x41`: VMA `0x005b728c` (File offset `0x1b728c`)
- **Extracted Assets**: Extracted 497 assets (bitmaps, cursor groups, dialogs, INI, strings) into `assets/`.

---

## 5. Profile Binary Format & Analysis
All `.bin` profiles are exactly 2048 bytes:
- `0x0000..0x0003`: Hardware Signature `1B CF 08 B8`.
- `0x0004`: Active Stages Bitmask (`0xDF` = 5 stages, `0xFF` = 6 stages).
- `0x002C..0x002E`: Nibble-packed DPI level indices for stages 1 to 6.
- `0x003C..0x0053`: RGB colors per stage (`[R, G, B, 0x00]`).
- `0x006F..0x007F`: Device string `"USB Gaming Mouse\0"`.
- `0x00A0..0x0122`: Button mapping definitions across multiple modes (Office Mode, Gaming Mode).
- `0x0400..0x07FF`: Macro sequence bytecode and padding (1024 bytes).

---

## 6. Challenges & Solutions Encountered During Session

### Issue 1: Windows HID Error `0x00000001 (Incorrect function)`
- **Cause**: `hidapi` defaulted to opening `MI_01&Col03` (system controller), which rejects Feature Report `0x40`.
- **Fix**: Filtered device enumeration in `src/device.rs` to strictly require `Usage Page 0xFF00` and `Usage 0x01` (`Col04`).

### Issue 2: Flash Commit Error `0x0000001F (A device attached to the system is not functioning)`
- **Cause**: Sending opcode `0x0E` causes the Sunplus MCU to flash its internal EEPROM and momentarily drop off the USB bus to reboot.
- **Fix**: Caught OS error code `0x0000001F` (`ERROR_GEN_FAILURE`) as a successful hardware acknowledgment rather than a critical failure.

### Issue 3: Unresponsive Physical Clicks on Non-DailyUse Profiles
- **Cause**: Profiles created for games (`Valorant+CS2.bin`, `Bodycam.bin`, etc.) had broken macro scancodes (`0x4E`, `0x4B`) in button sectors (`0x00C0..0x011F`) without populated macro sequences. The MCU suspended physical clicks while tracking remained active. Furthermore, enabling stage 6 (bit 5 = 1 in byte 4) triggered an internal MCU fault.
- **Fix**: Implemented `apply_og_compat()` / `sanitize_buttons()` in `src/profile.rs` to merge user custom DPI and RGB colors onto the verified working hardware baseline (`DailyUse.bin`) while masking stage 6 (`0xDF`).

### Resolution of Side Buttons & Checksum Verification
- **Cause of Side Button Discrepancy**: In `DailyUse.bin`, keys 2 and 5 were mapped to keyboard keys 'N' (`0x4E`) and 'K' (`0x4B`). When `DailyUse.bin` was previously used as a universal baseline, those bindings were unintentionally copied to other profiles.
- **Decompiled Solution**:
  - Reconstructed 12-key tables (4 bytes per key) for Office Mode (`0x00C0`) and Gaming Mode (`0x00F1`).
  - Standard Forward is `0x01, 0x08, 0x00, 0x02` (Key 2) and standard Backward is `0x01, 0x10, 0x00, 0x00` (Key 5).
  - Reverse-engineered exact Sunplus EEPROM checksum: `(sum(bytes) ^ 0x5A) & 0xFF` across Sector A (`0x00A4..0x00BF`), Office Table (`0x00C0..0x00F0`), Gaming Table (`0x00F1..0x0121`), and Footer Table (`0x03F8..0x03FF`).
- **Profile Conversion Tooling**:
  - Implemented `convert-profile` and `convert-all` subcommands in Rust CLI.
  - Implemented standalone Python converter `scripts/convert_profile.py` supporting `--buttons standard` (default) and `--buttons keep`.
  - All existing profiles in `profiles/` converted with 100% working buttons and verified checksums.

---

## 7. Workspace Tooling Implemented
- `Cargo.toml`: Rust driver crate (`hidapi`, `clap`, `serde`).
- `src/device.rs`: Low-level USB HID Feature Report protocol engine.
- `src/profile.rs`: Binary profile parser, validator, checksum calculator, and compatibility serializer.
- `src/main.rs`: CLI commands:
  - `list`: Lists connected USB devices and highlights mouse.
  - `inspect-profile <file.bin>`: Decodes DPI stages, RGB LED colors, active mode, and button assignments.
  - `apply-profile <file.bin> [--buttons standard|keep]`: Flashes profile to mouse memory.
  - `read-profile <out.bin>`: Reads current profile from mouse memory.
  - `convert-profile <file.bin> [-o out.bin] [--buttons standard|keep]`: Converts profile to guaranteed working format.
  - `convert-all [dir] [--buttons standard|keep]`: Batch converts all profiles in a directory.
- `scripts/convert_profile.py`: Standalone cross-platform profile converter with zero dependencies.
- `scripts/probe_windows.ps1`: Zero-dependency PowerShell script for device discovery.
- `.gitignore`: Configured to exclude binaries, proprietary assets, and debug dumps.
