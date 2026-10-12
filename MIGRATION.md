# Migration Guide: Moving from Stock EM624 App to Open Driver

This guide explains how to migrate from the official vendor utility (`EM624 Mouse.exe` / `SPCP665x 005A Gaming Mouse`) to the modern, open-source driver and CLI toolkit.

---

## Why Migrate?

| Feature | Stock Vendor App (`EM624 Mouse.exe`) | Open Driver (`em624-driver`) |
|---|---|---|
| **Architecture** | Legacy 32-bit Delphi GUI, requires admin | Modern Rust CLI / driver, minimal memory footprint |
| **Stage 6 Handling** | **Fatal bug**: Enables 6th stage in byte 4 (`0xFF`), triggering MCU trap that freezes all clicks | **Safety Guard**: Automatically clamps active stages to safe hardware limits (`0xDF`) |
| **Hardware Lockout Bug** | Sets register `0x03F9` to `0x84`, disabling button interrupts on game profiles | Automatically sanitizes register `0x03F9` to `0x04` |
| **Side Button Keybinds** | Corrupts button matrix when macros are unpopulated | Guarantees authentic Left, Right, Middle, Forward (Side 1), Backward (Side 2), and CPI loop |
| **Platform Support** | Windows only | Cross-platform (Windows & Linux native via `hidapi`) |
| **Live Diagnostics** | None (black box) | Real-time `probe` command reads actual mouse EEPROM |

---

## 1. Migrating Your Existing Profiles

If you created profiles in the stock app, they are saved as 2048-byte `.bin` files (usually in the app directory or exported via the "Save Profile" button).

### Step 1: Convert Your Profiles for Safety
Stock profiles often contain the stage 6 trap or the `0x84` button lockout register. Convert them before flashing:

#### Option A: Using the Driver CLI (Windows PowerShell)
```powershell
# Convert a single profile to guaranteed standard mouse buttons:
.\target\debug\em624.exe convert-profile "path\to\my_profile.bin" --buttons standard

# Or convert all profiles in a folder:
.\target\debug\em624.exe convert-all "profiles\" --buttons standard
```

#### Option B: Using Standalone Python (No Rust required)
```powershell
python scripts\convert_profile.py "path\to\my_profile.bin" --buttons standard
```
*(Original files are automatically backed up with a `.bin.bak` extension).*

---

## 2. Flashing Profiles to Your Mouse

Once converted, you can flash any profile directly from the terminal without having the stock software installed or running in the background.

### Standard Gaming Setup (Recommended)
Restores standard Left, Right, Middle, Forward (Side 1), Backward (Side 2), and CPI loop:
```powershell
.\target\debug\em624.exe apply-profile profiles\Valorant+CS2.bin
```

### Preserve Custom Profile Keybinds
If you assigned custom keyboard shortcuts (e.g. Spacebar or Q) to mouse buttons and want them preserved:
```powershell
.\target\debug\em624.exe apply-profile profiles\Valorant+CS2.bin --buttons keep
```

---

## 3. Live Hardware Diagnostic (`probe`)

Unlike the stock app, you can inspect the exact live hardware state currently flashed inside the mouse's physical memory:

```powershell
.\target\debug\em624.exe probe
```

This reads the active profile directly from the mouse over USB HID and displays:
- Active Profile Mode (Gaming vs Office)
- Active DPI stages and exact DPI levels
- 24-bit RGB LED colors per stage
- Active button mapping for all 9 key registers
- Hardware safety registers audit (checks for the `0x80` lockout bit)

---

## 4. Backing Up Current Mouse State

Before modifying settings, you can dump the mouse's active 2048-byte EEPROM into a binary file:

```powershell
.\target\debug\em624.exe read-profile backup_profile.bin
```

---

## 5. Emergency Recovery

If you previously used the stock app and your mouse buttons became unresponsive (optical tracking works but clicks do nothing):

1. **Quick CLI Recovery**:
   ```powershell
   .\target\debug\em624.exe apply-profile profiles\DailyUse.bin
   ```
2. **Physical Reset**:
   Unplug the USB cable for 5 seconds and plug it back in. The MCU reboots cleanly from persistent memory.
