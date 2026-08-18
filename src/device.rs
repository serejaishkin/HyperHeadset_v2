//! HID device abstraction for HyperX Cloud II Wireless DTS
use serde::{Serialize, Deserialize};
use hidapi::HidApi;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceState {
    pub connected: bool,
    pub battery_percent: u8,
    pub charging: bool,
    pub muted: bool,
    pub sidetone: bool,
    pub voice_prompts: bool,
    pub signal_dbm: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceEvent {
    StateChanged(DeviceState),
    Connected,
    Disconnected,
    BatteryLow(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceCommand {
    ToggleMute,
    SetSidetone(bool),
    SetVoicePrompts(bool),
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
const GET_CHARGING_CMD_ID: u8 = 3;

const RESPONSE_DELAY: Duration = Duration::from_millis(50);

impl HyperXDevice {
    pub fn new() -> Self {
        Self { device: None, state: DeviceState::default() }
    }

    /// Returns true when Windows still enumerates a HyperX HID interface.
    /// This is deliberately separate from the open handle: an enumerated
    /// device with a broken handle points to an application/HID transport
    /// problem rather than a physical USB removal.
    pub fn is_enumerated() -> bool {
        match HidApi::new() {
            Ok(api) => api.device_list().any(|info| {
                info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id())
            }),
            Err(e) => {
                log::warn!("[HID] Cannot enumerate HID devices: {}", e);
                false
            }
        }
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
                    log::debug!("[HID] Flushed {} bytes cmd={:02X}", len, buf.get(3).unwrap_or(&0));
                }
            }
        }
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let api = HidApi::new()?;
        let mut candidates = Vec::new();
        for info in api.device_list() {
            if info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id()) {
                log::info!(
                    "[HID] Candidate VID={:04X} PID={:04X} interface={} usage_page={:04X} usage={:04X} path={:?}",
                    info.vendor_id(), info.product_id(), info.interface_number(),
                    info.usage_page(), info.usage(), info.path()
                );
                candidates.push(info);
            }
        }

        if candidates.is_empty() {
            return Err(anyhow::anyhow!("No HyperX HID device enumerated"));
        }

        for info in &candidates {
            match api.open_path(info.path()) {
                Ok(device) => {
                    let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
                    match write_hid_report(&device, &packet) {
                        Ok(_) => {
                            let mut buf = [0u8; 256];
                            match device.read_timeout(&mut buf, 1000) {
                                Ok(len) if len >= 8 && is_valid_response(&buf, len, GET_BATTERY_CMD_ID) => {
                                    self.device = Some(device);
                                    self.state.connected = true;
                                    self.state.battery_percent = buf[7].min(100);
                                    log::info!(
                                        "[HID] Connected: battery={}%, raw={:02X?}",
                                        self.state.battery_percent,
                                        &buf[0..16.min(len)]
                                    );
                                    return Ok(());
                                }
                                Ok(len) => {
                                    log::debug!("[HID] Battery probe invalid: len={} raw={:02X?}", len, &buf[0..16.min(len)]);
                                }
                                Err(e) => log::debug!("[HID] Battery probe read failed: {}", e),
                            }
                        }
                        Err(e) => log::debug!("[HID] Battery probe write failed: {}", e),
                    }
                }
                Err(e) => log::debug!("[HID] open_path failed: {}", e),
            }
        }

        Err(anyhow::anyhow!("HyperX HID enumerated, but no compatible HID interface answered"))
    }

    pub fn disconnect(&mut self) {
        self.device = None;
        self.state = DeviceState::default();
    }

    /// Refreshes the device state. Battery is the transport heartbeat.
    /// Optional telemetry (mute/charging) must not turn a healthy device
    /// into a disconnect merely because one command is unsupported or times out.
    pub fn refresh_state(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };

        Self::flush_input_buffer(device);
        self.prepare_write();

        // Battery is the authoritative heartbeat. A write/read/invalid
        // response is returned to the caller so the connection supervisor
        // can distinguish transport failure from optional telemetry failure.
        let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
        write_hid_report(device, &packet)
            .map_err(|e| anyhow::anyhow!("battery write failed: {}", e))?;
        thread::sleep(RESPONSE_DELAY);

        let mut buf = [0u8; 256];
        let len = device
            .read_timeout(&mut buf, 1000)
            .map_err(|e| anyhow::anyhow!("battery read failed: {}", e))?;
        if len < 8 || !is_valid_response(&buf, len, GET_BATTERY_CMD_ID) {
            return Err(anyhow::anyhow!(
                "invalid battery response: len={} raw={:02X?}",
                len,
                &buf[0..16.min(len)]
            ));
        }
        self.state.battery_percent = buf[7].min(100);

        // These are best-effort. They do NOT decide connection state.
        self.prepare_write();
        match send_and_read(device, GET_MUTE_CMD_ID, &[]) {
            Ok(status) => self.state.muted = status == 1,
            Err(e) => log::debug!("[HID] Mute telemetry unavailable: {}", e),
        }

        self.prepare_write();
        match send_and_read_with_raw(device, GET_CHARGING_CMD_ID, &[]) {
            Ok((status, _)) => self.state.charging = status == 1,
            Err(e) => log::debug!("[HID] Charging telemetry unavailable: {}", e),
        }

        Ok(())
    }

    pub fn toggle_mute(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        let new_mute = !self.state.muted;
        self.prepare_write();
        let packet = build_packet(SET_MUTE_CMD_ID, &[new_mute as u8]);
        write_hid_report(device, &packet)
            .map_err(|e| anyhow::anyhow!("mute command failed: {}", e))?;
        self.state.muted = new_mute;
        Ok(())
    }

    pub fn set_sidetone(&mut self, enabled: bool) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        self.prepare_write();
        let packet = build_packet(SET_SIDE_TONE_CMD_ID, &[enabled as u8]);
        write_hid_report(device, &packet)
            .map_err(|e| anyhow::anyhow!("sidetone command failed: {}", e))?;
        self.state.sidetone = enabled;
        Ok(())
    }

    pub fn set_voice_prompts(&mut self, _enabled: bool) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Voice prompts are not supported by the current HID protocol"))
    }
}

