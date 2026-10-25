# TODO.md — E-DRA EM624 Mouse Reverse Engineering Backlog & Implementation Status

This document tracks all hardware capabilities, firmware registers, and driver features. Any feature that is partially implemented, not 100% verified on hardware, or flagged as experimental is cataloged below along with its reverse-engineered technical specification and definitive solution.

---

## 1. Win32 & Linux OS-Level Mouse Dynamics (Advanced Settings Tab) — [FEASIBILITY: 100% VERIFIED]
- **Current State**:
  - UI exposes Pointer Moving Speed (1..11), Enhance Pointer Precision checkbox, Double Click Speed (1..11), Rolling Speed (1..11 lines), and Scroll One Screen at a time.
  - Profile previously wrote dummy values to offsets `0x0014..0x0017`.
- **Reverse Engineering Discovery (Delphi Binary `0x00599ce8`, `0x0059c3a0`)**:
  - Static disassembly proved that the vendor software (`EM624 Mouse.exe`) does **NOT** store pointer speeds in mouse EEPROM registers.
  - The stock software invokes Win32 API `SystemParametersInfoA`:
    - Pointer Speed: `SPI_SETMOUSESPEED` (`0x0071`) with values `1..20`.
    - Double Click Speed: `SPI_SETDOUBLECLICKTIME` (`0x0020`) with values `200ms..900ms`.
    - Rolling Speed: `SPI_SETWHEELSCROLLLINES` (`0x0069`) with lines `1..10` or `WHEEL_PAGESCROLL` (`0xFFFFFFFF`).
    - Pointer Precision: `SPI_SETMOUSE` (`0x0004`) modifying the 3-element mouse acceleration array.
- **Definitive Solution**:
  - Windows: Synchronize UI sliders with `winuser::SystemParametersInfoW` in `src/platform/windows.rs`. Read OS defaults on launch, apply updates on slider change.
  - Linux: Synchronize via `xinput` pointer props / `libinput` acceleration profile.
  - Remove `[UNFINISHED]` badge from Advanced Settings tab.

---

## 2. Hardware Rapid Fire Key Configuration — [FEASIBILITY: 100% VERIFIED]
- **Current State**:
  - UI exposes Fire Mode (1..3), Repeat Count (1..20), Interval (10..100ms), and Continuous Fire.
- **Reverse Engineering Discovery (Delphi Binary `0x005aaaa8`, `0x005ab57b`)**:
  - Disassembly of `Btn_FireConfirmClick` (VA `0x005aaaa8`) proved that the Sunplus MCU does **NOT** store Fire Key at `0x0010..0x0013`.
  - When Fire Key is configured, Delphi writes two dedicated memory regions:
    1. **Fire Parameters Record (Offset `0x0122`, 12 bytes)**: Stores repeat count, burst delay, and loop mode.
    2. **Fire Bytecode Sequence (Offset `0x0130`, 168 bytes)**: Stores compiled macro bytecode (`a1` Mouse Left Down, `fe 00 06` Delay, `21` Mouse Left Up, `fe 00 06` Delay, `fc` Terminate).
- **Definitive Solution**:
  - Generate the 12-byte record at `0x0122` and bytecode at `0x0130` using `MacroCompiler` from `src/macro_engine.rs`.
  - Wire Button 7 (Top Wing Switch) rebind dropdown to trigger Fire Key.

---

## 3. Macro Manager & Onboard EEPROM Storage — [FEASIBILITY: 100% VERIFIED]
- **Current State**:
  - Binary opcode format (Mouse clicks, Keyboard keys, Modifiers, Delays, Loops, Terminator) 100% reverse-engineered in `src/macro_engine.rs`.
  - `render_macro_tab` in `src/gui.rs` renders the macro library, actions list, and loop settings, but is marked `[UNFINISHED]` because live keystroke capture and hardware EEPROM upload were disconnected.
