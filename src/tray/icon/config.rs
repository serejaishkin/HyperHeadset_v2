use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrayIconMode {
    Big,
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
            mode: TrayIconMode::Big,
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
                Ok(cfg) => {
                    let mut cfg: TrayIconConfig = cfg;
                    cfg.sanitize();
                    log::info!("[TrayIcon] Loaded config: mode={:?} high.fg={:?} high.outline={:?} path={:?}",
                        cfg.mode, cfg.colors.high.fg, cfg.colors.high.outline, path);
                    cfg
                }
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
