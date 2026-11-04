use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const PROFILE_SIZE: usize = 2048;
pub const PROFILE_MAGIC: [u8; 4] = [0x1b, 0xcf, 0x08, 0xb8];

/// Official DPI lookup table for the SunplusIT SPCP6662 (6662IC) sensor
/// extracted from the official application binary DFM (TForm1 gbCPI strings).
pub const OFFICIAL_DPI_STEPS: [u32; 15] = [
    200, 400, 600, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4800, 6400, 8000, 10000, 12000,
];

/// Standard 12-key button mapping (4 bytes per key, 48 bytes total) from factory firmware.
/// Guaranteed functional mouse actions:
/// Key 1: Left Click, Key 2: Forward (Side 1), Key 3: Fire Key,
/// Key 4: Right Click, Key 5: Backward (Side 2), Key 7: Middle Click
pub const STANDARD_BUTTON_TABLE: [u8; 48] = [
    0x01, 0x01, 0x00, 0x00, // Key 1: Left Click
    0x01, 0x08, 0x00, 0x02, // Key 2: Forward (Side Button 1)
    0x06, 0x92, 0x00, 0x1e, // Key 3: Fire Key / Burst
    0x01, 0x02, 0x00, 0x00, // Key 4: Right Click
    0x01, 0x10, 0x00, 0x00, // Key 5: Backward (Side Button 2)
    0x09, 0x01, 0x00, 0x00, // Key 6: Fn / Gaming
    0x01, 0x04, 0x00, 0x00, // Key 7: Middle Click
    0x09, 0x00, 0x00, 0x00, // Key 8: Fn / Office
    0x09, 0x02, 0x00, 0x00, // Key 9: DPI+
    0x00, 0x00, 0x00, 0x00, // Key 10
    0x00, 0x00, 0x00, 0x00, // Key 11
    0x00, 0x00, 0x00, 0x00, // Key 12
];

/// Verified 136-byte button table and mode flags (offsets 0x00A0..0x0128)
/// from DailyUse.bin that ensures working Left/Right/Middle clicks.
#[allow(dead_code)]
pub const VERIFIED_BUTTON_BLOCK: [u8; 136] = [
    0x00, 0x00, 0x00, 0x42, 0x01, 0x87, 0x01, 0x00, 0x0f, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xea,
    0x01, 0x01, 0x00, 0x00, 0x02, 0x00, 0x4e, 0x00, 0x06, 0x12, 0x00, 0x1e, 0x01, 0x02, 0x00, 0x00,
    0x02, 0x00, 0x4b, 0x00, 0x07, 0x81, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x2c, 0x01, 0x01, 0x00, 0x00, 0x02, 0x0a, 0x16, 0x00, 0x06, 0x12, 0x00, 0x1e, 0x01, 0x02, 0x00,
    0x00, 0x02, 0x08, 0x07, 0x00, 0x07, 0x80, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x09, 0x00, 0x00,
    0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x51, 0x00, 0x00, 0x02, 0x00, 0x1e, 0x04,
];

/// Verified 8-byte sector footer (offsets 0x03F8..0x0400)
#[allow(dead_code)]
pub const VERIFIED_FOOTER: [u8; 8] = [0x07, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x51];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpiStage {
    pub level_index: u8,
    pub approx_dpi: u32,
    pub color: RgbColor,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonBinding {
    pub key_number: u8,
    pub physical_button: String,
    pub key_type: u8,
    pub param1: u8,
    pub param2: u8,
    pub param3: u8,
    pub description: String,
}

/// Translate USB HID Keyboard Usage (Usage Page 0x07) code to human-readable string
pub fn format_hid_key_name(code: u8) -> &'static str {
    match code {
        0x04 => "A",
        0x05 => "B",
        0x06 => "C",
        0x07 => "D",
        0x08 => "E",
        0x09 => "F",
        0x0A => "G",
        0x0B => "H",
        0x0C => "I",
        0x0D => "J",
        0x0E => "K",
        0x0F => "L",
        0x10 => "M",
        0x11 => "N",
        0x12 => "O",
        0x13 => "P",
        0x14 => "Q",
        0x15 => "R",
        0x16 => "S",
        0x17 => "T",
        0x18 => "U",
        0x19 => "V",
        0x1A => "W",
        0x1B => "X",
        0x1C => "Y",
        0x1D => "Z",
        0x1E => "1",
        0x1F => "2",
        0x20 => "3",
        0x21 => "4",
        0x22 => "5",
        0x23 => "6",
        0x24 => "7",
        0x25 => "8",
        0x26 => "9",
        0x27 => "0",
        0x28 => "Enter",
        0x29 => "Esc",
        0x2A => "Backspace",
        0x2B => "Tab",
        0x2C => "Space",
        0x2D => "- (Minus)",
        0x2E => "= (Equals)",
        0x2F => "[ (Left Bracket)",
        0x30 => "] (Right Bracket)",
        0x31 => "\\ (Backslash)",
        0x32 => "# (Non-US)",
        0x33 => "; (Semicolon)",
        0x34 => "' (Quote)",
        0x35 => "` (Grave Accent)",
        0x36 => ", (Comma)",
        0x37 => ". (Period)",
        0x38 => "/ (Slash)",
        0x39 => "Caps Lock",
        0x3A => "F1",
        0x3B => "F2",
        0x3C => "F3",
        0x3D => "F4",
        0x3E => "F5",
        0x3F => "F6",
        0x40 => "F7",
        0x41 => "F8",
        0x42 => "F9",
        0x43 => "F10",
        0x44 => "F11",
        0x45 => "F12",
        0x46 => "Print Screen",
        0x47 => "Scroll Lock",
        0x48 => "Pause / Break",
        0x49 => "Insert",
        0x4A => "Home",
        0x4B => "Page Up",
        0x4C => "Delete",
        0x4D => "End",
        0x4E => "Page Down",
        0x4F => "Right Arrow",
        0x50 => "Left Arrow",
        0x51 => "Down Arrow",
        0x52 => "Up Arrow",
        0x53 => "Num Lock",
        0x54 => "Numpad /",
        0x55 => "Numpad *",
        0x56 => "Numpad -",
        0x57 => "Numpad +",
        0x58 => "Numpad Enter",
        0x59 => "Numpad 1",
        0x5A => "Numpad 2",
        0x5B => "Numpad 3",
        0x5C => "Numpad 4",
        0x5D => "Numpad 5",
        0x5E => "Numpad 6",
        0x5F => "Numpad 7",
        0x60 => "Numpad 8",
        0x61 => "Numpad 9",
        0x62 => "Numpad 0",
        0x63 => "Numpad .",
        0x64 => "\\ (Non-US)",
        0x65 => "App / Menu",
        0x68 => "F13",
        0x69 => "F14",
        0x6A => "F15",
        0x6B => "F16",
        0x6C => "F17",
        0x6D => "F18",
        0x6E => "F19",
        0x6F => "F20",
        0x70 => "F21",
        0x71 => "F22",
        0x72 => "F23",
        0x73 => "F24",
        0xE0 => "Left Ctrl",
        0xE1 => "Left Shift",
        0xE2 => "Left Alt",
        0xE3 => "Left Win",
        0xE4 => "Right Ctrl",
        0xE5 => "Right Shift",
        0xE6 => "Right Alt",
        0xE7 => "Right Win",
        _ => "Unknown Key",
    }
}

