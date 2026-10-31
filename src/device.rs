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
    SetAutoShutdown(u32),
}

pub struct HyperXDevice {
    device: Option<hidapi::HidDevice>,
    pub state: DeviceState,
    last_presence_check: Option<std::time::Instant>,
    last_sidetone_set: Option<std::time::Instant>,
    /// Number of sidetone toggles initiated on the headset itself (not by this app).
    pub sidetone_button_events: u32,
    /// True once the headset has sent an unsolicited mute event; until then mute is polled.
    pub mute_events_seen: bool,
    /// False while the headset itself is asleep/off (dongle still enumerated).
    pub headset_online: bool,
    /// Drivers from `devices/` for headsets that do not speak the Cloud II DTS protocol
    /// (Cloud III / III S / II Core / II Wireless / Alpha / Flight Wireless).
    legacy: Option<crate::devices::Headset>,
}

const VENDOR_ID: u16 = 0x03f0;
const PRODUCT_IDS: &[u16] = &[0x018b, 0x0d93, 0x0696, 0x1718, 0x058b, 0x038b];

const BASE_PACKET: [u8; 20] = [
    0x06, 0xff, 0xbb, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

const GET_BATTERY_CMD_ID: u8 = 2;
const GET_MUTE_CMD_ID: u8 = 5;
const SET_MUTE_CMD_ID: u8 = 32;
const GET_SIDE_TONE_CMD_ID: u8 = 6;
const SET_SIDE_TONE_CMD_ID: u8 = 33;
const GET_CHARGING_CMD_ID: u8 = 3;
const SET_AUTO_SHUTDOWN_CMD_ID: u8 = 34;
const GET_WIRELESS_STATUS_CMD_ID: u8 = 1;
// Extended (0x21 0xBB <cmd> <val>) commands used by NGenuity for voice prompts.
const GET_VOICE_PROMPT_CMD_ID: u8 = 9;
const SET_VOICE_PROMPT_CMD_ID: u8 = 19;

const RESPONSE_DELAY: Duration = Duration::from_millis(100);

impl HyperXDevice {
    pub fn new() -> Self {
        Self { device: None, state: DeviceState::default(), last_presence_check: None, last_sidetone_set: None, sidetone_button_events: 0, mute_events_seen: false, headset_online: true, legacy: None }
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
        match self.connect_dts() {
            Ok(()) => Ok(()),
            Err(e) => {
                // Only fall back to the other drivers when no Cloud II DTS-family dongle is present.
                if !e.to_string().starts_with("No HyperX HID device enumerated") {
                    return Err(e);
                }
                self.connect_legacy().map_err(|le| {
                    log::debug!("[HID] legacy connect failed: {}", le);
                    e
                })
            }
        }
    }

    fn connect_legacy(&mut self) -> anyhow::Result<()> {
        let mut headset = crate::devices::connect_compatible_device()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        headset
            .active_refresh_state()
            .map_err(|e| anyhow::anyhow!("initial refresh: {:?}", e))?;
        let props = headset.device_properties();
        log::info!(
            "[HID] Connected via legacy driver: {:?} {:04X}:{:04X}",
            props.device_name, props.vendor_id, props.product_id
        );
        self.legacy = Some(headset);
        self.state.connected = true;
        self.headset_online = true;
        self.sync_from_legacy();
        Ok(())
    }

    /// Copy the legacy driver's cached properties into `state`. Returns true if anything changed.
    fn sync_from_legacy(&mut self) -> bool {
        use crate::devices::ChargingStatus;
        let Some(h) = self.legacy.as_ref() else { return false };
        let p = h.device_properties();
        let before = (
            self.state.battery_percent, self.state.charging, self.state.muted,
            self.state.sidetone, self.state.voice_prompts, self.headset_online,
        );
        if let Some(b) = p.battery_level { if b > 0 { self.state.battery_percent = b.min(100); } }
        if let Some(c) = p.charging {
            self.state.charging = matches!(c, ChargingStatus::Charging | ChargingStatus::FullyCharged);
        }
        if let Some(m) = p.muted { self.state.muted = m; }
        if let Some(s) = p.side_tone_on { self.state.sidetone = s; }
        if let Some(v) = p.voice_prompt_on { self.state.voice_prompts = v; }
        if let Some(n) = p.device_name.clone() { self.state.name = n; }
        self.state.device_id = format!("{:04X}:{:04X}", p.vendor_id, p.product_id);
        if let Some(c) = p.connected { self.headset_online = c; }
        before != (
            self.state.battery_percent, self.state.charging, self.state.muted,
            self.state.sidetone, self.state.voice_prompts, self.headset_online,
        )
    }

    fn legacy_apply(&mut self, event: crate::devices::DeviceEvent) -> anyhow::Result<()> {
        let Some(h) = self.legacy.as_mut() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };
        h.try_apply(event).map_err(|e| anyhow::anyhow!("{} — disconnected?", e))?;
        self.sync_from_legacy();
        Ok(())
    }

    fn connect_dts(&mut self) -> anyhow::Result<()> {
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

        // NGenuity talks to these headsets on HID MI_03 (Col03 for the 018B/0696 dongles),
        // so probe that interface first; MI_00 is the USB audio function.
        candidates.sort_by_key(|i| match i.interface_number() {
            3 => 0,
            -1 => 1,
            _ => 2,
        });

        for info in &candidates {
            match api.open_path(info.path()) {
                Ok(device) => {
                    let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
                    match write_hid_report(&device, &packet) {
                        Ok(_) => {
                            let mut buf = [0u8; 256];
                            match device.read_timeout(&mut buf, 2000) {
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
                Err(e) => {
                    let err_msg = e.to_string();
                    // Check if device is in use by another application (e.g., NGENUITY)
                    if err_msg.contains("access") || err_msg.contains("busy") || err_msg.contains("permission") {
                        log::warn!("[HID] Device appears to be in use by another application: {}", err_msg);
                        log::warn!("[HID] Close NGENUITY or other HyperX software to allow HyperHeadsetv2 to access the device");
                    } else {
                        log::debug!("[HID] open_path failed: {}", e);
                    }
                }
            }
        }

        Err(anyhow::anyhow!("HyperX HID enumerated, but no compatible HID interface answered"))
    }

    pub fn disconnect(&mut self) {
        self.device = None;
        self.legacy = None;
        self.state = DeviceState::default();
    }

    pub fn refresh_state(&mut self) -> anyhow::Result<()> {
        if let Some(h) = self.legacy.as_mut() {
            h.active_refresh_state().map_err(|e| anyhow::anyhow!("Device read failed — disconnected: {:?}", e))?;
            self.sync_from_legacy();
            return Ok(());
        }
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };

        // Quick presence check: if the device is no longer enumerated by the OS,
        // report disconnect immediately instead of waiting for 4 slow read timeouts.
        {
            let api = match HidApi::new() {
                Ok(a) => a,
                Err(_) => return Err(anyhow::anyhow!("HID API unavailable")),
            };
            let present = api.device_list().any(|info| {
                info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id())
            });
            if !present {
                log::warn!("[HID] Device {:#06X}:{:#06X} no longer enumerated", VENDOR_ID, PRODUCT_IDS[0]);
                return Err(anyhow::anyhow!("Device no longer enumerated — disconnected"));
            }
        }

        log::debug!("[HID] Starting refresh_state for device {}", self.state.device_id);
        Self::flush_input_buffer(device);

        // Read battery — if this fails completely, device is disconnected.
        // Use short 500ms timeout for fast disconnect detection.
        self.prepare_write();
        let packet = build_packet(GET_BATTERY_CMD_ID, &[]);
        if write_hid_report(device, &packet).is_err() {
            return Err(anyhow::anyhow!("Device write failed — disconnected"));
        }
        thread::sleep(RESPONSE_DELAY);
        let mut buf = [0u8; 256];
        let len = match device.read_timeout(&mut buf, 500) {
            Ok(l) => l,
            Err(_) => return Err(anyhow::anyhow!("Device read failed — disconnected")),
        };
        if len < 5 || !is_valid_response(&buf, len, GET_BATTERY_CMD_ID) {
            return Err(anyhow::anyhow!("Invalid battery response — disconnected"));
        }
        self.state.battery_percent = buf.get(7).copied().unwrap_or(0).min(100);
        log::debug!("[HID] Battery level: {}% (raw={:02X?})", self.state.battery_percent, &buf[0..16.min(len)]);

        // Read mute — try feature report approach (some HID devices only respond to feature reports).
        // Log all raw bytes for debugging.
        let mute_packet = build_packet(GET_MUTE_CMD_ID, &[]);
        let mute_read_ok = device.send_feature_report(&mute_packet).is_ok()
            || device.write(&mute_packet).is_ok();
        
        if mute_read_ok {
            thread::sleep(RESPONSE_DELAY);
            // Try reading as input report first
            let mut buf = [0u8; 256];
            let mut got_response = false;
            
            if let Ok(len) = device.read_timeout(&mut buf, 1000) {
                log::debug!("[HID] Mute input report: len={} bytes={:02X?}", len, &buf[0..16.min(len)]);
                if len >= 5 && is_valid_response(&buf, len, GET_MUTE_CMD_ID) {
                    got_response = true;
                    self.state.muted = scan_mute_bytes(&buf, len);
                    log::info!("[HID] Mute status (input): {} (raw={:02X?})", self.state.muted, &buf[0..16.min(len)]);
                }
            }
            
            // If input report didn't work, try feature report read
            if !got_response {
                let mut feat_buf = [0u8; 256];
                feat_buf[0] = GET_MUTE_CMD_ID;
                if let Ok(len) = device.get_input_report(&mut feat_buf) {
                    log::debug!("[HID] Mute feature report: len={} bytes={:02X?}", len, &feat_buf[0..16.min(len)]);
                    if len >= 5 && is_valid_response(&feat_buf, len, GET_MUTE_CMD_ID) {
                        self.state.muted = scan_mute_bytes(&feat_buf, len);
                        log::info!("[HID] Mute status (feature): {} (raw={:02X?})", self.state.muted, &feat_buf[0..16.min(len)]);
                    }
                }
            }
        }

        // Read charging (non-fatal)
        self.prepare_write();
        match send_and_read_with_raw(device, GET_CHARGING_CMD_ID, &[]) {
            Ok((status, _)) => {
                self.state.charging = status == 1;
                log::debug!("[HID] Charging status: {}", self.state.charging);
            }
            Err(e) => log::debug!("[HID] Charging telemetry unavailable: {}", e),
        }

        // Read sidetone (non-fatal) — status byte at buf[4]
        self.prepare_write();
        match send_and_read(device, GET_SIDE_TONE_CMD_ID, &[]) {
            Ok(status) => {
                self.state.sidetone = status == 1;
                log::debug!("[HID] Sidetone status: {}", self.state.sidetone);
            }
            Err(e) => log::debug!("[HID] Sidetone telemetry unavailable: {}", e),
        }

        log::debug!("[HID] refresh_state completed successfully");
        Ok(())
    }

    /// Passive listen: no HID writes, so the headset is not kept busy. The headset reports
    /// button presses / state changes on its own (mute 0x20, sidetone 0x21, charging 0x03,
    /// battery 0x02), exactly as NGenuity consumes them in OnDeviceInputReportReceived.
    /// Returns Ok(true) if state changed. Err = device gone.
    pub fn poll_events(&mut self, timeout_ms: i32) -> anyhow::Result<bool> {
        if self.legacy.is_some() {
            return self.poll_events_legacy(timeout_ms);
        }
        // Enumeration is relatively expensive, so check presence only every few seconds.
        let due = self
            .last_presence_check
            .map_or(true, |t| t.elapsed() >= Duration::from_secs(5));
        if due {
            self.last_presence_check = Some(std::time::Instant::now());
            let api = HidApi::new().map_err(|_| anyhow::anyhow!("HID API unavailable"))?;
            let present = api.device_list().any(|info| {
                info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&info.product_id())
            });
            if !present {
                return Err(anyhow::anyhow!("Device no longer enumerated — disconnected"));
            }
        }

        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };

        let mut changed = false;
        let mut buf = [0u8; 256];
        // Drain everything that is queued, then return.
        let mut wait = timeout_ms;
        loop {
            let len = match device.read_timeout(&mut buf, wait) {
                Ok(0) => break,
                Ok(l) => l,
                Err(e) => return Err(anyhow::anyhow!("Device read failed — disconnected: {}", e)),
            };
            wait = 0;
            if len < 5 || buf[0] != 0x06 || buf[1] != 0xFF || buf[2] != 0xBB {
                continue;
            }
            log::debug!("[HID] Event: {:02X?}", &buf[0..16.min(len)]);
            match buf[3] {
                SET_MUTE_CMD_ID => {
                    let muted = buf[4] == 1;
                    self.mute_events_seen = true;
                    log::info!("[HID] Mute event from headset: muted={}", muted);
                    if self.state.muted != muted {
                        self.state.muted = muted;
                        changed = true;
                    }
                }
                SET_SIDE_TONE_CMD_ID => {
                    let on = buf[4] == 1;
                    let ours = self
                        .last_sidetone_set
                        .map_or(false, |t| t.elapsed() < Duration::from_millis(1500));
                    if !ours {
                        self.sidetone_button_events = self.sidetone_button_events.wrapping_add(1);
                        log::info!("[HID] Sidetone event from headset: on={}", on);
                    }
                    if self.state.sidetone != on {
                        self.state.sidetone = on;
                        changed = true;
                    }
                }
                GET_CHARGING_CMD_ID => {
                    let ch = buf[4] == 1;
                    if self.state.charging != ch {
                        self.state.charging = ch;
                        changed = true;
                    }
                }
                GET_BATTERY_CMD_ID if len > 7 => {
                    let pct = buf[7].min(100);
                    if pct > 0 && self.state.battery_percent != pct {
                        self.state.battery_percent = pct;
                        changed = true;
                    }
                }
                GET_WIRELESS_STATUS_CMD_ID => {
                    // 1 or 4 = headset connected to dongle; anything else = headset asleep/off.
                    // Dongle stays enumerated, so surface this as a state change only.
                    let online = buf[4] == 1 || buf[4] == 4;
                    log::debug!("[HID] Wireless status: {}", buf[4]);
                    if online != self.headset_online {
                        log::info!("[HID] Headset online: {} (status {})", online, buf[4]);
                        self.headset_online = online;
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        Ok(changed)
    }

    /// State for the UI: `connected` is false while the headset itself is off.
    pub fn ui_state(&self) -> DeviceState {
        let mut s = self.state.clone();
        s.connected = s.connected && self.headset_online;
        s
    }

    fn poll_events_legacy(&mut self, timeout_ms: i32) -> anyhow::Result<bool> {
        use crate::devices::DeviceEvent as E;
        let Some(h) = self.legacy.as_mut() else { return Ok(false) };
        let events = h.listen_events(Duration::from_millis(timeout_ms.max(1) as u64));
        let mut changed = false;
        if let Some(events) = events {
            for ev in events {
                match ev {
                    E::SideToneOn(_) => {
                        let ours = self
                            .last_sidetone_set
                            .map_or(false, |t| t.elapsed() < Duration::from_millis(1500));
                        if !ours {
                            self.sidetone_button_events = self.sidetone_button_events.wrapping_add(1);
                            log::info!("[HID] Sidetone event from headset (legacy)");
                        }
                    }
                    E::Muted(m) => {
                        self.mute_events_seen = true;
                        log::info!("[HID] Mute event from headset (legacy): muted={}", m);
                    }
                    _ => {}
                }
            }
            changed = self.sync_from_legacy();
        } else {
            // Nothing arrived: make sure the dongle is still there.
            let due = self.last_presence_check.map_or(true, |t| t.elapsed() >= Duration::from_secs(5));
            if due {
                self.last_presence_check = Some(std::time::Instant::now());
                let api = HidApi::new().map_err(|_| anyhow::anyhow!("HID API unavailable"))?;
                let name = self.state.device_id.clone();
                let present = api.device_list().any(|i| format!("{:04X}:{:04X}", i.vendor_id(), i.product_id()) == name);
                if !present {
                    return Err(anyhow::anyhow!("Device no longer enumerated — disconnected"));
                }
            }
        }
        Ok(changed)
    }

    /// Ask the dongle whether the headset is on; the reply is handled by `poll_events`.
    pub fn request_wireless_status(&mut self) -> anyhow::Result<bool> {
        if self.legacy.is_some() {
            return Ok(false);
        }
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };
        Self::flush_input_buffer(device);
        let packet = build_packet(GET_WIRELESS_STATUS_CMD_ID, &[]);
        if write_hid_report(device, &packet).is_err() {
            return Err(anyhow::anyhow!("Device write failed — disconnected"));
        }
        self.poll_events(300)
    }

    /// Lightweight mute-only poll, used until the headset proves it sends mute events itself.
    /// Returns Ok(true) if the mute state changed.
    pub fn poll_mute(&mut self) -> anyhow::Result<bool> {
        if self.legacy.is_some() {
            return Ok(false);
        }
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };
        Self::flush_input_buffer(device);
        let packet = build_packet(GET_MUTE_CMD_ID, &[]);
        if write_hid_report(device, &packet).is_err() {
            return Err(anyhow::anyhow!("Device write failed — disconnected"));
        }
        thread::sleep(Duration::from_millis(60));
        let mut buf = [0u8; 256];
        let len = device
            .read_timeout(&mut buf, 300)
            .map_err(|e| anyhow::anyhow!("Device read failed — disconnected: {}", e))?;
        if len >= 5 && is_valid_response(&buf, len, GET_MUTE_CMD_ID) {
            let muted = scan_mute_bytes(&buf, len);
            if muted != self.state.muted {
                log::info!("[HID] Mute changed (poll): {}", muted);
                self.state.muted = muted;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Discard any stale replies so the next request/response pair is not mismatched.
    fn drain(&self) {
        if let Some(device) = self.device.as_ref() {
            Self::flush_input_buffer(device);
        }
    }

    pub fn toggle_mute(&mut self) -> anyhow::Result<()> {
        if self.legacy.is_some() {
            let target = !self.state.muted;
            self.legacy_apply(crate::devices::DeviceEvent::Muted(target))?;
            self.state.muted = target;
            return Ok(());
        }
        let Some(device) = self.device.as_ref() else { return Err(anyhow::anyhow!("Device not connected")); };
        let new_mute = !self.state.muted;
        log::info!("[HID] Toggling mute: {} -> {}", self.state.muted, new_mute);
        self.drain();
        self.prepare_write();
        let packet = build_packet(SET_MUTE_CMD_ID, &[new_mute as u8]);
        write_hid_report(device, &packet).map_err(|e| anyhow::anyhow!("mute command failed: {}", e))?;
        self.state.muted = new_mute;
        log::info!("[HID] Mute toggled successfully to: {}", new_mute);
        Ok(())
    }

    pub fn set_sidetone(&mut self, enabled: bool) -> anyhow::Result<()> {
        if self.legacy.is_some() {
            self.last_sidetone_set = Some(std::time::Instant::now());
            self.legacy_apply(crate::devices::DeviceEvent::SideToneOn(enabled))?;
            self.state.sidetone = enabled;
            return Ok(());
        }
        let Some(device) = self.device.as_ref() else { return Err(anyhow::anyhow!("Device not connected")); };
        self.drain();
        self.prepare_write();
        let packet = build_packet(SET_SIDE_TONE_CMD_ID, &[enabled as u8]);
        self.last_sidetone_set = Some(std::time::Instant::now());
        write_hid_report(device, &packet).map_err(|e| anyhow::anyhow!("sidetone command failed: {}", e))?;
        thread::sleep(RESPONSE_DELAY);

        // Consume the ack and read the real state back instead of assuming success.
        // NGenuity does the same (SetOutputReport followed by GetInputReport).
        Self::flush_input_buffer(device);
        match send_and_read(device, GET_SIDE_TONE_CMD_ID, &[]) {
            Ok(status) => {
                let actual = status == 1;
                if actual != enabled {
                    log::warn!("[HID] Sidetone requested={} but headset reports {}", enabled, actual);
                }
                self.state.sidetone = actual;
            }
            Err(e) => {
                log::warn!("[HID] Sidetone readback failed ({}), assuming requested value", e);
                self.state.sidetone = enabled;
            }
        }
        Ok(())
    }

    pub fn set_voice_prompts(&mut self, enabled: bool) -> anyhow::Result<()> {
        if self.legacy.is_some() {
            self.legacy_apply(crate::devices::DeviceEvent::VoicePrompt(enabled))?;
            self.state.voice_prompts = enabled;
            return Ok(());
        }
        let Some(device) = self.device.as_ref() else { return Err(anyhow::anyhow!("Device not connected")); };
        self.drain();
        // NGenuity (CMDHSCloudIIWirelessDTSSetVoicePromptStatus): [0]=0x21 [1]=0xBB [2]=0x13 [3]=on
        let mut packet = [0u8; 20];
        packet[0] = 0x21;
        packet[1] = 0xBB;
        packet[2] = SET_VOICE_PROMPT_CMD_ID;
        packet[3] = enabled as u8;
        write_hid_report(device, &packet).map_err(|e| anyhow::anyhow!("voice prompt command failed: {}", e))?;
        thread::sleep(RESPONSE_DELAY);
        Self::flush_input_buffer(device);
        self.state.voice_prompts = enabled;
        let _ = GET_VOICE_PROMPT_CMD_ID;
        Ok(())
    }

    pub fn set_auto_shutdown(&mut self, minutes: u32) -> anyhow::Result<()> {
        if self.legacy.is_some() {
            return self.legacy_apply(crate::devices::DeviceEvent::AutomaticShutdownAfter(
                Duration::from_secs(minutes as u64 * 60),
            ));
        }
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device handle is not connected"));
        };
        self.drain();
        self.prepare_write();

        let packet = build_packet(SET_AUTO_SHUTDOWN_CMD_ID, &[minutes.min(255) as u8]);
        write_hid_report(device, &packet)
            .map_err(|e| anyhow::anyhow!("auto shutdown write failed: {}", e))?;
        thread::sleep(RESPONSE_DELAY);

        let mut buf = [0u8; 256];
        let len = device.read_timeout(&mut buf, 2000)
            .map_err(|e| anyhow::anyhow!("auto shutdown read failed: {}", e))?;
        if len < 5 || !is_valid_response(&buf, len, SET_AUTO_SHUTDOWN_CMD_ID) {
            return Err(anyhow::anyhow!("invalid auto shutdown response: len={} raw={:02X?}", len, &buf[0..16.min(len)]));
        }
        log::info!("[HID] Auto-shutdown set to {} min (ack={:02X?})", minutes, &buf[0..8.min(len)]);
        Ok(())
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

/// Scan response data bytes (positions 4+) for mute status.
/// Returns true if any data byte equals 1 (muted), false if all are 0.
fn scan_mute_bytes(buf: &[u8], len: usize) -> bool {
    for i in 4..len.min(20) {
        let b = buf[i];
        if b == 1 {
            log::debug!("[HID] Mute byte[{}] = 1 (muted)", i);
            return true;
        } else if b != 0 {
            log::debug!("[HID] Mute byte[{}] = {:02X} (non-binary, skipping)", i, b);
        }
    }
    false
}

fn send_and_read(device: &hidapi::HidDevice, cmd_id: u8, data: &[u8]) -> Result<u8, String> {
    const MAX_RETRIES: u32 = 2;
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        let packet = build_packet(cmd_id, data);
        if let Err(e) = write_hid_report(device, &packet) {
            last_error = Some(format!("write failed: {}", e));
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        thread::sleep(RESPONSE_DELAY);
        let mut buf = [0u8; 256];
        match device.read_timeout(&mut buf, 2000) {
            Ok(len) if len >= 5 && is_valid_response(&buf, len, cmd_id) => return Ok(buf[4]),
            Ok(len) => {
                last_error = Some(format!("invalid response: len={} raw={:02X?}", len, &buf[0..8.min(len)]));
                log::debug!("[HID] Invalid response for cmd {} (attempt {}/{}): len={} raw={:02X?}", cmd_id, attempt + 1, MAX_RETRIES, len, &buf[0..8.min(len)]);
            }
            Err(e) => {
                last_error = Some(format!("read failed: {}", e));
                log::debug!("[HID] Read error for cmd {} (attempt {}/{}): {}", cmd_id, attempt + 1, MAX_RETRIES, e);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    Err(last_error.unwrap_or_else(|| "unknown error".to_string()))
}

fn send_and_read_with_raw(device: &hidapi::HidDevice, cmd_id: u8, data: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let packet = build_packet(cmd_id, data);
    write_hid_report(device, &packet).map_err(|e| format!("write failed: {}", e))?;
    thread::sleep(RESPONSE_DELAY);
    let mut buf = [0u8; 256];
    match device.read_timeout(&mut buf, 2000) {
        Ok(0) => Err("read timeout/empty".into()),
        Ok(len) if len >= 5 && is_valid_response(&buf, len, cmd_id) => Ok((buf[4], buf[..len].to_vec())),
        Ok(len) => Err(format!("invalid response: len={} raw={:02X?}", len, &buf[0..8.min(len)])),
        Err(e) => Err(format!("read error: {}", e)),
    }
}

fn write_hid_report(device: &hidapi::HidDevice, packet: &[u8]) -> anyhow::Result<()> {
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match device.write(packet) {
            Ok(_) => return Ok(()),
            Err(write_err) => {
                #[cfg(target_os = "windows")]
                {
                    if let hidapi::HidError::HidApiError { message } = &write_err {
                        // Windows-specific: Try send_feature_report for "Incorrect function" errors
                        if message.contains("Incorrect function") || message.contains("(0x00000001)") {
                            log::debug!("[HID] Write failed with 'Incorrect function', trying send_feature_report (attempt {}/{})", attempt + 1, MAX_RETRIES);
                            if device.send_feature_report(packet).is_ok() {
                                return Ok(());
                            }
                        }
                        // Retry on transient Windows errors
                        if message.contains("busy") || message.contains("access") || message.contains("timeout") {
                            log::debug!("[HID] Transient write error (attempt {}/{}): {}", attempt + 1, MAX_RETRIES, message);
                            last_error = Some(write_err);
                            thread::sleep(Duration::from_millis(50));
                            continue;
                        }
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    // Cross-platform retry for transient errors
                    if write_err.to_string().contains("busy") || write_err.to_string().contains("timeout") {
                        log::debug!("[HID] Transient write error (attempt {}/{}): {}", attempt + 1, MAX_RETRIES, write_err);
                        last_error = Some(write_err);
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                }
                last_error = Some(write_err);
            }
        }
    }

    Err(last_error.map(|e| anyhow::anyhow!("write failed after {} retries: {}", MAX_RETRIES, e)).unwrap_or_else(|| anyhow::anyhow!("write failed")))
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
                                match device.read_timeout(&mut buf, 2000) {
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
            if new_devices.is_empty() {
                anyhow::bail!("No HyperX device accessible (NGENUITY may be holding it)");
            }
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
                                    match device.read_timeout(&mut buf, 2000) {
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
