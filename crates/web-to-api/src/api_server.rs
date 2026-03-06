//! REST API server placeholder

use axum::{Router, routing::get};

/// API server for extracted data
pub struct ApiServer;

impl ApiServer {
    pub fn new() -> Self {
        Self
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(|| async { "OK" }))
    }

    pub async fn run(&self, addr: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.router()).await?;
        Ok(())
    }
}

impl Default for ApiServer {
    fn default() -> Self {
        Self::new()
    }
}
