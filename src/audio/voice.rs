use std::io::Cursor;

#[cfg(feature = "embedded-voice")]
mod embedded {
    pub const BAT_000: &[u8] = include_bytes!("../../../assets/voice/bat_000.wav");
    pub const BAT_010: &[u8] = include_bytes!("../../../assets/voice/bat_010.wav");
    pub const BAT_020: &[u8] = include_bytes!("../../../assets/voice/bat_020.wav");
    pub const BAT_050: &[u8] = include_bytes!("../../../assets/voice/bat_050.wav");
    pub const BAT_100: &[u8] = include_bytes!("../../../assets/voice/bat_100.wav");
    pub const CHARGING: &[u8] = include_bytes!("../../../assets/voice/charging.wav");
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
    let bytes: Option<&'static [u8]> = match event {
        VoiceEvent::Battery(p) => Some(nearest_battery(p)),
        VoiceEvent::Charging | VoiceEvent::FullCharge => Some(embedded::CHARGING),
        VoiceEvent::LowBattery => Some(embedded::BAT_010),
        VoiceEvent::Connected | VoiceEvent::Disconnected => None,
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
