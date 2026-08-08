//! HID device abstraction for HyperX Cloud II Wireless DTS
use crate::audio::voice;

use hidapi::HidApi;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct DeviceState {
    pub connected: bool,
    pub battery_percent: u8,
    pub charging: bool,
    pub muted: bool,
    pub sidetone: bool,
    pub voice_prompts: bool,
    pub signal_dbm: i8,
}

pub struct HyperXDevice {
    device: Option<hidapi::HidDevice>,
    pub state: DeviceState,
}

const VENDOR_ID: u16 = 0x03f0;
const PRODUCT_IDS: &[u16] = &[0x018b, 0x0d93, 0x0696, 0x1718];

const BASE_PACKET: [u8; 20] = [
    0x06, 0xff, 0xbb, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

const GET_BATTERY_CMD_ID: u8 = 2;
const GET_MUTE_CMD_ID: u8 = 5;
const SET_MUTE_CMD_ID: u8 = 32;
const SET_SIDE_TONE_CMD_ID: u8 = 33;
const GET_SIDE_TONE_CMD_ID: u8 = 6;
const GET_CHARGING_CMD_ID: u8 = 3;

const RESPONSE_DELAY: Duration = Duration::from_millis(50);

impl HyperXDevice {
    pub fn new() -> Self {
        Self { device: None, state: DeviceState::default() }
    }

    fn prepare_write(&self) {
        let Some(device) = self.device.as_ref() else { return };
        let mut buf = [0u8; 64];
        buf[0] = 0x06;
        let _ = device.get_input_report(&mut buf);
    }

    fn flush_input_buffer(device: &hidapi::HidDevice) {
        let mut buf = [0u8; 256];
        for _ in 0..3 {
            match device.read_timeout(&mut buf, 5) {
                Ok(0) | Err(_) => break,
                Ok(len) => {
                    log::debug!("[Device] Flushed {} bytes cmd={:02X}", len, buf.get(3).unwrap_or(&0));
                }
            }
        }
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let api = HidApi::new()?;
        let mut candidates = Vec::new();
        for info in api.device_list() {
            if info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id()) {
                candidates.push(info);
            }
        }
        for info in &candidates {
            if let Ok(device) = api.open_path(info.path()) {
                let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
                if write_hid_report(&device, &packet).is_ok() {
                    let mut buf = [0u8; 256];
                    if let Ok(len) = device.read_timeout(&mut buf, 1000) {
                        if len >= 8 && is_valid_response(&buf, len, GET_BATTERY_CMD_ID) {
                            self.device = Some(device);
                            self.state.connected = true;
                            self.state.battery_percent = buf[7];
                            log::info!(
                                "[Device] connect() battery={}% raw[0..16]={:02X?}",
                                buf[7],
                                &buf[0..16.min(len)]
                            );
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err(anyhow::anyhow!("No HyperX device found"))
    }

    pub fn disconnect(&mut self) {
        self.device = None;
        self.state.connected = false;
    }

    pub fn refresh_state(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };

        Self::flush_input_buffer(device);

        // Battery (cmd 2, level at byte 7)
        self.prepare_write();
        let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
        if write_hid_report(device, &packet).is_ok() {
            thread::sleep(RESPONSE_DELAY);
            let mut buf = [0u8; 256];
            if let Ok(len) = device.read_timeout(&mut buf, 1000) {
                if len >= 8 && is_valid_response(&buf, len, GET_BATTERY_CMD_ID) {
                    self.state.battery_percent = buf[7];
                }
            }
        }

        // Mute (cmd 5, status at byte 4)
        self.prepare_write();
        if let Ok(status) = send_and_read(device, GET_MUTE_CMD_ID, &[]) {
            self.state.muted = status == 1;
        }

        // Charging (cmd 3, status at byte 4)
        self.prepare_write();
        log::debug!("[Device] Reading charging status...");
        match send_and_read_with_raw(device, GET_CHARGING_CMD_ID, &[]) {
            Ok((status, raw)) => {
                voice::vlog(&format!(
                    "[Device] Charging raw: status={} raw[0..8]={:02X?}",
                    status,
                    &raw[0..8.min(raw.len())]
                ));
                self.state.charging = status == 1;
            }
            Err(e) => {
                voice::vlog(&format!(
                    "[Device] Charging read FAILED: {}",
                    e
                ));
            }
        }

        Ok(())
    }

    pub fn toggle_mute(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        let new_mute = !self.state.muted;
        log::debug!("[Device] toggle_mute: current={}, target={}", self.state.muted, new_mute);
        
        self.prepare_write();
        let packet = build_packet(SET_MUTE_CMD_ID, &[new_mute as u8]);
        log::debug!("[Device] Mute packet: {:02X?}", packet);
        
        match write_hid_report(device, &packet) {
            Ok(_) => {
                self.state.muted = new_mute;
                Ok(())
            }
            Err(e) => {
                log::debug!("[Device] Mute command failed: {}", e);
                Err(e)
            }
        }
    }

    pub fn set_sidetone(&mut self, enabled: bool) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        log::debug!("[Device] set_sidetone: {}", enabled);
        
        self.prepare_write();
        let packet = build_packet(SET_SIDE_TONE_CMD_ID, &[enabled as u8]);
        match write_hid_report(device, &packet) {
            Ok(_) => {
                self.state.sidetone = enabled;
                Ok(())
            }
            Err(e) => {
                log::debug!("[Device] Sidetone command failed: {}", e);
                Err(e)
            }
        }
    }

    pub fn set_voice_prompts(&mut self, _enabled: bool) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Voice prompts not supported on this model"))
    }
}

fn build_packet(cmd_id: u8, data: &[u8]) -> Vec<u8> {
    let mut packet = BASE_PACKET.to_vec();
    packet[3] = cmd_id;
    for (i, b) in data.iter().enumerate() {
        if 4 + i < packet.len() {
            packet[4 + i] = *b;
        }
    }
    packet
}

fn is_valid_response(buf: &[u8], len: usize, expected_cmd: u8) -> bool {
    len >= 5 && buf[0] == 0x06 && buf[1] == 0xFF && buf[2] == 0xBB && buf[3] == expected_cmd
}

fn send_and_read(device: &hidapi::HidDevice, cmd_id: u8, data: &[u8]) -> Result<u8, ()> {
    let packet = build_packet(cmd_id, data);
    if write_hid_report(device, &packet).is_err() {
        return Err(());
    }
    thread::sleep(RESPONSE_DELAY);
    let mut buf = [0u8; 256];
    match device.read_timeout(&mut buf, 1000) {
        Ok(len) if len >= 5 && is_valid_response(&buf, len, cmd_id) => Ok(buf[4]),
        _ => Err(()),
    }
}

fn send_and_read_with_raw(device: &hidapi::HidDevice, cmd_id: u8, data: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let packet = build_packet(cmd_id, data);
    if let Err(e) = write_hid_report(device, &packet) {
        return Err(format!("write failed: {}", e));
    }
    thread::sleep(RESPONSE_DELAY);
    let mut buf = [0u8; 256];
    match device.read_timeout(&mut buf, 1000) {
        Ok(0) => Err("read timeout/empty".into()),
        Ok(len) => {
            if len >= 5 && is_valid_response(&buf, len, cmd_id) {
                Ok((buf[4], buf[..len].to_vec()))
            } else {
                Err(format!("invalid response: len={} buf[0..8]={:02X?}", len, &buf[0..8.min(len)]))
            }
        }
        Err(e) => Err(format!("read error: {}", e)),
    }
}

fn write_hid_report(device: &hidapi::HidDevice, packet: &[u8]) -> anyhow::Result<()> {
    match device.write(packet) {
        Ok(_) => Ok(()),
        Err(write_err) => {
            #[cfg(target_os = "windows")]
            {
                if let hidapi::HidError::HidApiError { message } = &write_err {
                    if message.contains("Incorrect function") || message.contains("(0x00000001)") {
                        if device.send_feature_report(packet).is_err() {
                            return Err(write_err.into());
                        }
                        return Ok(());
                    }
                }
            }
            Err(write_err.into())
        }
    }
}