/// Format modifier byte (bits: 0x01=Ctrl, 0x02=Shift, 0x04=Alt, 0x08=Win)
pub fn format_modifiers(mod_byte: u8) -> String {
    let mut parts = Vec::new();
    if (mod_byte & 0x01) != 0 {
        parts.push("Ctrl");
    }
    if (mod_byte & 0x02) != 0 {
        parts.push("Shift");
    }
    if (mod_byte & 0x04) != 0 {
        parts.push("Alt");
    }
    if (mod_byte & 0x08) != 0 {
        parts.push("Win");
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{}+", parts.join("+"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseProfile {
    pub raw: Vec<u8>,
    pub device_name: String,
    pub active_stages_mask: u8,
    pub dpi_stages: Vec<DpiStage>,
    pub is_gaming_mode: bool,
    pub office_buttons: Vec<ButtonBinding>,
    pub gaming_buttons: Vec<ButtonBinding>,
}

impl MouseProfile {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        Self::from_bytes(bytes)
    }

    /// Refresh parsed DPI stages, RGB colors, and button mapping tables from raw bytes
    pub fn refresh_stages(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        // Verified from live hardware probe (`0x40 0x0D` -> `41 82 5a 01` -> `"I2C"` branch)
        // and `DPILevel_OnClick` (`0x005861d4`):
        // Upper 3 bits (5..3) of `0x00A6` store `(num_active_stages - 1)` when < 6, or `0` when all 6 stages are active.
        let upper = ((self.raw[0x00a6] >> 3) & 0x07) as usize;
        let num_active = if upper == 0 {
            6
        } else {
            (upper + 1).clamp(1, 6)
        };
        self.active_stages_mask = ((1u16 << num_active) - 1) as u8;
        self.is_gaming_mode = (self.raw[0x00a4] & 0x0f) != 0;

        let levels = [
            self.raw[44] & 0x0f,
            (self.raw[44] >> 4) & 0x0f,
            self.raw[45] & 0x0f,
            (self.raw[45] >> 4) & 0x0f,
            self.raw[46] & 0x0f,
            (self.raw[46] >> 4) & 0x0f,
        ];
        self.dpi_stages.clear();
        for (i, &lvl) in levels.iter().enumerate() {
            // Verified from `edt_Col1RChange` (0x00575b44) & `lbl_DPI1ColClick` ("I2C" branch at 0x00587fa4):
            // Stage 1..6 RGB colors are 3-byte [R, G, B] triples starting at 0x0039 (57).
            let color_offset = 0x0039 + (i * 3);
            let color = RgbColor {
                r: self.raw[color_offset],
                g: self.raw[color_offset + 1],
                b: self.raw[color_offset + 2],
            };
            let enabled = i < num_active;
            let approx_dpi = if (lvl as usize) < OFFICIAL_DPI_STEPS.len() {
                OFFICIAL_DPI_STEPS[lvl as usize]
            } else {
                8000
            };
            self.dpi_stages.push(DpiStage {
                level_index: lvl,
                approx_dpi,
                color,
                enabled,
            });
        }

        self.office_buttons.clear();
        for k in 0..12 {
            let start = 0x00c0 + k * 4;
            self.office_buttons.push(Self::decode_button(
                k as u8 + 1,
                &self.raw[start..start + 4],
            ));
        }

        self.gaming_buttons.clear();
        for k in 0..12 {
            let start = 0x00f1 + k * 4;
            self.gaming_buttons.push(Self::decode_button(
                k as u8 + 1,
                &self.raw[start..start + 4],
            ));
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PROFILE_SIZE {
            bail!(
                "Invalid profile length: {} bytes (expected {})",
                bytes.len(),
                PROFILE_SIZE
            );
        }
        if bytes[0..4] != PROFILE_MAGIC {
            bail!(
                "Invalid profile magic: {:02x?} (expected {:02x?})",
                &bytes[0..4],
                PROFILE_MAGIC
            );
        }

        let raw = bytes.to_vec();
        let upper = ((raw[0x00a6] >> 3) & 0x07) as usize;
        let num_active = if upper == 0 {
            6
        } else {
            (upper + 1).clamp(1, 6)
        };
        let active_stages_mask = ((1u16 << num_active) - 1) as u8;

        // Device name string around offset 111 (0x6F)
        let name_bytes = &raw[111..130];
        let null_pos = name_bytes
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(name_bytes.len());
        let device_name = String::from_utf8_lossy(&name_bytes[..null_pos])
            .trim()
            .to_string();

        // Parse 6 DPI stages from nibbles at offsets 44, 45, 46
        let n0 = raw[44] & 0x0f;
        let n1 = (raw[44] >> 4) & 0x0f;
        let n2 = raw[45] & 0x0f;
        let n3 = (raw[45] >> 4) & 0x0f;
        let n4 = raw[46] & 0x0f;
        let n5 = (raw[46] >> 4) & 0x0f;
        let levels = [n0, n1, n2, n3, n4, n5];

        let mut dpi_stages = Vec::new();
        for (i, &lvl) in levels.iter().enumerate() {
            // Read 3-byte RGB starting at offset 0x0039 + (i * 3)
            let color_offset = 0x0039 + (i * 3);
            let color = if color_offset + 3 <= PROFILE_SIZE {
                RgbColor {
                    r: raw[color_offset],
                    g: raw[color_offset + 1],
                    b: raw[color_offset + 2],
                }
            } else {
                RgbColor {
                    r: 255,
                    g: 255,
                    b: 255,
                }
            };

            let enabled = i < num_active;
            let approx_dpi = if (lvl as usize) < OFFICIAL_DPI_STEPS.len() {
                OFFICIAL_DPI_STEPS[lvl as usize]
            } else {
                8000
            };

            dpi_stages.push(DpiStage {
                level_index: lvl,
                approx_dpi,
                color,
                enabled,
            });
        }

        let is_gaming_mode = (raw[0x00a4] & 0x0f) != 0;

        let mut office_buttons = Vec::new();
        for k in 0..12 {
            let start = 0x00c0 + k * 4;
            office_buttons.push(Self::decode_button(k as u8 + 1, &raw[start..start + 4]));
        }

        let mut gaming_buttons = Vec::new();
        for k in 0..12 {
            let start = 0x00f1 + k * 4;
            gaming_buttons.push(Self::decode_button(k as u8 + 1, &raw[start..start + 4]));
        }

        Ok(Self {
            raw,
            device_name,
            active_stages_mask,
            dpi_stages,
            is_gaming_mode,
            office_buttons,
            gaming_buttons,
        })
    }

    fn decode_button(key_num: u8, slice: &[u8]) -> ButtonBinding {
        let key_type = slice[0];
        let p1 = slice[1];
        let p2 = slice[2];
        let p3 = slice[3];

        let physical_button = match key_num {
            1 => "Button 1: Left Key",
            2 => "Button 4: Forward (Side 1)",
            3 => "Aux Slot 3: Virtual",
            4 => "Button 2: Right Key",
            5 => "Button 5: Backward (Side 2)",
            6 => "Aux Slot 6: Virtual",
            7 => "Button 3: Middle Key (Wheel)",
            8 => "Button 7: DPI -",
            9 => "Button 6: DPI +",
            _ => "Unused Slot",
        }
        .to_string();
        let desc = match key_type {
            0x01 => match p1 {
                0x01 => "Left Click".to_string(),
                0x02 => "Right Click".to_string(),
                0x04 => "Middle Click".to_string(),
                0x08 => "Forward (Side 1)".to_string(),
                0x10 => "Backward (Side 2)".to_string(),
                other => format!("Mouse Button 0x{:02x}", other),
            },
            0x02 => {
                let mod_str = format_modifiers(p1);
                let key_name = format_hid_key_name(p2);
                if key_name == "Unknown Key" {
                    format!("Key: {}0x{:02x}", mod_str, p2)
                } else {
                    format!("{}{}", mod_str, key_name)
                }
            }
            0x03 => match (p1, p2) {
                (0xb5, 0x00) | (0x00, 0xb5) => "Media: Next Track".to_string(),
                (0xb6, 0x00) | (0x00, 0xb6) => "Media: Prev Track".to_string(),
                (0xb7, 0x00) | (0x00, 0xb7) => "Media: Stop".to_string(),
                (0xcd, 0x00) | (0x00, 0xcd) => "Media: Play/Pause".to_string(),
                (0xe2, 0x00) | (0x00, 0xe2) => "Media: Mute".to_string(),
                (0xe9, 0x00) | (0x00, 0xe9) => "Media: Vol+".to_string(),
                (0xea, 0x00) | (0x00, 0xea) => "Media: Vol-".to_string(),
                (0x83, 0x01) => "Media: Player".to_string(),
                (0x23, 0x02) | (0x00, 0x81) => "Browser: Home".to_string(),
                (0x27, 0x02) | (0x00, 0x82) => "Browser: Refresh".to_string(),
                (0x24, 0x02) | (0x00, 0x83) => "Browser: Back".to_string(),
                (0x25, 0x02) | (0x00, 0x84) => "Browser: Forward".to_string(),
                (0x26, 0x02) => "Browser: Stop".to_string(),
                (0x21, 0x02) => "Browser: Search".to_string(),
                (0x2a, 0x02) => "Browser: Favorites".to_string(),
                (0x8a, 0x01) | (0x00, 0x8a) => "App: Mail".to_string(),
                (0x92, 0x01) | (0x00, 0x92) => "App: Calculator".to_string(),
                (0x94, 0x01) | (0x00, 0x94) => "App: My Computer".to_string(),
                _ => format!("Consumer 0x{:02x}{:02x}", p2, p1),
            },
            0x05 => format!("Macro #{}", p2.saturating_add(1)),
            0x06 => {
                if p1 == 0x01 && p2 == 0x02 {
                    "Double Click".to_string()
                } else {
                    "Fire Key".to_string()
                }
            }
            0x07 => "ACPI / Power".to_string(),
            0x08 => "DPI Loop".to_string(),
            0x09 => match p1 {
                0x00 => "Office Mode / DPI -".to_string(),
                0x01 => "Gaming Mode".to_string(),
                0x02 => "DPI +".to_string(),
                0x03 => "DPI -".to_string(),
                other => format!("Fn 0x{:02x}", other),
            },
            0x0a => "LED Mode Switch".to_string(),
            0x00 => "None".to_string(),
            other => format!("Type 0x{:02x}", other),
        };

        ButtonBinding {
            key_number: key_num,
            physical_button,
            key_type,
            param1: p1,
            param2: p2,
            param3: p3,
            description: desc,
        }
    }

    /// Calculate Delphi/Sunplus firmware checksum: (sum(bytes) ^ 0x5a) & 0xff
    pub fn calculate_checksum(slice: &[u8]) -> u8 {
        let sum: u32 = slice.iter().map(|&b| b as u32).sum();
        ((sum ^ 0x5a) & 0xff) as u8
    }

    /// Recalculate and update all 4 sector checksums across profile memory
    pub fn recalculate_checksums(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        // Sector 0 (0x0000..0x00A2, 163 bytes): Magic, Active Stages, DPI nibbles, RGB LED colors
        self.raw[0x00a3] = Self::calculate_checksum(&self.raw[0x0000..0x00a3]);
        // Sector 1 (0x00A4..0x00BE, 27 bytes): Mode flags and control registers
        self.raw[0x00bf] = Self::calculate_checksum(&self.raw[0x00a4..0x00bf]);
        // Sector 2 (0x00C0..0x00EF, 48 bytes): Office Mode 12-button table
        self.raw[0x00f0] = Self::calculate_checksum(&self.raw[0x00c0..0x00f0]);
        // Sector 3 (0x00F1..0x0120, 48 bytes): Gaming Mode 12-button table
        self.raw[0x0121] = Self::calculate_checksum(&self.raw[0x00f1..0x0121]);
        // Sector Footer (0x03F8..0x03FF, 7 bytes): MCU hardware footer
        self.raw[0x03ff] = Self::calculate_checksum(&self.raw[0x03f8..0x03ff]);
    }

    /// Clamp all 6 DPI stage nibbles to max 14 (12000 DPI max hardware spec).
    /// Prevents out-of-bounds sensor calibration table lookups in MCU firmware.
    pub fn clamp_dpi_nibbles(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        for byte_idx in 0x2c..=0x2e {
            let low = (self.raw[byte_idx] & 0x0f).min(14);
            let high = ((self.raw[byte_idx] >> 4) & 0x0f).min(14);
            self.raw[byte_idx] = low | (high << 4);
        }
    }

    /// Convert USB HID Keyboard Usage ID (Page 0x07) to Windows Virtual-Key code (VK_*)
    /// matching the inverse of `EM624 Mouse.exe` table at `0x005c7018`.
    pub fn hid_usage_to_vk(hid: u8) -> u8 {
        match hid {
            0x04..=0x1d => b'A' + (hid - 0x04), // A..Z -> 0x41..0x5A
            0x1e..=0x26 => b'1' + (hid - 0x1e), // 1..9 -> 0x31..0x39
            0x27 => b'0',                       // 0 -> 0x30
            0x28 => 0x0d,                       // Enter -> VK_RETURN
            0x29 => 0x1b,                       // Esc -> VK_ESCAPE
            0x2a => 0x08,                       // Backspace -> VK_BACK
            0x2b => 0x09,                       // Tab -> VK_TAB
            0x2c => 0x20,                       // Space -> VK_SPACE
            0x2d => 0xbd,                       // - -> VK_OEM_MINUS
            0x2e => 0xbb,                       // = -> VK_OEM_PLUS
            0x2f => 0xdb,                       // [ -> VK_OEM_4
            0x30 => 0xdd,                       // ] -> VK_OEM_6
            0x31 => 0xdc,                       // \ -> VK_OEM_5
            0x33 => 0xba,                       // ; -> VK_OEM_1
            0x34 => 0xde,                       // ' -> VK_OEM_7
            0x35 => 0xc0,                       // ` -> VK_OEM_3
            0x36 => 0xbc,                       // , -> VK_OEM_COMMA
            0x37 => 0xbe,                       // . -> VK_OEM_PERIOD
            0x38 => 0xbf,                       // / -> VK_OEM_2
            0x39 => 0x14,                       // CapsLock -> VK_CAPITAL
            0x3a..=0x45 => 0x70 + (hid - 0x3a), // F1..F12 -> 0x70..0x7B
            0x46 => 0x2c,                       // PrintScreen -> VK_SNAPSHOT
            0x47 => 0x91,                       // ScrollLock -> VK_SCROLL
            0x48 => 0x13,                       // Pause -> VK_PAUSE
            0x49 => 0x2d,                       // Insert -> VK_INSERT
            0x4a => 0x24,                       // Home -> VK_HOME
            0x4b => 0x21,                       // PageUp -> VK_PRIOR
            0x4c => 0x2e,                       // Delete -> VK_DELETE
            0x4d => 0x23,                       // End -> VK_END
            0x4e => 0x22,                       // PageDown -> VK_NEXT
            0x4f => 0x27,                       // Right -> VK_RIGHT
            0x50 => 0x25,                       // Left -> VK_LEFT
            0x51 => 0x28,                       // Down -> VK_DOWN
            0x52 => 0x26,                       // Up -> VK_UP
            0x53 => 0x90,                       // NumLock -> VK_NUMLOCK
            0x54 => 0x6f,                       // Num / -> VK_DIVIDE
            0x55 => 0x6a,                       // Num * -> VK_MULTIPLY
            0x56 => 0x6d,                       // Num - -> VK_SUBTRACT
            0x57 => 0x6b,                       // Num + -> VK_ADD
            0x58 => 0x0d,                       // Num Enter -> VK_RETURN
            0x59..=0x61 => 0x61 + (hid - 0x59), // Num 1..9 -> 0x61..0x69
            0x62 => 0x60,                       // Num 0 -> VK_NUMPAD0
            0x63 => 0x6e,                       // Num . -> VK_DECIMAL
            _ => 0x00,
        }
    }

    /// Apply 100% standard mouse button actions (Left, Right, Middle, Forward, Backward, DPI Loop)
    /// to both Office Mode and Gaming Mode tables, while recalculating checksums.
    pub fn apply_standard_buttons(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        self.raw[0x00c0..0x00f0].copy_from_slice(&STANDARD_BUTTON_TABLE);
        self.raw[0x00f1..0x0121].copy_from_slice(&STANDARD_BUTTON_TABLE);

        // Hardware safety registers (verified against all 6 original EM624 .bin profiles):
        // 0x0005 = 0x0F (Package / CPI Indication / CPI Set / CPI Level)
        // 0x0006 = 0x7F (180 deg PCB sensor orientation, bits 7..6 = 01)
        // 0x0007 = 0x7F (EM624 hardware fuse byte)
        // 0x0008 = 0xAF (EM624 EFT / ProfileLEDFlash fuse byte)
        self.raw[0x0005] = 0x0f;
        self.raw[0x0006] = 0x7f;
        self.raw[0x0007] = 0x7f;
        self.raw[0x0008] = 0xaf;
        self.clamp_dpi_nibbles();
        self.raw[0x00a4] = 0x01; // Gaming Mode (0x01 in all 6 original EM624 .bin files)
        self.raw[0x00a6] &= 0x07; // EM624 current DPI stage index 0..5 (bits 5..3 = 0)
        self.raw[0x00a8] = 0x0f; // Active button mask
        self.raw[0x00aa] = 0x18; // Feature report enable
        self.raw[0x03f9] = 0x04; // Clear 0x80 button lockout flag in MCU footer

        self.recalculate_checksums();
        self.refresh_stages();
    }

    /// Keep profile's own native button bindings, polling rate (0x0004), and mode untouched,
    /// restore verified hardware optical sensor orientation (0x0006 = 0x7F) and hardware fuse bytes (0x0005 = 0x0F, 0x0007 = 0x7F, 0x0008 = 0xAF),
    /// clamp any out-of-bounds DPI nibbles, clear hardware lockout flag (bit 7 of 0x03F9), and recalculate all 5 checksums.
    pub fn apply_safe_profile(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        // 0. Enforce verified hardware optical sensor orientation and fuse registers (identical across all 6 original EM624 .bin files):
        // Byte 0x0005 = 0x0F, 0x0006 = 0x7F (180° PCB sensor orientation), 0x0007 = 0x7F, 0x0008 = 0xAF.
        self.raw[0x0005] = 0x0f;
        self.raw[0x0006] = 0x7f;
        self.raw[0x0007] = 0x7f;
        self.raw[0x0008] = 0xaf;

        // 1. Preserve Polling Rate in 0x0004 (bits 5..4: 00=125Hz, 01=250Hz, 10=500Hz, 11=1000Hz)
        // while keeping IC Type bits 7..6 = 11 and sensor bits 3..0 = 1111 (0xCF..0xFF)
        self.raw[0x0004] = (self.raw[0x0004] & 0x30) | 0xcf;

        // 2. Clamp any out-of-bounds DPI nibbles (> 14 / 12000 DPI) to 14
        self.clamp_dpi_nibbles();

        // 3. Ensure active mode byte 0x00A4 and companion gaming registers (0x00A8=0x0F, 0x00AA=0x18) match original EM624 .bin profiles
        if (self.raw[0x00a4] & 0x0f) != 0 {
            self.raw[0x00a4] = 0x01;
            self.raw[0x00a8] = 0x0f;
            self.raw[0x00aa] = 0x18;
        }
        // Preserve upper 3 bits of 0x00A6 (active stage count) and clamp current stage index (bits 2..0) to 0..5
        let upper = self.raw[0x00a6] & 0x38;
        let cur = (self.raw[0x00a6] & 0x07).min(5);
        self.raw[0x00a6] = upper | cur;

        // 4. Clear hardware button lockout bit (bit 7 of 0x03F9)
        self.raw[0x03f9] &= 0x7f;

        // 5. Recalculate all 5 hardware checksums
        self.recalculate_checksums();

        // 6. Refresh decoded stages and button bindings
        self.refresh_stages();
    }

    /// Get current USB polling rate (125, 250, 500, 1000 Hz) from register 0x0004 (bits 5..4).
    /// Verified from live hardware probe (`0x40 0x0D` -> `82 5a 01` -> `"I2C"` branch),
    /// `cb_RepRateChange` (`0x0056d860`), and `ReportRate_OnClick` (`0x00589084`):
    /// - 0b00 (`0xCF`): 125 Hz
    /// - 0b01 (`0xDF`): 250 Hz (`DailyUse.bin`)
    /// - 0b10 (`0xEF`): 500 Hz (`LN.bin`, `OSU.bin`, `WarThunder.bin`)
    /// - 0b11 (`0xFF`): 1000 Hz (`Bodycam.bin`, `Valorant+CS2.bin`)
    pub fn get_polling_rate(&self) -> u16 {
        if self.raw.len() != PROFILE_SIZE {
            return 1000;
        }
        match (self.raw[0x0004] >> 4) & 0x03 {
            0 => 125,
            1 => 250,
            2 => 500,
            _ => 1000,
        }
    }

    /// Set USB polling rate (125, 250, 500, 1000 Hz) in register 0x0004 (bits 5..4)
    /// and sync `0x00AC` (bits 1..0) so both register tables stay consistent.
    /// Bit formula: raw[0x0004] = 0xCF | (rate_bits << 4).
    pub fn set_polling_rate(&mut self, rate_hz: u16) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        let rate_bits: u8 = match rate_hz {
            125 => 0x00,
            250 => 0x01,
            500 => 0x02,
            _ => 0x03,
        };
        self.raw[0x0004] = (self.raw[0x0004] & 0x0f) | 0xc0 | (rate_bits << 4);
        self.raw[0x00ac] = (self.raw[0x00ac] & 0xfc) | rate_bits;
        self.recalculate_checksums();
    }

    /// Get active DPI stage count (1..=6) and current active DPI stage index (0..=5) from `0x00A6`.
    /// Verified from `DPILevel_OnClick` (`0x005861d4`) and `CurrentDPI_OnClick` (`0x00587c48`):
    /// Upper 3 bits (5..3) store `(num_stages - 1)`, or `0` when all 6 stages are active.
    pub fn get_active_dpi_config(&self) -> (u8, u8) {
        if self.raw.len() != PROFILE_SIZE {
            return (6, 0);
        }
        let a6 = self.raw[0x00a6];
        let upper = (a6 >> 3) & 0x07;
        let num_stages = if upper == 0 {
            6
        } else {
            (upper + 1).clamp(1, 6)
        };
        let current_stage = (a6 & 0x07).min(num_stages - 1);
        (num_stages, current_stage)
    }

    /// Set active DPI stage count (1..=6) and current active DPI stage index (0..=5) in `0x00A6`.
    pub fn set_active_dpi_config(&mut self, num_stages: u8, current_stage: u8) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        let n = num_stages.clamp(1, 6);
        let cur = current_stage.min(n - 1);
        let upper = if n == 6 { 0 } else { (n - 1) & 0x07 };
        self.raw[0x00a6] = (upper << 3) | (cur & 0x07);
        self.recalculate_checksums();
        self.refresh_stages();
    }

    /// Set 3-byte RGB color for DPI stage `stage_idx` (0..=5) at offset `0x0039 + stage_idx * 3`
    /// (`edt_Col1RChange` `0x00575b44` & `lbl_DPI1ColClick` `"I2C"` branch at `0x00587fa4`).
    pub fn set_stage_color(&mut self, stage_idx: usize, color: RgbColor) {
        if self.raw.len() != PROFILE_SIZE || stage_idx >= 6 {
            return;
        }
        let off = 0x0039 + stage_idx * 3;
        self.raw[off] = color.r;
        self.raw[off + 1] = color.g;
        self.raw[off + 2] = color.b;
        self.recalculate_checksums();
        self.refresh_stages();
    }

    /// Get LED mode index (0..=7) from register 0x00A5.
    /// Verified from `RGBImgMode_OnClick` (`0x00589784`) & `RGBMode_OnClick` (`0x0058a6dc`):
    /// - 0 (DPI Breathing): 0x81 (`LN.bin`, `Valorant+CS2.bin`)
    /// - 1 (Multicolor Breathing): 0x82
    /// - 2 (Rainbow / Steady): 0x83
    /// - 3 (Floe Light / Flow): 0x84 (`Bodycam.bin`, `OSU.bin`, `WarThunder.bin`)
    /// - 4 (Waltz): 0x85
    /// - 5 (Four Seasons): 0x86
    /// - 6 (Follow / Reactive): 0x87 (`DailyUse.bin`)
    /// - 7 (LED OFF): bit 7 == 0 (`raw[0x00A5] & 0x7F`)
    pub fn get_led_mode(&self) -> u8 {
        if self.raw.len() != PROFILE_SIZE {
            return 0;
        }
        let mode_a5 = self.raw[0x00a5];
        if (mode_a5 & 0x80) == 0 {
            7 // Off
        } else {
            let effect = mode_a5 & 0x7f;
            effect.saturating_sub(1).min(6)
        }
    }

    /// Set LED mode (0..=7) in register 0x00A5 (`0x81..0x87` when ON, or `raw[0x00A5] & 0x7F` when OFF).
    pub fn set_led_mode(&mut self, mode: u8) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        let clean_mode = mode.min(7);
        if clean_mode >= 7 {
            let base = (self.raw[0x00a5] & 0x7f).clamp(1, 7);
            self.raw[0x00a5] = base;
        } else {
            self.raw[0x00a5] = 0x80 | (clean_mode + 1);
        }
        self.recalculate_checksums();
    }

    /// Get LED speed step (1..=8) from register 0x0038 (`cb_MCULEDNumChange` `"I2C"` branch at `0x00571424`).
    /// Hardware table at 0x0038: `[0x0A, 0x28, 0x46, 0x64, 0x82, 0xA0, 0xBE, 0xDC]` (default `0x64` = step 4 in all 6 `.bin` files).
    pub fn get_led_speed(&self) -> u8 {
        if self.raw.len() != PROFILE_SIZE {
            return 4;
        }
        match self.raw[0x0038] {
            0..=0x19 => 1,
            0x1a..=0x37 => 2,
            0x38..=0x55 => 3,
            0x56..=0x73 => 4,
            0x74..=0x91 => 5,
            0x92..=0xaf => 6,
            0xb0..=0xcd => 7,
            _ => 8,
        }
    }

    /// Set LED speed step (1..=8) in register 0x0038 (`[0x0A, 0x28, 0x46, 0x64, 0x82, 0xA0, 0xBE, 0xDC]`).
    /// Preserves hardware fuse register 0x0007 (`0x7F`).
    pub fn set_led_speed(&mut self, speed: u8) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        const SPEED_TABLE: [u8; 8] = [0x0a, 0x28, 0x46, 0x64, 0x82, 0xa0, 0xbe, 0xdc];
        let idx = (speed.clamp(1, 8) - 1) as usize;
        self.raw[0x0038] = SPEED_TABLE[idx];
        self.recalculate_checksums();
    }

    /// Get LED brightness (0 = Full `0x80`, 1 = Half/Dim `0x40`) from upper nibble of 0x00A7 (`RGBLight_OnClick`).
    /// Preserves hardware EFT register 0x0008 (`0xAF`).
    pub fn get_led_brightness(&self) -> u8 {
        if self.raw.len() != PROFILE_SIZE {
            return 0;
        }
        if (self.raw[0x00a7] & 0xf0) == 0x40 {
            1
        } else {
            0
        }
    }

    /// Set LED brightness in 0x00A7 (`0x84` = Full, `0x44` = Half/Dim, matching `RGBLight_OnClick` at `0x00589390`).
    pub fn set_led_brightness(&mut self, dim: bool) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        self.raw[0x00a7] = if dim { 0x44 } else { 0x84 };
        self.raw[0x00a5] |= 0x80;
        self.recalculate_checksums();
    }

    /// Map 1-based physical button number (1..=7) to 0-based hardware slot in the 12-button table:
    /// - Button 1 (Left Click) -> Slot 0 (0x00C0 / 0x00F1)
    /// - Button 2 (Right Click) -> Slot 3 (0x00CC / 0x00FD)
    /// - Button 3 (Middle Click) -> Slot 6 (0x00D8 / 0x0109)
    /// - Button 4 (Forward / Side 1 Front) -> Slot 4 (0x00D0 / 0x0101)
    /// - Button 5 (Backward / Side 2 Rear) -> Slot 1 (0x00C4 / 0x00F5)
    /// - Button 6 (DPI + / Top Front) -> Slot 5 (0x00D4 / 0x0105)
    /// - Button 7 (DPI Loop / Top Back) -> Slot 7 (0x00DC / 0x010D)
    pub const fn button_num_to_slot(btn_num: u8) -> usize {
        match btn_num {
            1 => 0, // Left Click
            2 => 3, // Right Click
            3 => 6, // Middle Click
            4 => 4, // Forward (Side 1 Front)
            5 => 1, // Backward (Side 2 Rear)
            6 => 5, // DPI + (Top Front)
            7 => 7, // DPI Loop (Top Back)
            _ => 0,
        }
    }

    /// Map 0-based hardware slot in the 12-button table to 1-based physical button number (1..=7).
    #[allow(dead_code)]
    pub const fn slot_to_button_num(slot: usize) -> u8 {
        match slot {
            0 => 1,
            3 => 2,
            6 => 3,
            4 => 4,
            1 => 5,
            5 => 6,
            7 => 7,
            _ => 1,
        }
    }

    /// Set keybinding for physical button slot (0..11) and synchronize to both Office and Gaming tables.
    /// Auto-fills Param3 with the Windows VK code when Type == 0x02 (Keyboard) and Param3 == 0x00,
    /// matching `Key_AssemblyClick` (0x005828b0).
    pub fn set_button_binding(&mut self, slot_idx: usize, mut binding: [u8; 4]) {
        if self.raw.len() != PROFILE_SIZE || slot_idx >= 12 {
            return;
        }
        if binding[0] == 0x02 && binding[3] == 0x00 {
            binding[3] = Self::hid_usage_to_vk(binding[2]);
        }
        let office_offset = 0x00c0 + slot_idx * 4;
        let gaming_offset = 0x00f1 + slot_idx * 4;

        self.raw[office_offset..office_offset + 4].copy_from_slice(&binding);
        self.raw[gaming_offset..gaming_offset + 4].copy_from_slice(&binding);

        self.raw[0x00a4] = 0x01; // Gaming Mode active (0x01 in all 6 original EM624 .bin files)
        self.raw[0x00a8] = 0x0f; // Enable button matrix
        self.raw[0x00aa] = 0x18; // Feature report enable

        self.recalculate_checksums();
    }

    /// Refresh decoded button bindings and DPI stage structures from raw buffer
    pub fn refresh_decoded_buttons(&mut self) {
        self.refresh_stages();
    }

    /// Write compiled macro bytecode into profile onboard macro regions.
    #[allow(dead_code)]
    pub fn write_macro_bytecode(&mut self, slot: usize, bytecode: &[u8]) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }

        let len = bytecode.len().min(256);
        let target_offset = match slot {
            0 => 0x01f0,
            _ => 0x0338,
        };

        if target_offset + len <= PROFILE_SIZE {
            self.raw[target_offset..target_offset + len].copy_from_slice(&bytecode[..len]);
        }

        // Mirror into extended macro bank at 0x0400
        let ext_offset = 0x0400 + (slot * 256);
        if ext_offset + len <= PROFILE_SIZE {
            self.raw[ext_offset..ext_offset + len].copy_from_slice(&bytecode[..len]);
        }

        self.recalculate_checksums();
    }

    /// Check if the button configuration matches verified working standard mouse clicks
    #[allow(dead_code)]
    pub fn is_button_table_valid(&self) -> bool {
        if self.raw.len() != PROFILE_SIZE {
            return false;
        }
        // Check if button block matches the verified functional template
        self.raw[0xa0..0x128] == VERIFIED_BUTTON_BLOCK[..]
            && self.raw[0x3f8..0x400] == VERIFIED_FOOTER[..]
    }

    /// Apply full compatibility transform matching the original Delphi application:
    /// Preserves user DPI levels, active stages, and RGB LED colors while guaranteeing
    /// that button mapping tables and hardware registers match the working Delphi baseline.
    pub fn apply_og_compat(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }

        // 1. Extract user DPI levels (offsets 44..47) and 3-byte RGB LED colors (0x0039..0x004E)
        let mut dpi_nibbles = [0u8; 3];
        dpi_nibbles.copy_from_slice(&self.raw[0x2c..0x2f]);

        let mut rgb_colors = [0u8; 21];
        rgb_colors.copy_from_slice(&self.raw[0x39..0x4e]);

        let user_rate_byte = self.raw[0x0004];
        let user_dpi_cfg = self.raw[0x00a6];
        let user_poll_ac = self.raw[0x00ac];

        // 2. Clone from verified working hardware baseline (DailyUse)
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        self.raw.copy_from_slice(baseline);

        // 3. Inject user configuration preserving Polling Rate in 0x0004 (bits 5..4) and DPI config in 0x00A6
        self.raw[0x0004] = (user_rate_byte & 0x30) | 0xcf;
        let upper = user_dpi_cfg & 0x38;
        let cur = (user_dpi_cfg & 0x07).min(5);
        self.raw[0x00a6] = upper | cur;
        self.raw[0x00ac] = user_poll_ac & 0x03;

        // Restore verified hardware sensor orientation (byte 0x0006 = 0x7F) and hardware fuses (0x0005=0x0F, 0x0007=0x7F, 0x0008=0xAF)
        self.raw[0x0005] = 0x0f;
        self.raw[0x0006] = 0x7f;
        self.raw[0x0007] = 0x7f;
        self.raw[0x0008] = 0xaf;

        // Inject user DPI stages and 3-byte RGB colors
        self.raw[0x2c..0x2f].copy_from_slice(&dpi_nibbles);
        self.raw[0x39..0x4e].copy_from_slice(&rgb_colors);

        // Recalculate Sector 0 checksum (0x00A3) and all sector checksums
        self.recalculate_checksums();
        self.refresh_stages();
    }

    /// Sanitize button table and mode flags with the verified standard working template.
    /// Preserves all user DPI stages, DPI levels, LED colors, and lighting effects.
    #[allow(dead_code)]
    pub fn sanitize_buttons(&mut self) -> bool {
        if self.raw.len() != PROFILE_SIZE {
            return false;
        }
        let was_valid = self.is_button_table_valid();
        self.raw[0xa0..0x128].copy_from_slice(&VERIFIED_BUTTON_BLOCK);
        self.raw[0x3f8..0x400].copy_from_slice(&VERIFIED_FOOTER);
        !was_valid
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut buf = Vec::with_capacity(PROFILE_SIZE);
        file.read_to_end(&mut buf)?;
        Self::from_bytes(&buf)
    }

    #[allow(dead_code)]
    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.raw)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hid_key_name_full_keyboard() {
        assert_eq!(format_hid_key_name(0x4e), "Page Down");
        assert_eq!(format_hid_key_name(0x4b), "Page Up");
        assert_eq!(format_hid_key_name(0x2c), "Space");
        assert_eq!(format_hid_key_name(0x28), "Enter");
        assert_eq!(format_hid_key_name(0x58), "Numpad Enter");
        assert_eq!(format_hid_key_name(0x59), "Numpad 1");
        assert_eq!(format_hid_key_name(0x3a), "F1");
        assert_eq!(format_hid_key_name(0x45), "F12");
        assert_eq!(format_modifiers(0x01), "Ctrl+");
        assert_eq!(format_modifiers(0x08), "Win+");
        assert_eq!(format_modifiers(0x0a), "Shift+Win+");
    }

    #[test]
    fn test_daily_use_checksums() {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let profile = MouseProfile::from_bytes(baseline).expect("Failed to load DailyUse baseline");

        // Verify all 5 hardware checksums on the baseline profile
        let c0 = MouseProfile::calculate_checksum(&profile.raw[0x0000..0x00a3]);
        let c1 = MouseProfile::calculate_checksum(&profile.raw[0x00a4..0x00bf]);
        let c2 = MouseProfile::calculate_checksum(&profile.raw[0x00c0..0x00f0]);
        let c3 = MouseProfile::calculate_checksum(&profile.raw[0x00f1..0x0121]);
        let c4 = MouseProfile::calculate_checksum(&profile.raw[0x03f8..0x03ff]);

        assert_eq!(c0, profile.raw[0x00a3], "Sector 0 checksum mismatch");
        assert_eq!(c1, profile.raw[0x00bf], "Sector 1 checksum mismatch");
        assert_eq!(c2, profile.raw[0x00f0], "Sector 2 checksum mismatch");
        assert_eq!(c3, profile.raw[0x0121], "Sector 3 checksum mismatch");
        assert_eq!(c4, profile.raw[0x03ff], "Sector Footer checksum mismatch");
    }

    #[test]
    fn test_safe_profile_clamping() {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let mut profile = MouseProfile::from_bytes(baseline).unwrap();

        // Simulate an out-of-bounds stage 6 nibble (e.g. 14 = 0x0E)
        profile.raw[0x2e] = 0xec;
        profile.apply_safe_profile();

        // Ensure high nibble was clamped to <= 14 (12000 DPI max)
        let high_nibble = (profile.raw[0x2e] >> 4) & 0x0f;
        assert!(
            high_nibble <= 14,
            "Nibble should be clamped <= 14, got {}",
            high_nibble
        );
        // Ensure Polling Rate in 0x0004 (250 Hz = 0xDF in DailyUse) and hardware fuses (0x0007=0x7F, 0x0008=0xAF) are preserved
        assert_eq!(profile.raw[0x0004], 0xdf);
        assert_eq!(profile.raw[0x0007], 0x7f);
        assert_eq!(profile.raw[0x0008], 0xaf);
    }

    #[test]
    fn test_button_rebind_refresh() {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let mut profile = MouseProfile::from_bytes(baseline).unwrap();

        // Slot 1 (Left Key) in Gaming Mode is at 0x00F1..0x00F5
        // Rebind to Keyboard Key 'A' (0x02, 0x00, 0x04, 0x00)
        profile.raw[0x00f1..0x00f5].copy_from_slice(&[0x02, 0x00, 0x04, 0x00]);
        profile.refresh_stages();

        assert_eq!(profile.gaming_buttons[0].description, "A");

        // Rebind Slot 2 (Forward Side 1) in Gaming Mode at 0x00F5..0x00F9 to Volume Up (0x03, 0x00, 0xe9, 0x00)
        profile.raw[0x00f5..0x00f9].copy_from_slice(&[0x03, 0x00, 0xe9, 0x00]);
        profile.apply_safe_profile(); // apply_safe_profile calls refresh_stages

        assert_eq!(profile.gaming_buttons[1].description, "Media: Vol+");
    }

    #[test]
    fn test_sensor_orientation_safeguard() {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let mut profile = MouseProfile::from_bytes(baseline).unwrap();

        // Baseline profile has verified hardware value 0x7F (180 deg physical mounting)
        assert_eq!(profile.raw[0x0006], 0x7f);

        // Simulate corrupted/mutated orientation register (e.g., 0x00 or other rotated value)
        profile.raw[0x0006] = 0x00; // Inverted orientation
        assert_eq!(profile.raw[0x0006], 0x00);

        // apply_safe_profile must strictly restore standard hardware orientation (0x7F)
        profile.apply_safe_profile();
        assert_eq!(
            profile.raw[0x0006], 0x7f,
            "Sensor orientation must be restored to 0x7F (hardware normal)"
        );

        // apply_standard_buttons must also enforce 0x7F
        profile.raw[0x0006] = 0x01;
        profile.apply_standard_buttons();
        assert_eq!(
            profile.raw[0x0006], 0x7f,
            "Sensor orientation must be 0x7F in standard profile"
        );
    }

    #[test]
    fn test_write_macro_bytecode() {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let mut profile = MouseProfile::from_bytes(baseline).unwrap();

        let dummy_code = [0x82, 0x04, 0xfe, 0x00, 0x32, 0x02, 0x04, 0xfc];
        profile.write_macro_bytecode(0, &dummy_code);

        assert_eq!(&profile.raw[0x01f0..0x01f8], &dummy_code);
        assert_eq!(&profile.raw[0x0400..0x0408], &dummy_code);
    }

    #[test]
    fn test_hardware_register_setters() {
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        let mut profile = MouseProfile::from_bytes(baseline).unwrap();

        // 1. Polling Rate (0x0004 bits 5..4: 0xCF=125Hz, 0xDF=250Hz, 0xEF=500Hz, 0xFF=1000Hz)
        // Baseline DailyUse.bin has 0x0004 = 0xDF (250 Hz), LN.bin has 0xEF (500 Hz), Bodycam.bin has 0xFF (1000 Hz)
        assert_eq!(profile.get_polling_rate(), 250);
        assert_eq!(profile.raw[0x0004], 0xdf);

        profile.set_polling_rate(125);
        assert_eq!(profile.get_polling_rate(), 125);
        assert_eq!(profile.raw[0x0004], 0xcf);
        assert_eq!(profile.raw[0x00ac], 0x00);

        profile.set_polling_rate(250);
        assert_eq!(profile.get_polling_rate(), 250);
        assert_eq!(profile.raw[0x0004], 0xdf);
        assert_eq!(profile.raw[0x00ac], 0x01);

        profile.set_polling_rate(500);
        assert_eq!(profile.get_polling_rate(), 500);
        assert_eq!(profile.raw[0x0004], 0xef);
        assert_eq!(profile.raw[0x00ac], 0x02);

        profile.set_polling_rate(1000);
        assert_eq!(profile.get_polling_rate(), 1000);
        assert_eq!(profile.raw[0x0004], 0xff);
        assert_eq!(profile.raw[0x00ac], 0x03);
        profile.apply_safe_profile();
        assert_eq!(profile.get_polling_rate(), 1000);
        assert_eq!(profile.raw[0x0004], 0xff);
        assert_eq!(profile.raw[0x0007], 0x7f);
        assert_eq!(profile.raw[0x0008], 0xaf);

        // 2. Active DPI Stage Config (0x00A6: upper 3 bits = num_stages - 1 or 0 for 6 stages; lower 3 bits = current stage 0..5)
        profile.set_active_dpi_config(6, 0);
        assert_eq!(profile.raw[0x00a6], 0x00);
        assert_eq!(profile.get_active_dpi_config(), (6, 0));
        profile.set_active_dpi_config(5, 1);
        assert_eq!(profile.raw[0x00a6], (4 << 3) | 1);
        assert_eq!(profile.get_active_dpi_config(), (5, 1));

        // 2b. LED Mode (0x00A5: 0x81..0x87 when ON, bit 7=0 when OFF) and Speed (0x0038)
        profile.set_led_mode(0); // DPI Breathing -> 0x81
        assert_eq!(profile.get_led_mode(), 0);
        assert_eq!(profile.raw[0x00a5], 0x81);

        profile.set_led_mode(3); // Floe Light -> 0x84
        assert_eq!(profile.get_led_mode(), 3);
        assert_eq!(profile.raw[0x00a5], 0x84);

        profile.set_led_mode(6); // Follow / Reactive -> 0x87 (DailyUse.bin default)
        assert_eq!(profile.get_led_mode(), 6);
        assert_eq!(profile.raw[0x00a5], 0x87);

        profile.set_led_mode(7); // Off -> clears bit 7 (0x07)
        assert_eq!(profile.get_led_mode(), 7);
        assert_eq!(profile.raw[0x00a5], 0x07);

        profile.set_led_speed(4); // Default step 4 -> 0x64 at 0x0038
        profile.set_led_brightness(false); // Full brightness -> 0x00A7 upper nibble = 0x80
        assert_eq!(profile.get_led_speed(), 4);
        assert_eq!(profile.get_led_brightness(), 0);
        assert_eq!(profile.raw[0x0038], 0x64);
        assert_eq!(profile.raw[0x00a7], 0x84);
        assert_eq!(profile.raw[0x0007], 0x7f); // Hardware fuse untouched

        // 3. Stage RGB Colors (3-byte [R, G, B] starting at 0x0039 in DailyUse.em624: Green, Red, Blue, Cyan, Yellow, Magenta)
        assert_eq!(profile.dpi_stages[0].color, RgbColor { r: 0, g: 255, b: 0 });
        assert_eq!(profile.dpi_stages[1].color, RgbColor { r: 255, g: 0, b: 0 });
        assert_eq!(profile.dpi_stages[2].color, RgbColor { r: 0, g: 0, b: 255 });
        assert_eq!(
            profile.dpi_stages[3].color,
            RgbColor {
                r: 0,
                g: 255,
                b: 255
            }
        );
        assert_eq!(
            profile.dpi_stages[4].color,
            RgbColor {
                r: 255,
                g: 255,
                b: 0
            }
        );
        assert_eq!(
            profile.dpi_stages[5].color,
            RgbColor {
                r: 255,
                g: 128,
                b: 0
            }
        );
        profile.set_stage_color(
            0,
            RgbColor {
                r: 12,
                g: 34,
                b: 56,
            },
        );
        assert_eq!(&profile.raw[0x0039..0x003c], &[12, 34, 56]);
        assert_eq!(
            profile.dpi_stages[0].color,
            RgbColor {
                r: 12,
                g: 34,
                b: 56
            }
        );

        // 4. Button Remap (auto-populates Param3 VK code for Type 0x02)
        let key_a = [0x02, 0x00, 0x04, 0x00];
        profile.set_button_binding(1, key_a); // Slot 2 Forward
        assert_eq!(&profile.raw[0x00c4..0x00c8], &[0x02, 0x00, 0x04, 0x41]);
        assert_eq!(&profile.raw[0x00f5..0x00f9], &[0x02, 0x00, 0x04, 0x41]);
        assert_eq!(profile.raw[0x00a4], 0x01);

        // Verify checksums valid
        assert_eq!(
            MouseProfile::calculate_checksum(&profile.raw[0x0000..0x00a3]),
            profile.raw[0x00a3]
        );
        assert_eq!(
            MouseProfile::calculate_checksum(&profile.raw[0x00a4..0x00bf]),
            profile.raw[0x00bf]
        );
        assert_eq!(
            MouseProfile::calculate_checksum(&profile.raw[0x00c0..0x00f0]),
            profile.raw[0x00f0]
        );
        assert_eq!(
            MouseProfile::calculate_checksum(&profile.raw[0x00f1..0x0121]),
            profile.raw[0x0121]
        );
    }
}