fn build_packet(cmd_id: u8, data: &[u8]) -> Vec<u8> {
    let mut packet = BASE_PACKET.to_vec();
    packet[3] = cmd_id;
    for (i, b) in data.iter().enumerate() {
        if 4 + i < packet.len() { packet[4 + i] = *b; }
    }
    packet
}

fn is_valid_response(buf: &[u8], len: usize, expected_cmd: u8) -> bool {
    len >= 5 && buf[0] == 0x06 && buf[1] == 0xFF && buf[2] == 0xBB && buf[3] == expected_cmd
}

fn send_and_read(device: &hidapi::HidDevice, cmd_id: u8, data: &[u8]) -> Result<u8, String> {
    let packet = build_packet(cmd_id, data);
    write_hid_report(device, &packet).map_err(|e| e.to_string())?;
    thread::sleep(RESPONSE_DELAY);
    let mut buf = [0u8; 256];
    match device.read_timeout(&mut buf, 1000) {
        Ok(len) if len >= 5 && is_valid_response(&buf, len, cmd_id) => Ok(buf[4]),
        Ok(len) => Err(format!("invalid response: len={} raw={:02X?}", len, &buf[0..8.min(len)])),
        Err(e) => Err(format!("read failed: {}", e)),
    }
}

fn send_and_read_with_raw(device: &hidapi::HidDevice, cmd_id: u8, data: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let packet = build_packet(cmd_id, data);
    write_hid_report(device, &packet).map_err(|e| format!("write failed: {}", e))?;
    thread::sleep(RESPONSE_DELAY);
    let mut buf = [0u8; 256];
    match device.read_timeout(&mut buf, 1000) {
        Ok(0) => Err("read timeout/empty".into()),
        Ok(len) if len >= 5 && is_valid_response(&buf, len, cmd_id) => Ok((buf[4], buf[..len].to_vec())),
        Ok(len) => Err(format!("invalid response: len={} raw={:02X?}", len, &buf[0..8.min(len)])),
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
                        device.send_feature_report(packet).map_err(|_| write_err.clone())?;
                        return Ok(());
                    }
                }
            }
            Err(write_err.into())
        }
    }
}
