#!/usr/bin/env bash
# ==============================================================================
# E-DRA EM624 Wired Gaming Mouse (VID: 0x1BCF, PID: 0x08B8)
# Multifunction Hardware Probe, LED Lighting Sync/Change Test, Automated Test,
# & Deep Register Debug Script (Linux / usbipd-win USB/IP Forwarded Environment)
#
# Features:
#   - Automatic .log capture on every launch:
#       logs/em624_tool_<YYYYMMDD_HHMMSS>.log + logs/em624_latest.log
#   - Full verbose mode (-v / --verbose, ON by default)
#   - Subcommands:
#       ./scripts/em624_tool.sh all        (Run probe + auto-test + led-test + debug)
#       ./scripts/em624_tool.sh probe      (USB/IP vhci_hcd, hidraw, descriptor & live probe)
#       ./scripts/em624_tool.sh led-test   (Live + offline LED mode/speed/brightness/RGB sync test)
#       ./scripts/em624_tool.sh auto-test  (Automated 5-sector checksums, invariants & cargo test)
#       ./scripts/em624_tool.sh debug [p1] [p2] (Deep register decoder & profile diff)
#       ./scripts/em624_tool.sh sniff      (usbmon / tshark / tcpdump capture on usbip bus)
#
# Immutability Mandate:
#   NEVER modifies `exampleProfiles/*.bin`. All temporary/backup profiles are
#   written exclusively under `logs/*.em624`.
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/logs"
mkdir -p "$LOG_DIR"

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
SUBCMD="${1:-all}"
VERBOSE=1
LIVE_DELAY="1.2"
CUSTOM_LOG=""
ARG_PROFILE1=""
ARG_PROFILE2=""

# Parse flags
shift_count=0
if [[ $# -gt 0 && ! "$1" =~ ^- ]]; then
    SUBCMD="$1"
    shift
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        -q|--quiet)
            VERBOSE=0
            shift
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        --delay)
            LIVE_DELAY="${2:-1.2}"
            shift 2
            ;;
        --log-file)
            CUSTOM_LOG="${2:-}"
            shift 2
            ;;
        -h|--help)
            cat <<EOF
Usage: $0 [all|probe|led-test|auto-test|debug|sniff] [options]

Subcommands:
  all         Run probe, auto-test, led-test (with live USB/IP hardware if attached), and debug (default)
  probe       Inspect Linux kernel, usbipd (vhci_hcd), /dev/hidraw*, report descriptors, and live EEPROM readback
  led-test    Verify LED Lighting Sync & live lighting changes across modes (0..7), speeds (1..8), brightness, & RGB
  auto-test   Run automated 5-sector checksum, sensor orientation (0x7F), button matrix, and Rust unit tests
  debug       Decode 2048-byte register map and diff two profiles
  sniff       Capture raw USB traffic on the usbipd bus via tshark/tcpdump

Options:
  -v, --verbose       Enable full verbose output (default: enabled)
  -q, --quiet         Quiet console output (full verbose trace still written to .log file)
  --delay <sec>       Delay in seconds between live LED hardware transitions (default: 1.2)
  --log-file <path>   Custom .log file output path
EOF
            exit 0
            ;;
        *)
            if [[ -z "$ARG_PROFILE1" ]]; then
                ARG_PROFILE1="$1"
            elif [[ -z "$ARG_PROFILE2" ]]; then
                ARG_PROFILE2="$1"
            fi
            shift
            ;;
    esac
done

LOG_FILE="${CUSTOM_LOG:-$LOG_DIR/em624_tool_${TIMESTAMP}.log}"
LATEST_LOG="$LOG_DIR/em624_latest.log"
mkdir -p "$(dirname "$LOG_FILE")"
: > "$LOG_FILE"
: > "$LATEST_LOG"

# Ensure /dev/hidraw* nodes for 1BCF:08B8 have read/write permissions before teeing output
for hpath in /sys/class/hidraw/hidraw*; do
    if [[ -e "$hpath/device/uevent" ]]; then
        uev="$(tr '\n' ' ' < "$hpath/device/uevent" 2>/dev/null || true)"
        if [[ "${uev^^}" == *"1BCF"* && "${uev^^}" == *"08B8"* ]]; then
            devnode="/dev/$(basename "$hpath")"
            if [[ -e "$devnode" && ( ! -r "$devnode" || ! -w "$devnode" ) ]]; then
                echo "[PERM] $devnode is owned by root (crw-------). Requesting sudo to set 0666 permissions..."
                sudo chmod 0666 "$devnode" || echo "[WARN] Could not chmod $devnode; run 'sudo ./em624_tool.sh' for live USB/IP hardware I/O."
            fi
        fi
    fi
done

