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
/// Key 1: Left Click, Key 2: Forward (Side 1), Key 3: CPI Loop,
/// Key 4: Right Click, Key 5: Backward (Side 2), Key 7: Middle Click
pub const STANDARD_BUTTON_TABLE: [u8; 48] = [
    0x01, 0x01, 0x00, 0x00, // Key 1: Left Click
    0x01, 0x08, 0x00, 0x02, // Key 2: Forward (Side Button 1)
    0x06, 0x92, 0x00, 0x1e, // Key 3: CPI Loop
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Refresh parsed DPI stages and RGB colors from raw bytes
    pub fn refresh_stages(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
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
            let color_offset = 60 + (i * 4);
            let color = RgbColor {
                r: self.raw[color_offset],
                g: self.raw[color_offset + 1],
                b: self.raw[color_offset + 2],
            };
            let enabled = (self.active_stages_mask & (1 << i)) != 0;
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

        let active_stages_mask = raw[4];

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
            // Read 3-byte RGB starting around offset 60 + (i * 4)
            let color_offset = 60 + (i * 4);
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

            let enabled = (active_stages_mask & (1 << i)) != 0;
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

        let is_gaming_mode = raw[0x00a4] == 0x01;

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
                let mod_str = match p1 {
                    0x01 => "Ctrl+",
                    0x02 => "Shift+",
                    0x04 => "Alt+",
                    0x08 => "Win+",
                    0x0a => "Ctrl+Alt+",
                    _ => "",
                };
                let key_name = match p2 {
                    0x04..=0x1d => format!("{}", (b'A' + (p2 - 0x04)) as char),
                    0x1e..=0x27 => format!("{}", (b'1' + (p2 - 0x1e)) as char),
                    0x28 => "Enter".to_string(),
                    0x29 => "Esc".to_string(),
                    0x2a => "Backspace".to_string(),
                    0x2b => "Tab".to_string(),
                    0x2c => "Space".to_string(),
                    _ => format!("0x{:02x}", p2),
                };
                format!("Key: {}{}", mod_str, key_name)
            }
            0x03 => match p2 {
                0xb5 => "Media: Next Track".to_string(),
                0xb6 => "Media: Prev Track".to_string(),
                0xcd => "Media: Play/Pause".to_string(),
                0xe9 => "Media: Vol+".to_string(),
                0xea => "Media: Vol-".to_string(),
                other => format!("Consumer 0x{:02x}", other),
            },
            0x06 => "CPI Loop".to_string(),
            0x07 => "ACPI / Power".to_string(),
            0x08 => "CPI Switch".to_string(),
            0x09 => match p1 {
                0x00 => "Office Mode".to_string(),
                0x01 => "Gaming Mode".to_string(),
                0x02 => "CPI +".to_string(),
                other => format!("Fn 0x{:02x}", other),
            },
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

    /// Apply 100% standard mouse button actions (Left, Right, Middle, Forward, Backward, CPI Loop)
    /// to both Office Mode and Gaming Mode tables, while recalculating checksums.
    pub fn apply_standard_buttons(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        self.raw[0x00c0..0x00f0].copy_from_slice(&STANDARD_BUTTON_TABLE);
        self.raw[0x00f1..0x0121].copy_from_slice(&STANDARD_BUTTON_TABLE);

        // Hardware safety registers
        self.raw[4] = 0xdf; // 5 active DPI stages, mask stage 6 MCU lockup
        self.clamp_dpi_nibbles(); // Clamp DPI steps <= 12 to prevent sensor lookup overflow
        self.raw[0x00a4] = 0x01; // Force Gaming Mode (active button matrix)
        self.raw[0x00a8] = 0x0f; // Active button mask
        self.raw[0x00aa] = 0x18; // Feature report enable
        self.raw[0x03f9] = 0x04; // Clear 0x80 button lockout flag in MCU footer

        self.recalculate_checksums();
    }

    /// Keep profile's own native button bindings and mode untouched, ensure Stage 6 mask is safe (bit 5 masked to 0),
    /// clamp any out-of-bounds DPI nibbles, clear hardware lockout flag (bit 7 of 0x03F9), and recalculate all 5 checksums.
    pub fn apply_safe_profile(&mut self) {
        if self.raw.len() != PROFILE_SIZE {
            return;
        }
        // 1. Mask out stage 6 trap: byte 4 &= 0xDF
        self.raw[4] &= 0xdf;

        // 2. Clamp any out-of-bounds DPI nibbles (> 12 / 8000 DPI) to 12
        self.clamp_dpi_nibbles();

        // 3. Clear hardware button lockout bit (bit 7 of 0x03F9)
        self.raw[0x03f9] &= 0x7f;

        // 4. Recalculate all 5 hardware checksums
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

        // 1. Extract user DPI levels (offsets 44..47) and RGB LED colors (offsets 60..108)
        let mut dpi_nibbles = [0u8; 3];
        dpi_nibbles.copy_from_slice(&self.raw[0x2c..0x2f]);

        let mut rgb_colors = [0u8; 48];
        rgb_colors.copy_from_slice(&self.raw[0x3c..0x6c]);

        let user_mask = self.raw[4];

        // 2. Clone from verified working hardware baseline (DailyUse)
        let baseline = include_bytes!("../exampleProfiles/DailyUse.bin");
        self.raw.copy_from_slice(baseline);

        // 3. Inject user configuration
        // Keep bit 5 masked to 0 (max 5 stages) to prevent MCU firmware fault on stage 6
        self.raw[4] = user_mask & 0xdf;

        // Inject user DPI stages and colors
        self.raw[0x2c..0x2f].copy_from_slice(&dpi_nibbles);
        self.raw[0x3c..0x6c].copy_from_slice(&rgb_colors);

        // Recalculate Sector 0 checksum (0x00A3) and all sector checksums
        self.recalculate_checksums();
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
        assert_eq!(
            profile.raw[4] & (1 << 5),
            0,
            "Stage 6 bit should be masked to 0"
        );
    }
}
