//! Windows global hotkey capture using Raw Input + GetAsyncKeyState

use super::{GlobalHotkeyCapture, KeyCombo};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const VK_CODES: &[(u16, &str)] = &[
    // Modifiers
    (0x10, "Shift"),
    (0x11, "Ctrl"),
    (0x12, "Alt"),
    (0x5B, "Win"),
    (0x5C, "Win"),

    // Left / Right modifiers
    (0xA0, "LShift"),
    (0xA1, "RShift"),
    (0xA2, "LCtrl"),
    (0xA3, "RCtrl"),
    (0xA4, "LAlt"),
    (0xA5, "RAlt"),

    // Navigation
    (0x21, "PageUp"),
    (0x22, "PageDown"),
    (0x23, "End"),
    (0x24, "Home"),
    (0x25, "Left"),
    (0x26, "Up"),
    (0x27, "Right"),
    (0x28, "Down"),
    (0x2D, "Insert"),
    (0x2E, "Delete"),

    // Function keys
    (0x70, "F1"),
    (0x71, "F2"),
    (0x72, "F3"),
    (0x73, "F4"),
    (0x74, "F5"),
    (0x75, "F6"),
    (0x76, "F7"),
    (0x77, "F8"),
    (0x78, "F9"),
    (0x79, "F10"),
    (0x7A, "F11"),
    (0x7B, "F12"),
    (0x7C, "F13"),
    (0x7D, "F14"),
    (0x7E, "F15"),
    (0x7F, "F16"),
    (0x80, "F17"),
    (0x81, "F18"),
    (0x82, "F19"),
    (0x83, "F20"),
    (0x84, "F21"),
    (0x85, "F22"),
    (0x86, "F23"),
    (0x87, "F24"),

    // Media
    (0xAD, "MediaVolumeMute"),
    (0xAE, "MediaVolumeDown"),
    (0xAF, "MediaVolumeUp"),
    (0xB0, "MediaNextTrack"),
    (0xB1, "MediaPrevTrack"),
    (0xB2, "MediaStop"),
    (0xB3, "MediaPlayPause"),
	
	    // Numpad
    (0x60, "Numpad0"),
    (0x61, "Numpad1"),
    (0x62, "Numpad2"),
    (0x63, "Numpad3"),
    (0x64, "Numpad4"),
    (0x65, "Numpad5"),
    (0x66, "Numpad6"),
    (0x67, "Numpad7"),
    (0x68, "Numpad8"),
    (0x69, "Numpad9"),
    (0x6A, "NumpadMultiply"),
    (0x6B, "NumpadAdd"),
    (0x6C, "NumpadSeparator"),
    (0x6D, "NumpadSubtract"),
    (0x6E, "NumpadDecimal"),
    (0x6F, "NumpadDivide"),
];

pub fn spawn_capture_thread(capture: Arc<GlobalHotkeyCapture>) {
    println!("[DEBUG HOTKEY] spawn_capture_thread started");
    thread::spawn(move || {
        let mut captured_keys: Option<Vec<String>> = None;
        let mut tick = 0u32;
        loop {
            thread::sleep(Duration::from_millis(10));
            if !capture.is_recording() {
                captured_keys = None;
                continue;
            }

            tick += 1;
            if tick % 50 == 0 {
                println!("[DEBUG HOTKEY] still recording... (tick {})", tick);
            }

            let mut currently_pressed = Vec::new();
            unsafe {
                use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
                for (vk, name) in VK_CODES.iter() {
                    if GetAsyncKeyState(*vk as i32) < 0 {
                        currently_pressed.push(name.to_string());
                    }
                }
                for vk in 0x30..=0x5A {
                    if GetAsyncKeyState(vk) < 0 {
                        let ch = (vk as u8 as char).to_ascii_uppercase();
                        currently_pressed.push(format!("{}", ch));
                    }
                }
            }

            currently_pressed.sort();
            currently_pressed.dedup();

            if !currently_pressed.is_empty() {
                println!("[DEBUG HOTKEY] keys held: {:?}", currently_pressed);
                captured_keys = Some(currently_pressed.clone());
            } else if let Some(keys) = captured_keys.take() {
                if !keys.is_empty() {
                    let display = keys.join("+");
                    println!("[DEBUG HOTKEY] Captured: {}", display);
                    *capture.result.lock().unwrap() = Some(KeyCombo { keys, display });
                    capture.stop_recording();
                    println!("[DEBUG HOTKEY] Recording stopped, result saved");
                }
            }
        }
    });
}