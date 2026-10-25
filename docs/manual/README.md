# E-DRA EM624 Documentation & Reverse Engineering Manual

Welcome to the technical documentation and developer manual for the **E-DRA EM624 Gaming Mouse Reverse Engineering Project**.

This project provides a **100% native Rust open-source driver and modern GUI** replacing the proprietary 32-bit Delphi vendor utility (`EM624 Mouse.exe`), delivering cross-platform Linux and Windows support, sub-10MB working set memory footprint, verified checksum protection, and advanced features such as dynamic cloud typography and emergency sensor calibration.

---

## At a Glance

| Specification | Hardware Value | Driver Status |
|---|---|---|
| **USB Vendor ID (VID)** | `0x1BCF` (Sunplus Innovation Technology Inc.) | Verified & Auto-detected |
| **USB Product ID (PID)** | `0x08B8` (EM624 Gaming Mouse) | Verified & Auto-detected |
| **Optical Sensor** | SunplusIT SPCP6662 (6662IC) Optical Gaming Sensor | Calibrated 14-step DPI Table |
| **Microcontroller (MCU)** | SunplusIT SPCP6651B High-Speed USB MCU | 2048-Byte EEPROM Read/Write |
| **Physical Switches** | Huano Blue Shell Pink Dot (50M rated clicks) | 7 Buttons + 1 Wheel Encoder |
| **Memory Budget** | < 10 MB Steady-State Working Set | Verified Heap Compaction & Trimming |
| **License** | MIT License | 100% Open-Source |

---

## Why Reverse Engineer the EM624?

The stock vendor software distributed with the E-DRA EM624 mouse is an aging 32-bit Windows-only binary compiled with Embarcadero Delphi in 2018. Through static decompilation and dynamic USB packet analysis, we discovered multiple critical flaws in the vendor implementation:

1. **Firmware Click Lockup Bug (Stage 6)**: The vendor utility allowed users to activate DPI Stage 6 without masking bit 5 of the active stage bitmask (`0x0004`). In hardware, this causes an internal MCU interrupt deadlock, suspending all physical clicks until the mouse is physically unplugged and replugged.
2. **Missing Sensor Orientation Registers**: Custom profiles created in the vendor tool frequently overwrote byte `0x0006`, resetting the DSP axis transformation matrix to `0x00` (270° rotation). Because the optical sensor is mounted at 180° on the EM624 PCB, this inverted cursor movement and swapped X/Y axes.
3. **Proprietary Checksum Failure**: Modifying button tables without recalculating the MCU's 5-sector checksum (`(sum ^ 0x5a) & 0xff`) causes the microcontroller to boot into tracking-only mode while silently disabling all button event dispatches.
4. **No Linux Support**: Linux users had zero ability to adjust DPI, poll rate, RGB illumination, or keybindings.
5. **Bloated Resource Consumption**: The stock application consumed 60MB+ RAM and frequently failed to release GDI handles.

Our reversed driver solves every single one of these issues.

---

## System Architecture Overview

```
┌────────────────────────────────────────────────────────┐
│                   Desktop User Interface               │
│          egui / eframe (Cross-Platform GUI)            │
│  - Multi-theme (OLED, Win11, White)                    │
│  - Cloud Typography (Google Fonts & Fontshare)         │
│  - Free Window Resize & Dimension Presets              │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│               Profile & Checksum Engine                │
│             (src/profile.rs, src/macro_engine.rs)      │
│  - 2048-byte EEPROM layout verification                │
│  - Stage 6 Hardware Lockup Masking (active_mask & 0xDF) │
│  - Sensor Orientation Enforcement (0x0006 = 0x7F)      │
│  - 5-Sector Hardware Checksum: (sum ^ 0x5A) & 0xFF     │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│                 USB HID Transfer Engine                │
│                    (src/device.rs)                     │
│  - Interface 1, Col 04 (Usage Page 0xFF00, Usage 0x01) │
│  - Feature Report 0x40 (Control Dispatcher)            │
│  - Feature Report 0x41 (7-Byte Payload Streaming)      │
│  - Opcode 0x0D (Live Soft Apply)                       │
│  - Opcode 0x0E (EEPROM Burn & USB Reset Handling)      │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
              ┌──────────────────────────┐
              │   E-DRA EM624 Hardware   │
              │  (Sunplus SPCP6662/6651B)│
              └──────────────────────────┘
```

---

## Documentation Structure

The documentation is organized into two primary tracks:

### 📖 User Guides & How-Tos
Designed for gamers and daily users who want to configure and optimize their mouse:
- **[Installation & Linux Udev](installation.md)**: Setup guides for Windows and Linux without requiring root permissions.
- **[DPI, RGB & Button Setup](configuration.md)**: Calibrating DPI stages, choosing RGB lighting modes, and remapping buttons.
- **[Cloud Fonts & Typography](typography.md)**: Applying and previewing web fonts from Google Fonts and Fontshare.
- **[Window Sizing & Presets](window-sizing.md)**: Customizing window dimensions and toggling free resize mode.
- **[Troubleshooting & Emergency Recovery](troubleshooting.md)**: One-click fixes for inverted axes, dead clicks, and hardware reset.
- **[Frequently Asked Questions](faq.md)**: Hardware safety, bricking risks, and compatibility.

### 🤓 Nerdy / Developer Zone
Designed for reverse engineers, hardware hackers, and driver developers:
- **[`em624-driver` Internals & Memory Architecture](driver-internals.md)**: Deep dive into the Rust crate, sub-10MB working set management, `HeapCompact` + `malloc_trim(0)`, and dual-table synchronization.
- **[Silicon & Hardware Architecture](hardware.md)**: Sunplus SPCP6662 optical sensor, SPCP6651B MCU, and the 180° DSP coordinate transformation matrix.
- **[USB HID Protocol & Packets](protocol.md)**: Feature Reports 0x40 and 0x41, opcodes 0x07, 0x0D, 0x0E, and the 5-sector checksum formula.
- **[2048-Byte EEPROM Memory Map](memory-map.md)**: Byte-by-byte layout of the onboard flash memory (0x0000..0x07FF).
- **[Macro Bytecode Engine & VM](macros.md)**: Reverse-engineered bytecode instructions, delay timers, and keyup/keydown opcodes.
- **[CLI Automation & Raw Input Sniffer](cli-reference.md)**: Headless CLI commands, batch profile repair, and low-level Win32 Raw Input packet sniffing.
