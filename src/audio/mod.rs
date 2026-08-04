pub mod debounce;
pub use debounce::DebouncedEQ;
pub mod windows;
pub mod linux;
pub mod macos_eqmac;
#[cfg(feature = "embedded-voice")]
pub mod voice;
