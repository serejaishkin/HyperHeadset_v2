use std::sync::Arc;

pub struct KeyCombo {
    pub display: String,
}

pub struct GlobalHotkeyCapture;

impl GlobalHotkeyCapture {
    pub fn new() -> Self {
        Self
    }
    pub fn start_recording(&self) {}
    pub fn cancel(&self) {}
    pub fn poll_result(&self) -> Option<KeyCombo> {
        None
    }
}

pub fn spawn_capture(_capture: Arc<GlobalHotkeyCapture>) {}
