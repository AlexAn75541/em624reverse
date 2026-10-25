# Installation & Linux Udev Setup

The E-DRA EM624 driver is packaged as a standalone native executable with zero external runtime dependencies.

---

<!-- tabs:start -->

#### **Windows**

### Option A: Standalone Pre-built Binary
1. Download the latest release package `em624-reversed-windows-x64.zip` from [GitHub Releases](https://github.com/AlexAn75541/em624reverse/releases).
2. Extract the archive into any folder of your choice (e.g. `C:\Tools\EM624`).
3. Run `em624-reversed.exe`. The application automatically monitors USB connections and detects your mouse on startup.

### Option B: Build from Source with Cargo
Ensure you have the Rust toolchain installed:
```powershell
# Clone the repository
git clone https://github.com/AlexAn75541/em624reverse.git
cd em624reverse

# Build the optimized release binary
cargo build --release

# Run the configurator GUI
.\target\release\em624-reversed.exe
```

---

#### **Linux (Ubuntu / Debian / Arch / Fedora)**

### 1. Prerequisites
Install the required development headers for USB HID and OpenGL:

```bash
# Ubuntu / Debian / Pop!_OS
sudo apt update
sudo apt install -y pkg-config libudev-dev libusb-1.0-0-dev libxkbcommon-dev

# Arch Linux / Manjaro
sudo pacman -S pkgconf systemd libusb libxkbcommon

# Fedora
sudo dnf install -y pkgconf-pkg-config systemd-devel libusb1-devel libxkbcommon-devel
```

### 2. Configure Non-Root USB Permissions (Udev Rules)
By default, the Linux kernel restricts raw USB HID feature reports on `/dev/hidraw*` to `root`. To configure your mouse without typing `sudo`, install the project's udev rule:

```bash
# Run the included installer script
sudo ./scripts/install_udev.sh
```

Or manually create `/etc/udev/rules.d/99-em624-mouse.rules`:
```udev
# E-DRA EM624 Gaming Mouse (Sunplus VID 0x1BCF, PID 0x08B8)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1bcf", ATTRS{idProduct}=="08b8", MODE="0666", TAG+="uaccess"
SUBSYSTEM=="usb", ATTRS{idVendor}=="1bcf", ATTRS{idProduct}=="08b8", MODE="0666", TAG+="uaccess"
```

Reload udev rules and replug your mouse:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 3. Build & Run
```bash
cargo build --release
./target/release/em624-reversed
```

<!-- tabs:end -->

---

## Verifying Hardware Access

To verify that your operating system detects the EM624 vendor configuration interface, run the CLI `list` subcommand:

```bash
cargo run -- list
```

**Expected output:**
```
Found candidate E-DRA EM624 mouse:
  Path: \\?\hid#vid_1bcf&pid_08b8&mi_01&col04...
  Manufacturer: SunplusIT Inc
  Product:      Gaming Mouse
  Usage Page:   0xff00
  Usage:        0x0001
  [CONFIRMED] Ready for configuration transfers.
```
