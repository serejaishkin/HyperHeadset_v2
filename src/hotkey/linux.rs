use super::{GlobalHotkeyCapture, KeyCombo};
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const EV_KEY: u16 = 1;
const KEY_PRESS: i32 = 1;
const KEY_RELEASE: i32 = 0;

fn code_to_name(code: u16) -> Option<String> {
    if (183..=194).contains(&code) {
        return Some(format!("F{}", code - 183 + 13));
    }
    if (2..=10).contains(&code) {
        return Some(((code - 2 + (b'1' as u16)) as u8 as char).to_string());
    }
    if code == 11 {
        return Some("0".into());
    }
    match code {
        29 => Some("LCtrl".into()),
        42 => Some("LShift".into()),
        56 => Some("LAlt".into()),
        125 => Some("LWin".into()),
        97 => Some("RCtrl".into()),
        54 => Some("RShift".into()),
        100 => Some("RAlt".into()),
        126 => Some("RWin".into()),
        113 => Some("MediaVolumeMute".into()),
        114 => Some("MediaVolumeDown".into()),
        115 => Some("MediaVolumeUp".into()),
        163 => Some("MediaNextTrack".into()),
        164 => Some("MediaPlayPause".into()),
        165 => Some("MediaPrevTrack".into()),
        82 => Some("Numpad0".into()),
        79 => Some("Numpad1".into()),
        80 => Some("Numpad2".into()),
        81 => Some("Numpad3".into()),
        75 => Some("Numpad4".into()),
        76 => Some("Numpad5".into()),
        77 => Some("Numpad6".into()),
        71 => Some("Numpad7".into()),
        72 => Some("Numpad8".into()),
        73 => Some("Numpad9".into()),
        98 => Some("NumpadDivide".into()),
        55 => Some("NumpadMultiply".into()),
        74 => Some("NumpadSubtract".into()),
        78 => Some("NumpadAdd".into()),
        83 => Some("NumpadDecimal".into()),
        16 => Some("Q".into()),
        17 => Some("W".into()),
        18 => Some("E".into()),
        19 => Some("R".into()),
        20 => Some("T".into()),
        21 => Some("Y".into()),
        22 => Some("U".into()),
        23 => Some("I".into()),
        24 => Some("O".into()),
        25 => Some("P".into()),
        30 => Some("A".into()),
        31 => Some("S".into()),
        32 => Some("D".into()),
        33 => Some("F".into()),
        34 => Some("G".into()),
        35 => Some("H".into()),
        36 => Some("J".into()),
        37 => Some("K".into()),
        38 => Some("L".into()),
        44 => Some("Z".into()),
        45 => Some("X".into()),
        46 => Some("C".into()),
        47 => Some("V".into()),
        48 => Some("B".into()),
        49 => Some("N".into()),
        50 => Some("M".into()),
        _ => None,
    }
}

pub fn spawn_capture_thread(capture: Arc<GlobalHotkeyCapture>) {
    thread::spawn(move || {
        let input_dir = Path::new("/dev/input");
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(input_dir) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with("event") {
                    if let Ok(f) = fs::OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(entry.path()) {
                        files.push(f);
                    }
                }
            }
        }
        if files.is_empty() {
            log::warn!("[LinuxHotkey] No input devices. Run with sudo or add udev rules.");
            return;
        }
        log::info!("[LinuxHotkey] Monitoring {} input devices", files.len());

        let mut currently_pressed: HashSet<u16> = HashSet::new();
        let mut captured_keys: Option<Vec<String>> = None;
        let mut last_press = std::time::Instant::now();

        loop {
            if !capture.is_recording() {
                currently_pressed.clear(); captured_keys = None;
                thread::sleep(Duration::from_millis(50)); continue;
            }
            for file in &mut files {
                use std::io::Read;
                let mut buf = [0u8; 24];
                loop {
                    match file.read(&mut buf) {
                        Ok(24) => {
                            let type_ = u16::from_ne_bytes([buf[16], buf[17]]);
                            let code = u16::from_ne_bytes([buf[18], buf[19]]);
                            let value = i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]);
                            if type_ == EV_KEY {
                                if value == KEY_PRESS { currently_pressed.insert(code); last_press = std::time::Instant::now(); }
                                else if value == KEY_RELEASE { currently_pressed.remove(&code); }
                            }
                        }
                        Ok(_) | Err(_) => break,
                    }
                }
            }
            if !currently_pressed.is_empty() {
                let mut names: Vec<String> = currently_pressed.iter().filter_map(|&c| code_to_name(c)).collect();
                names.sort(); names.dedup();
                if !names.is_empty() { captured_keys = Some(names); }
            } else if let Some(keys) = captured_keys.take() {
                if !keys.is_empty() && last_press.elapsed() > Duration::from_millis(50) {
                    let display = keys.join("+");
                    log::info!("[LinuxHotkey] Captured: {}", display);
                    *capture.result.lock().unwrap() = Some(KeyCombo { keys, display });
                    *capture.recording.lock().unwrap() = false;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
}
