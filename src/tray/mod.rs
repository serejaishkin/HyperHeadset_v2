//! System tray integration
//!
//! Windows: tray-icon + winit
//! Linux: ksni
//! macOS: tray-icon + winit

use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ShowWindow,
    ToggleMute,
    RefreshBattery,
    Quit,
}

pub trait TrayBackend {
    fn run(self, tx: Sender<TrayCommand>);
    fn update_battery(&self, percent: u8);
    fn update_mute(&self, muted: bool);
}

// ===== Windows tray implementation =====
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "windows")]
pub use windows::WindowsTray as PlatformTray;

// ===== Linux tray implementation =====
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub use linux::LinuxTray as PlatformTray;

// ===== macOS tray implementation =====
#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOSTray as PlatformTray;
