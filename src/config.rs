use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub use crate::input::MuteButtonMode;

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
    /// How often (seconds) the headset is asked whether it is switched on.
    #[serde(default = "default_status_poll_secs")]
    pub status_poll_secs: u32,
    /// How long (seconds) the headset must stay off before audio is switched to the fallback.
    #[serde(default = "default_offline_debounce_secs")]
    pub offline_debounce_secs: u32,
    pub audio: AudioConfig,
    pub device: DeviceConfig,
    pub voice: VoiceConfig,
    pub discord: DiscordConfig,
    pub input: InputConfig,
    #[serde(default)]
    pub per_device: HashMap<String, PerDeviceConfig>,
    #[serde(default)]
    pub custom_voice_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerDeviceConfig {
    #[serde(default)]
    pub name: String,
    pub audio: Option<AudioConfig>,
    pub device: Option<DeviceConfig>,
    pub voice: Option<VoiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub eq_bands: [f32; 10],
    #[serde(default)]
    pub fallback_output_device: Option<String>,
    #[serde(default)]
    pub fallback_input_device: Option<String>,
    #[serde(default)]
    pub headset_output_device: Option<String>,
    #[serde(default)]
    pub headset_input_device: Option<String>,
    #[serde(default)]
    pub auto_switch_audio: bool,
    /// After a default-device switch, also move apps' running streams to the new output.
    #[serde(default = "default_true")]
    pub move_app_streams: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub sidetone: bool,
    pub voice_prompts: bool,
    pub auto_shutdown_minutes: u32,
    /// Headset-originated sidetone event -> media Play/Pause.
    #[serde(rename = "sidetone_playpause", default = "default_true")]
    pub sidetone_play_pause: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub on_battery_low: bool,
    pub on_charging: bool,
    pub on_full_charge: bool,
    pub on_connected: bool,
    pub on_disconnected: bool,
    pub on_button_check: bool,
    pub exact_percent: bool,
    /// Pause between two queued voice announcements, in milliseconds.
    #[serde(default = "default_announce_gap_ms")]
    pub announce_gap_ms: u32,
}

fn default_announce_gap_ms() -> u32 {
    700
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DiscordMode {
    None,
    Keybind,
    Direct,
    /// Direct (RPC) and the keybind together.
    Both,
}

fn default_status_poll_secs() -> u32 {
    5
}

fn default_offline_debounce_secs() -> u32 {
    2
}

impl AudioConfig {
    /// True when the app should watch the headset's on/off state closely (fast audio switching).
    pub fn wants_fast_status(&self) -> bool {
        self.auto_switch_audio && (self.fallback_output_device.is_some() || self.fallback_input_device.is_some())
    }
}

impl DiscordMode {
    pub fn uses_keybind(self) -> bool {
        matches!(self, DiscordMode::Keybind | DiscordMode::Both)
    }
    pub fn uses_direct(self) -> bool {
        matches!(self, DiscordMode::Direct | DiscordMode::Both)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub mode: DiscordMode,
    /// Direct mode: when Discord's mic mute changes, make the headset's mute follow it.
    #[serde(default)]
    pub sync_from_discord: bool,
    pub keybind: Option<String>,
    pub direct: DirectDiscordConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDiscordConfig {
    pub app_id: String,
    /// OAuth client secret of the user's Discord application (needed for the RPC authorization).
    #[serde(default)]
    pub client_secret: String,
    pub show_battery: bool,
    pub show_mute_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub mute_button_mode: MuteButtonMode,
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
            status_poll_secs: default_status_poll_secs(),
            offline_debounce_secs: default_offline_debounce_secs(),
            audio: AudioConfig {
                eq_bands: [0.0; 10],
                fallback_output_device: None,
                fallback_input_device: None,
                headset_output_device: None,
                headset_input_device: None,
                auto_switch_audio: false,
                move_app_streams: true,
            },
            device: DeviceConfig {
                sidetone: false,
                voice_prompts: true,
                auto_shutdown_minutes: 30,
                sidetone_play_pause: true,
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
                announce_gap_ms: default_announce_gap_ms(),
            },
            discord: DiscordConfig {
                mode: DiscordMode::Keybind,
                sync_from_discord: false,
                keybind: Some("F20".to_string()),
                direct: DirectDiscordConfig {
                    app_id: "1234567890123456789".to_string(),
                    client_secret: String::new(),
                    show_battery: false,
                    show_mute_status: false,
                },
            },
            input: InputConfig {
                mute_button_mode: MuteButtonMode::SmartDouble,
            },
            per_device: HashMap::new(),
            custom_voice_dir: None,
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

    pub fn get_per_device(&self, device_id: &str) -> Option<&PerDeviceConfig> {
        self.per_device.get(device_id)
    }

    pub fn upsert_per_device(&mut self, device_id: String, cfg: PerDeviceConfig) {
        self.per_device.insert(device_id, cfg);
    }

    pub fn effective_audio(&self, device_id: &str) -> AudioConfig {
        self.per_device.get(device_id).and_then(|p| p.audio.clone()).unwrap_or_else(|| self.audio.clone())
    }
}

fn default_true() -> bool {
    true
}
