pub mod device;
pub mod input;
pub mod tray;
pub mod config;
pub mod audio;
pub mod discord;
pub mod platform;
pub mod i18n;

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