//! Linux system tray using ksni (D-Bus StatusNotifierItem)
//!
//! Features:
//! - Battery icon with percentage tooltip
//! - Mute status indicator
//! - Menu: Open / Toggle Mute / Battery / Quit

use std::sync::mpsc::Sender;
use super::TrayCommand;

pub struct LinuxTray {
    tx: Sender<TrayCommand>,
    battery: std::sync::Arc<std::sync::Mutex<u8>>,
    muted: std::sync::Arc<std::sync::Mutex<bool>>,
}

impl LinuxTray {
    pub fn new(tx: Sender<TrayCommand>) -> Self {
        let battery = std::sync::Arc::new(std::sync::Mutex::new(0u8));
        let muted = std::sync::Arc::new(std::sync::Mutex::new(false));

        let tray = Self {
            tx: tx.clone(),
            battery: battery.clone(),
            muted: muted.clone(),
        };

        // Spawn ksni tray
        let _handle = ksni::TrayService::new(HyperXTray {
            tx,
            battery,
            muted,
        });

        tray
    }

    pub fn update_battery(&self, percent: u8) {
        *self.battery.lock().unwrap() = percent;
    }

    pub fn update_mute(&self, muted: bool) {
        *self.muted.lock().unwrap() = muted;
    }
}

struct HyperXTray {
    tx: Sender<TrayCommand>,
    battery: std::sync::Arc<std::sync::Mutex<u8>>,
    muted: std::sync::Arc<std::sync::Mutex<bool>>,
}

impl ksni::Tray for HyperXTray {
    fn id(&self) -> String {
        "hyperx-ngenuity-open".to_string()
    }

    fn title(&self) -> String {
        let bat = *self.battery.lock().unwrap();
        let muted = *self.muted.lock().unwrap();
        if muted {
            format!("HyperX 🔇 {}%", bat)
        } else {
            format!("HyperX 🎤 {}%", bat)
        }
    }

    fn icon_name(&self) -> String {
        let bat = *self.battery.lock().unwrap();
        if bat > 50 {
            "battery-full-symbolic".to_string()
        } else if bat > 20 {
            "battery-good-symbolic".to_string()
        } else {
            "battery-low-symbolic".to_string()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let tx_open = self.tx.clone();
        let tx_mute = self.tx.clone();
        let tx_quit = self.tx.clone();

        vec![
            ksni::MenuItem::Standard(ksni::menu::StandardItem {
                label: "Open".to_string(),
                icon_name: "window-new".to_string(),
                activate: Box::new(move |_| {
                    let _ = tx_open.send(TrayCommand::ShowWindow);
                }),
                ..Default::default()
            }),
            ksni::MenuItem::Standard(ksni::menu::StandardItem {
                label: "Toggle Mute".to_string(),
                icon_name: "audio-input-microphone".to_string(),
                activate: Box::new(move |_| {
                    let _ = tx_mute.send(TrayCommand::ToggleMute);
                }),
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(ksni::menu::StandardItem {
                label: format!("Battery: {}%", *self.battery.lock().unwrap()),
                enabled: false,
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(ksni::menu::StandardItem {
                label: "Quit".to_string(),
                icon_name: "application-exit".to_string(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }),
        ]
    }
}
