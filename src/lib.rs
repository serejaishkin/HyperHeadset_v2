pub mod device;
pub mod devices;
pub mod input;
pub mod tray;
pub mod config;
pub mod audio;
pub mod discord;
pub mod platform;
pub mod i18n;
pub mod gui;
pub mod notifications;
pub mod dialog;
pub mod hotkey;

pub use device::{DeviceCommand, DeviceEvent};

#[macro_export]
macro_rules! debug_println {
    ($($args:tt)*) => {
        if *$crate::VERBOSE.get().unwrap_or(&false) {
            println!($($args)*);
        }
    };
}

use std::sync::OnceLock;
pub static VERBOSE: OnceLock<bool> = OnceLock::new();
