#![cfg_attr(windows, windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{Manager, State, Emitter, WindowEvent};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri_plugin_autostart::MacosLauncher;
use hyperx_ngenuity_open::config::Config;
use hyperx_ngenuity_open::device::{DeviceState, MultiDeviceManager, DeviceCommand};
use hyperx_ngenuity_open::audio_switch::AUDIO_SWITCH;
use hyperx_ngenuity_open::input::GLOBAL_MUTE_HANDLER;
use hyperx_ngenuity_open::tray::icon::{OFFLINE_PERCENT, TrayIconConfig, TrayIconMode, generate_battery_gauge_rgba, generate_big_digits_rgba};
use hyperx_ngenuity_open::input::MuteButtonMode as InputMuteButtonMode;
use hyperx_ngenuity_open::config::MuteButtonMode;

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
static TRAY_MENU_ITEMS: Mutex<Option<[tauri::menu::MenuItem<tauri::Wry>; 4]>> = Mutex::new(None);

/// Re-apply tray menu labels in the current UI language (called after settings are saved).
fn refresh_tray_menu_labels() {
    if let Some(items) = TRAY_MENU_ITEMS.lock().unwrap().as_ref() {
        let t = |ru: &str, en: &str| hyperx_ngenuity_open::tr_pick(ru, en);
        let _ = items[0].set_text(t("Открыть", "Open"));
        let _ = items[1].set_text(t("Мьют", "Mute"));
        let _ = items[2].set_text(t("Компактное окно", "Compact window"));
        let _ = items[3].set_text(t("Выход", "Quit"));
    }
}

