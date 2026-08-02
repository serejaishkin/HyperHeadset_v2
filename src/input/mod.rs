//! Smart mute button handler with multiple action modes
//!
//! Supports:
//! - Standard: always MicMute (F20)
//! - MediaPlayPause: always Play/Pause
//! - SmartDouble: single = MicMute, double-click = Play/Pause
//! - SmartHold: short press = Play/Pause, long press = MicMute

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::{Duration, Instant};
use parking_lot::Mutex;

const DOUBLE_CLICK_MS: u64 = 400;
const LONG_PRESS_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuteButtonMode {
    Standard,
    MediaPlayPause,
    SmartDouble,
    SmartHold,
}

impl Default for MuteButtonMode {
    fn default() -> Self { MuteButtonMode::SmartDouble }
}

pub struct MuteHandler {
    mode: Mutex<MuteButtonMode>,
    enigo: Mutex<Enigo>,
    last_toggle: Mutex<Option<Instant>>,
    press_start: Mutex<Option<Instant>>,
}

impl MuteHandler {
    pub fn new() -> Self {
        Self {
            mode: Mutex::new(MuteButtonMode::default()),
            enigo: Mutex::new(Enigo::new(&Settings::default()).unwrap()),
            last_toggle: Mutex::new(None),
            press_start: Mutex::new(None),
        }
    }

    pub fn set_mode(&self, mode: MuteButtonMode) {
        *self.mode.lock() = mode;
    }

    pub fn get_mode(&self) -> MuteButtonMode {
        *self.mode.lock()
    }

    // ===== Standard mode =====
    fn do_mute(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::F20, Direction::Click);
        log::info!("[MuteHandler] MicMute (F20)");
    }

    fn do_media_play_pause(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::MediaPlayPause, Direction::Click);
        log::info!("[MuteHandler] MediaPlayPause");
    }

    // ===== Called when mute state CHANGES (toggle event) =====
    pub fn on_mute_toggled(&self, _muted: bool) {
        match self.get_mode() {
            MuteButtonMode::Standard => self.do_mute(),
            MuteButtonMode::MediaPlayPause => self.do_media_play_pause(),
            MuteButtonMode::SmartDouble => self.handle_smart_double(),
            MuteButtonMode::SmartHold => {
                // In SmartHold we rely on button_down/up, not toggle
                // Fallback: treat as mute
                self.do_mute();
            }
        }
    }

    // ===== SmartDouble: detect double-click via toggle interval =====
    fn handle_smart_double(&self) {
        let now = Instant::now();
        let mut last = self.last_toggle.lock();

        if let Some(t) = *last {
            if now.duration_since(t) < Duration::from_millis(DOUBLE_CLICK_MS) {
                // Double click detected
                self.do_media_play_pause();
                *last = None;
                return;
            }
        }

        // Single click
        self.do_mute();
        *last = Some(now);
    }

    // ===== SmartHold: requires down/up events =====
    /// Call when HID reports "mute button pressed"
    pub fn on_button_down(&self) {
        if self.get_mode() == MuteButtonMode::SmartHold {
            *self.press_start.lock() = Some(Instant::now());
        }
    }

    /// Call when HID reports "mute button released"
    pub fn on_button_up(&self) {
        if self.get_mode() != MuteButtonMode::SmartHold {
            return;
        }

        let mut press = self.press_start.lock();
        if let Some(start) = *press {
            let held = start.elapsed();

            if held < Duration::from_millis(LONG_PRESS_MS) {
                // Short press -> Play/Pause
                self.do_media_play_pause();
            } else {
                // Long press -> MicMute
                self.do_mute();
            }
            *press = None;
        }
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_MUTE_HANDLER: MuteHandler = MuteHandler::new();
}
