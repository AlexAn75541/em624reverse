# CLI Command Reference

In addition to the GUI configurator, `em624-reversed` provides an extensive suite of command-line tools for headless administration, batch profile auditing, automated flashing, and packet sniffing.

---

## Command Overview

```
Usage: em624-reversed [OPTIONS] [COMMAND]

Commands:
  gui               Launch the cross-platform GUI configurator (default)
  list              Enumerate all connected USB HID devices and highlight candidate mice
  inspect-profile   Inspect and validate a 2048-byte profile binary (.bin / .em624)
  apply-profile     Flash a profile to mouse EEPROM memory with verified checksums
  read-profile      Dump the 2048-byte active profile from mouse EEPROM to a binary file
  fix-profile       Repair checksums, button tables, and sensor orientation in a profile file
  repair-profiles   Batch-scan and repair an entire directory of profile files
  listen            Listen for mouse input events via raw hardware packet sniffer
  help              Print this message or the help of the given subcommand(s)
```

---

## Detailed Command Documentation

### 1. `list`
Enumerates connected USB HID devices and detects the vendor interface (`Usage Page 0xFF00, Usage 0x01`):

```bash
cargo run -- list
```

---

### 2. `inspect-profile <FILE>`
Parses and displays the decoded fields of any profile file, verifying the 5 hardware checksum sectors:

```bash
cargo run -- inspect-profile exampleProfiles/DailyUse.em624
```

**Output includes:**
- Magic hardware signature (VID `0x1BCF`, PID `0x08B8`).
- Active DPI stage bitmask and Stage 6 safety check.
- Calibrated DPI values for stages 1..6.
- Stage RGB colors in hex format.
- Sensor orientation register analysis (`0x0006`).
- Decoded 12-key button mapping tables for Office and Gaming modes.
- Pass/Fail verification for all 5 checksum sectors.

---

### 3. `apply-profile <FILE> [--buttons <standard|keep>]`
Transmits and burns a 2048-byte profile into mouse EEPROM:

```bash
# Standard mode (Recommended): Enforces standard mouse clicks + verified checksums
cargo run -- apply-profile exampleProfiles/DailyUse.em624 --buttons standard

# Keep mode: Preserves authentic custom keybindings (e.g. game macros)
cargo run -- apply-profile exampleProfiles/Valorant+CS2.em624 --buttons keep
```

---

### 4. `read-profile <OUTPUT_FILE>`
Dumps the current profile from mouse RAM/EEPROM over USB Feature Report `0x40 0x09` into a local `.bin` file:

```bash
cargo run -- read-profile dumped_mouse_profile.bin
```

---

### 5. `fix-profile <INPUT_FILE> [-o <OUTPUT_FILE>]`
Repairs broken button tables, clamps Stage 6, enforces 180° sensor orientation, and recalculates all 5 checksums in a single step:

```bash
cargo run -- fix-profile broken_profile.bin -o repaired_profile.em624
```

---

### 6. `repair-profiles [DIRECTORY]`
Recursively scans a folder of `.bin` and `.em624` files, repairing any broken profiles in batch:

```bash
cargo run -- repair-profiles exampleProfiles/
```

---

### 7. `listen`
Starts the Windows Raw Input hardware packet sniffer to log real-time mouse button clicks, wheel deltas, and cursor movement directly from kernel drivers:

```bash
cargo run -- listen
```
