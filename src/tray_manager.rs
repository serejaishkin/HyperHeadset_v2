use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, MenuId},
};
use hyperx_ngenuity_open::device::DeviceState;
use hyperx_ngenuity_open::config::TrayConfig;
use crate::tray_battery_icon_state::{TrayBatteryIconState, WindowsIconKey};

const SZ: u32 = 16;

fn rect(img: &mut image::RgbaImage, x: i32, y: i32, w: i32, h: i32, c: image::Rgba<u8>) {
    for px in x.max(0)..(x+w).min(SZ as i32) {
        for py in y.max(0)..(y+h).min(SZ as i32) {
            img.put_pixel(px as u32, py as u32, c);
        }
    }
}

fn digit(img: &mut image::RgbaImage, d: char, x: i32, y: i32, s: i32, c: image::Rgba<u8>) {
    let rows = match d {
        '0'=>&["111","101","101","101","111"], '1'=>&["01","01","01","01","01"],
        '2'=>&["111","001","111","100","111"], '3'=>&["111","001","111","001","111"],
        '4'=>&["101","101","111","001","001"], '5'=>&["111","100","111","001","111"],
        '6'=>&["111","100","111","101","111"], '7'=>&["111","001","010","010","010"],
        '8'=>&["111","101","111","101","111"], '9'=>&["111","101","111","001","111"],
        _ =>&["000","000","000","000","000"],
    };
    for (ri,row) in rows.iter().enumerate() {
        for (ci,b) in row.chars().enumerate() {
            if b=='1' { rect(img, x+(ci as i32*s), y+(ri as i32*s), s, s, c); }
        }
    }
}

fn render_battery_icon(key: WindowsIconKey, cfg: &TrayConfig) -> Vec<u8> {
    let mut img = image::RgbaImage::from_pixel(SZ, SZ, image::Rgba([0,0,0,0]));
    let bg = if key.charging { 
        image::Rgba([cfg.color_charging[0], cfg.color_charging[1], cfg.color_charging[2], 255]) 
    } else if key.percent < 30 { 
        image::Rgba([cfg.color_low[0], cfg.color_low[1], cfg.color_low[2], 255]) 
    } else { 
        image::Rgba([cfg.color_high[0], cfg.color_high[1], cfg.color_high[2], 255]) 
    };
    rect(&mut img, 0, 0, SZ as i32, SZ as i32, bg);

    if key.percent == 100 {
        let tc = image::Rgba([10,10,10,255]); let y=3;
        rect(&mut img, 1, y, 1, 10, tc); rect(&mut img, 0, y+9, 3, 1, tc);
        let z1=4;
        rect(&mut img, z1, y, 5, 1, tc); rect(&mut img, z1, y+9, 5, 1, tc);
        rect(&mut img, z1, y, 1, 10, tc); rect(&mut img, z1+4, y, 1, 10, tc);
        let z2=10;
        rect(&mut img, z2, y, 5, 1, tc); rect(&mut img, z2, y+9, 5, 1, tc);
        rect(&mut img, z2, y, 1, 10, tc); rect(&mut img, z2+4, y, 1, 10, tc);
        return img.into_raw();
    }

    let text = key.percent.to_string();
    let mut scale = 2;
    let sp = if text.len()>=3 {0} else {1};
    let hp = if text.len()>=3 {0} else {1};
    let il = hp;
    let ir = (SZ as i32-1-hp).max(il);
    let us = (ir-il+1).max(1);

    let mut widths: Vec<i32> = text.chars().map(|d| if d=='1'{2*scale}else{3*scale}).collect();
    let mut total = widths.iter().sum::<i32>() + sp*(text.len().saturating_sub(1) as i32);
    if total>us && scale>1 {
        scale=1;
        widths = text.chars().map(|d| if d=='1'{2*scale}else{3*scale}).collect();
        total = widths.iter().sum::<i32>() + sp*(text.len().saturating_sub(1) as i32);
    }
    let sx = (il+((us-total).max(0)/2)).clamp(il, (ir-total+1).max(il));
    let sy = if scale==2{3}else{5};
    let tc = image::Rgba([10,10,10,255]);
    let mut x = sx;
    for (idx,d) in text.chars().enumerate() {
        digit(&mut img, d, x, sy, scale, tc);
        x += widths[idx] + sp;
    }
    img.into_raw()
}

fn default_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/headphone.png");
    let img = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (w,h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).unwrap()
}

