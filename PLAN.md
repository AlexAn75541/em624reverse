# E-DRA EM624 Mouse Reverse Engineering & Modern Driver Plan

## Target Overview
- **Device**: E-DRA EM624 Gaming Mouse (7D + 1 Wheel)
- **Controller & Sensor**: SunplusIT SPCP6662 (6662IC) Optical Gaming Sensor + SPCP6651B MCU
- **Hardware IDs**: `VID 0x1BCF`, `PID 0x08B8`
- **Interfaces**:
  - `MI_00` (Endpoint `0x81` IN): Standard mouse movement and click events (1000 Hz)
  - `MI_01&Col01`: HID Keyboard & Macro keybind reports
  - `MI_01&Col02`: HID Consumer & Media controls
  - `MI_01&Col03`: HID System power controls
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
- [x] Centered ergonomic mouse chassis visualization with badges `[1]` through `[7]`.
- [x] Interactive button remapper matching standard mouse software (Standard, Special, Media, Shortcuts).
- [x] Advanced sensor tuning: 15 calibrated DPI steps up to 12,000 DPI, Stage 6 safety lock, interchangeable polling rates (`125Hz`, `250Hz`, `500Hz`, `1000Hz`).
- [x] LED lighting controls (Breathing, Colorful, Rainbow, Flow, Waltz, 4-Seasons, Off) with speed & brightness.
- [x] Fire Key and pointer/wheel speed settings.
- [x] Embedded on-the-fly `.bin` profile converter into `.cache/` at load time.
- [x] Static UI scaling presets (`100%`, `125%`, `150%`, `175%`, `200%`) in `.config/settings.json`.
- [x] OS native terminal debug mode (`em624.exe log-viewer` read-only log monitor in dedicated console window).
- [x] Cross-platform GitHub Actions CI/CD building and packaging `.zip` releases for Windows and Linux.

### Milestone 3: Memory Footprint Optimization (<32MB RAM Target) (COMPLETED)
- [x] Replaced default DirectX 12 (`wgpu`) backend with lightweight OpenGL (`glow`), eliminating ~50MB of D3D12/DXGI driver runtime state.
- [x] Implemented Win32 working set trimming (`SetProcessWorkingSetSize(GetCurrentProcess(), -1, -1)`), flushing cold initialization pages post-launch.
- [x] Purged CPU-side PNG pixel allocations immediately after uploading texture to GPU.
- [x] Verified working set profile: CLI subcommands (`probe`, `listen`, `log-viewer`) run at ~2-4MB RAM; GUI runs comfortably within ~18-28MB RAM on Windows and Linux.

### Milestone 4: Interactive Hardware Sniffing & Input Listener (COMPLETED)
- [x] Added `em624 listen` (alias `sniff`) CLI subcommand to observe mouse hardware events in real time.
- [x] Simultaneously listens to `MI_00` (mouse movement, clicks, wheel) and `MI_01` (hotkeys, consumer media, system reports).
- [x] Decodes raw bytes into button names, delta X/Y, wheel steps, and HID usages for live verification of profile behavior.

---

## Future Steps & Hardware Backlog (Inferred from TODO.md & Probed Data)

```
Current Hardware State
         │
         ▼
┌────────────────────────────────────────────────────────┐
│ Phase 1: Live Input Sniffing & Switch Calibration     │
│   - Run `em624 listen` while clicking all 7 switches   │
│   - Verify physical buttons vs internal memory slots   │
└────────────────────────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────────────────┐
│ Phase 2: Reverse Engineer Macro Bytecode (0x0400..0x07FF)│
│   - Sniff packets while assigning custom test macros   │
│   - Map opcodes for KeyDown, KeyUp, Delays (ms), Loops │
│   - Build compiler & disassembler in `src/profile.rs`  │
└────────────────────────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────────────────┐
│ Phase 3: SPCP6662 Optical Sensor DSP Register Tuning   │
│   - Lift-Off Distance (LOD) cutoff threshold register  │
│   - Hardware angle snapping / prediction enable flag   │
│   - Independent X/Y asymmetric DPI scaling registers   │
└────────────────────────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────────────────┐
│ Phase 4: Custom LED PWM & Timing Curve Parameters      │
│   - Map user RGB custom color palettes for flow effects│
│   - Extract millisecond transition timing formula      │
└────────────────────────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────────────────┐
│ Phase 5: Packaging & Background System Integration     │
│   - Windows System Tray icon for quick DPI/profile switch│
│   - Linux AppImage & standalone desktop bundle         │
│   - Non-root udev script (`scripts/install_udev.sh`)   │
└────────────────────────────────────────────────────────┘
```

### Action Items for Phase 1 & 2 (Next Immediate Work):
1. **Physical Button Mapping Verification**: Use `em624 listen` to verify button reports from physical switches 1 through 7 under different profiles.
2. **Macro Opcode Matrix**: In Delphi app, record 3 distinct macros (single key, key+delay, key loop), dump profiles via `em624 read-profile`, and diff offsets `0x0400..0x07FF`.
3. **LOD & DSP Register Mapping**: Inspect Delphi memory writes during `Adjust_XY_Option` and sensor calibration calls.
