use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const MACRO_SLOT_COUNT: usize = 100;
pub const MACRO_NAME_LEN: usize = 16;
pub const MACRO_BYTECODE_LEN: usize = 256;
pub const MACRO_SLOT_LEN: usize = MACRO_NAME_LEN + MACRO_BYTECODE_LEN; // 272 bytes
pub const MACRO_FILE_LEN: usize = MACRO_SLOT_COUNT * MACRO_SLOT_LEN; // 27,200 bytes

// Opcodes reversed from EM624 Mouse.exe (VA 0x00595820..0x00595dfe)
pub const OP_MOUSE_LEFT_DOWN: u8 = 0xa1;
pub const OP_MOUSE_LEFT_UP: u8 = 0x21;
pub const OP_MOUSE_RIGHT_DOWN: u8 = 0xa2;
pub const OP_MOUSE_RIGHT_UP: u8 = 0x22;
pub const OP_MOUSE_MIDDLE_DOWN: u8 = 0xa4;
pub const OP_MOUSE_MIDDLE_UP: u8 = 0x24;
pub const OP_MOUSE_FORWARD_DOWN: u8 = 0xa8;
pub const OP_MOUSE_FORWARD_UP: u8 = 0x28;
pub const OP_MOUSE_BACKWARD_DOWN: u8 = 0xb0;
pub const OP_MOUSE_BACKWARD_UP: u8 = 0x30;

pub const OP_KEY_DOWN: u8 = 0x82;
pub const OP_KEY_UP: u8 = 0x02;

pub const OP_MODIFIER_DOWN: u8 = 0x81;
pub const OP_MODIFIER_UP: u8 = 0x01;

