# EM624 Mouse Reverse Engineering & Modern Driver

Open-source driver, reverse-engineering toolkit, and configuration CLI for the **EM624 Gaming Mouse** (SunplusIT SPCP6651B controller).

---

## Hardware Identity

| Attribute | Value |
|---|---|
| **Vendor ID (VID)** | `0x1BCF` (Sunplus Innovation Technology Inc.) |
| **Product ID (PID)** | `0x08B8` (EM624 Gaming Mouse) |
| **Controller** | SunplusIT SPCP6651B MCU |
| **Mouse Input Endpoint** | Interface 0 (`MI_00`), EP `0x81` IN (1000 Hz) |
| **Vendor Config Endpoint** | Interface 1 (`MI_01&Col04`), Usage Page `0xFF00`, Usage `0x01` |
| **Profile Size** | Exactly 2048 bytes (`0x800` bytes) |
| **Profile Signature** | `1B CF 08 B8` (Little-endian VID `0x1BCF` + PID `0x08B8`) |

---

## Features Implemented

- **Cross-Platform USB HID Driver**: Written in Rust using `hidapi`, supporting both Windows (Win32 HID) and Linux (`hidraw`).
- **Profile Parser & Decoder**: Reads 2048-byte `.bin` profiles and decodes active DPI stages, nibble-packed DPI values, and 24-bit RGB LED colors.
- **Direct Flash Read/Write**: Flashes custom profiles directly into mouse EEPROM via USB HID Feature Reports (`0x40` control + `0x41` 7-byte chunk streaming).
- **MCU Safety Guard (`--og-compat`)**: Prevents firmware hang/lockup by safely masking stage 6 and ensuring click responsiveness.

---

## Clean Recompile & Profile Switching Workflow

To perform a clean build and switch between multiple profiles on Windows PowerShell:

### 1. Clean Rebuild (Single Command)
```powershell
# Ensure official stock software (EM624 Mouse.exe) is closed to release USB HID interface
cargo clean; cargo build
```

### 2. Load Profile 1 (Daily Use / Productivity)
```powershell
.\target\debug\em624.exe apply-profile profiles\DailyUse.bin
```
*Applies 1200 / 2000 / 2800 / 3600 / 4400 DPI stages, RGB lighting, and standard mouse buttons with functional Forward & Backward side buttons.*

### 3. Load Profile 2 (Gaming / Valorant + CS2 / WarThunder)
```powershell
.\target\debug\em624.exe apply-profile profiles\Valorant+CS2.bin
```
*Applies tailored competitive DPI stages and colors without freezing mouse buttons.*

### 4. Verify Active State via Hardware Probe
```powershell
.\target\debug\em624.exe probe
```
*Reads active configuration directly from the mouse memory to confirm successful application.*

---

## Quick Start (PowerShell / Windows)

### 1. Build the Driver
```powershell
cargo build
```

### 2. Inspect a Profile
```powershell
.\target\debug\em624.exe inspect-profile profiles\DailyUse.bin
```

### 3. Apply a Profile to the Mouse

#### A. Standard Gaming Setup (Recommended: Left, Right, Middle, Forward, Back, CPI Loop)
```powershell
.\target\debug\em624.exe apply-profile profiles\Valorant+CS2.bin
```
*(By default, `--buttons standard` is active: preserves your custom DPI stages and RGB LED colors while restoring authentic Browser Forward and Backward side buttons!)*

#### B. Preserve Profile's Custom Game Bindings (e.g. Spacebar & Q in Valorant)
```powershell
.\target\debug\em624.exe apply-profile profiles\Valorant+CS2.bin --buttons keep
```

#### C. Restore Daily Use Bindings
```powershell
.\target\debug\em624.exe apply-profile profiles\DailyUse.bin --buttons keep
```

### 4. Live Hardware Diagnostic (Probe Mouse State)
```powershell
.\target\debug\em624.exe probe
```
*Queries the mouse directly over USB HID and prints a live diagnostic report of active DPI levels, RGB LED colors, active profile mode, button mappings, and hardware safety registers.*

### 5. Convert Profiles from Stock App
```powershell
# Convert single profile:
.\target\debug\em624.exe convert-profile profiles\WarThunder.bin --buttons standard

# Convert all profiles in a directory:
.\target\debug\em624.exe convert-all profiles --buttons standard
```

### 6. Read Active Profile from Mouse Memory
```powershell
.\target\debug\em624.exe read-profile backup.bin
```

---

## Current Status

- **DPI & RGB LED Control**: Fully functional. Custom DPI stages and RGB colors write cleanly to mouse hardware.
- **Button Mapping**:
  - **Standard Mode (`--buttons standard`)**: Default. Flashes 100% verified standard mouse actions (Left Click, Right Click, Middle Click, Forward Side Button, Backward Side Button, CPI Loop) on both Office and Gaming profiles.
  - **Keep Mode (`--buttons keep`)**: Retains custom keyboard shortcuts or media actions assigned to side buttons in each profile file, while ensuring safe Stage 6 masking and recalculated checksums.
  - **Checksum Engine**: Complete reverse-engineered `(sum ^ 0x5A) & 0xFF` algorithm across all profile sectors.

For in-depth analysis and technical details, see:
- [`MIGRATION.md`](./MIGRATION.md): Migration guide from official vendor software (`EM624 Mouse.exe`).
- [`AGENTS.md`](./AGENTS.md): Full technical specification, decompiled assembly addresses, and recovery procedure.
- [`SESSION_NOTES.md`](./SESSION_NOTES.md): Comprehensive session log, protocol discoveries, challenges, and solutions.
- [`PLAN.md`](./PLAN.md): Development roadmap and milestones.

---

## License & Disclaimer

This project is for educational and interoperability purposes only. Reverse-engineered from public USB HID traffic and software binaries.
