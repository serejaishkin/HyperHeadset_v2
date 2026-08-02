//! macOS system tray using tray-icon crate
//!
//! Features:
//! - Battery level in tooltip
//! - Mute status
//! - Menu: Open / Toggle Mute / Battery / Quit

use std::sync::mpsc::Sender;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder,
};

use super::TrayCommand;

pub struct MacOSTray {
    tray_icon: tray_icon::TrayIcon,
    _menu: Menu,
}

impl MacOSTray {
    pub fn new(tx: Sender<TrayCommand>) -> Self {
        let menu = Menu::new();

        let open_item = MenuItem::new("Open", true, None);
        let toggle_mute_item = MenuItem::new("Toggle Mute", true, None);
        let battery_item = MenuItem::new("Battery: --%", false, None);
        let sep = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);

        menu.append(&open_item).unwrap();
        menu.append(&toggle_mute_item).unwrap();
        menu.append(&sep).unwrap();
        menu.append(&battery_item).unwrap();
        menu.append(&sep).unwrap();
        menu.append(&quit_item).unwrap();

        // macOS tray icon (use template icon for dark mode support)
        let icon = load_template_icon();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("HyperX NGENUITY Open")
            .with_icon(icon)
            .with_icon_as_template(true)
            .build()
            .unwrap();

        // Menu event handler
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == open_item.id() {
                        let _ = tx_clone.send(TrayCommand::ShowWindow);
                    } else if event.id == toggle_mute_item.id() {
                        let _ = tx_clone.send(TrayCommand::ToggleMute);
                    } else if event.id == quit_item.id() {
                        let _ = tx_clone.send(TrayCommand::Quit);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        Self { tray_icon, _menu: menu }
    }

    pub fn update_battery(&self, percent: u8) {
        let _ = self.tray_icon.set_tooltip(&format!(
            "HyperX Cloud II Wireless\nBattery: {}%",
            percent
        ));
    }

    pub fn update_mute(&self, muted: bool) {
        let tooltip = if muted {
            "HyperX Cloud II Wireless\n🔇 Muted"
        } else {
            "HyperX Cloud II Wireless\n🎤 Unmuted"
        };
        let _ = self.tray_icon.set_tooltip(tooltip);
    }
}

fn load_template_icon() -> tray_icon::Icon {
    // 16x16 template icon for macOS
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];

    // Simple headset shape (white for template)
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            // Draw simple icon
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = (x as f32 - cx).abs();
            let dy = (y as f32 - cy).abs();

            if dx < 6.0 && dy < 6.0 {
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else {
                rgba[idx + 3] = 0;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size as u32, size as u32).unwrap()
}
