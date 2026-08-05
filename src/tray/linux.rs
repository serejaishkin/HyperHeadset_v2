use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use ksni;
use super::{TrayCommand, icon::{TrayIconConfig, generate_battery_icon_rgba, rgba_to_argb32}};

#[derive(Clone)]
struct TrayState {
    percent: u8,
    charging: bool,
    muted: bool,
}

pub struct LinuxTray {
    state: Arc<Mutex<TrayState>>,
}

impl LinuxTray {
    pub fn new(tx: Sender<TrayCommand>) -> Self {
        let icon_config = TrayIconConfig::load_or_create();
        let state = Arc::new(Mutex::new(TrayState { percent: 100, charging: false, muted: false }));
        let service = ksni::TrayService::new(MyTray {
            tx: tx.clone(),
            state: state.clone(),
            icon_config,
        });
        std::thread::spawn(move || {
            let _handle = service.spawn();
            std::thread::park();
        });
        Self { state }
    }

    pub fn poll(&self) {}

    pub fn update_battery(&self, percent: u8, charging: bool) {
        let mut s = self.state.lock().unwrap();
        s.percent = percent;
        s.charging = charging;
    }

    pub fn refresh_icon(&mut self) {
        let mut s = self.state.lock().unwrap();
        let (rgba, w, h) = generate_battery_icon_rgba(&self.icon_config, s.percent, s.charging);
        s._rgba = rgba;
        s._w = w;
        s._h = h;
    }
    pub fn update_mute(&self, muted: bool) {
        let mut s = self.state.lock().unwrap();
        s.muted = muted;
    }
}

#[derive(Clone)]
struct MyTray {
    tx: Sender<TrayCommand>,
    state: Arc<Mutex<TrayState>>,
    icon_config: TrayIconConfig,
}

impl ksni::Tray for MyTray {
    fn title(&self) -> String {
        "HyperX NGENUITY Open".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let s = self.state.lock().unwrap();
        let (rgba, w, h) = generate_battery_icon_rgba(&self.icon_config, s.percent, s.charging);
        let data = rgba_to_argb32(&rgba);
        vec![ksni::Icon {
            width: w as i32,
            height: h as i32,
            data,
        }]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Открыть".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::ShowWindow);
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Переключить мьют".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::ToggleMute);
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Выход".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}
