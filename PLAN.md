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
- [x] Decompiled official Delphi executable (`extracted/EM624 Mouse.exe`, `TForm1` at `0x0059bc8c`, flash writer at `0x005b888c`).
- [x] Mapped USB HID Feature Report protocol (`0x40` commands, `0x41` 293-chunk streaming).
- [x] Solved Stage 6 firmware lockup by clamping active stages mask (`&= 0xDF`).
- [x] Discovered and fixed 5-sector hardware checksum engine (`(sum ^ 0x5A) & 0xFF`), especially Sector 0 at `0x00A3`.
- [x] Clamped sensor calibration nibbles to valid range `<= 14` (`10,000 DPI`).
- [x] Mapped 7 physical switches and PCB routing to memory array slots.
- [x] Solved Sensor Orientation / Axis Inversion: byte `0x0006` locked to `0x7F` (180° rotation).

### Milestone 2: Modern Native UI & Cross-Platform Tooling (COMPLETED)
- [x] Built 100% offline native Rust GUI (`src/gui.rs`) with `eframe` (egui) and OpenGL `glow`.
- [x] Centered ergonomic mouse chassis visualization with badges `[1]` through `[7]` and proportional scaling.
- [x] Interactive button remapper matching standard mouse software (Standard, Special, Media, Shortcuts).
- [x] Mode-synchronized button flashing: dual table sync (`0x00C0` Office & `0x00F1` Gaming) + `0x00A4` active mode byte.
- [x] Advanced sensor tuning: 14 calibrated DPI steps up to 10,000 DPI, Stage 6 safety lock, interchangeable polling rates (`125Hz`, `250Hz`, `500Hz`, `1000Hz` with read-modify-write on `0x0004` & `0x0008`).
- [x] Unified Safe Flash Pipeline: Soft-apply Opcode `0x40 0x0D` before Opcode `0x40 0x0E` commit to latch MCU scan matrix.
- [ ] LED Backlight Subsystem [UNFINISHED]: Register `0x00A5 = (mode & 0x07) | 0x80` (bit 7 master enable), speed `0x0007` (1..5), brightness `0x0008` (bit 0). Staged in profile; awaiting live hardware verification.
- [x] Button Remap Physical Alignment: Mapped physical switch routing (Slot 1 Left, Slot 2 Forward, Slot 4 Right, Slot 5 Back, Slot 7 Middle, Slot 8 DPI-) with dual-table sync (`0x00C0` & `0x00F1`).
- [x] Cloud typography service with Google Fonts and Fontshare verification and disk caching.
- [x] Free window resizing and dimension presets in settings.
- [x] Bilingual English & Vietnamese translations.
- [x] Cross-platform GitHub Actions CI/CD building clean `.zip` releases (zero `.md` files) for Windows and Linux.

### Milestone 3: Memory Footprint Optimization (<10MB RAM Target) (COMPLETED)
- [x] Replaced default DirectX 12 (`wgpu`) backend with lightweight OpenGL (`glow`), eliminating ~50MB of D3D12/DXGI driver runtime state.
- [x] Implemented Win32 working set trimming (`HeapCompact` + `K32EmptyWorkingSet` + `SetProcessWorkingSetSize`) and Linux `malloc_trim`.
- [x] Verified working set profile: GUI runs within ~5-8MB RAM on Windows and Linux, strictly within the <= 10MB budget.

### Milestone 4: Interactive Hardware Sniffing & Input Listener (COMPLETED)
- [x] Added `em624-reversed listen` (alias `sniff`) CLI subcommand to observe mouse hardware events in real time.
- [x] Switched Windows listener to Windows Raw Input subsystem (`RegisterRawInputDevices` with `RIDEV_INPUTSINK`), bypassing the exclusive lock of `mouhid.sys`.
- [x] Decodes raw bytes into button names, delta X/Y, wheel steps, and HID usages for live verification of profile behavior.

---

## Action Plan: Completing Unfinished Features

```
                         UNFINISHED FEATURES ROADMAP
                                     │
         ┌───────────────────────────┼───────────────────────────┐
         ▼                           ▼                           ▼
┌──────────────────┐       ┌──────────────────┐        ┌──────────────────┐
│ Phase 1: OS      │       │ Phase 2: Rapid   │        │ Phase 3: Macro   │
│ Mouse Dynamics   │       │ Fire Hardware    │        │ Bank & Upload    │
│ Solution: Win32  │       │ Solution: EEPROM │        │ Solution: 272B   │
│ SystemParameters │       │ 0x0122 & 0x0130  │        │ blocks to 0x01F0 │
└──────────────────┘       └──────────────────┘        └──────────────────┘
```

