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

    pub fn refresh_icon(&self) {}



    pub fn poll(&self) {}

    pub fn update_battery(&self, percent: u8, charging: bool) {
        let mut s = self.state.lock().unwrap();
        s.percent = percent;
        s.charging = charging;
    }
}

    pub fn update_mute(&self, muted: bool) {
        let mut s = self.state.lock().unwrap();
        s.muted = muted;
    }
}

struct MyTray {
    tx: Sender<TrayCommand>,
    state: Arc<Mutex<TrayState>>,
    icon_config: TrayIconConfig,
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        "hyperx-ngenuity-open".to_string()
    }
    fn title(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.charging {
            format!("HyperX — ⚡ {}%", s.percent)
        } else {
            format!("HyperX — 🔋 {}%", s.percent)
        }
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let s = self.state.lock().unwrap();
        let (rgba, w, h) = generate_battery_icon_rgba(&self.icon_config, s.percent, s.charging);
        let argb = rgba_to_argb32(&rgba, w, h);
        vec![ksni::Icon {
            width: w as i32,
            height: h as i32,
            data: argb,
        }]
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let s = self.state.lock().unwrap();
        let battery_text = if s.charging {
            format!("⚡ Заряд: {}%", s.percent)
        } else {
            format!("🔋 Батарея: {}%", s.percent)
        };
        let mic_text = if s.muted {
            "🔇 Микрофон: выкл"
        } else {
            "🎙️ Микрофон: вкл"
        };
        vec![
            ksni::MenuItem::Label(battery_text, Box::new(|_| {})),
            ksni::MenuItem::Label(mic_text.to_string(), Box::new(|_| {})),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Label("Открыть".to_string(), Box::new(|t: &mut Self| { let _ = t.tx.send(TrayCommand::ShowWindow); })),
            ksni::MenuItem::Label("Переключить мьют".to_string(), Box::new(|t: &mut Self| { let _ = t.tx.send(TrayCommand::ToggleMute); })),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Label("Выход".to_string(), Box::new(|t: &mut Self| { let _ = t.tx.send(TrayCommand::Quit); })),
        ]
    }
}
