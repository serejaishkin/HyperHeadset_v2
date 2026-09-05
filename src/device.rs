//! HID device abstraction for HyperX Cloud II Wireless DTS
use serde::{Serialize, Deserialize};
use hidapi::HidApi;
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
mod win_enumerate {
    use std::ptr::{null, null_mut};

    type HDEVINFO = *mut core::ffi::c_void;
    type HANDLE = *mut core::ffi::c_void;
    type BOOL = i32;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct SP_DEVICE_INTERFACE_DATA {
        cbSize: u32,
        InterfaceClassGuid: GUID,
        Flags: u32,
        Reserved: usize,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct SP_DEVICE_INTERFACE_DETAIL_DATA_W {
        cbSize: u32,
        DevicePath: [u16; 1],
    }

    #[link(name = "setupapi")]
    #[link(name = "hid")]
    extern "system" {
        fn HidD_GetHidGuid() -> GUID;
        fn SetupDiGetClassDevsW(
            classguid: *const GUID,
            enumerator: *const u16,
            hwndparent: HANDLE,
            flags: u32,
        ) -> HDEVINFO;
        fn SetupDiEnumDeviceInterfaces(
            deviceinfoset: HDEVINFO,
            deviceinfodata: *const core::ffi::c_void,
            interfaceclassguid: *const GUID,
            memberindex: u32,
            deviceinterfacedata: *mut SP_DEVICE_INTERFACE_DATA,
        ) -> BOOL;
        fn SetupDiGetDeviceInterfaceDetailW(
            deviceinfoset: HDEVINFO,
            deviceinterfacedata: *const SP_DEVICE_INTERFACE_DATA,
            deviceinterfacedetaildata: *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W,
            deviceinterfacedetaildatasize: u32,
            requiredsize: *mut u32,
            deviceinfodata: *mut core::ffi::c_void,
        ) -> BOOL;
        fn SetupDiDestroyDeviceInfoList(deviceinfoset: HDEVINFO) -> BOOL;
    }

    const DIGCF_PRESENT: u32 = 0x02;
    const DIGCF_DEVICEINTERFACE: u32 = 0x10;
    const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    pub fn enumerate_hid_paths() -> Vec<String> {
        let guid = unsafe { HidD_GetHidGuid() };

        let dev_info = unsafe {
            SetupDiGetClassDevsW(
                &guid,
                null(),
                INVALID_HANDLE_VALUE,
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        };
        if dev_info == INVALID_HANDLE_VALUE {
            log::warn!("[HID] SetupDiGetClassDevsW failed");
            return Vec::new();
        }

        let mut paths = Vec::new();
        let mut idx = 0u32;
        loop {
            let mut interface_data: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
            interface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            let result = unsafe {
                SetupDiEnumDeviceInterfaces(
                    dev_info,
                    null(),
                    &guid,
                    idx,
                    &mut interface_data,
                )
            };
            if result == 0 { break; }
            idx += 1;

            let mut required_size = 0u32;
            unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    dev_info,
                    &interface_data,
                    null_mut(),
                    0,
                    &mut required_size,
                    null_mut(),
                )
            };

            let detail_size = required_size as usize;
            if detail_size == 0 { continue; }

            let alloc_size = detail_size + 2;
            let mut buf: Vec<u8> = vec![0u8; alloc_size];
            let detail_ptr = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            unsafe {
                (*detail_ptr).cbSize = 8;
            }

            let result = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    dev_info,
                    &interface_data,
                    detail_ptr,
                    required_size,
                    null_mut(),
                    null_mut(),
                )
            };
            if result == 0 { continue; }

