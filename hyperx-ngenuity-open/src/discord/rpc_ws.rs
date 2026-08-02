//! Discord Local RPC WebSocket client
//!
//! Connects to Discord's local RPC endpoint (ws://127.0.0.1:6463)
//! to receive VOICE_STATE_UPDATE events for bidirectional mute sync.
//!
//! This is an unofficial API. Discord may change it without notice.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};

const DISCORD_RPC_WS: &str = "ws://127.0.0.1:6463/?v=1&client_id=";

#[derive(Debug, Clone)]
pub struct DiscordVoiceState {
    pub mute: bool,
    pub deaf: bool,
    pub self_mute: bool,
    pub self_deaf: bool,
}

pub struct DiscordRPCClient {
    app_id: String,
    voice_state: Arc<Mutex<Option<DiscordVoiceState>>>,
    connected: Arc<Mutex<bool>>,
}

impl DiscordRPCClient {
    pub fn new(app_id: String) -> Self {
        Self {
            app_id,
            voice_state: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
        }
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    pub fn get_voice_state(&self) -> Option<DiscordVoiceState> {
        self.voice_state.lock().unwrap().clone()
    }

    pub async fn connect(&self) -> anyhow::Result<()> {
        let url = format!("{}{}", DISCORD_RPC_WS, self.app_id);
        let (ws_stream, _) = connect_async(&url).await?;

        *self.connected.lock().unwrap() = true;
        log::info!("[DiscordRPC] Connected to {}", url);

        let voice_state = self.voice_state.clone();
        let connected = self.connected.clone();

        tokio::spawn(async move {
            let (mut write, mut read) = ws_stream.split();

            // Subscribe to voice state updates
            let subscribe_msg = serde_json::json!({
                "cmd": "SUBSCRIBE",
                "evt": "VOICE_STATE_UPDATE",
                "nonce": "initial_subscribe"
            });
            let _ = write.send(Message::Text(subscribe_msg.to_string())).await;

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(event) = serde_json::from_str::<RpcEvent>(&text) {
                            if event.evt == Some("VOICE_STATE_UPDATE".to_string()) {
                                if let Some(data) = event.data {
                                    let state = DiscordVoiceState {
                                        mute: data.mute.unwrap_or(false),
                                        deaf: data.deaf.unwrap_or(false),
                                        self_mute: data.self_mute.unwrap_or(false),
                                        self_deaf: data.self_deaf.unwrap_or(false),
                                    };
                                    *voice_state.lock().unwrap() = Some(state);
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        log::warn!("[DiscordRPC] Connection closed");
                        break;
                    }
                    Err(e) => {
                        log::error!("[DiscordRPC] WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            *connected.lock().unwrap() = false;
        });

        Ok(())
    }

    pub async fn disconnect(&self) {
        *self.connected.lock().unwrap() = false;
    }
}

#[derive(Debug, Deserialize)]
struct RpcEvent {
    cmd: Option<String>,
    evt: Option<String>,
    data: Option<VoiceStateData>,
}

#[derive(Debug, Deserialize)]
struct VoiceStateData {
    mute: Option<bool>,
    deaf: Option<bool>,
    self_mute: Option<bool>,
    self_deaf: Option<bool>,
}

// Wrapper that bridges IPC Rich Presence + WS RPC
pub struct DiscordFullClient {
    pub rpc_ws: DiscordRPCClient,
    pub rich_presence: Option<crate::discord::DiscordClient>,
}

impl DiscordFullClient {
    pub fn new(app_id: String) -> Self {
        Self {
            rpc_ws: DiscordRPCClient::new(app_id.clone()),
            rich_presence: None,
        }
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.rpc_ws.connect().await?;

        // Also connect Rich Presence
        let mut rp = crate::discord::DiscordClient::new(self.rpc_ws.app_id.clone());
        rp.connect()?;
        self.rich_presence = Some(rp);

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.rpc_ws.is_connected()
    }

    pub fn get_mute_state(&self) -> Option<bool> {
        self.rpc_ws.get_voice_state().map(|s| s.self_mute || s.mute)
    }

    pub fn set_rich_presence(&mut self, muted: bool, battery: Option<u8>) {
        if let Some(rp) = self.rich_presence.as_mut() {
            rp.set_mute(muted, battery);
        }
    }
}
