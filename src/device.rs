//! HID device abstraction for HyperX Cloud II Wireless DTS
use serde::{Serialize, Deserialize};
use hidapi::HidApi;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceState {
    pub connected: bool,
    pub device_id: String,
    pub name: String,
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
                Ok(len) => log::debug!("[HID] Flushed {} bytes cmd={:02X}", len, buf.get(3).unwrap_or(&0)),
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
                                    log::info!("[HID] Connected: battery={}%, raw={:02X?}", self.state.battery_percent, &buf[0..16.min(len)]);
                                    return Ok(());
                                }
                                Ok(len) => log::debug!("[HID] Battery probe invalid: len={} raw={:02X?}", len, &buf[0..16.min(len)]),
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

    pub fn refresh_state(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };

        Self::flush_input_buffer(device);
        self.prepare_write();

        let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
        write_hid_report(device, &packet)
            .map_err(|e| anyhow::anyhow!("battery write failed: {}", e))?;
        thread::sleep(RESPONSE_DELAY);

        let mut buf = [0u8; 256];
        let len = device.read_timeout(&mut buf, 1000)
            .map_err(|e| anyhow::anyhow!("battery read failed: {}", e))?;
        if len < 8 || !is_valid_response(&buf, len, GET_BATTERY_CMD_ID) {
            return Err(anyhow::anyhow!("invalid battery response: len={} raw={:02X?}", len, &buf[0..16.min(len)]));
        }
        self.state.battery_percent = buf[7].min(100);

        // Optional telemetry must never turn a healthy HID link into a disconnect.
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
        let Some(device) = self.device.as_ref() else { return Err(anyhow::anyhow!("Device not connected")); };
        let new_mute = !self.state.muted;
        self.prepare_write();
        let packet = build_packet(SET_MUTE_CMD_ID, &[new_mute as u8]);
        write_hid_report(device, &packet).map_err(|e| anyhow::anyhow!("mute command failed: {}", e))?;
        self.state.muted = new_mute;
        Ok(())
    }

    pub fn set_sidetone(&mut self, enabled: bool) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else { return Err(anyhow::anyhow!("Device not connected")); };
        self.prepare_write();
        let packet = build_packet(SET_SIDE_TONE_CMD_ID, &[enabled as u8]);
        write_hid_report(device, &packet).map_err(|e| anyhow::anyhow!("sidetone command failed: {}", e))?;
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

pub struct MultiDeviceManager {
    pub devices: Vec<HyperXDevice>,
    pub active_index: usize,
    api: HidApi,
}

impl MultiDeviceManager {
    pub fn new() -> Self {
        let api = HidApi::new().expect("Failed to initialize HID API");
        Self { devices: Vec::new(), active_index: 0, api }
    }

    pub fn is_enumerated(&self) -> bool {
        self.api.device_list().any(|info| {
            info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id())
        })
    }

    pub fn scan_and_connect(&mut self) -> anyhow::Result<()> {
        self.api.refresh_devices()?;
        let mut new_devices = Vec::new();

        for info in self.api.device_list() {
            if info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id()) {
                if let Ok(device) = self.api.open_path(info.path()) {
                    let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
                    if write_hid_report(&device, &packet).is_ok() {
                        let mut buf = [0u8; 256];
                        if let Ok(len) = device.read_timeout(&mut buf, 500) {
                            if len >= 8 && is_valid_response(&buf, len, GET_BATTERY_CMD_ID) {
                                let mut hx = HyperXDevice::new();
                                hx.device = Some(device);
                                hx.state.connected = true;
                                hx.state.device_id = info.path().to_string_lossy().to_string();
                                hx.state.name = info.product_string().map(|s| s.to_string()).unwrap_or_else(|| "HyperX Headset".to_string());
                                hx.state.battery_percent = buf[7].min(100);
                                new_devices.push(hx);
                            }
                        }
                    }
                }
            }
        }

        self.devices = new_devices;
        if self.active_index >= self.devices.len() {
            self.active_index = 0;
        }

        if self.devices.is_empty() {
            Err(anyhow::anyhow!("No devices found"))
        } else {
            Ok(())
        }
    }

    pub fn active_device(&mut self) -> Option<&mut HyperXDevice> {
        self.devices.get_mut(self.active_index)
    }

    pub fn active_state(&self) -> DeviceState {
        if let Some(dev) = self.devices.get(self.active_index) {
            dev.state.clone()
        } else {
            DeviceState::default()
        }
    }
}
