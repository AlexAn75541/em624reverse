# 🤓 `em624-driver` Internals & Architecture

Welcome to the **Nerdy Zone**! This document provides an in-depth architectural breakdown of the `em624-driver` Rust crate, its internal design, zero-compromise memory management, USB HID transmission engine, and hardware protection systems.

---

## 1. Crate Overview & Module Hierarchy

The project is structured as a single high-performance Rust crate with zero unneeded dependencies, compiled with Link-Time Optimization (`lto = true`), single codegen unit (`codegen-units = 1`), and stripped symbols:

```
src/
├── main.rs            # Application entrypoint & CLI subcommand router
├── device.rs          # USB HID interface abstraction, Report 0x40/0x41 transmission
├── profile.rs         # 2048-byte EEPROM profile parser, button table validator, checksums
├── macro_engine.rs    # Onboard macro bytecode compiler, disassembler, and VM validator
├── font_service.rs    # Cloud font verification (Google Fonts & Fontshare) & local disk cache
├── gui.rs             # 100% native egui/eframe desktop interface
├── i18n.rs            # Bilingual dictionary (English & Vietnamese)
└── platform/
    ├── mod.rs         # Cross-platform trimming & system API router
    ├── windows.rs     # Win32 Raw Input packet sniffer & HeapCompact / EmptyWorkingSet
    └── linux.rs       # Linux malloc_trim(0) memory compaction
```

---

## 2. Memory Optimization (< 10MB Working Set)

A core requirement of `em624-driver` is that it **must never exceed 10 MB RAM** in steady-state execution. Unlike Electron or CEF utilities that consume 150MB–300MB of RAM for a simple mouse utility, `em624-driver` runs on native `eframe` (OpenGL/Glow) with aggressive heap compaction.

### Continuous Heap Compaction Strategy

Whenever a significant state transition occurs (tab switch, modal close, profile flash, profile save/load, font apply, or periodic 2-second background tick), the runtime calls:

```rust
crate::platform::trim_working_set();
```

#### Windows Implementation (`src/platform/windows.rs`):
```rust
pub fn trim_working_set() {
    unsafe {
        // 1. Compact active process heap
        let heap = GetProcessHeap();
        if !heap.is_null() {
            HeapCompact(heap, 0);
        }
        // 2. Empty unreferenced physical pages from working set to OS standby
        let process = GetCurrentProcess();
        K32EmptyWorkingSet(process);
        // 3. Advise OS memory manager of minimal working set boundary
        SetProcessWorkingSetSize(process, usize::MAX, usize::MAX);
    }
}
```

#### Linux Implementation (`src/platform/linux.rs`):
```rust
pub fn trim_working_set() {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::malloc_trim(0);
    }
}
```

### Texture Memory Hygiene
The mouse chassis diagram (`assets/damouse.png`) is decoded at high resolution (600x600) to ensure crisp rendering at 125%–200% Windows display scaling. However, CPU-side pixel allocations are immediately dropped after GPU texture upload:

```rust
let pixels = load_image_from_memory(bytes)?;
let texture = ctx.load_texture("mouse_chassis", pixels, TextureOptions::LINEAR);
drop(pixels);
crate::platform::trim_working_set();
```

---

## 3. USB HID Feature Report Engine (`src/device.rs`)

The driver communicates with the Sunplus SPCP6651B MCU exclusively using **USB HID Feature Reports** over **Interface 1, Collection 04** (`Usage Page 0xFF00, Usage 0x01`).

### The 4-Stage Hardware Flash Pipeline

When saving settings to the physical mouse, the driver executes the following sequence:

```
[Profile Data] 
       │
       ▼
 1. Prepare Write ────► Opcode 0x40 0x07 (Offset: 0x0000, Length: 0x0800)
       │
       ▼
 2. Stream Data   ────► 293 chunks of Report 0x41 (7 bytes payload each, 4-6ms pacing)
       │
       ▼
 3. Soft Apply    ────► Opcode 0x40 0x0D (Loads tables into MCU active registers)
       │
       ▼
 4. Commit Flash  ────► Opcode 0x40 0x0E (Burns RAM to EEPROM & resets USB bus)
```

### Code Implementation (`src/device.rs`):

