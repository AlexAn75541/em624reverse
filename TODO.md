# TODO.md — E-DRA EM624 Mouse Reverse Engineering Backlog & Implementation Status

This document tracks all hardware capabilities, firmware registers, and driver features. Any feature that is partially implemented, not 100% verified on hardware, or flagged as experimental is cataloged below along with its reverse-engineered technical specification and definitive solution.

---

## Hardware Test Results Ledger (Physical E-DRA EM624 Hardware — Tested March 2025)

| Subsystem / Feature | Hardware Test Result | Current Status | Notes & Diagnostic Findings |
|---|---|---|---|
| **App Settings: Quick Access Folders** | **PASSED (100%)** | Active & Shipped | Native OS file explorer successfully opens App folder, Data folder, and Cache folder. |
| **Cloud Typography: Random Batch & Variants** | **PASSED (100%)** | Active & Shipped | Fetches 10 random fonts from catalog, "+ See More" appends 10 more, dynamic variant query populates and downloads exact weights. |
| **Optical Sensor Orientation & DSP Axis** | **PASSED (100%)** | Active & Shipped | Strictly locked to `0x0006 = 0x7F` (180° PCB mounting). Eliminates inverted cursor tracking. |
| **DPI Resolution Stages & Calibration** | **PASSED (100%)** | Active & Shipped | Discrete 14-step table (200..10,000 DPI) and stage colors successfully write to EEPROM. |
| **OS-Level Pointer Dynamics** | **PASSED (100%)** | Active & Shipped | Win32 `SystemParametersInfoW` pointer speed, enhance precision, double click, and rolling speed work in real-time. |
| **3. LED Backlight & Lighting Effects** | **Ghidra-Verified & Live-Flashed** | **[UNFINISHED - Awaiting Visual Verification]** | `RGBImgMode_OnClick` (`0x00589784`) & `cb_MCULEDNumChange` (`0x00571424`) prove `0x00A5 = 0x81..0x86` (bit 7=0 for OFF), LED speed step at **`0x0038`** (`[0x0A, 0x28, 0x46, 0x64, 0x82, 0xA0, 0xBE, 0xDC]`), and 3-byte `[R,G,B]` stage colors at **`0x0039..0x004A`**. Badged `[UNFINISHED]` per user instruction. |
| **4. Button Remap & Flash** | **Ghidra-Verified & Live-Flashed** | Ready for Hardware Verification | `DefaultParameterConfirm1` (`0x0059bdf8`) & `Key_AssemblyClick` (`0x005828b0`) prove keyboard remap is `[0x02, mod, hid_usage, vk_code]`, media is 16-bit LE `[0x03, lsb, msb, 0x00]`, and requires `0x40 0x05` Mode Toggle (`0x00A4 <- 0x0F -> 0x01/0x00`) to reload scan matrix. |
| **5. Advanced Tab: Polling Rate** | **Ghidra-Verified & Live-Flashed** | Ready for Hardware Verification | `cb_RepRateChange` (`0x0056d860`) & `ReportRate_OnClick` (`0x00589084`) prove `0x0004` bits 5..4 is Polling Rate (`0xCF`=125Hz, `0xDF`=250Hz, `0xEF`=500Hz, `0xFF`=1000Hz) written via `0x40 0x05`. Removed `raw[0x0004] &= 0xDF` bug that forced 250 Hz on every flash. |

---

## Detailed Root-Cause Assertions & Ghidra Decompilation Proof

