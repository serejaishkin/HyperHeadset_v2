#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

pub mod icon;

#[cfg(target_os = "windows")]
pub use windows::WindowsTray as PlatformTray;
#[cfg(target_os = "macos")]
pub use macos::MacOSTray as PlatformTray;
#[cfg(target_os = "linux")]
pub use linux::LinuxTray as PlatformTray;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ShowWindow,
    ToggleMute,
    Quit,
    RefreshBattery,
}