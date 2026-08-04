use std::io::Cursor;

// === ВШИТЫЕ АССЕТЫ ===
#[cfg(feature = "embedded-voice")]
mod embedded {
    pub const BAT_000: &[u8] = include_bytes!("../../../assets/voice/bat_000.wav");
    pub const BAT_010: &[u8] = include_bytes!("../../../assets/voice/bat_010.wav");
    pub const BAT_020: &[u8] = include_bytes!("../../../assets/voice/bat_020.wav");
    pub const BAT_030: &[u8] = include_bytes!("../../../assets/voice/bat_030.wav");
    pub const BAT_040: &[u8] = include_bytes!("../../../assets/voice/bat_040.wav");
    pub const BAT_050: &[u8] = include_bytes!("../../../assets/voice/bat_050.wav");
    pub const BAT_060: &[u8] = include_bytes!("../../../assets/voice/bat_060.wav");
    pub const BAT_070: &[u8] = include_bytes!("../../../assets/voice/bat_070.wav");
    pub const BAT_080: &[u8] = include_bytes!("../../../assets/voice/bat_080.wav");
    pub const BAT_090: &[u8] = include_bytes!("../../../assets/voice/bat_090.wav");
    pub const BAT_100: &[u8] = include_bytes!("../../../assets/voice/bat_100.wav");
    pub const CHARGING: &[u8] = include_bytes!("../../../assets/voice/charging.wav");
    pub const FULL_CHARGE: &[u8] = include_bytes!("../../../assets/voice/full_charge.wav");
    pub const LOW_BATTERY: &[u8] = include_bytes!("../../../assets/voice/low_battery.wav");
    pub const CONNECTED: &[u8] = include_bytes!("../../../assets/voice/connected.wav");
    pub const DISCONNECTED: &[u8] = include_bytes!("../../../assets/voice/disconnected.wav");
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

/// Воспроизводит голос в отдельном потоке. Не блокирует GUI.
#[cfg(feature = "embedded-voice")]
pub fn play(event: VoiceEvent) {
    let bytes: &'static [u8] = match event {
        VoiceEvent::Battery(p) => nearest_battery(p),
        VoiceEvent::Charging => embedded::CHARGING,
        VoiceEvent::FullCharge => embedded::FULL_CHARGE,
        VoiceEvent::LowBattery => embedded::LOW_BATTERY,
        VoiceEvent::Connected => embedded::CONNECTED,
        VoiceEvent::Disconnected => embedded::DISCONNECTED,
    };

    std::thread::spawn(move || {
        if let Err(e) = play_blocking(bytes) {
            log::warn!("[Voice] Playback error: {}", e);
        }
    });
}

/// Заглушка, если фича выключена.
#[cfg(not(feature = "embedded-voice"))]
pub fn play(_event: VoiceEvent) {
    log::warn!("[Voice] Feature 'embedded-voice' disabled, skipping");
}

#[cfg(feature = "embedded-voice")]
fn nearest_battery(percent: u8) -> &'static [u8] {
    match percent {
        0..=5 => embedded::BAT_000,
        6..=15 => embedded::BAT_010,
        16..=25 => embedded::BAT_020,
        26..=35 => embedded::BAT_030,
        36..=45 => embedded::BAT_040,
        46..=55 => embedded::BAT_050,
        56..=65 => embedded::BAT_060,
        66..=75 => embedded::BAT_070,
        76..=85 => embedded::BAT_080,
        86..=95 => embedded::BAT_090,
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
