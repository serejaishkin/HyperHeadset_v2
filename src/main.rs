#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

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

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = Config::load().unwrap_or_default();

    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        hyperx_ngenuity_open::config::MuteButtonMode::Standard => input::MuteButtonMode::Standard,
        hyperx_ngenuity_open::config::MuteButtonMode::MediaPlayPause => input::MuteButtonMode::MediaPlayPause,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartDouble => input::MuteButtonMode::SmartDouble,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartHold => input::MuteButtonMode::SmartHold,
    });

    if let Some(keybind) = config.discord.keybind.clone() {
        GLOBAL_MUTE_HANDLER.set_keybind(Some(keybind));
    }

    let apo_available = false;
    let debounced_eq = Arc::new(DebouncedEQ::new(500));

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();

    let (device_tx, device_rx) = std::sync::mpsc::channel::<DeviceEvent>();
    let _device_tx_gui = device_tx.clone();

    let (device_cmd_tx, device_cmd_rx) = std::sync::mpsc::channel::<hyperx_ngenuity_open::DeviceCommand>();
    let (tray_tx, tray_rx) = std::sync::mpsc::channel::<TrayCommand>();

    let tray = PlatformTray::new(tray_tx.clone());

    std::thread::spawn(move || {
        let mut device = HyperXDevice::new();
        let mut last_mute: Option<bool> = None;
        let mut was_connected = false;
        let mut last_charging = false;
        let mut last_battery_low = false;

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
                        was_connected = true;
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
                        log::info!("[Device] ToggleMute command received");
                        if let Err(e) = device.toggle_mute() {
                            log::warn!("Toggle mute failed: {}", e);
                        } else {
                            // toggle_mute() УЖЕ обновил device.state.muted
                            log::info!("[Device] Mute toggled to: {}", device.state.muted);
                            
                            // МГНОВЕННО шлём состояние в GUI/трей
                            let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));
                            
                            // Синхронизируем с Discord
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
                log::warn!("Refresh failed: {}", e);
                device.state.connected = false;
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }

            {
                let mut state = device_state_clone.lock().unwrap();
                *state = device.state.clone();
            }
            let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));

            if device.state.battery_percent <= 15 && device.state.battery_percent > 0 && !last_battery_low {
                last_battery_low = true;
                let _ = device_tx.send(DeviceEvent::BatteryLow(device.state.battery_percent));
                log::warn!("[Device] Battery low: {}%", device.state.battery_percent);
            }
            if device.state.battery_percent > 20 {
                last_battery_low = false;
            }

            if device.state.charging && !last_charging {
                log::info!("[Device] Headset charging");
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
            let mut discord_ws = hyperx_ngenuity_open::discord::rpc_ws::DiscordRPCClient::new(discord_app_id);
            if let Err(e) = discord_ws.connect().await {
                log::warn!("Discord RPC WebSocket failed: {}", e);
            }
        });
    });

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
        .with_tray_receiver(tray_rx)
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