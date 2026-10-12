# TODO.md — E-DRA EM624 Mouse Reverse Engineering Backlog

This document tracks all hardware capabilities, firmware registers, and driver features that have not yet been 100% determined or fully reverse-engineered from the stock Delphi binary and USB HID protocol.

---

## 1. EEPROM Macro Bytecode Engine (`0x0400..0x07FF`)
- **Current State**: Static 1024-byte macro bank and blank template (`SPCP6651B-Macro.bin`) identified. Basic single-key assignments work.
- **Undetermined**:
  - Exact binary opcodes for multi-step key sequences with custom millisecond sleep delays.
  - Bytecode format for mouse click events inside macro playback (e.g. Left Down -> Wait 50ms -> Left Up).
  - Loop execution boundary markers and memory termination tokens.
- **Target Plan**: Record custom macros in the stock app with varied key/delay patterns, dump the resulting `.bin` profiles, and construct an opcode parser/encoder in `src/profile.rs`.

---

## 2. Advanced Optical Sensor DSP Registers (SPCP6662 / 6662IC)
- **Current State**: 15 discrete DPI steps (`200` to `12,000 DPI`) mapped via nibbles `0x002C..0x002E`.
- **Undetermined**:
  - Lift-Off Distance (LOD) sensor cutoff threshold register offset.
  - Hardware angle snapping / line prediction toggle register.
  - Independent X-axis and Y-axis asymmetric DPI scaling registers (referenced in `Skin.ini` as `Adjust_XY_Option`).
- **Target Plan**: Trace Delphi function calls around `Adjust_XY_Option` (`0x0059b432`) to find the exact memory offsets in the 2048-byte payload.

---

## 3. Custom LED Lighting Curve Timing & Color Palettes
- **Current State**: 7 primary LED modes (`Breathing`, `Colorful Light`, `Rainbow`, `Flow Light`, `Waltz`, `4-Seasons`, `Off`), speed slider (1..5), and brightness (Full/Half) mapped to offsets `0x0006..0x0008`.
- **Undetermined**:
  - Custom user RGB color palettes for multi-color flow effects (whether user can specify custom 7-color cycling palettes).
  - Exact millisecond transition timing formula for breathing speed steps 1 through 5.
- **Target Plan**: Capture USB Feature Reports while tweaking custom color palettes in the vendor `TForm_Color` dialog.

---

## 4. Hardware vs OS-Level Pointer Sensitivity Sync
- **Current State**: Pointer moving speed (1..11), double click speed, and rolling speed controls exposed in GUI.
- **Undetermined**:
  - Whether pointer speed (1..11) mutates hardware DSP registers directly or simply synchronizes with Windows `SystemParametersInfo(SPI_SETMOUSESPEED)` at runtime.
- **Target Plan**: Test profile writes while observing optical DPI tracking counts to confirm whether the MCU DSP scales motion counts internally.

---

## 5. Linux Standalone Packaging
- **Current State**: Standalone native Linux binary compiled with `hidapi` and `egui`.
- **Planned**:
  - Create AppImage or Flatpak packaging for single-click desktop installation across non-Debian distributions without manual library installs.
  - Automate udev rule installation via a setup script (`scripts/install_udev.sh`).
