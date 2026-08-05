pub mod config;
pub mod gui;
pub mod discord;
pub mod audio;
pub mod notifications;
#[cfg(target_os = "linux")]
pub mod bluetooth;
#[cfg(target_os = "linux")]
mod airoha_race;
pub mod device;
pub mod devices;
pub mod input;
pub mod tray;
pub mod hotkey;
pub mod dialog;
pub mod platform;

#[macro_export]
macro_rules! debug_println {
    ($($args:tt)*) => {
        if *$crate::VERBOSE.get().unwrap_or(&false) {
            println!($($args)*);
        }
    };
}

#[derive(Debug, Clone)]
pub enum DeviceEvent {
    StateChanged(crate::device::DeviceState),
    Connected,
    Disconnected,
    BatteryLow(u8),
}

#[derive(Debug, Clone)]
pub enum DeviceCommand {
    ToggleMute,
    SetSidetone(bool),
    SetVoicePrompts(bool),
}

use std::sync::OnceLock;

pub static VERBOSE: OnceLock<bool> = OnceLock::new();

pub static DEBUG_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn dlog(msg: &str) {
    log::info!("{}", msg);
    if DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed) {
        let line = format!("[DEBUG] {}\n", msg);
        let path = std::env::temp_dir().join("hyperx-debug.log");
        let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    }
}

#[macro_export]
macro_rules! debug_print {
    ($($args:tt)*) => {
        if *$crate::VERBOSE.get().unwrap_or(&false) {
            println!($($args)*);
        }
    };
}