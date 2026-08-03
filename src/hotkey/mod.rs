use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub struct KeyCombo {
    pub keys: Vec<String>,
    pub display: String,
}

pub struct GlobalHotkeyCapture {
    pub recording: Arc<Mutex<bool>>,
    pub result: Arc<Mutex<Option<KeyCombo>>>,
    pub start_time: Arc<Mutex<Option<Instant>>>,
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
        *self.recording.lock().unwrap() = true;
        *self.result.lock().unwrap() = None;
        *self.start_time.lock().unwrap() = Some(Instant::now());
    }

    pub fn is_recording(&self) -> bool {
        *self.recording.lock().unwrap()
    }

    pub fn poll_result(&self) -> Option<KeyCombo> {
        self.result.lock().unwrap().take()
    }

    pub fn cancel(&self) {
        *self.recording.lock().unwrap() = false;
        *self.result.lock().unwrap() = None;
    }

    pub fn stop_recording(&self) {
        *self.recording.lock().unwrap() = false;
    }
}

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

pub fn spawn_capture(capture: Arc<GlobalHotkeyCapture>) {
    #[cfg(target_os = "windows")]
    windows::spawn_capture_thread(capture);
    #[cfg(target_os = "linux")]
    linux::spawn_capture_thread(capture);
    #[cfg(target_os = "macos")]
    macos::spawn_capture_thread(capture);
}