use std::sync::{mpsc::Sender, Arc, Mutex};
use std::collections::HashMap;
use tray_icon::{
    TrayIcon, TrayIconBuilder, Icon, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent, MenuId},
};
use super::icon::{TrayIconConfig, generate_battery_icon_rgba};

pub struct WindowsTray {
    tray: Option<TrayIcon>,
    tx: Sender<super::TrayCommand>,
    callbacks: Arc<Mutex<HashMap<MenuId, Box<dyn Fn() + Send + Sync>>>>,
    last_percent: u8,
    last_charging: bool,
    last_muted: bool,
    icon_config: TrayIconConfig,
}

impl WindowsTray {
    pub fn new(tx: Sender<super::TrayCommand>) -> Self {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        let callbacks: Arc<Mutex<HashMap<MenuId, Box<dyn Fn() + Send + Sync>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let icon_config = TrayIconConfig::load_or_create();

        let (rgba, w, h) = generate_battery_icon_rgba(&icon_config, 0, false);
        let icon = Icon::from_rgba(rgba, w, h)
            .unwrap_or_else(|_| Icon::from_rgba(vec![255; 4], 1, 1).unwrap());

        println!("[TrayWin] Before build()...");
        let tray = TrayIconBuilder::new()
            .with_tooltip("HyperX — подключение...")
            .with_icon(icon)
            .with_menu(Box::new(Menu::new()))
            .with_menu_on_left_click(false)
            .build();

        println!("[TrayWin] build() returned: is_ok={}", tray.is_ok());
        let tray = match tray {
            Ok(t) => {
                log::info!("[Tray] Tray icon created successfully");
                Some(t)
            }
            Err(e) => {
                log::error!("[Tray] Failed to create tray icon: {}. Continuing without tray.", e);
                None
            }
        };

        let mut this = Self {
            tray,
            tx: tx.clone(),
            callbacks,
            last_percent: 0,
            last_charging: false,
            last_muted: false,
            icon_config,
        };

        println!("[TrayWin] Returning WindowsTray instance");
        this.update_tooltip();
        println!("[TrayWin] Returning WindowsTray instance");
        this.rebuild_menu();

        println!("[TrayWin] Spawning menu thread...");
        let tx_menu = tx.clone();
        let cb_clone = this.callbacks.clone();
        std::thread::spawn(move || {
            let rx = MenuEvent::receiver();
            loop {
                if let Ok(event) = rx.recv() {
                    log::info!("[Tray] Menu click: id={:?}", event.id);
                    if let Ok(map) = cb_clone.try_lock() {
                        if let Some(f) = map.get(&event.id) {
                            f();
                        } else {
                            log::warn!("[Tray] Unknown menu id: {:?}", event.id);
                        }
                    }
                }
            }
        });

        println!("[TrayWin] Spawning icon thread...");
        let tx_icon = tx.clone();
        std::thread::spawn(move || {
            let rx = TrayIconEvent::receiver();
            loop {
                if let Ok(event) = rx.recv() {
                    if let TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        ..
                    } = event {
                        let _ = tx_icon.send(super::TrayCommand::ShowWindow);
                    }
                }
            }
        });

        println!("[TrayWin] Returning WindowsTray instance");
        this
    }

    fn rebuild_menu(&mut self) {
        let Some(tray) = &self.tray else { return };
        let menu = Menu::new();
        let mut new_callbacks: HashMap<MenuId, Box<dyn Fn() + Send + Sync>> = HashMap::new();

        let battery_text = if self.last_charging {
            format!("⚡ Заряд: {}%", self.last_percent)
        } else {
            format!("🔋 Батарея: {}%", self.last_percent)
        };
        let _ = menu.append(&MenuItem::new(&battery_text, false, None));

        let mic_text = if self.last_muted {
            "🔇 Микрофон: выкл"
        } else {
            "🎙️ Микрофон: вкл"
        };
        let _ = menu.append(&MenuItem::new(mic_text, false, None));

        let _ = menu.append(&PredefinedMenuItem::separator());

        let open_i = MenuItem::new("Открыть", true, None);
        let _ = menu.append(&open_i);
        let tx_open = self.tx.clone();
        new_callbacks.insert(
            open_i.id().clone(),
            Box::new(move || { let _ = tx_open.send(super::TrayCommand::ShowWindow); }),
        );

        let toggle_i = MenuItem::new("Переключить мьют", true, None);
        let _ = menu.append(&toggle_i);
        let tx_toggle = self.tx.clone();
        new_callbacks.insert(
            toggle_i.id().clone(),
            Box::new(move || { let _ = tx_toggle.send(super::TrayCommand::ToggleMute); }),
        );

        let _ = menu.append(&PredefinedMenuItem::separator());

        let quit_i = MenuItem::new("Выход", true, None);
        let _ = menu.append(&quit_i);
        let tx_quit = self.tx.clone();
        new_callbacks.insert(
            quit_i.id().clone(),
            Box::new(move || { let _ = tx_quit.send(super::TrayCommand::Quit); }),
        );

        *self.callbacks.lock().unwrap() = new_callbacks;
        let _ = tray.set_menu(Some(Box::new(menu)));
    }

    fn update_tooltip(&self) {
        let Some(tray) = &self.tray else { return };
        let mic_status = if self.last_muted { "🔇 Выключен" } else { "🎙️ Включён" };
        let tooltip = if self.last_charging {
            format!("HyperX NGENUITY Open\n⚡ Заряжается: {}%\n🎤 Микрофон: {}", self.last_percent, mic_status)
        } else {
            format!("HyperX NGENUITY Open\n🔋 Батарея: {}%\n🎤 Микрофон: {}", self.last_percent, mic_status)
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    fn update_icon(&mut self) {
        let Some(tray) = &self.tray else { return };
        let (rgba, w, h) = generate_battery_icon_rgba(&self.icon_config, self.last_percent, self.last_charging);
        if let Ok(icon) = Icon::from_rgba(rgba, w, h) {
            let _ = tray.set_icon(Some(icon));
        }
    }

    pub fn poll(&self) {}

    pub fn update_battery(&mut self, percent: u8, charging: bool) {
        log::info!(
            "[Tray] update_battery called: percent={} charging={} (last={}/{})",
            percent, charging, self.last_percent, self.last_charging
        );
        if self.last_percent != percent || self.last_charging != charging {
            log::info!("[Tray] Battery/charging changed, updating icon");
            self.last_percent = percent;
            self.last_charging = charging;
            self.update_tooltip();
            self.update_icon();
            self.rebuild_menu();
        } else {
            log::info!("[Tray] Battery/charging unchanged, skipping");
        }
    }

    pub fn update_mute(&mut self, muted: bool) {
        if self.last_muted != muted {
            self.last_muted = muted;
            self.update_tooltip();
            self.rebuild_menu();
        }
    }
}
