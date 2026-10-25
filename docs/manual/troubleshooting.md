# Troubleshooting & Recovery Bible

This guide provides definitive troubleshooting procedures and technical root causes for all known edge cases, hardware behaviors, and recovery methods for the E-DRA EM624 gaming mouse.

---

<div class="callout-warning">
<strong>Active Development Notice</strong><br>
This driver and configuration software is an open-source reverse-engineering effort developed via static binary disassembly and dynamic USB packet analysis. While the driver includes comprehensive checksum verification and hardware clamping safeguards, users should familiarize themselves with the recovery procedures below.
</div>

---

## 1. Mouse Cursor Moves in Reverse or Swaps X/Y Axes

### Symptom:
Moving the physical mouse forward moves the cursor left or down. Moving the mouse left moves the cursor up or right.

### Technical Root Cause:
The Sunplus SPCP6662 optical gaming sensor is physically soldered to the EM624 internal circuit board in a **180° inverted position**. The optical DSP coordinate transformation register at **byte `0x0006`** controls this rotation. If a corrupted profile, vendor preset, or unverified flash writes `0x00` to this byte, the DSP applies a 270° coordinate rotation, throwing motion 90° out-of-phase with the chassis.

### Immediate Fix:
1. Open the **App Settings** tab.
2. Click the emergency button: **"Calibrate & Fix Sensor Tracking Orientation (Emergency Axis Repair)"**.
3. The driver immediately forces `0x0006 = 0x7F` and `0x0007 = 0x7F`, burns the EEPROM, and restores correct 1:1 tracking.
4. Alternatively via CLI:
   ```bash
   cargo run -- apply-profile exampleProfiles/DailyUse.em624
   ```

---

## 2. Cursor Moves Fine, but All Physical Clicks Are Completely Dead

### Symptom:
The optical sensor tracks your hand movement normally, but left click, right click, side buttons, and scroll wheel do not register in the OS.

### Technical Root Causes:

#### Cause A: Checksum Sector 0 (`0x00A3`) or Button Tables Corrupted
The Sunplus microcontroller validates 5 distinct sector checksums before activating button event dispatches:
```
checksum = (sum(sector_bytes) ^ 0x5A) & 0xFF
```
If byte `0x00A3` or `0x00F0`/`0x0121` fails validation, the MCU boots optical cursor tracking but **completely shuts down physical button click dispatching**.

#### Cause B: Stage 6 Microcontroller Interrupt Deadlock
In factory firmware, enabling bit 5 in byte `0x0004` (DPI Stage 6) triggers an internal hardware interrupt race condition inside the SPCP6651B MCU. The MCU locks up its input polling loop until power is cycled.

### Recovery Procedure:

#### Step 1: Replug Mouse (Hard Reset)
Unplug the mouse USB cable from your computer, wait 5 seconds, and plug it back in. The microcontroller reboots into cold flash.

#### Step 2: Flash Standard Clean Profile
Open a terminal in the project directory and run:
```powershell
cargo run -- apply-profile exampleProfiles/DailyUse.em624 --buttons standard
```
The driver automatically:
- Clamps active stages to `0xDF` (disables the Stage 6 deadlock trigger).
- Restores factory standard button tables.
- Recalculates all 5 checksum sectors.
- Flashes and applies settings cleanly.

---

## 3. Flash Error Code `0x0000001F` on Windows or `-EPIPE` on Linux

### Symptom:
During EEPROM write, the console logs `Windows error 0x0000001F` or Linux returns `Broken Pipe (-EPIPE)`.

### Explanation:
**This is NOT an error — it is a successful flash confirmation.**

When the driver transmits Opcode `0x0E` (`40 0e 00 00 00 00 00 00`), the Sunplus MCU commits the RAM buffer into permanent EEPROM flash. To finalize this operation, the MCU **momentarily power-cycles its internal USB controller**, causing the USB bus to drop connection for ~200 milliseconds.

Windows logs this sudden disconnection as `0x0000001F` (`ERROR_GEN_FAILURE`), while Linux logs `-EPIPE`. The reversed driver explicitly intercepts these specific return codes and treats them as a successful flash event.

---

## 4. "Device Not Detected" or Permission Denied on Linux

### Symptom:
The configurator says `No device connected` on Linux, or opening `/dev/hidraw*` fails with `Permission Denied`.

### Fix:
Linux restricts access to raw USB HID interfaces to `root` by default. Install the non-root udev rules:

```bash
sudo ./scripts/install_udev.sh
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Then unplug and replug the mouse USB cable.

---

## 5. Official Vendor Utility Emergency Reset

If you ever need to restore factory hardware defaults using the original vendor utility:
1. Launch `extracted/EM624 Mouse.exe`.
2. Click the **"Default"** button (triggers in-memory buffer reset from address `0x005c7cdc`).
3. Click **"Apply"** / **"Confirm"** (triggers `Btn_Confirm1_1Click` at `0x0059bc8c`).
4. Replug your mouse.
