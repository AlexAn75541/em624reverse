# E-DRA EM624 Mouse Reverse Engineering & Modern Driver Plan

## Target Overview
- **Device**: E-DRA EM624 Gaming Mouse (7D + 1 Wheel)
- **Controller & Sensor**: SunplusIT SPCP6662 (6662IC) Optical Gaming Sensor + SPCP6651B MCU
- **Hardware IDs**: `VID 0x1BCF`, `PID 0x08B8`
- **Interfaces**:
  - `MI_00` (Endpoint `0x81` IN): Standard mouse movement and click events (1000 Hz)
  - `MI_01&Col04` (Usage Page `0xFF00`, Usage `0x01`): Vendor configuration & EEPROM flashing interface

---

## Reverse Engineering Milestones & Plan

### Milestone 1: Core Protocol & Hardware Recovery (COMPLETED)
- [x] Identified Sunplus MCU VID (`0x1BCF`) and PID (`0x08B8`).
- [x] Decompiled official Delphi executable (`EM624 Mouse.exe`, `TForm1` at `0x0059bc8c`, flash writer at `0x005b7c8c`).
- [x] Mapped USB HID Feature Report protocol (`0x40` commands, `0x41` 293-chunk streaming).
- [x] Solved Stage 6 firmware lockup by clamping active stages mask (`&= 0xDF`).
- [x] Discovered and fixed 5-sector hardware checksum engine (`(sum ^ 0x5A) & 0xFF`), especially Sector 0 at `0x00A3`.
- [x] Clamped sensor calibration nibbles to valid range `<= 14` (`12,000 DPI`).
- [x] Mapped 7 physical switches and PCB routing to memory array slots.

### Milestone 2: Modern Native UI & Cross-Platform Tooling (COMPLETED)
- [x] Built 100% offline native Rust GUI (`src/gui.rs`) with `eframe` (egui).
- [x] Centered, fatter ergonomic mouse chassis visualization with badges `[1]` through `[7]`.
- [x] Interactive button remapper matching standard mouse software (Standard, Special, Media, Shortcuts).
- [x] Advanced sensor tuning: 15 calibrated DPI steps up to 12,000 DPI, Stage 6 safety lock, interchangeable polling rates (`125Hz`, `250Hz`, `500Hz`, `1000Hz`).
- [x] LED lighting controls (Breathing, Colorful, Rainbow, Flow, Waltz, 4-Seasons, Off) with speed & brightness.
- [x] Fire Key and pointer/wheel speed settings.
- [x] Embedded on-the-fly `.bin` profile converter into `.cache/` at load time.
- [x] Static UI scaling presets (`100%`, `125%`, `150%`, `175%`, `200%`) in `.config/settings.json`.
- [x] OS native terminal debug mode (`cmd.exe` allocation on Windows).
- [x] Cross-platform GitHub Actions CI/CD building and packaging `.zip` releases for Windows and Linux.

---

## Future Backlog & Undetermined Capabilities (Detailed in `TODO.md`)

### Phase 1: Macro Bytecode Reverse Engineering
1. **Opcode Capture**: Use USBPcap while recording complex macros in stock app.
2. **Grammar Mapping**: Map keydown, keyup, delay, and loop syntax in byte buffer `0x0400..0x07FF`.
3. **Execution Engine**: Build macro compiler and disassembler in `src/profile.rs`.

### Phase 2: Sensor DSP Extended Registers
1. **LOD Tuning**: Locate register offset for optical Lift-Off Distance threshold.
2. **Angle Snapping**: Test toggling prediction registers and measure motion paths.
3. **Asymmetric XY DPI**: Locate registers for separate X-axis and Y-axis DPI resolutions.

### Phase 3: Packaging & System Integration
1. **Linux Packaging**: Create single-file AppImage bundle.
2. **Automated udev Installer**: Provide one-command `scripts/install_udev.sh` for non-root USB access.
