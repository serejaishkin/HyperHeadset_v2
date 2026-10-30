//! Smart mute button handler with multiple action modes

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::system_audio;

const DOUBLE_CLICK_MS: u64 = 400;
const LONG_PRESS_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuteButtonMode {
    Standard,
    MediaPlayPause,
    SmartDouble,
    SmartHold,
    HoldPlayPause,
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
    keybind_enabled: AtomicBool,
    sidetone_play_pause: AtomicBool,
    last_sidetone_btn: Mutex<Option<Instant>>,
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
            keybind_enabled: AtomicBool::new(true),
            sidetone_play_pause: AtomicBool::new(false),
            last_sidetone_btn: Mutex::new(None),
        }
    }

    pub fn set_mode(&self, mode: MuteButtonMode) {
        *self.mode.lock() = mode;
    }

    pub fn get_mode(&self) -> MuteButtonMode {
        *self.mode.lock()
    }

    pub fn set_keybind(&self, keybind: Option<String>) {
        let mut cur = self.keybind.lock();
        if *cur != keybind {
            log::info!("[MuteHandler] Keybind set to: {:?}", keybind);
            *cur = keybind;
        }
    }

    /// Discord mode: only emulate the keybind when the mode is "Keybind".
    pub fn set_keybind_enabled(&self, enabled: bool) {
        self.keybind_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_sidetone_play_pause(&self, enabled: bool) {
        self.sidetone_play_pause.store(enabled, Ordering::Relaxed);
    }

    /// Called when the headset itself reports a sidetone toggle (not one we requested).
    pub fn on_sidetone_button(&self) {
        if !self.sidetone_play_pause.load(Ordering::Relaxed) {
            return;
        }
        {
            let mut last = self.last_sidetone_btn.lock();
            if let Some(t) = *last {
                if t.elapsed() < Duration::from_millis(300) {
                    return;
                }
            }
            *last = Some(Instant::now());
        }
        log::info!("[MuteHandler] Sidetone button -> Play/Pause");
        self.do_media_play_pause();
    }

    /// Send the configured keybind right now (GUI "Test" button).
    pub fn send_keybind_now(&self) -> Result<(), String> {
        let keybind = self.keybind.lock().clone();
        let combo = keybind.as_deref().unwrap_or("F20").to_string();
        let mut enigo = self.enigo.lock();
        let r = send_combo(&mut enigo, &combo);
        log::info!("[MuteHandler] Test combo {:?}: {:?}", combo, r);
        r
    }

    pub fn get_keybind(&self) -> Option<String> {
        self.keybind.lock().clone()
    }

    pub fn volume_up(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::VolumeUp, Direction::Click);
    }

    pub fn volume_down(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::VolumeDown, Direction::Click);
    }

    pub fn do_media_play_pause(&self) {
        let mut enigo = self.enigo.lock();
        let _ = enigo.key(Key::MediaPlayPause, Direction::Click);
        log::info!("[MuteHandler] MediaPlayPause");
    }

    pub fn do_mute(&self) {
        let mut last = self.last_hotkey.lock();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_millis(150) {
                log::warn!("[MuteHandler] Mute throttled (150ms debounce)");
                return;
            }
        }
        *last = Some(Instant::now());
        drop(last);

        let keybind = self.keybind.lock().clone();
        if self.keybind_enabled.load(Ordering::Relaxed) {
            let mut enigo = self.enigo.lock();
            log::info!("[MuteHandler] Attempting mute with keybind: {:?}", keybind);
            let combo = keybind.as_deref().unwrap_or("F20");
            let result = send_combo(&mut enigo, combo);
            log::info!("[MuteHandler] Sent combo {:?}, result: {:?}", combo, result);
        } else {
            log::info!("[MuteHandler] Discord keybind mode is off, key not sent");
        }

        // Also toggle system microphone mute so Discord sees the mic muted directly
        match system_audio::toggle_mic_mute() {
            Ok(()) => log::info!("[MuteHandler] System mic mute toggled"),
            Err(e) => log::warn!("[MuteHandler] System mic mute failed: {}", e),
        }
    }

    pub fn on_mute_toggled(&self, _muted: bool) {
        match self.get_mode() {
            MuteButtonMode::Standard => self.do_mute(),
            MuteButtonMode::MediaPlayPause => self.do_media_play_pause(),
            MuteButtonMode::SmartDouble => self.handle_smart_double(),
            MuteButtonMode::SmartHold => {
                // SmartHold requires down/up events which Tauri doesn't provide
                // Fall back to SmartDouble behavior for now
                log::warn!("[MuteHandler] SmartHold mode requires on_button_down/on_button_up events, using SmartDouble fallback");
                self.handle_smart_double();
            }
            MuteButtonMode::HoldPlayPause => {
                // HoldPlayPause requires down/up events which Tauri doesn't provide
                // Fall back to SmartDouble behavior for now
                log::warn!("[MuteHandler] HoldPlayPause mode requires on_button_down/on_button_up events, using SmartDouble fallback");
                self.handle_smart_double();
            }
        }
    }

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

    pub fn on_button_down(&self) {
        let mode = self.get_mode();
        if mode == MuteButtonMode::SmartHold || mode == MuteButtonMode::HoldPlayPause {
            *self.press_start.lock() = Some(Instant::now());
        }
    }

    pub fn on_button_up(&self) {
        let mode = self.get_mode();
        if mode != MuteButtonMode::SmartHold && mode != MuteButtonMode::HoldPlayPause {
            return;
        }
        let mut press = self.press_start.lock();
        if let Some(start) = *press {
            let held = start.elapsed();
            if mode == MuteButtonMode::SmartHold {
                if held < Duration::from_millis(LONG_PRESS_MS) {
                    self.do_media_play_pause();
                } else {
                    self.do_mute();
                }
            } else if mode == MuteButtonMode::HoldPlayPause {
                if held < Duration::from_millis(LONG_PRESS_MS) {
                    self.do_mute();
                } else {
                    self.do_media_play_pause();
                }
            }
            *press = None;
        }
    }
}

