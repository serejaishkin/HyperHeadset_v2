use image::{RgbaImage, Rgba};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrayIconMode {
    Icon,
    Digits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayIconConfig {
    pub mode: TrayIconMode,
    pub size: u32,
    pub font_scale: u32,
    pub outline_width: i32,
    pub border_width: u32,
    pub gap_between_digits: u32,
    pub colors: TrayIconColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayIconColors {
    pub charging: IconColors,
    pub high: IconColors,
    pub medium: IconColors,
    pub low: IconColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconColors {
    pub bg: [u8; 4],
    pub fg: [u8; 4],
    pub outline: [u8; 4],
    pub border: [u8; 4],
}

impl Default for TrayIconConfig {
    fn default() -> Self {
        Self {
            mode: TrayIconMode::Icon,
            size: 256,
            font_scale: 8,
            outline_width: 2,
            border_width: 0,
            gap_between_digits: 4,
            colors: TrayIconColors {
                charging: IconColors {
                    bg: [255, 200, 0, 255],
                    fg: [20, 20, 20, 255],
                    outline: [255, 255, 255, 255],
                    border: [180, 140, 0, 255],
                },
                high: IconColors {
                    bg: [0, 180, 80, 255],
                    fg: [255, 255, 255, 255],
                    outline: [10, 10, 10, 255],
                    border: [0, 110, 50, 255],
                },
                medium: IconColors {
                    bg: [255, 140, 0, 255],
                    fg: [50, 25, 0, 255],
                    outline: [255, 255, 255, 255],
                    border: [180, 90, 0, 255],
                },
                low: IconColors {
                    bg: [230, 40, 40, 255],
                    fg: [255, 255, 255, 255],
                    outline: [10, 10, 10, 255],
                    border: [150, 20, 20, 255],
                },
            },
        }
    }
}

impl TrayIconConfig {
    pub fn sanitize(&mut self) {
        self.size = self.size.clamp(16, 512);
        self.font_scale = self.font_scale.clamp(1, 20);
        self.outline_width = self.outline_width.clamp(0, 10);
        self.border_width = self.border_width.clamp(0, 20);
        self.gap_between_digits = self.gap_between_digits.clamp(0, 50);
        let all_colors = [
            &mut self.colors.charging,
            &mut self.colors.high,
            &mut self.colors.medium,
            &mut self.colors.low,
        ];
        for c in all_colors {
            for arr in [&mut c.bg, &mut c.fg, &mut c.outline, &mut c.border] {
                for v in arr.iter_mut() { *v = (*v).min(255); }
            }
        }
    }

    pub fn default_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("tray_icon.toml")))
            .unwrap_or_else(|| PathBuf::from("tray_icon.toml"))
    }

    pub fn load_or_create() -> Self {
        let path = Self::default_path();
        if !path.exists() {
            log::info!("[TrayIcon] Config not found, creating default at {:?}", path);
            let cfg = Self::default();
            let _ = cfg.save(&path);
            return cfg;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => { let mut cfg: TrayIconConfig = cfg; cfg.sanitize(); cfg }
                Err(e) => {
                    log::warn!("[TrayIcon] Bad config file, recreating default: {}", e);
                    let cfg = Self::default();
                    let _ = cfg.save(&path);
                    cfg
                }
            },
            Err(e) => {
                log::warn!("[TrayIcon] Cannot read config, using default: {}", e);
                Self::default()
            }
        }
    }

    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut cfg = self.clone();
        cfg.sanitize();
        let content = toml::to_string_pretty(&cfg)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub fn generate_battery_icon_rgba(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> (Vec<u8>, u32, u32) {
    let size = config.size;
    let mut img = RgbaImage::new(size, size);

    let scheme = if charging {
        &config.colors.charging
    } else if percent > 50 {
        &config.colors.high
    } else if percent > 20 {
        &config.colors.medium
    } else {
        &config.colors.low
    };

    let bg = Rgba(scheme.bg);
    let fg = Rgba(scheme.fg);
    let outline = Rgba(scheme.outline);
    let border = Rgba(scheme.border);

    for pixel in img.pixels_mut() { *pixel = bg; }

    let bw = config.border_width;
    if bw > 0 {
        for y in 0..size {
            for x in 0..size {
                if x < bw || x >= size - bw || y < bw || y >= size - bw {
                    img.put_pixel(x, y, border);
                }
            }
        }
    }

    let digits: [[u8; 7]; 10] = [
        [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        [0b01110, 0b10001, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b10001, 0b01110],
    ];

    let scale = config.font_scale;
    let outline_px = config.outline_width;
    let digit_w = 5 * scale;
    let digit_h = 7 * scale;
    let gap = config.gap_between_digits;

    let text = format!("{}", percent);
    let chars: Vec<char> = text.chars().collect();
    let total_w = chars.len() as u32 * digit_w + chars.len().saturating_sub(1) as u32 * gap;
    let start_x = size.saturating_sub(total_w) / 2;
    let start_y = size.saturating_sub(digit_h) / 2;

    if outline_px > 0 {
        for (idx, ch) in chars.iter().enumerate() {
            let d = ch.to_digit(10).unwrap_or(0) as usize;
            let digit = digits[d];
            let off_x = start_x + idx as u32 * (digit_w + gap);
            for row in 0..7u32 {
                for col in 0..5u32 {
                    if (digit[row as usize] >> (4 - col)) & 1 == 1 {
                        for dy in -outline_px..=outline_px {
                            for dx in -outline_px..=outline_px {
                                let xi = off_x as i32 + col as i32 * scale as i32 + dx;
                                let yi = start_y as i32 + row as i32 * scale as i32 + dy;
                                if xi >= 0 && yi >= 0 {
                                    let x = xi as u32; let y = yi as u32;
                                    if x < size && y < size { img.put_pixel(x, y, outline); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for (idx, ch) in chars.iter().enumerate() {
        let d = ch.to_digit(10).unwrap_or(0) as usize;
        let digit = digits[d];
        let off_x = start_x + idx as u32 * (digit_w + gap);
        for row in 0..7u32 {
            for col in 0..5u32 {
                if (digit[row as usize] >> (4 - col)) & 1 == 1 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = off_x + col * scale + dx;
                            let y = start_y + row * scale + dy;
                            if x < size && y < size { img.put_pixel(x, y, fg); }
                        }
                    }
                }
            }
        }
    }

    let rgba = img.into_raw();
    (rgba, size, size)
}

/// PNG export for Tauri tray
pub fn generate_battery_icon_png(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> anyhow::Result<Vec<u8>> {
    let (rgba, w, h) = generate_battery_icon_rgba(config, percent, charging);
    let img = image::RgbaImage::from_raw(w, h, rgba)
        .ok_or_else(|| anyhow::anyhow!("Invalid image buffer"))?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;
    Ok(buf)
}