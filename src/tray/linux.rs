use std::sync::mpsc::Sender;
use ksni;

pub struct LinuxTray {
    _service: ksni::TrayService<MyTray>,
    tx: Sender<super::TrayCommand>,
}

impl LinuxTray {
    pub fn new(tx: Sender<super::TrayCommand>) -> Self {
        let service = ksni::TrayService::new(MyTray { tx: tx.clone() });
        service.spawn();
        Self { _service: service, tx }
    }

    pub fn poll(&self) {}

    pub fn update_battery(&self, _percent: u8, _charging: bool) {}

    pub fn update_mute(&self, _muted: bool) {}
}

struct MyTray {
    tx: Sender<super::TrayCommand>,
}

impl ksni::Tray for MyTray {
    fn title(&self) -> String { "HyperX NGENUITY Open".into() }
    fn icon_name(&self) -> String { "audio-headset".into() }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            ksni::MenuItem::Standard(ksni::StandardItem {
                label: "Открыть".into(),
                activate: Some(Box::new(|this: &mut Self| { let _ = this.tx.send(super::TrayCommand::ShowWindow); })),
                ..Default::default()
            }),
            ksni::MenuItem::Standard(ksni::StandardItem {
                label: "Переключить мьют".into(),
                activate: Some(Box::new(|this: &mut Self| { let _ = this.tx.send(super::TrayCommand::ToggleMute); })),
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(ksni::StandardItem {
                label: "Выход".into(),
                activate: Some(Box::new(|this: &mut Self| { let _ = this.tx.send(super::TrayCommand::Quit); })),
                ..Default::default()
            }),
        ]
    }
}