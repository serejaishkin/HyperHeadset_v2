#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tauri::{Manager, State, Emitter, WindowEvent};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri_plugin_autostart::MacosLauncher;
use hyperx_ngenuity_open::config::Config;
use hyperx_ngenuity_open::device::{DeviceState, MultiDeviceManager, DeviceCommand};
use hyperx_ngenuity_open::input::GLOBAL_MUTE_HANDLER;
use hyperx_ngenuity_open::tray::icon::{TrayIconConfig, TrayIconMode, generate_battery_icon_rgba, generate_big_digits_rgba};

fn load_tray_png() -> (Vec<u8>, u32, u32) {
    static CACHED: std::sync::OnceLock<(Vec<u8>, u32, u32)> = std::sync::OnceLock::new();
    CACHED.get_or_init(|| {
        let bytes = include_bytes!("../../assets/tray_16.png");
        let img = image::load_from_memory(bytes).expect("tray_16.png").to_rgba8();
        let w = img.width(); let h = img.height();
        (img.into_raw(), w, h)
    }).clone()
}

#[derive(Clone)]
pub struct AppState {
    pub device_state: Arc<Mutex<DeviceState>>,
    pub all_devices: Arc<Mutex<Vec<DeviceState>>>,
    pub device_cmd_tx: mpsc::Sender<DeviceCommand>,
    pub select_device_tx: mpsc::Sender<usize>,
}

#[tauri::command]
fn get_device_state(state: State<AppState>) -> DeviceState { state.device_state.lock().unwrap().clone() }
#[tauri::command]
fn get_connected_devices(state: State<AppState>) -> Vec<DeviceState> { state.all_devices.lock().unwrap().clone() }
#[tauri::command]
fn select_device(index: usize, state: State<AppState>) -> Result<(), String> {
    state.select_device_tx.send(index).map_err(|e| e.to_string())
}
#[tauri::command]
fn get_config() -> Result<Config, String> { Ok(Config::load().unwrap_or_default()) }
#[tauri::command]
fn get_per_device_config(device_id: String) -> Result<Option<hyperx_ngenuity_open::config::PerDeviceConfig>, String> {
    let cfg = Config::load().unwrap_or_default();
    Ok(cfg.per_device.get(&device_id).cloned())
}
#[tauri::command]
fn save_per_device_config(device_id: String, per_cfg: hyperx_ngenuity_open::config::PerDeviceConfig) -> Result<(), String> {
    let mut cfg = Config::load().unwrap_or_default();
    cfg.per_device.insert(device_id, per_cfg);
    cfg.save().map_err(|e| e.to_string())
}
#[tauri::command]
fn set_custom_voice_dir(path: String) -> Result<(), String> {
    let mut cfg = Config::load().unwrap_or_default();
    cfg.custom_voice_dir = if path.is_empty() { None } else { Some(path) };
    cfg.save().map_err(|e| e.to_string())
}
#[tauri::command]
fn upload_voice_file(filename: String, data: Vec<u8>) -> Result<String, String> {
    let cfg = Config::load().unwrap_or_default();
    let base = cfg.custom_voice_dir.clone().unwrap_or_else(|| {
        std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("custom_voice"))).unwrap_or_else(|| std::path::PathBuf::from("custom_voice")).to_string_lossy().to_string()
    });
    let dir = std::path::PathBuf::from(&base);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe = std::path::Path::new(&filename).file_name().unwrap_or_default();
    let dest = dir.join(safe);
    if dest.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase() != "wav" {
        return Err("only .wav files allowed".into());
    }
    std::fs::write(&dest, &data).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}
