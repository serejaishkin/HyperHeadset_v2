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
    keybind: Mutex<Option<String>>,
    last_hotkey: Mutex<Option<Instant>>,
}

impl MuteHandler {
    pub fn new() -> Self {
        Self {
            mode: Mutex::new(MuteButtonMode::default()),
            enigo: Mutex::new(Enigo::new(&Settings::default()).unwrap()),
            last_toggle: Mutex::new(None),
            press_start: Mutex::new(None),
            keybind: Mutex::new(None),
            last_hotkey: Mutex::new(None),
        }
    }

    pub fn set_mode(&self, mode: MuteButtonMode) {
        *self.mode.lock() = mode;
    }

    pub fn get_mode(&self) -> MuteButtonMode {
        *self.mode.lock()
    }

    pub fn set_keybind(&self, keybind: Option<String>) {
        *self.keybind.lock() = keybind;
    }

    // ===== Volume controls =====
    pub fn volume_up(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::VolumeUp, Direction::Click);
    }

    pub fn volume_down(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::VolumeDown, Direction::Click);
    }

    // ===== Media controls =====
    pub fn do_media_play_pause(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::MediaPlayPause, Direction::Click);
        log::info!("[MuteHandler] MediaPlayPause");
    }

    // ===== Debounced key sender =====
    fn do_mute(&self) {
        let mut last = self.last_hotkey.lock();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_millis(150) {
                return; // debounce
            }
        }
        *last = Some(Instant::now());
        drop(last);

        let mut enigo = self.enigo.lock();
        let keybind = self.keybind.lock().clone();

        let key = match keybind.as_deref() {
            Some("F13") => Key::F13,
            Some("F14") => Key::F14,
            Some("F15") => Key::F15,
            Some("F16") => Key::F16,
            Some("F17") => Key::F17,
            Some("F18") => Key::F18,
            Some("F19") => Key::F19,
            Some("F20") => Key::F20,
            Some("F21") => Key::F21,
            Some("F22") => Key::F22,
            Some("F23") => Key::F23,
            Some("F24") => Key::F24,
            Some("MediaPlayPause") => Key::MediaPlayPause,
            Some("MediaVolumeMute") | Some("Mute") => Key::VolumeMute,
            Some("MediaVolumeDown") => Key::VolumeDown,
            Some("MediaVolumeUp") => Key::VolumeUp,
            Some("MediaNextTrack") => Key::MediaNextTrack,
            Some("MediaPrevTrack") => Key::MediaPrevTrack,
            Some("MediaStop") => Key::MediaStop,
            Some("Numpad0") => Key::Numpad0,
            Some("Numpad1") => Key::Numpad1,
            Some("Numpad2") => Key::Numpad2,
            Some("Numpad3") => Key::Numpad3,
            Some("Numpad4") => Key::Numpad4,
            Some("Numpad5") => Key::Numpad5,
            Some("Numpad6") => Key::Numpad6,
            Some("Numpad7") => Key::Numpad7,
            Some("Numpad8") => Key::Numpad8,
            Some("Numpad9") => Key::Numpad9,
            Some("NumpadAdd") => Key::Add,
            Some("NumpadSubtract") => Key::Subtract,
            Some("NumpadMultiply") => Key::Multiply,
            Some("NumpadDivide") => Key::Divide,
            Some("NumpadDecimal") => Key::Decimal,
            Some(s) if s.len() == 1 => Key::Unicode(s.chars().next().unwrap()),
            _ => Key::F20,
        };

        let _ = enigo.key(key, Direction::Click);
        log::info!("[MuteHandler] Sent key: {:?}", keybind);
    }

    // ===== Called when mute state CHANGES (toggle event) =====
    pub fn on_mute_toggled(&self, _muted: bool) {
        match self.get_mode() {
            MuteButtonMode::Standard => self.do_mute(),
            MuteButtonMode::MediaPlayPause => self.do_media_play_pause(),
            MuteButtonMode::SmartDouble => self.handle_smart_double(),
            MuteButtonMode::SmartHold => {
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
                self.do_media_play_pause();
                *last = None;
                return;
            }
        }

        self.do_mute();
        *last = Some(now);
    }

    // ===== SmartHold: requires down/up events =====
    pub fn on_button_down(&self) {
        if self.get_mode() == MuteButtonMode::SmartHold {
            *self.press_start.lock() = Some(Instant::now());
        }
    }

    pub fn on_button_up(&self) {
        if self.get_mode() != MuteButtonMode::SmartHold {
            return;
        }

        let mut press = self.press_start.lock();
        if let Some(start) = *press {
            let held = start.elapsed();

            if held < Duration::from_millis(LONG_PRESS_MS) {
                self.do_media_play_pause();
            } else {
                self.do_mute();
            }
            *press = None;
        }
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_MUTE_HANDLER: MuteHandler = MuteHandler::new();
}