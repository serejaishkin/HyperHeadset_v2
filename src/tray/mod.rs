#[cfg(not(target_os = "linux"))]
mod windows;
#[cfg(not(target_os = "linux"))]
pub use windows::WindowsTray as PlatformTray;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxTray as PlatformTray;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ShowWindow,
    ToggleMute,
    Quit,
    RefreshBattery,
}