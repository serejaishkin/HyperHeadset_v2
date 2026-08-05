// #![cfg_attr(target_os = "windows", windows_subsystem = "windows")] temporarily removed for debug

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
    tray::{PlatformTray, TrayCommand},
    DeviceEvent,
};

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};
#[cfg(target_os = "windows")]
use windows::core::PCWSTR;

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

fn setup_logging() {
    use std::fs::OpenOptions;
    use std::io::Write;
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("hyperx-ngenuity-open.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("hyperx-ngenuity-open.log"));
    let _ = OpenOptions::new().create(true).append(true).open(&log_path);
    env_logger::Builder::from_default_env()
        .format(move |buf, record| {
            let line = format!("[{}] {} - {}\n", record.level(), record.target(), record.args());
            let _ = std::io::Write::write_all(buf, line.as_bytes());
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = std::io::Write::write_all(&mut file, line.as_bytes());
            }
            Ok(())
        })
        .init();
}

fn main() -> anyhow::Result<()> {
    setup_logging();

    let config = Config::load().unwrap_or_default();
    hyperx_ngenuity_open::audio::voice::update_config(config.voice.clone());

    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        hyperx_ngenuity_open::config::MuteButtonMode::Standard => input::MuteButtonMode::Standard,
        hyperx_ngenuity_open::config::MuteButtonMode::MediaPlayPause => input::MuteButtonMode::MediaPlayPause,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartDouble => input::MuteButtonMode::SmartDouble,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartHold => input::MuteButtonMode::SmartHold,
    });

    if let Some(keybind) = config.discord.keybind.clone() {
        GLOBAL_MUTE_HANDLER.set_keybind(Some(keybind));
    }

    let apo_available = check_apo_available();
    log::info!("[Main] APO available: {}", apo_available);

    let debounced_eq = Arc::new(DebouncedEQ::new(500));

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();

    let (device_tx, device_rx) = std::sync::mpsc::channel::<DeviceEvent>();
    let (device_cmd_tx, device_cmd_rx) = std::sync::mpsc::channel::<hyperx_ngenuity_open::DeviceCommand>();
    let (tray_tx, tray_rx) = std::sync::mpsc::channel::<TrayCommand>();

    let tray = PlatformTray::new(tray_tx.clone());

    #[cfg(target_os = "windows")]
    {
        let device_cmd_tx_tray = device_cmd_tx.clone();
        std::thread::spawn(move || {
            while let Ok(cmd) = tray_rx.recv() {
                match cmd {
                    TrayCommand::ShowWindow => {
                        let title: Vec<u16> = "HyperX NGENUITY Open\0".encode_utf16().collect();
                        unsafe {
                            match FindWindowW(None, PCWSTR(title.as_ptr())) {
                                Ok(hwnd) if !hwnd.0.is_null() => {
                                    let _ = ShowWindow(hwnd, SW_RESTORE);
                                    let _ = SetForegroundWindow(hwnd);
                                }
                                Ok(_) => log::warn!("[TrayThread] HWND is null"),
                                Err(e) => log::warn!("[TrayThread] FindWindow failed: {:?}", e),
                            }
                        }
                    }
                    TrayCommand::ToggleMute => {
                        let _ = device_cmd_tx_tray.send(hyperx_ngenuity_open::DeviceCommand::ToggleMute);
                    }
                    TrayCommand::Quit => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        });
    }

    std::thread::spawn(move || {
        let mut device = HyperXDevice::new();
        let mut last_mute: Option<bool> = None;
        let mut was_connected = false;
        let mut last_charging = false;
        let mut last_battery_low = false;
        let mut error_count = 0;
        let mut startup_battery_announced = false;

        loop {
            if !device.state.connected {
                if was_connected {
                    log::warn!("[Device] Headset disconnected");
                    let _ = device_tx.send(DeviceEvent::Disconnected);
                    was_connected = false;
                }
                match device.connect() {
                    Ok(_) => {
                        log::info!("[Device] Headset connected");
                        let _ = device_tx.send(DeviceEvent::Connected);
                        hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Connected);
                        was_connected = true;
                        error_count = 0;
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
                    device.disconnect();
                    error_count = 0;
                }
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            error_count = 0;

            if !startup_battery_announced && device.state.battery_percent > 0 {
                startup_battery_announced = true;
                hyperx_ngenuity_open::audio::voice::play(
                    hyperx_ngenuity_open::audio::voice::VoiceEvent::Battery(device.state.battery_percent)
                );
                hyperx_ngenuity_open::notifications::notify_startup_battery(
                    device.state.battery_percent, device.state.charging
                );
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
                hyperx_ngenuity_open::notifications::notify_low_battery(device.state.battery_percent);
                log::warn!("[Device] Battery low: {}%", device.state.battery_percent);
            }
            if device.state.battery_percent > 20 {
                last_battery_low = false;
            }

            if device.state.charging && !last_charging {
                log::info!("[Device] Headset charging");
                hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging);
            }
            if device.state.battery_percent == 100 && device.state.charging && !last_charging {
                hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::FullCharge);
                hyperx_ngenuity_open::notifications::notify_full_charge();
                log::info!("[Device] Battery full");
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
    if !discord_app_id.is_empty() {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let discord_ws = hyperx_ngenuity_open::discord::rpc_ws::DiscordRPCClient::new(discord_app_id);
                if let Err(e) = discord_ws.connect().await {
                    log::warn!("Discord RPC WebSocket failed: {}", e);
                }
            });
        });
    }

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    let initial_state = {
        let state = device_state.lock().unwrap();
        state.clone()
    };

    let app = HyperXApp::new(config, initial_state, apo_available, debounced_eq)
        .with_tray(tray_tx)
        .with_tray_backend(tray)
        .with_device_receiver(device_rx)
        .with_device_command_sender(device_cmd_tx);

    #[cfg(target_os = "linux")]
    let app = app.with_tray_receiver(tray_rx);

    eframe::run_native(
        "HyperX NGENUITY Open",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| anyhow!("eframe failed to run native app: {}", e))?;

    Ok(())
}