            let path_ptr = unsafe { (*detail_ptr).DevicePath.as_ptr() };
            let path_wide: Vec<u16> = unsafe {
                let mut len = 0;
                while *path_ptr.add(len) != 0 { len += 1; }
                std::slice::from_raw_parts(path_ptr, len).to_vec()
            };
            if let Ok(path) = String::from_utf16(&path_wide) {
                paths.push(path);
            }
        }

        unsafe { SetupDiDestroyDeviceInfoList(dev_info); };
        paths
    }

    pub fn filter_paths_by_vid_pid(paths: &[String], vid: u16, pids: &[u16]) -> Vec<String> {
        let vid_lower = format!("vid_{:04x}", vid);
        paths.iter().filter(|p| {
            let pl = p.to_lowercase();
            if !pl.contains(&vid_lower) { return false; }
            pids.iter().any(|pid| {
                let pid_str = format!("pid_{:04x}", pid);
                pl.contains(&pid_str)
            })
        }).cloned().collect()
    }
}

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
    pub fn new() -> Option<Self> {
        use std::sync::mpsc;
        log::info!("[HID] Initializing HidApi (no-enumerate, timeout 5s)...");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                HidApi::disable_device_discovery();
                HidApi::new()
            });
            let _ = tx.send(result);
        });
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(Ok(api))) => { log::info!("[HID] HidApi initialized OK (no-enumerate)"); Some(Self { devices: Vec::new(), active_index: 0, api }) }
            Ok(Ok(Err(e))) => { log::error!("[HID] HidApi::new() error: {}", e); None }
            Ok(Err(_)) => { log::error!("[HID] HidApi::new() panicked"); None }
            Err(_) => { log::error!("[HID] HidApi::new() timed out after 5s"); None }
        }
    }

    pub fn is_enumerated(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            let paths = win_enumerate::enumerate_hid_paths();
            let filtered = win_enumerate::filter_paths_by_vid_pid(&paths, VENDOR_ID, PRODUCT_IDS);
            !filtered.is_empty()
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.api.device_list().any(|info| {
                info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id())
            })
        }
    }

    pub fn scan_and_connect(&mut self) -> anyhow::Result<()> {
        log::info!("[HID] scan_and_connect: enumerating HID paths via Win32...");

        #[cfg(target_os = "windows")]
        {
            let paths = win_enumerate::enumerate_hid_paths();
            log::info!("[HID] scan: total HID paths={}", paths.len());
            let candidates = win_enumerate::filter_paths_by_vid_pid(&paths, VENDOR_ID, PRODUCT_IDS);
            log::info!("[HID] scan: {} candidate paths after VID/PID filter", candidates.len());

            let mut new_devices = Vec::new();
            for path in &candidates {
                log::info!("[HID] scan: trying path={}", path);
                match self.api.open_path(std::ffi::CString::new(path.as_str()).unwrap().as_c_str()) {
                    Ok(device) => {
                        let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
                        match write_hid_report(&device, &packet) {
                            Ok(_) => {
                                let mut buf = [0u8; 256];
                                match device.read_timeout(&mut buf, 500) {
                                    Ok(len) if len >= 8 && is_valid_response(&buf, len, GET_BATTERY_CMD_ID) => {
                                        let mut hx = HyperXDevice::new();
                                        hx.device = Some(device);
                                        hx.state.connected = true;
                                        hx.state.device_id = path.clone();
                                        hx.state.name = "HyperX Headset".to_string();
                                        hx.state.battery_percent = buf[7].min(100);
                                        log::info!("[HID] scan: connected battery={}%", hx.state.battery_percent);
                                        new_devices.push(hx);
                                    }
                                    Ok(len) => log::info!("[HID] scan: probe got {} bytes, not valid response: {:?}", len, &buf[0..8.min(len)]),
                                    Err(e) => log::info!("[HID] scan: probe read failed: {}", e),
                                }
                            }
                            Err(e) => log::info!("[HID] scan: probe write failed: {}", e),
                        }
                    }
                    Err(e) => log::info!("[HID] scan: open_path failed: {}", e),
                }
            }

            log::info!("[HID] scan: {} candidates, {} connected", candidates.len(), new_devices.len());
            self.devices = new_devices;
        }

        #[cfg(not(target_os = "windows"))]
        {
            log::info!("[HID] scan_and_connect: refreshing devices...");
            self.api.refresh_devices()?;
            let mut new_devices = Vec::new();
            let mut candidates_seen = 0u32;

            for info in self.api.device_list() {
                if info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id()) {
                    candidates_seen += 1;
                    log::info!("[HID] scan: candidate VID={:04X} PID={:04X} path={:?}",
                        info.vendor_id(), info.product_id(), info.path());
                    match self.api.open_path(info.path()) {
                        Ok(device) => {
                            let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
                            match write_hid_report(&device, &packet) {
                                Ok(_) => {
                                    let mut buf = [0u8; 256];
                                    match device.read_timeout(&mut buf, 500) {
                                        Ok(len) if len >= 8 && is_valid_response(&buf, len, GET_BATTERY_CMD_ID) => {
                                            let mut hx = HyperXDevice::new();
                                            hx.device = Some(device);
                                            hx.state.connected = true;
                                            hx.state.device_id = info.path().to_string_lossy().to_string();
                                            hx.state.name = info.product_string().map(|s| s.to_string()).unwrap_or_else(|| "HyperX Headset".to_string());
                                            hx.state.battery_percent = buf[7].min(100);
                                            log::info!("[HID] scan: connected battery={}%", hx.state.battery_percent);
                                            new_devices.push(hx);
                                        }
                                        Ok(len) => log::info!("[HID] scan: probe got {} bytes, not valid response: {:?}", len, &buf[0..8.min(len)]),
                                        Err(e) => log::info!("[HID] scan: probe read failed: {}", e),
                                    }
                                }
                                Err(e) => log::info!("[HID] scan: probe write failed: {}", e),
                            }
                        }
                        Err(e) => log::info!("[HID] scan: open_path failed: {}", e),
                    }
                }
            }

            log::info!("[HID] scan: {} candidates found, {} connected", candidates_seen, new_devices.len());
            self.devices = new_devices;
        }

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
