#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(not(target_os = "linux"))]
mod tray_manager;
#[cfg(not(target_os = "linux"))]
mod tray_battery_icon_state;

fn setup_logging(config: &hyperx_ngenuity_open::config::Config) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(if config.debug_logging { log::LevelFilter::Debug } else { log::LevelFilter::Info });

    let log_to_console = config.log_to_console;
    let log_to_file = config.log_to_file;

    if log_to_file {
        let log_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("hyperx-ngenuity-open.log")))
            .unwrap_or_else(|| std::path::PathBuf::from("hyperx-ngenuity-open.log"));
        let _ = OpenOptions::new().create(true).append(true).open(&log_path);
        builder.format(move |buf, record| {
            let line = format!("[{}] {} - {}\n", record.level(), record.target(), record.args());
            if log_to_console {
                let _ = std::io::Write::write_all(buf, line.as_bytes());
            }
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = std::io::Write::write_all(&mut file, line.as_bytes());
            }
            Ok(())
        });
    } else if !log_to_console {
        builder.filter_level(log::LevelFilter::Off);
    }

    builder.init();
}

use anyhow::anyhow;
use eframe::NativeOptions;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hyperx_ngenuity_open::{
    audio::DebouncedEQ,
    config::Config,
    device::{DeviceState, HyperXDevice},
    gui::HyperXApp,
    input::{self, GLOBAL_MUTE_HANDLER},
    DeviceEvent,
};

#[cfg(target_os = "windows")]
fn check_apo_available() -> bool {
    use std::path::Path;
    let paths = [
        r"C:\\Program Files\\EqualizerAPO\\Editor.exe",
        r"C:\\Program Files\\EqualizerAPO\\config.txt",
        r"C:\\Program Files (x86)\\EqualizerAPO\\Editor.exe",
        r"C:\\Program Files (x86)\\EqualizerAPO\\config.txt",
    ];
    if paths.iter().any(|p| Path::new(p).exists()) {
        return true;
    }
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if hklm.open_subkey(r"SOFTWARE\\EqualizerAPO").is_ok() {
        return true;
    }
    hklm.open_subkey(r"SOFTWARE\\WOW6432Node\\EqualizerAPO").is_ok()
}

#[cfg(not(target_os = "windows"))]
fn check_apo_available() -> bool {
    false
}

fn main() -> anyhow::Result<()> {
    let config = Config::load().unwrap_or_default();
    setup_logging(&config);

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

    let apo_available = check_apo_available();
    log::info!("[Main] APO available: {}", apo_available);

    let debounced_eq = Arc::new(DebouncedEQ::new(500));

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();

    // Tray battery icon (Windows/macOS only)
    #[cfg(not(target_os = "linux"))]
    {
        let tray_state = Arc::new(Mutex::new(None::<DeviceState>));
        let tray_state_clone = Arc::clone(&tray_state);
        tray_manager::spawn_tray_battery_thread(tray_state_clone, config.tray.clone());
    }

    let (device_tx, device_rx) = std::sync::mpsc::channel();
    let (device_cmd_tx, device_cmd_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let mut device = HyperXDevice::new();
        let mut last_mute: Option<bool> = None;
        let mut was_connected = false;
        let mut last_charging = false;
        let mut last_full_charge_announced = false;
        let mut last_battery_low = false;
        let mut error_count = 0;
        let mut startup_battery_announced = false;

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
                    let _ = device_tx.send(DeviceEvent::StateChanged(DeviceState::default()));
                }
                match device.connect() {
                    Ok(_) => {
                        log::info!("[Device] Headset connected");
                        let _ = device_tx.send(DeviceEvent::Connected);
                        hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Connected);
                        was_connected = true;
                        error_count = 0;
                        if let Err(e) = device.refresh_state() {
                            log::warn!("[Device] Initial refresh after connect failed: {}", e);
                        }
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
                        if let Err(e) = device.set_sidetone(enabled) {
                            log::warn!("Set sidetone failed: {}", e);
                        }
                        let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));
                    }
                    hyperx_ngenuity_open::DeviceCommand::SetVoicePrompts(_enabled) => {}
                }
            }

            if let Err(e) = device.refresh_state() {
                error_count += 1;
                log::warn!("[Device] Refresh failed ({}/3): {}", error_count, e);
                if error_count >= 3 {
                    log::warn!("[Device] Too many errors, disconnecting");
                    let _ = device_tx.send(DeviceEvent::Disconnected);
                    device.disconnect();
                    error_count = 0;
                }
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            error_count = 0;

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

            log::info!(
                "[Device] State: battery={}% charging={} muted={}",
                device.state.battery_percent,
                device.state.charging,
                device.state.muted
            );

            {
                let mut state = device_state_clone.lock().unwrap();
                *state = device.state.clone();
            }
            let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));

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
                last_mute = Some(device.state.muted);
                GLOBAL_MUTE_HANDLER.on_mute_toggled(device.state.muted);
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    });

    let discord_app_id = config.discord.direct.app_id.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let discord_ws = hyperx_ngenuity_open::discord::rpc_ws::DiscordRPCClient::new(discord_app_id);
            if let Err(e) = discord_ws.connect().await {
                log::warn!("Discord RPC WebSocket failed: {}", e);
            }
        });
    });

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(if config.start_in_compact_mode { egui::vec2(220.0, 200.0) } else { egui::vec2(980.0, 600.0) })
            .with_min_inner_size(if config.start_in_compact_mode { egui::vec2(200.0, 180.0) } else { egui::vec2(600.0, 400.0) }),
        ..Default::default()
    };

    let initial_state = {
        let state = device_state.lock().unwrap();
        state.clone()
    };

    let app = HyperXApp::new(config, initial_state, apo_available, debounced_eq, i18n)
        .with_device_receiver(device_rx)
        .with_device_command_sender(device_cmd_tx);

    eframe::run_native(
        "HyperX NGENUITY Open",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| anyhow!("eframe failed to run native app: {}", e))?;

    Ok(())
}
