//! Request/Response interceptor trait and implementations

pub mod adblock;
pub mod privacy;

use async_trait::async_trait;
use http::{Request, Response};
use hyper::body::Incoming;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Body type for intercepted requests/responses
pub type Body = Incoming;

/// Result type for interceptor operations
pub type InterceptorResult<T> = Result<T, InterceptorError>;

/// Error type for interceptor operations
#[derive(Debug, thiserror::Error)]
pub enum InterceptorError {
    #[error("Request blocked: {0}")]
    Blocked(String),
    
    #[error("Request modified")]
    Modified,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Trait for request/response interceptors
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// Intercept and potentially modify an incoming request
    /// 
    /// Returns:
    /// - Ok(()) if the request should proceed
    /// - Err(InterceptorError::Blocked) if the request should be blocked
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()>;
    
    /// Intercept and potentially modify an outgoing response
    /// 
    /// Returns:
    /// - Ok(()) if the response should proceed
    /// - Err(InterceptorError::Blocked) if the response should be blocked
    async fn intercept_response(&self, response: &mut Response<Body>) -> InterceptorResult<()>;
    
    /// Get the name of this interceptor
    fn name(&self) -> &str;
    
    /// Check if this interceptor is enabled
    fn is_enabled(&self) -> bool {
        true
    }
    
    /// Get the priority of this interceptor (higher = earlier execution)
    fn priority(&self) -> i32 {
        0
    }
}

/// Chain of interceptors that can be applied to requests/responses
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    /// Create a new empty interceptor chain
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }
    
    /// Add an interceptor to the chain
    pub fn add(&mut self, interceptor: Arc<dyn Interceptor>) {
        self.interceptors.push(interceptor);
        // Sort by priority (highest first)
        self.interceptors.sort_by_key(|i| -i.priority());
    }
    
    /// Remove an interceptor by name
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Interceptor>> {
        if let Some(pos) = self.interceptors.iter().position(|i| i.name() == name) {
            Some(self.interceptors.remove(pos))
        } else {
            None
        }
    }
    
    /// Get all interceptors
    pub fn interceptors(&self) -> &[Arc<dyn Interceptor>] {
        &self.interceptors
    }
    
    /// Clear all interceptors
    pub fn clear(&mut self) {
        self.interceptors.clear();
    }
    
    /// Process a request through all interceptors
    pub async fn process_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        for interceptor in &self.interceptors {
            if !interceptor.is_enabled() {
                continue;
            }
            
            debug!("Processing request with interceptor: {}", interceptor.name());
            
            match interceptor.intercept_request(request).await {
                Ok(()) => continue,
                Err(InterceptorError::Blocked(reason)) => {
                    info!("Request blocked by {}: {}", interceptor.name(), reason);
                    return Err(InterceptorError::Blocked(reason));
                }
                Err(e) => {
                    warn!("Interceptor {} error: {}", interceptor.name(), e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }
    
    /// Process a response through all interceptors
    pub async fn process_response(&self, response: &mut Response<Body>) -> InterceptorResult<()> {
        for interceptor in &self.interceptors {
            if !interceptor.is_enabled() {
                continue;
            }
            
            debug!("Processing response with interceptor: {}", interceptor.name());
            
            match interceptor.intercept_response(response).await {
                Ok(()) => continue,
                Err(InterceptorError::Blocked(reason)) => {
                    info!("Response blocked by {}: {}", interceptor.name(), reason);
                    return Err(InterceptorError::Blocked(reason));
                }
                Err(e) => {
                    warn!("Interceptor {} error: {}", interceptor.name(), e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Logging interceptor that logs all requests and responses
pub struct LoggingInterceptor {
    enabled: std::sync::atomic::AtomicBool,
}

impl LoggingInterceptor {
    pub fn new() -> Self {
        Self {
            enabled: std::sync::atomic::AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl Interceptor for LoggingInterceptor {
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        info!(
            "[REQUEST] {} {} {:?}",
            request.method(),
            request.uri(),
            request.headers().get("host").and_then(|h| h.to_str().ok())
        );
        Ok(())
    }
    
    async fn intercept_response(&self, response: &mut Response<Body>) -> InterceptorResult<()> {
        info!(
            "[RESPONSE] status={} content-type={:?}",
            response.status(),
            response.headers().get("content-type").and_then(|h| h.to_str().ok())
        );
        Ok(())
    }
    
    fn name(&self) -> &str {
        "logging"
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Modify interceptor that applies custom modifications
pub struct ModifyInterceptor {
    name: String,
    request_modifier: Option<Box<dyn Fn(&mut Request<Body>) + Send + Sync>>,
    response_modifier: Option<Box<dyn Fn(&mut Response<Body>) + Send + Sync>>,
}

impl ModifyInterceptor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            request_modifier: None,
            response_modifier: None,
        }
    }
    
    pub fn with_request_modifier<F>(mut self, modifier: F) -> Self
    where
        F: Fn(&mut Request<Body>) + Send + Sync + 'static,
    {
        self.request_modifier = Some(Box::new(modifier));
        self
    }
    
    pub fn with_response_modifier<F>(mut self, modifier: F) -> Self
    where
        F: Fn(&mut Response<Body>) + Send + Sync + 'static,
    {
        self.response_modifier = Some(Box::new(modifier));
        self
    }
}

#[async_trait]
impl Interceptor for ModifyInterceptor {
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        if let Some(ref modifier) = self.request_modifier {
            modifier(request);
        }
        Ok(())
    }
    
    async fn intercept_response(&self, response: &mut Response<Body>) -> InterceptorResult<()> {
        if let Some(ref modifier) = self.response_modifier {
            modifier(response);
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    
    struct TestInterceptor {
        name: String,
        should_block: bool,
    }
    
    #[async_trait]
    impl Interceptor for TestInterceptor {
        async fn intercept_request(&self, _request: &mut Request<Body>) -> InterceptorResult<()> {
            if self.should_block {
                Err(InterceptorError::Blocked("test".to_string()))
            } else {
                Ok(())
            }
        }
        
        async fn intercept_response(&self, _response: &mut Response<Body>) -> InterceptorResult<()> {
            Ok(())
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    #[test]
    fn test_interceptor_chain() {
        let mut chain = InterceptorChain::new();
        
        let interceptor = Arc::new(TestInterceptor {
            name: "test".to_string(),
            should_block: false,
        });
        
        chain.add(interceptor);
        assert_eq!(chain.interceptors().len(), 1);
    }
}