pub const OP_DELAY: u8 = 0xfe;
pub const OP_LOOP_COUNT: u8 = 0xfd;
pub const OP_TERMINATE: u8 = 0xfc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroAction {
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    KeyDown(u8),
    KeyUp(u8),
    ModifierDown(ModifierKey),
    ModifierUp(ModifierKey),
    Delay(u16),
    Loop(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierKey {
    Ctrl,
    Shift,
    Alt,
    Win,
    Other(u8),
}

impl ModifierKey {
    pub fn to_byte(self) -> u8 {
        match self {
            ModifierKey::Ctrl => 0x01,
            ModifierKey::Shift => 0x02,
            ModifierKey::Alt => 0x04,
            ModifierKey::Win => 0x08,
            ModifierKey::Other(b) => b,
        }
    }

    pub fn from_byte(b: u8) -> Self {
        match b {
            0x01 => ModifierKey::Ctrl,
            0x02 => ModifierKey::Shift,
            0x04 => ModifierKey::Alt,
            0x08 => ModifierKey::Win,
            _ => ModifierKey::Other(b),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEntry {
    pub name: String,
    pub actions: Vec<MacroAction>,
}

impl MacroEntry {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            actions: Vec::new(),
        }
    }

    /// Compile actions into 256-byte bytecode buffer
    pub fn compile_bytecode(&self) -> Result<[u8; MACRO_BYTECODE_LEN]> {
        let mut buf = [0u8; MACRO_BYTECODE_LEN];
        let mut pos = 0;

        for action in &self.actions {
            match action {
                MacroAction::MouseDown(btn) => {
                    if pos >= MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = match btn {
                        MouseButton::Left => OP_MOUSE_LEFT_DOWN,
                        MouseButton::Right => OP_MOUSE_RIGHT_DOWN,
                        MouseButton::Middle => OP_MOUSE_MIDDLE_DOWN,
                        MouseButton::Forward => OP_MOUSE_FORWARD_DOWN,
                        MouseButton::Backward => OP_MOUSE_BACKWARD_DOWN,
                    };
                    pos += 1;
                }
                MacroAction::MouseUp(btn) => {
                    if pos >= MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = match btn {
                        MouseButton::Left => OP_MOUSE_LEFT_UP,
                        MouseButton::Right => OP_MOUSE_RIGHT_UP,
                        MouseButton::Middle => OP_MOUSE_MIDDLE_UP,
                        MouseButton::Forward => OP_MOUSE_FORWARD_UP,
                        MouseButton::Backward => OP_MOUSE_BACKWARD_UP,
                    };
                    pos += 1;
                }
                MacroAction::KeyDown(key) => {
                    if pos + 2 > MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = OP_KEY_DOWN;
                    buf[pos + 1] = *key;
                    pos += 2;
                }
                MacroAction::KeyUp(key) => {
                    if pos + 2 > MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = OP_KEY_UP;
                    buf[pos + 1] = *key;
                    pos += 2;
                }
                MacroAction::ModifierDown(m) => {
                    if pos + 2 > MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = OP_MODIFIER_DOWN;
                    buf[pos + 1] = m.to_byte();
                    pos += 2;
                }
                MacroAction::ModifierUp(m) => {
                    if pos + 2 > MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = OP_MODIFIER_UP;
                    buf[pos + 1] = m.to_byte();
                    pos += 2;
                }
                MacroAction::Delay(ms) => {
                    if pos + 3 > MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = OP_DELAY;
                    buf[pos + 1] = ((*ms >> 8) & 0xff) as u8;
                    buf[pos + 2] = (*ms & 0xff) as u8;
                    pos += 3;
                }
                MacroAction::Loop(count) => {
                    if pos + 2 > MACRO_BYTECODE_LEN {
                        bail!("Macro bytecode exceeded 256 bytes limit");
                    }
                    buf[pos] = OP_LOOP_COUNT;
                    buf[pos + 1] = *count;
                    pos += 2;
                }
            }
        }

        // Add termination marker if room
        if pos < MACRO_BYTECODE_LEN {
            buf[pos] = OP_TERMINATE;
        }

        Ok(buf)
    }

    /// Disassemble a 256-byte bytecode buffer into high-level actions
    pub fn disassemble_bytecode(name: String, raw: &[u8]) -> Self {
        let mut actions = Vec::new();
        let mut idx = 0;

        while idx < raw.len().min(MACRO_BYTECODE_LEN) {
            let op = raw[idx];
            if op == 0x00 || op == OP_TERMINATE {
                break;
            }

            match op {
                OP_MOUSE_LEFT_DOWN => {
                    actions.push(MacroAction::MouseDown(MouseButton::Left));
                    idx += 1;
                }
                OP_MOUSE_LEFT_UP => {
                    actions.push(MacroAction::MouseUp(MouseButton::Left));
                    idx += 1;
                }
                OP_MOUSE_RIGHT_DOWN => {
                    actions.push(MacroAction::MouseDown(MouseButton::Right));
                    idx += 1;
                }
                OP_MOUSE_RIGHT_UP => {
                    actions.push(MacroAction::MouseUp(MouseButton::Right));
                    idx += 1;
                }
                OP_MOUSE_MIDDLE_DOWN => {
                    actions.push(MacroAction::MouseDown(MouseButton::Middle));
                    idx += 1;
                }
                OP_MOUSE_MIDDLE_UP => {
                    actions.push(MacroAction::MouseUp(MouseButton::Middle));
                    idx += 1;
                }
                OP_MOUSE_FORWARD_DOWN => {
                    actions.push(MacroAction::MouseDown(MouseButton::Forward));
                    idx += 1;
                }
                OP_MOUSE_FORWARD_UP => {
                    actions.push(MacroAction::MouseUp(MouseButton::Forward));
                    idx += 1;
                }
                OP_MOUSE_BACKWARD_DOWN => {
                    actions.push(MacroAction::MouseDown(MouseButton::Backward));
                    idx += 1;
                }
                OP_MOUSE_BACKWARD_UP => {
                    actions.push(MacroAction::MouseUp(MouseButton::Backward));
                    idx += 1;
                }
                OP_KEY_DOWN => {
                    if idx + 1 < raw.len() {
                        actions.push(MacroAction::KeyDown(raw[idx + 1]));
                        idx += 2;
                    } else {
                        break;
                    }
                }
                OP_KEY_UP => {
                    if idx + 1 < raw.len() {
                        actions.push(MacroAction::KeyUp(raw[idx + 1]));
                        idx += 2;
                    } else {
                        break;
                    }
                }
                OP_MODIFIER_DOWN => {
                    if idx + 1 < raw.len() {
                        actions.push(MacroAction::ModifierDown(ModifierKey::from_byte(
                            raw[idx + 1],
                        )));
                        idx += 2;
                    } else {
                        break;
                    }
                }
                OP_MODIFIER_UP => {
                    if idx + 1 < raw.len() {
                        actions.push(MacroAction::ModifierUp(ModifierKey::from_byte(
                            raw[idx + 1],
                        )));
                        idx += 2;
                    } else {
                        break;
                    }
                }
                OP_DELAY => {
                    if idx + 2 < raw.len() {
                        let ms = ((raw[idx + 1] as u16) << 8) | (raw[idx + 2] as u16);
                        actions.push(MacroAction::Delay(ms));
                        idx += 3;
                    } else {
                        break;
                    }
                }
                OP_LOOP_COUNT => {
                    if idx + 1 < raw.len() {
                        actions.push(MacroAction::Loop(raw[idx + 1]));
                        idx += 2;
                    } else {
                        break;
                    }
                }
                _ => {
                    idx += 1;
                }
            }
        }

        Self { name, actions }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MacroBank {
    pub macros: Vec<MacroEntry>,
}

impl MacroBank {
    pub fn load_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("Failed to read macro bank: {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != MACRO_FILE_LEN {
            bail!(
                "Invalid macro bank size: expected {} bytes, got {}",
                MACRO_FILE_LEN,
                bytes.len()
            );
        }

        let mut macros = Vec::new();
        for i in 0..MACRO_SLOT_COUNT {
            let offset = i * MACRO_SLOT_LEN;
            let name_bytes = &bytes[offset..offset + MACRO_NAME_LEN];
            let name_end = name_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(MACRO_NAME_LEN);
            let name = String::from_utf8_lossy(&name_bytes[..name_end])
                .trim()
                .to_string();

            if !name.is_empty() {
                let code_bytes = &bytes[offset + MACRO_NAME_LEN..offset + MACRO_SLOT_LEN];
                let entry = MacroEntry::disassemble_bytecode(name, code_bytes);
                macros.push(entry);
            }
        }

        Ok(Self { macros })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; MACRO_FILE_LEN];

        for (i, entry) in self.macros.iter().take(MACRO_SLOT_COUNT).enumerate() {
            let offset = i * MACRO_SLOT_LEN;
            let name_bytes = entry.name.as_bytes();
            let copy_len = name_bytes.len().min(MACRO_NAME_LEN - 1);
            buf[offset..offset + copy_len].copy_from_slice(&name_bytes[..copy_len]);

            let code = entry.compile_bytecode()?;
            buf[offset + MACRO_NAME_LEN..offset + MACRO_SLOT_LEN].copy_from_slice(&code);
        }

        Ok(buf)
    }

    pub fn save_file(&self, path: &Path) -> Result<()> {
        let bytes = self.to_bytes()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)
            .with_context(|| format!("Failed to save macro bank: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_compile_disassemble_roundtrip() {
        let mut entry = MacroEntry::new("Test Burst");
        entry
            .actions
            .push(MacroAction::MouseDown(MouseButton::Left));
        entry.actions.push(MacroAction::Delay(45));
        entry.actions.push(MacroAction::MouseUp(MouseButton::Left));
        entry.actions.push(MacroAction::Delay(150));
        entry.actions.push(MacroAction::KeyDown(0x11)); // 'W' key
        entry.actions.push(MacroAction::Delay(30));
        entry.actions.push(MacroAction::KeyUp(0x11));

        let bytecode = entry.compile_bytecode().unwrap();
        assert_eq!(bytecode[0], OP_MOUSE_LEFT_DOWN);
        assert_eq!(bytecode[1], OP_DELAY);
        assert_eq!(bytecode[2], 0x00);
        assert_eq!(bytecode[3], 45);
        assert_eq!(bytecode[4], OP_MOUSE_LEFT_UP);

        let roundtrip = MacroEntry::disassemble_bytecode("Test Burst".to_string(), &bytecode);
        assert_eq!(roundtrip.actions.len(), entry.actions.len());
        assert_eq!(roundtrip.actions, entry.actions);
    }

    #[test]
    fn test_empty_macro_bank_roundtrip() {
        let bank = MacroBank::default();
        let bytes = bank.to_bytes().unwrap();
        assert_eq!(bytes.len(), MACRO_FILE_LEN);

        let loaded = MacroBank::from_bytes(&bytes).unwrap();
        assert_eq!(loaded.macros.len(), 0);
    }
}
