//! HTTP request/response interceptor

use crate::filter::{FilterAction, FilterEngine, Modification};
use async_trait::async_trait;
use reqwest::{Client, Request, Response};
use reqwest_middleware::{Middleware, Next};
use shared::{Request as BrowserRequest, Response as BrowserResponse};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Middleware for intercepting HTTP requests and responses
pub struct InterceptorMiddleware {
    filter_engine: Arc<RwLock<FilterEngine>>,
    logger: Arc<dyn RequestLogger>,
}

#[async_trait]
impl Middleware for InterceptorMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut http::Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        // Convert to browser request format
        let browser_req = self.convert_request(&req);
        
        // Log the request
        self.logger.log_request(&browser_req).await;
        
        // Evaluate filters
        let filter_engine = self.filter_engine.read().await;
        let result = filter_engine.evaluate_request(&browser_req);
        drop(filter_engine);
        
        match result.action {
            FilterAction::Allow => {
                // Proceed with the request
                debug!("Request allowed: {}", browser_req.url);
                let response = next.run(req, extensions).await?;
                
                // Convert and log response
                let browser_resp = self.convert_response(&response, browser_req.id);
                self.logger.log_response(&browser_resp).await;
                
                // Evaluate response filters
                let filter_engine = self.filter_engine.read().await;
                let resp_result = filter_engine.evaluate_response(&browser_req, &browser_resp);
                drop(filter_engine);
                
                match resp_result.action {
                    FilterAction::Block { reason } => {
                        warn!("Response blocked: {:?}", reason);
                        Err(reqwest_middleware::Error::Middleware(
                            anyhow::anyhow!("Response blocked: {:?}", reason)
                        ))
                    }
                    _ => Ok(response),
                }
            }
            
            FilterAction::Block { reason } => {
                info!("Request blocked: {} - {:?}", browser_req.url, reason);
                Err(reqwest_middleware::Error::Middleware(
                    anyhow::anyhow!("Request blocked: {:?}", reason)
                ))
            }
            
            FilterAction::Redirect { url } => {
                info!("Redirecting request to: {}", url);
                let mut new_req = req;
                *new_req.url_mut() = url.parse().map_err(|e| {
                    reqwest_middleware::Error::Middleware(
                        anyhow::anyhow!("Invalid redirect URL: {}", e)
                    )
                })?;
                next.run(new_req, extensions).await
            }
            
            FilterAction::Modify { modifications } => {
                debug!("Modifying request: {}", browser_req.url);
                let modified_req = self.apply_request_modifications(req, &modifications)?;
                let response = next.run(modified_req, extensions).await?;
                Ok(response)
            }
            
            FilterAction::Delay { milliseconds } => {
                debug!("Delaying request by {}ms", milliseconds);
                tokio::time::sleep(tokio::time::Duration::from_millis(milliseconds)).await;
                let response = next.run(req, extensions).await?;
                Ok(response)
            }
            
            FilterAction::CustomResponse { status_code, headers, body } => {
                // This would require creating a mock response
                // For now, just proceed with the original request
                warn!("CustomResponse not yet implemented, proceeding with original request");
                let response = next.run(req, extensions).await?;
                Ok(response)
            }
            
            FilterAction::LogOnly => {
                let response = next.run(req, extensions).await?;
                let browser_resp = self.convert_response(&response, browser_req.id);
                self.logger.log_response(&browser_resp).await;
                Ok(response)
            }
        }
    }
}

impl InterceptorMiddleware {
    pub fn new(filter_engine: Arc<RwLock<FilterEngine>>, logger: Arc<dyn RequestLogger>) -> Self {
        Self {
            filter_engine,
            logger,
        }
    }

