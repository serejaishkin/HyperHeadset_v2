//! Global hotkey capture for recording key combinations in GUI
//!
//! Usage:
//!   let mut capture = GlobalHotkeyCapture::new();
//!   capture.start_recording();
//!   // User presses Ctrl+Shift+M
//!   if let Some(keys) = capture.poll_result() {
//!       println!("Captured: {:?}", keys);
//!   }

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct KeyCombo {
    pub keys: Vec<String>,
    pub display: String,
}

pub struct GlobalHotkeyCapture {
    recording: Arc<Mutex<bool>>,
    result: Arc<Mutex<Option<KeyCombo>>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl GlobalHotkeyCapture {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(Mutex::new(false)),
            result: Arc::new(Mutex::new(None)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start_recording(&self) {
        *self.recording.lock() = true;
        *self.result.lock() = None;
        *self.start_time.lock() = Some(Instant::now());
    }

    pub fn is_recording(&self) -> bool {
        *self.recording.lock()
    }

    pub fn poll_result(&self) -> Option<KeyCombo> {
        self.result.lock().clone()
    }

    pub fn cancel(&self) {
        *self.recording.lock() = false;
        *self.result.lock() = None;
    }
}

// Platform-specific implementations
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;