#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    hyperx_ngenuity_open::audio::voice::update_config(config.voice.clone());
    GLOBAL_MUTE_HANDLER.set_keybind(Some(config.keybind.clone()));
    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        hyperx_ngenuity_open::config::MuteButtonMode::Standard => hyperx_ngenuity_open::input::MuteButtonMode::Standard,
        hyperx_ngenuity_open::config::MuteButtonMode::MediaPlayPause => hyperx_ngenuity_open::input::MuteButtonMode::MediaPlayPause,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartDouble => hyperx_ngenuity_open::input::MuteButtonMode::SmartDouble,
        hyperx_ngenuity_open::config::MuteButtonMode::SmartHold => hyperx_ngenuity_open::input::MuteButtonMode::SmartHold,
        hyperx_ngenuity_open::config::MuteButtonMode::HoldPlayPause => hyperx_ngenuity_open::input::MuteButtonMode::HoldPlayPause,
    });
    config.save().map_err(|e| e.to_string())
}
#[tauri::command]
fn get_tray_config() -> Result<TrayIconConfig, String> { Ok(TrayIconConfig::load_or_create()) }
#[tauri::command]
fn save_tray_config(app: tauri::AppHandle, mut config: TrayIconConfig) -> Result<(), String> {
    log::info!("[Tray] save_tray_config: mode={:?} high.fg={:?} high.outline={:?} medium.fg={:?} low.fg={:?} charging.fg={:?}",
        config.mode, config.colors.high.fg, config.colors.high.outline,
        config.colors.medium.fg, config.colors.low.fg, config.colors.charging.fg);
    config.sanitize();
    let path = TrayIconConfig::default_path();
    log::info!("[Tray] saving to {:?}", path);
    config.save(&path).map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id("main") {
        let state = app.state::<AppState>();
        let device = state.device_state.lock().unwrap().clone();
        let (rgba, w, h) = match config.mode {
            TrayIconMode::Big => generate_big_digits_rgba(device.battery_percent, device.charging, &config),
            TrayIconMode::Digits => generate_battery_icon_rgba(&config, device.battery_percent, device.charging),
        };
        match tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h))) {
            Ok(_) => log::info!("[Tray] set_icon OK after save"),
            Err(e) => log::warn!("[Tray] set_icon failed after save: {}", e),
        }
    }
    Ok(())
}
#[tauri::command]
fn get_autostart_enabled(app: tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}
#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled { app.autolaunch().enable().map_err(|e| e.to_string())?; }
    else { app.autolaunch().disable().map_err(|e| e.to_string())?; }
    Ok(())
}
#[tauri::command]
fn check_battery_voice(state: State<AppState>) -> Result<(), String> {
    let device = state.device_state.lock().unwrap().clone();
    if !device.connected { return Err("Headset is not connected".into()); }
    let config = Config::load().unwrap_or_default();
    if !config.voice.enabled || !config.voice.on_button_check { return Err("Battery voice notification is disabled in Settings → Voice".into()); }
    hyperx_ngenuity_open::audio::voice::update_config(config.voice);
    hyperx_ngenuity_open::audio::voice::play(if device.charging { hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging } else { hyperx_ngenuity_open::audio::voice::VoiceEvent::Battery(device.battery_percent) });
    Ok(())
}
#[tauri::command]
fn test_voice() -> Result<(), String> { hyperx_ngenuity_open::audio::voice::play_test(); Ok(()) }
#[tauri::command]
fn get_audio_levels() -> Result<hyperx_ngenuity_open::system_audio::AudioLevels, String> { hyperx_ngenuity_open::system_audio::get_levels().map_err(|e| e.to_string()) }
#[tauri::command]
fn set_volume(percent: u8) -> Result<(), String> { hyperx_ngenuity_open::system_audio::set_output(percent).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_mic_volume(percent: u8) -> Result<(), String> { hyperx_ngenuity_open::system_audio::set_input(percent).map_err(|e| e.to_string()) }
#[tauri::command]
fn toggle_system_mic_mute() -> Result<(), String> { hyperx_ngenuity_open::system_audio::toggle_mic_mute().map_err(|e| e.to_string()) }
#[tauri::command]
fn toggle_system_output_mute() -> Result<(), String> { hyperx_ngenuity_open::system_audio::toggle_output_mute().map_err(|e| e.to_string()) }
#[tauri::command]
fn play_pause() -> Result<(), String> { hyperx_ngenuity_open::system_audio::play_pause().map_err(|e| e.to_string()) }
#[tauri::command]
fn apply_eq(bands: [f32; 10]) -> Result<(), String> {
    #[cfg(target_os = "windows")] { return hyperx_ngenuity_open::audio::windows::apply_eq_bands(&bands).map_err(|e| e.to_string()); }
    #[cfg(target_os = "linux")] { return hyperx_ngenuity_open::audio::linux::apply_eq_bands(&bands).map_err(|e| e.to_string()); }
    #[cfg(target_os = "macos")] { return hyperx_ngenuity_open::audio::macos_eqmac::apply_eq_bands(&bands).map_err(|e| e.to_string()); }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))] { let _ = bands; Err("EQ backend is not implemented for this platform".into()) }
}
#[tauri::command]
fn toggle_mute(state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::ToggleMute).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_sidetone(enabled: bool, state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::SetSidetone(enabled)).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_voice_prompts(enabled: bool, state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::SetVoicePrompts(enabled)).map_err(|e| e.to_string()) }

