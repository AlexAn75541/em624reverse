# E-DRA EM624 Mouse Configurator & Driver

Open-source, cross-platform driver and native configuration utility for the **E-DRA EM624 Gaming Mouse**.

- **Hardware**: SunplusIT SPCP6662 (6662IC) Optical Sensor + SPCP6651B MCU
- **USB IDs**: Vendor ID `0x1BCF`, Product ID `0x08B8`
- **Target Interface**: Interface 1 Col04 (`Usage Page 0xFF00`, `Usage 0x01`)
- **Compatibility**: Windows 10/11 & Linux (x86_64)

---

## Features

- **100% Native Desktop GUI**: Fast, lightweight Rust GUI powered by `egui` (zero webview/Electron).
- **Themes**: Pitch Black (OLED), Windows 11 Dark (default), and High-Contrast White.
- **DPI Tuning**: 15 discrete hardware steps (`200` to `12,000 DPI`) with safe Stage 6 protection.
- **Keybinding Matrix**: Full keyboard, multimedia, macro, and mouse button remapping.
- **Automatic Device Detection**: Native OS USB PnP monitoring for seamless plug/unplug tracking.
- **Hardware Sniffer**: Built-in Windows Raw Input packet listener (`em624 listen`).

---

## Quick Start

### Build & Run GUI
```bash
# Debug run
cargo run

# Optimized release build
cargo run --release
```

### CLI Subcommands
```powershell
# Probe active mouse settings over USB HID
em624 probe

# Apply a configuration profile directly
em624 apply-profile exampleProfiles/DailyUse.em624

# Sniff live hardware button clicks, wheel, and hotkeys
em624 listen
```

---

## Linux Setup

Install udev rules to allow non-root access to the mouse:
```bash
sudo bash scripts/install_udev.sh
```

---

## Documentation

- [`AGENTS.md`](./AGENTS.md): Hardware reverse-engineering specifications, USB HID protocol, and recovery guide.
- [`BUTTON_SPEC.md`](./BUTTON_SPEC.md): Button bytecode matrix, PCB switch routing, and opcode reference.
- [`MIGRATION.md`](./MIGRATION.md): Migration guide from stock vendor software.
- [`LINUX.md`](./LINUX.md): Linux setup, build prerequisites, and troubleshooting.

---

## License

MIT License. Independent open-source project for interoperability and hardware preservation. Not affiliated with E-DRA or Sunplus Innovation Technology Inc.
