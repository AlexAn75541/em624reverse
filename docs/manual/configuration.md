# Configuration Guide: DPI, RGB & Button Rebinding

The GUI configurator provides an intuitive interface for tuning all hardware capabilities of the EM624 mouse.

---

## DPI & Optical Performance Tuning

The EM624 supports up to 6 discrete DPI stages. In the **Game Mode** tab, you can configure each stage individually:

1. **Active Stages Selection**: Click any of the numbered stage badges (1 to 5) to select it as the active editing target.
2. **DPI Adjustment Slider**: Move the slider across the 14 calibrated hardware levels:
   `[200, 400, 600, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4800, 6400, 8000, 10000]`
3. **Stage Color Customization**:
   - Each DPI stage has a dedicated RGB LED indicator color.
   - Adjust the RGB sliders (`R: 0..255`, `G: 0..255`, `B: 0..255`) or pick from the preset swatches (Red, Green, Blue, Yellow, Cyan, Purple, White).
   - When switching DPI using Button 6 on the mouse, the chassis illumination flashes the corresponding stage color.
4. **USB Polling Rate**: Select between **125 Hz**, **250 Hz**, **500 Hz**, or **1000 Hz** (1ms latency).

---

## LED Illumination Modes

The mouse chassis features 7 built-in hardware lighting algorithms controlled by the internal MCU:

| Mode | Visual Effect | Speed Control |
|---|---|---|
| **Breathing** | Smoothly pulses the active DPI stage color | 1..5 seconds cycle |
| **Colorful (Default)** | Dynamic multi-color RGB streaming wave | 1..5 seconds speed |
| **Rainbow Breathing** | Pulses through the entire RGB color spectrum | 1..5 seconds speed |
| **Flowing Wave** | Chasing light wave effect across chassis edge | 1..5 seconds speed |
| **Waltz** | Alternating dual-color rhythmic strobe | 1..5 seconds speed |
| **4-Seasons** | Slow seasonal temperature transition | 1..5 seconds speed |
| **Off** | Turns off all chassis LEDs completely | N/A |

---

## Button Rebinding Engine

To customize any of the 7 physical buttons:

1. Click on the button in the diagram or select it from the button list.
2. The **Rebind Modal Dialog** opens, displaying 4 distinct category tabs:
   - **Standard Mouse Actions**: Left Click, Right Click, Middle Click, Forward (Side 1), Backward (Side 2), DPI Loop.
   - **Keyboard Keys & Hotkeys**:
     - Single key remapping across all standard US QWERTY keys (A-Z, 0-9, F1-F12, Space, Enter, Escape, Arrow keys, etc.).
     - Modifier combinations: check **Ctrl**, **Shift**, **Alt**, or **Windows** to create instant chord shortcuts (e.g. `Ctrl + Shift + T`, `Alt + F4`).
   - **Consumer & Multimedia**:
     - Media controls: Play/Pause, Next Track, Previous Track, Stop, Volume Up, Volume Down, Mute.
     - Browser shortcuts: Home, Refresh, Back, Forward.
     - System tools: Calculator, Mail, My Computer.
   - **Rapid Fire / Burst Key**: Configures Button 7 to emit rapid successive left clicks with custom millisecond intervals.
3. Click **Apply Rebind**. The change is instantly reflected in the profile view and synchronized to both internal tables (Office & Gaming).
4. Click **Apply to Mouse** in the bottom bar to write your changes to the mouse EEPROM.
