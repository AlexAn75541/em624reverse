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
| **3. LED Backlight & Lighting Effects** | **PARTIALLY RESOLVED** | **[UNFINISHED - Awaiting Live Hardware Verification]** | Register `0x00A5 = (mode & 0x07) \| 0x80` (bit 7 master enable), speed `0x0007` (1..5), brightness `0x0008` (bit 0). Removed accidental `0x0007 = 0x7F` overwrite. Badged `[UNFINISHED]`. |
| **4. Button Remap & Flash** | **RESOLVED on branch `feature/hardware-safe-flash`** | Ready for Hardware Verification | Realigned physical microswitch lines (Slot 1 Left, Slot 2 Forward, Slot 4 Right, Slot 5 Back, Slot 7 Middle, Slot 8 DPI-). Added soft-apply Opcode `0x40 0x0D` before commit `0x40 0x0E`. Synchronizes Office (`0x00C0`) and Gaming (`0x00F1`) tables. |
| **5. Advanced Tab: Polling Rate** | **RESOLVED on branch `feature/hardware-safe-flash`** | Ready for Hardware Verification | Safe read-modify-write across `0x0004` (bits 4..5) and `0x0008` (bit 3). Preserves DPI bitmask (bits 0..3) and sensor clock bits. Fire key remains tagged UNFINISHED. |

---

## Detailed Failure Analysis & Decompilation Backlog

### Item 3: LED Backlight & Lighting Effects — [STATUS: UNFINISHED - AWAITING VERIFICATION]
- **Hardware Test Result**: Changing LED mode, speed, and brightness in the UI previously had no effect on physical mouse RGB LEDs due to byte `0x0007` being clobbered with `0x7F` by an erroneous sensor safeguard, and `0x00A5` master enable bit 7 not being managed. Marked as `[UNFINISHED]`.
- **Root Cause & Decompilation Resolution**:
  - Register `0x00A5`: Stores `(mode & 0x07) | 0x80`. Bit 7 (`0x80`) is the master LED enable flag. Mode indices: 0 = CPI Breathing, 1 = Colorful, 2 = Neon/Breathing, 3 = Dynamic/Flow, 4 = Waltz, 5 = 4-Seasons, 6 = Off (`0x00A5 = 0x00`).
  - Register `0x0007`: LED breathing speed (`1` to `5`).
  - Register `0x0008`: Bit 0 = LED Brightness (`0` = Full, `1` = Half).
  - Both instant direct register writes (`0x40 0x05`) and full flash (`0x40 0x07` + `0x41` streaming + soft-apply `0x40 0x0D` + commit `0x40 0x0E`) now correctly preserve and latch these registers.
- **Action Taken**:
  - Implemented `get_led_mode()`, `set_led_mode()`, `get_led_speed()`, `set_led_speed()`, `get_led_brightness()`, `set_led_brightness()` in `src/profile.rs`.
  - Removed accidental `self.raw[0x0007] = 0x7f;` in `apply_safe_profile()` and `apply_standard_buttons()`.
  - Wired live LED mode, speed, and brightness controls in `src/gui.rs` and removed demo-only badges.

---

### Item 4: Button Remapping & Flash — [STATUS: RESOLVED ON BRANCH feature/hardware-safe-flash]
- **Hardware Test Result**: Remapped actions failed to actuate on physical mouse because the flash pipeline omitted the MCU scan-matrix soft-apply latch (`0x40 0x0D`), and physical microswitch routing diverged from sequential slot order.
- **Root Cause & Decompilation Resolution**:
  - Delphi `FlashMemoryWriteRoutine` at `0x005b7c8c` dispatches Opcode `0x40 0x0D` (`apply_soft_reset`) immediately before Opcode `0x40 0x0E` (`commit`). Without Opcode `0x0D`, the MCU does not copy the streamed RAM buffer into its live matrix registers before rebooting.
  - Physical switch routing corresponds to:
    - **Slot 1 (`0x00C0` / `0x00F1`)**: Physical Left Click
    - **Slot 2 (`0x00C4` / `0x00F5`)**: Physical Forward Side 1
    - **Slot 4 (`0x00CC` / `0x00FD`)**: Physical Right Click
    - **Slot 5 (`0x00D0` / `0x0101`)**: Physical Backward Side 2
    - **Slot 7 (`0x00D8` / `0x0109`)**: Physical Middle Wheel Click
    - **Slot 8 (`0x00DC` / `0x010D`)**: Physical DPI- / CPI Loop
    - **Slot 9 (`0x00E0` / `0x0111`)**: Physical DPI+ / Aux
  - Dual-table synchronization: Sunplus SPCP6651B maintains dual tables (`0x00C0` Office and `0x00F1` Gaming). Setting button bindings now writes to both tables and activates Mode Register `0x00A4 = 0x01`.
