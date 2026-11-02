use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrayIconMode {
    #[default]
    Big,
    Digits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrayIconConfig {
    #[serde(default)]
    pub mode: TrayIconMode,
    pub size: u32,
    pub font_scale: u32,
    pub outline_width: i32,
    pub border_width: u32,
    pub gap_between_digits: u32,
    /// Charge above this percent uses the "high" colors.
    #[serde(default = "default_high_threshold")]
    pub high_threshold: u8,
    /// Charge above this percent (and not above high) uses "medium"; at or below it uses "low".
    #[serde(default = "default_low_threshold")]
    pub low_threshold: u8,
    pub colors: TrayIconColors,
}

fn default_high_threshold() -> u8 { 50 }
fn default_low_threshold() -> u8 { 20 }

/// Pass this as the percent to draw the "headset off / not connected" icon ("--").
pub const OFFLINE_PERCENT: u8 = 255;

fn default_offline_colors() -> IconColors {
    IconColors {
        bg: [90, 90, 90, 255],
        fg: [210, 210, 210, 255],
        outline: [20, 20, 20, 255],
        border: [60, 60, 60, 255],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrayIconColors {
    pub charging: IconColors,
    pub high: IconColors,
    pub medium: IconColors,
    pub low: IconColors,
    #[serde(default = "default_offline_colors")]
    pub offline: IconColors,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            high_threshold: 50,
            low_threshold: 20,
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
                offline: default_offline_colors(),
            },
        }
    }
}

/// The size the OS shows tray icons at (small-icon metric), if known.
pub fn native_tray_size() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetSystemMetrics(index: i32) -> i32;
        }
        let v = unsafe { GetSystemMetrics(49) }; // SM_CXSMICON
        if v >= 16 {
            return Some(v as u32);
        }
    }
    None
}

impl TrayIconConfig {
    /// Copy sized for the real tray: rendered at the OS tray-icon size (crisp, no downscaling blur).
    pub fn for_tray(&self) -> TrayIconConfig {
        let mut c = self.clone();
        c.size = native_tray_size().unwrap_or_else(|| self.size.min(48)).clamp(16, 64);
        c
    }

    /// Color scheme for a status: offline, charging, or by charge level using the thresholds.
    pub fn scheme_for(&self, percent: u8, charging: bool) -> &IconColors {
        if percent == OFFLINE_PERCENT {
            &self.colors.offline
        } else if charging {
            &self.colors.charging
        } else if percent > self.high_threshold {
            &self.colors.high
        } else if percent > self.low_threshold {
            &self.colors.medium
        } else {
            &self.colors.low
        }
    }

    pub fn sanitize(&mut self) {
        self.high_threshold = self.high_threshold.clamp(1, 100);
        self.low_threshold = self.low_threshold.min(self.high_threshold.saturating_sub(1));
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
            &mut self.colors.offline,
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
