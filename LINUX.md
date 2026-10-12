# Linux Setup Guide — E-DRA EM624 Mouse Configurator

The `em624` driver and GUI are compiled as a single native, standalone binary using Rust, `hidapi`, and `egui`. It contains zero webviews, zero HTML, and zero network calls.

---

## 1. System Dependencies

To run the graphical desktop interface on Linux, the windowing and graphics libraries are required.

### Ubuntu / Debian / Pop!_OS / Linux Mint
```bash
sudo apt update
sudo apt install -y libudev1 libgl1 libx11-6 libxcursor1 libxrandr2 libxi6 libxkbcommon0
```

### Fedora / RHEL / AlmaLinux
```bash
sudo dnf install -y systemd-libs mesa-libGL libX11 libXcursor libXrandr libXi libxkbcommon
```

### Arch Linux / Manjaro
```bash
sudo pacman -S --needed systemd-libs mesa libx11 libxcursor libxrandr libxi libxkbcommon
```

---

## 2. USB Permissions (udev rule)

By default, Linux limits raw USB HID access on vendor-defined interfaces to the `root` user. To allow your user account to configure the mouse without `sudo`:

1. Create a udev rule file:
   ```bash
   sudo nano /etc/udev/rules.d/99-em624.rules
   ```

2. Add this line:
   ```udev
   SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1bcf", ATTRS{idProduct}=="08b8", MODE="0666", TAG+="uaccess"
   ```

3. Reload udev rules and reconnect your mouse:
   ```bash
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   ```

---

## 3. Running the Configurator

### Launch Native Graphical UI
```bash
./em624
# or explicitly:
./em624 gui
```

### Command Line Usage (Headless / Terminal)
```bash
# List HID endpoints:
./em624 list

# Probe live mouse hardware memory:
./em624 probe

# Apply a profile:
./em624 apply-profile exampleProfiles/WarThunder.em624

# Convert a stock vendor profile:
./em624 convert-profile exampleProfiles/WarThunder.bin -o custom.em624
```
