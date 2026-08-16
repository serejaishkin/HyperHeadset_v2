pub mod rpc_ws;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct DiscordRPC {
    pub app_id: String,
    pub connected: Arc<AtomicBool>,
}

impl DiscordRPC {
    pub fn new(app_id: String) -> Self {
        Self { app_id, connected: Arc::new(AtomicBool::new(false)) }
    }

    pub fn connect(&self) -> anyhow::Result<()> {
        log::info!("[DiscordRPC] Connecting with app_id={}", self.app_id);
        self.connected.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
    }
}