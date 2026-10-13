#!/usr/bin/env bash
# ==============================================================================
# E-DRA EM624 Mouse - Linux Udev Rules Installer
# Vendor ID: 0x1BCF (Sunplus Innovation Technology)
# Product ID: 0x08B8 (EM624 Gaming Mouse)
# ==============================================================================

set -euo pipefail

RULES_PATH="/etc/udev/rules.d/99-em624.rules"

echo "=========================================================================="
echo "  E-DRA EM624 Gaming Mouse - Udev Rules Setup"
echo "  Grants non-root user permissions to access USB HID Feature Reports & Raw"
echo "=========================================================================="

if [ "$EUID" -ne 0 ]; then
  echo "Error: This script must be run as root (or via sudo)."
  echo "Usage: sudo bash scripts/install_udev.sh"
  exit 1
fi

echo "Writing udev rules to $RULES_PATH..."

cat << 'EOF' > "$RULES_PATH"
# E-DRA EM624 Gaming Mouse (SunplusIT SPCP6662 + SPCP6651B)
# Interface 0: Mouse Movement & Standard Buttons (hidraw)
KERNEL=="hidraw*", ATTRS{idVendor}=="1bcf", ATTRS{idProduct}=="08b8", MODE="0666", TAG+="uaccess"

# Interface 1 Col04: Vendor Config & EEPROM Feature Reports
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1bcf", ATTRS{idProduct}=="08b8", MODE="0666", TAG+="uaccess"

# USB Generic Subsystem access
SUBSYSTEM=="usb", ATTRS{idVendor}=="1bcf", ATTRS{idProduct}=="08b8", MODE="0666", TAG+="uaccess"
EOF

chmod 644 "$RULES_PATH"

echo "Reloading udev service and re-triggering devices..."
udevadm control --reload-rules || true
udevadm trigger || true

echo "Done! E-DRA EM624 can now be configured without root/sudo."
echo "If your mouse is currently connected, replug it or run:"
echo "  ./target/release/em624"
