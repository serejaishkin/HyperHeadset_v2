use hyperx_ngenuity_open::device::DeviceState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayBatteryIconState {
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
            return Self::Disconnected;
        };
        if !state.connected {
            return Self::Disconnected;
        }
        if state.battery_percent == 0 {
            return Self::ConnectedUnknown;
        }
        Self::Connected {
            percent: state.battery_percent.min(100),
            charging: state.charging,
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
