#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tauri::{Manager, State, Emitter};
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
fn toggle_mute(state: State<AppState>) {
    let _ = state.device_cmd_tx.send(DeviceCommand::ToggleMute);
}

fn main() {
    env_logger::init();
    GLOBAL_MUTE_HANDLER.set_keybind(Some("F20".to_string()));

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    let device_state_clone = device_state.clone();
    let (device_cmd_tx, device_cmd_rx) = mpsc::channel::<DeviceCommand>();

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
            let quit_i = MenuItem::new(&app_handle, "Выход", true, None::<&str>)?;
            menu.append(&open_i)?;
            menu.append(&toggle_i)?;
            menu.append(&PredefinedMenuItem::separator(&app_handle)?)?;
            menu.append(&quit_i)?;

            let _tray = tauri::tray::TrayIconBuilder::new()
                .id("main")
                .icon(app_handle.default_window_icon().cloned().unwrap_or_default())
                .menu(&menu)
                .tooltip("HyperHeadsetv2 — подключение...")
                .on_menu_event({
                    let app_handle = app_handle.clone();
                    let device_cmd_tx = device_cmd_tx.clone();
                    move |app, event| {
                        if event.id == open_i.id() {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show(); let _ = window.set_focus();
                            }
                        } else if event.id == toggle_i.id() {
                            let _ = device_cmd_tx.send(DeviceCommand::ToggleMute);
                        } else if event.id == quit_i.id() {
                            app.exit(0);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up, ..
                    } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show(); let _ = window.set_focus();
                        }
                    }
                })
                .build(&app_handle)?;

            let app_handle_device = app_handle.clone();
            thread::spawn(move || {
                let mut device = HyperXDevice::new();
                let mut was_connected = false;
                let mut error_count = 0;

                loop {
                    if !device.state.connected {
                        if was_connected {
                            let _ = app_handle_device.emit("device-disconnected", ());
                            let mut st = device_state_inner.lock().unwrap();
                            st.connected = false;
                            was_connected = false;
                        }
                        match device.connect() {
                            Ok(_) => {
                                let _ = app_handle_device.emit("device-connected", ());
                                let mut st = device_state_inner.lock().unwrap();
                                *st = device.state.clone();
                                was_connected = true;
                                error_count = 0;
                                let _ = device.refresh_state();
                            }
                            Err(_) => { thread::sleep(Duration::from_secs(3)); continue; }
                        }
                    }

                    while let Ok(cmd) = device_cmd_rx.try_recv() {
                        match cmd {
                            DeviceCommand::ToggleMute => {
                                let _ = device.toggle_mute();
                                let _ = app_handle_device.emit("device-state", device.state.clone());
                            }
                        }
                    }

                    if let Err(_) = device.refresh_state() {
                        error_count += 1;
                        if error_count >= 3 {
                            let _ = app_handle_device.emit("device-disconnected", ());
                            device.disconnect();
                            let mut st = device_state_inner.lock().unwrap();
                            st.connected = false;
                            error_count = 0;
                        }
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                    error_count = 0;

                    { let mut st = device_state_inner.lock().unwrap(); *st = device.state.clone(); }
                    let _ = app_handle_device.emit("device-state", device.state.clone());

                    if let Some(tray) = app_handle_device.tray_by_id("main") {
                        let icon_config = TrayIconConfig::load_or_create();
                        if device.state.connected {
                            let (rgba, w, h) = generate_battery_icon_rgba(&icon_config, device.state.battery_percent, device.state.charging);
                            let img = image::RgbaImage::from_raw(w, h, rgba).unwrap();
                            let mut png = Vec::new();
                            img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
                            if let Ok(tauri_img) = tauri::image::Image::from_bytes(&png) {
                                let _ = tray.set_icon(Some(tauri_img));
                            }
                        }
                        let tooltip = if !device.state.connected {
                            "HyperHeadsetv2 — нет подключения".to_string()
                        } else if device.state.charging {
                            format!("HyperHeadsetv2\n⚡ {}%", device.state.battery_percent)
                        } else {
                            format!("HyperHeadsetv2\n🔋 {}%", device.state.battery_percent)
                        };
                        let _ = tray.set_tooltip(Some(&tooltip));
                    }

                    thread::sleep(Duration::from_millis(500));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_device_state, toggle_mute])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