/// Parsed element of a keybind string such as "Ctrl+Shift+M" or "LCtrl+RAlt+F13".
enum Part {
    Modifier(Key),
    Main(Key),
}

#[cfg(target_os = "windows")]
fn char_key(c: char) -> Key {
    // Raw virtual-key codes: letters/digits map 1:1 to their upper-case ASCII code.
    // Using VK codes (instead of Key::Unicode) makes the event a real key press, so
    // modifiers + letter work as a global hotkey (Discord, OBS, ...).
    Key::Other(c.to_ascii_uppercase() as u32)
}

#[cfg(not(target_os = "windows"))]
fn char_key(c: char) -> Key {
    Key::Unicode(c.to_ascii_lowercase())
}

fn parse_part(token: &str) -> Option<Part> {
    let t = token.trim();
    let part = match t.to_ascii_lowercase().as_str() {
        // Modifiers — left/right/generic variants collapse to one (Windows capture reports both
        // "Ctrl" and "LCtrl" for a single physical key).
        "ctrl" | "control" | "lctrl" | "rctrl" => Part::Modifier(Key::Control),
        "shift" | "lshift" | "rshift" => Part::Modifier(Key::Shift),
        "alt" | "lalt" | "ralt" => Part::Modifier(Key::Alt),
        "win" | "meta" | "super" | "cmd" | "command" => Part::Modifier(Key::Meta),

        "f1" => Part::Main(Key::F1),
        "f2" => Part::Main(Key::F2),
        "f3" => Part::Main(Key::F3),
        "f4" => Part::Main(Key::F4),
        "f5" => Part::Main(Key::F5),
        "f6" => Part::Main(Key::F6),
        "f7" => Part::Main(Key::F7),
        "f8" => Part::Main(Key::F8),
        "f9" => Part::Main(Key::F9),
        "f10" => Part::Main(Key::F10),
        "f11" => Part::Main(Key::F11),
        "f12" => Part::Main(Key::F12),
        "f13" => Part::Main(Key::F13),
        "f14" => Part::Main(Key::F14),
        "f15" => Part::Main(Key::F15),
        "f16" => Part::Main(Key::F16),
        "f17" => Part::Main(Key::F17),
        "f18" => Part::Main(Key::F18),
        "f19" => Part::Main(Key::F19),
        "f20" => Part::Main(Key::F20),
        #[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
        "f21" => Part::Main(Key::F21),
        #[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
        "f22" => Part::Main(Key::F22),
        #[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
        "f23" => Part::Main(Key::F23),
        #[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
        "f24" => Part::Main(Key::F24),

        "mediaplaypause" => Part::Main(Key::MediaPlayPause),
        "mediastop" => Part::Main(Key::MediaStop),
        "mediavolumemute" | "mute" => Part::Main(Key::VolumeMute),
        "mediavolumedown" => Part::Main(Key::VolumeDown),
        "mediavolumeup" => Part::Main(Key::VolumeUp),
        "medianexttrack" => Part::Main(Key::MediaNextTrack),
        "mediaprevtrack" => Part::Main(Key::MediaPrevTrack),

        "numpad0" => Part::Main(Key::Numpad0),
        "numpad1" => Part::Main(Key::Numpad1),
        "numpad2" => Part::Main(Key::Numpad2),
        "numpad3" => Part::Main(Key::Numpad3),
        "numpad4" => Part::Main(Key::Numpad4),
        "numpad5" => Part::Main(Key::Numpad5),
        "numpad6" => Part::Main(Key::Numpad6),
        "numpad7" => Part::Main(Key::Numpad7),
        "numpad8" => Part::Main(Key::Numpad8),
        "numpad9" => Part::Main(Key::Numpad9),
        "numpadadd" => Part::Main(Key::Add),
        "numpadsubtract" | "numpadminus" => Part::Main(Key::Subtract),
        "numpadmultiply" => Part::Main(Key::Multiply),
        "numpaddivide" => Part::Main(Key::Divide),
        "numpaddecimal" => Part::Main(Key::Decimal),

        "space" => Part::Main(Key::Space),
        "enter" | "return" => Part::Main(Key::Return),
        "tab" => Part::Main(Key::Tab),
        "esc" | "escape" => Part::Main(Key::Escape),
        "backspace" => Part::Main(Key::Backspace),
        "insert" => Part::Main(Key::Insert),
        "delete" | "del" => Part::Main(Key::Delete),
        "home" => Part::Main(Key::Home),
        "end" => Part::Main(Key::End),
        "pageup" => Part::Main(Key::PageUp),
        "pagedown" => Part::Main(Key::PageDown),
        "left" => Part::Main(Key::LeftArrow),
        "right" => Part::Main(Key::RightArrow),
        "up" => Part::Main(Key::UpArrow),
        "down" => Part::Main(Key::DownArrow),

        s if s.chars().count() == 1 && s.chars().next().map_or(false, |c| c.is_ascii_alphanumeric()) => {
            Part::Main(char_key(s.chars().next().unwrap()))
        }
        _ => return None,
    };
    Some(part)
}

