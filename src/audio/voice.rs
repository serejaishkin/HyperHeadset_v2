use std::io::{Cursor, Write};
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref VOICE_LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);
}

fn vlog(msg: &str) {
    let mut guard = VOICE_LOG.lock().unwrap();
    if guard.is_none() {
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("hyper_voice_debug.log")))
            .unwrap_or_else(|| std::path::PathBuf::from("hyper_voice_debug.log"));
        *guard = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok();
    }
    if let Some(ref mut f) = *guard {
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
    vlog(&format!("play() called with {:?}", event));
    
    let bytes: Option<&'static [u8]> = match event {
        VoiceEvent::Battery(p) => Some(nearest_battery(p)),
        VoiceEvent::Charging => Some(embedded::CHARGING),
        VoiceEvent::FullCharge => Some(embedded::FULL_CHARGE),
        VoiceEvent::LowBattery => Some(embedded::LOW_BATTERY),
        VoiceEvent::Connected | VoiceEvent::Disconnected => None,
    };

    if let Some(bytes) = bytes {
        vlog(&format!("Starting playback, bytes len = {}", bytes.len()));
        std::thread::spawn(move || {
            if let Err(e) = play_blocking(bytes) {
                vlog(&format!("Playback FAILED: {}", e));
            } else {
                vlog("Playback finished OK");
            }
        });
    } else {
        vlog("No audio for this event");
    }
}

#[cfg(not(feature = "embedded-voice"))]
pub fn play(_event: VoiceEvent) {
    vlog("play() called but feature DISABLED");
}

#[cfg(feature = "embedded-voice")]
fn nearest_battery(percent: u8) -> &'static [u8] {
    vlog(&format!("nearest_battery({})", percent));
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
    vlog("rodio: creating OutputStream...");
    let (_stream, stream_handle) = OutputStream::try_default()?;
    vlog("rodio: OutputStream OK");
    let sink = Sink::try_new(&stream_handle)?;
    vlog("rodio: Sink OK");
    let cursor = Cursor::new(bytes);
    let source = Decoder::new(cursor)?;
    vlog("rodio: Decoder OK, playing...");
    sink.append(source);
    sink.sleep_until_end();
    vlog("rodio: sleep_until_end finished");
    Ok(())
}
