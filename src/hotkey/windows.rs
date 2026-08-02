//! Windows global hotkey capture using Raw Input + GetAsyncKeyState

use super::{GlobalHotkeyCapture, KeyCombo};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const VK_CODES: &[(u16, &str)] = &[
    (0xA0, "Shift"), (0xA1, "Shift"), (0xA2, "Ctrl"), (0xA3, "Ctrl"),
    (0xA4, "Alt"), (0xA5, "Alt"),
    (0x70, "F1"), (0x71, "F2"), (0x72, "F3"), (0x73, "F4"),
    (0x74, "F5"), (0x75, "F6"), (0x76, "F7"), (0x77, "F8"),
    (0x78, "F9"), (0x79, "F10"), (0x7A, "F11"), (0x7B, "F12"),
    (0x7C, "F13"), (0x7D, "F14"), (0x7E, "F15"), (0x7F, "F16"),
    (0x80, "F17"), (0x81, "F18"), (0x82, "F19"), (0x83, "F20"),
    (0x84, "F21"), (0x85, "F22"), (0x86, "F23"), (0x87, "F24"),
    (0xAD, "MediaVolumeMute"), (0xAE, "MediaVolumeDown"), (0xAF, "MediaVolumeUp"),
    (0xB0, "MediaNextTrack"), (0xB1, "MediaPrevTrack"), (0xB2, "MediaStop"), (0xB3, "MediaPlayPause"),
];

pub fn spawn_capture_thread(capture: Arc<GlobalHotkeyCapture>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(10));

            if !capture.is_recording() {
                continue;
            }

            let mut pressed = Vec::new();

            // Check modifier keys
            unsafe {
                use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

                for (vk, name) in VK_CODES.iter() {
                    let state = GetAsyncKeyState(*vk as i32);
                    if state < 0 { // High bit set = currently pressed
                        pressed.push(name.to_string());
                    }
                }

                // Check regular keys A-Z, 0-9
                for vk in 0x30..=0x5A {
                    let state = GetAsyncKeyState(vk);
                    if state < 0 {
                        let name = format!("Key{}", (vk as u8 as char).to_ascii_uppercase());
                        pressed.push(name);
                    }
                }
            }

            // Deduplicate and sort
            pressed.sort();
            pressed.dedup();

            // If we have keys and user released them (detected by checking no keys pressed after a delay)
            if !pressed.is_empty() {
                thread::sleep(Duration::from_millis(100));

                // Check if all released
                let mut any_pressed = false;
                unsafe {
                    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
                    for (vk, _) in VK_CODES.iter() {
                        if GetAsyncKeyState(*vk as i32) < 0 {
                            any_pressed = true;
                            break;
                        }
                    }
                }

                if !any_pressed {
                    let display = pressed.join("+");
                    let combo = KeyCombo {
                        keys: pressed.clone(),
                        display: display.clone(),
                    };
                    *capture.result.lock().unwrap() = Some(combo);
                    *capture.recording.lock().unwrap() = false;
                }
            }
        }
    });
}
