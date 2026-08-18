#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tauri::{Manager, State, Emitter, WindowEvent};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};

use hyperx_ngenuity_open::device::{DeviceState, HyperXDevice, DeviceCommand};
use hyperx_ngenuity_open::input::GLOBAL_MUTE_HANDLER;
use hyperx_ngenuity_open::tray::icon::{TrayIconConfig, generate_battery_icon_rgba};

#[derive(Clone)]
pub struct AppState {
    pub device_state: Arc<Mutex<DeviceState>>,
    pub device_cmd_tx: mpsc::Sender<DeviceCommand>,
}

#[tauri::command]
fn get_device_state(state: State<AppState>) -> DeviceState {
    state.device_state.lock().unwrap().clone()
}

#[tauri::command]
fn toggle_mute(state: State<AppState>) -> Result<(), String> {
    state.device_cmd_tx.send(DeviceCommand::ToggleMute).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_sidetone(enabled: bool, state: State<AppState>) -> Result<(), String> {
    state.device_cmd_tx.send(DeviceCommand::SetSidetone(enabled)).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_voice_prompts(enabled: bool, state: State<AppState>) -> Result<(), String> {
    state.device_cmd_tx.send(DeviceCommand::SetVoicePrompts(enabled)).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_compact_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("compact") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        "compact",
        tauri::WebviewUrl::App("compact.html".into()),
    )
    .title("HyperHeadsetv2 Compact")
    .inner_size(220.0, 200.0)
    .min_inner_size(200.0, 180.0)
    .max_inner_size(300.0, 280.0)
    .resizable(true)
    .center()
    .build()
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn publish_disconnected(app: &tauri::AppHandle, state: &Arc<Mutex<DeviceState>>) {
    {
        let mut st = state.lock().unwrap();
        *st = DeviceState::default();
    }
    let _ = app.emit("device-disconnected", ());
    let _ = app.emit("device-state", DeviceState::default());

    if let Some(tray) = app.tray_by_id("main") {
        let config = TrayIconConfig::load_or_create();
        let (rgba, w, h) = config.default_icon_rgba();
        let _ = tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h)));
        let _ = tray.set_tooltip(Some("HyperHeadsetv2 — нет подключения"));
    }
}

