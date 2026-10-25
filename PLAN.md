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

## Reverse Engineering Milestones & Completed Features

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
- [x] Centered ergonomic mouse chassis visualization with badges `[1]` through `[7]` and proportional scaling.
- [x] Interactive button remapper matching standard mouse software (Standard, Special, Media, Shortcuts).
- [x] Advanced sensor tuning: 15 calibrated DPI steps up to 12,000 DPI, Stage 6 safety lock, interchangeable polling rates (`125Hz`, `250Hz`, `500Hz`, `1000Hz`).
- [x] LED lighting controls (Breathing, Colorful, Rainbow, Flow, Waltz, 4-Seasons, Off) with speed & brightness.
- [x] Fire Key and pointer/wheel speed settings.
- [x] Embedded on-the-fly `.bin` profile converter into `.cache/` at load time.
- [x] Static UI scaling presets (`100%`, `125%`, `150%`, `175%`, `200%`) in `.config/settings.json`.
- [x] OS native terminal debug mode (`em624.exe log-viewer` read-only log monitor in dedicated console window).
- [x] Pure GUI subsystem (`#![windows_subsystem = "windows"]`) preventing console popups on launch.
- [x] Multi-resolution icon (`assets/em624.ico`) embedded into `.exe` via `build.rs` and `winres`.
- [x] Bilingual English & Vietnamese translations with dynamic OS font loading (`configure_fonts`).
- [x] Cross-platform GitHub Actions CI/CD building clean `.zip` releases (zero `.md` files) for Windows and Linux.

### Milestone 3: Memory Footprint Optimization (<10MB RAM Target) (COMPLETED)
- [x] Replaced default DirectX 12 (`wgpu`) backend with lightweight OpenGL (`glow`), eliminating ~50MB of D3D12/DXGI driver runtime state.
- [x] Implemented Win32 working set trimming (`HeapCompact` + `K32EmptyWorkingSet` + `SetProcessWorkingSetSize`), flushing cold pages periodically and after major UI transitions.
- [x] Downsampled chassis texture to exact 320x340 rendered dimensions, reducing texture memory by >1MB.
- [x] Verified working set profile: GUI runs within ~5-8MB RAM on Windows and Linux, strictly within the <= 10MB budget.

### Milestone 4: Interactive Hardware Sniffing & Input Listener (COMPLETED)
- [x] Added `em624 listen` (alias `sniff`) CLI subcommand to observe mouse hardware events in real time.
- [x] Switched Windows listener to Windows Raw Input subsystem (`RegisterRawInputDevices` with `RIDEV_INPUTSINK`), bypassing the exclusive lock of `mouhid.sys`.
- [x] Simultaneously listens to `MI_00` (mouse movement, clicks, wheel) and `MI_01` (hotkeys, consumer media, system reports).
- [x] Decodes raw bytes into button names, delta X/Y, wheel steps, and HID usages for live verification of profile behavior.

---

## Detailed Technical Solutions for Unfinished Features (Backlog)

