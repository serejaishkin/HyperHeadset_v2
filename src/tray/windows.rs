use std::sync::{mpsc::Sender, Arc, Mutex};
use std::collections::HashMap;
use tray_icon::{
    TrayIcon, TrayIconBuilder, Icon,
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent, MenuId},
};

pub struct WindowsTray {
    tray: TrayIcon,
    tx: Sender<super::TrayCommand>,
    callbacks: Arc<Mutex<HashMap<MenuId, Box<dyn Fn() + Send + Sync>>>>,
    last_percent: u8,
    last_charging: bool,
    last_muted: bool,
}

impl WindowsTray {
    pub fn new(tx: Sender<super::TrayCommand>) -> Self {
        let callbacks: Arc<Mutex<HashMap<MenuId, Box<dyn Fn() + Send + Sync>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let icon = Icon::from_rgba(vec![0; 4], 1, 1)
            .unwrap_or_else(|_| Icon::from_rgba(vec![255; 4], 1, 1).unwrap());

        let tray = TrayIconBuilder::new()
            .with_tooltip("HyperX — подключение...")
            .with_icon(icon)
            .with_menu(Box::new(Menu::new()))
            .build()
            .expect("Failed to create tray icon");

        let mut this = Self {
            tray,
            tx: tx.clone(),
            callbacks,
            last_percent: 255,
            last_charging: false,
            last_muted: false,
        };

        this.rebuild_menu();

        // Фоновый поток: слушаем клики по пунктам меню трея
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

        this
    }

    fn rebuild_menu(&mut self) {
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
            Box::new(move || { 
                log::info!("[Tray] 'Open' clicked"); 
                let _ = tx_open.send(super::TrayCommand::ShowWindow); 
            }),
        );

        let toggle_i = MenuItem::new("Переключить мьют", true, None);
        let _ = menu.append(&toggle_i);
        let tx_toggle = self.tx.clone();
        new_callbacks.insert(
            toggle_i.id().clone(),
            Box::new(move || { 
                log::info!("[Tray] 'ToggleMute' clicked"); 
                let _ = tx_toggle.send(super::TrayCommand::ToggleMute); 
            }),
        );

        let _ = menu.append(&PredefinedMenuItem::separator());

        let quit_i = MenuItem::new("Выход", true, None);
        let _ = menu.append(&quit_i);
        let tx_quit = self.tx.clone();
        new_callbacks.insert(
            quit_i.id().clone(),
            Box::new(move || { 
                log::info!("[Tray] 'Quit' clicked"); 
                let _ = tx_quit.send(super::TrayCommand::Quit); 
            }),
        );

        *self.callbacks.lock().unwrap() = new_callbacks;
        let _ = self.tray.set_menu(Some(Box::new(menu)));
    }

    pub fn poll(&self) {}

    pub fn update_battery(&mut self, percent: u8, charging: bool) {
        if self.last_percent != percent || self.last_charging != charging {
            self.last_percent = percent;
            self.last_charging = charging;

            let tooltip = if charging {
                format!("HyperX — ⚡ Заряжается: {}%", percent)
            } else {
                format!("HyperX — 🔋 Батарея: {}%", percent)
            };
            let _ = self.tray.set_tooltip(Some(&tooltip));

            self.rebuild_menu();
        }
    }

    pub fn update_mute(&mut self, muted: bool) {
        if self.last_muted != muted {
            self.last_muted = muted;
            self.rebuild_menu();
        }
    }
}