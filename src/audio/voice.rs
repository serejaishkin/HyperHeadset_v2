use std::io::Cursor;
use std::sync::OnceLock;

static VOICE_CFG: OnceLock<std::sync::Mutex<crate::config::VoiceConfig>> = OnceLock::new();

fn get_cfg() -> &'static std::sync::Mutex<crate::config::VoiceConfig> {
    VOICE_CFG.get_or_init(|| std::sync::Mutex::new(crate::config::VoiceConfig::default()))
}

pub fn update_config(cfg: crate::config::VoiceConfig) {
    *get_cfg().lock().unwrap() = cfg;
}

pub fn vlog(msg: &str) {
    let log_path = std::env::temp_dir().join("hyper_voice_debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        use std::io::Write;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let _ = writeln!(f, "[{}] {}", now, msg);
        let _ = f.flush();
    }
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
    let cfg = get_cfg().lock().unwrap().clone();
    if !cfg.enabled { return; }
    vlog(&format!("play() called: {:?}", event));
    if let Some(data) = try_custom_voice(&event) {
        play_bytes_owned(data);
        return;
    }
    let bytes: Option<&'static [u8]> = match event {
        VoiceEvent::Battery(p) => Some(crate::audio::embedded_voice::get(p)),
        VoiceEvent::Charging => { if !cfg.on_charging { return; } Some(crate::audio::embedded_voice::CHARGING) }
        VoiceEvent::FullCharge => { if !cfg.on_full_charge { return; } Some(crate::audio::embedded_voice::FULL_CHARGE) }
        VoiceEvent::LowBattery => { if !cfg.on_battery_low { return; } Some(crate::audio::embedded_voice::LOW_BATTERY) }
        VoiceEvent::Connected | VoiceEvent::Disconnected => None,
    };
    if let Some(bytes) = bytes {
        play_bytes(bytes);
    }
}

#[cfg(feature = "embedded-voice")]
fn try_custom_voice(event: &VoiceEvent) -> Option<Vec<u8>> {
    let cfg = crate::config::Config::load().unwrap_or_default();
    let dir = cfg.custom_voice_dir?;
    let fname = match event {
        VoiceEvent::Battery(p) => format!("bat_{:03}.wav", p),
        VoiceEvent::Charging => "charging.wav".to_string(),
        VoiceEvent::FullCharge => "full_charge.wav".to_string(),
        VoiceEvent::LowBattery => "low_battery.wav".to_string(),
        _ => return None,
    };
    let path = std::path::PathBuf::from(dir).join(fname);
    if path.exists() { std::fs::read(path).ok() } else { None }
}

/// Plays one of the bundled WAV files without requiring a connected headset.
/// This makes Settings -> Voice -> Test voice useful even when HID is unavailable.
#[cfg(feature = "embedded-voice")]
pub fn play_test() {
    vlog("play_test() called");
    play_bytes(crate::audio::embedded_voice::BAT_050);
}

#[cfg(not(feature = "embedded-voice"))]
pub fn play(_event: VoiceEvent) { vlog("play() called but feature DISABLED"); }

#[cfg(not(feature = "embedded-voice"))]
pub fn play_test() { vlog("play_test() called but feature DISABLED"); }

/// Pause between two queued announcements so they never overlap or run into each other.
#[cfg(feature = "embedded-voice")]
const VOICE_GAP: std::time::Duration = std::time::Duration::from_millis(700);

#[cfg(feature = "embedded-voice")]
enum Clip {
    Static(&'static [u8]),
    Owned(Vec<u8>),
}

/// All announcements go through one queue: they play one after another with a pause in between.
#[cfg(feature = "embedded-voice")]
fn queue() -> &'static std::sync::mpsc::Sender<Clip> {
    use std::sync::{mpsc, OnceLock};
    static REF: OnceLock<&'static mpsc::Sender<Clip>> = OnceLock::new();
    REF.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Clip>();
        std::thread::Builder::new()
            .name("voice-queue".into())
            .spawn(move || {
                while let Ok(clip) = rx.recv() {
                    let r = match clip {
                        Clip::Static(b) => play_blocking(b),
                        Clip::Owned(d) => play_blocking_owned(d),
                    };
                    match r {
                        Ok(()) => vlog("playback OK"),
                        Err(e) => vlog(&format!("playback ERROR: {}", e)),
                    }
                    std::thread::sleep(VOICE_GAP);
                }
            })
            .ok();
        Box::leak(Box::new(tx))
    })
}

#[cfg(feature = "embedded-voice")]
fn play_bytes(bytes: &'static [u8]) {
    if bytes.len() < 44 { log::warn!("[Voice] WAV is too small ({} bytes)", bytes.len()); return; }
    vlog(&format!("playback queued, len={}", bytes.len()));
    let _ = queue().send(Clip::Static(bytes));
}

#[cfg(feature = "embedded-voice")]
fn play_bytes_owned(data: Vec<u8>) {
    if data.len() < 44 { log::warn!("[Voice] WAV is too small ({} bytes)", data.len()); return; }
    vlog(&format!("playback queued custom, len={}", data.len()));
    let _ = queue().send(Clip::Owned(data));
}

#[cfg(feature = "embedded-voice")]
fn play_blocking(bytes: &'static [u8]) -> Result<(), Box<dyn std::error::Error>> {
    use rodio::{Decoder, OutputStream, Sink};
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let source = Decoder::new(Cursor::new(bytes))?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

#[cfg(feature = "embedded-voice")]
fn play_blocking_owned(data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    use rodio::{Decoder, OutputStream, Sink};
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let source = Decoder::new(Cursor::new(data))?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