### 1. Why `write_profile` Previously Failed to Latch Live MCU Registers (`0x005a02e0` vs Dead Code `0x005b7c8c`)
- **Assertion**: Function `0x005b7c8c` (`0x40 0x07 00 00 08 00` + 293 chunks of `0x41` + `0x40 0x0E`) has **zero callers** in `EM624 Mouse.exe`. On the Sunplus SPCP6651B MCU, `0x40 0x07` only writes EEPROM backing pages (`>= 0x00C0`) and does **not** update active peripheral registers (`0x0000..0x00BF`).
- **Decompiled Proof (`MemoToDataBuf_OnClick` at `0x005a02e0` & `DefaultParameterConfirm1` at `0x0059bdf8`)**:
  1. **Sector 0 (`0x0000..0x00A3`, 164 bytes)**: Written via `FUN_005b8304` using **41 packets of `0x40 0x05` (4-byte direct register writes)**.
  2. **Sector 1 (`0x00A4..0x00BF`, 28 bytes)**: Written via `FUN_005b8304` using **7 packets of `0x40 0x05` (4-byte direct register writes)**.
  3. **Button Tables & Macros (`>= 0x00C0`)**: Written via `FUN_005b888c` (`0x40 0x07` + `0x41` 7-byte chunks) in 5 exact segments: `0x00C0..0x0121` (98B), `0x0122..0x012D` (12B), `0x0130..0x01D7` (168B), `0x03F0..0x03FF` (16B), and `0x0400..0x07FF` (1024B), followed by commit header `[0x40, 0x07, 0xB4, 0x00, 0x30, 0x00, 0x01, 0x00]`.
  4. **Live Reload Toggles (`0x0059c103` & `0x005742c2`)**: Sends `0x40 0x05` Mode Toggle (`0x00A4 <- 0x0F`, then `0x00A4 <- 0x01/0x00`) to reload the 12-button table into live RAM, and `0x00A6` stage toggle to reload active CPI & RGB LED color.

### Item 3: LED Backlight & Lighting Effects — [STATUS: UNFINISHED - AWAITING VISUAL VERIFICATION]
- **Assertions (Why Previous LED Code Failed)**:
  1. **Wrong LED Speed Offset (`0x0007` vs `0x0038`)**: Previous code wrote `1..5` to `0x0007`, which is the hardware fuse register (`0x7F` in all 6 original EM624 `.bin` files: `LEDCtrl0/1`, `LEDModeFuse`, `SensorSleep`, `LEDPower` at `0x0056e1d8..0x0056e4ec`). `cb_MCULEDNumChange` (`0x00571424`) and `slider_MCULEDStepMouseUp` (`0x005b1114`) write LED speed (`[0x0A, 0x28, 0x46, 0x64, 0x82, 0xA0, 0xBE, 0xDC]`) to **Offset `0x0038`**.
  2. **Wrong Stage RGB Color Offset & Stride (`0x003C` 4-byte vs `0x0039` 3-byte)**: Previous code wrote 4-byte `[R, G, B, 0x00]` colors at `0x003C + i*4`, zeroing Stage 1's RGB bytes at `0x0039..0x003B` and shifting Stages 2..6 channels. `lbl_DPI1ColClick` (`0x00587fa4`) and all 6 original `.bin` files prove Stage 1..6 colors are **3-byte `[R, G, B]`** starting at **`0x0039`** (`0x0039..0x004A`, with Color 7 at `0x004B..0x004D`).
  3. **Hardware Fuse `0x0008` Corrupted by Brightness**: Byte `0x0008` (`0xAF` in all 6 original `.bin` files) is `EFTrelyNAK / EFTreplug / KeepProfileInfo / ProfileLEDFlash` (`0x0056e558..0x0056e69c`), not LED brightness.
  4. **LED Effect Mode (`0x00A5`)**: `RGBImgMode_OnClick` (`0x00589784`) writes `0x81..0x86` for Modes 0..5, and clears bit 7 (`raw[0x00A5] & 0x7F`) for LED OFF.
- **Action Taken**:
  - Updated `src/profile.rs` to read/write 3-byte RGB colors at `0x0039..0x004A`, LED speed at `0x0038`, LED mode at `0x00A5`, and lock hardware fuses `0x0005=0x0F`, `0x0006=0x7F`, `0x0007=0x7F`, `0x0008=0xAF`.
  - Kept LED tab badged `[UNFINISHED]` per user instruction.

---

