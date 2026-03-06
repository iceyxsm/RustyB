//! Request/Response logger placeholder

use shared::{Request, Response};

/// Trait for logging requests and responses
#[async_trait::async_trait]
pub trait RequestLogger: Send + Sync {
    async fn log_request(&self, request: &Request);
    async fn log_response(&self, response: &Response);
}

/// Default logger that uses tracing
pub struct TracingLogger;

#[async_trait::async_trait]
impl RequestLogger for TracingLogger {
    async fn log_request(&self, request: &Request) {
        tracing::debug!(
            "[REQUEST] {:?} {} - {:?}",
            request.method, request.url, request.id
        );
    }

    async fn log_response(&self, response: &Response) {
        tracing::debug!(
            "[RESPONSE] {} {} - {:?}",
            response.status_code, response.status_text, response.request_id
        );
    }
}
