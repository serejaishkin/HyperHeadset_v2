//! Native Windows key injection (SendInput with hardware scan codes).
//!
//! Scan-code events look like a real keyboard to apps and low-level hooks (Discord, OBS, games),
//! which is why this is used instead of virtual-key-only injection. Everything is logged so a
//! failing setup can be diagnosed from the log alone.

use std::time::Duration;

#[repr(C)]
#[derive(Clone, Copy)]
struct KeybdInput {
    vk: u16,
    scan: u16,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
union InputUnion {
    ki: KeybdInput,
    mi: MouseInput,
}

#[repr(C)]
struct Input {
    kind: u32,
    u: InputUnion,
}

#[link(name = "user32")]
extern "system" {
    fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
    fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
    fn GetAsyncKeyState(vk: i32) -> i16;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const KEYEVENTF_SCANCODE: u32 = 0x0008;
const MAPVK_VK_TO_VSC_EX: u32 = 4;

/// Keys that need KEYEVENTF_EXTENDEDKEY.
fn is_extended(vk: u16) -> bool {
    matches!(
        vk,
        0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x6F | 0xA3 | 0xA5
    )
}

/// Windows virtual-key code for a keybind token, plus whether it is a modifier.
pub fn vk_for(token: &str) -> Option<(u16, bool)> {
    let t = token.trim().to_ascii_lowercase();
    let m = |v| Some((v, true));
    let k = |v| Some((v, false));
    match t.as_str() {
        "ctrl" | "control" | "lctrl" => m(0xA2),
        "rctrl" => m(0xA3),
        "shift" | "lshift" => m(0xA0),
        "rshift" => m(0xA1),
        "alt" | "lalt" => m(0xA4),
        "ralt" => m(0xA5),
        "win" | "meta" | "super" | "cmd" | "command" => m(0x5B),
        "backspace" => k(0x08),
        "tab" => k(0x09),
        "enter" | "return" => k(0x0D),
        "esc" | "escape" => k(0x1B),
        "space" => k(0x20),
        "pageup" => k(0x21),
        "pagedown" => k(0x22),
        "end" => k(0x23),
        "home" => k(0x24),
        "left" | "arrowleft" => k(0x25),
        "up" | "arrowup" => k(0x26),
        "right" | "arrowright" => k(0x27),
        "down" | "arrowdown" => k(0x28),
        "insert" => k(0x2D),
        "delete" | "del" => k(0x2E),
        "-" | "minus" => k(0xBD),
        "=" | "+" | "equals" | "plus" => k(0xBB),
        "," | "comma" => k(0xBC),
        "." | "period" => k(0xBE),
        "/" | "slash" => k(0xBF),
        ";" | "semicolon" => k(0xBA),
        "'" | "quote" => k(0xDE),
        "[" => k(0xDB),
        "]" => k(0xDD),
        "\\" | "backslash" => k(0xDC),
        "`" => k(0xC0),
        "mediavolumemute" | "mute" => k(0xAD),
        "mediavolumedown" => k(0xAE),
        "mediavolumeup" => k(0xAF),
        "medianexttrack" => k(0xB0),
        "mediaprevtrack" => k(0xB1),
        "mediastop" => k(0xB2),
        "mediaplaypause" => k(0xB3),
        "numpadmultiply" => k(0x6A),
        "numpadadd" => k(0x6B),
        "numpadseparator" => k(0x6C),
        "numpadsubtract" | "numpadminus" => k(0x6D),
        "numpaddecimal" => k(0x6E),
        "numpaddivide" => k(0x6F),
        s => {
            if let Some(n) = s.strip_prefix("numpad").and_then(|n| n.parse::<u16>().ok()) {
                return (n <= 9).then(|| (0x60 + n, false));
            }
            if let Some(n) = s.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
                return (1..=24).contains(&n).then(|| (0x70 + n - 1, false));
            }
            let mut ch = s.chars();
            match (ch.next(), ch.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() => k(c.to_ascii_uppercase() as u16),
                _ => None,
            }
        }
    }
}

fn key_input(vk: u16, up: bool) -> Input {
    let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC_EX) };
    let mut flags = if up { KEYEVENTF_KEYUP } else { 0 };
    if is_extended(vk) || (scan & 0xE000) != 0 {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    // Scan-code mode for ordinary keys; media/browser keys have no scan code, use the vk.
    let (wvk, wscan) = if (0xA6..=0xB7).contains(&vk) || scan & 0xFF == 0 {
        (vk, 0u16)
    } else {
        flags |= KEYEVENTF_SCANCODE;
        (0u16, (scan & 0xFF) as u16)
    };
    Input {
        kind: INPUT_KEYBOARD,
        u: InputUnion { ki: KeybdInput { vk: wvk, scan: wscan, flags, time: 0, extra: 0 } },
    }
}

fn send(vk: u16, up: bool) -> Result<(), String> {
    let input = key_input(vk, up);
    let n = unsafe { SendInput(1, &input, std::mem::size_of::<Input>() as i32) };
    if n != 1 {
        return Err(format!("SendInput vk=0x{:02X} up={} returned {} (GetLastError={})", vk, up, n, unsafe { GetLastError() }));
    }
    Ok(())
}

/// Press modifiers, click `main`, release everything (modifiers always released).
pub fn send_keys(modifiers: &[u16], main: u16) -> Result<(), String> {
    let mut res = Ok(());
    for &m in modifiers {
        if let Err(e) = send(m, false) {
            res = Err(e);
        }
    }
    std::thread::sleep(Duration::from_millis(20));
    let r = send(main, false);
    std::thread::sleep(Duration::from_millis(50));
    let seen_down = unsafe { GetAsyncKeyState(main as i32) } < 0;
    log::info!("[MuteHandler] SendInput vk=0x{:02X} down ok={} system sees it pressed={}", main, r.is_ok(), seen_down);
    let r2 = send(main, true);
    std::thread::sleep(Duration::from_millis(20));
    for &m in modifiers.iter().rev() {
        let _ = send(m, true);
    }
    res.and(r).and(r2)
}