# Redirect all stdout and stderr through timestamped logger into both .log files
exec > >(
    awk -v logfile="$LOG_FILE" -v latest="$LATEST_LOG" '
    {
        cmd = "date +\"%Y-%m-%d %H:%M:%S.%3N\""
        cmd | getline ts
        close(cmd)
        line = sprintf("[%s] %s", ts, $0)
        print line >> logfile
        fflush(logfile)
        print line >> latest
        fflush(latest)
        print $0
        fflush()
    }'
) 2>&1

echo "=============================================================================="
echo "  E-DRA EM624 Wired Gaming Mouse (VID: 0x1BCF, PID: 0x08B8) Multifunction Tool"
echo "  Command     : $SUBCMD"
echo "  Verbose     : $VERBOSE"
echo "  Log File    : $LOG_FILE"
echo "  Latest Log  : $LATEST_LOG"
echo "  Host Kernel : $(uname -srmo)"
echo "=============================================================================="

# Export environment variables for the embedded Python hardware/HID ioctl engine
export EM624_REPO_ROOT="$REPO_ROOT"
export EM624_LOG_DIR="$LOG_DIR"
export EM624_SUBCMD="$SUBCMD"
export EM624_VERBOSE="$VERBOSE"
export EM624_LIVE_DELAY="$LIVE_DELAY"
export EM624_ARG_PROFILE1="$ARG_PROFILE1"
export EM624_ARG_PROFILE2="$ARG_PROFILE2"

