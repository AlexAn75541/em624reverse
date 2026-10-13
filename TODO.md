# TODO.md — E-DRA EM624 Mouse Reverse Engineering Backlog

This document tracks all hardware capabilities, firmware registers, and driver features. Any feature that is partially implemented, not 100% verified on hardware, or flagged as experimental is explicitly documented below.

---

## 1. Live Mode Selection (Gaming Mode vs Office Mode) — [UNFINISHED / EXPERIMENTAL]
- **Current State**:
  - Profile bytes `0x00A4` (`0x01` = Gaming, `0x00` = Office), `0x00C0..0x00EF` (Office button table), and `0x00F1..0x0120` (Gaming button table) are parsed and editable.
  - An experimental toggle has been added to App Settings to hide/show the bottom toolbar mode switcher (`Select Mode: [Gaming Mode]` / `Select Mode: [Office Mode]`).
- **Why Unfinished**:
  - In stock vendor software, clicking "Apply" sends a full profile flash (`0x40 0x07` + 293 chunks + `0x40 0x0E`), causing the Sunplus MCU to power-cycle its USB bus (`0x0000001F`).
  - Switching mode live over USB without power-cycling (e.g. sending soft-reset opcode `0x40 0x0D`) does not always switch active button matrices in MCU RAM if the active profile was burned with the opposite mode selector.
- **Needed Steps**:
  1. Capture USB bus packets during official app mode switching to determine if a separate control report (e.g. `0x40 0x05` or `0x40 0x0B`) switches active table pointers in RAM without flashing.
  2. Implement reliable table switching verification.

---

## 2. Stage 6 DPI Firmware Deadlock & Sensor Clamping — [UNFINISHED / EXPERIMENTAL]
- **Current State**:
  - Clamped to safe 5 stages (`active_stages_mask &= 0xDF`) by default.
  - An experimental checkbox in App Settings allows bypassing the clamp for hardware register research.
- **Why Unfinished**:
  - Enabling bit 5 in byte `0x04` (`0xFF`) triggers a firmware trap in the Sunplus SPCP6651B MCU when cycling DPI to Stage 6, suspending physical button click interrupts until USB replug.
- **Needed Steps**:
  1. Determine why the MCU sensor register lookup fails on step 6: check whether register offset `0x002F` holds a 7th/8th nibble or if a sensor initialization command is required.
  2. Test if modifying Stage 6 RGB color or DPI nibbles allows stable 6-stage cycling.

---

## 3. Advanced Pointer & Fire Key Hardware Registers — [UNFINISHED]
- **Current State**:
  - UI exposes Fire Mode (1..3), Repeat Count (1..20), Interval (10..100ms), Continuous Fire (bytes `0x0010..0x0013`).
  - UI exposes Pointer Moving Speed (1..11), Double Click Speed, Rolling Speed (bytes `0x0014..0x0017`).
- **Undetermined**:
  - Whether pointer moving speed (1..11) mutates internal sensor DSP motion counts or relies on Windows `SystemParametersInfo(SPI_SETMOUSESPEED)` at runtime.
  - Whether continuous fire requires a background timer thread on the host OS or executes autonomously in the SPCP6651B MCU.
- **Needed Steps**:
  1. Probe mouse motion packet deltas over USB while varying byte `0x0014` with OS mouse acceleration disabled.
  2. Measure rapid fire click interval timestamps from USB HID Input Reports (`MI_00`) to confirm on-chip hardware timer generation.

---

## 4. Custom LED Lighting Curves & Color Palettes — [UNFINISHED]
- **Current State**:
  - 7 preset LED modes (`Breathing`, `Colorful Light`, `Rainbow`, `Flow Light`, `Waltz`, `4-Seasons`, `Off`), speed slider (1..5), and brightness (Full/Half) mapped to bytes `0x0006..0x0008`.
- **Undetermined**:
  - Location of custom RGB color transition palettes (e.g. whether user can define a custom 7-color cycling sequence).
  - Exact millisecond transition formulas for speeds 1 through 5.
- **Needed Steps**:
  1. Disassemble the Delphi vendor `TForm_Color` color picker dialog and trace where RGB palette entries are written in profile RAM (`0x003C..0x0053` or beyond `0x0130`).
  2. Map LED pulse curve coefficients.

---

## 5. Macro Manager Execution & UI Recording Hooks — [UNFINISHED]
- **Current State**:
  - Binary opcode format (Mouse clicks, Keyboard keys, Modifiers, Delays, Loops, Terminator) reverse-engineered in `src/macro_engine.rs`.
  - Static library serialization to `SPCP6651B-Macro.bin` (27,200 bytes, 100 slots) implemented.
- **Undetermined**:
  - Live keystroke recording hook in the GUI (capturing OS keyboard/mouse events during record mode).
  - Writing macro bytecode banks directly to mouse EEPROM addresses `0x0400..0x07FF` via Feature Report `0x40 0x07` at offset `0x0400`.
- **Needed Steps**:
  1. Implement OS global input hook (e.g. `WH_KEYBOARD_LL`, `WH_MOUSE_LL` on Windows or `evdev` on Linux) inside GUI recording mode.
  2. Add dedicated EEPROM flash routine targeting offset `0x0400` with length `0x0400` (1024 bytes) for macro banks.

---

## 6. Advanced Optical Sensor DSP Tuning (SPCP6662 / 6662IC) — [UNFINISHED]
- **Current State**:
  - 15 discrete DPI steps (`200` to `12,000 DPI`) mapped via nibbles `0x002C..0x002E`.
- **Undetermined**:
  - Lift-Off Distance (LOD) sensor cutoff threshold register offset.
  - Hardware angle snapping / line prediction toggle register.
  - Independent X-axis and Y-axis asymmetric DPI scaling registers (referenced in `Skin.ini` as `Adjust_XY_Option`).
- **Needed Steps**:
  1. Trace Delphi decompiled function calls around `Adjust_XY_Option` (`0x0059b432`) to find exact memory offsets.
  2. Disassemble sensor register write sequences in `0x005b7c8c`.

---

## 7. Packaging & Desktop Distribution — [PLANNED]
- **Current State**:
  - Cross-platform GitHub Actions building clean release `.zip` archives for Windows x64 (standalone GUI `.exe` with embedded icon) and Linux x64 (standalone binary).
- **Remaining Task**:
  - Package a single-file AppImage bundle for Linux desktop distributions.
