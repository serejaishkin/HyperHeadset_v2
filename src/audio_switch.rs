//! Switches the default audio devices to the saved fallback ones while the headset is off or
//! unplugged, and back when it returns. Shared by the egui and Tauri front-ends.

use crate::config::Config;
use crate::system_audio;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

static DEBOUNCE_SECS: AtomicU32 = AtomicU32::new(2);

/// Seconds the headset must stay off before audio switches away (0 = immediately).
pub fn set_debounce_secs(secs: u32) {
    DEBOUNCE_SECS.store(secs.min(60), Ordering::Relaxed);
}

#[derive(Default)]
pub struct AudioSwitch {
    away: bool,
    prev_out: Option<String>,
    prev_in: Option<String>,
    offline_since: Option<Instant>,
    back_attempts: u32,
    last_back_try: Option<Instant>,
}

lazy_static::lazy_static! {
    pub static ref AUDIO_SWITCH: Mutex<AudioSwitch> = Mutex::new(AudioSwitch::default());
}

impl AudioSwitch {
    /// Feed the headset's online state. Offline is only acted on after 2 s (status flickers while
    /// the headset reconnects); restoring is retried until the headset's endpoint exists again.
    pub fn update(&mut self, online: bool) {
        if !online {
            self.back_attempts = 0;
            let since = *self.offline_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(DEBOUNCE_SECS.load(Ordering::Relaxed) as u64) {
                self.go_away();
            }
        } else {
            self.offline_since = None;
            if self.away
                && self.back_attempts < 10
                && self.last_back_try.map_or(true, |t| t.elapsed() >= Duration::from_secs(2))
            {
                self.last_back_try = Some(Instant::now());
                self.back_attempts += 1;
                if self.come_back() {
                    self.back_attempts = 0;
                }
            }
        }
    }

    /// Dongle unplugged / device lost: switch immediately.
    pub fn force_away(&mut self) {
        self.go_away();
    }

    fn go_away(&mut self) {
        if self.away {
            return;
        }
        let a = Config::load().unwrap_or_default().audio;
        if !a.auto_switch_audio {
            return;
        }
        if a.fallback_output_device.is_none() && a.fallback_input_device.is_none() {
            log::warn!("[Audio] Auto-switch is on but no fallback devices are saved");
            return;
        }
        if let Some(ref id) = a.fallback_output_device {
            if self.prev_out.is_none() {
                self.prev_out = system_audio::get_default_output_device().ok().filter(|c| c != id);
            }
            if system_audio::get_default_output_device().map_or(false, |c| &c == id) {
                log::info!("[Audio] Fallback output already default, not touching it");
            } else if let Err(e) = { log::info!("[Audio] Headset away -> fallback output {}", id); system_audio::set_default_output_device(id) } {
                log::warn!("[Audio] Fallback output failed: {}", e);
            }
        }
        if let Some(ref id) = a.fallback_input_device {
            if self.prev_in.is_none() {
                self.prev_in = system_audio::get_default_input_device().ok().filter(|c| c != id);
            }
            if system_audio::get_default_input_device().map_or(false, |c| &c == id) {
                log::info!("[Audio] Fallback input already default, not touching it");
            } else if let Err(e) = { log::info!("[Audio] Headset away -> fallback input {}", id); system_audio::set_default_input_device(id) } {
                log::warn!("[Audio] Fallback input failed: {}", e);
            }
        }
        self.away = true;
    }

    fn come_back(&mut self) -> bool {
        let a = Config::load().unwrap_or_default().audio;
        let mut ok = true;
        if let Some(id) = a.headset_output_device.clone().or_else(|| self.prev_out.clone()) {
            if system_audio::get_default_output_device().map_or(false, |c| c == id) {
                log::info!("[Audio] Output already {}, not touching it", id);
            } else if let Err(e) = { log::info!("[Audio] Headset back -> output {}", id); system_audio::set_default_output_device(&id) } {
                log::warn!("[Audio] Restore output failed: {}", e);
                ok = false;
            }
        }
        if let Some(id) = a.headset_input_device.clone().or_else(|| self.prev_in.clone()) {
            if system_audio::get_default_input_device().map_or(false, |c| c == id) {
                log::info!("[Audio] Input already {}, not touching it", id);
            } else if let Err(e) = { log::info!("[Audio] Headset back -> input {}", id); system_audio::set_default_input_device(&id) } {
                log::warn!("[Audio] Restore input failed: {}", e);
                ok = false;
            }
        }
        if ok {
            self.away = false;
            self.prev_out = None;
            self.prev_in = None;
        }
        ok
    }
}