/// Press `modifiers` (in order), click the main key, release modifiers in reverse order.
/// Accepts either a single key ("F20") or a combo ("Ctrl+Shift+M").
fn send_combo(enigo: &mut Enigo, combo: &str) -> Result<(), String> {
    let mut modifiers: Vec<Key> = Vec::new();
    let mut main: Option<Key> = None;

    for token in combo.split('+') {
        match parse_part(token) {
            Some(Part::Modifier(k)) => {
                if !modifiers.contains(&k) {
                    modifiers.push(k);
                }
            }
            Some(Part::Main(k)) => main = Some(k),
            None => log::warn!("[MuteHandler] Unknown key '{}' in keybind '{}'", token.trim(), combo),
        }
    }

    // Only modifiers captured (e.g. user bound plain "Ctrl") — treat the last one as the key.
    if main.is_none() {
        main = modifiers.pop();
    }
    let Some(main) = main else {
        return Err(format!("keybind '{}' has no usable key", combo));
    };

    for m in &modifiers {
        enigo.key(*m, Direction::Press).map_err(|e| format!("press {:?}: {}", m, e))?;
    }
    if !modifiers.is_empty() {
        std::thread::sleep(Duration::from_millis(15));
    }
    let click = enigo
        .key(main, Direction::Press)
        .map_err(|e| format!("press {:?}: {}", main, e))
        .and_then(|_| {
            std::thread::sleep(Duration::from_millis(40));
            enigo.key(main, Direction::Release).map_err(|e| format!("release {:?}: {}", main, e))
        });
    if !modifiers.is_empty() {
        std::thread::sleep(Duration::from_millis(15));
    }
    // Always release modifiers, even if the click failed, so Ctrl/Shift never stay stuck.
    for m in modifiers.iter().rev() {
        let _ = enigo.key(*m, Direction::Release);
    }
    click
}


lazy_static::lazy_static! {
    pub static ref GLOBAL_MUTE_HANDLER: MuteHandler = MuteHandler::new();
}