# ==============================================================================
# Stage 1: Shell-Level Linux / usbipd / Kernel / hidraw Diagnostic Check
# ==============================================================================
run_shell_usbip_diagnostics() {
    echo ""
    echo "------------------------------------------------------------------------------"
    echo "  [STAGE 1] LINUX USB/IP (usbipd) & KERNEL HIDRAW DIAGNOSTICS"
    echo "------------------------------------------------------------------------------"

    echo "[KERNEL] Loaded USB / VHCI / HID modules:"
    if [[ -r /proc/modules ]]; then
        grep -E 'vhci|usbip|usbhid|hid_generic|usbmon' /proc/modules || echo "  (Built-in or no matching module rows in /proc/modules)"
    fi

    echo ""
    echo "[USBIP-VHCI] Checking virtual USB/IP host controllerstatus (/sys/devices/platform/vhci_hcd*):"
    local vhci_found=0
    for status_file in /sys/devices/platform/vhci_hcd*/status*; do
        if [[ -r "$status_file" ]]; then
            vhci_found=1
            echo "  File: $status_file"
            awk 'NR==1 || $3 != "000" { print "    " $0 }' "$status_file"
        fi
    done
    if [[ "$vhci_found" -eq 0 ]]; then
        echo "  [INFO] No /sys/devices/platform/vhci_hcd*/status* exposed in sysfs."
    fi

    echo ""
    echo "[SYSFS-USB] Scanning /sys/bus/usb/devices for VID=1bcf, PID=08b8:"
    local usb_found=0
    for devpath in /sys/bus/usb/devices/*; do
        if [[ -r "$devpath/idVendor" && -r "$devpath/idProduct" ]]; then
            local vid pid prod mfg busnum devnum
            vid="$(tr -d '\n' < "$devpath/idVendor" 2>/dev/null || true)"
            pid="$(tr -d '\n' < "$devpath/idProduct" 2>/dev/null || true)"
            if [[ "${vid,,}" == "1bcf" && "${pid,,}" == "08b8" ]]; then
                usb_found=1
                prod="$(tr -d '\n' < "$devpath/product" 2>/dev/null || echo "Unknown")"
                mfg="$(tr -d '\n' < "$devpath/manufacturer" 2>/dev/null || echo "SunplusIT")"
                busnum="$(tr -d '\n' < "$devpath/busnum" 2>/dev/null || echo "?")"
                devnum="$(tr -d '\n' < "$devpath/devnum" 2>/dev/null || echo "?")"
                echo "  [FOUND] $devpath -> Bus $busnum Dev $devnum | ${vid}:${pid} | Product: '$prod' | Manufacturer: '$mfg'"
            fi
        fi
    done
    if [[ "$usb_found" -eq 0 ]]; then
        echo "  [WARN] 1bcf:08b8 not currently visible in /sys/bus/usb/devices (check 'usbipd attach --wsl --busid <BUSID>' on Windows if detached)."
    fi

    if command -v lsusb >/dev/null 2>&1; then
        echo ""
        echo "[LSUSB] Output for 1bcf:08b8:"
        lsusb -d 1bcf:08b8 || echo "  (lsusb returned no matching 1bcf:08b8 device)"
    fi

    echo ""
    echo "[HIDRAW] Scanning /sys/class/hidraw/hidraw* nodes:"
    local hid_found=0
    for hpath in /sys/class/hidraw/hidraw*; do
        if [[ -e "$hpath/device/uevent" ]]; then
            local hname devnode uev perms
            hname="$(basename "$hpath")"
            devnode="/dev/$hname"
            uev="$(tr '\n' ' ' < "$hpath/device/uevent" 2>/dev/null || true)"
            if [[ "${uev^^}" == *"1BCF"* && "${uev^^}" == *"08B8"* ]]; then
                hid_found=1
                if [[ ! -r "$devnode" || ! -w "$devnode" ]]; then
                    echo "  [PERM] Granting read/write access (0666) on $devnode via sudo..."
                    sudo chmod 0666 "$devnode" 2>/dev/null || true
                fi
                perms="$(ls -l "$devnode" 2>/dev/null || echo "node missing")"
                echo "  [MATCH] $devnode -> $uev"
                echo "          Permissions: $perms"
                if [[ -r "$hpath/device/report_descriptor" ]]; then
                    local rdesc_hex
                    rdesc_hex="$(od -An -tx1 -v "$hpath/device/report_descriptor" | tr -d ' \n')"
                    if [[ "$rdesc_hex" == *"0600ff0901"* ]]; then
                        echo "          Collection : VENDOR CONFIG INTERFACE (Usage Page 0xFF00, Usage 0x01, Report IDs 0x40 & 0x41)"
                    else
                        echo "          Collection : Standard HID Mouse / Keyboard / Consumer Interface"
                    fi
                fi
            fi
        fi
    done
    if [[ "$hid_found" -eq 0 ]]; then
        echo "  [WARN] No /dev/hidraw* nodes matching 1BCF:08B8 found."
    fi
}

if [[ "$SUBCMD" == "sniff" ]]; then
    run_shell_usbip_diagnostics
    echo ""
    echo "------------------------------------------------------------------------------"
    echo "  [SNIFF] CAPTURING RAW USB TRAFFIC TO $LOG_DIR/usb_capture_${TIMESTAMP}.pcap"
    echo "------------------------------------------------------------------------------"
    if command -v tshark >/dev/null 2>&1; then
        tshark -i usbmon0 -w "$LOG_DIR/usb_capture_${TIMESTAMP}.pcap"
    elif command -v tcpdump >/dev/null 2>&1; then
        tcpdump -i usbmon0 -w "$LOG_DIR/usb_capture_${TIMESTAMP}.pcap"
    else
        echo "[ERROR] Neither tshark nor tcpdump is installed for usbmon capture."
        exit 1
    fi
    exit 0
fi

if [[ "$SUBCMD" == "all" || "$SUBCMD" == "probe" || "$SUBCMD" == "led-test" ]]; then
    run_shell_usbip_diagnostics
fi

# ==============================================================================
# Stage 2: Direct Linux hidraw ioctl + Register/LED/Checksum Engine
# ==============================================================================
python3 - <<'PYEOF'
import ctypes
import fcntl
import glob
import hashlib
import os
import pathlib
import struct
import subprocess
import sys
import time
from typing import Dict, List, Optional, Tuple

REPO_ROOT = pathlib.Path(os.environ["EM624_REPO_ROOT"])
LOG_DIR = pathlib.Path(os.environ["EM624_LOG_DIR"])
SUBCMD = os.environ["EM624_SUBCMD"]
VERBOSE = int(os.environ.get("EM624_VERBOSE", "1")) == 1
LIVE_DELAY = float(os.environ.get("EM624_LIVE_DELAY", "1.2"))
ARG_P1 = os.environ.get("EM624_ARG_PROFILE1", "")
ARG_P2 = os.environ.get("EM624_ARG_PROFILE2", "")

VID = 0x1BCF
PID = 0x08B8
PROFILE_SIZE = 2048

DPI_TABLE = [200, 400, 600, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4000, 4800, 6400, 8000, 10000, 12000]
LED_SPEED_TABLE = [0x0A, 0x28, 0x46, 0x64, 0x82, 0xA0, 0xBE, 0xDC]
LED_MODES = {
    0: ("DPI Breathing", 0x81, 0x00),
    1: ("Multicolor Breathing", 0x82, 0x01),
    2: ("Rainbow / Steady", 0x83, 0x02),
    3: ("Floe Light (Flow)", 0x84, 0x03),
    4: ("Waltz", 0x85, 0x04),
    5: ("Four Seasons", 0x86, 0x05),
    6: ("Follow / Reactive", 0x87, None),
    7: ("LED OFF", 0x01, None),
}
BUTTON_SLOT_MAP = {
    1: (0, "Left Click"),
    2: (3, "Right Click"),
    3: (6, "Middle Click"),
    4: (4, "Forward (Side 1 Front)"),
    5: (1, "Backward (Side 2 Rear)"),
    6: (5, "DPI + (Top Front)"),
    7: (7, "DPI - / Loop (Top Back)"),
}


def vprint(msg: str):
    if VERBOSE:
        print(f"  [VERBOSE] {msg}")


# Linux hidraw ioctl constants for 8-byte Feature Reports (0x40 and 0x41)
# _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x06, len) and _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x07, len)
def HIDIOCSFEATURE(length: int) -> int:
    return (3 << 30) | (length << 16) | (ord('H') << 8) | 0x06


def HIDIOCGFEATURE(length: int) -> int:
    return (3 << 30) | (length << 16) | (ord('H') << 8) | 0x07


class HidrawVendorDevice:
    """Direct Linux /dev/hidraw* Feature Report 0x40/0x41 transceiver for 1BCF:08B8 Col04 (0xFF00:0x0001)."""

    def __init__(self, devnode: str):
        self.devnode = devnode
        self.fd = os.open(devnode, os.O_RDWR)

    def close(self):
        if self.fd is not None:
            try:
                os.close(self.fd)
            except OSError:
                pass
            self.fd = None

    def set_feature(self, report8: bytes) -> None:
        assert len(report8) == 8, "Feature Report must be 8 bytes"
        buf = (ctypes.c_ubyte * 8).from_buffer_copy(report8)
        vprint(f"TX SetFeature({self.devnode}): [{report8.hex(' ')}]")
        fcntl.ioctl(self.fd, HIDIOCSFEATURE(8), buf, True)

    def get_feature(self, report_id: int) -> bytes:
        buf = (ctypes.c_ubyte * 8)()
        buf[0] = report_id & 0xFF
        fcntl.ioctl(self.fd, HIDIOCGFEATURE(8), buf, True)
        out = bytes(buf)
        vprint(f"RX GetFeature({self.devnode}, 0x{report_id:02X}): [{out.hex(' ')}]")
        return out

    def query_status_0x41(self) -> bytes:
        time.sleep(0.005)
        return self.get_feature(0x41)

    def send_cmd_0x40(self, b1: int, b2: int, b3: int, b4: int, b5: int, b6: int, b7: int) -> bytes:
        pkt = bytes([0x40, b1 & 0xFF, b2 & 0xFF, b3 & 0xFF, b4 & 0xFF, b5 & 0xFF, b6 & 0xFF, b7 & 0xFF])
        self.set_feature(pkt)
        time.sleep(0.005)
        return self.query_status_0x41()

    def write_registers_0x05(self, start_addr: int, data: bytes) -> None:
        for i in range(0, len(data), 3):
            chunk = data[i:i + 3]
            addr = start_addr + i
            b = [0x40, 0x05, (addr >> 8) & 0xFF, addr & 0xFF, 0x00, 0x00, 0x00, len(chunk)]
            for j, val in enumerate(chunk):
                b[4 + j] = val
            self.set_feature(bytes(b))
            time.sleep(0.005)
            self.query_status_0x41()

    def read_profile_2048(self) -> bytes:
        print(f"  [HIDRAW] Requesting 2048-byte hardware profile from {self.devnode} (Opcode 0x40 0x09)...")
        ack = self.send_cmd_0x40(0x09, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00)
        print(f"  [HIDRAW] Opcode 0x09 Handshake ACK: [{ack.hex(' ')}]")
        out = bytearray()
        for chunk_idx in range(293):
            resp = self.get_feature(0x41)
            take = min(7, PROFILE_SIZE - len(out))
            out.extend(resp[1:1 + take])
        return bytes(out)

    def sync_led_live(self, profile: bytes) -> None:
        # 1. Write 0x0036..0x004D (LED speed 0x0038 + 6 stages 3-byte RGB 0x0039..0x004A)
        self.write_registers_0x05(0x0036, profile[0x0036:0x004E])
        # 2. Write Sector 0 checksum 0x00A3
        self.write_registers_0x05(0x00A3, profile[0x00A3:0x00A4])
        # 3. Write Sector 1 0x00A4..0x00BF (Mode 0x00A4, LED Mode 0x00A5, Stage 0x00A6, Brightness 0x00A7, Checksum 0x00BF)
        self.write_registers_0x05(0x00A4, profile[0x00A4:0x00C0])
        # 4. Write MCULED sub-controller direct commands (0x40 0x01 0x07)
        sub_mode = profile[0x03F8]
        if sub_mode <= 5:
            self.send_cmd_0x40(0x01, 0x07, sub_mode, 0x00, 0x00, 0x00, 0x00)
            self.send_cmd_0x40(0x01, 0x07, profile[0x03FA], 0x00, 0x00, 0x00, 0x00)


def discover_vendor_hidraw() -> Tuple[Optional[str], Optional[str]]:
    """
    Find the /dev/hidraw* node corresponding to E-DRA EM624 Vendor Collection Col04
    (Usage Page 0xFF00, Usage 0x01, Report IDs 0x40 and 0x41).
    Returns (devnode, error_reason).
    """
    candidates = []
    for hdir in sorted(glob.glob("/sys/class/hidraw/hidraw*")):
        uev_file = pathlib.Path(hdir) / "device" / "uevent"
        rdesc_file = pathlib.Path(hdir) / "device" / "report_descriptor"
        if not uev_file.exists():
            continue
        uev = uev_file.read_text(errors="ignore").upper()
        if "1BCF" in uev and "08B8" in uev:
            devnode = f"/dev/{pathlib.Path(hdir).name}"
            is_vendor = False
            if rdesc_file.exists():
                rdesc = rdesc_file.read_bytes()
                if b"\x06\x00\xff\x09\x01" in rdesc or b"\x85\x40" in rdesc:
                    is_vendor = True
            candidates.append((devnode, is_vendor))

    if not candidates:
        return None, "No /dev/hidraw* device with VID 1BCF & PID 08B8 found in sysfs."

    # Prefer the exact vendor collection node (0xFF00:0x0001)
    vendor_nodes = [c[0] for c in candidates if c[1]]
    target = vendor_nodes[0] if vendor_nodes else candidates[-1][0]
    if not os.access(target, os.R_OK | os.W_OK):
        return target, f"Permission denied on {target} (run with sudo or install scripts/install_udev.sh)."
    return target, None


# ==============================================================================
# Checksum & Profile Helpers
# ==============================================================================
def calc_checksum(data: bytes) -> int:
    return (sum(data) ^ 0x5A) & 0xFF


def verify_all_checksums(buf: bytes):
    return [
        ("Sector 0 (0x0000..0x00A2 -> 0x00A3)", buf[0x00A3], calc_checksum(buf[0x0000:0x00A3])),
        ("Sector 1 (0x00A4..0x00BE -> 0x00BF)", buf[0x00BF], calc_checksum(buf[0x00A4:0x00BF])),
        ("Sector 2 (0x00C0..0x00EF -> 0x00F0)", buf[0x00F0], calc_checksum(buf[0x00C0:0x00F0])),
        ("Sector 3 (0x00F1..0x0120 -> 0x0121)", buf[0x0121], calc_checksum(buf[0x00F1:0x0121])),
        ("Footer   (0x03F8..0x03FE -> 0x03FF)", buf[0x03FF], calc_checksum(buf[0x03F8:0x03FF])),
    ]


def recalculate_checksums(buf: bytearray) -> None:
    buf[0x00A3] = calc_checksum(buf[0x0000:0x00A3])
    buf[0x00BF] = calc_checksum(buf[0x00A4:0x00BF])
    buf[0x00F0] = calc_checksum(buf[0x00C0:0x00F0])
    buf[0x0121] = calc_checksum(buf[0x00F1:0x0121])
    buf[0x03FF] = calc_checksum(buf[0x03F8:0x03FF])


def set_led_mode(buf: bytearray, mode: int) -> None:
    clean = max(0, min(7, mode))
    if clean >= 7:
        base = max(1, min(7, buf[0x00A5] & 0x7F))
        buf[0x00A5] = base
    else:
        buf[0x00A5] = 0x80 | (clean + 1)
        if clean <= 5:
            buf[0x03F8] = clean
            buf[0x03F9] = 0x04
    recalculate_checksums(buf)


def get_led_mode(buf: bytes) -> int:
    m = buf[0x00A5]
    if (m & 0x80) == 0:
        return 7
    return min(6, max(0, (m & 0x7F) - 1))


def set_led_speed(buf: bytearray, speed: int) -> None:
    clamped = max(1, min(8, speed))
    buf[0x0038] = LED_SPEED_TABLE[clamped - 1]
    mcu_step = max(0, 5 - min(5, clamped))
    buf[0x03FA] = mcu_step | 0x10
    buf[0x03FB] = mcu_step
    buf[0x03FC] = mcu_step
    buf[0x03FD] = mcu_step
    recalculate_checksums(buf)


def get_led_speed(buf: bytes) -> int:
    v = buf[0x0038]
    for idx, threshold in enumerate([0x19, 0x37, 0x55, 0x73, 0x91, 0xAF, 0xCD], start=1):
        if v <= threshold:
            return idx
    return 8


def set_led_brightness(buf: bytearray, dim: bool) -> None:
    buf[0x00A7] = 0x44 if dim else 0x84
    buf[0x00A5] |= 0x80
    recalculate_checksums(buf)


def get_led_brightness(buf: bytes) -> int:
    return 1 if (buf[0x00A7] & 0xF0) == 0x40 else 0


def set_stage_rgb(buf: bytearray, stage_idx: int, r: int, g: int, b: int) -> None:
    off = 0x0039 + stage_idx * 3
    buf[off] = r & 0xFF
    buf[off + 1] = g & 0xFF
    buf[off + 2] = b & 0xFF
    recalculate_checksums(buf)


def get_stage_rgb(buf: bytes, stage_idx: int) -> Tuple[int, int, int]:
    off = 0x0039 + stage_idx * 3
    return (buf[off], buf[off + 1], buf[off + 2])


# ==============================================================================
# Subcommand Implementations
# ==============================================================================
def do_probe() -> bool:
    print("\n==============================================================================")
    print("  [PROBE] HARDWARE HANDSHAKE, LIVE EEPROM READBACK & REFERENCE PROFILES")
    print("==============================================================================")
    devnode, err = discover_vendor_hidraw()
    if devnode and not err:
        print(f"[LIVE-USBIP] Opening vendor configuration node: {devnode}")
        dev = HidrawVendorDevice(devnode)
        try:
            ack = dev.send_cmd_0x40(0x0D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00)
            print(f"[LIVE-USBIP] Handshake Opcode 0x0D -> 0x41 Response: [{ack.hex(' ')}]")
            live_buf = dev.read_profile_2048()
            live_out = LOG_DIR / "live_readback.em624"
            live_out.write_bytes(live_buf)
            print(f"[LIVE-USBIP] Saved 2048-byte live EEPROM readback to: {live_out}")
            for name, st, ex in verify_all_checksums(live_buf):
                print(f"  {name}: stored=0x{st:02X}, expected=0x{ex:02X} ({'OK' if st == ex else 'MISMATCH'})")
        finally:
            dev.close()
    else:
        print(f"[LIVE-USBIP] Live hardware probe status: {err}")

    print("\n[REFERENCE PROFILES] Checking read-only `exampleProfiles/*.bin`:")
    all_ok = True
    for bf in sorted((REPO_ROOT / "exampleProfiles").glob("*.bin")):
        raw = bf.read_bytes()
        chks = verify_all_checksums(raw)
        ok = all(st == ex for _, st, ex in chks) and len(raw) == PROFILE_SIZE
        all_ok = all_ok and ok
        mode_s = "Gaming(0x01)" if raw[0xA4] == 1 else "Office(0x00)"
        led_m = get_led_mode(raw)
        print(
            f"  - {bf.name:18s} | {len(raw)}B | Mode:{mode_s} | "
            f"LED:0x{raw[0xA5]:02X} ({LED_MODES[led_m][0]}) | Speed:0x{raw[0x38]:02X} | "
            f"Bright:0x{raw[0xA7]:02X} | Checksums:{'PASS' if ok else 'FAIL'}"
        )
    return all_ok


def do_led_test() -> bool:
    print("\n==============================================================================")
    print("  [LED-TEST] LED LIGHTING SYNC & LIGHTING CHANGE VERIFICATION SUITE")
    print("==============================================================================")
    base_bin = REPO_ROOT / "exampleProfiles" / "DailyUse.bin"
    buf = bytearray(base_bin.read_bytes())
    passed = 0
    failed = 0

    def check(cond: bool, msg: str):
        nonlocal passed, failed
        if cond:
            passed += 1
            print(f"  [PASS] {msg}")
        else:
            failed += 1
            print(f"  [FAIL] {msg}")

    print("\n[STEP 1] Offline Register, Fuse & 5-Sector Checksum Verification across all LED states:")
    for mode_idx, (mode_name, exp_a5, exp_sub) in LED_MODES.items():
        set_led_mode(buf, mode_idx)
        chks_ok = all(st == ex for _, st, ex in verify_all_checksums(buf))
        fuses_ok = (buf[0x05] == 0x0F and buf[0x06] == 0x7F and buf[0x07] == 0x7F and buf[0x08] == 0xAF)
        sub_ok = True if exp_sub is None else (buf[0x03F8] == exp_sub and buf[0x03F9] == 0x04)
        check(
            get_led_mode(buf) == mode_idx and chks_ok and fuses_ok and sub_ok,
            f"Mode {mode_idx} ({mode_name:20s}) -> 0x00A5=0x{buf[0xA5]:02X}, 0x03F8=0x{buf[0x3F8]:02X}, Chk1=0x{buf[0xBF]:02X}, ChkF=0x{buf[0x3FF]:02X}",
        )

    set_led_mode(buf, 0)
    for spd in range(1, 9):
        set_led_speed(buf, spd)
        chks_ok = all(st == ex for _, st, ex in verify_all_checksums(buf))
        fuses_ok = (buf[0x07] == 0x7F and buf[0x08] == 0xAF)
        check(
            get_led_speed(buf) == spd and buf[0x38] == LED_SPEED_TABLE[spd - 1] and chks_ok and fuses_ok,
            f"Speed Step {spd} -> 0x0038=0x{buf[0x38]:02X}, 0x03FA=0x{buf[0x3FA]:02X}, 0x0007(Fuse)=0x{buf[0x07]:02X}, Chk0=0x{buf[0xA3]:02X}",
        )

    for dim, label, exp_a7 in [(False, "Full 100%", 0x84), (True, "Half 50%", 0x44)]:
        set_led_brightness(buf, dim)
        chks_ok = all(st == ex for _, st, ex in verify_all_checksums(buf))
        check(
            get_led_brightness(buf) == (1 if dim else 0) and buf[0xA7] == exp_a7 and buf[0x08] == 0xAF and chks_ok,
            f"Brightness {label:10s} -> 0x00A7=0x{buf[0xA7]:02X}, 0x0008(Fuse)=0x{buf[0x08]:02X}, Chk1=0x{buf[0xBF]:02X}",
        )

    test_rgbs = [(255, 0, 64), (0, 255, 128), (32, 96, 255), (255, 208, 0), (160, 0, 255), (0, 240, 240)]
    for s_idx, (r, g, b) in enumerate(test_rgbs):
        set_stage_rgb(buf, s_idx, r, g, b)
        chks_ok = all(st == ex for _, st, ex in verify_all_checksums(buf))
        check(
            get_stage_rgb(buf, s_idx) == (r, g, b) and chks_ok,
            f"Stage {s_idx + 1} RGB (0x{0x39 + s_idx * 3:04X}) -> #{r:02X}{g:02X}{b:02X}, Chk0=0x{buf[0xA3]:02X}",
        )

    # Step 2: Live Hardware LED Lighting Sync & Change Sequence (if usbipd device is attached)
    devnode, err = discover_vendor_hidraw()
    if devnode and not err:
        print(f"\n[STEP 2] LIVE USB/IP HARDWARE LED LIGHTING SYNC TEST ON {devnode}")
        dev = HidrawVendorDevice(devnode)
        try:
            orig_hw = dev.read_profile_2048()
            backup_path = LOG_DIR / "pre_led_test_backup.em624"
            backup_path.write_bytes(orig_hw)
            print(f"  [BACKUP] Backed up live mouse profile (2048B) to: {backup_path}")

            work = bytearray(orig_hw)
            if work[0:4] != bytes([0x1B, 0xCF, 0x08, 0xB8]):
                work = bytearray(base_bin.read_bytes())

            live_sequence = [
                ("Mode 0: DPI Breathing (Red Stage 1, Speed 8 Fast, 100% Bright)", 0, 8, False, (255, 0, 0)),
                ("Mode 1: Multicolor Breathing (Speed 5, 100% Bright)", 1, 5, False, (0, 255, 0)),
                ("Mode 2: Rainbow / Steady (50% Half Brightness)", 2, 5, True, (0, 128, 255)),
                ("Mode 2: Rainbow / Steady (100% Full Brightness)", 2, 5, False, (255, 0, 255)),
                ("Mode 3: Floe Light / Flow (Speed 7)", 3, 7, False, (255, 255, 0)),
                ("Mode 4: Waltz (Speed 6)", 4, 6, False, (0, 255, 255)),
                ("Mode 5: Four Seasons (Speed 4)", 5, 4, False, (255, 128, 0)),
                ("Mode 6: Follow / Reactive", 6, 5, False, (255, 255, 255)),
                ("Mode 7: LED OFF", 7, 5, False, (255, 0, 0)),
            ]

            for step_label, m_idx, spd, dim, (r, g, b) in live_sequence:
                print(f"\n  >>> [LIVE LED CHANGE] {step_label}")
                set_led_mode(work, m_idx)
                set_led_speed(work, spd)
                set_led_brightness(work, dim)
                active_stage = max(0, min(5, work[0x00A6] & 0x0F))
                set_stage_rgb(work, active_stage, r, g, b)
                dev.sync_led_live(work)
                print(
                    f"      Synced: 0x00A5=0x{work[0xA5]:02X}, 0x0038=0x{work[0x38]:02X}, "
                    f"0x00A7=0x{work[0xA7]:02X}, 0x03F8=0x{work[0x3F8]:02X}, 0x03FA=0x{work[0x3FA]:02X}"
                )
                time.sleep(LIVE_DELAY)

            print("\n  <<< [RESTORE] Restoring original LED registers from backup...")
            dev.sync_led_live(orig_hw)
            print("  [PASS] Live hardware LED cycle completed and original LED state restored.")
        finally:
            dev.close()
    else:
        print(f"\n[STEP 2] Live USB/IP hardware LED cycle skipped ({err})")
        print("         Attach the mouse via usbipd and re-run `./scripts/em624_tool.sh led-test` (or `sudo ./scripts/em624_tool.sh led-test`) for live physical LED cycling.")

    print(f"\n[LED-TEST SUMMARY] Passed: {passed} | Failed: {failed}")
    return failed == 0


def do_auto_test() -> bool:
    print("\n==============================================================================")
    print("  [AUTO-TEST] AUTOMATED REGRESSION & HARDWARE INVARIANT SUITE")
    print("==============================================================================")
    passed = 0
    failed = 0

    for bf in sorted((REPO_ROOT / "exampleProfiles").glob("*.bin")):
        raw = bf.read_bytes()
        chks_ok = all(st == ex for _, st, ex in verify_all_checksums(raw))
        inv_ok = (
            len(raw) == PROFILE_SIZE
            and raw[0:4] == bytes([0x1B, 0xCF, 0x08, 0xB8])
            and raw[0x05] == 0x0F
            and raw[0x06] == 0x7F
            and raw[0x07] == 0x7F
            and raw[0x08] == 0xAF
            and chks_ok
        )
        if inv_ok:
            passed += 1
            print(f"  [PASS] Read-only profile invariant & 5-sector checksums: {bf.name}")
        else:
            failed += 1
            print(f"  [FAIL] Read-only profile invariant & 5-sector checksums: {bf.name}")

    print("\n[CARGO TEST] Running Rust unit tests...")
    res = subprocess.run(["cargo", "test"], cwd=REPO_ROOT, capture_output=True, text=True)
    if VERBOSE:
        for line in res.stdout.splitlines():
            if line.strip():
                print(f"    {line}")
    if res.returncode == 0:
        passed += 1
        print("  [PASS] `cargo test` suite succeeded")
    else:
        failed += 1
        print("  [FAIL] `cargo test` suite failed")
        print(res.stderr)

    print(f"\n[AUTO-TEST SUMMARY] Passed: {passed} | Failed: {failed}")
    return failed == 0


def do_debug() -> bool:
    print("\n==============================================================================")
    print("  [DEBUG] DEEP REGISTER DECODER & PROFILE DIFF")
    print("==============================================================================")
    p1 = pathlib.Path(ARG_P1) if ARG_P1 else (REPO_ROOT / "exampleProfiles" / "DailyUse.bin")
    p2 = pathlib.Path(ARG_P2) if ARG_P2 else (REPO_ROOT / "exampleProfiles" / "Valorant+CS2.bin")
    if not p1.exists():
        print(f"  [ERROR] Profile not found: {p1}")
        return False

    raw = p1.read_bytes()
    print(f"  Target Profile                  : {p1} ({len(raw)}B, sha256={hashlib.sha256(raw).hexdigest()[:16]})")
    print(f"  Magic Signature (0x0000..0x0003): {raw[0:4].hex(' ')}")
    print(f"  Polling / Mask  (0x0004)        : 0x{raw[0x04]:02X} ({ {0:125,1:250,2:500,3:1000}[(raw[0x04]>>4)&3] } Hz)")
    print(f"  Sensor & Fuses  (0x0005..0x0008): {raw[0x05:0x09].hex(' ')} (0x0006=0x{raw[0x06]:02X} -> 180 deg PCB lock)")
    print(f"  DPI Nibbles     (0x002C..0x002E): {raw[0x2C:0x2F].hex(' ')}")
    print(f"  LED Speed       (0x0038)        : 0x{raw[0x38]:02X} (Step {get_led_speed(raw)})")
    for s in range(6):
        r, g, b = get_stage_rgb(raw, s)
        print(f"  Stage {s + 1} RGB     (0x{0x39 + s * 3:04X}..0x{0x3B + s * 3:04X}): #{r:02X}{g:02X}{b:02X}")
    print(f"  Active Mode     (0x00A4)        : 0x{raw[0xA4]:02X} ({'Gaming' if raw[0xA4] else 'Office'})")
    print(f"  LED Effect Mode (0x00A5)        : 0x{raw[0xA5]:02X} (Mode {get_led_mode(raw)}: {LED_MODES[get_led_mode(raw)][0]})")
    print(f"  Active Stage    (0x00A6)        : 0x{raw[0xA6]:02X}")
    print(f"  LED Brightness  (0x00A7)        : 0x{raw[0xA7]:02X} ({'50% Half' if get_led_brightness(raw) else '100% Full'})")
    print(f"  Hardware Footer (0x03F8..0x03FF): {raw[0x3F8:0x400].hex(' ')}")

    if p2.exists():
        raw2 = p2.read_bytes()
        diffs = [i for i in range(min(len(raw), len(raw2))) if raw[i] != raw2[i]]
        print(f"\n  [DIFF] {p1.name} vs {p2.name}: {len(diffs)} differing bytes")
        for off in diffs[:40]:
            vprint(f"Offset 0x{off:04X}: {p1.name}=0x{raw[off]:02X} vs {p2.name}=0x{raw2[off]:02X}")
    return True


ok = True
if SUBCMD in ("all", "probe"):
    ok = do_probe() and ok
if SUBCMD in ("all", "led-test"):
    ok = do_led_test() and ok
if SUBCMD in ("all", "auto-test"):
    ok = do_auto_test() and ok
if SUBCMD in ("all", "debug"):
    ok = do_debug() and ok

sys.exit(0 if ok else 1)
PYEOF

EXIT_CODE=$?
echo ""
echo "=============================================================================="
echo "  [COMPLETE] Exit Code: $EXIT_CODE"
echo "  Full Verbose Log Saved To : $LOG_FILE"
echo "  Latest Log Symlink/Mirror : $LATEST_LOG"
echo "=============================================================================="
exit "$EXIT_CODE"
