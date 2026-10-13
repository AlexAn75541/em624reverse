# TODO.md — E-DRA EM624 Mouse Reverse Engineering Backlog

This document tracks all hardware capabilities, firmware registers, and driver features.

---

## 1. EEPROM Macro Bytecode Engine (`0x0400..0x07FF`) — [SOLVED & IMPLEMENTED]
- **Status**: 100% Reverse-Engineered & Implemented in `src/macro_engine.rs`.
- **Reversed File Specification**:
  - `SPCP6651B-Macro.bin`: 27,200 bytes total, 100 macro slots (0..99).
  - Slot structure: 16 bytes UTF-8 name + 256 bytes bytecode sequence = 272 bytes per slot.
- **Opcode Instruction Set**:
  - `0xA1` / `0x21`: Mouse Left Click Down / Up
  - `0xA2` / `0x22`: Mouse Right Click Down / Up
  - `0xA4` / `0x24`: Mouse Middle Click Down / Up
  - `0xA8` / `0x28`: Mouse Forward (Side 1) Down / Up
  - `0xB0` / `0x30`: Mouse Backward (Side 2) Down / Up
  - `[0x82, KeyCode]` / `[0x02, KeyCode]`: Keyboard Key Down / Up
  - `[0x81, Modifier]` / `[0x01, Modifier]`: Modifier Down / Up (`0x01`=Ctrl, `0x02`=Shift, `0x04`=Alt, `0x08`=Win)
  - `[0xFE, MSB, LSB]`: Delay in milliseconds (`(MSB << 8) | LSB`)
  - `[0xFD, Count]`: Loop Repeat Count
  - `0xFC` / `0x00`: Sequence Terminator

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

## 5. Linux Standalone Packaging & Permissions — [SOLVED & IMPLEMENTED]
- **Status**: Standalone binary compiled for Linux, and `scripts/install_udev.sh` provided.
- **Udev Rule**: Grants non-root write access to `hidraw*` and `usb` devices with `1bcf:08b8`.
- **Remaining Task**: Optional single-file AppImage bundle packaging.

---

## 6. Zero-Install WebHID Configurator — [SOLVED & IMPLEMENTED]
- **Status**: Fully implemented in `web/index.html`.
- Allows configuring DPI stages, polling rate, LED backlight modes, and flashing EEPROM over Chrome/Edge WebHID without installing any software or runtime.
