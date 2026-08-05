use std::sync::mpsc::Sender;
use std::sync::Mutex;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, Icon,
};
use super::{TrayCommand, icon::{TrayIconConfig, generate_battery_icon_rgba}};

pub struct MacOSTray {
    tray_icon: tray_icon::TrayIcon,
    _menu: Menu,
    current_percent: Mutex<u8>,
    current_muted: Mutex<bool>,
    current_charging: Mutex<bool>,
    icon_config: TrayIconConfig,
}

impl MacOSTray {
    pub fn new(tx: Sender<TrayCommand>) -> Self {
        let menu = Menu::new();

        let open_item = MenuItem::new("Открыть", true, None);
        let toggle_mute_item = MenuItem::new("Переключить мьют", true, None);
        let battery_item = MenuItem::new("Батарея: --%", false, None);
        let sep1 = PredefinedMenuItem::separator();
        let sep2 = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Выход", true, None);

        menu.append(&open_item).unwrap();
        menu.append(&toggle_mute_item).unwrap();
        menu.append(&sep1).unwrap();
        menu.append(&battery_item).unwrap();
        menu.append(&sep2).unwrap();
        menu.append(&quit_item).unwrap();

        let icon_config = TrayIconConfig::load_or_create();
        let (rgba, w, h) = generate_battery_icon_rgba(&*crate::tray::icon::crate::tray::icon::get_tray_icon_config().lock().unwrap(), 100, false);
        let icon = Icon::from_rgba(rgba, w, h).unwrap_or_else(|_| load_fallback_icon());

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("HyperX NGENUITY Open")
            .with_icon(icon)
            .build()
            .unwrap();

        let open_id = open_item.id().clone();
        let toggle_mute_id = toggle_mute_item.id().clone();
        let quit_id = quit_item.id().clone();
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            loop {
                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == open_id {
                        let _ = tx_clone.send(TrayCommand::ShowWindow);
                    } else if event.id == toggle_mute_id {
                        let _ = tx_clone.send(TrayCommand::ToggleMute);
                    } else if event.id == quit_id {
                        let _ = tx_clone.send(TrayCommand::Quit);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        Self {
            tray_icon,
            _menu: menu,
            current_percent: Mutex::new(100),
            current_muted: Mutex::new(false),
            current_charging: Mutex::new(false),
            icon_config,
        }
    }

    pub fn poll(&mut self) {}
    pub fn update_battery(&self, percent: u8, charging: bool) {
        *self.current_percent.lock().unwrap() = percent;
        *self.current_charging.lock().unwrap() = charging;
        let muted = *self.current_muted.lock().unwrap();

        let (rgba, w, h) = generate_battery_icon_rgba(&*crate::tray::icon::crate::tray::icon::get_tray_icon_config().lock().unwrap(), percent, charging);
        if let Ok(icon) = Icon::from_rgba(rgba, w, h) {
            let _ = self.tray_icon.set_icon(Some(icon));
        }
        let _ = self.tray_icon.set_tooltip(Some(&build_tooltip(percent, muted)));
    }

    pub fn update_mute(&self, muted: bool) {
        *self.current_muted.lock().unwrap() = muted;
        let percent = *self.current_percent.lock().unwrap();
        let _ = self.tray_icon.set_tooltip(Some(&build_tooltip(percent, muted)));
    }
}

fn build_tooltip(percent: u8, muted: bool) -> String {
    let mic = if muted { "🔇 Выключен" } else { "🎙️ Включён" };
    format!("HyperX NGENUITY Open\n🔋 Батарея: {}%\n🎤 Микрофон: {}", percent, mic)
}

fn load_fallback_icon() -> tray_icon::Icon {
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = (x as f32 - cx).abs();
            let dy = (y as f32 - cy).abs();
            if dx < 6.0 && dy < 6.0 {
                rgba[idx] = 255; rgba[idx + 1] = 255; rgba[idx + 2] = 255; rgba[idx + 3] = 255;
            } else {
                rgba[idx + 3] = 0;
            }
        }
    }
    tray_icon::Icon::from_rgba(rgba, size as u32, size as u32).unwrap()
}