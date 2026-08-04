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

#[macro_export]
macro_rules! debug_print {
    ($($args:tt)*) => {
        if *$crate::VERBOSE.get().unwrap_or(&false) {
            println!($($args)*);
        }
    };
}