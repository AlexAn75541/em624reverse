# EM624 Mouse Reverse Engineering & Modern Driver Plan

## Target Overview
- **Installer Binary**: `EM624 Mouse.exe` (~2.4 MB, NSIS installer)
- **Extracted Stock Application** (`extracted/`):
  - `EM624 Mouse.exe` (~4.2 MB, 32-bit PE Delphi GUI using `HID.dll`, `SetupApi.dll`)
  - `SPCP6651B-Macro.bin` (27.2 KB uninitialized macro buffer for SunplusIT SPCP6651B MCU)
  - `uninstall.exe`
- **Hardware Profile (Reverse-Engineered)**:
  - **VID**: `0x1BCF` (Sunplus Innovation Technology Inc.)
  - **PID**: `0x08B8` (EM624 Gaming Mouse)
  - **Interface 0 (`MI_00`)**: Standard mouse input reports (EP `0x81`, movement/buttons)
  - **Interface 1 (`MI_01` / `COL04`)**: Vendor-defined configuration interface (Page `0xFF00`, Usage `0x01`)
  - **Hardware ID Signature**: Profile magic header `1B CF 08 B8` is `VID_1BCF` + `PID_08B8` in little-endian.

---

## USB HID Protocol Specification (Verified via `em624.pcap`)

### Feature Reports
| Report ID | Direction | Length | Purpose |
|---|---|---|---|
| `0x40` | Host -> Mouse | 8 bytes | Control / Command Dispatcher |
| `0x41` | Bidirectional | 8 bytes (1 ID + 7 payload) | Profile Data Stream (chunked transfer) |

### Control Command Format (Report `0x40`)
An 8-byte Feature Report formatted as:
```
[ReportID, Opcode, Offset_LSB, Offset_MSB, Len_LSB, Len_MSB, Pad0, Pad1]
```
- **`0x07` (Start Flash Write)**: `40 07 00 00 08 00 00 00` (prepare mouse memory to receive 0x0800 = 2048 bytes at offset 0)
- **`0x0E` (Commit Flash)**: `40 0e 00 00 00 00 00 00` (write RAM buffer to flash storage)
- **`0x0D` (Apply / Soft Reload)**: `40 0d 00 00 00 00 00 00` (activate settings in sensor/MCU)
- **`0x09` (Read Flash)**: `40 09 00 00 08 00 00 00` (request read of 2048 bytes from offset 0)

### Data Streaming Format (Report `0x41`)
The 2048-byte configuration profile is transferred in **293 chunks** of 7 bytes each:
- `Report ID`: `0x41`
- `Payload`: 7 bytes from the `.bin` profile
- Total transferred: 292 chunks * 7 bytes = 2044 bytes + 1 chunk of 4 remaining bytes + 3 padding bytes (`0xFF`)
- Delay between chunks: ~2ms to 4ms

---

## Decompiled Stock Application (`extracted/EM624 Mouse.exe`)
- **Framework**: Borland Delphi PE32 GUI application
- **Key Method Addresses (Decompiled)**:
  - `Btn_Confirm1_1Click` (Apply Settings): `0x0059bc8c` (File offset `0x19b08c`)
  - `Btn_DefaultClick` (Factory Reset): `0x0057c93c` (File offset `0x17bd3c`)
  - `FlashMemoryWriteRoutine`: `0x005b7c8c` (File offset `0x1b7c8c`)
  - `SendControlReport_0x40`: `0x005b7218` (File offset `0x1b7218`)
  - `SendDataChunkReport_0x41`: `0x005b728c` (File offset `0x1b728c`)
- **Why buttons/scroll died on `Valorant+CS2.bin`**:
  `Valorant+CS2.bin` had macro keybind overrides at offsets `0xA0..0x122` (bytes 163, 165, 196-199, 208-213) replacing standard mouse clicks with keyboard macro codes (`0x4E`, `0x4B`).
  **Recovery**: Unplug/replug mouse, or run `cargo run -- apply-profile profiles/DailyUse.bin`, or click Default in stock `EM624 Mouse.exe`.

---

## Profile Binary Layout (`profiles/*.bin`, 2048 bytes)
- **0x00..0x03**: Magic hardware ID signature (`1B CF 08 B8`)
- **0x04**: Active stages bitmask (e.g. `0xDF` = 5 stages, `0xFF` = 6 stages)
- **0x2C..0x2E**: Nibble-packed DPI level indexes for stages 1 to 6
- **0x3C..0x6B**: DPI stage RGB LED colors (3-byte RGB per stage, 4-byte aligned)
- **0x6F..0x7F**: Device name string (`USB Gaming Mouse\0`)
- **0xA0..0x122**: Button bindings and macro lookup tables
- **0x400..0x7FF**: Macro sequence bytecode and padding

---

## Workspace Structure
- `Cargo.toml`: Cross-platform Rust driver configuration (`hidapi`)
- `src/device.rs`: Device handle, feature report read/write protocol implementation
- `src/profile.rs`: 2048-byte binary profile parser & validator
- `src/main.rs`: CLI interface (`list`, `inspect-profile`, `apply-profile`, `read-profile`)
- `.github/workflows/ci.yml`: GitHub Actions matrix CI for Ubuntu and Windows
- `scripts/probe_windows.ps1`: Zero-dependency PowerShell device locator
- `em624.pcap`: Captured USBPcap trace confirming full read/write protocol

---

## Current Status & Known Issues
- **DPI & RGB Control**: 100% verified and working. Flashes cleanly to the physical mouse memory.
- **Button Mapping Issue**: While profile flashing succeeds without locking the mouse, custom button bindings (particularly side thumb buttons: Forward/Back, and profile-specific macros) are not yet accurately mapped in custom profiles.
- **Root Cause**: The original Delphi software utilizes mode-dependent multi-sector button dispatch tables (`0x00C0..0x014F`). Re-serializing using default baselines prevents mouse locks but requires an exact map for side-button definitions.

---

## Next Steps
1. Map exact 6-button bytecode table (Left, Right, Middle, Forward, Back, CPI loop) for granular custom key assignments.
2. Build lightweight WebHID web application for browser-based, zero-install configuration.
3. Package Tauri v2 desktop application for system tray integration and auto profile switching.
