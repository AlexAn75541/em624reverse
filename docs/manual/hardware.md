# Hardware Profile & Optical Sensor Architecture

This chapter details the physical silicon, optical DSP, microcontroller, and USB interface routing discovered during static decompilation and hardware packet sniffing of the E-DRA EM624 gaming mouse.

---

## Hardware Identity

- **Manufacturer**: Sunplus Innovation Technology Inc. (SunplusIT)
- **Vendor ID (VID)**: `0x1BCF`
- **Product ID (PID)**: `0x08B8`
- **Primary Chipset**: SunplusIT SPCP6662 (6662IC) Optical Gaming Sensor + SPCP6651B High-Speed USB MCU.
- **Physical Switches**: Huano 50M Click Rating (Blue Shell, Pink Dot).
- **Physical Buttons**: 7 physical microswitches + 1 mechanical scroll wheel encoder:
  1. Button 1: Left Click (Microswitch)
  2. Button 2: Right Click (Microswitch)
  3. Button 3: Middle Click (Wheel Microswitch)
  4. Button 4: Side Forward 1 (Upper Thumb Switch)
  5. Button 5: Side Backward 2 (Lower Thumb Switch)
  6. Button 6: DPI Loop (Top Center Switch)
  7. Button 7: Rapid Fire / Custom (Top Left Wing Switch)
  8. Wheel: Vertical Optical Scroll Encoder

---

## USB Interface Architecture

Upon USB enumeration, the EM624 reports two distinct logical interfaces (`MI_00` and `MI_01`):

```
Device (VID: 0x1BCF, PID: 0x08B8)
├── Interface 0 (MI_00) - Standard HID Mouse
│   ├── Endpoint 0x81 IN (Interrupt, 1000Hz polling rate)
│   └── Transmits standard 5-byte relative motion reports [Buttons, dX, dY, Wheel, Pan]
│
└── Interface 1 (MI_01) - Multi-Collection HID
    ├── Collection 01: HID Keyboard / Consumer Controls
    ├── Collection 02: HID Consumer Controls
    ├── Collection 03: HID System Controls
    └── Collection 04: Vendor Configuration & Flash Interface
        ├── Usage Page: 0xFF00 (Vendor-defined)
        ├── Usage: 0x01 (Configuration Port)
        └── Handles Feature Reports 0x40 and 0x41 exclusively
```

> **Critical Routing Rule**: Any USB Feature Report dispatched to `MI_00` or `MI_01/Col01..Col03` will fail with `0x0000001F` (`ERROR_GEN_FAILURE`) on Windows or `-EPIPE` on Linux. All configuration writes and reads MUST open `MI_01&Col04` (`Usage Page 0xFF00, Usage 0x01`).

---

## Optical Sensor Coordinate System & DSP Rotation Matrix

The Sunplus SPCP6662 optical DSP contains an internal hardware coordinate rotation register mapped to byte `0x0006` of the profile EEPROM.

Because of the physical layout of the EM624 mouse PCB, the optical sensor IC is soldered in a **180° inverted position** relative to the mouse chassis.

### Bits 7..6 Orientation Truth Table:

| Bits 7..6 | Hex Value | Coordinate Rotation | Result on EM624 Chassis |
|---|---|---|---|
| `00` | `0x00` | 270° Rotation | **Inverted Tracking**: Moving mouse Up moves cursor Left; moving mouse Right moves cursor Up. |
| `01` | `0x40` or `0x7F` | **180° Rotation** | **Correct Physical Tracking**: 1:1 true tracking across all axes. |
| `10` | `0x80` | 90° Rotation | **Inverted Tracking**: Moving mouse Up moves cursor Right; moving mouse Left moves cursor Up. |
| `11` | `0xC0` | 0° Rotation | **Inverted Tracking**: Moving mouse Up moves cursor Down (Inverted Y-axis). |

```rust
// Verified driver safeguard in src/profile.rs:
pub fn ensure_correct_sensor_orientation(&mut self) {
    self.raw[0x0006] = 0x7F; // 180° orientation enforced
    self.raw[0x0007] = 0x7F; // LED breathing rate / DSP register safe byte
}
```

---

## Hardware Calibrated DPI Steps

The SPCP6662 optical sensor does not support arbitrary non-discrete DPI steps. Instead, the internal DSP supports **14 calibrated discrete multiplier levels**:

```
Index: [ 0,   1,   2,   3,   4,    5,    6,    7,    8,    9,   10,   11,   12,    13   ]
DPI:   [200, 400, 600, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4800, 6400, 8000, 10000]
```

When custom DPI values are selected, the driver rounds to the nearest calibrated hardware step and packs the index into the corresponding profile nibble at offsets `0x002C..0x002E`.

---

## Polling Rate Control

The USB transfer polling rate is configured via registers at byte `0x00A0` and `0x00A5`:

| Setting | Polling Frequency | Transfer Interval | Register Value |
|---|---|---|---|
| 1 | 125 Hz | 8 ms | `0x01` |
| 2 | 250 Hz | 4 ms | `0x02` |
| 3 | 500 Hz | 2 ms | `0x03` |
| 4 | 1000 Hz | 1 ms (Default) | `0x04` |
