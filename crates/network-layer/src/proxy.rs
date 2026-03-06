//! HTTP Proxy placeholder

use std::net::SocketAddr;

/// HTTP Proxy server
pub struct ProxyServer {
    addr: SocketAddr,
}

impl ProxyServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("Proxy server would run on: {}", self.addr);
        Ok(())
    }
}
