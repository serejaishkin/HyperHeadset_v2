use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub keybind: String,
    pub enabled: bool,
    pub double_tap_ms: u64,
    pub start_with_os: bool,
    pub compact_mode: bool,
    pub language: String,
    pub debug_logging: bool,
    pub log_to_console: bool,
    pub log_to_file: bool,
    pub start_in_compact_mode: bool,
    pub audio: AudioConfig,
    pub device: DeviceConfig,
    pub voice: VoiceConfig,
    pub discord: DiscordConfig,
    pub input: InputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub eq_bands: [f32; 10],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub sidetone: bool,
    pub voice_prompts: bool,
    pub auto_shutdown_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
 #[derive(Default)]

pub struct VoiceConfig {
    pub enabled: bool,
    pub on_battery_low: bool,
    pub on_charging: bool,
    pub on_full_charge: bool,
    pub on_connected: bool,
    pub on_disconnected: bool,
    pub on_button_check: bool,
    pub exact_percent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub keybind: Option<String>,
    pub direct: DirectDiscordConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDiscordConfig {
    pub app_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub mute_button_mode: crate::input::MuteButtonMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybind: "F20".to_string(),
            enabled: true,
            double_tap_ms: 500,
            start_with_os: false,
            compact_mode: false,
            language: "ru".to_string(),
            debug_logging: false,
            log_to_console: true,
            log_to_file: false,
            start_in_compact_mode: false,
            audio: AudioConfig { eq_bands: [0.0; 10] },
            device: DeviceConfig {
                sidetone: false,
                voice_prompts: true,
                auto_shutdown_minutes: 30,
            },
            voice: VoiceConfig {
                enabled: true,
                on_battery_low: true,
                on_charging: true,
                on_full_charge: true,
                on_connected: true,
                on_disconnected: true,
                on_button_check: true,
                exact_percent: false,
            },
            discord: DiscordConfig {
                keybind: Some("F20".to_string()),
                direct: DirectDiscordConfig {
                    app_id: "1234567890123456789".to_string(),
                },
            },
            input: InputConfig {
                mute_button_mode: crate::input::MuteButtonMode::SmartDouble,
            },
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    pub fn load() -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(Self::path())?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::write(Self::path(), toml::to_string_pretty(self)?)?;
        Ok(())
    }
}