# EM624 Mouse — Button Architecture & Bytecode Specification

This document details the complete reverse-engineered hardware architecture, PCB pin routing, and 4-byte bytecode dispatch matrix for the **Dareu / E-DRA EM624 Gaming Mouse** (SunplusIT SPCP6651B controller).

---

## 1. Physical Buttons & PCB GPIO Mapping

The EM624 mouse features **7 physical buttons**, as shown in the official E-DRA software screenshots (`screenshotOG/`):

```
                   [ Cable ]
                  /         \
         (1) Left [ (3) Whl ] (2) Right
                  |         |
      (4) Fwd > [ | (6) DPI+| ]
      (5) Back> [ | (7) DPI-| ]
                  \         \
                   \ [Logo]  \
```

### Physical Switch to Memory Slot Routing:

| UI Button # | Official UI Label | Physical Location | MCU Memory Slot | Office Table Offset | Gaming Table Offset |
|---|---|---|---|---|---|
| **Button 1** | `Left Key` | Left click switch | **Slot 1** | `0x00C0..0x00C3` | `0x00F1..0x00F4` |
| **Button 2** | `Right Key` | Right click switch | **Slot 4** | `0x00CC..0x00CF` | `0x00FD..0x0100` |
| **Button 3** | `Middle Key` | Scroll wheel click switch | **Slot 7** | `0x00D8..0x00DB` | `0x0109..0x010C` |
| **Button 4** | `Forward` | Upper side thumb switch | **Slot 2** | `0x00C4..0x00C7` | `0x00F5..0x00F8` |
| **Button 5** | `Backward` | Lower side thumb switch | **Slot 5** | `0x00D0..0x00D3` | `0x0101..0x0104` |
| **Button 6** | `DPI +` | Top center switch (behind wheel) | **Slot 9** | `0x00E0..0x00E3` | `0x0111..0x0114` |
| **Button 7** | `DPI -` | Bottom center switch (behind DPI+) | **Slot 8** | `0x00DC..0x00DF` | `0x010D..0x0110` |
| *Aux 1* | *Fire / Aux* | Internal virtual slot | Slot 3 | `0x00C8..0x00CB` | `0x00F9..0x00FC` |
| *Aux 2* | *Mode Fn* | Internal virtual slot | Slot 6 | `0x00D4..0x00D7` | `0x0105..0x0108` |
| *Aux 3-5* | *Reserved* | Unwired slots | Slots 10-12 | `0x00E4..0x00EF` | `0x0115..0x0120` |

> **Critical Architecture Note**: The MCU's physical switch index does **not** match sequential numbering. Slot 2 is wired to the Forward side switch, Slot 4 to Right Click, Slot 5 to Backward, Slot 7 to Wheel Click, Slot 8 to DPI-, and Slot 9 to DPI+.

---

## 2. 4-Byte Button Bytecode Structure

Each button binding is encoded as a **4-byte little-endian record**:
```
[ Byte 0: Type,  Byte 1: Param 1,  Byte 2: Param 2,  Byte 3: Param 3 ]
```

### Type 0x01: Standard HID Mouse Actions
```
01 [Mask] 00 [Flags]
```
- `01 01 00 00` -> **Left Click**
- `01 02 00 00` -> **Right Click**
- `01 04 00 00` -> **Middle Click (Wheel Click)**
- `01 08 00 02` -> **Browser Forward (Button 4)**
- `01 10 00 00` -> **Browser Backward (Button 5)**
- `01 20 00 00` -> Double Click (Left)

### Type 0x02: Custom Keyboard Keystroke & Modifiers
```
02 [Modifier_Bitmask] [USB_HID_KeyCode] 00
```
#### Modifier Bitmask (`Byte 1`):
- `0x01`: `Ctrl`
- `0x02`: `Shift`
- `0x04`: `Alt`
- `0x08`: `Win` (Windows / Super key)
- *Combinations are bitwise OR'd*:
  - `0x03`: `Ctrl + Shift`
  - `0x0A`: `Shift + Win` (e.g. Snipping Tool)
  - `0x09`: `Ctrl + Win`

#### USB HID KeyCodes (`Byte 2`):
| Key | Byte 2 Hex | Key | Byte 2 Hex |
|---|---|---|---|
| `A` .. `Z` | `0x04` .. `0x1D` | `1` .. `0` | `0x1E` .. `0x27` |
| `Enter` | `0x28` | `Escape` | `0x29` |
| `Backspace`| `0x2A` | `Tab` | `0x2B` |
| `Spacebar` | `0x2C` | `D` (Desktop) | `0x07` |
| `S` (Snip) | `0x16` | `F1` .. `F12` | `0x3A` .. `0x45` |

#### Verified Examples from Stock App Profiles:
- **`02 08 07 00`**: `Win + D` (**Show Desktop** - Screenshot 2, Button 4)
- **`02 0A 16 00`**: `Shift + Win + S` (**Windows Screen Snip** - Screenshot 2, Button 5)
- **`02 00 2C 00`**: `Spacebar` (Valorant Jump bind)
- **`02 00 14 00`**: `Q` key (Valorant Ability bind)

