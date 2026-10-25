# 2048-Byte Profile EEPROM Memory Map

This document details the exact byte layout of the 2048-byte EEPROM profile (`.bin` and `.em624`) used by the Sunplus SPCP6651B microcontroller.

---

## High-Level Layout Overview

```
Offset Range        Size         Description
────────────────────────────────────────────────────────────────────────
0x0000 - 0x0003     4 bytes      Hardware Signature: VID/PID (1B CF 08 B8)
0x0004              1 byte       Active DPI Stages Bitmask (0xDF = 5 stages)
0x0006              1 byte       Optical Sensor Orientation Bitfield (0x7F = 180°)
0x0007              1 byte       LED Breathing Speed (1..5)
0x0008              1 byte       LED Brightness (0 = Full, 1 = Half)
0x0018              1 byte       LED Illumination Mode (0..6)
0x002C - 0x002E     3 bytes      Packed DPI Stage Levels (Stages 1..6 nibbles)
0x003C - 0x0053     24 bytes     DPI Stage RGB Colors (4 bytes/stage: R, G, B, 0x00)
0x006F - 0x007F     17 bytes     Product String: "USB Gaming Mouse\0"
0x00A0              1 byte       Active USB Polling Rate Index (1=125Hz .. 4=1000Hz)
0x00A3              1 byte       Sector 0 Checksum (covers 0x0000..0x00A2)
0x00A4              1 byte       Active Mode Byte (0x00 = Office, 0x01 = Gaming)
0x00BF              1 byte       Sector 1 Checksum (covers 0x00A4..0x00BE)
0x00C0 - 0x00EF     48 bytes     Office Mode 12-Key Button Mapping Table
0x00F0              1 byte       Sector 2 Checksum (covers 0x00C0..0x00EF)
0x00F1 - 0x0120     48 bytes     Gaming Mode 12-Key Button Mapping Table
0x0121              1 byte       Sector 3 Checksum (covers 0x00F1..0x0120)
0x03F8 - 0x03FE     7 bytes      Hardware Footer Block
0x03FF              1 byte       Sector 4 Checksum (covers 0x03F8..0x03FE)
0x0400 - 0x07FF     1024 bytes   Hardware Macro Bytecode Storage & Padding
```

---

## Detailed Field Descriptions

### 1. Hardware Signature (`0x0000..0x0003`)
- `0x0000`: `0x1B`
- `0x0001`: `0xCF` (Little-endian Vendor ID: `0x1BCF`)
- `0x0002`: `0x08`
- `0x0003`: `0xB8` (Little-endian Product ID: `0x08B8`)

### 2. Active DPI Stages Bitmask (`0x0004`)
Bitfield representing enabled DPI stages:
- Bit 0: Stage 1 Enabled
- Bit 1: Stage 2 Enabled
- Bit 2: Stage 3 Enabled
- Bit 3: Stage 4 Enabled
- Bit 4: Stage 5 Enabled
- Bit 5: Stage 6 Enabled (Caution: Known MCU Lockup Trigger)
- **Safe Driver Default**: `0xDF` (`11011111b`, Stages 1-5 enabled, Stage 6 masked).

### 3. LED Lighting Mode (`0x0018`)
- `0x00`: Breathing Mode (Cycles single DPI color)
- `0x01`: Colorful Streaming (Rainbow spectrum flow)
- `0x02`: Rainbow Breathing
- `0x03`: Flowing Wave
- `0x04`: Waltz
- `0x05`: 4-Seasons
- `0x06`: Completely Off (Zero illumination)

### 4. Packed DPI Level Nibbles (`0x002C..0x002E`)
Each byte stores two 4-bit nibbles representing indices from the 14-step calibrated DPI table:
- `0x002C`: `[Stage 1 (high nibble) | Stage 2 (low nibble)]`
- `0x002D`: `[Stage 3 (high nibble) | Stage 4 (low nibble)]`
- `0x002E`: `[Stage 5 (high nibble) | Stage 6 (low nibble)]`

### 5. DPI Stage RGB Colors (`0x003C..0x0053`)
Each stage has 4 bytes in `[Red, Green, Blue, 0x00]` format:
- `0x003C..0x003F`: Stage 1 Color (Default: Red `FF 00 00 00`)
- `0x0040..0x0043`: Stage 2 Color (Default: Blue `00 00 FF 00`)
- `0x0044..0x0047`: Stage 3 Color (Default: Green `00 FF 00 00`)
- `0x0048..0x004B`: Stage 4 Color (Default: Yellow `FF FF 00 00`)
- `0x004C..0x004F`: Stage 5 Color (Default: Cyan `00 FF FF 00`)
- `0x0050..0x0053`: Stage 6 Color (Default: Magenta `FF 00 FF 00`)

---

## 12-Key Button Mapping Tables

The firmware maintains two tables of 12 keys (4 bytes each):
- **Office Mode**: `0x00C0..0x00EF` (48 bytes)
- **Gaming Mode**: `0x00F1..0x0120` (48 bytes)

### Physical Button to Slot Index Mapping:

| Button | Physical Switch | Slot Index | Default Function |
|---|---|---|---|
| 1 | Left Click | Slot 0 | Left Mouse Button (`01 01 00 00`) |
| 2 | Right Click | Slot 1 | Right Mouse Button (`01 02 00 00`) |
| 3 | Middle Click | Slot 2 | Middle Mouse Button (`01 04 00 00`) |
| 4 | Side 1 (Forward) | Slot 3 | Forward Button (`01 08 00 00`) |
| 5 | Side 2 (Backward) | Slot 4 | Backward Button (`01 10 00 00`) |
| 6 | DPI Switch | Slot 5 | DPI Loop (`08 12 00 00`) |
| 7 | Rapid Fire | Slot 6 | Fire Key (`06 00 00 00`) |
| 8-12 | Virtual / Macro | Slots 7..11 | Mode Switch or Macro triggers |
