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

/// After the default output changed, move apps' running streams that stayed on another device
/// (runs in the background: apps often open/close streams a moment after the switch).
pub fn move_streams_after_switch(output_changed: bool) {
    if !output_changed {
        return;
    }
    if !Config::load().unwrap_or_default().audio.move_app_streams {
        return;
    }
    std::thread::spawn(|| {
        for delay in [1500u64, 3500] {
            std::thread::sleep(Duration::from_millis(delay));
            match system_audio::get_default_output_device() {
                Ok(id) => match crate::audio_route::move_streams_to(&id) {
                    Ok(0) => {}
                    Ok(n) => log::info!("[Audio] Moved {} app stream(s) to the current output", n),
                    Err(e) => { log::warn!("[Audio] Moving app streams failed: {}", e); return; }
                },
                Err(_) => return,
            }
        }
    });
}

/// Finds the headset's own audio endpoint by name (HyperX / Cloud ...), used when no explicit
/// headset device is chosen in the settings.
fn find_headset_device(output: bool) -> Option<String> {
    let devices = system_audio::get_audio_devices().ok()?;
    devices
        .into_iter()
        .filter(|d| if output { d.is_output } else { d.is_input })
        .find(|d| {
            let n = d.name.to_lowercase();
            n.contains("hyperx") || n.contains("cloud")
        })
        .map(|d| d.id)
}

#[derive(Default)]
pub struct AudioSwitch {
    away: bool,
    prev_out: Option<String>,
    prev_in: Option<String>,
    offline_since: Option<Instant>,
    back_attempts: u32,
    last_back_try: Option<Instant>,
    /// Defaults seen while the headset was on (Windows may already have moved them when it left).
    last_online_out: Option<String>,
    last_online_in: Option<String>,
    last_remember: Option<Instant>,
    logged_off: bool,
    startup_done: bool,
    /// Set when going away was refused (feature off / nothing configured); retried after 30 s.
    gave_up_at: Option<Instant>,
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
            if !self.logged_off && !self.away {
                self.logged_off = true;
                log::info!("[Audio] Headset offline seen, switching in {} s", DEBOUNCE_SECS.load(Ordering::Relaxed));
            }
            let since = *self.offline_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(DEBOUNCE_SECS.load(Ordering::Relaxed) as u64) {
                self.go_away();
            }
        } else {
            self.offline_since = None;
            self.logged_off = false;
            self.gave_up_at = None;
            // First time we see the headset on (app start): make the system use it, even if some
            // other device (e.g. a second headset) is the default right now.
            if !self.startup_done && !self.away {
                self.startup_done = true;
                if Config::load().unwrap_or_default().audio.auto_switch_audio {
                    log::info!("[Audio] Headset is on at start -> making it the default audio device");
                    self.come_back();
                }
            }
            if !self.away && self.last_remember.is_none_or(|t| t.elapsed() >= Duration::from_secs(30)) {
                self.last_remember = Some(Instant::now());
                let a = Config::load().unwrap_or_default().audio;
                if a.auto_switch_audio {
                    if let Ok(o) = system_audio::get_default_output_device() {
                        if Some(&o) != a.fallback_output_device.as_ref() { self.last_online_out = Some(o); }
                    }
                    if let Ok(i) = system_audio::get_default_input_device() {
                        if Some(&i) != a.fallback_input_device.as_ref() { self.last_online_in = Some(i); }
                    }
                }
            }
            // After 10 quick tries keep trying every 30 s: the endpoint can appear late.
            if self.back_attempts >= 10 && self.last_back_try.is_none_or(|t| t.elapsed() >= Duration::from_secs(30)) {
                self.back_attempts = 0;
            }
            if self.away
                && self.back_attempts < 10
                && self.last_back_try.is_none_or(|t| t.elapsed() >= Duration::from_secs(2))
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
        // Refused earlier (settings off / no fallback): don't hit the disk and the log on every loop pass.
        if self.gave_up_at.is_some_and(|t| t.elapsed() < Duration::from_secs(30)) {
            return;
        }
        let mut out_changed = false;
        let a = Config::load().unwrap_or_default().audio;
        if !a.auto_switch_audio {
            log::warn!("[Audio] Headset is off but 'auto switch audio' is disabled in settings");
            self.gave_up_at = Some(Instant::now());
            return;
        }
        if a.fallback_output_device.is_none() && a.fallback_input_device.is_none() {
            log::warn!("[Audio] Auto-switch is on but no fallback devices are saved");
            self.gave_up_at = Some(Instant::now());
            return;
        }
        if let Some(ref id) = a.fallback_output_device {
            if self.prev_out.is_none() {
                self.prev_out = system_audio::get_default_output_device().ok().filter(|c| c != id).or_else(|| self.last_online_out.clone());
            }
            if system_audio::get_default_output_device().is_ok_and(|c| &c == id) {
                log::info!("[Audio] Fallback output already default, not touching it");
            } else {
                log::info!("[Audio] Headset away -> fallback output {}", id);
                match system_audio::set_default_output_device(id) {
                    Ok(()) => out_changed = true,
                    Err(e) => log::warn!("[Audio] Fallback output failed: {}", e),
                }
            }
        }
        if let Some(ref id) = a.fallback_input_device {
            if self.prev_in.is_none() {
                self.prev_in = system_audio::get_default_input_device().ok().filter(|c| c != id).or_else(|| self.last_online_in.clone());
            }
            if system_audio::get_default_input_device().is_ok_and(|c| &c == id) {
                log::info!("[Audio] Fallback input already default, not touching it");
            } else if let Err(e) = { log::info!("[Audio] Headset away -> fallback input {}", id); system_audio::set_default_input_device(id) } {
                log::warn!("[Audio] Fallback input failed: {}", e);
            }
        }
        self.away = true;
        move_streams_after_switch(out_changed);
    }

    fn come_back(&mut self) -> bool {
        let a = Config::load().unwrap_or_default().audio;
        let mut ok = true;
        let mut out_changed = false;
        let out_target = a.headset_output_device.clone()
            .or_else(|| find_headset_device(true))
            .or_else(|| self.prev_out.clone());
        let in_target = a.headset_input_device.clone()
            .or_else(|| find_headset_device(false))
            .or_else(|| self.prev_in.clone());
        if out_target.is_none() {
            log::warn!("[Audio] Headset is back but its output device is unknown - choose it in Audio settings");
        }
        if let Some(id) = out_target {
            if system_audio::get_default_output_device().is_ok_and(|c| c == id) {
                log::info!("[Audio] Output already {}, not touching it", id);
            } else if let Err(e) = { log::info!("[Audio] Headset back -> output {}", id); system_audio::set_default_output_device(&id) } {
                log::warn!("[Audio] Restore output failed: {}", e);
                ok = false;
            } else {
                out_changed = true;
            }
        }
        if let Some(id) = in_target {
            if system_audio::get_default_input_device().is_ok_and(|c| c == id) {
                log::info!("[Audio] Input already {}, not touching it", id);
            } else if let Err(e) = { log::info!("[Audio] Headset back -> input {}", id); system_audio::set_default_input_device(&id) } {
                log::warn!("[Audio] Restore input failed: {}", e);
                ok = false;
            }
        }
        if ok {
            self.away = false;
            move_streams_after_switch(out_changed);
            self.prev_out = None;
            self.prev_in = None;
        }
        ok
    }
}