pub struct TrayBatteryManager {
    tray: Option<TrayIcon>,
    cache: HashMap<WindowsIconKey, Vec<u8>>,
    current_key: Option<WindowsIconKey>,
    config: TrayConfig,
    show_id: MenuId,
    mute_id: MenuId,
    quit_id: MenuId,
}

impl TrayBatteryManager {
    pub fn new(config: TrayConfig) -> Self {
        let menu = Menu::new();
        let show_i = MenuItem::new("Show", true, None);
        let mute_i = MenuItem::new("Toggle Mute", true, None);
        let quit_i = MenuItem::new("Quit", true, None);
        
        let show_id = show_i.id().clone();
        let mute_id = mute_i.id().clone();
        let quit_id = quit_i.id().clone();
        
        menu.append(&show_i).unwrap();
        menu.append(&mute_i).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&quit_i).unwrap();

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(default_icon())
            .with_tooltip("HyperX NGENUITY Open")
            .build()
            .ok();

        Self { tray, cache: HashMap::new(), current_key: None, config, show_id, mute_id, quit_id }
    }

    pub fn update(&mut self, state: Option<&DeviceState>) {
        if !self.config.show_battery_percentage {
            if let Some(tray) = self.tray.as_ref() {
                let _ = tray.set_icon(Some(default_icon()));
            }
            return;
        }

        let st = TrayBatteryIconState::from_device_state(state);
        let desired = st.windows_icon_key();
        if desired == self.current_key { return; }
        let Some(tray) = self.tray.as_ref() else { return; };

        if let Some(key) = desired {
            let rgba = self.cache.entry(key).or_insert_with(|| render_battery_icon(key, &self.config)).clone();
            if let Ok(icon) = tray_icon::Icon::from_rgba(rgba, 16, 16) {
                let _ = tray.set_icon(Some(icon));
            }
            let tip = if key.charging { format!("HyperX — {}% (charging)", key.percent) }
                        else { format!("HyperX — {}% battery", key.percent) };
            let _ = tray.set_tooltip(Some(&tip));
        } else {
            let _ = tray.set_icon(Some(default_icon()));
            let tip = if state.map(|s| s.connected).unwrap_or(false) { "HyperX — Battery unknown" }
                        else { "HyperX NGENUITY Open — No device" };
            let _ = tray.set_tooltip(Some(tip));
        }
        self.current_key = desired;
    }

    pub fn run_event_loop(&self, device_cmd_tx: std::sync::mpsc::Sender<hyperx_ngenuity_open::DeviceCommand>) {
        let show_id = self.show_id.clone();
        let mute_id = self.mute_id.clone();
        let quit_id = self.quit_id.clone();
        
        std::thread::spawn(move || {
            let menu_channel = MenuEvent::receiver();
            let tray_channel = TrayIconEvent::receiver();
            loop {
                if let Ok(event) = menu_channel.try_recv() {
                    if event.id == show_id {
                        // Show window — handled by tray click
                    } else if event.id == mute_id {
                        let _ = device_cmd_tx.send(hyperx_ngenuity_open::DeviceCommand::ToggleMute);
                    } else if event.id == quit_id {
                        std::process::exit(0);
                    }
                }
                if let Ok(event) = tray_channel.try_recv() {
                    if let TrayIconEvent::Click { .. } = event {
                        #[cfg(target_os = "windows")]
                        {
                            let title: Vec<u16> = "HyperX NGENUITY Open\0".encode_utf16().collect();
                            unsafe {
                                use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};
                                use windows::core::PCWSTR;
                                if let Ok(hwnd) = FindWindowW(None, PCWSTR(title.as_ptr())) {
                                    if !hwnd.0.is_null() {
                                        let _ = ShowWindow(hwnd, SW_RESTORE);
                                        let _ = SetForegroundWindow(hwnd);
                                    }
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
    }
}

pub fn spawn_tray_battery_thread(
    shared_state: Arc<Mutex<Option<DeviceState>>>,
    config: TrayConfig,
    device_cmd_tx: std::sync::mpsc::Sender<hyperx_ngenuity_open::DeviceCommand>,
) {
    let interval = std::time::Duration::from_secs(config.refresh_interval_secs);
    std::thread::spawn(move || {
        let mut manager = TrayBatteryManager::new(config);
        manager.run_event_loop(device_cmd_tx);
        loop {
            if let Ok(lock) = shared_state.lock() {
                manager.update(lock.as_ref());
            }
            std::thread::sleep(interval);
        }
    });
}
