use crate::device::DeviceState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayBatteryIconState {
    NoDevice,
    Disconnected,
    ConnectedUnknown,
    Connected { percent: u8, charging: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowsIconKey {
    pub percent: u8,
    pub charging: bool,
}

impl TrayBatteryIconState {
    pub fn from_device_state(state: Option<&DeviceState>) -> Self {
        let Some(state) = state else {
            return Self::NoDevice;
        };
        if !state.connected {
            return Self::Disconnected;
        }
        let charging = state.charging;
        if state.battery_percent == 0 {
            return Self::ConnectedUnknown;
        }
        Self::Connected {
            percent: state.battery_percent.min(100),
            charging,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn windows_icon_key(self) -> Option<WindowsIconKey> {
        match self {
            Self::Connected { percent, charging } => Some(WindowsIconKey { percent, charging }),
            _ => None,
        }
    }
}
