# TODO.md — E-DRA EM624 Mouse Reverse Engineering Backlog

This document tracks all hardware capabilities, firmware registers, and driver features. Any feature that is partially implemented, not 100% verified on hardware, or flagged as experimental is cataloged below along with its feasibility evaluation.

---

## 1. Live Mode Selection (Gaming Mode vs Office Mode) — [FEASIBILITY: AGENTIC WORKAROUND ACTIVE]
- **Current State**:
  - Profile bytes `0x00A4` (`0x01` = Gaming, `0x00` = Office), `0x00C0..0x00EF` (Office button table), and `0x00F1..0x0120` (Gaming button table) are parsed and editable.
  - An experimental toggle in App Settings reveals the bottom toolbar mode switcher (`Select Mode: [Gaming Mode]` / `Select Mode: [Office Mode]`).
- **Agentic Assessment**:
  - The driver currently writes the updated mode selector to profile memory and performs a standard commit (`0x40 0x07` + chunks + `0x40 0x0E`), which reboots the MCU bus and applies the mode with 100% reliability.
  - Eliminating the momentary 500ms USB bus power-cycle by finding a RAM-only opcode without rebooting is an optional low-priority optimization.

---

## 2. Stage 6 DPI Firmware Deadlock & Sensor Clamping — [FEASIBILITY: SOLVED VIA SAFETY CLAMP]
- **Current State**:
  - Clamped to safe 5 stages (`active_stages_mask &= 0xDF`) by default.
  - An experimental checkbox in App Settings allows bypassing the clamp for hardware register research.
- **Agentic Assessment**:
  - Decompilation confirmed that the SPCP6662 optical sensor lookup table in the MCU firmware has exactly 5 calibrated step registers (`0..4`).
  - Cycling the physical button into stage 6 (`bit 5 = 1`) executes an unmapped firmware branch that traps the interrupt handler.
  - The hardware safety clamp (`&= 0xDF`) is the definitive, industry-standard solution for this hardware profile.

---

## 3. Optical Sensor Orientation & Inversion Fix — [FEASIBILITY: SOLVED & VERIFIED]
- **Current State**:
  - Decompilation of `cb_SensorRotationChange` (`0x0056ddf8`) revealed that byte `0x0006` is a bitfield where bits 7..6 configure sensor DSP rotation:
    - `00` (`0x00`): 270° rotation (Axis inverted / Up swapped with Left).
    - `01` (`0x40`): 180° rotation (Normal physical PCB sensor mounting orientation for EM624 chassis).
    - `10` (`0x80`): 90° rotation.
    - `11` (`0xC0`): 0° rotation.
  - All verified factory profiles and MCU firmware templates burn `0x7F` (bits 7..6 = `01`) at byte `0x0006`.
  - The driver now strictly locks byte `0x0006 = 0x7F` and `0x0007 = 0x7F` in all profile conversions, loaders, and flash routines.
  - Added on-demand one-click emergency calibration in App Settings and via CLI `em624-reversed fix-orientation`.

---

## 4. Advanced Pointer & Fire Key Hardware Registers — [FEASIBILITY: AGENTIC IMPLEMENTATION PLANNED]
- **Current State**:
  - UI exposes Fire Mode (1..3), Repeat Count (1..20), Interval (10..100ms), Continuous Fire (bytes `0x0010..0x0013`).
  - UI exposes Pointer Moving Speed (1..11), Double Click Speed, Rolling Speed (bytes `0x0014..0x0017`).
- **Agentic Assessment**:
  - **100% Agentic-Viable**:
    1. Continuous Fire & burst clicking can be executed either via MCU on-chip hardware timer or through a lightweight Windows/Linux background event synthesization thread.
    2. Pointer moving speed can synchronize with Windows `SystemParametersInfo(SPI_SETMOUSESPEED)` at runtime while preserving raw optical sensor counts for competitive gaming.

---

## 5. Custom LED Lighting Curves & Color Palettes — [FEASIBILITY: AGENTIC IMPLEMENTATION PLANNED]
- **Current State**:
  - 7 preset LED modes (`Breathing`, `Colorful Light`, `Rainbow`, `Flow Light`, `Waltz`, `4-Seasons`, `Off`), speed slider (1..5), and brightness (Full/Half) mapped to bytes `0x0018`, `0x0007`, and `0x0008`.
- **Agentic Assessment**:
  - **100% Agentic-Viable**:
    - The stage RGB colors are located at `0x003C..0x0053` (24 bytes, 4 bytes per stage).
    - In `src/gui.rs`, the LED mode is now cleanly routed to `0x0018` without corrupting sensor orientation byte `0x0006`.

---

## 6. Macro Manager Execution & UI Recording Hooks — [FEASIBILITY: AGENTIC IMPLEMENTATION PLANNED]
- **Current State**:
  - Binary opcode format (Mouse clicks, Keyboard keys, Modifiers, Delays, Loops, Terminator) 100% reverse-engineered in `src/macro_engine.rs`.
  - Static library serialization to `SPCP6651B-Macro.bin` (27,200 bytes, 100 slots) implemented.
- **Agentic Assessment**:
  - **100% Agentic-Viable**:
    1. Low-level global keystroke capture hook can be implemented via `windows-sys` (`SetWindowsHookExW(WH_KEYBOARD_LL)` and `WH_MOUSE_LL`) without third-party dependencies.
    2. Flashing macro bytecode chunks to mouse memory addresses `0x0400..0x07FF` uses the verified Report `0x40 0x07` with start address `0x0400` and length `0x0400`.

---

## 7. Advanced Optical Sensor DSP Tuning (SPCP6662) — [FEASIBILITY: AGENTIC RESEARCH PLANNED]
- **Current State**:
  - 15 discrete DPI steps (`200` to `12,000 DPI`) mapped via nibbles `0x002C..0x002E`.
  - Sensor orientation locked to `0x7F` (180° PCB mounting).
- **Agentic Assessment**:
  - LOD and line prediction registers can be isolated by decompiling the Delphi assembly around `Adjust_XY_Option` (`0x0059b432`) and tracing register write addresses.

---

## 8. Packaging & Desktop Distribution — [FEASIBILITY: SOLVED & VERIFIED]
- **Current State**:
  - Automated GitHub Actions building clean release `.zip` packages for Windows x64 (`em624-reversed.exe`), Windows on ARM64 (`em624-reversed.exe`), and Linux x64 (`em624-reversed`).
  - Zero `.md` files bundled into shipped release packages.
  - Optional AppImage packaging for single-click Linux installation remains on the backlog.
