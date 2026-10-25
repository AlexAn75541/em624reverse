# Frequently Asked Questions (FAQ)

### Q: Does this driver support Linux?
**Yes.** The reverse-engineered driver and GUI are 100% written in portable Rust using `eframe`/`glow` and `hidapi`. It runs natively on Linux (X11 and Wayland) without Wine or emulation. Simply install the non-root udev rules (`scripts/install_udev.sh`).

---

### Q: Why does the application use less than 10 MB of RAM?
The software was engineered from day one with strict resource budgets. It uses zero Electron or Webview overhead, renders pure GPU immediate-mode vector graphics via `egui`, and continuously executes OS-level working set compaction (`HeapCompact` + `K32EmptyWorkingSet` on Windows, `malloc_trim(0)` on Linux) on tab switches and user interactions.

---

### Q: Can flashing a custom profile permanently brick my mouse?
**No.** The Sunplus SPCP6651B microcontroller contains immutable factory boot ROM code that handles USB enumeration. Even if you flash invalid button tables or mismatched checksums, the mouse hardware can always be restored either via our CLI (`cargo run -- apply-profile exampleProfiles/DailyUse.em624`) or by clicking "Default" in the stock vendor utility.

---

### Q: Why did the official software freeze when selecting Stage 6?
In the stock firmware, bit 5 of the active stage bitmask (`0x0004`) was inadvertently mapped to an internal hardware test interrupt on the SPCP6651B MCU. Activating this bit causes a microcontroller deadlock that halts physical button click dispatches until USB power is cycled. Our driver automatically masks this bit (`active_stages_mask &= 0xDF`) across all profiles.

---

### Q: How do custom cloud fonts work without an internet connection?
Once a font is verified and applied from Google Fonts or Fontshare, the TrueType font binary is permanently saved locally in your active master data directory (`.fonts/<font_name>.ttf`). Subsequent launches load the cached font directly from local disk with zero network requests.

---

### Q: Can I use this driver for other Sunplus SPCP6651B / SPCP6662 mice?
**Yes.** Many OEM gaming mice share the same Sunplus SPCP6651B MCU and SPCP6662 optical sensor architecture. If your device shares the same USB Feature Report protocol (`0x40`/`0x41`), this software can serve as a foundation for your mouse driver.