- **Reverse Engineering Discovery (Delphi Binary `0x00591ea0`, `0x00591f9e`, `0x005b888c`)**:
  - Disassembly of `Btn_SaveMacroClick` (VA `0x00591ea0`) revealed the hardware flash workflow:
    1. Writes updated Button Mapping Table to offset `0x00C0` (length `0x0093` = 147 bytes).
    2. Writes Macro Slot 1 to offset `0x01F0` (length `0x0148` = 328 bytes).
    3. Writes Macro Slot 2 to offset `0x0338` (length `0x0148` = 328 bytes).
    4. Extended macro bank is stored at offset `0x0400..0x07FF` (1024 bytes).
  - Software macro bank format: `PCMACRODATA_2052.bin` in `assets/RCDATA/` is 27,200 bytes (100 macros x 272 bytes [16B ASCII name + 256B bytecode]).
- **Definitive Solution**:
  - Add global low-level keyboard/mouse recording hook (`SetWindowsHookExW` on Windows, `evdev` on Linux).
  - Wire Macro UI save/apply to compile sequences into 272-byte blocks and flash to offsets `0x01F0` and `0x0338`.
  - Remove `[UNFINISHED]` badge from Macro Manager tab.

---

## 4. LED Backlight & Lighting Effects — [FEASIBILITY: 100% VERIFIED]
- **Current State**:
  - UI exposes 7 preset LED modes (`Breathing`, `Colorful Light`, `Rainbow`, `Flow Light`, `Waltz`, `4-Seasons`, `Off`), speed slider (1..5), and brightness (Full/Half) mapped to bytes `0x0018` and `0x0008`.
  - Marked `[UNFINISHED]` in heading and window title.
- **Reverse Engineering Discovery (Delphi Binary `0x0057839c`, `0x0059eb48`)**:
  - Byte `0x0018`: Controls lighting effect (0=Breathing, 1=Colorful, 2=Rainbow, 3=Flow, 4=Waltz, 5=4-Seasons, 6=Off).
  - Byte `0x0008`: Brightness (`0x00` Full Brightness, `0x01` Half Brightness).
  - Byte `0x003C..0x0053`: 24 bytes storing 6 DPI stage RGB colors (`[R, G, B, 0x00]`).
  - Byte `0x0007`: In stock profiles, `0x07` is set to `0x7F`. Dynamic speed adjustments write single-byte feature commands via `0x5b7f8c`.
- **Definitive Solution**:
  - Remove `[UNFINISHED]` badge from LED Lighting tab.

---

## 5. Live Mode Switching (Gaming Mode 0x01 vs Office Mode 0x00) — [FEASIBILITY: SOLVED]
- **Current State**:
  - Profile bytes `0x00A4` (`0x01` = Gaming, `0x00` = Office), `0x00C0..0x00EF` (Office button table), and `0x00F1..0x0120` (Gaming button table) are parsed and editable.
  - Active mode remapping in GUI synchronizes both tables to prevent physical hardware button lockout.
  - Flashing via `0x40 0x07` + chunks + `0x40 0x0E` works reliably.

---

## 6. Optical Sensor Orientation & DSP Axis Safeguard — [FEASIBILITY: SOLVED & VERIFIED]
- **Current State**:
  - Decompilation of `cb_SensorRotationChange` (`0x0056ddf8`) confirmed byte `0x0006` bits 7..6 configure optical DSP rotation:
    - `00` (`0x00`): 270° rotation (Inverted axes).
    - `01` (`0x40` or default `0x7F`): 180° rotation (Normal physical PCB sensor mounting for EM624).
    - `10` (`0x80`): 90° rotation.
    - `11` (`0xC0`): 0° rotation.
  - Driver strictly locks byte `0x0006 = 0x7F` across all profiles. Emergency calibration button exists in App Settings and via CLI `em624-reversed fix-orientation`.

---

## 7. Packaging & Desktop Distribution — [FEASIBILITY: SOLVED & VERIFIED]
- **Current State**:
  - Automated GitHub Actions builds clean `.zip` packages for Windows x64 and Linux x64 with zero `.md` files bundled.
  - Sub-10MB RAM footprint strictly enforced via `HeapCompact` + `K32EmptyWorkingSet` + `malloc_trim`.
