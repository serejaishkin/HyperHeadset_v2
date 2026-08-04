use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref VOICE_CFG: Mutex<crate::config::VoiceConfig> = Mutex::new(crate::config::VoiceConfig::default());
}

pub fn update_config(cfg: crate::config::VoiceConfig) {
    *VOICE_CFG.lock().unwrap() = cfg;
}

#[cfg(feature = "embedded-voice")]
mod embedded {
    pub const BAT_000: &[u8] = include_bytes!("../../assets/voice/bat_000.wav");
    pub const BAT_010: &[u8] = include_bytes!("../../assets/voice/bat_010.wav");
    pub const BAT_020: &[u8] = include_bytes!("../../assets/voice/bat_020.wav");
    pub const BAT_050: &[u8] = include_bytes!("../../assets/voice/bat_050.wav");
    pub const BAT_100: &[u8] = include_bytes!("../../assets/voice/bat_100.wav");
    pub const CHARGING: &[u8] = include_bytes!("../../assets/voice/charging.wav");
    pub const FULL_CHARGE: &[u8] = include_bytes!("../../assets/voice/full_charge.wav");
    pub const LOW_BATTERY: &[u8] = include_bytes!("../../assets/voice/low_battery.wav");
}

#[derive(Debug, Clone, Copy)]
pub enum VoiceEvent {
    Battery(u8),
    Charging,
    FullCharge,
    LowBattery,
    Connected,
    Disconnected,
}

#[cfg(feature = "embedded-voice")]
pub fn play(event: VoiceEvent) {
    let cfg = VOICE_CFG.lock().unwrap().clone();
    if !cfg.enabled { return; }

    let bytes: Option<&'static [u8]> = match event {
        VoiceEvent::Battery(p) => {
            if p <= 20 && !cfg.on_battery_low { return; }
            if p > 20 && !cfg.on_button_check { return; }
            Some(select_battery(p, cfg.exact_percent))
        }
        VoiceEvent::Charging => {
            if !cfg.on_charging { return; }
            Some(embedded::CHARGING)
        }
        VoiceEvent::FullCharge => {
            if !cfg.on_full_charge { return; }
            Some(embedded::FULL_CHARGE)
        }
        VoiceEvent::LowBattery => {
            if !cfg.on_battery_low { return; }
            Some(embedded::LOW_BATTERY)
        }
        VoiceEvent::Connected => {
            if !cfg.on_connected { return; }
            None
        }
        VoiceEvent::Disconnected => {
            if !cfg.on_disconnected { return; }
            None
        }
    };

    if let Some(bytes) = bytes {
        std::thread::spawn(move || {
            if let Err(e) = play_blocking(bytes) {
                log::warn!("[Voice] Playback error: {}", e);
            }
        });
    }
}

#[cfg(not(feature = "embedded-voice"))]
pub fn play(_event: VoiceEvent) {}

#[cfg(feature = "embedded-voice")]
fn select_battery(percent: u8, exact: bool) -> &'static [u8] {
    if exact {
        if let Some(bytes) = get_exact_battery(percent) {
            return bytes;
        }
    }
    nearest_battery(percent)
}

#[cfg(not(feature = "embedded-voice"))]
fn select_battery(_percent: u8, _exact: bool) -> &'static [u8] {
    &[]
}

#[cfg(feature = "embedded-voice")]
fn get_exact_battery(percent: u8) -> Option<&'static [u8]> {
    match percent {
        0 => Some(embedded::BAT_000),
        10 => Some(embedded::BAT_010),
        20 => Some(embedded::BAT_020),
        50 => Some(embedded::BAT_050),
        100 => Some(embedded::BAT_100),
        _ => None,
    }
}

#[cfg(not(feature = "embedded-voice"))]
fn get_exact_battery(_percent: u8) -> Option<&'static [u8]> {
    None
}

#[cfg(feature = "embedded-voice")]
fn nearest_battery(percent: u8) -> &'static [u8] {
    match percent {
        0..=5 => embedded::BAT_000,
        6..=15 => embedded::BAT_010,
        16..=35 => embedded::BAT_020,
        36..=65 => embedded::BAT_050,
        _ => embedded::BAT_100,
    }
}

#[cfg(not(feature = "embedded-voice"))]
fn nearest_battery(_percent: u8) -> &'static [u8] {
    &[]
}

#[cfg(feature = "embedded-voice")]
fn play_blocking(bytes: &'static [u8]) -> Result<(), Box<dyn std::error::Error>> {
    use rodio::{Decoder, OutputStream, Sink};
    use std::io::Cursor;
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = Cursor::new(bytes);
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
