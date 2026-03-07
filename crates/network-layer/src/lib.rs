//! Network layer with MITM proxy capabilities
//!
//! This crate provides a production-grade network layer with:
//! - MITM HTTPS proxy with TLS 1.3 interception
//! - DNS resolution with Hickory DNS (DoH, DoT, DoQ)
//! - HTTP/2 and HTTP/3 support
//! - WebSocket proxying
//! - Request/response interception
//! - Ad blocking with EasyList support
//! - Privacy protection
//!
//! # Example
//!
//! ```rust,no_run
//! use network_layer::proxy::{MitmProxy, ProxyConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ProxyConfig::default();
//!     let proxy = MitmProxy::new(config).await?;
//!     proxy.run().await
//! }
//! ```

pub mod dns;
pub mod filter;
pub mod interceptor;
pub mod logger;
pub mod proxy;
pub mod tls;

// Re-export main types
pub use dns::*;
pub use filter::*;
pub use interceptor::*;
pub use logger::*;
pub use proxy::*;

/// Version of the network-layer crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the network layer with tracing
pub fn init() {
    tracing_subscriber::fmt::init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
