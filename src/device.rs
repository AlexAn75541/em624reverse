use anyhow::{bail, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::thread::sleep;
use std::time::Duration;

pub const DEFAULT_VID: u16 = 0x1BCF; // Sunplus Innovation Technology
pub const DEFAULT_PID: u16 = 0x08B8; // EM624 Gaming Mouse

pub const REPORT_ID_CMD: u8 = 0x40;
pub const REPORT_ID_DATA: u8 = 0x41;

pub const CMD_START_WRITE: u8 = 0x07;
pub const CMD_READ: u8 = 0x09;
#[allow(dead_code)]
pub const CMD_APPLY_SETTINGS: u8 = 0x0D;
#[allow(dead_code)]
pub const CMD_COMMIT_WRITE: u8 = 0x0E;

pub const PROFILE_SIZE: usize = 2048;
pub const CHUNK_SIZE: usize = 7;
pub const TOTAL_CHUNKS: usize = 293;

#[allow(dead_code)]
pub struct InputDevice {
    pub device: HidDevice,
    pub interface_name: String,
}

pub struct Em624Device {
    device: HidDevice,
    pub vid: u16,
    pub pid: u16,
}

impl Em624Device {
    pub fn open_auto(api: &HidApi) -> Result<Self> {
        let matching = Self::find_candidate(api)?;
        let device = matching
            .open_device(api)
            .context("Failed to open HID device handle (check permissions or Win32 access)")?;

        println!(
            "Connected to HID Endpoint: Page=0x{:04x}, Usage=0x{:04x}, Path={}",
            matching.usage_page(),
            matching.usage(),
            matching.path().to_string_lossy()
        );

        Ok(Self {
            device,
            vid: matching.vendor_id(),
            pid: matching.product_id(),
        })
    }

    pub fn open_vid_pid(api: &HidApi, vid: u16, pid: u16) -> Result<Self> {
        let mut target_info: Option<&DeviceInfo> = None;
        for dev in api.device_list() {
            if dev.vendor_id() == vid && dev.product_id() == pid {
                let path = dev.path().to_string_lossy().to_lowercase();
                // Prioritize vendor-defined usage page (0xFF00) and Col04
                if dev.usage_page() == 0xff00 || path.contains("&col04") {
                    target_info = Some(dev);
                    break;
                }
                if dev.interface_number() == 1 || path.contains("hidraw1") || target_info.is_none()
                {
                    target_info = Some(dev);
                }
            }
        }

        let info = target_info.context(format!(
            "Device 0x{:04x}:0x{:04x} not found in connected HID devices",
            vid, pid
        ))?;

        let device = info.open_device(api).context("Failed to open HID device")?;

        println!(
            "Connected to HID Endpoint: Page=0x{:04x}, Usage=0x{:04x}, Path={}",
            info.usage_page(),
            info.usage(),
            info.path().to_string_lossy()
        );

        Ok(Self { device, vid, pid })
    }

    pub fn find_candidate(api: &HidApi) -> Result<&DeviceInfo> {
        let mut fallback: Option<&DeviceInfo> = None;

        for dev in api.device_list() {
            if dev.vendor_id() == DEFAULT_VID && dev.product_id() == DEFAULT_PID {
                let path = dev.path().to_string_lossy().to_lowercase();
                // Target vendor-defined configuration interface (Usage Page 0xFF00, Col04)
                if dev.usage_page() == 0xff00 || path.contains("&col04") {
                    return Ok(dev);
                }
                if dev.interface_number() == 1 || path.contains("hidraw1") || fallback.is_none() {
                    fallback = Some(dev);
                }
            }
        }

        if let Some(dev) = fallback {
            return Ok(dev);
        }

        bail!("No compatible EM624 mouse found with VID 0x1BCF and PID 0x08B8.");
    }

    pub fn send_feature_report(&self, report: &[u8]) -> Result<()> {
        self.device
            .send_feature_report(report)
            .context("Failed to send HID feature report to mouse")?;
        Ok(())
    }

    pub fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize> {
        let len = self
            .device
            .get_feature_report(buf)
            .context("Failed to read HID feature report from mouse")?;
        Ok(len)
    }

    /// Open available input endpoints (MI_00 mouse, MI_01 hotkeys/consumer) for live hardware listening
    #[allow(dead_code)]
    pub fn open_input_endpoints(api: &HidApi) -> Result<Vec<InputDevice>> {
        let mut endpoints = Vec::new();
        for dev in api.device_list() {
            if dev.vendor_id() == DEFAULT_VID && dev.product_id() == DEFAULT_PID {
                let path = dev.path().to_string_lossy().to_string();
                let iface_desc = if dev.interface_number() == 0 {
                    "MI_00 (Mouse Movement & Physical Buttons)".to_string()
                } else if path.contains("&col01") {
                    "MI_01:Col01 (Keyboard Hotkeys & Macros)".to_string()
                } else if path.contains("&col02") {
                    "MI_01:Col02 (Multimedia / Consumer Keys)".to_string()
                } else if path.contains("&col03") {
                    "MI_01:Col03 (System Power / Sleep Keys)".to_string()
                } else if path.contains("&col04") || dev.usage_page() == 0xff00 {
                    "MI_01:Col04 (Vendor Config / Feature Reports)".to_string()
                } else {
                    format!(
                        "Interface {} (Usage Page 0x{:04x})",
                        dev.interface_number(),
                        dev.usage_page()
                    )
                };

                if let Ok(device) = dev.open_device(api) {
                    endpoints.push(InputDevice {
                        device,
                        interface_name: iface_desc,
                    });
                }
            }
        }

        if endpoints.is_empty() {
            bail!("No connected EM624 mouse interfaces could be opened for live input listening.");
        }

        Ok(endpoints)
    }

    /// Write a block of registers (3 bytes per packet) using Opcode 0x40 0x05 (`FUN_005b8304` at `0x005b8304`).
    /// Verified live on EM624 (`0x40 0x0D` -> `41 82 5a 01` -> `"I2C"` EEPROM branch):
    /// Each `0x40 0x05` packet (`[0x40, 0x05, addr_msb, addr_lsb, d0, d1, d2, len_1_to_3]`) performs a hardware
    /// I2C EEPROM page write + live MCU register latch and requires >= 10ms pacing before the next USB control transfer.
    pub fn write_registers_3b(&self, start_addr: u16, slice: &[u8]) -> Result<()> {
        for (idx, chunk) in slice.chunks(3).enumerate() {
            let addr = start_addr + (idx as u16) * 3;
            let mut cmd = [0u8; 8];
            cmd[0] = REPORT_ID_CMD; // 0x40
            cmd[1] = 0x05; // Opcode 0x05 (1..3-byte direct I2C EEPROM + live register write)
            cmd[2] = ((addr >> 8) & 0xff) as u8;
            cmd[3] = (addr & 0xff) as u8;
            for (i, &b) in chunk.iter().enumerate() {
                cmd[4 + i] = b;
            }
            cmd[7] = chunk.len() as u8;
            self.send_feature_report(&cmd).with_context(|| {
                format!("Failed to write 3-byte register block at 0x{:04x}", addr)
            })?;
            sleep(Duration::from_millis(12));
        }
        Ok(())
    }

    /// Write a single register byte using Opcode 0x40 0x05 (`FUN_005b7f8c` at `0x005b7f8c`).
    #[allow(dead_code)]
    pub fn write_register_1b(&self, addr: u8, val: u8) -> Result<()> {
        let cmd = [REPORT_ID_CMD, 0x05, 0x00, addr, val, 0x00, 0x00, 0x01];
        self.send_feature_report(&cmd)
            .with_context(|| format!("Failed to write single register 0x{:02x}", addr))?;
        sleep(Duration::from_millis(12));
        Ok(())
    }

    /// Write a variable-length memory region using Opcode 0x40 0x07 + 7-byte 0x41 chunks
    /// matching `FUN_005b888c` (`0x005b888c` at `0x005b88b2..0x005b88f8`).
    /// Both `start_addr` and `len` are Big-Endian (`[0x40, 0x07, addr_msb, addr_lsb, len_msb, len_lsb, 0x00, 0x00]`).
    pub fn write_chunked_region(&self, start_addr: u16, slice: &[u8]) -> Result<()> {
        let len = slice.len() as u16;
        let start_cmd = [
            REPORT_ID_CMD,
            CMD_START_WRITE, // 0x07
            ((start_addr >> 8) & 0xff) as u8,
            (start_addr & 0xff) as u8,
            ((len >> 8) & 0xff) as u8,
            (len & 0xff) as u8,
            0x00,
            0x00,
        ];
        self.send_feature_report(&start_cmd).with_context(|| {
            format!(
                "Failed to start chunked write at 0x{:04x} (len {})",
                start_addr, len
            )
        })?;
        sleep(Duration::from_millis(12));

        for (chunk_idx, chunk) in slice.chunks(CHUNK_SIZE).enumerate() {
            let mut report = [0xffu8; 8];
            report[0] = REPORT_ID_DATA; // 0x41
            report[1..1 + chunk.len()].copy_from_slice(chunk);
            self.send_feature_report(&report).with_context(|| {
                format!(
                    "Failed on chunk {} of region 0x{:04x}",
                    chunk_idx + 1,
                    start_addr
                )
            })?;
            sleep(Duration::from_millis(8));
        }
        Ok(())
    }

    /// Flash an entire 2048-byte configuration profile to mouse memory using the exact
    /// decompiled `EM624 Mouse.exe` `"I2C"` sequence (`0x00560d54`, `MemoToDataBuf_OnClick` `0x005a02e0`,
    /// and `DefaultParameterConfirm1` `0x0059bdf8`), verified live against the connected EM624 mouse:
    /// 1. Sector 0 (`0x0000..0x00A3`, 164 bytes: Polling Rate `0x0004`, Sensor `0x0006`, DPI stages `0x002C..0x002E`,
    ///    LED Speed `0x0038`, 3-byte RGB Colors `0x0039..0x004D`, Checksum `0x00A3`) via `0x40 0x05` (3-byte writes, 12ms pacing).
    /// 2. Sector 1 (`0x00A4..0x00BF`, 28 bytes: Mode `0x00A4`, LED Mode `0x00A5`, Active DPI Config `0x00A6`,
    ///    LED Brightness `0x00A7`, Checksum `0x00BF`) via `0x40 0x05` (3-byte writes, 12ms pacing).
    /// 3. Sectors >= `0x00C0` (Office Buttons `0x00C0..0x00F0`, Gaming Buttons `0x00F1..0x0121`, Gun/Footer/Macro regions
    ///    `0x0122..0x07FF`) via `0x40 0x07` + 7-byte `0x41` chunks (`FUN_005b888c`).
    /// 4. Final hardware reload trigger `[0x40, 0x07, 0xB4, 0x30, 0x01, 0x00, 0x00, 0x00]` (`0x005a04ee..0x005a0502`).
    ///
    /// Flash an entire 2048-byte configuration profile to mouse memory with a progress callback `(fraction_0_to_1, stage_msg)`.
    pub fn write_profile_with_progress<F>(&self, data: &[u8], mut on_progress: F) -> Result<()>
    where
        F: FnMut(f32, &str),
    {
        if data.len() != PROFILE_SIZE {
            bail!(
                "Invalid profile size: expected {} bytes, got {}",
                PROFILE_SIZE,
                data.len()
            );
        }

        // 1. Write Sector 0 (0x0000..0x00A3, 164 bytes = 55 chunks of 3B) via 0x40 0x05
        let s0_chunks: Vec<&[u8]> = data[0x0000..0x00a4].chunks(3).collect();
        for (idx, chunk) in s0_chunks.iter().enumerate() {
            let addr = (idx as u16) * 3;
            let mut cmd = [0u8; 8];
            cmd[0] = REPORT_ID_CMD;
            cmd[1] = 0x05;
            cmd[2] = ((addr >> 8) & 0xff) as u8;
            cmd[3] = (addr & 0xff) as u8;
            for (i, &b) in chunk.iter().enumerate() {
                cmd[4 + i] = b;
            }
            cmd[7] = chunk.len() as u8;
            self.send_feature_report(&cmd).with_context(|| {
                format!("Failed to write Sector 0 register block at 0x{:04x}", addr)
            })?;
            sleep(Duration::from_millis(12));
            if idx % 5 == 0 {
                let frac = (idx as f32 / s0_chunks.len() as f32) * 0.35;
                on_progress(frac, "Flashing Sector 0 (DPI, Polling, RGB)...");
            }
        }

        // 2. Write Sector 1 (0x00A4..0x00BF, 28 bytes = 10 chunks of 3B) via 0x40 0x05
        on_progress(0.36, "Flashing Sector 1 (Mode & LED Control)...");
        self.write_registers_3b(0x00a4, &data[0x00a4..0x00c0])
            .context("Failed to write Sector 1 (0x00A4..0x00BF) via 0x40 0x05")?;

        // 3. Write Office Mode 12-button table (0x00C0..0x00F0, 49 bytes) via 0x40 0x07
        on_progress(0.45, "Flashing Office Button Table...");
        self.write_chunked_region(0x00c0, &data[0x00c0..0x00f1])
            .context("Failed to write Office Button Table (0x00C0..0x00F0)")?;

        // 4. Write Gaming Mode 12-button table (0x00F1..0x0121, 49 bytes) via 0x40 0x07
        on_progress(0.55, "Flashing Gaming Button Table...");
        self.write_chunked_region(0x00f1, &data[0x00f1..0x0122])
            .context("Failed to write Gaming Button Table (0x00F1..0x0121)")?;

        // 5. Write remaining profile regions (0x0122..0x07FF) via 0x40 0x07
        on_progress(0.65, "Flashing Hardware Macro & Footer Tables...");
        self.write_chunked_region(0x0122, &data[0x0122..0x012e])
            .context("Failed to write region 0x0122..0x012D")?;
        self.write_chunked_region(0x0130, &data[0x0130..0x01d8])
            .context("Failed to write region 0x0130..0x01D7")?;
        self.write_chunked_region(0x01d8, &data[0x01d8..0x03f0])
            .context("Failed to write region 0x01D8..0x03EF")?;
        self.write_chunked_region(0x03f0, &data[0x03f0..0x03fa])
            .context("Failed to write region 0x03F0..0x03F9")?;
        self.write_chunked_region(0x03fa, &data[0x03fa..0x0400])
            .context("Failed to write region 0x03FA..0x03FF")?;

        on_progress(0.80, "Flashing Extended Macro Bank...");
        self.write_chunked_region(0x0400, &data[0x0400..0x0800])
            .context("Failed to write macro region 0x0400..0x07FF")?;

        // Sync MCULED sub-controller command (0x40 0x01 0x07) matching cb_MCULEDNumChange (0x00571424)
        let sub_mode = data[0x03f8];
        if sub_mode <= 5 {
            let mcu_cmd = [REPORT_ID_CMD, 0x01, 0x07, sub_mode, 0x00, 0x00, 0x00, 0x00];
            let _ = self.send_feature_report(&mcu_cmd);
            sleep(Duration::from_millis(12));
            let step_cmd = [
                REPORT_ID_CMD,
                0x01,
                0x07,
                data[0x03fa],
                0x00,
                0x00,
                0x00,
                0x00,
            ];
            let _ = self.send_feature_report(&step_cmd);
        }

        sleep(Duration::from_millis(50));
        on_progress(1.0, "Flash Complete.");
        Ok(())
    }

    pub fn write_profile(&self, data: &[u8]) -> Result<()> {
        self.write_profile_with_progress(data, |_, _| {})
    }

    /// Fast live LED lighting sync (~250ms) matching `RGBImgMode_OnClick` (`0x00589784`),
    /// `RGBLight_OnClick` (`0x00589390`), and `cb_MCULEDNumChange` (`0x00571424`):
    /// 1. Writes LED Speed (`0x0038`) + 7x 3-byte RGB Colors (`0x0039..0x004D`) + Sector 0 Checksum (`0x00A3`) via `0x40 0x05`.
    /// 2. Writes Sector 1 (`0x00A4..0x00BF`: Mode `0x00A4`, LED Mode `0x00A5`, Stage `0x00A6`, Brightness `0x00A7`, Checksum `0x00BF`) via `0x40 0x05`.
    /// 3. Writes Footer (`0x03F0..0x03FF`) and MCULED sub-controller commands (`0x40 0x01 0x07`).
    pub fn sync_led_lighting(&self, data: &[u8]) -> Result<()> {
        if data.len() != PROFILE_SIZE {
            anyhow::bail!(
                "Invalid profile size: expected {} bytes, got {}",
                PROFILE_SIZE,
                data.len()
            );
        }

        // 1. Write 0x0036..0x004E (24 bytes = 8 chunks of 3B: covers 0x0038 speed + 0x0039..0x004D RGB colors)
        self.write_registers_3b(0x0036, &data[0x0036..0x004e])
            .context("Failed to write LED speed & RGB color registers (0x0036..0x004D)")?;

        // 2. Write Sector 0 Checksum at 0x00A3 (1 byte via 0x40 0x05 matching 0x005718b0)
        self.write_registers_3b(0x00a3, &data[0x00a3..0x00a4])
            .context("Failed to write Sector 0 checksum (0x00A3)")?;

        // 3. Write Sector 1 (0x00A4..0x00BF: LED mode 0x00A5, stage 0x00A6, brightness 0x00A7, checksum 0x00BF)
        self.write_registers_3b(0x00a4, &data[0x00a4..0x00c0])
            .context("Failed to write Sector 1 LED registers (0x00A4..0x00BF)")?;

        // 4. Write Footer (0x03F0..0x03FF) and MCULED sub-controller commands (0x40 0x01 0x07)
        self.write_chunked_region(0x03f0, &data[0x03f0..0x03fa])
            .context("Failed to write LED footer 0x03F0..0x03F9")?;
        self.write_chunked_region(0x03fa, &data[0x03fa..0x0400])
            .context("Failed to write LED footer 0x03FA..0x03FF")?;

        let sub_mode = data[0x03f8];
        if sub_mode <= 5 {
            let mcu_cmd = [REPORT_ID_CMD, 0x01, 0x07, sub_mode, 0x00, 0x00, 0x00, 0x00];
            let _ = self.send_feature_report(&mcu_cmd);
            sleep(Duration::from_millis(12));
            let step_cmd = [
                REPORT_ID_CMD,
                0x01,
                0x07,
                data[0x03fa],
                0x00,
                0x00,
                0x00,
                0x00,
            ];
            let _ = self.send_feature_report(&step_cmd);
        }

        Ok(())
    }

    /// Read active 2048-byte configuration profile with a progress callback `(fraction_0_to_1, stage_msg)`.
    pub fn read_profile_with_progress<F>(&self, mut on_progress: F) -> Result<Vec<u8>>
    where
        F: FnMut(f32, &str),
    {
        on_progress(0.02, "Requesting EEPROM read (0x40 0x09)...");
        let mut read_cmd = [0u8; 8];
        read_cmd[0] = REPORT_ID_CMD;
        read_cmd[1] = CMD_READ;
        read_cmd[2] = 0x00;
        read_cmd[3] = 0x00;
        read_cmd[4] = 0x08;
        read_cmd[5] = 0x00;
        read_cmd[6] = 0x00;
        read_cmd[7] = 0x00;
        self.send_feature_report(&read_cmd)
            .context("Failed to send Read command (0x40, 0x09)")?;
        sleep(Duration::from_millis(15));

        let mut profile_data = Vec::with_capacity(PROFILE_SIZE);
        for chunk_idx in 0..TOTAL_CHUNKS {
            let mut buf = [0u8; 8];
            buf[0] = REPORT_ID_DATA;
            let len = self.get_feature_report(&mut buf).with_context(|| {
                format!("Failed on reading chunk {}/{}", chunk_idx + 1, TOTAL_CHUNKS)
            })?;
            if len > 1 {
                profile_data.extend_from_slice(&buf[1..8]);
            }
            sleep(Duration::from_millis(4));
            if chunk_idx % 15 == 0 || chunk_idx + 1 == TOTAL_CHUNKS {
                let frac = (chunk_idx + 1) as f32 / TOTAL_CHUNKS as f32;
                on_progress(frac, "Reading mouse EEPROM profile...");
            }
        }

        profile_data.truncate(PROFILE_SIZE);
        if profile_data.len() != PROFILE_SIZE {
            bail!(
                "Incomplete profile read: got {} bytes, expected {}",
                profile_data.len(),
                PROFILE_SIZE
            );
        }

        on_progress(1.0, "Read Complete.");
        Ok(profile_data)
    }

    /// Read an arbitrary slice of registers/EEPROM using Opcode 0x40 0x09 (`FUN_005b8b04` at `0x005b8b04`).
    /// Dispatches `[0x40, 0x09, addr_msb, addr_lsb, len_msb, len_lsb, 0, 0]`, then drains 7-byte chunks via `0x41`.
    /// Used by Delphi to query active mode (`0x00A4`, 1 byte) and active DPI stage (`0x00A6`, 1 byte) in real-time.
    #[allow(dead_code)]
    pub fn read_registers(&self, offset: u16, len: u16) -> Result<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let read_cmd = [
            REPORT_ID_CMD,
            CMD_READ, // 0x09
            ((offset >> 8) & 0xff) as u8,
            (offset & 0xff) as u8,
            ((len >> 8) & 0xff) as u8,
            (len & 0xff) as u8,
            0x00,
            0x00,
        ];
        self.send_feature_report(&read_cmd).with_context(|| {
            format!(
                "Failed to send Read command (0x40 0x09) for offset 0x{:04X} len {}",
                offset, len
            )
        })?;
        sleep(Duration::from_millis(15));

        let num_chunks = (len as usize).div_ceil(CHUNK_SIZE);
        let mut result = Vec::with_capacity(num_chunks * CHUNK_SIZE);
        for chunk_idx in 0..num_chunks {
            let mut buf = [0u8; 8];
            buf[0] = REPORT_ID_DATA; // 0x41
            let read_len = self.get_feature_report(&mut buf).with_context(|| {
                format!(
                    "Failed reading chunk {}/{} at offset 0x{:04X}",
                    chunk_idx + 1,
                    num_chunks,
                    offset
                )
            })?;
            if read_len > 1 {
                result.extend_from_slice(&buf[1..8]);
            }
            sleep(Duration::from_millis(4));
        }

        result.truncate(len as usize);
        Ok(result)
    }

    /// Read active 2048-byte configuration profile directly from mouse flash memory
    pub fn read_profile(&self) -> Result<Vec<u8>> {
        self.read_profile_with_progress(|_, _| {})
    }

    /// Query the 3-byte hardware chip ID via `0x40 0x0D` followed immediately by `GetFeature(0x41)`
    /// (`0x00560d0b` / `0x005b8b04` -> returns `[0x41, 0x82, 0x5A, 0x01, ...]`).
    /// IMPORTANT: Never send `0x40 0x0D` without draining `GetFeature(0x41)`, or the MCU's control
    /// endpoint remains pending and blocks button/sensor dispatch until replug.
    #[allow(dead_code)]
    pub fn read_chip_id(&self) -> Result<[u8; 8]> {
        let mut cmd = [0u8; 8];
        cmd[0] = REPORT_ID_CMD;
        cmd[1] = CMD_APPLY_SETTINGS; // 0x0D (Hardware Chip ID Query)
        self.send_feature_report(&cmd)
            .context("Failed to send Chip ID query (0x40, 0x0D)")?;
        sleep(Duration::from_millis(15));
        let mut resp = [0u8; 8];
        resp[0] = REPORT_ID_DATA; // 0x41
        self.get_feature_report(&mut resp)
            .context("Failed to drain Chip ID response (0x41)")?;
        sleep(Duration::from_millis(15));
        Ok(resp)
    }

    /// Write 1 to 4 raw hardware registers via real-time Opcode 0x40 0x05 (Delphi direct setter)
    #[allow(dead_code)]
    pub fn write_registers(&self, start_addr: u8, values: &[u8]) -> Result<()> {
        if values.is_empty() || values.len() > 4 {
            bail!("write_registers accepts 1 to 4 bytes, got {}", values.len());
        }
        let mut cmd = [0u8; 8];
        cmd[0] = REPORT_ID_CMD; // 0x40
        cmd[1] = 0x05; // Opcode 0x05 (Direct register write)
        cmd[2] = 0x00;
        cmd[3] = start_addr; // Target register address
        for (i, &v) in values.iter().enumerate() {
            cmd[4 + i] = v;
        }
        cmd[7] = values.len() as u8; // Byte count
        self.send_feature_report(&cmd).context(format!(
            "Failed to write register 0x{:02X} via 0x40 0x05",
            start_addr
        ))?;
        sleep(Duration::from_millis(10));
        Ok(())
    }
}