```
                         E-DRA EM624 ROADMAP
                                  │
         ┌────────────────────────┼────────────────────────┐
         ▼                        ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ Mode Switching   │    │ Stage 6 & Sensor │    │ Macro Engine     │
│ Solution: USB    │    │ Solution: DSP    │    │ Solution: Win32/ │
│ bus packet trace │    │ register mapping │    │ evdev hook +     │
│ & RAM soft-reset │    │ & stage cycle SM │    │ EEPROM 0x0400    │
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

### 1. Solution: Live Mode Switching (Gaming 0x01 vs Office 0x00)
- **Feasibility**: High (Agentic Workaround Active).
- **Status**: The driver updates byte `0x00A4` and flashes the profile to hardware via standard commit (`0x40 0x07` + chunks + `0x40 0x0E`), which reliably switches the mode with complete hardware compliance.

### 2. Solution: Optical Sensor Orientation & Inversion Fix
- **Feasibility**: 100% Solved & Verified.
- **Root Cause Discovered via Assembly Analysis**:
  - Delphi assembly at `0x0056ddf8` (`cb_SensorRotationChange`) confirmed byte `0x0006` is a bitfield where bits 7..6 configure the DSP axis rotation:
    - `00` (`0x00`): 270° rotation.
    - `01` (`0x40`): 180° rotation (Normal physical PCB sensor mounting orientation for EM624 chassis).
    - `10` (`0x80`): 90° rotation.
    - `11` (`0xC0`): 0° rotation.
  - In all factory dumps and working `.bin` profiles, byte `0x0006` is `127` (`0x7F`, bits 7..6 = `01`).
  - Previously, byte `0x0006` was mistakenly set to `0x00`, turning bits 7..6 to `00` (270°), which rotated tracking 90° out-of-phase and inverted the X and Y axes.
- **Definitive Fix**:
  1. Profile parser strictly locks `raw[0x0006] = 0x7F` and `raw[0x0007] = 0x7F` in `apply_safe_profile()`, `apply_standard_buttons()`, and `apply_og_compat()`.
  2. Added an on-demand emergency repair button in App Settings (`Calibrate & Fix Sensor Tracking Orientation`) and a CLI command (`em624-reversed fix-orientation`) to calibrate and flash orientation to hardware immediately.

### 3. Solution: Stage 6 DPI Firmware Deadlock & Sensor Trapping
- **Feasibility**: Solved via Hardware Safety Clamp.
- **Root Cause**: The SPCP6662 optical sensor lookup table in the MCU firmware has exactly 5 calibrated step registers (`0..4`). Cycling the physical button into stage 6 (`bit 5 = 1`) executes an unmapped firmware branch that traps the interrupt handler.
- **Definitive Fix**: Hard-clamped `active_stages_mask &= 0xDF` with optional research toggle in App Settings.

### 4. Solution: Macro Manager Live Recording Hooks & EEPROM Write
- **Feasibility**: High (Agentic Implementation in Progress).
- **Proposed Solution**:
  1. Add low-level input recording hooks:
     - **Windows**: `SetWindowsHookExW(WH_KEYBOARD_LL)` and `SetWindowsHookExW(WH_MOUSE_LL)` during recording mode.
     - **Linux**: `evdev` event capture stream.
  2. Convert captured keydown/keyup/clicks and timestamp deltas into `MacroAction` variants.
  3. Implement `write_macro_bank(&self, data: &[u8])` in `src/device.rs`:
     - Send Report `0x40 0x07` with offset `0x0400` and length `0x0400` (1024 bytes).
     - Stream 147 data chunks (Report `0x41`).
     - Transmit Commit `0x40 0x0E`.

### 5. Solution: Advanced Pointer & Fire Key Hardware Registers
- **Feasibility**: High.
- **Proposed Solution**:
  1. Run `em624-reversed listen` while moving the mouse at a constant velocity with byte `0x0014` set to `1` vs `11`.
  2. Measure continuous fire click intervals from raw HID Input Reports (`MI_00`) to confirm on-chip hardware timer autonomy.

### 6. Solution: Custom LED Lighting Curves & User Color Palettes
- **Feasibility**: High.
- **Proposed Solution**:
  1. Decompile `TForm_Color` (`DFM` offset `0x3c5120`). Trace where RGB values are copied when closing the modal.
  2. Verify if custom palettes are stored in bytes `0x003C..0x0053` (currently mapped as DPI stage colors) or in the padding zone `0x0130..0x01D8`.
  3. Map transition timing coefficients for speeds 1 through 5.

### 7. Solution: Advanced Optical Sensor DSP Tuning (SPCP6662)
- **Feasibility**: Medium.
- **Proposed Solution**:
  1. Trace function calls around `Adjust_XY_Option` (`0x0059b432`) in `EM624 Mouse.exe`.
  2. Decompile the initialization packet sequence in `FlashMemoryWriteRoutine` (`0x005b7c8c`).
  3. Map register offsets and provide sliders for LOD cutoff height and angle-snapping tolerance.

### 8. Solution: Linux Single-File AppImage Distribution
- **Feasibility**: High (CI Packaging).
- **Proposed Solution**:
  1. Add `linuxdeploy` and `linuxdeploy-plugin-appimage` steps to `.github/workflows/ci.yml`.
  2. Bundle `libudev` and OpenGL shared libraries into a self-contained `.AppImage` executable.

### 9. Future Roadmap & UI/UX Inspiration: OpenMouse-Project
- **Reference**: [OpenMouse-Project](https://github.com/OpenMouse-Project) (Open-source mouse software, hardware, and configurator ecosystem)
- **Key Inspirations & To-Do for EM624 Driver**:
  1. **Next-Gen Configurator UI/UX**:
     - Modern componentized layouts with responsive modular cards (DPI stages, polling monitors, LOD calibration).
     - Interactive vector-based mouse topology canvas with real-time button click and coordinate feedback.
     - Dark-mode ergonomic aesthetic tailored for high-refresh-rate displays and zero latency.
  2. **Sensor & Latency Telemetry**:
     - Real-time polling rate stability graphs (jitter analysis, actual report interval deltas).
     - Motion tracking precision visualizer and input delay benchmarking inspired by OpenMouse tools.
  3. **Universal Profile Architecture**:
     - Cloud-synced, vendor-agnostic profile definitions with automatic export to open formats.
     - Integration with community mouse tuning databases and standardized button action trees.
