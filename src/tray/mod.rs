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

#[cfg(target_os = "windows")]
pub type PlatformTray = windows::WindowsTray;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub type PlatformTray = linux::LinuxTray;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub type PlatformTray = macos::MacosTray;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub struct PlatformTray;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
impl PlatformTray {
    pub fn new(_tx: std::sync::mpsc::Sender<TrayCommand>) -> Self { Self }
    pub fn poll(&mut self) {}
    pub fn update_battery(&mut self, _percent: u8, _charging: bool) {}
    pub fn update_mute(&mut self, _muted: bool) {}
    pub fn update_icon_config(&mut self, _config: crate::tray::icon::TrayIconConfig) {}
}