/// Compact is a configured Tauri window. Never create a second WebView for it.
#[tauri::command]
fn open_compact_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") { let _ = main.hide(); }
    let window = app.get_webview_window("compact").ok_or_else(|| "Compact window is not registered by Tauri".to_string())?;
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}
#[tauri::command]
fn show_main_window_cmd(app: tauri::AppHandle) -> Result<(), String> {
    show_main_window(&app);
    Ok(())
}
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(compact) = app.get_webview_window("compact") { let _ = compact.hide(); }
    if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.unminimize(); let _ = window.set_focus(); }
}
fn publish_disconnected(app: &tauri::AppHandle, state: &Arc<Mutex<DeviceState>>) {
    { let mut st = state.lock().unwrap(); *st = DeviceState::default(); }
    let _ = app.emit("device-disconnected", ()); let _ = app.emit("device-state", DeviceState::default());
    if let Some(tray) = app.tray_by_id("main") {
        let icon_config = TrayIconConfig::load_or_create();
        let (rgba, w, h) = match icon_config.mode {
            TrayIconMode::Big => generate_big_digits_rgba(0, false, &icon_config),
            TrayIconMode::Digits => generate_battery_icon_rgba(&icon_config, 0, false),
        };
        if let Err(e) = tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h))) {
            log::warn!("[Tray] disconnected set_icon failed: {}", e);
        }
        let _ = tray.set_tooltip(Some("HyperHeadsetv2 — нет подключения"));
    }
}

