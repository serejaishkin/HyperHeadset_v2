//! macOS global hotkey capture placeholder
//!
//! For full implementation, use:
//! - Carbon RegisterEventHotKey
//! - Cocoa NSEvent addGlobalMonitorForEventsMatchingMask

use super::GlobalHotkeyCapture;
use std::sync::Arc;

pub fn spawn_capture_thread(_capture: Arc<GlobalHotkeyCapture>) {
    // TODO: Implement using Carbon or Cocoa
    log::warn!("Global hotkey capture not yet implemented on macOS");
}