### Phase 1: Connect OS-Level Mouse Dynamics (Advanced Tab)
1. **Windows System Parameters Synchronization**:
   - In `src/platform/windows.rs`, add functions:
     - `get_mouse_speed() -> u32` / `set_mouse_speed(val: u32)` (`SPI_GETMOUSESPEED`, `SPI_SETMOUSESPEED` range 1..20).
     - `get_double_click_time() -> u32` / `set_double_click_time(ms: u32)` (`GetDoubleClickTime`, `SetDoubleClickTime`).
     - `get_wheel_scroll_lines() -> u32` / `set_wheel_scroll_lines(lines: u32)` (`SPI_SETWHEELSCROLLLINES`).
     - `get_pointer_precision() -> bool` / `set_pointer_precision(enabled: bool)` (`SPI_SETMOUSE`).
2. **GUI Hookup**:
   - Query OS values on GUI startup and populate sliders.
   - When user adjusts sliders or clicks "Apply", call the OS setters.
3. **Badge Removal**:
   - Remove `[UNFINISHED]` badge from `render_advanced_tab` in `src/gui.rs` and `src/i18n.rs`.

### Phase 2: Hardware Rapid Fire Key (EEPROM `0x0122` & `0x0130`)
1. **Profile Byte Layout**:
   - Disassembly of `Btn_FireConfirmClick` (VA `0x005aaaa8`) proved Fire Key is stored in:
     - `0x0122..0x012D` (12 bytes): repeat count, interval delay, and mode.
     - `0x0130..0x01D7` (168 bytes): compiled macro bytecode.
2. **Implementation**:
   - Stop writing dummy values to `0x0010..0x0013`.
   - In `src/profile.rs`, add `set_fire_key(repeat_count: u8, interval_ms: u16, continuous: bool)`.
   - Compile left-down (`0xa1`), delay (`0xfe`), left-up (`0x21`), delay, terminate (`0xfc`) bytecode into `raw[0x0130..]`.
3. **Button Remap**:
   - Map Button 7 (Top Wing Switch) default action to Fire Key bytecode trigger (`0x06 0x00 0x00 0x00`).

### Phase 3: Macro Manager UI Recording & EEPROM Flashing
1. **Input Recording Hook**:
   - Windows: Low-level keyboard hook (`SetWindowsHookExW(WH_KEYBOARD_LL)`) and mouse hook (`WH_MOUSE_LL`) during recording.
   - Linux: Read events from `/dev/input/event*` or X11/Wayland event stream.
2. **EEPROM Flashing Workflow**:
   - Disassembly of `Btn_SaveMacroClick` (VA `0x00591ea0`) verified:
     - Macro Slot 1 flashes to offset `0x01F0` (328 bytes).
     - Macro Slot 2 flashes to offset `0x0338` (328 bytes).
     - Extended macro bank stored in `0x0400..0x07FF` (1024 bytes).
3. **Badge Removal**:
   - Remove `[UNFINISHED]` badge from `render_macro_tab` in `src/gui.rs` and `src/i18n.rs`.

### Phase 4: LED Lighting Tab Finalization
1. **Verification**:
   - Lighting mode `0x0018` (0=Breathing..6=Off) and brightness `0x0008` (0=Full, 1=Half) are already fully functional.
2. **Badge Removal**:
   - Remove `[UNFINISHED]` badge from `render_led_tab` in `src/gui.rs` and `src/i18n.rs`.

---

## Next Major Initiative: OpenMouse-Inspired UI Rework (PLANNED)

> **Status**: Architecture Designed & Audited. **Awaiting User Confirmation** to create and switch to dedicated branch `feat/openmouse-ui-rework`.

