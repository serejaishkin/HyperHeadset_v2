use std::sync::{mpsc::Sender, Arc, Mutex};
use std::collections::HashMap;
use tray_icon::{
    TrayIcon, TrayIconBuilder, Icon, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent, MenuId},
};
use super::icon::{TrayIconConfig, generate_battery_icon_rgba};

fn create_default_tray_icon() -> Icon {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/headphone.png"));
    let img = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).unwrap()
}

pub struct WindowsTray {
    tray: Option<TrayIcon>,
    tx: Sender<super::TrayCommand>,
    callbacks: Arc<Mutex<HashMap<MenuId, Box<dyn Fn() + Send>>>>,
    last_percent: u8,
    last_charging: bool,
    last_muted: bool,
    connected: bool,
    icon_config: TrayIconConfig,
}

impl WindowsTray {
    pub fn new(tx: Sender<super::TrayCommand>) -> Self {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        let callbacks: Arc<Mutex<HashMap<MenuId, Box<dyn Fn() + Send>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let icon_config = TrayIconConfig::load_or_create();
        let icon = create_default_tray_icon();

        let tray = TrayIconBuilder::new()
            .with_tooltip("HyperX — подключение...")
            .with_icon(icon)
            .with_menu(Box::new(Menu::new()))
            .build();

        let tray: Option<TrayIcon> = match tray {
            Ok(t) => { log::info!("[Tray] Tray icon created successfully"); Some(t) }
            Err(e) => { log::error!("[Tray] Failed to create tray icon: {}", e); None }
        };

        let mut this = Self {
            tray, tx: tx.clone(), callbacks,
            last_percent: 0, last_charging: false, last_muted: false,
            connected: false, icon_config,
        };
        this.update_tooltip();
        this.rebuild_menu();

        let cb_clone = this.callbacks.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            log::info!("[Tray] Menu click: id={:?}", e.id);
            if let Ok(map) = cb_clone.try_lock() {
                if let Some(f) = map.get(&e.id) { f(); }
                else { log::warn!("[Tray] Unknown menu id: {:?}", e.id); }
            }
        }));

        let tx_icon = tx.clone();
        std::thread::spawn(move || {
            let rx = TrayIconEvent::receiver();
            loop {
                if let Ok(event) = rx.recv() {
                    if let TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } = event {
                        let _ = tx_icon.send(super::TrayCommand::ShowWindow);
                    }
                }
            }
        });
        this
    }

    fn rebuild_menu(&mut self) {
        let Some(tray) = &self.tray else { return; };
        let menu = Menu::new();
        let mut new_callbacks: HashMap<MenuId, Box<dyn Fn() + Send>> = HashMap::new();

        if self.connected {
            let battery_text = if self.last_charging { format!("⚡ Заряд: {}%", self.last_percent) }
                               else { format!("🔋 Батарея: {}%", self.last_percent) };
            let _ = menu.append(&MenuItem::new(&battery_text, false, None));
            let mic_text = if self.last_muted { "🔇 Микрофон: выкл" } else { "🎙️ Микрофон: вкл" };
            let _ = menu.append(&MenuItem::new(mic_text, false, None));
            let _ = menu.append(&PredefinedMenuItem::separator());
        } else {
            let _ = menu.append(&MenuItem::new("🎧 Гарнитура отключена", false, None));
            let _ = menu.append(&PredefinedMenuItem::separator());
        }

        let open_i = MenuItem::new("Открыть", true, None);
        let _ = menu.append(&open_i);
        let tx_open = self.tx.clone();
        new_callbacks.insert(open_i.id().clone(), Box::new(move || { let _ = tx_open.send(super::TrayCommand::ShowWindow); }));

        let toggle_i = MenuItem::new("Переключить мьют", true, None);
        let _ = menu.append(&toggle_i);
        let tx_toggle = self.tx.clone();
        new_callbacks.insert(toggle_i.id().clone(), Box::new(move || { let _ = tx_toggle.send(super::TrayCommand::ToggleMute); }));

        let _ = menu.append(&PredefinedMenuItem::separator());
        let quit_i = MenuItem::new("Выход", true, None);
        let _ = menu.append(&quit_i);
        let tx_quit = self.tx.clone();
        new_callbacks.insert(quit_i.id().clone(), Box::new(move || { let _ = tx_quit.send(super::TrayCommand::Quit); }));

        *self.callbacks.lock().unwrap() = new_callbacks;
        let _ = tray.set_menu(Some(Box::new(menu)));
    }

    fn update_tooltip(&self) {
        let Some(tray) = &self.tray else { return; };
        let tooltip = if !self.connected {
            "HyperX NGENUITY Open
🎧 Гарнитура отключена".to_string()
        } else {
            let mic_status = if self.last_muted { "🔇 Выключен" } else { "🎙️ Включён" };
            if self.last_charging { format!("HyperX NGENUITY Open
⚡ Заряжается: {}%
🎤 Микрофон: {}", self.last_percent, mic_status) }
            else { format!("HyperX NGENUITY Open
🔋 Батарея: {}%
🎤 Микрофон: {}", self.last_percent, mic_status) }
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    fn update_icon(&mut self) {
        let Some(tray) = &self.tray else { return; };
        if !self.connected { let _ = tray.set_icon(Some(create_default_tray_icon())); return; }
        let (rgba, w, h) = generate_battery_icon_rgba(&self.icon_config, self.last_percent, self.last_charging);
        if let Ok(icon) = Icon::from_rgba(rgba, w, h) { let _ = tray.set_icon(Some(icon)); }
    }

    pub fn refresh_icon(&mut self) { self.update_icon(); }
    pub fn update_icon_config(&mut self, config: TrayIconConfig) { self.icon_config = config; self.update_icon(); }
    pub fn poll(&mut self) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Ok(map) = self.callbacks.lock() {
                if let Some(f) = map.get(&event.id) {
                    f();
                }
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button: tray_icon::MouseButton::Left, button_state: tray_icon::MouseButtonState::Up, .. } = event {
                let _ = self.tx.send(super::TrayCommand::ShowWindow);
            }
        }
    }

    pub fn update_battery(&mut self, percent: u8, charging: bool) {
        if !self.connected { return; }
        if self.last_percent != percent || self.last_charging != charging {
            self.last_percent = percent; self.last_charging = charging;
            self.update_tooltip(); self.update_icon(); self.rebuild_menu();
        }
    }

    pub fn update_mute(&mut self, muted: bool) {
        if self.last_muted != muted { self.last_muted = muted; self.update_tooltip(); self.rebuild_menu(); }
    }

    pub fn update_connected(&mut self, connected: bool) {
        if self.connected != connected {
            self.connected = connected;
            if !connected { self.last_percent = 0; self.last_charging = false; }
            self.update_tooltip(); self.update_icon(); self.rebuild_menu();
        }
    }
}