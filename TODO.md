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
| **3. LED Backlight & Lighting Effects** | **FAILED on Hardware** | **REVERTED to Blank Demo UI Only** | Writing bytes `0x0018`, `0x0007`, `0x0008` did not change physical RGB LEDs on mouse. Likely uses dedicated single-byte command reports (`0x5b7f8c`). |
| **4. Button Remap & Flash** | **FAILED on Hardware** | **REVERTED to Blank Demo UI Only** | Remapped actions failed to actuate on physical mouse. Physical switch lines do not match linear array indices (Slot 4 is physical Right click, Slot 2 is Side 1). |
| **5. Advanced Tab: Polling Rate & Fire Key** | **FAILED on Hardware** | **REVERTED to Blank Demo UI Only** | Polling rate byte 4 bits 5..4 and 0x00A8 failed to alter USB reporting rate. Fire key bytecode at 0x0130 failed to actuate. |

---

## Detailed Failure Analysis & Decompilation Backlog

### Item 3: LED Backlight & Lighting Effects — [STATUS: FAILED -> BLANK DEMO UI]
- **Hardware Test Result**: Changing LED mode, speed, and brightness in the UI and flashing to mouse had **no effect on physical mouse RGB LEDs**.
- **Root Cause Analysis**:
  - The stock Delphi application has two separate routines:
    1. Bulk EEPROM flash routine (`0x005b7c8c`).
    2. Real-time LED feature command dispatch (`0x005b7f8c`).
  - Physical RGB LED animation is driven by a secondary dedicated controller register that is not triggered by bulk profile flash alone.
- **Action Taken**:
  - Reverted all profile mutation codes in `src/profile.rs` (`set_led_mode`, `set_led_speed`, `set_led_brightness`).
  - Removed LED register writes from `sync_settings_to_profile` in `src/gui.rs`.
  - Reverted `render_led_tab` to a **Blank Demo UI Only** marked with `[UNFINISHED - DEMO UI ONLY]`.
- **Next Steps / Definitive Solution**:
  - Capture USB packets using Wireshark / USBPcap while clicking LED radio buttons in official `EM624 Mouse.exe`.
  - Identify the exact single-packet HID Feature Report sent to `Page 0xFF00, Usage 0x01` on LED change.

---

### Item 4: Button Remapping & Flash — [STATUS: FAILED -> BLANK DEMO UI]
- **Hardware Test Result**: Remapping button actions in the GUI and clicking "Flash to Mouse" **failed to trigger physical actions on the mouse**.
- **Root Cause Analysis**:
  - Analysis of raw live mouse EEPROM dump (`em624/current_dump.em624`) compared with `DailyUse.bin` revealed that physical hardware microswitch lines **do not follow sequential array ordering**:
    - **Slot 1 (`0x00C0` / `0x00F1`)**: Physical Left Click (`01 01 00 00`)
    - **Slot 2 (`0x00C4` / `0x00F5`)**: Physical Forward Side 1 (`02 0a 16 00`)
    - **Slot 3 (`0x00C8` / `0x00F9`)**: Physical DPI Loop Switch (`06 12 00 1e`)
    - **Slot 4 (`0x00CC` / `0x00FD`)**: Physical Right Click (`01 02 00 00`)
    - **Slot 5 (`0x00D0` / `0x0101`)**: Physical Backward Side 2 (`02 08 07 00`)
    - **Slot 6 (`0x00D4` / `0x0105`)**: Physical Rapid Fire Key (`07 80 00 00`)
    - **Slot 7 (`0x00D8` / `0x0109`)**: Physical Middle Wheel Click (`01 04 00 00`)
  - The previous UI assumed Slot 2 was Right Click, Slot 3 was Middle Click, Slot 4 was Forward, and Slot 5 was Backward.
  - Editing Slot 4 in the GUI actually overwrote the physical Right Click microswitch, and editing Slot 2 overwrote Side 1.
  - In addition, custom keyboard keycodes failed to actuate on Interface 1 `Col01` (Keyboard).
- **Action Taken**:
  - Reverted button rebind application in `render_rebind_modal` so it no longer writes to `self.profile.raw`.
  - Replaced rebind application with a notice that rebind is currently Demo UI only.
  - Added `[UNFINISHED - DEMO UI ONLY]` badge to rebind modal.
- **Next Steps / Definitive Solution**:
  - Remap GUI slot indices to the exact physical hardware microswitch lines identified above.
  - Sniff USB packet stream during a rebind in official `EM624 Mouse.exe` to verify if an additional activation command (e.g. `0x40 0x0A`) is required to bind physical switches.

---

### Item 5: Advanced Tab (Polling Rate & Fire Key) — [STATUS: FAILED -> BLANK DEMO UI]
- **Hardware Test Result**: Selecting 125Hz, 250Hz, 500Hz, or 1000Hz did not change physical mouse report rate; Fire Key parameters had no effect.
- **Root Cause Analysis**:
  - Polling rate: Writing bits 5..4 of byte `0x0004` collided with the active DPI stages bitmask (`0xDF`), and byte `0x00A8` is an active button mask (`0x0F`), not a report rate selector. Setting `0x00A8 = 0x01` masked out button lines.
  - Fire key: Writing bytecode to `0x0130` did not execute on hardware because Button 7 was not properly bound to fire key trigger routine.
- **Action Taken**:
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