fn main() {
    env_logger::init();
    GLOBAL_MUTE_HANDLER.set_keybind(Some("F20".to_string()));
    hyperx_ngenuity_open::audio::voice::update_config(Config::load().unwrap_or_default().voice);
    let device_state = Arc::new(Mutex::new(DeviceState::default())); let device_state_clone = device_state.clone();
    let all_devices = Arc::new(Mutex::new(Vec::<DeviceState>::new())); let all_devices_clone = all_devices.clone();
    let (device_cmd_tx, device_cmd_rx) = mpsc::channel();
    let (select_device_tx, select_device_rx) = mpsc::channel::<usize>();
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(AppState { device_state: device_state_clone, all_devices: all_devices_clone, device_cmd_tx: device_cmd_tx.clone(), select_device_tx: select_device_tx.clone() })
        .setup(move |app| {
            let app_handle = app.handle().clone(); let device_state_inner = device_state.clone();
            let menu = Menu::new(&app_handle)?;
            let open_i = MenuItem::new(&app_handle, "Открыть", true, None::<&str>)?;
            let toggle_i = MenuItem::new(&app_handle, "Мьют", true, None::<&str>)?;
            let compact_i = MenuItem::new(&app_handle, "Компактное окно", true, None::<&str>)?;
            let quit_i = MenuItem::new(&app_handle, "Выход", true, None::<&str>)?;
            menu.append(&open_i)?; menu.append(&toggle_i)?; menu.append(&compact_i)?; menu.append(&PredefinedMenuItem::separator(&app_handle)?)?; menu.append(&quit_i)?;
            let (tray_rgba, tray_w, tray_h) = load_tray_png();
            let tray_icon = tauri::image::Image::new(&tray_rgba, tray_w, tray_h);
            let _tray = tauri::tray::TrayIconBuilder::with_id("main").icon(tray_icon).menu(&menu).tooltip("HyperHeadsetv2 — поиск наушников...")
                .on_menu_event({ let device_cmd_tx = device_cmd_tx.clone(); move |app, event| {
                    if event.id == open_i.id() { show_main_window(app); }
                    else if event.id == toggle_i.id() { let _ = device_cmd_tx.send(DeviceCommand::ToggleMute); }
                    else if event.id == compact_i.id() { let _ = open_compact_window(app.clone()); }
                    else if event.id == quit_i.id() { app.exit(0); }
                }})
                .on_tray_icon_event(|tray, event| { if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event { show_main_window(&tray.app_handle()); } })
                .build(&app_handle)?;
            if let Some(main_window) = app.get_webview_window("main") {
                let window_for_close = main_window.clone();
                main_window.on_window_event(move |event| { if let WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window_for_close.hide(); } });
            }
            if let Some(compact_window) = app.get_webview_window("compact") {
                let app_for_close = app_handle.clone();
                compact_window.on_window_event(move |event| { if let WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); show_main_window(&app_for_close); } });
            }
            let app_handle_device = app_handle.clone();
            let all_devices_inner = all_devices.clone();
            thread::spawn(move || {
                log::info!("[Device] Device thread started");
                log::info!("[Device] Creating MultiDeviceManager...");
                let mut manager = loop {
                    match MultiDeviceManager::new() {
                        Some(m) => { log::info!("[Device] MultiDeviceManager created, entering loop"); break m; }
                        None => { log::warn!("[Device] Failed to create MultiDeviceManager, retrying in 5s..."); thread::sleep(Duration::from_secs(5)); }
                    }
                };
                let mut was_connected = false; let mut heartbeat_failures = 0u32;
                let mut last_charging = false; let mut last_battery_low = false; let mut last_full_charge = false; let mut startup_announced = false;
                loop {
                    while let Ok(idx) = select_device_rx.try_recv() {
                        if idx < manager.devices.len() {
                            manager.active_index = idx;
                            let st = manager.active_state();
                            { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); }
                            let _ = app_handle_device.emit("device-state", st.clone());
                            let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                            { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); }
                            let _ = app_handle_device.emit("devices-list", all);
                            log::info!("[Device] Switched active device to index {}", idx);
                        }
                    }
                    if manager.devices.is_empty() || !manager.devices.iter().any(|d| d.state.connected) {
                        log::info!("[Device] Devices empty or disconnected, calling scan_and_connect...");
                        match manager.scan_and_connect() {
                            Ok(_) => {
                                was_connected = true; heartbeat_failures = 0; startup_announced = false; last_charging = false; last_battery_low = false; last_full_charge = false;
                                let st = manager.active_state();
                                { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); }
                                let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                                { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); }
                                let _ = app_handle_device.emit("device-connected", ()); let _ = app_handle_device.emit("device-state", st.clone()); let _ = app_handle_device.emit("devices-list", all);
                            }
                            Err(e) => { if was_connected { was_connected = false; publish_disconnected(&app_handle_device, &device_state_inner); { let mut a = all_devices_inner.lock().unwrap(); *a = Vec::new(); } let _ = app_handle_device.emit("devices-list", Vec::<DeviceState>::new()); } log::debug!("[Device] scan: {}", e); thread::sleep(Duration::from_secs(2)); continue; }
                        }
                    }
                    while let Ok(cmd) = device_cmd_rx.try_recv() {
                        if let Some(dev) = manager.active_device() {
                            let result = match cmd { DeviceCommand::ToggleMute => dev.toggle_mute(), DeviceCommand::SetSidetone(enabled) => dev.set_sidetone(enabled), DeviceCommand::SetVoicePrompts(enabled) => dev.set_voice_prompts(enabled) };
                            if let Err(e) = &result { let _ = app_handle_device.emit("device-command-error", e.to_string()); }
                            let st = manager.active_state();
                            { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); } let _ = app_handle_device.emit("device-state", st);
                            let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                            { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); } let _ = app_handle_device.emit("devices-list", all);
                        }
                    }
                    let mut refresh_ok = true;
                    for dev in manager.devices.iter_mut() {
                        if let Err(e) = dev.refresh_state() {
                            log::debug!("[Device] refresh failed for {}: {}", dev.state.device_id, e);
                            refresh_ok = false;
                        }
                    }
                    if refresh_ok {
                        heartbeat_failures = 0;
                        let st = manager.active_state();
                        { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); } let _ = app_handle_device.emit("device-state", st.clone());
                        let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                        { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); } let _ = app_handle_device.emit("devices-list", all.clone());
                        if !startup_announced && st.battery_percent > 0 { startup_announced = true; let v = if st.charging { hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging } else { hyperx_ngenuity_open::audio::voice::VoiceEvent::Battery(st.battery_percent) }; hyperx_ngenuity_open::audio::voice::play(v); last_charging = st.charging; }
                        if st.battery_percent <= 20 && st.battery_percent > 0 && !last_battery_low { last_battery_low = true; hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::LowBattery); }
                        if st.battery_percent > 20 { last_battery_low = false; }
                        if st.charging && !last_charging { hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging); }
                        if st.battery_percent == 100 && st.charging && !last_full_charge { last_full_charge = true; hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::FullCharge); }
                        if !st.charging { last_full_charge = false; } last_charging = st.charging;
                        if let Some(tray) = app_handle_device.tray_by_id("main") {
                            let icon_config = TrayIconConfig::load_or_create();
                            log::debug!("[Tray] update: mode={:?} bat={} charging={} high.fg={:?} high.outline={:?}",
                                icon_config.mode, st.battery_percent, st.charging,
                                icon_config.colors.high.fg, icon_config.colors.high.outline);
                            let (rgba, w, h) = match icon_config.mode {
                                TrayIconMode::Big => generate_big_digits_rgba(st.battery_percent, st.charging, &icon_config),
                                TrayIconMode::Digits => generate_battery_icon_rgba(&icon_config, st.battery_percent, st.charging),
                            };
                            if let Err(e) = tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h))) {
                                log::warn!("[Tray] set_icon failed: {}", e);
                            }
                            let tooltip = if st.charging { format!("HyperHeadsetv2\n⚡ {}% — {}", st.battery_percent, st.name) } else { format!("HyperHeadsetv2\n🔋 {}% — {}", st.battery_percent, st.name) };
                            let _ = tray.set_tooltip(Some(&tooltip));
                        }
                    } else {
                        heartbeat_failures += 1; let enumerated = manager.is_enumerated(); log::warn!("[HID] Heartbeat failed {}/5; enumeration={}", heartbeat_failures, enumerated); if heartbeat_failures >= 5 {
                            for d in manager.devices.iter_mut() { d.disconnect(); }
                            publish_disconnected(&app_handle_device, &device_state_inner);
                            { let mut a = all_devices_inner.lock().unwrap(); *a = Vec::new(); }
                            let _ = app_handle_device.emit("devices-list", Vec::<DeviceState>::new());
                            thread::sleep(Duration::from_secs(3));
                            heartbeat_failures = 0;
                        }
                    }
                    thread::sleep(Duration::from_millis(500));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_device_state, get_connected_devices, select_device, get_config, save_config, get_per_device_config, save_per_device_config, set_custom_voice_dir, upload_voice_file, get_tray_config, save_tray_config, get_autostart_enabled, set_autostart_enabled, check_battery_voice, test_voice, get_audio_levels, set_volume, set_mic_volume, toggle_system_mic_mute, toggle_system_output_mute, play_pause, apply_eq, toggle_mute, set_sidetone, set_voice_prompts, open_compact_window, show_main_window_cmd])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
