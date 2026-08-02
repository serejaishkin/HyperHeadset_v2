//! Linux global hotkey capture using evdev
//!
//! Reads from /dev/input/event* devices to capture key presses
//! Requires root or udev rules for input group access.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fs;
use std::path::Path;

use super::{GlobalHotkeyCapture, KeyCombo};

// Key codes from linux/input-event-codes.h
const KEY_F13: u16 = 183;
const KEY_F14: u16 = 184;
const KEY_F15: u16 = 185;
const KEY_F16: u16 = 186;
const KEY_F17: u16 = 187;
const KEY_F18: u16 = 188;
const KEY_F19: u16 = 189;
const KEY_F20: u16 = 190;
const KEY_F21: u16 = 191;
const KEY_F22: u16 = 192;
const KEY_F23: u16 = 193;
const KEY_F24: u16 = 194;
const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_LEFTALT: u16 = 56;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_RIGHTALT: u16 = 100;
const KEY_MUTE: u16 = 113;
const KEY_VOLUMEDOWN: u16 = 114;
const KEY_VOLUMEUP: u16 = 115;
const KEY_NEXTSONG: u16 = 163;
const KEY_PLAYPAUSE: u16 = 164;
const KEY_PREVIOUSSONG: u16 = 165;

const EV_KEY: u16 = 1;
const EV_SYN: u16 = 0;
const KEY_PRESS: i32 = 1;
const KEY_RELEASE: i32 = 0;

#[repr(C)]
struct InputEvent {
    time_sec: i64,
    time_usec: i64,
    type_: u16,
    code: u16,
    value: i32,
}

fn code_to_name(code: u16) -> Option<String> {
    match code {
        KEY_F13 => Some("F13".to_string()),
        KEY_F14 => Some("F14".to_string()),
        KEY_F15 => Some("F15".to_string()),
        KEY_F16 => Some("F16".to_string()),
        KEY_F17 => Some("F17".to_string()),
        KEY_F18 => Some("F18".to_string()),
        KEY_F19 => Some("F19".to_string()),
        KEY_F20 => Some("F20".to_string()),
        KEY_F21 => Some("F21".to_string()),
        KEY_F22 => Some("F22".to_string()),
        KEY_F23 => Some("F23".to_string()),
        KEY_F24 => Some("F24".to_string()),
        KEY_LEFTCTRL | KEY_RIGHTCTRL => Some("Ctrl".to_string()),
        KEY_LEFTSHIFT | KEY_RIGHTSHIFT => Some("Shift".to_string()),
        KEY_LEFTALT | KEY_RIGHTALT => Some("Alt".to_string()),
        KEY_MUTE => Some("MediaMute".to_string()),
        KEY_VOLUMEDOWN => Some("MediaVolumeDown".to_string()),
        KEY_VOLUMEUP => Some("MediaVolumeUp".to_string()),
        KEY_NEXTSONG => Some("MediaNextTrack".to_string()),
        KEY_PLAYPAUSE => Some("MediaPlayPause".to_string()),
        KEY_PREVIOUSSONG => Some("MediaPrevTrack".to_string()),
        _ => None,
    }
}

pub fn spawn_capture_thread(capture: Arc<GlobalHotkeyCapture>) {
    thread::spawn(move || {
        // Find keyboard devices
        let input_dir = Path::new("/dev/input");
        let mut devices = Vec::new();

        if let Ok(entries) = fs::read_dir(input_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("event") {
                    let path = entry.path();
                    // Try to open and check if it's a keyboard
                    if let Ok(file) = fs::OpenOptions::new().read(true).open(&path) {
                        devices.push((file, path));
                    }
                }
            }
        }

        if devices.is_empty() {
            log::warn!("[LinuxHotkey] No input devices found. Try running with sudo or adding udev rules.");
            return;
        }

        log::info!("[LinuxHotkey] Monitoring {} input devices", devices.len());

        let mut pressed_keys: Vec<u16> = Vec::new();
        let mut last_event_time = std::time::Instant::now();

        loop {
            if !capture.is_recording() {
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            for (file, _path) in &mut devices {
                use std::io::Read;
                let mut buf = [0u8; std::mem::size_of::<InputEvent>()];

                if let Ok(n) = file.read(&mut buf) {
                    if n == buf.len() {
                        let event: InputEvent = unsafe { std::ptr::read(buf.as_ptr() as *const _) };

                        if event.type_ == EV_KEY {
                            if event.value == KEY_PRESS {
                                if !pressed_keys.contains(&event.code) {
                                    pressed_keys.push(event.code);
                                    last_event_time = std::time::Instant::now();
                                }
                            } else if event.value == KEY_RELEASE {
                                pressed_keys.retain(|&k| k != event.code);

                                // If all keys released and we had a combo
                                if pressed_keys.is_empty() {
                                    let elapsed = last_event_time.elapsed();
                                    if elapsed < Duration::from_millis(2000) {
                                        let names: Vec<String> = pressed_keys
                                            .iter()
                                            .filter_map(|&c| code_to_name(c))
                                            .collect();

                                        if !names.is_empty() {
                                            let display = names.join("+");
                                            let combo = KeyCombo {
                                                keys: names,
                                                display: display.clone(),
                                            };
                                            *capture.result.lock().unwrap() = Some(combo);
                                            *capture.recording.lock().unwrap() = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(5));
        }
    });
}
