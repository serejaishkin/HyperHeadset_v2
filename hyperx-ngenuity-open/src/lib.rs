pub mod config;
pub mod gui;
pub mod discord;
pub mod audio;
pub mod device;
pub mod input;
pub mod tray;
pub mod hotkey;
pub mod dialog;

#[derive(Debug, Clone)]
pub enum DeviceEvent {
    StateChanged(crate::device::DeviceState),
    Connected,
    Disconnected,
    BatteryLow(u8),
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