#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    hyperx_ngenuity_open::audio::voice::update_config(config.voice.clone());
    GLOBAL_MUTE_HANDLER.set_keybind(Some(config.keybind.clone()));
    if let Some(discord_keybind) = config.discord.keybind.clone() {
        GLOBAL_MUTE_HANDLER.set_keybind(Some(discord_keybind));
    }
    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        MuteButtonMode::Standard => InputMuteButtonMode::Standard,
        MuteButtonMode::MediaPlayPause => InputMuteButtonMode::MediaPlayPause,
        MuteButtonMode::SmartDouble => InputMuteButtonMode::SmartDouble,
        MuteButtonMode::SmartHold => InputMuteButtonMode::SmartHold,
        MuteButtonMode::HoldPlayPause => InputMuteButtonMode::HoldPlayPause,
    });
    apply_input_config(&config);
    let res = config.save().map_err(|e| e.to_string());
    refresh_tray_menu_labels();
    res
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
        let (pct, chg) = if device.connected { (device.battery_percent, device.charging) } else { (OFFLINE_PERCENT, false) };
        let (rgba, w, h) = match config.mode {
            TrayIconMode::Big => generate_big_digits_rgba(pct, chg, &config.for_tray()),
            TrayIconMode::Battery => generate_battery_gauge_rgba(&config.for_tray(), pct, chg),
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
fn get_audio_devices() -> Result<Vec<hyperx_ngenuity_open::system_audio::AudioDevice>, String> { hyperx_ngenuity_open::system_audio::get_audio_devices().map_err(|e| e.to_string()) }
#[tauri::command]
fn get_default_audio_devices() -> serde_json::Value {
    serde_json::json!({
        "output": hyperx_ngenuity_open::system_audio::get_default_output_device().ok(),
        "input": hyperx_ngenuity_open::system_audio::get_default_input_device().ok(),
    })
}
#[tauri::command]
fn set_default_output_device(device_id: String) -> Result<(), String> { hyperx_ngenuity_open::system_audio::set_default_output_device(&device_id).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_default_input_device(device_id: String) -> Result<(), String> { hyperx_ngenuity_open::system_audio::set_default_input_device(&device_id).map_err(|e| e.to_string()) }
#[tauri::command]
fn switch_to_fallback_audio(state: State<AppState>) -> Result<(), String> {
    let device = state.device_state.lock().unwrap().clone();
    if !device.connected {
        let config = Config::load().map_err(|e| e.to_string())?;
        if config.audio.auto_switch_audio {
            if let Some(ref output_id) = config.audio.fallback_output_device {
                log::info!("[Audio] Switching to fallback output: {}", output_id);
                hyperx_ngenuity_open::system_audio::set_default_output_device(output_id).map_err(|e| e.to_string())?;
            }
            if let Some(ref input_id) = config.audio.fallback_input_device {
                log::info!("[Audio] Switching to fallback input: {}", input_id);
                hyperx_ngenuity_open::system_audio::set_default_input_device(input_id).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
#[tauri::command]
fn apply_eq(bands: [f32; 10]) -> Result<(), String> {
    #[cfg(target_os = "windows")] { return hyperx_ngenuity_open::audio::windows::apply_eq_bands(&bands).map_err(|e| e.to_string()); }
    #[cfg(target_os = "linux")] { return hyperx_ngenuity_open::audio::linux::apply_eq_bands(&bands).map_err(|e| e.to_string()); }
    #[cfg(target_os = "macos")] { return hyperx_ngenuity_open::audio::macos_eqmac::apply_eq_bands(&bands).map_err(|e| e.to_string()); }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))] { let _ = bands; Err("EQ backend is not implemented for this platform".into()) }
}
#[tauri::command]
fn eq_backend_status() -> serde_json::Value {
    #[cfg(target_os = "windows")] { return serde_json::json!({"os":"windows","installed":hyperx_ngenuity_open::audio::windows::is_apo_installed()}); }
    #[cfg(target_os = "linux")] { return serde_json::json!({"os":"linux","installed":hyperx_ngenuity_open::audio::linux::is_installed()}); }
    #[cfg(target_os = "macos")] { return serde_json::json!({"os":"macos","installed":hyperx_ngenuity_open::audio::macos_eqmac::is_installed()}); }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))] { serde_json::json!({"os":"other","installed":false}) }
}
#[tauri::command]
fn toggle_mute(state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::ToggleMute).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_sidetone(enabled: bool, state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::SetSidetone(enabled)).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_voice_prompts(enabled: bool, state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::SetVoicePrompts(enabled)).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_auto_shutdown(minutes: u32, state: State<AppState>) -> Result<(), String> { state.device_cmd_tx.send(DeviceCommand::SetAutoShutdown(minutes)).map_err(|e| e.to_string()) }
#[tauri::command]
fn set_mute_button_mode(mode: String) -> Result<(), String> {
    let parsed_mode = match mode.as_str() {
        "standard" => MuteButtonMode::Standard,
        "media_play_pause" => MuteButtonMode::MediaPlayPause,
        "smart_double" => MuteButtonMode::SmartDouble,
        "smart_hold" => MuteButtonMode::SmartHold,
        "hold_play_pause" => MuteButtonMode::HoldPlayPause,
        _ => return Err("Invalid mute button mode".to_string()),
    };
    GLOBAL_MUTE_HANDLER.set_mode(match parsed_mode {
        MuteButtonMode::Standard => InputMuteButtonMode::Standard,
        MuteButtonMode::MediaPlayPause => InputMuteButtonMode::MediaPlayPause,
        MuteButtonMode::SmartDouble => InputMuteButtonMode::SmartDouble,
        MuteButtonMode::SmartHold => InputMuteButtonMode::SmartHold,
        MuteButtonMode::HoldPlayPause => InputMuteButtonMode::HoldPlayPause,
    });
    // Also update keybind from discord config
    if let Ok(config) = Config::load() {
        if let Some(discord_keybind) = config.discord.keybind {
            GLOBAL_MUTE_HANDLER.set_keybind(Some(discord_keybind));
        }
    }
    Ok(())
}

#[tauri::command]
fn get_discord_status() -> String { hyperx_ngenuity_open::discord::voice::status() }
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("bad url".into());
    }
    let (prog, args): (&str, Vec<String>) = if cfg!(windows) {
        ("rundll32", vec!["url.dll,FileProtocolHandler".to_string(), url])
    } else if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else {
        ("xdg-open", vec![url])
    };
    std::process::Command::new(prog).args(args).spawn().map(|_| ()).map_err(|e| e.to_string())
}
#[tauri::command]
fn get_current_keybind() -> Option<String> {
    GLOBAL_MUTE_HANDLER.get_keybind()
}

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
fn apply_input_config(config: &Config) {
    let kb = config.discord.keybind.clone().filter(|k| !k.trim().is_empty()).unwrap_or_else(|| config.keybind.clone());
    GLOBAL_MUTE_HANDLER.set_keybind(Some(kb.clone()));
    GLOBAL_MUTE_HANDLER.set_keybind_enabled(config.discord.mode.uses_keybind());
    GLOBAL_MUTE_HANDLER.set_discord_direct(config.discord.mode.uses_direct());
    GLOBAL_MUTE_HANDLER.set_discord_to_headset(config.discord.sync_from_discord);
    hyperx_ngenuity_open::audio_switch::set_debounce_secs(config.offline_debounce_secs);
    GLOBAL_MUTE_HANDLER.set_sidetone_play_pause(config.device.sidetone_play_pause);
    log::info!("[Main] keybind={:?} discord.mode={:?} mute_button_mode={:?} sidetone_play_pause={}", kb, config.discord.mode, config.input.mute_button_mode, config.device.sidetone_play_pause);
}

/// State for the UI: `connected` is false while the headset itself is off (dongle still present).
fn ui_state(m: &MultiDeviceManager) -> DeviceState {
    let mut s = m.active_state();
    if let Some(d) = m.devices.get(m.active_index) {
        s.connected = s.connected && d.headset_online;
    }
    s
}

/// Redraw the tray icon and tooltip for the given device state (offline icon when the headset is off).
fn update_tray(app: &tauri::AppHandle, st: &DeviceState) {
    let Some(tray) = app.tray_by_id("main") else { return; };
    let icon_config = TrayIconConfig::load_or_create();
    log::debug!("[Tray] update: mode={:?} bat={} charging={} high.fg={:?} high.outline={:?}",
        icon_config.mode, st.battery_percent, st.charging,
        icon_config.colors.high.fg, icon_config.colors.high.outline);
    // Headset off / dongle only: show the dedicated offline icon, never the last percent.
    let (pct, chg) = if st.connected { (st.battery_percent, st.charging) } else { (OFFLINE_PERCENT, false) };
    let (rgba, w, h) = match icon_config.mode {
        TrayIconMode::Big => generate_big_digits_rgba(pct, chg, &icon_config.for_tray()),
        TrayIconMode::Battery => generate_battery_gauge_rgba(&icon_config.for_tray(), pct, chg),
    };
    if let Err(e) = tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h))) {
        log::warn!("[Tray] set_icon failed: {}", e);
    }
    let tooltip = if !st.connected { hyperx_ngenuity_open::tr_pick("HyperX NGENUITY Open\nгарнитура выключена", "HyperX NGENUITY Open\nheadset off") } else if st.charging { format!("HyperX NGENUITY Open\n⚡ {}% — {}", st.battery_percent, st.name) } else { format!("HyperX NGENUITY Open\n🔋 {}% — {}", st.battery_percent, st.name) };
    let _ = tray.set_tooltip(Some(&tooltip));
}

fn publish_disconnected(app: &tauri::AppHandle, state: &Arc<Mutex<DeviceState>>) {
    { let mut st = state.lock().unwrap(); *st = DeviceState::default(); }
    let _ = app.emit("device-disconnected", ()); let _ = app.emit("device-state", DeviceState::default());

    // Switch to the saved fallback audio devices (restored automatically when the headset returns)
    AUDIO_SWITCH.lock().force_away();

    if let Some(tray) = app.tray_by_id("main") {
        let icon_config = TrayIconConfig::load_or_create();
        let (rgba, w, h) = match icon_config.mode {
            TrayIconMode::Big => generate_big_digits_rgba(OFFLINE_PERCENT, false, &icon_config.for_tray()),
            TrayIconMode::Battery => generate_battery_gauge_rgba(&icon_config.for_tray(), OFFLINE_PERCENT, false),
        };
        if let Err(e) = tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h))) {
            log::warn!("[Tray] disconnected set_icon failed: {}", e);
        }
        let _ = tray.set_tooltip(Some(&hyperx_ngenuity_open::tr_pick("HyperX NGENUITY Open — нет подключения", "HyperX NGENUITY Open — not connected")));
    }
}

