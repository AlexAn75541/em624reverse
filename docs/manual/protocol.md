# USB HID Protocol & Checksum Architecture

This chapter specifies the exact communication protocol, control opcodes, packet chunking, and mathematical checksum algorithms reverse-engineered from the Sunplus SPCP6651B microcontroller.

---

## HID Feature Reports

All device configuration, profile burning, and EEPROM dumps occur through two fixed 8-byte HID Feature Reports targeting `Interface 1, Collection 04` (`Usage Page 0xFF00, Usage 0x01`).

### Feature Report 0x40: Control Dispatcher (8 Bytes)

Used to dispatch high-level commands, prepare buffers, trigger soft-applies, and burn EEPROM flash:

```
┌──────┬────────┬────────────┬────────────┬────────────┬────────────┬──────────┬──────────┐
│ Byte0│ Byte 1 │   Byte 2   │   Byte 3   │   Byte 4   │   Byte 5   │  Byte 6  │  Byte 7  │
├──────┼────────┼────────────┼────────────┼────────────┼────────────┼──────────┼──────────┤
│ 0x40 │ Opcode │ Addr (LSB) │ Addr (MSB) │  Len (LSB) │  Len (MSB) │ Reserved │ Reserved │
└──────┴────────┴────────────┴────────────┴────────────┴────────────┴──────────┴──────────┘
```

#### Documented Control Opcodes:

| Opcode | Hex Sequence | Purpose & MCU Action |
|---|---|---|
| `0x07` | `40 07 00 00 08 00 00 00` | **Prepare Write**: Informs the MCU that a 2048-byte (`0x0800`) data stream starting at offset `0x0000` is incoming. |
| `0x0D` | `40 0D 00 00 00 00 00 00` | **Live Soft Apply**: Forces the MCU to load the RAM buffer into active registers without resetting the USB bus. Matches Delphi decompilation `0x0055c789`. |
| `0x0E` | `40 0E 00 00 00 00 00 00` | **Commit Flash & Reset**: Commits the 2048-byte buffer into non-volatile EEPROM and power-cycles the USB bus to re-enumerate. |
| `0x09` | `40 09 00 00 08 00 00 00` | **Prepare Read**: Requests a 2048-byte read of the current EEPROM profile starting at offset `0x0000`. |

---

### Feature Report 0x41: Streaming Data Chunks (8 Bytes)

The Sunplus MCU transfers profile payload memory in **7-byte slices**:

```
┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│  Byte 0  │  Byte 1  │  Byte 2  │  Byte 3  │  Byte 4  │  Byte 5  │  Byte 6  │  Byte 7  │
├──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┤
│   0x41   │ Payload0 │ Payload1 │ Payload2 │ Payload3 │ Payload4 │ Payload5 │ Payload6 │
└──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```

- Total profile length: **2048 bytes**.
- Full chunks: `2048 / 7 = 292` full 7-byte chunks (`2044` bytes).
- Remainder chunk: `2048 % 7 = 4` bytes, padded with `0xFF` to 7 bytes.
- Total chunks per flash operation: **293 chunks**.
- Inter-chunk delay: `4 ms` to `6 ms` (required to prevent MCU buffer overrun).

---

## 5-Sector Hardware Checksum Algorithm

The Sunplus SPCP6651B microcontroller validates five distinct profile sectors. If **any** sector checksum is invalid, the MCU disables button event dispatches or rejects the EEPROM write.

### Universal Formula:
```
Sector_Checksum = (SUM(Sector_Bytes) ^ 0x5A) & 0xFF
```

### The 5 Checksum Sectors:

```
0x0000 ┌────────────────────────────────────────────────────────┐
       │ Sector 0: Header, Magic, DPI, Colors (163 bytes)        │
0x00A2 ├────────────────────────────────────────────────────────┤
0x00A3 │ [CHECKSUM 0] = (sum(0x0000..0x00A2) ^ 0x5A) & 0xFF    │
       ├────────────────────────────────────────────────────────┤
       │ Sector 1: Mode & Polling Control Registers (27 bytes)   │
0x00BE ├────────────────────────────────────────────────────────┤
0x00BF │ [CHECKSUM 1] = (sum(0x00A4..0x00BE) ^ 0x5A) & 0xFF    │
       ├────────────────────────────────────────────────────────┤
       │ Sector 2: Office Mode Button Table (48 bytes)          │
0x00EF ├────────────────────────────────────────────────────────┤
0x00F0 │ [CHECKSUM 2] = (sum(0x00C0..0x00EF) ^ 0x5A) & 0xFF    │
       ├────────────────────────────────────────────────────────┤
       │ Sector 3: Gaming Mode Button Table (48 bytes)          │
0x0120 ├────────────────────────────────────────────────────────┤
0x0121 │ [CHECKSUM 3] = (sum(0x00F1..0x0120) ^ 0x5A) & 0xFF    │
       ├────────────────────────────────────────────────────────┤
       │ Profile Body / Macro Bytecode Storage                   │
       │ ...                                                    │
0x03F8 ├────────────────────────────────────────────────────────┤
       │ Sector 4: Hardware Footer Table (7 bytes)              │
0x03FE ├────────────────────────────────────────────────────────┤
0x03FF │ [CHECKSUM 4] = (sum(0x03F8..0x03FE) ^ 0x5A) & 0xFF    │
0x0400 └────────────────────────────────────────────────────────┘
```

```rust
// Verified Implementation in src/profile.rs:
fn calculate_checksum(slice: &[u8]) -> u8 {
    let sum: u32 = slice.iter().map(|&b| b as u32).sum();
    ((sum ^ 0x5A) & 0xFF) as u8
}
```

---

## The Complete Write Sequence

When the user clicks **Apply to Mouse** in the configurator, the driver executes the following sequence:

1. **Safety Sanitization**:
   - Forces `active_stages_mask &= 0xDF` (disables Stage 6 lockup).
   - Enforces `0x0006 = 0x7F` (180° sensor orientation).
   - Synchronizes button tables between Office (`0x00C0`) and Gaming (`0x00F1`).
   - Recalculates all 5 checksums.
2. **Transfer Initialization**:
   - Dispatches Opcode `0x07` (`40 07 00 00 08 00 00 00`) via `HidD_SetFeature`.
   - Waits 10ms.
3. **Chunk Streaming**:
   - Dispatches 293 Report `0x41` packets with 4ms delay between packets.
4. **Soft Application**:
   - Dispatches Opcode `0x0D` (`40 0d 00 00 00 00 00 00`) to load incoming registers immediately.
   - Waits 50ms.
5. **Flash Burn & Reset**:
   - Dispatches Opcode `0x0E` (`40 0e 00 00 00 00 00 00`).
   - The MCU burns RAM to persistent flash and resets its USB controller.
   - Windows returns error code `0x0000001F` (`ERROR_GEN_FAILURE`), and Linux returns `-EPIPE`. The driver explicitly intercepts this code and treats it as a **successful hardware burn confirmation**.
