use std::io::Cursor;

pub fn vlog(msg: &str) {
    let log_path = std::env::temp_dir().join("hyper_voice_debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        use std::io::Write;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{}] {}", now, msg);
        let _ = f.flush();
    }
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
    vlog(&format!("play() called: {:?}", event));
    let bytes: Option<&'static [u8]> = match event {
        VoiceEvent::Battery(p) => Some(nearest_battery(p)),
        VoiceEvent::Charging => Some(embedded::CHARGING),
        VoiceEvent::FullCharge => Some(embedded::FULL_CHARGE),
        VoiceEvent::LowBattery => Some(embedded::LOW_BATTERY),
        VoiceEvent::Connected | VoiceEvent::Disconnected => None,
    };
    if let Some(bytes) = bytes {
        vlog(&format!("playback start, len={}", bytes.len()));
        std::thread::spawn(move || {
            if let Err(e) = play_blocking(bytes) {
                vlog(&format!("playback ERROR: {}", e));
            } else {
                vlog("playback OK");
            }
        });
    } else {
        vlog("no audio for event");
    }
}

#[cfg(not(feature = "embedded-voice"))]
pub fn play(_event: VoiceEvent) {
    vlog("play() called but feature DISABLED");
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

#[cfg(feature = "embedded-voice")]
fn play_blocking(bytes: &'static [u8]) -> Result<(), Box<dyn std::error::Error>> {
    use rodio::{Decoder, OutputStream, Sink};
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = Cursor::new(bytes);
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