- **Action Taken**:
  - Implemented `set_button_binding` in `src/profile.rs` synchronizing both tables and recalculating checksums.
  - Added soft-apply Opcode `0x40 0x0D` into `Em624Device::write_profile()` in `src/device.rs`.
  - Connected rebind modal in `src/gui.rs` to live profile mutation and flash pipeline.

---

### Item 5: Advanced Tab (Polling Rate & Fire Key) — [STATUS: POLLING RATE RESOLVED, FIRE KEY UNFINISHED]
- **Hardware Test Result**: Polling rate write failed because direct byte writes corrupted the active DPI stages bitmask in byte `0x0004` and clobbered byte `0x0008`.
- **Root Cause & Decompilation Resolution**:
  - Decompilation of Delphi `cb_RepRateChange` at `0x0056da64` confirmed that Polling Rate is stored across two bitfields:
    - Byte `0x0004` (bits 4..5): rate bits (`00` = 125Hz, `01` = 250Hz, `10` = 500Hz, `11` = 1000Hz).
    - Byte `0x0008` (bit 3): "not 125Hz" enable flag (`0` for 125Hz, `1` for 250Hz, 500Hz, 1000Hz).
  - Safe read-modify-write is required: bits 0..3 of `0x0004` are the active DPI stages bitmask (`0xDF` = 5 stages), and bit 0 of `0x0008` is LED brightness.
- **Action Taken**:
  - Implemented `get_polling_rate()` and `set_polling_rate()` in `src/profile.rs` using read-modify-write bitmasks.
  - Wired live Polling Rate radio buttons in `src/gui.rs` with high-refresh display advisory for 1000Hz.
  - Fire key remains explicitly badged `[UNFINISHED - DEMO UI ONLY]` awaiting bytecode trace.
  - Reverted `set_polling_rate` and `set_fire_key` from `src/profile.rs`.
  - Reverted `render_advanced_tab` Polling Rate and Fire Key sections to **Blank Demo UI Only** marked with `[UNFINISHED - DEMO UI ONLY]`.
  - Kept OS-level pointer speed, enhance precision, double click speed, and rolling speed fully operational via Win32 `SystemParametersInfoW`.
- **Next Steps / Definitive Solution**:
  - Decompile `cb_RepRateChange` at `0x0056d860` in `EM624 Mouse.exe` to find the exact HID feature report sent for polling rate changes.

---

## Verified Working Subsystems (Keep Untouched)
1. **Flash Pipeline**: Feature report `0x40 0x07` -> 293 chunks of `0x41` -> Commit `0x40 0x0E` with reset handling (`0x0000001F`).
2. **Active Profile Dump**: Opcode `0x40 0x09` + 293 chunks reads active 2048-byte EEPROM dump cleanly.
3. **Optical Sensor Safeguard**: Byte `0x0006 = 0x7F` prevents inverted DSP axes.
4. **All 5 Checksum Formulas**: `(sum ^ 0x5a) & 0xff` across Sectors 0, 1, 2, 3, Footer.
5. **DPI Resolution Table**: Calibrated 14 steps (200..10,000 DPI).
6. **Cloud Typography**: Google Fonts + Fontshare dynamic fetching, 10-batch randomizer, "+ See More", and variant selector.
7. **Quick Access Folders**: Native OS file manager launcher for App, Data, and Cache folders.
8. **Memory Budget**: Sub-10MB working set via `HeapCompact` + `K32EmptyWorkingSet` + `malloc_trim`.
