# E-DRA EM624 Modern Mouse Configurator & Driver

Cross-platform, 100% native Rust configuration application and low-level driver for the **E-DRA EM624 Gaming Mouse** (SunplusIT SPCP6662 / 6662IC sensor).

Completely offline and self-contained: **Zero HTML, zero webview, zero network activity.**

---

## Hardware Identity

| Attribute | Value |
|---|---|
| **Device Name** | E-DRA EM624 Gaming Mouse |
| **Vendor ID (VID)** | `0x1BCF` (Sunplus Innovation Technology Inc.) |
| **Product ID (PID)** | `0x08B8` (EM624 Gaming Mouse) |
| **Optical Gaming Sensor** | SunplusIT SPCP6662 (`6662IC` / PMW3327 equivalent) |
| **Microcontroller** | SunplusIT SPCP6651B MCU |
| **Switches** | Huano 50M (50 million clicks rating) |
| **Buttons** | 7 physical switches + 1 scroll wheel |
| **Config Endpoint** | Interface 1 (`MI_01&Col04`), Usage Page `0xFF00`, Usage `0x01` |
| **Profile Size** | Exactly 2048 bytes (`0x800` bytes) |
| **Profile Signature** | `1B CF 08 B8` (Little-endian VID `0x1BCF` + PID `0x08B8`) |

---

## Features

- **100% Native Desktop GUI (eframe / egui)**: Instant startup, hardware-accelerated, offline desktop interface on both Windows and Linux.
- **Physical Mouse Visualizer**: Center display featuring large PC mouse layout with hardware badges `(1)` through `(7)` pointing to physical switch positions.
- **14-Step Calibrated DPI Sliders**: Full optical resolution tuning from `200` to `10,000 DPI` matching official sensor calibration steps.
- **RGB Backlight Control**: 24-bit color pickers for each active stage.
- **Mode Dispatch**: Support for both **Office Mode** and **Gaming Mode** button mapping tables.
- **Authentic Game Bindings Preserved**: Retains custom gaming shortcuts (`Win+D`, `Shift+Win+S`, `Ctrl+Numpad 1`, `Ctrl+M`, `Q`, `3`, `Media keys`).
- **MCU Safety Engine**:
  - Clamps out-of-bounds sensor calibration nibbles (`<= 12`).
  - Masks Stage 6 firmware lockup (`&= 0xDF`).
  - Clears hardware button lockout bit (`0x03F9 &= 0x7F`).
  - Recalculates all 5 hardware checksums with `(sum ^ 0x5A) & 0xFF`.
- **Built-in Profile Converter Tool**: Import and convert legacy vendor `.bin` exports into safe `.em624` format right from the UI.

---

## Quick Start (Windows)

### 1. Launch the Native GUI
Simply run `em624.exe` or launch from PowerShell:
```powershell
.\target\debug\em624.exe
```
Or explicitly:
```powershell
.\target\debug\em624.exe gui
```

### 2. Apply a Profile via CLI (Headless)

#### Load Gaming Profile (WarThunder / Valorant+CS2 / Bodycam)
```powershell
.\target\debug\em624.exe apply-profile exampleProfiles\WarThunder.em624
```

#### Load Productivity Profile (Daily Use)
```powershell
.\target\debug\em624.exe apply-profile exampleProfiles\DailyUse.em624
```

### 3. Live Hardware Diagnostic (Probe Mouse State)
```powershell
.\target\debug\em624.exe probe
```
*Queries the mouse directly over USB HID and prints a live diagnostic report of active DPI levels, RGB LED colors, active profile mode, button mappings, and hardware safety registers.*

### 4. Convert Stock Vendor `.bin` Profiles
```powershell
# Convert a single stock profile:
.\target\debug\em624.exe convert-profile exampleProfiles\WarThunder.bin -o custom.em624

# Batch-convert all stock profiles in a directory:
.\target\debug\em624.exe convert-all exampleProfiles --buttons keep
```

---

## Linux Support

See [`LINUX.md`](./LINUX.md) for system dependencies (Ubuntu, Fedora, Arch Linux) and udev rules for unprivileged USB HID communication.

---

## Documentation

- [`BUTTON_SPEC.md`](./BUTTON_SPEC.md): Complete button bytecode matrix, PCB switch routing, and opcode reference.
- [`MIGRATION.md`](./MIGRATION.md): Migration guide from official vendor software (`EM624 Mouse.exe`).
- [`AGENTS.md`](./AGENTS.md): Full technical specification, decompiled assembly addresses, and recovery procedure.
- [`LINUX.md`](./LINUX.md): Linux setup, dependencies, and udev rules.

---

## License & Disclaimer

This project is for educational and interoperability purposes only. Reverse-engineered from public USB HID traffic and software binaries.