### 1. Vision & Architectural Goals
Incorporate modern, modular UI principles inspired by [OpenMouse-Project](https://github.com/OpenMouse-Project), Wootility, and Pulsar Fusion:
1. **Hardware Topology as Primary Interface**: Mouse chassis diagram acts as an interactive canvas with responsive switch nodes and callouts.
2. **Unified Sensor Cockpit**: Integrates 14-step calibrated DPI curves, live polling rates (`125Hz..1000Hz`), and DSP axis inversion safety lock (`0x7F` / 180°).
3. **Visual Timeline Macro Editor**: Horizontal millisecond event tracks (KeyDown, KeyUp, Delay blocks) with real-time bytecode compilation and EEPROM slot budget bars (328 bytes / slot).
4. **Live Telemetry & Diagnostics Dock**: Real-time polling rate jitter graph (Hz vs. time) and click debounce / latency tester backed by zero-allocation ring buffers.
5. **Strict AGENTS.md Section 10 Compliance**:
   - Memory Working Set strictly `<= 10 MB` with periodic `trim_working_set()` hooks.
   - Universal scrolling via `egui::ScrollArea::both().auto_shrink([false, false])`.
   - Zero hardcoded height clamps (no `allocate_ui_with_layout(..., ui.available_height())`).
   - Clean typographic ASCII labels (no Nerd Font / Private Use Unicode escapes that cause tofu rendering).
   - High-contrast AAA/AA palettes across `PitchBlackOled`, `Windows11Dark`, and `LightWhite`.

### 2. Proposed Module Directory Structure (`src/gui/`)
```
src/gui/
├── mod.rs                  // App entry, event routing, working-set GC coordinator (<250 lines)
├── config.rs               // AppConfig, persistent state, master directories
├── theme/
│   ├── mod.rs              // Theme token resolution & WCAG contrast audit
│   ├── tokens.rs           // Semantic tokens (surfaces, borders, text, state)
│   └── palettes.rs         // PitchBlackOled, Windows11Dark, LightWhite
├── shell/
│   ├── mod.rs              // Shell layout coordinator
│   ├── header.rs           // Hardware status banner, profile selector, connection dot
│   ├── sidebar.rs          // Compact vertical navigation dock with ASCII badges
│   └── footer.rs           // 2-story action bar (Flash/Read) & IO progress dock
├── views/
│   ├── canvas/             // Interactive Hardware Topology Canvas
│   │   ├── mod.rs          // Canvas layout & responsive scaling
│   │   ├── chassis.rs      // Native 600x600 texture rendering & subpixel switch pinouts
│   │   ├── callouts.rs     // Responsive leader lines, switch node hitboxes, hover glow
│   │   └── rebind_popover.rs // In-place non-modal button remapping drawer
│   ├── cockpit/            // Unified DPI + Sensor Cockpit
│   │   ├── mod.rs          // Sensor parameter orchestration
│   │   ├── dpi_curve.rs    // 14-step calibrated SPCP6662 resolution curve
│   │   ├── polling_rate.rs // Read-modify-write rate selector (125Hz..1000Hz)
│   │   └── dsp_matrix.rs   // Byte 0x0006 axis orientation lock (180 deg)
│   ├── led_studio/         // Visual LED Studio
│   │   ├── mod.rs          // Lighting coordinator
│   │   ├── preview.rs      // Live chassis illumination simulator
│   │   ├── modes.rs        // 8 hardware modes (CPI Breathing, Flow, Waltz, etc.)
│   │   └── color_palette.rs// Per-stage 24-bit RGB pickers (0x0039..0x004D)
│   ├── macro_timeline/     // Visual Timeline Macro Editor
│   │   ├── mod.rs          // Sequence bank & playback options
│   │   ├── timeline.rs     // Horizontal/stacked millisecond event tracks
│   │   ├── action_block.rs // KeyDown, KeyUp, MouseClick, Delay visual blocks
│   │   └── compiler.rs     // Bytecode verification against EEPROM maps
│   ├── telemetry/          // Live Telemetry Dock
│   │   ├── mod.rs          // Telemetry dock lifecycle and ring buffers (512 capacity)
│   │   ├── polling_chart.rs// Real-time polling rate jitter graph (Hz vs. time)
│   │   ├── latency_test.rs // Switch debounce & click latency tester
│   │   └── motion_feed.rs  // Raw Input / evdev coordinate delta vectors
│   └── settings/           // System & Driver Configuration
│       ├── mod.rs          // App settings, language, theme switcher
│       └── font_manager.rs // Typography verification, caching, and clean loader
└── io_worker.rs            // Non-blocking HID thread channel dispatcher
```

### 3. Phased Implementation Roadmap
- **Phase 1: Foundation & Theme System**:
  - Create dedicated branch `feat/openmouse-ui-rework`.
  - Implement `theme/tokens.rs` and `theme/palettes.rs`.
  - Implement `shell/` (`header.rs`, `sidebar.rs`, `footer.rs`) with clean ASCII indicators.
- **Phase 2: Hardware Topology Canvas**:
  - Implement interactive canvas with switch pins, leader lines, and anchored rebind popovers.
  - Retain dual-table sync (`0x00C0` Office & `0x00F1` Gaming) and hardware checksum validation.
- **Phase 3: Sensor Cockpit & LED Studio**:
  - Implement 14-step DPI curve, Stage 6 safety clamp, polling rate selector, and live LED preview.
- **Phase 4: Timeline Macro Studio**:
  - Implement millisecond timeline tracks, removing all `allocate_ui_with_layout` height clamps.
- **Phase 5: Telemetry Dock**:
  - Implement 512-entry fixed ring buffer for live polling rate jitter and click latency testing.
- **Phase 6: Verification & Merge**:
  - Verify `<= 10MB` working set RAM across Windows and Linux.
  - Verify universal scrolling down to 280x200 window resolution.
  - Ensure zero clippy/formatting warnings before PR merge.
