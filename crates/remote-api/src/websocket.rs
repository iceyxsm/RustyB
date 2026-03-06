//! WebSocket server placeholder

use crate::commands::RemoteCommand;
use std::net::SocketAddr;

/// WebSocket command server
pub struct WebSocketServer {
    addr: SocketAddr,
}

impl WebSocketServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("WebSocket server would run on: {}", self.addr);
        Ok(())
    }

    pub async fn broadcast(&self, _command: &RemoteCommand) -> anyhow::Result<()> {
        Ok(())
    }
}