```rust
// 1. Prepare 2048-byte write
let prep = [0x40, 0x07, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00];
device.send_feature_report(&prep)?;

// 2. Stream 293 chunks of 7 bytes
for chunk in profile.raw.chunks(7) {
    let mut report = [0u8; 8];
    report[0] = 0x41;
    report[1..1 + chunk.len()].copy_from_slice(chunk);
    if chunk.len() < 7 {
        report[1 + chunk.len()..].fill(0xFF); // Pad final chunk
    }
    device.send_feature_report(&report)?;
    std::thread::sleep(std::time::Duration::from_millis(5));
}

// 3. Soft Apply (Opcode 0x0D)
let apply = [0x40, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
device.send_feature_report(&apply)?;

// 4. Commit to EEPROM (Opcode 0x0E)
let commit = [0x40, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
match device.send_feature_report(&commit) {
    Ok(_) => (),
    Err(e) if is_bus_reset_error(&e) => (), // Expected code 0x1F on bus reset
    Err(e) => return Err(e),
}
```

---

## 4. Checksum Verification Architecture (`src/profile.rs`)

The Sunplus SPCP6651B MCU validates 5 distinct checksum sectors before activating button event dispatch. If any checksum fails, the mouse enters safe tracking mode where cursor motion works but **physical clicks are completely shut down**.

The formula for all sectors is:

$$\text{Checksum} = \left( \sum_{i} \text{byte}_i \oplus \text{0x5A} \right) \ \& \ \text{0xFF}$$

### Sector Definitions:

```rust
pub fn calculate_checksum(data: &[u8]) -> u8 {
    let sum: u32 = data.iter().map(|&b| b as u32).sum();
    ((sum ^ 0x5A) & 0xFF) as u8
}

pub fn update_all_checksums(&mut self) {
    // Sector 0: Magic, Active DPI Mask, RGB Colors
    self.raw[0x00A3] = calculate_checksum(&self.raw[0x0000..=0x00A2]);
    // Sector 1: Report & Mode Control
    self.raw[0x00BF] = calculate_checksum(&self.raw[0x00A4..=0x00BE]);
    // Sector 2: Office Mode Button Table
    self.raw[0x00F0] = calculate_checksum(&self.raw[0x00C0..=0x00EF]);
    // Sector 3: Gaming Mode Button Table
    self.raw[0x0121] = calculate_checksum(&self.raw[0x00F1..=0x0120]);
    // Sector Footer: Hardware Footer
    self.raw[0x03FF] = calculate_checksum(&self.raw[0x03F8..=0x03FE]);
}
```

---

## 5. Mode Synchronization & Dual Button Tables

The hardware contains dual 48-byte button tables:
- **Office Mode Table**: `0x00C0..0x00EF` (checksum at `0x00F0`)
- **Gaming Mode Table**: `0x00F1..0x0120` (checksum at `0x0121`)
- **Active Mode Byte**: `0x00A4` (`0x00` = Office, `0x01` = Gaming)

When the user rebinds any button in the GUI, `em624-driver` performs **Dual Table Synchronization**:
1. It updates the targeted slot in both the Office and Gaming tables simultaneously.
2. It sets `0x00A4` to `0x01` (Active Gaming Mode).
3. It recalculates checksums for both Sector 2 and Sector 3.

This guarantees that if the mouse hardware internally switches modes (or if an Fn key was mapped), the physical buttons never become unresponsive.

---

## 6. Optical DSP 180° Sensor Safeguard

Byte `0x0006` controls the coordinate transformation inside the Sunplus SPCP6662 optical sensor DSP:
- `0x00`: 270° rotation (Inverted axes!).
- `0x7F` (or `0x40`): **180° rotation** (The exact physical PCB mounting orientation).

`em624-driver` strictly enforces:
```rust
if self.raw[0x0006] != 0x7F {
    self.raw[0x0006] = 0x7F;
}
```
This permanently prevents the inverted tracking bug created by the stock Delphi vendor app.

---

## 7. Windows Raw Input Hardware Sniffer (`src/platform/windows.rs`)

For debugging and reverse engineering hardware button clicks in real time, the driver registers a headless Win32 Raw Input window targeting `Usage Page 0x01` (Generic Desktop) and `Usage 0x02` (Mouse):

```rust
let rid = RAWINPUTDEVICE {
    usUsagePage: 0x01,
    usUsage: 0x02,
    dwFlags: RIDEV_INPUTSINK,
    hwndTarget: hwnd,
};
RegisterRawInputDevices(&rid, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
```

When packets arrive, the window procedure intercepts `WM_INPUT`, extracts `RAWINPUT`, and decodes raw physical button flags (`RI_MOUSE_BUTTON_1_DOWN` through `BUTTON_5_DOWN`, wheel deltas, and relative motion `lLastX`, `lLastY`).

Run `em624-reversed listen` in terminal to run the sniffer!