### Item 4: Button Remapping & Flash — [STATUS: RESOLVED & LIVE-FLASHED ON BRANCH feature/hardware-safe-flash]
- **Assertions (Why Previous Button Remap Failed)**:
  1. **Keyboard Binding (`Type 0x02`) Used Windows VK Code Instead of USB HID Usage ID**: `Key_AssemblyClick` (`0x005828b0`) translates Windows VK codes through `DAT_005c7018` into `[0x02, mod_mask, hid_usage_id, vk_code]`. Previous code wrote `[0x02, mod_mask, vk_code, 0x00]`, causing the MCU to emit wrong USB HID keycodes (e.g. `'Q'` VK `0x51` -> HID Down Arrow; `'Space'` VK `0x20` -> HID `'3'`).
  2. **Media Key Binding (`Type 0x03`) Shifted 16-bit Consumer Usage by 8 Bits**: `Key_MediaClick` (`0x005814b0`) writes `[0x03, usage_lsb, usage_msb, 0x00]`. Previous code wrote `[0x03, 0x00, code, 0x00]`.
  3. **Missing `0x00A4` Mode Reload Toggle**: Added `40 05 A4 00 01 00 0F 00` -> `40 05 A4 00 01 00 <mode> 00` at the end of `write_profile` matching `DefaultParameterConfirm1` (`0x0059c103`).

---

### Item 5: Advanced Tab (Polling Rate & Fire Key) — [STATUS: POLLING RATE RESOLVED & LIVE-FLASHED, FIRE KEY UNFINISHED]
- **Assertions (Why Previous Polling Rate Failed)**:
  1. **`0x0004` Clobbered to 250 Hz (`0xDF`) on Every Flash**: `cb_RepRateChange` (`0x0056d860`) and `ReportRate_OnClick` (`0x00589084`) prove `0x0004` bits 5..4 is Polling Rate (`0xCF`=125Hz, `0xDF`=250Hz, `0xEF`=500Hz, `0xFF`=1000Hz). Previous `sanitize_buttons` and `apply_safe_profile` executed `raw[0x0004] &= 0xDF`, forcing 250 Hz on every flash!
  2. **Direct Register Write Required**: `0x0004` is in Sector 0 (`0x0000..0x00A3`) and must be written via `0x40 0x05` together with Sector 0 checksum `0x00A3`.
- **Action Taken**:
  - Removed `&= 0xDF` mask from `apply_safe_profile` and `sanitize_buttons`.
  - Verified live 1000 Hz (`0x0004 = 0xFF`) write and readback on the plugged-in mouse on Windows host.

---

## Verified Working Subsystems (Keep Untouched)
1. **Flash Pipeline (`MemoToDataBuf_OnClick` + `DefaultParameterConfirm1`)**: `0x40 0x05` for Sectors 0 & 1 (`0x0000..0x00BF`), `0x40 0x07` + `0x41` chunks for `>= 0x00C0`, commit header `40 07 B4 00 30 00 01 00`, and `0x00A4` / `0x00A6` reload toggles.
2. **Active Profile Dump**: Opcode `0x40 0x09` + 293 chunks reads active 2048-byte EEPROM dump cleanly.
3. **Optical Sensor & Hardware Fuse Safeguard**: `0x0005 = 0x0F`, `0x0006 = 0x7F` (180° PCB orientation), `0x0007 = 0x7F`, `0x0008 = 0xAF`.
4. **All 5 Checksum Formulas**: `(sum ^ 0x5a) & 0xff` across Sectors 0, 1, 2, 3, Footer.
5. **DPI Resolution Table**: Calibrated 14 steps (200..10,000 DPI).
6. **Cloud Typography**: Google Fonts + Fontshare dynamic fetching, 10-batch randomizer, "+ See More", and variant selector.
7. **Quick Access Folders**: Native OS file manager launcher for App, Data, and Cache folders.
8. **Memory Budget**: Sub-10MB working set via `HeapCompact` + `K32EmptyWorkingSet` + `malloc_trim`.
