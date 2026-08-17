#[cfg(not(target_os = "linux"))]
pub mod icon;
#[cfg(not(target_os = "linux"))]
pub mod windows;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ShowWindow,
    ToggleMute,
    Quit,
    RefreshBattery,
}

pub struct PlatformTray;

impl PlatformTray {
    pub fn new(_tx: std::sync::mpsc::Sender<TrayCommand>) -> Self {
        Self
    }
    pub fn poll(&mut self) {}
    pub fn update_battery(&mut self, _percent: u8, _charging: bool) {}
    pub fn update_mute(&mut self, _muted: bool) {}
    pub fn update_icon_config(&mut self, _config: crate::tray::icon::TrayIconConfig) {}
}
