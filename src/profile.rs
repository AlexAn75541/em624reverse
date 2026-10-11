use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const PROFILE_SIZE: usize = 2048;
pub const PROFILE_MAGIC: [u8; 4] = [0x1b, 0xcf, 0x08, 0xb8];

/// Verified 136-byte button table and mode flags (offsets 0x00A0..0x0128)
/// from DailyUse.bin that ensures 100% working Left/Right/Middle/Forward/Back/DPI clicks.
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
pub const VERIFIED_FOOTER: [u8; 8] = [
    0x07, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x51,
];

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
pub struct MouseProfile {
    pub raw: Vec<u8>,
    pub device_name: String,
    pub active_stages_mask: u8,
    pub dpi_stages: Vec<DpiStage>,
}

impl MouseProfile {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        Self::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PROFILE_SIZE {
            bail!("Invalid profile length: {} bytes (expected {})", bytes.len(), PROFILE_SIZE);
        }
        if bytes[0..4] != PROFILE_MAGIC {
            bail!("Invalid profile magic: {:02x?} (expected {:02x?})", &bytes[0..4], PROFILE_MAGIC);
        }

        let raw = bytes.to_vec();

        let active_stages_mask = raw[4];

        // Device name string around offset 111 (0x6F)
        let name_bytes = &raw[111..130];
        let null_pos = name_bytes.iter().position(|&c| c == 0).unwrap_or(name_bytes.len());
        let device_name = String::from_utf8_lossy(&name_bytes[..null_pos]).trim().to_string();

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
                RgbColor { r: 255, g: 255, b: 255 }
            };

            let enabled = (active_stages_mask & (1 << i)) != 0;
            // Common SPCP6651B step: index 1=800, 2=1200, 3=1600, 4=2400, etc.
            let approx_dpi = (lvl as u32) * 400 + 400;

            dpi_stages.push(DpiStage {
                level_index: lvl,
                approx_dpi,
                color,
                enabled,
            });
        }

        Ok(Self {
            raw,
            device_name,
            active_stages_mask,
            dpi_stages,
        })
    }

    /// Check if the button configuration matches verified working standard mouse clicks
    pub fn is_button_table_valid(&self) -> bool {
        if self.raw.len() != PROFILE_SIZE {
            return false;
        }
        // Check if button block matches the verified functional template
        &self.raw[0xa0..0x128] == &VERIFIED_BUTTON_BLOCK[..]
            && &self.raw[0x3f8..0x400] == &VERIFIED_FOOTER[..]
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
        let baseline = include_bytes!("../profiles/DailyUse.bin");
        self.raw.copy_from_slice(baseline);

        // 3. Inject user configuration
        // Keep bit 5 masked to 0 (max 5 stages) to prevent MCU firmware fault on stage 6
        self.raw[4] = user_mask & 0xdf;

        // Inject user DPI stages and colors
        self.raw[0x2c..0x2f].copy_from_slice(&dpi_nibbles);
        self.raw[0x3c..0x6c].copy_from_slice(&rgb_colors);
    }

    /// Sanitize button table and mode flags with the verified standard working template.
    /// Preserves all user DPI stages, DPI levels, LED colors, and lighting effects.
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