fn main() {
    env_logger::init();
    GLOBAL_MUTE_HANDLER.set_keybind(Some("F20".to_string()));

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();
    let (device_cmd_tx, device_cmd_rx) = mpsc::channel();

    tauri::Builder::default()
        .manage(AppState {
            device_state: device_state_clone,
            device_cmd_tx: device_cmd_tx.clone(),
        })
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let device_state_inner = device_state.clone();

            let menu = Menu::new(&app_handle)?;
            let open_i = MenuItem::new(&app_handle, "Открыть", true, None::<&str>)?;
            let toggle_i = MenuItem::new(&app_handle, "Мьют", true, None::<&str>)?;
            let compact_i = MenuItem::new(&app_handle, "Компактное окно", true, None::<&str>)?;
            let quit_i = MenuItem::new(&app_handle, "Выход", true, None::<&str>)?;
            menu.append(&open_i)?;
            menu.append(&toggle_i)?;
            menu.append(&compact_i)?;
            menu.append(&PredefinedMenuItem::separator(&app_handle)?)?;
            menu.append(&quit_i)?;

            let tray_icon = app_handle.default_window_icon().cloned().unwrap_or_else(|| {
                tauri::image::Image::new(&[0, 0, 0, 0], 1, 1)
            });

            let _tray = tauri::tray::TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .menu(&menu)
                .tooltip("HyperHeadsetv2 — поиск наушников...")
                .on_menu_event({
                    let device_cmd_tx = device_cmd_tx.clone();
                    move |app, event| {
                        if event.id == open_i.id() {
                            show_main_window(app);
                        } else if event.id == toggle_i.id() {
                            let _ = device_cmd_tx.send(DeviceCommand::ToggleMute);
                        } else if event.id == compact_i.id() {
                            let _ = open_compact_window(app.clone());
                        } else if event.id == quit_i.id() {
                            app.exit(0);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event {
                        show_main_window(&tray.app_handle());
                    }
                })
                .build(&app_handle)?;

            // X hides the main window; the process stays alive in the tray.
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(|event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = event.window().hide();
                    }
                });
            }

            let app_handle_device = app_handle.clone();
            thread::spawn(move || {
                let mut device = HyperXDevice::new();
                let mut was_connected = false;
                let mut heartbeat_failures = 0u32;

                loop {
                    if !device.state.connected {
                        match device.connect() {
                            Ok(_) => {
                                was_connected = true;
                                heartbeat_failures = 0;
                                log::info!("[Device] Headset connected");
                                {
                                    let mut st = device_state_inner.lock().unwrap();
                                    *st = device.state.clone();
                                }
                                let _ = app_handle_device.emit("device-connected", ());
                                let _ = app_handle_device.emit("device-state", device.state.clone());
                            }
                            Err(e) => {
                                if was_connected {
                                    log::warn!("[Device] Lost connection: {}", e);
                                    was_connected = false;
                                    publish_disconnected(&app_handle_device, &device_state_inner);
                                }
                                thread::sleep(Duration::from_secs(2));
                                continue;
                            }
                        }
                    }

                    while let Ok(cmd) = device_cmd_rx.try_recv() {
                        let result = match cmd {
                            DeviceCommand::ToggleMute => device.toggle_mute(),
                            DeviceCommand::SetSidetone(enabled) => device.set_sidetone(enabled),
                            DeviceCommand::SetVoicePrompts(enabled) => device.set_voice_prompts(enabled),
                        };

                        if let Err(e) = result {
                            log::warn!("[Device] Command failed: {}", e);
                        }
                        let _ = app_handle_device.emit("device-state", device.state.clone());
                    }

                    match device.refresh_state() {
                        Ok(()) => {
                            heartbeat_failures = 0;
                            {
                                let mut st = device_state_inner.lock().unwrap();
                                *st = device.state.clone();
                            }
                            let _ = app_handle_device.emit("device-state", device.state.clone());

                            if let Some(tray) = app_handle_device.tray_by_id("main") {
                                let icon_config = TrayIconConfig::load_or_create();
                                let (rgba, w, h) = generate_battery_icon_rgba(
                                    &icon_config,
                                    device.state.battery_percent,
                                    device.state.charging,
                                );
                                let _ = tray.set_icon(Some(tauri::image::Image::new(&rgba, w, h)));
                                let tooltip = if device.state.charging {
                                    format!("HyperHeadsetv2\n⚡ {}%", device.state.battery_percent)
                                } else {
                                    format!("HyperHeadsetv2\n🔋 {}%", device.state.battery_percent)
                                };
                                let _ = tray.set_tooltip(Some(&tooltip));
                            }
                        }
                        Err(e) => {
                            heartbeat_failures += 1;
                            let enumerated = HyperXDevice::is_enumerated();
                            log::warn!(
                                "[HID] Heartbeat failed {}/5: {}; Windows enumeration={}",
                                heartbeat_failures,
                                e,
                                enumerated
                            );

                            // Do not call this a USB removal immediately. We first
                            // allow transient HID timeouts. Five consecutive battery
                            // heartbeat failures force a clean handle reset.
                            if heartbeat_failures >= 5 {
                                log::warn!(
                                    "[HID] Resetting handle after 5 failures; enumeration={}",
                                    enumerated
                                );
                                device.disconnect();
                                heartbeat_failures = 0;
                                publish_disconnected(&app_handle_device, &device_state_inner);
                            }
                        }
                    }

                    thread::sleep(Duration::from_millis(500));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_device_state,
            toggle_mute,
            set_sidetone,
            set_voice_prompts,
            open_compact_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
