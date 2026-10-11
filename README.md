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
```powershell
.\target\debug\em624.exe apply-profile profiles\DailyUse.bin
```
Or apply a gaming profile with full compatibility mode:
```powershell
.\target\debug\em624.exe apply-profile profiles\Valorant+CS2.bin --og-compat
```

### 4. Read Active Profile from Mouse Memory
```powershell
.\target\debug\em624.exe read-profile backup.bin
```

---

## Current Status & Known Issues

- **DPI & RGB LED Control**: Fully functional. Custom DPI stages and RGB colors write cleanly to mouse hardware.
- **Button Mapping**:
  - Profiles are applied safely without disabling physical mouse clicks or freezing the MCU.
  - **Known Issue**: Custom button mappings (specifically the two side thumb buttons: Forward/Back, and profile-specific macros) are not yet accurately mapped when switching between non-default profiles.
  - **Reason**: The stock Delphi application writes complex mode-dependent button matrices (`Office Mode` vs `Gaming Mode` at `0x00C0..0x011F`).

For in-depth analysis and technical details, see:
- [`AGENTS.md`](./AGENTS.md): Full technical specification, decompiled assembly addresses, and recovery procedure.
- [`SESSION_NOTES.md`](./SESSION_NOTES.md): Comprehensive session log, protocol discoveries, challenges, and solutions.
- [`PLAN.md`](./PLAN.md): Development roadmap and milestones.

---

## License & Disclaimer

This project is for educational and interoperability purposes only. Reverse-engineered from public USB HID traffic and software binaries.
