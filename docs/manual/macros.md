# Hardware Macro Bytecode Engine

The EM624 mouse features an onboard EEPROM macro execution bank located between offsets `0x0400` and `0x07FF` (1,024 bytes). This enables hardware-level execution of complex keyboard and mouse sequences stored directly on the mouse MCU.

---

## Bytecode Instruction Architecture

The Sunplus SPCP6651B macro virtual machine reads variable-length bytecode instructions:

### 1. Key Down Instruction
```
[0x01, KeyCode, 0x00]
```
- `0x01`: Opcode `KEY_DOWN`
- `KeyCode`: USB HID scan code (e.g. `0x2C` for Space, `0x04` for 'A')

### 2. Key Up Instruction
```
[0x02, KeyCode, 0x00]
```
- `0x02`: Opcode `KEY_UP`

### 3. Mouse Button Down / Up Instruction
```
[0x03, ButtonMask, 0x00]   # Mouse Down
[0x04, ButtonMask, 0x00]   # Mouse Up
```
- Button Mask: `0x01` = Left, `0x02` = Right, `0x04` = Middle

### 4. Millisecond Delay Instruction
```
[0x05, DurationLSB, DurationMSB]
```
- Duration encoded as a 16-bit little-endian integer in milliseconds (e.g., `[0x05, 0x32, 0x00]` for 50 ms).

### 5. Sequence Terminator
```
[0xFF, 0xFF]
```
- Halts macro playback in the MCU virtual machine.

---

## Playback Loop Modes

When binding a macro to a physical mouse button, the configurator supports three distinct execution models:

1. **Play Once**: Executing the sequence exactly once on button press.
2. **Hold Until Released**: Loops the macro continuously as long as the physical button remains depressed, immediately stopping upon button release.
3. **Repeat N Times**: Repeats the complete bytecode sequence for a fixed number of iterations (1..255).
