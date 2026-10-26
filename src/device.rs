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
pub const CMD_APPLY_SETTINGS: u8 = 0x0D;
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
                if dev.interface_number() == 1 || target_info.is_none() {
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
                if dev.interface_number() == 1 || fallback.is_none() {
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

    /// Flash an entire 2048-byte configuration profile to mouse memory
    pub fn write_profile(&self, data: &[u8]) -> Result<()> {
        if data.len() != PROFILE_SIZE {
            bail!(
                "Invalid profile size: expected {} bytes, got {}",
                PROFILE_SIZE,
                data.len()
            );
        }

        // Step 1: Start write command: 40 07 00 00 08 00 00 00
        let mut start_cmd = [0u8; 8];
        start_cmd[0] = REPORT_ID_CMD;
        start_cmd[1] = CMD_START_WRITE;
        start_cmd[2] = 0x00;
        start_cmd[3] = 0x00;
        start_cmd[4] = 0x08; // length 0x0800 (2048 bytes)
        start_cmd[5] = 0x00;
        start_cmd[6] = 0x00;
        start_cmd[7] = 0x00;
        self.send_feature_report(&start_cmd)
            .context("Failed to send Start Write command (0x40, 0x07)")?;
        sleep(Duration::from_millis(15));

        // Step 2: Send profile in 293 chunks of 7 bytes each
        for chunk_idx in 0..TOTAL_CHUNKS {
            let offset = chunk_idx * CHUNK_SIZE;
            let mut report = [0xFFu8; 8];
            report[0] = REPORT_ID_DATA;

            let end = (offset + CHUNK_SIZE).min(PROFILE_SIZE);
            if offset < PROFILE_SIZE {
                let slice = &data[offset..end];
                report[1..1 + slice.len()].copy_from_slice(slice);
            }

            self.send_feature_report(&report).with_context(|| {
                format!(
                    "Failed on chunk {}/{} (offset 0x{:04x})",
                    chunk_idx + 1,
                    TOTAL_CHUNKS,
                    offset
                )
            })?;
            sleep(Duration::from_millis(5));
        }

        // Step 3: Commit flash write (0x40, 0x0E)
        let mut commit_cmd = [0u8; 8];
        commit_cmd[0] = REPORT_ID_CMD;
        commit_cmd[1] = CMD_COMMIT_WRITE;
        match self.device.send_feature_report(&commit_cmd) {
            Ok(_) => (),
            Err(e) => {
                let err_str = e.to_string();
                // 0x1F (ERROR_GEN_FAILURE) or 0x48F (ERROR_DEVICE_NOT_CONNECTED):
                // Hardware acknowledge: Sunplus MCU flashes internal memory and resets USB
                if err_str.contains("0x0000001F")
                    || err_str.contains("0x0000048F")
                    || err_str.contains("31")
                {
                    // Flash successfully committed
                } else {
                    return Err(anyhow::Error::new(e))
                        .context("Failed to send Commit command (0x40, 0x0E)");
                }
            }
        }
        sleep(Duration::from_millis(250));

        Ok(())
    }

    /// Read active 2048-byte configuration profile directly from mouse flash memory
    pub fn read_profile(&self) -> Result<Vec<u8>> {
        // Step 1: Request full 2048-byte profile: 40 09 00 00 08 00 00 00
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

        // Step 2: Read all 293 chunks via GET_REPORT 0x41
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
        }

        profile_data.truncate(PROFILE_SIZE);
        if profile_data.len() != PROFILE_SIZE {
            bail!(
                "Incomplete profile read: got {} bytes, expected {}",
                profile_data.len(),
                PROFILE_SIZE
            );
        }

        Ok(profile_data)
    }

    /// Transmit Opcode 0x40 0x0D (Apply Settings / Soft Reset) to mouse MCU
    pub fn apply_soft_reset(&self) -> Result<()> {
        let mut cmd = [0u8; 8];
        cmd[0] = REPORT_ID_CMD;
        cmd[1] = CMD_APPLY_SETTINGS;
        self.send_feature_report(&cmd)
            .context("Failed to send Apply/Soft-Reset command (0x40, 0x0D)")?;
        sleep(Duration::from_millis(15));
        Ok(())
    }
}