fn main() {
    let config = Config::load().unwrap_or_default();
    hyperx_ngenuity_open::logging::init(&config);
    hyperx_ngenuity_open::single_instance::kill_duplicates();
    GLOBAL_MUTE_HANDLER.set_keybind(Some(config.keybind.clone()));
    if let Some(discord_keybind) = config.discord.keybind.clone() {
        GLOBAL_MUTE_HANDLER.set_keybind(Some(discord_keybind));
    }
    GLOBAL_MUTE_HANDLER.set_mode(match config.input.mute_button_mode {
        MuteButtonMode::Standard => InputMuteButtonMode::Standard,
        MuteButtonMode::MediaPlayPause => InputMuteButtonMode::MediaPlayPause,
        MuteButtonMode::SmartDouble => InputMuteButtonMode::SmartDouble,
        MuteButtonMode::SmartHold => InputMuteButtonMode::SmartHold,
        MuteButtonMode::HoldPlayPause => InputMuteButtonMode::HoldPlayPause,
    });
    apply_input_config(&config);
    hyperx_ngenuity_open::audio::voice::update_config(config.voice);
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
            let open_i = MenuItem::new(&app_handle, &hyperx_ngenuity_open::tr_pick("Открыть", "Open"), true, None::<&str>)?;
            let toggle_i = MenuItem::new(&app_handle, &hyperx_ngenuity_open::tr_pick("Мьют", "Mute"), true, None::<&str>)?;
            let compact_i = MenuItem::new(&app_handle, &hyperx_ngenuity_open::tr_pick("Компактное окно", "Compact window"), true, None::<&str>)?;
            let quit_i = MenuItem::new(&app_handle, &hyperx_ngenuity_open::tr_pick("Выход", "Quit"), true, None::<&str>)?;
            *TRAY_MENU_ITEMS.lock().unwrap() = Some([open_i.clone(), toggle_i.clone(), compact_i.clone(), quit_i.clone()]);
            menu.append(&open_i)?; menu.append(&toggle_i)?; menu.append(&compact_i)?; menu.append(&PredefinedMenuItem::separator(&app_handle)?)?; menu.append(&quit_i)?;
            let (tray_rgba, tray_w, tray_h) = load_tray_png();
            let tray_icon = tauri::image::Image::new(&tray_rgba, tray_w, tray_h);
            let _tray = tauri::tray::TrayIconBuilder::with_id("main").icon(tray_icon).menu(&menu).tooltip(&hyperx_ngenuity_open::tr_pick("HyperX NGENUITY Open — поиск наушников...", "HyperX NGENUITY Open — searching for headset..."))
                .on_menu_event({ let device_cmd_tx = device_cmd_tx.clone(); move |app, event| {
                    if event.id == open_i.id() { show_main_window(app); }
                    else if event.id == toggle_i.id() { let _ = device_cmd_tx.send(DeviceCommand::ToggleMute); }
                    else if event.id == compact_i.id() { let _ = open_compact_window(app.clone()); }
                    else if event.id == quit_i.id() { app.exit(0); }
                }})
                .on_tray_icon_event(|tray, event| { if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event { show_main_window(&tray.app_handle()); } })
                .build(&app_handle)?;
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.set_title(&format!("HyperX NGENUITY Open v{}", env!("CARGO_PKG_VERSION")));
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
                let mut last_hid_refresh = Instant::now() - Duration::from_secs(60);
                let mut last_mute: Option<bool> = None; let mut seen_sidetone_events = 0u32;
                let mut user_sidetone: bool = Config::load().map(|c| c.device.sidetone).unwrap_or(false);
                let mut last_mute_poll = Instant::now(); let mut next_wireless_query = Instant::now();
                let mut last_charging = false; let mut low_battery = hyperx_ngenuity_open::low_battery::LowBatteryTracker::new(); let mut last_full_charge = false; let mut startup_announced = false;
                let mut consecutive_refresh_failures = 0u32;
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
                                was_connected = true; heartbeat_failures = 0; last_mute = None; seen_sidetone_events = 0; startup_announced = false; last_charging = false; last_full_charge = false;
                                let st = manager.active_state();
                                { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); }
                                let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                                { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); }
                                let _ = app_handle_device.emit("device-connected", ()); let _ = app_handle_device.emit("device-state", st.clone()); let _ = app_handle_device.emit("devices-list", all);
                            }
                            Err(e) => { if was_connected { was_connected = false; publish_disconnected(&app_handle_device, &device_state_inner); { let mut a = all_devices_inner.lock().unwrap(); *a = Vec::new(); } let _ = app_handle_device.emit("devices-list", Vec::<DeviceState>::new()); } log::debug!("[Device] scan: {}", e); thread::sleep(Duration::from_secs(10)); continue; }
                        }
                    }
                    while let Ok(cmd) = device_cmd_rx.try_recv() {
                        if let Some(dev) = manager.active_device() {
                            if let DeviceCommand::SetSidetone(en) = &cmd { user_sidetone = *en; }
                            let prev_muted = dev.state.muted;
                            let prev_sidetone = dev.state.sidetone;
                            let result = match cmd { DeviceCommand::ToggleMute => dev.toggle_mute(), DeviceCommand::SetSidetone(enabled) => dev.set_sidetone(enabled), DeviceCommand::SetVoicePrompts(enabled) => dev.set_voice_prompts(enabled), DeviceCommand::SetAutoShutdown(minutes) => dev.set_auto_shutdown(minutes) };
                            if let Err(e) = &result { let _ = app_handle_device.emit("device-command-error", e.to_string()); }
                            let st = manager.active_state();
                            let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                            { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); } let _ = app_handle_device.emit("device-state", st.clone());
                            { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); } let _ = app_handle_device.emit("devices-list", all);
                            last_mute = Some(st.muted);
                            // Trigger Discord mute on device mute change
                            if prev_muted != st.muted {
                                // Check if sidetone changed (indicates long press)
                                if prev_sidetone != st.sidetone {
                                    // Long press detected: trigger Play/Pause instead of mute
                                    log::info!("[Device] Long press detected (sidetone changed), triggering Play/Pause");
                                    hyperx_ngenuity_open::input::GLOBAL_MUTE_HANDLER.do_media_play_pause();
                                    // Restore sidetone to prevent drift
                                    if let Some(dev) = manager.active_device() {
                                        if let Err(e) = dev.set_sidetone(prev_sidetone) {
                                            log::warn!("[Device] Failed to restore sidetone: {}", e);
                                        }
                                    }
                                } else {
                                    // Normal press: trigger mute
                                    hyperx_ngenuity_open::input::GLOBAL_MUTE_HANDLER.on_mute_toggled(st.muted);
                                }
                            }
                        }
                    }
                    // Event-driven part (like NGenuity): listen for what the headset sends by itself —
                    // mute button, sidetone button, wireless on/off — between the slow full refreshes.
                    {
                        let mut event_changed = false;
                        let mut muted_now: Option<bool> = None;
                        let mut side_events = 0u32;
                        // Every 30 s ask every connected headset whether it is switched on.
                        // Every 30 s (every 5 s when audio auto-switching is on, so the switch to the
                        // fallback device is quick) ask each connected headset if it is switched on.
                        if Instant::now() >= next_wireless_query {
                            let secs = Config::load().map(|c| c.status_poll_secs).unwrap_or(30).clamp(1, 120);
                            next_wireless_query = Instant::now() + Duration::from_secs(secs as u64);
                            let active = manager.active_index;
                            for (i, d) in manager.devices.iter_mut().enumerate() {
                                if d.request_wireless_status().unwrap_or(false) && i == active { event_changed = true; }
                            }
                        }
                        // First odd status reading: re-ask in 1 s to confirm quickly instead of waiting a full poll.
                        if manager.devices.get(manager.active_index).map_or(false, |d| d.headset_online && d.offline_readings > 0) {
                            let soon = Instant::now() + Duration::from_secs(1);
                            if next_wireless_query > soon { next_wireless_query = soon; }
                        }
                        // The active headset went off/away while another one is on: follow it.
                        let active_ok = manager.devices.get(manager.active_index).map_or(false, |d| d.headset_online && d.state.connected);
                        if !active_ok {
                            if let Some(i) = manager.devices.iter().position(|d| d.headset_online && d.state.connected) {
                                if i != manager.active_index {
                                    log::info!("[Device] Active headset is off, switching to device {}", i);
                                    manager.active_index = i;
                                    last_mute = None;
                                    seen_sidetone_events = manager.devices[i].sidetone_button_events;
                                    event_changed = true;
                                    let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                                    { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); }
                                    let _ = app_handle_device.emit("devices-list", all);
                                }
                            }
                        }
                        if let Some(dev) = manager.active_device() {
                            if dev.poll_events(100).unwrap_or(false) { event_changed = true; }
                            if !dev.mute_events_seen && last_mute_poll.elapsed() >= Duration::from_secs(1) {
                                last_mute_poll = Instant::now();
                                if dev.poll_mute().unwrap_or(false) { event_changed = true; }
                            }
                            muted_now = Some(dev.state.muted);
                            side_events = dev.sidetone_button_events;
                            AUDIO_SWITCH.lock().update_confirmed(dev.headset_online, dev.status_known, dev.state.battery_percent > 0);
                        }
                        if let Some(m) = muted_now {
                            if last_mute != Some(m) {
                                let first = last_mute.is_none();
                                last_mute = Some(m);
                                if first { GLOBAL_MUTE_HANDLER.sync_discord_mute(m); } else { GLOBAL_MUTE_HANDLER.on_mute_toggled(m); }
                            }
                        }
                        if GLOBAL_MUTE_HANDLER.discord_to_headset() {
                            if let Some(dm) = hyperx_ngenuity_open::discord::voice::take_discord_change() {
                                if let Some(dev) = manager.active_device() {
                                    if dev.state.muted != dm {
                                        log::info!("[Discord] Discord mute -> headset mute {}", dm);
                                        let _ = dev.toggle_mute();
                                        if dev.state.muted == dm {
                                            last_mute = Some(dm);
                                            event_changed = true;
                                        }
                                    }
                                }
                            }
                        }
                        if side_events < seen_sidetone_events { seen_sidetone_events = side_events; }
                        if side_events != seen_sidetone_events {
                            seen_sidetone_events = side_events;
                            GLOBAL_MUTE_HANDLER.on_sidetone_button();
                            // The headset flips its own sidetone on the button press; put it back so the
                            // button acts as Play/Pause only.
                            if GLOBAL_MUTE_HANDLER.sidetone_play_pause_enabled() {
                                if let Some(dev) = manager.active_device() {
                                    if dev.state.sidetone != user_sidetone {
                                        log::info!("[Device] Restoring sidetone to {} after button press", user_sidetone);
                                        let _ = dev.set_sidetone(user_sidetone);
                                        event_changed = true;
                                    }
                                }
                            }
                        }
                        if event_changed {
                            let st = ui_state(&manager);
                            { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); }
                            update_tray(&app_handle_device, &st);
                            let _ = app_handle_device.emit("device-state", st);
                        }
                    }
                    // Adaptive refresh interval based on consecutive failures
                    let refresh_interval = if consecutive_refresh_failures > 3 {
                        Duration::from_secs(60) // Slower refresh if device is unstable
                    } else {
                        Duration::from_secs(30) // same cadence as NGenuity's timer
                    };

                    if last_hid_refresh.elapsed() < refresh_interval {
                        thread::sleep(Duration::from_millis(30));
                        continue;
                    }
                    last_hid_refresh = Instant::now();
                    let mut refresh_ok = true;
                    for dev in manager.devices.iter_mut() {
                        if let Err(e) = dev.refresh_state() {
                            log::debug!("[Device] refresh failed for {}: {}", dev.state.device_id, e);
                            refresh_ok = false;
                        }
                    }
                    if refresh_ok {
                        heartbeat_failures = 0;
                        consecutive_refresh_failures = 0;
                        let st = ui_state(&manager);
                        let needs_sidetone_restore = if let Ok(config) = Config::load() {
                            st.sidetone != config.device.sidetone
                        } else { false };
                        { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); } let _ = app_handle_device.emit("device-state", st.clone());
                        let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                        { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); } let _ = app_handle_device.emit("devices-list", all.clone());
                        // Restore sidetone from config if needed (outside borrow scope)
                        if needs_sidetone_restore {
                            if let Ok(config) = Config::load() {
                                if let Some(dev) = manager.active_device() {
                                    log::info!("[Device] Restoring sidetone from config: {} -> {}", dev.state.sidetone, config.device.sidetone);
                                    if let Err(e) = dev.set_sidetone(config.device.sidetone) {
                                        log::warn!("[Device] Failed to restore sidetone: {}", e);
                                    }
                                }
                            }
                        }
                        if !startup_announced && st.battery_percent > 0 { startup_announced = true; let v = if st.charging { hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging } else { hyperx_ngenuity_open::audio::voice::VoiceEvent::Battery(st.battery_percent) }; hyperx_ngenuity_open::audio::voice::play(v); last_charging = st.charging; }
                        {
                            let vcfg = Config::load().unwrap_or_default().voice;
                            if let Some(alert) = low_battery.update(st.battery_percent, st.charging, &vcfg) {
                                log::warn!("[Device] Battery low: {}% (repeat x{})", alert.percent, alert.repeat);
                                hyperx_ngenuity_open::audio::voice::play_low_battery(alert.percent, alert.repeat);
                            }
                        }
                        if st.charging && !last_charging { hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::Charging); }
                        if st.battery_percent == 100 && st.charging && !last_full_charge { last_full_charge = true; hyperx_ngenuity_open::audio::voice::play(hyperx_ngenuity_open::audio::voice::VoiceEvent::FullCharge); }
                        if !st.charging { last_full_charge = false; } last_charging = st.charging;
                    } else {
                        consecutive_refresh_failures += 1;
                        log::debug!("[Device] Consecutive refresh failures: {}", consecutive_refresh_failures);
                    }
                    if !refresh_ok {
                    heartbeat_failures += 1; let enumerated = manager.is_enumerated(); log::warn!("[HID] Heartbeat failed {}/5; enumeration={}", heartbeat_failures, enumerated); if heartbeat_failures >= 5 {
                        if enumerated {
                            log::warn!("[HID] Device enumerated but not accessible — retrying HID open without publishing disconnect.");
                            if let Err(e) = manager.scan_and_connect() {
                                log::debug!("[HID] Silent HID reopen failed: {}", e);
                            } else {
                                let st = manager.active_state();
                                { let mut s = device_state_inner.lock().unwrap(); *s = st.clone(); }
                                let all: Vec<DeviceState> = manager.devices.iter().map(|d| d.state.clone()).collect();
                                { let mut a = all_devices_inner.lock().unwrap(); *a = all.clone(); }
                                let _ = app_handle_device.emit("device-state", st);
                                let _ = app_handle_device.emit("devices-list", all);
                            }
                            thread::sleep(Duration::from_secs(15));
                        } else {
                            for d in manager.devices.iter_mut() { d.disconnect(); }
                            publish_disconnected(&app_handle_device, &device_state_inner);
                            { let mut a = all_devices_inner.lock().unwrap(); *a = Vec::new(); }
                            let _ = app_handle_device.emit("devices-list", Vec::<DeviceState>::new());
                            thread::sleep(Duration::from_secs(3));
                        }
                        heartbeat_failures = 0;
                    }
                    }
                    // Update tray icon only if refresh was successful
                    if refresh_ok {
                        let st = ui_state(&manager);
                        update_tray(&app_handle_device, &st);
                    }
                    thread::sleep(Duration::from_millis(500));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_device_state, get_connected_devices, select_device, get_config, save_config, get_per_device_config, save_per_device_config, set_custom_voice_dir, upload_voice_file, get_tray_config, save_tray_config, get_autostart_enabled, set_autostart_enabled, check_battery_voice, test_voice, get_audio_levels, set_volume, set_mic_volume, toggle_system_mic_mute, toggle_system_output_mute, play_pause, apply_eq, eq_backend_status, toggle_mute, set_sidetone, set_voice_prompts, set_auto_shutdown, set_mute_button_mode, get_current_keybind, get_discord_status, open_url, open_compact_window, show_main_window_cmd, get_audio_devices, get_default_audio_devices, set_default_output_device, set_default_input_device, switch_to_fallback_audio])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
