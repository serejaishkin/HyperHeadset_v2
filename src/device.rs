//! HID device abstraction for HyperX Cloud II Wireless
//! Based on HyperHeadset reverse-engineered protocol.

use hidapi::HidApi;

#[derive(Debug, Clone, Default)]
pub struct DeviceState {
    pub connected: bool,
    pub battery_percent: u8,
    pub charging: bool,
    pub muted: bool,
    pub signal_dbm: i8,
    pub sidetone: bool,
    pub voice_prompts: bool,
}

pub struct HyperXDevice {
    device: Option<hidapi::HidDevice>,
    pub state: DeviceState,
}

// Vendor/Product IDs for HyperX Cloud II Wireless
const VENDOR_IDS: &[u16] = &[0x03f0, 0x0951, 0x03f0]; // HP, Kingston, HyperX
const PRODUCT_IDS: &[u16] = &[0x018b, 0x018c, 0x018d]; // Various revisions

impl HyperXDevice {
    pub fn new() -> Self {
        Self {
            device: None,
            state: DeviceState::default(),
        }
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let api = HidApi::new()?;
        for (vid, pid) in VENDOR_IDS.iter().zip(PRODUCT_IDS.iter()) {
            if let Ok(device) = api.open(*vid, *pid) {
                self.device = Some(device);
                self.state.connected = true;
                self.refresh_state()?;
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("No HyperX Cloud II Wireless found"))
    }

    pub fn disconnect(&mut self) {
        self.device = None;
        self.state.connected = false;
        self.state.battery_percent = 0;
        self.state.charging = false;
        self.state.muted = false;
    }

    pub fn refresh_state(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        let mut buf = [0u8; 64];

        let cmd = [0x21, 0xBB, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00];
        device.write(&cmd)?;
        let len = device.read_timeout(&mut buf, 1000)?;
        if len > 0 && buf[0] == 0x21 && buf[1] == 0xBB && buf[2] == 0x0b {
            self.state.battery_percent = buf[3];
            self.state.charging = len > 4 && buf[4] & 0x01 != 0;
        }

        let cmd = [0x21, 0xBB, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00];
        device.write(&cmd)?;
        let len = device.read_timeout(&mut buf, 1000)?;
        if len > 0 && buf[2] == 0x23 {
            self.state.muted = buf[3] == 0x01;
        }
        Ok(())
    }

    pub fn toggle_mute(&mut self) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        let cmd = [0x21, 0xBB, 0x24, 0x01, 0x00, 0x00, 0x00, 0x00];
        device.write(&cmd)?;
        self.state.muted = !self.state.muted;
        Ok(())
    }

    pub fn set_sidetone(&mut self, enabled: bool) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        let cmd = [0x21, 0xBB, 0x10, enabled as u8, 0x00, 0x00, 0x00, 0x00];
        device.write(&cmd)?;
        self.state.sidetone = enabled;
        Ok(())
    }

    pub fn set_voice_prompts(&mut self, enabled: bool) -> anyhow::Result<()> {
        let Some(device) = self.device.as_ref() else {
            return Err(anyhow::anyhow!("Device not connected"));
        };
        let cmd = [0x21, 0xBB, 0x12, enabled as u8, 0x00, 0x00, 0x00, 0x00];
        device.write(&cmd)?;
        self.state.voice_prompts = enabled;
        Ok(())
    }
}
