use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscordMode {
    None,
    Keybind,
    Direct,
}

impl Default for DiscordMode {
    fn default() -> Self { DiscordMode::Keybind }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDiscordConfig {
    pub app_id: String,
    #[serde(default = "default_true")]
    pub show_battery: bool,
    #[serde(default = "default_true")]
    pub show_mute_status: bool,
}

impl Default for DirectDiscordConfig {
    fn default() -> Self {
        Self {
            app_id: "1234567890123456789".to_string(),
            show_battery: true,
            show_mute_status: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default)]
    pub mode: DiscordMode,
    pub keybind: Option<String>,
    #[serde(default)]
    pub direct: DirectDiscordConfig,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            mode: DiscordMode::Keybind,
            keybind: Some("F20".to_string()),
            direct: DirectDiscordConfig::default(),
        }
    }
}

// ===== Mute button action modes =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub exact_percent: bool,
    #[serde(default = "default_true")]
    pub on_battery_low: bool,
    #[serde(default = "default_true")]
    pub on_charging: bool,
    #[serde(default = "default_true")]
    pub on_full_charge: bool,
    #[serde(default)]
    pub on_connected: bool,
    #[serde(default)]
    pub on_disconnected: bool,
    #[serde(default = "default_true")]
    pub on_button_check: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            exact_percent: false,
            on_battery_low: true,
            on_charging: true,
            on_full_charge: true,
            on_connected: false,
            on_disconnected: false,
            on_button_check: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuteButtonMode {
    Standard,
    MediaPlayPause,
    SmartDouble,
    SmartHold,
    HoldPlayPause,
}

impl Default for MuteButtonMode {
    fn default() -> Self { MuteButtonMode::SmartDouble }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    #[serde(default)]
    pub mute_button_mode: MuteButtonMode,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            mute_button_mode: MuteButtonMode::SmartDouble,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default = "default_true")]
    pub system_eq_enabled: bool,
    pub eq_preset: String,
    pub eq_bands: [f32; 10],
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            system_eq_enabled: true,
            eq_preset: "flat".to_string(),
            eq_bands: [0.0; 10],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    #[serde(default = "default_true")]
    pub sidetone: bool,
    pub auto_shutdown_minutes: u8,
    #[serde(default = "default_true")]
    pub voice_prompts: bool,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            sidetone: true,
            auto_shutdown_minutes: 30,
            voice_prompts: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub debug_logging: bool,\n    #[serde(default)]\n    pub language: String,
    #[serde(default = "default_true")]
    pub log_to_console: bool,
    #[serde(default)]
    pub log_to_file: bool,
    #[serde(default)]
    pub voice: VoiceConfig,
    #[serde(default)]
    pub input: InputConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub device: DeviceConfig,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn path() -> anyhow::Result<PathBuf> {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let portable = dir.join("config.toml");
                if portable.exists() {
                    return Ok(portable);
                }
                let test = dir.join(".write_test_tmp");
                if std::fs::File::create(&test).is_ok() {
                    let _ = std::fs::remove_file(&test);
                    return Ok(portable);
                }
            }
        }
        let mut path = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("No config dir"))?;
        path.push("hyperx-ngenuity-open");
        path.push("config.toml");
        Ok(path)
    }
}

fn default_true() -> bool { true }
