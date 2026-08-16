use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

pub struct DiscordRPCClient {
    pub app_id: String,
    pub running: Arc<AtomicBool>,
    pub cmd_tx: Option<mpsc::UnboundedSender<RPCCommand>>,
}

#[derive(Debug, Clone)]
pub enum RPCCommand {
    SetActivity(String),
    ClearActivity,
    Disconnect,
}

impl DiscordRPCClient {
    pub fn new(app_id: String) -> Self {
        Self { app_id, running: Arc::new(AtomicBool::new(false)), cmd_tx: None }
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        log::info!("[DiscordRPC] Connecting to Discord IPC with app_id={}", self.app_id);
        self.running.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn disconnect(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}