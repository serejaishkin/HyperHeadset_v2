use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use crate::tray_battery_icon_state::{TrayBatteryIconState, WindowsIconKey};
use crate::tray_icon_renderer::{render_windows_battery_icon_rgba, create_default_tray_icon};
use crate::device::DeviceState;

const NO_DEVICE_TOOLTIP: &str = "HyperX NGENUITY Open — No device";
const DISCONNECTED_TOOLTIP: &str = "HyperX NGENUITY Open — Headset disconnected";

pub struct TrayManager {
    tray_icon: Option<TrayIcon>,
    icon_cache: HashMap<WindowsIconKey, Vec<u8>>,
    current_icon_key: Option<WindowsIconKey>,
}

impl TrayManager {
    pub fn new() -> Self {
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(Menu::new()))
            .with_icon(create_default_tray_icon())
            .with_tooltip(NO_DEVICE_TOOLTIP)
            .with_menu_on_left_click(true)
            .build()
            .ok();

        Self {
            tray_icon,
            icon_cache: HashMap::new(),
            current_icon_key: None,
        }
    }

    pub fn update(&mut self, device_state: Option<&DeviceState>) {
        let icon_state = TrayBatteryIconState::from_device_state(device_state);
        let desired_key = icon_state.windows_icon_key();

        // Only update icon if state changed
        if desired_key == self.current_icon_key {
            return;
        }

        let Some(tray) = self.tray_icon.as_ref() else {
            return;
        };

        if let Some(key) = desired_key {
            let rgba = self
                .icon_cache
                .entry(key)
                .or_insert_with(|| render_windows_battery_icon_rgba(key))
                .clone();

            if let Ok(icon) = tray_icon::Icon::from_rgba(rgba, 16, 16) {
                let _ = tray.set_icon(Some(icon));
            }

            let tooltip = if key.charging {
                format!("HyperX — {}% (charging)", key.percent)
            } else {
                format!("HyperX — {}% battery", key.percent)
            };
            let _ = tray.set_tooltip(Some(&tooltip));
        } else {
            let _ = tray.set_icon(Some(create_default_tray_icon()));
            let tooltip = match icon_state {
                TrayBatteryIconState::NoDevice => NO_DEVICE_TOOLTIP,
                TrayBatteryIconState::Disconnected => DISCONNECTED_TOOLTIP,
                TrayBatteryIconState::ConnectedUnknown => "HyperX — Battery unknown",
                _ => NO_DEVICE_TOOLTIP,
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }

        self.current_icon_key = desired_key;
    }
}
