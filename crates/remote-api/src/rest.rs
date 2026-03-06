//! REST API placeholder

use axum::{Router, routing::get};

/// REST API server
pub struct RestApi;

impl RestApi {
    pub fn new() -> Self {
        Self
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/api/health", get(|| async { "OK" }))
    }

    pub async fn run(&self, addr: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.router()).await?;
        Ok(())
    }
}

impl Default for RestApi {
    fn default() -> Self {
        Self::new()
    }
}
