#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use anyhow::anyhow;
use eframe::NativeOptions;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hyperx_ngenuity_open::{
    audio::{AudioManager, DebouncedEQ},
    config::Config,
    device::{DeviceState, HyperXDevice},
    gui::HyperXApp,
    input::{self, GLOBAL_MUTE_HANDLER},
    tray::{PlatformTray, TrayCommand},
    DeviceEvent,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Load config
    let config = Config::load().unwrap_or_default();

    // Apply config to input handler
    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        hyperx_ngenuity_open::config::MuteButtonMode::Standard => input::MuteButtonMode::Standard,
        hyperx_ngenuity_open::config::MuteButtonMode::MediaPlayPause => input::MuteButtonMode::MediaPlayPause,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartDouble => input::MuteButtonMode::SmartDouble,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartHold => input::MuteButtonMode::SmartHold,
    });

    // Check EQ backend availability
    let audio_manager = AudioManager::new();
    let apo_available = audio_manager.backend().is_available();

    // Setup debounced EQ (500ms delay)
    let debounced_eq = Arc::new(DebouncedEQ::new(500));
    let debounced_eq_worker = debounced_eq.clone();

    #[cfg(target_os = "windows")]
    {
        let backend = AudioManager::new();
        debounced_eq_worker.spawn_worker(move |bands| {
            let _ = backend.backend().apply_eq(&bands);
        });
    }

    // Shared device state
    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();

    // Channel for device events -> GUI
    let (device_tx, device_rx) = std::sync::mpsc::channel::<DeviceEvent>();
    let device_tx_gui = device_tx.clone();

    // Tray channel
    let (tray_tx, tray_rx) = std::sync::mpsc::channel::<TrayCommand>();

    // Create tray
    let tray = PlatformTray::new(tray_tx.clone());

    // Device communication thread with reconnect handling
    std::thread::spawn(move || {
        let mut device = HyperXDevice::new();
        let mut last_mute: Option<bool> = None;
        let mut was_connected = false;

        loop {
            // Connection loop
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

            // Refresh loop
            if let Err(e) = device.refresh_state() {
                log::warn!("Refresh failed: {}", e);
                device.state.connected = false;
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }

            // Update shared state
            {
                let mut state = device_state_clone.lock().unwrap();
                *state = device.state.clone();
            }

            // Send state update to GUI
            let _ = device_tx.send(DeviceEvent::StateChanged(device.state.clone()));

            // Battery low warning
            if device.state.battery_percent <= 15 && device.state.battery_percent > 0 {
                let _ = device_tx.send(DeviceEvent::BatteryLow(device.state.battery_percent));
            }

            // Handle mute change -> Input handler -> Discord sync
            if last_mute != Some(device.state.muted) {
                last_mute = Some(device.state.muted);
                GLOBAL_MUTE_HANDLER.on_mute_toggled(device.state.muted);
            }

            std::thread::sleep(Duration::from_secs(3));
        }
    });

    // Discord WebSocket RPC thread (for bidirectional sync)
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

    // Global hotkey capture thread (Windows only)
    #[cfg(target_os = "windows")]
    {
        let hotkey_capture = Arc::new(hyperx_ngenuity_open::hotkey::GlobalHotkeyCapture::new());
        hyperx_ngenuity_open::hotkey::windows::spawn_capture_thread(hotkey_capture);
    }

    // GUI with device event handling
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    let initial_state = {
        let state = device_state.lock().unwrap();
        state.clone()
    };

    let mut app = HyperXApp::new(config, initial_state, apo_available, debounced_eq)
        .with_tray(tray_tx)
        .with_tray_backend(tray)
        .with_device_receiver(device_rx);

    eframe::run_native(
        "HyperX NGENUITY Open",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| anyhow!("eframe failed to run native app: {}", e))?;

    Ok(())
}
