pub mod rpc_ws;

use anyhow::anyhow;
use discord_rich_presence::{
    activity::{Activity, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DiscordClient {
    client: Option<DiscordIpcClient>,
    start_time: i64,
    app_id: String,
}

impl DiscordClient {
    pub fn new(app_id: String) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            client: None,
            start_time,
            app_id,
        }
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let mut client = DiscordIpcClient::new(&self.app_id)
            .map_err(|e| anyhow!("Discord IPC create failed: {}", e))?;
        client.connect().map_err(|e| anyhow!("Discord IPC connect failed: {}", e))?;
        self.client = Some(client);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    pub fn set_mute(&mut self, muted: bool, battery: Option<u8>) {
        let Some(client) = self.client.as_mut() else { return };

        let state = if muted {
            "🔇 Микрофон выключен"
        } else {
            "🎤 В эфире"
        };

        let details = if let Some(bat) = battery {
            format!("HyperX Cloud II Wireless • 🔋 {}%", bat)
        } else {
            "HyperX Cloud II Wireless".to_string()
        };

        let large_image = if muted { "mute_icon" } else { "headset_icon" };
        let large_text = if muted { "Мьют" } else { "В эфире" };

        let activity = Activity::new()
            .details(&details)
            .state(state)
            .assets(
                Assets::new()
                    .large_image(large_image)
                    .large_text(large_text)
            )
            .timestamps(Timestamps::new().start(self.start_time));

        let _ = client.set_activity(activity);
    }

    pub fn clear(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let _ = client.clear_activity();
        }
    }
}

pub fn parse_keybind(keybind: &str) -> Vec<enigo::Key> {
    let mut keys = Vec::new();
    let parts: Vec<&str> = keybind.split('+').map(|s| s.trim()).collect();

    for part in &parts {
        let key = match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => enigo::Key::Control,
            "shift" => enigo::Key::Shift,
            "alt" => enigo::Key::Alt,
            "f20" => enigo::Key::F20,
            "mediamute" => enigo::Key::VolumeMute,
            "mediavolup" => enigo::Key::VolumeUp,
            "mediavoldown" => enigo::Key::VolumeDown,
            "mediaplay" => enigo::Key::MediaPlayPause,
            _ => {
                if let Some(c) = part.chars().next() {
                    enigo::Key::Unicode(c)
                } else {
                    continue;
                }
            }
        };
        keys.push(key);
    }
    keys
}
