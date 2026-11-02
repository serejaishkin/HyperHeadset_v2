#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn setup_logging(config: &hyperx_ngenuity_open::config::Config) {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(if config.debug_logging { log::LevelFilter::Debug } else { log::LevelFilter::Info });

    // The Windows build has no console, so the log file is always written (next to the exe,
    // falling back to the temp dir) regardless of the saved `log_to_file` setting.
    let log_to_console = config.log_to_console;
    let mut log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("hyperx-ngenuity-open.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("hyperx-ngenuity-open.log"));
    if std::fs::OpenOptions::new().create(true).append(true).open(&log_path).is_err() {
        log_path = std::env::temp_dir().join("hyperx-ngenuity-open.log");
    }
    // Keep the file small: start fresh when it grows past 5 MB.
    if std::fs::metadata(&log_path).map(|m| m.len() > 5_000_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&log_path);
    }
    let panic_path = log_path.clone();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&panic_path) {
            let _ = std::io::Write::write_all(&mut f, format!("[PANIC] {}\n", info).as_bytes());
        }
    }));
    builder.format(move |buf, record| {
        let line = format!("[{}] {} - {}\n", record.level(), record.target(), record.args());
        if log_to_console {
            let _ = std::io::Write::write_all(buf, line.as_bytes());
        }
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = std::io::Write::write_all(&mut file, line.as_bytes());
        }
        Ok(())
    });

    builder.init();
    log::info!("[Main] Log started (v{})", env!("CARGO_PKG_VERSION"));
}

use anyhow::anyhow;
use eframe::NativeOptions;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hyperx_ngenuity_open::{
    audio::debounce::DebouncedEQ,
    config::Config,
    device::{DeviceState, HyperXDevice},
    gui::HyperXApp,
    input::{self, GLOBAL_MUTE_HANDLER},
    system_audio,
    DeviceEvent,
};

use hyperx_ngenuity_open::tray::PlatformTray;

#[cfg(target_os = "windows")]
fn check_apo_available() -> bool {
    use std::path::Path;
    let paths = [
        r"C:\Program Files\EqualizerAPO\Editor.exe",
        r"C:\Program Files\EqualizerAPO\config.txt",
        r"C:\Program Files (x86)\EqualizerAPO\Editor.exe",
        r"C:\Program Files (x86)\EqualizerAPO\config.txt",
    ];
    if paths.iter().any(|p| Path::new(p).exists()) {
        return true;
    }
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if hklm.open_subkey(r"SOFTWARE\EqualizerAPO").is_ok() {
        return true;
    }
    hklm.open_subkey(r"SOFTWARE\WOW6432Node\EqualizerAPO").is_ok()
}

#[cfg(not(target_os = "windows"))]
fn check_apo_available() -> bool {
    false
}

fn live_audio_cfg() -> hyperx_ngenuity_open::config::AudioConfig {
    Config::load().unwrap_or_default().audio
}

/// Headset went away (off / unplugged): switch default devices to the saved fallback ones.
fn audio_away(away: &mut bool, prev_out: &mut Option<String>, prev_in: &mut Option<String>) {
    if *away {
        return;
    }
    let a = live_audio_cfg();
    if !a.auto_switch_audio {
        return;
    }
    if a.fallback_output_device.is_none() && a.fallback_input_device.is_none() {
        log::warn!("[Audio] Auto-switch is on but no fallback devices are saved");
        return;
    }
    if let Some(ref id) = a.fallback_output_device {
        if prev_out.is_none() {
            *prev_out = system_audio::get_default_output_device().ok().filter(|c| c != id);
        }
        log::info!("[Audio] Headset away -> fallback output {}", id);
        if let Err(e) = system_audio::set_default_output_device(id) {
            log::warn!("[Audio] Fallback output failed: {}", e);
        }
    }
    if let Some(ref id) = a.fallback_input_device {
        if prev_in.is_none() {
            *prev_in = system_audio::get_default_input_device().ok().filter(|c| c != id);
        }
        log::info!("[Audio] Headset away -> fallback input {}", id);
        if let Err(e) = system_audio::set_default_input_device(id) {
            log::warn!("[Audio] Fallback input failed: {}", e);
        }
    }
    *away = true;
}

