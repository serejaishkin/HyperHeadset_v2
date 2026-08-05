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

    pub fn refresh_icon(&self) {}