    fn convert_request(&self, req: &Request) -> BrowserRequest {
        let headers = req
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|v| (k.as_str().to_lowercase(), v.to_string()))
            })
            .collect();

        BrowserRequest {
            id: uuid::Uuid::new_v4(),
            method: match req.method().as_str() {
                "GET" => shared::HttpMethod::Get,
                "POST" => shared::HttpMethod::Post,
                "PUT" => shared::HttpMethod::Put,
                "DELETE" => shared::HttpMethod::Delete,
                "PATCH" => shared::HttpMethod::Patch,
                "HEAD" => shared::HttpMethod::Head,
                "OPTIONS" => shared::HttpMethod::Options,
                "CONNECT" => shared::HttpMethod::Connect,
                "TRACE" => shared::HttpMethod::Trace,
                _ => shared::HttpMethod::Get,
            },
            url: req.url().to_string(),
            headers,
            body: None, // Body would need to be extracted separately
            timestamp: chrono::Utc::now(),
        }
    }

    fn convert_response(&self, resp: &Response, request_id: uuid::Uuid) -> BrowserResponse {
        let headers = resp
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|v| (k.as_str().to_lowercase(), v.to_string()))
            })
            .collect();

        BrowserResponse {
            request_id,
            status_code: resp.status().as_u16(),
            status_text: resp.status().canonical_reason().unwrap_or("Unknown").to_string(),
            headers,
            body: None, // Body would need to be extracted separately
            timestamp: chrono::Utc::now(),
        }
    }

    fn apply_request_modifications(
        &self,
        mut req: Request,
        modifications: &[Modification],
    ) -> anyhow::Result<Request> {
        use reqwest::header::{HeaderName, HeaderValue};

        for modification in modifications {
            match modification {
                Modification::AddHeader { name, value } => {
                    if let (Ok(name), Ok(value)) = (
                        HeaderName::from_bytes(name.as_bytes()),
                        HeaderValue::from_str(value),
                    ) {
                        req.headers_mut().insert(name, value);
                    }
                }
                Modification::RemoveHeader { name } => {
                    req.headers_mut().remove(name);
                }
                Modification::SetHeader { name, value } => {
                    if let (Ok(name), Ok(value)) = (
                        HeaderName::from_bytes(name.as_bytes()),
                        HeaderValue::from_str(value),
                    ) {
                        req.headers_mut().insert(name, value);
                    }
                }
                Modification::SetUrl { url } => {
                    *req.url_mut() = url.parse()?;
                }
                // Body modifications would require handling the body stream
                _ => {
                    warn!("Body modifications not yet implemented");
                }
            }
        }

        Ok(req)
    }
}

/// Trait for logging requests and responses
#[async_trait]
pub trait RequestLogger: Send + Sync {
    async fn log_request(&self, request: &BrowserRequest);
    async fn log_response(&self, response: &BrowserResponse);
}

/// Default logger that uses tracing
pub struct TracingLogger;

#[async_trait]
impl RequestLogger for TracingLogger {
    async fn log_request(&self, request: &BrowserRequest) {
        debug!(
            "[REQUEST] {:?} {} - {:?}",
            request.method, request.url, request.id
        );
    }

    async fn log_response(&self, response: &BrowserResponse) {
        debug!(
            "[RESPONSE] {} {} - {:?}",
            response.status_code, response.status_text, response.request_id
        );
    }
}

/// File-based request logger
pub struct FileLogger {
    log_path: std::path::PathBuf,
}

impl FileLogger {
    pub fn new(log_path: std::path::PathBuf) -> Self {
        Self { log_path }
    }
}

#[async_trait]
impl RequestLogger for FileLogger {
    async fn log_request(&self, request: &BrowserRequest) {
        let log_entry = serde_json::json!({
            "type": "request",
            "timestamp": request.timestamp,
            "id": request.id,
            "method": request.method,
            "url": request.url,
            "headers": request.headers,
        });

        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await
        {
            let _ = tokio::io::AsyncWriteExt::write_all(
                &mut file,
                format!("{}\n", log_entry).as_bytes(),
            )
            .await;
        }
    }

    async fn log_response(&self, response: &BrowserResponse) {
        let log_entry = serde_json::json!({
            "type": "response",
            "timestamp": response.timestamp,
            "request_id": response.request_id,
            "status_code": response.status_code,
            "status_text": response.status_text,
            "headers": response.headers,
        });

        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await
        {
            let _ = tokio::io::AsyncWriteExt::write_all(
                &mut file,
                format!("{}\n", log_entry).as_bytes(),
            )
            .await;
        }
    }
}

/// Builder for creating an HTTP client with interception
pub struct InterceptedClientBuilder {
    filter_engine: Arc<RwLock<FilterEngine>>,
    logger: Arc<dyn RequestLogger>,
    user_agent: Option<String>,
    timeout: Option<std::time::Duration>,
}

impl InterceptedClientBuilder {
    pub fn new(filter_engine: Arc<RwLock<FilterEngine>>) -> Self {
        Self {
            filter_engine,
            logger: Arc::new(TracingLogger),
            user_agent: None,
            timeout: None,
        }
    }

    pub fn with_logger(mut self, logger: Arc<dyn RequestLogger>) -> Self {
        self.logger = logger;
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build(self) -> reqwest_middleware::ClientWithMiddleware {
        let mut client_builder = Client::builder();

        if let Some(user_agent) = self.user_agent {
            client_builder = client_builder.user_agent(user_agent);
        }

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        let client = client_builder.build().expect("Failed to build HTTP client");

        let middleware = InterceptorMiddleware::new(self.filter_engine, self.logger);

        reqwest_middleware::ClientBuilder::new(client)
            .with(middleware)
            .build()
    }
}
