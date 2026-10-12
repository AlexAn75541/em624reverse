#!/usr/bin/env python3
"""
EM624 Mouse Profile Converter & Fixer
Converts any EM624 2048-byte profile .bin file into a fully compatible,
stable profile for the mouse hardware and modern driver app.

Features:
- Fixes MCU crash/lockup by safely masking out invalid Stage 6 (byte 4 &= 0xDF)
- Option to restore 100% standard buttons (Left, Right, Middle, Forward, Backward, CPI Loop)
  or preserve custom profile keybindings
- Recomputes exact hardware checksums: (sum(bytes) ^ 0x5A) & 0xFF
"""

import argparse
import os
import shutil
import sys

PROFILE_SIZE = 2048
PROFILE_MAGIC = bytes([0x1B, 0xCF, 0x08, 0xB8])

# Factory standard 12-key button mapping (4 bytes per key, 48 bytes total)
# Key 1: Left Click (0x01, 0x01)
# Key 2: Forward Side Button 1 (0x01, 0x08)
# Key 3: CPI Loop (0x06, 0x92)
# Key 4: Right Click (0x01, 0x02)
# Key 5: Backward Side Button 2 (0x01, 0x10)
# Key 7: Middle Click (0x01, 0x04)
STANDARD_BUTTON_TABLE = bytes([
    0x01, 0x01, 0x00, 0x00, # Key 1: Left Click
    0x01, 0x08, 0x00, 0x02, # Key 2: Forward (Side 1)
    0x06, 0x92, 0x00, 0x1E, # Key 3: CPI Loop
    0x01, 0x02, 0x00, 0x00, # Key 4: Right Click
    0x01, 0x10, 0x00, 0x00, # Key 5: Backward (Side 2)
    0x09, 0x01, 0x00, 0x00, # Key 6: Fn
    0x01, 0x04, 0x00, 0x00, # Key 7: Middle Click
    0x09, 0x00, 0x00, 0x00, # Key 8: Fn
    0x09, 0x02, 0x00, 0x00, # Key 9: CPI +
    0x00, 0x00, 0x00, 0x00, # Key 10
    0x00, 0x00, 0x00, 0x00, # Key 11
    0x00, 0x00, 0x00, 0x00, # Key 12
])

def calc_checksum(data: bytes) -> int:
    return (sum(data) ^ 0x5A) & 0xFF

def convert_profile_bytes(raw_bytes: bytes, button_mode: str = "standard") -> bytearray:
    if len(raw_bytes) != PROFILE_SIZE:
        raise ValueError(f"Invalid profile size: {len(raw_bytes)} bytes (expected {PROFILE_SIZE})")
    if raw_bytes[:4] != PROFILE_MAGIC:
        raise ValueError(f"Invalid magic header: {raw_bytes[:4].hex()} (expected {PROFILE_MAGIC.hex()})")

    buf = bytearray(raw_bytes)

    # 1. Hardware safety registers (fix Stage 6 lockup, clear 0x80 lockout flag)
    buf[4] &= 0xDF       # Safe active DPI stages (mask bit 5)
    buf[0x03F9] &= 0x7F  # Clear 0x80 hardware button lockout flag

    # 1b. Clamp all 6 DPI stage nibbles to max 12 (8000 DPI)
    # Sensor calibration lookup table overflows on nibble > 12 (e.g. 14 = 10000+ DPI),
    # causing the SPCP6662 MCU button interrupt dispatch loop to trap.
    for off in range(0x2C, 0x2F):
        low = min(buf[off] & 0x0F, 12)
        high = min((buf[off] >> 4) & 0x0F, 12)
        buf[off] = low | (high << 4)

    # 2. Button handling
    if button_mode == "standard":
        buf[0x00A4] = 0x01 # Force Gaming Mode
        buf[0x00A8] = 0x0F
        buf[0x00AA] = 0x18
        buf[0x00C0:0x00F0] = STANDARD_BUTTON_TABLE # Office Mode table
        buf[0x00F1:0x0121] = STANDARD_BUTTON_TABLE # Gaming Mode table
    elif button_mode == "keep":
        pass # Preserve profile's own bindings and mode flags 100% untouched
    else:
        raise ValueError(f"Unknown button mode: {button_mode}")

    # 3. Recalculate all 5 hardware checksums:
    buf[0x00A3] = calc_checksum(buf[0x0000:0x00A3]) # Sector 0 (Magic, DPI, RGB)
    buf[0x00BF] = calc_checksum(buf[0x00A4:0x00BF]) # Sector 1 (Mode & Control)
    buf[0x00F0] = calc_checksum(buf[0x00C0:0x00F0]) # Sector 2 (Office Table)
    buf[0x0121] = calc_checksum(buf[0x00F1:0x0121]) # Sector 3 (Gaming Table)
    buf[0x03FF] = calc_checksum(buf[0x03F8:0x03FF]) # Sector Footer

    return buf

def process_file(file_path: str, output_path: str = None, button_mode: str = "standard", backup: bool = True):
    print(f"Processing: {file_path}")
    with open(file_path, "rb") as f:
        data = f.read()

    converted = convert_profile_bytes(data, button_mode=button_mode)

    dest = output_path or file_path
    if dest == file_path and backup:
        bak_path = file_path + ".bak"
        shutil.copyfile(file_path, bak_path)
        print(f"  -> Original backed up to: {bak_path}")

    with open(dest, "wb") as f:
        f.write(converted)
    print(f"  -> Converted profile saved to: {dest} (Button mode: {button_mode})")

def main():
    parser = argparse.ArgumentParser(description="EM624 Profile Converter & Fixer")
    parser.add_argument("path", help="Path to a profile .bin file or directory of profiles")
    parser.add_argument("-o", "--output", help="Output file path (default: overwrite input with backup)")
    parser.add_argument("-b", "--buttons", choices=["standard", "keep"], default="standard",
                        help="Button mapping mode: 'standard' (default: Left, Right, Middle, Forward, Backward, CPI Loop) or 'keep' (preserve custom game bindings)")
    parser.add_argument("--no-backup", action="store_true", help="Do not create .bak backup files")

    args = parser.parse_args()

    if os.path.isdir(args.path):
        for root, _, files in os.walk(args.path):
            for file in sorted(files):
                if file.endswith(".bin") and not file.endswith(".bin.bak"):
                    fp = os.path.join(root, file)
                    process_file(fp, button_mode=args.buttons, backup=not args.no_backup)
    elif os.path.isfile(args.path):
        process_file(args.path, output_path=args.output, button_mode=args.buttons, backup=not args.no_backup)
    else:
        print(f"Error: Path not found: {args.path}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
