pub mod device;
pub mod devices;
pub mod input;
pub mod tray;
pub mod config;
pub mod audio;
pub mod system_audio;
pub mod discord;
pub mod platform;
pub mod i18n;
pub mod gui;
pub mod notifications;
pub mod dialog;
pub mod hotkey;
pub mod airoha_hid;
pub mod audio_switch;
pub mod audio_route;
pub mod logging;
pub mod single_instance;
#[cfg(target_os = "linux")]
pub mod airoha_race;
#[cfg(target_os = "linux")]
pub mod bluetooth;

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
