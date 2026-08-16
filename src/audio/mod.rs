pub mod debounce;
pub mod voice;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos_eqmac;
#[cfg(target_os = "windows")]
pub mod windows;