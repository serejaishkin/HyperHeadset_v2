//! Windows system tray using tray-icon crate
//!
//! Features:
//! - Battery level icon (colored segments)
//! - Mute status indicator
//! - Context menu: Open / Toggle Mute / Battery / Quit

use std::sync::mpsc::Sender;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder,
};

use super::TrayCommand;

pub struct WindowsTray {
    tray_icon: tray_icon::TrayIcon,
    _menu: Menu,
}

impl WindowsTray {
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

        // Build icon from battery level (we'll generate dynamically)
        let icon = load_battery_icon(100);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("HyperX NGENUITY Open")
            .with_icon(icon)
            .build()
            .unwrap();

        // Spawn menu event handler
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

        Self { tray_icon, _menu: menu }
    }

    pub fn update_battery(&self, percent: u8) {
        let icon = load_battery_icon(percent);
        let _ = self.tray_icon.set_icon(Some(icon));
        let _ = self.tray_icon.set_tooltip(Some(&format!(
            "HyperX Cloud II Wireless\nBattery: {}%",
            percent
        )));
    }

    pub fn update_mute(&self, muted: bool) {
        let tooltip = if muted {
            "HyperX Cloud II Wireless\n🔇 Muted"
        } else {
            "HyperX Cloud II Wireless\n🎤 Unmuted"
        };
        let _ = self.tray_icon.set_tooltip(Some(tooltip));
    }
}

/// Generate a simple battery icon (16x16 RGBA)
fn load_battery_icon(percent: u8) -> tray_icon::Icon {
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];

    // Background (dark gray)
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            rgba[idx] = 40;     // R
            rgba[idx + 1] = 40; // G
            rgba[idx + 2] = 40; // B
            rgba[idx + 3] = 255; // A
        }
    }

    // Battery fill color based on level
    let (r, g, b) = if percent > 50 {
        (0, 255, 0)   // Green
    } else if percent > 20 {
        (255, 255, 0) // Yellow
    } else {
        (255, 0, 0)   // Red
    };

    // Fill battery bar (simple vertical bar on the right)
    let fill_height = (size - 4) * percent as usize / 100;
    for y in (size - 2 - fill_height)..(size - 2) {
        for x in (size - 6)..(size - 2) {
            let idx = (y * size + x) * 4;
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = 255;
        }
    }

    tray_icon::Icon::from_rgba(rgba, size as u32, size as u32).unwrap()
}