/// Headset is back: restore headset devices (configured ones, else what was default before).
/// Returns true when finished. The headset's audio endpoint can show up a few seconds after the
/// dongle reports it online, so a failed switch keeps `away` set and the caller retries.
fn audio_back(away: &mut bool, prev_out: &mut Option<String>, prev_in: &mut Option<String>) -> bool {
    if !*away {
        return true;
    }
    let a = live_audio_cfg();
    let mut ok = true;
    if let Some(id) = a.headset_output_device.clone().or_else(|| prev_out.clone()) {
        log::info!("[Audio] Headset back -> output {}", id);
        if let Err(e) = system_audio::set_default_output_device(&id) {
            log::warn!("[Audio] Restore output failed: {}", e);
            ok = false;
        }
    }
    if let Some(id) = a.headset_input_device.clone().or_else(|| prev_in.clone()) {
        log::info!("[Audio] Headset back -> input {}", id);
        if let Err(e) = system_audio::set_default_input_device(&id) {
            log::warn!("[Audio] Restore input failed: {}", e);
            ok = false;
        }
    }
    if ok {
        *away = false;
        *prev_out = None;
        *prev_in = None;
    }
    ok
}

fn main() -> anyhow::Result<()> {
    let config = Config::load().unwrap_or_default();
    setup_logging(&config);
    hyperx_ngenuity_open::single_instance::kill_duplicates();

    let lang_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("lang")))
        .unwrap_or_else(|| std::path::PathBuf::from("lang"));
    let i18n = hyperx_ngenuity_open::i18n::I18n::new(&lang_dir, &config.language);
    if !lang_dir.join("default.lang").exists() {
        let _ = hyperx_ngenuity_open::i18n::I18n::generate_default(&lang_dir, hyperx_ngenuity_open::i18n::DEFAULT_KEYS);
    }
    hyperx_ngenuity_open::audio::voice::update_config(config.voice.clone());

    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        hyperx_ngenuity_open::config::MuteButtonMode::Standard => input::MuteButtonMode::Standard,
        hyperx_ngenuity_open::config::MuteButtonMode::MediaPlayPause => input::MuteButtonMode::MediaPlayPause,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartDouble => input::MuteButtonMode::SmartDouble,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartHold => input::MuteButtonMode::SmartHold,
        hyperx_ngenuity_open::config::MuteButtonMode::HoldPlayPause => input::MuteButtonMode::HoldPlayPause,
    });

    if let Some(keybind) = config.discord.keybind.clone() {
        GLOBAL_MUTE_HANDLER.set_keybind(Some(keybind));
    }
    GLOBAL_MUTE_HANDLER.set_keybind_enabled(
        config.discord.mode == hyperx_ngenuity_open::config::DiscordMode::Keybind,
    );
    GLOBAL_MUTE_HANDLER.set_discord_direct(config.discord.mode == hyperx_ngenuity_open::config::DiscordMode::Direct);
    GLOBAL_MUTE_HANDLER.set_discord_to_headset(config.discord.sync_from_discord);
    GLOBAL_MUTE_HANDLER.set_sidetone_play_pause(config.device.sidetone_play_pause);
    log::info!("[Main] mute_button_mode={:?} discord.mode={:?} keybind={:?} sidetone_play_pause={}", config.input.mute_button_mode, config.discord.mode, config.discord.keybind, config.device.sidetone_play_pause);

    let apo_available = check_apo_available();
    log::info!("[Main] APO available: {}", apo_available);

    let debounced_eq = Arc::new(DebouncedEQ::new(500));

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();

    let (device_tx, device_rx) = std::sync::mpsc::channel();
    let (device_cmd_tx, device_cmd_rx) = std::sync::mpsc::channel();

    #[cfg(target_os = "linux")]
    let (tray_tx, tray_rx) = std::sync::mpsc::channel();
    #[cfg(target_os = "linux")]
    let tray = PlatformTray::new(tray_tx.clone());

    #[cfg(not(target_os = "linux"))]
    let (tray_tx, tray_rx) = std::sync::mpsc::channel();
    #[cfg(not(target_os = "linux"))]
    let tray = PlatformTray::new(tray_tx.clone());


    std::thread::spawn(move || {
        let mut device = HyperXDevice::new();
        let mut last_mute: Option<bool> = None;
        let mut was_connected = false;
        let mut last_charging = false;
        let mut last_full_charge_announced = false;
        let mut last_battery_low = false;
        let mut startup_battery_announced = false;
        let mut prev_output_device: Option<String> = None;
        let mut prev_input_device: Option<String> = None;
        // Active HID polling (battery/mute/charging/sidetone requests) wakes the headset and keeps
        // it "online"; between full refreshes we only listen for events the headset sends itself.
        const FULL_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
        let mut last_full_refresh = std::time::Instant::now();
        let mut last_mute_poll = std::time::Instant::now();
        let mut seen_sidetone_events: u32 = 0;
        let mut away = false;
        let mut user_sidetone: bool = Config::load().map(|c| c.device.sidetone).unwrap_or(false);
        let mut offline_since: Option<std::time::Instant> = None;
        let mut back_attempts: u32 = 0;
        let mut last_back_try = std::time::Instant::now();
        let mut last_wireless_query = std::time::Instant::now() - Duration::from_secs(10);

        loop {
            if !device.state.connected {
                if was_connected {
                    log::warn!("[Device] Headset disconnected");
                    let _ = device_tx.send(DeviceEvent::Disconnected);
                    was_connected = false;
                    last_mute = None;
                    last_charging = false;
                    startup_battery_announced = false;
                    last_battery_low = false;
                    last_full_charge_announced = false;

                    audio_away(&mut away, &mut prev_output_device, &mut prev_input_device);
                }
                match device.connect() {
                    Ok(_) => {
                        log::info!("[Device] Headset connected");
                        let _ = device_tx.send(DeviceEvent::Connected);
                        hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Connected);
                        was_connected = true;
                        if let Err(e) = device.refresh_state() {
                            log::warn!("[Device] Initial refresh after connect failed: {}", e);
                        }
                        last_full_refresh = std::time::Instant::now();
                        last_wireless_query = std::time::Instant::now() - Duration::from_secs(10);

                    }
                    Err(e) => {
                        log::debug!("Connection failed: {}", e);
                        std::thread::sleep(Duration::from_secs(3));
                        continue;
                    }
                }
            }

            while let Ok(cmd) = device_cmd_rx.try_recv() {
                match cmd {
                    hyperx_ngenuity_open::DeviceCommand::ToggleMute => {
                        if let Err(e) = device.toggle_mute() {
                            log::warn!("Toggle mute failed: {}", e);
                        } else {
                            let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));
                            if last_mute != Some(device.state.muted) {
                                last_mute = Some(device.state.muted);
                                GLOBAL_MUTE_HANDLER.on_mute_toggled(device.state.muted);
                            }
                        }
                    }
                    hyperx_ngenuity_open::DeviceCommand::SetSidetone(enabled) => {
                        user_sidetone = enabled;
                        if let Err(e) = device.set_sidetone(enabled) {
                            log::warn!("Set sidetone failed: {}", e);
                        }
                        let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));
                    }
                    hyperx_ngenuity_open::DeviceCommand::SetVoicePrompts(enabled) => {
                        if let Err(e) = device.set_voice_prompts(enabled) {
                            log::warn!("Set voice prompts failed: {}", e);
                        }
                        let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));
                    }
                    hyperx_ngenuity_open::DeviceCommand::SetAutoShutdown(minutes) => {
                        if let Err(e) = device.set_auto_shutdown(minutes) {
                            log::warn!("Set auto shutdown failed: {}", e);
                        }
                        let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));
                    }
                }
            }

            let refresh_result = if last_full_refresh.elapsed() >= FULL_REFRESH_INTERVAL {
                last_full_refresh = std::time::Instant::now();
                device.refresh_state()
            } else {
                let ev = device.poll_events(500).map(|_| ());
                // Until the headset shows it sends mute events on its own, poll mute once a second
                // so the mute button always reaches Discord.
                if ev.is_ok()
                    && !device.mute_events_seen
                    && last_mute_poll.elapsed() >= Duration::from_secs(1)
                {
                    last_mute_poll = std::time::Instant::now();
                    device.poll_mute().map(|_| ())
                } else {
                    ev
                }
            };
            if let Err(e) = refresh_result {
                log::warn!("[Device] Refresh failed: {}", e);
                if was_connected {
                    log::warn!("[Device] Headset disconnected");
                    let _ = device_tx.send(DeviceEvent::Disconnected);
                    was_connected = false;
                    last_mute = None;
                    last_charging = false;
                    startup_battery_announced = false;
                    last_battery_low = false;
                    last_full_charge_announced = false;

                    audio_away(&mut away, &mut prev_output_device, &mut prev_input_device);
                }
                device.disconnect();
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }

            // NGenuity2 (HeadsetCloudIIWirelessDTS) ticks every 30 s; everything else is event-driven.
            if last_wireless_query.elapsed() >= FULL_REFRESH_INTERVAL {
                last_wireless_query = std::time::Instant::now();
                let _ = device.request_wireless_status();
            }
            if !device.headset_online {
                back_attempts = 0;
                // Debounce: status flickers while the headset (re)connects; only treat it as gone
                // after it has been offline for 5 s, so audio never bounces between devices.
                let since = *offline_since.get_or_insert_with(std::time::Instant::now);
                if since.elapsed() >= Duration::from_secs(5) {
                    audio_away(&mut away, &mut prev_output_device, &mut prev_input_device);
                }
            } else {
                offline_since = None;
                if away && back_attempts < 10 && last_back_try.elapsed() >= Duration::from_secs(2) {
                    last_back_try = std::time::Instant::now();
                    back_attempts += 1;
                    if audio_back(&mut away, &mut prev_output_device, &mut prev_input_device) {
                        back_attempts = 0;
                    }
                }
            }

            if GLOBAL_MUTE_HANDLER.discord_to_headset() {
                if let Some(dm) = hyperx_ngenuity_open::discord::voice::take_discord_change() {
                    if device.state.muted != dm {
                        log::info!("[Discord] Discord mute -> headset mute {}", dm);
                        let _ = device.toggle_mute();
                        if device.state.muted == dm {
                            last_mute = Some(dm);
                        }
                    }
                }
            }

            if device.sidetone_button_events != seen_sidetone_events {
                seen_sidetone_events = device.sidetone_button_events;
                GLOBAL_MUTE_HANDLER.on_sidetone_button();
                if GLOBAL_MUTE_HANDLER.sidetone_play_pause_enabled() && device.state.sidetone != user_sidetone {
                    log::info!("[Device] Restoring sidetone to {} after button press", user_sidetone);
                    let _ = device.set_sidetone(user_sidetone);
                }
            }

            if !startup_battery_announced && device.state.battery_percent > 0 {
                startup_battery_announced = true;
                if !device.state.charging {
                    hyperx_ngenuity_open::audio::voice::play(
                        hyperx_ngenuity_open::audio::voice::VoiceEvent::Battery(device.state.battery_percent)
                    );
                } else {
                    hyperx_ngenuity_open::audio::voice::play(
                        hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging
                    );
                }
                hyperx_ngenuity_open::notifications::notify_startup_battery(
                    device.state.battery_percent, device.state.charging
                );
                last_charging = device.state.charging;
            }

            log::debug!(
                "[Device] State: battery={}% charging={} muted={}",
                device.state.battery_percent,
                device.state.charging,
                device.state.muted
            );

            {
                let mut state = device_state_clone.lock().unwrap();
                *state = device.ui_state();
            }
            let _ = device_tx.send(DeviceEvent::StateChanged(device.ui_state()));

            if device.state.battery_percent <= 20 && device.state.battery_percent > 0 && !last_battery_low {
                last_battery_low = true;
                let _ = device_tx.send(DeviceEvent::BatteryLow(device.state.battery_percent));
                hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Battery(device.state.battery_percent));
                log::warn!("[Device] Battery low: {}%", device.state.battery_percent);
            }
            if device.state.battery_percent > 20 {
                last_battery_low = false;
            }

            if device.state.charging && !last_charging {
                log::info!("[Device] Headset charging");
                hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging);
            }
            if device.state.battery_percent == 100 && device.state.charging && !last_full_charge_announced {
                hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::FullCharge);
                hyperx_ngenuity_open::notifications::notify_full_charge();
                log::info!("[Device] Battery full — announced");
                last_full_charge_announced = true;
            }
            if !device.state.charging {
                last_full_charge_announced = false;
            }
            last_charging = device.state.charging;

            if last_mute != Some(device.state.muted) {
                let first_observation = last_mute.is_none();
                last_mute = Some(device.state.muted);
                if first_observation {
                    GLOBAL_MUTE_HANDLER.sync_discord_mute(device.state.muted);
                } else {
                    GLOBAL_MUTE_HANDLER.on_mute_toggled(device.state.muted);
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    });

    let discord_app_id = config.discord.direct.app_id.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut discord_ws = hyperx_ngenuity_open::discord::rpc_ws::DiscordRPCClient::new(discord_app_id);
            if let Err(e) = discord_ws.connect().await {
                log::warn!("Discord RPC WebSocket failed: {}", e);
            }
        });
    });

    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size(if config.start_in_compact_mode { egui::vec2(220.0, 200.0) } else { egui::vec2(980.0, 600.0) })
        .with_min_inner_size(if config.start_in_compact_mode { egui::vec2(200.0, 180.0) } else { egui::vec2(600.0, 400.0) });

    if let Ok(icon_data) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport_builder = viewport_builder.with_icon(std::sync::Arc::new(icon_data));
    }

    let options = NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    let initial_state = {
        let state = device_state.lock().unwrap();
        state.clone()
    };

    #[cfg(target_os = "linux")]
    let app = HyperXApp::new(config, initial_state, apo_available, debounced_eq, i18n)
        .with_tray(tray_tx)
        .with_tray_backend(tray)
        .with_device_receiver(device_rx)
        .with_device_command_sender(device_cmd_tx);

    #[cfg(not(target_os = "linux"))]
    let app = HyperXApp::new(config, initial_state, apo_available, debounced_eq, i18n)
        .with_tray(tray_tx)
        .with_tray_backend(tray)
        .with_device_receiver(device_rx)
        .with_device_command_sender(device_cmd_tx)
        .with_tray_receiver(tray_rx);

    eframe::run_native(
        "HyperX NGENUITY Open",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| anyhow!("eframe failed to run native app: {}", e))?;

    Ok(())
}