### Type 0x03: Multimedia & Consumer Audio Controls
```
03 00 [Consumer_Code] 00
```
- `03 00 CD 00` -> Play / Pause
- `03 00 B5 00` -> Next Track
- `03 00 B6 00` -> Previous Track
- `03 00 B7 00` -> Stop
- `03 00 E9 00` -> Volume Up (`Vol +`)
- `03 00 EA 00` -> Volume Down (`Vol -`)
- `03 00 E2 00` -> Mute

### Type 0x06: DPI Loop (Single-Button Cycling)
```
06 92 00 1E
```
- Cycles forward through all active DPI stages (1 -> 2 -> 3 -> 4 -> 5 -> 1).

### Type 0x07: System ACPI & Hardware Control
- `07 80 00 00` -> **LED ON / OFF Toggle** (Screenshot 2, Button 7)
- `07 81 00 00` -> LED Mode Cycle (Breathing / Rainbow / Flow / etc.)
- `07 01 00 00` -> Power Down
- `07 02 00 00` -> Sleep
- `07 04 00 00` -> Wake Up

### Type 0x08: DPI Step Switch
- `08 00 00 00` -> DPI Step Mode Switch

### Type 0x09: Special MCU Functions & Mode Toggles
- `09 00 00 00` -> **DPI -** (Decrease DPI stage) OR **Switch to Office Mode** (Screenshot 2, Button 6)
- `09 01 00 00` -> **Switch to Gaming Mode**
- `09 02 00 00` -> **DPI +** (Increase DPI stage)

### Type 0x0A: Internet Explorer / Web Browser Actions
- `0A 01 00 00` -> Web Home
- `0A 02 00 00` -> Web Search
- `0A 03 00 00` -> Web Back
- `0A 04 00 00` -> Web Forward
- `0A 05 00 00` -> Web Stop
- `0A 06 00 00` -> Web Refresh
- `0A 07 00 00` -> Web Favorites

### Type 0x0B: Windows System Shortcuts
- `0B 01 00 00` -> Open "This PC" (My Computer)
- `0B 02 00 00` -> Open Calculator
- `0B 03 00 00` -> Open E-Mail client
- `0B 04 00 00` -> Minimize All to Desktop

---

## 3. Verified Stock Profiles Comparison

### A. Factory Default (Office Mode — Screenshot 1)
```
Slot 1 (Left Key)    : 01 01 00 00  (Left Click)
Slot 2 (Forward)     : 01 08 00 02  (Browser Forward)
Slot 3 (Aux)         : 06 92 00 1E
Slot 4 (Right Key)   : 01 02 00 00  (Right Click)
Slot 5 (Backward)    : 01 10 00 00  (Browser Backward)
Slot 6 (Aux)         : 09 01 00 00
Slot 7 (Middle Key)  : 01 04 00 00  (Middle Click)
Slot 8 (DPI -)       : 09 00 00 00  (DPI -)
Slot 9 (DPI +)       : 09 02 00 00  (DPI +)
```

### B. Custom Gaming Setup (Gaming Mode — Screenshot 2)
```
Slot 1 (Left Key)    : 01 01 00 00  (Left Click)
Slot 2 (Forward)     : 02 0A 16 00  (Shift + Win + S: Windows Snip)
Slot 3 (Aux)         : 06 12 00 1E
Slot 4 (Right Key)   : 01 02 00 00  (Right Click)
Slot 5 (Backward)    : 02 08 07 00  (Win + D: Show Desktop)
Slot 6 (Aux)         : 07 81 00 00
Slot 7 (Middle Key)  : 01 04 00 00  (Middle Click)
Slot 8 (DPI -)       : 09 00 00 00  (Switch to Office Mode)
Slot 9 (DPI +)       : 07 80 00 00  (LED ON / OFF Toggle)
```

---

## 4. Checksum Algorithm Specification

The Sunplus MCU validates 4 separate profile sectors before loading keybindings into SRAM:
```
Checksum = (Sum_of_Bytes XOR 0x5A) AND 0xFF
```

### Sector Ranges & Checksum Target Offsets:
1. **Sector 1 (Mode & Header)**: `0x00A4..0x00BF` -> Checksum stored at `0x00BF`
2. **Sector 2 (Office Button Table)**: `0x00C0..0x00F0` -> Checksum stored at `0x00F0`
3. **Sector 3 (Gaming Button Table)**: `0x00F1..0x0121` -> Checksum stored at `0x0121`
4. **Sector 4 (Hardware Register Footer)**: `0x03F8..0x03FF` -> Checksum stored at `0x03FF`

If any sector checksum is invalid, the MCU drops all physical button click events.
Our driver automatically validates and recalculates all four checksums whenever a profile is converted or applied.
