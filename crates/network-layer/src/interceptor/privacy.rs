//! Privacy protection interceptor
//!
//! This interceptor removes tracking headers and prevents data leakage:
//! - Removes common tracking headers (DNT, X-Forwarded-For, etc.)
//! - Strips referrer information
//! - Removes ETags that can be used for tracking
//! - Prevents browser fingerprinting headers

use super::{Body, Interceptor, InterceptorResult};
use async_trait::async_trait;
use http::{header, Request, Response};
use std::collections::HashSet;
use tracing::{debug, trace};

/// Privacy protection modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyMode {
    /// No privacy protection
    None,
    /// Basic protection - remove obvious tracking headers
    Basic,
    /// Strict protection - remove all potentially identifying headers
    Strict,
    /// Paranoid mode - minimal headers, may break some sites
    Paranoid,
}

impl Default for PrivacyMode {
    fn default() -> Self {
        PrivacyMode::Basic
    }
}

/// Privacy interceptor that removes tracking headers
pub struct PrivacyInterceptor {
    mode: PrivacyMode,
    enabled: std::sync::atomic::AtomicBool,
    custom_headers_to_remove: HashSet<String>,
    user_agent: Option<String>,
}

impl PrivacyInterceptor {
    /// Create a new privacy interceptor with the given mode
    pub fn new(mode: PrivacyMode) -> Self {
        Self {
            mode,
            enabled: std::sync::atomic::AtomicBool::new(true),
            custom_headers_to_remove: HashSet::new(),
            user_agent: None,
        }
    }

    /// Create with custom user agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Add custom headers to remove
    pub fn add_custom_header(&mut self, header: impl Into<String>) {
        self.custom_headers_to_remove.insert(header.into().to_lowercase());
    }

    /// Get headers that should be removed based on privacy mode
    fn get_headers_to_remove(&self) -> Vec<header::HeaderName> {
        let mut headers = Vec::new();

        // Headers to remove in Basic mode
        let basic_headers = [
            "x-forwarded-for",
            "x-forwarded-host",
            "x-forwarded-proto",
            "x-real-ip",
            "x-client-ip",
            "x-cluster-client-ip",
            "cf-connecting-ip",
            "cf-ray",
            "true-client-ip",
        ];

        // Headers to remove in Strict mode
        let strict_headers = [
            "dnt",
            "save-data",
            "device-memory",
            "dpr",
            "viewport-width",
            "ect",
            "rtt",
            "downlink",
            "sec-ch-ua",
            "sec-ch-ua-mobile",
            "sec-ch-ua-platform",
            "sec-ch-ua-platform-version",
            "sec-ch-ua-model",
            "sec-ch-ua-full-version",
            "sec-ch-ua-full-version-list",
            "sec-ch-prefers-color-scheme",
            "sec-ch-prefers-reduced-motion",
        ];

        // Headers to remove in Paranoid mode
        let paranoid_headers = [
            "accept-language",
            "accept-encoding",
            "referer",
            "origin",
        ];

        match self.mode {
            PrivacyMode::None => {}
            PrivacyMode::Basic => {
                for h in basic_headers.iter() {
                    if let Ok(name) = h.parse() {
                        headers.push(name);
                    }
                }
            }
            PrivacyMode::Strict => {
                for h in basic_headers.iter().chain(strict_headers.iter()) {
                    if let Ok(name) = h.parse() {
                        headers.push(name);
                    }
                }
            }
            PrivacyMode::Paranoid => {
                for h in basic_headers
                    .iter()
                    .chain(strict_headers.iter())
                    .chain(paranoid_headers.iter())
                {
                    if let Ok(name) = h.parse() {
                        headers.push(name);
                    }
                }
            }
        }

        headers
    }

    /// Remove tracking headers from a request
    fn sanitize_request(&self, request: &mut Request<Body>) {
        let headers_to_remove = self.get_headers_to_remove();

        for header_name in &headers_to_remove {
            if request.headers_mut().remove(header_name).is_some() {
                trace!("Removed request header: {:?}", header_name);
            }
        }

        // Remove custom headers
        for header_name in &self.custom_headers_to_remove {
            if let Ok(name) = header_name.parse::<header::HeaderName>() {
                request.headers_mut().remove(&name);
            }
        }

        // Replace user agent if specified
        if let Some(ref ua) = self.user_agent {
            request
                .headers_mut()
                .insert(header::USER_AGENT, ua.parse().unwrap());
            debug!("Replaced User-Agent header");
        }

        // In paranoid mode, also manipulate referrer
        if self.mode == PrivacyMode::Paranoid {
            // Remove referrer or set to origin only
            if let Some(referer) = request.headers_mut().get_mut(header::REFERER) {
                if let Ok(referer_str) = referer.to_str() {
                    if let Ok(url) = url::Url::parse(referer_str) {
                        let origin = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
                        *referer = origin.parse().unwrap();
                    }
                }
            }
        }
    }

    /// Remove tracking headers from a response
    fn sanitize_response(&self, response: &mut Response<Body>) {
        // Remove ETags (can be used for tracking)
        if response.headers_mut().remove(header::ETAG).is_some() {
            trace!("Removed ETag header from response");
        }

        // Remove server fingerprinting headers
        let server_headers = [
            "server",
            "x-powered-by",
            "x-aspnet-version",
            "x-runtime",
            "x-version",
        ];

        for header in &server_headers {
            if let Ok(name) = header.parse::<header::HeaderName>() {
                if response.headers_mut().remove(&name).is_some() {
                    trace!("Removed server header: {}", header);
                }
            }
        }

        // Remove tracking cookies indicators
        if let Some(set_cookie) = response.headers_mut().get_mut(header::SET_COOKIE) {
            // This is a simplified check - real implementation would parse cookies
            if let Ok(cookie_str) = set_cookie.to_str() {
                if cookie_str.contains("tracking") || cookie_str.contains("analytics") {
                    debug!("Potentially tracking cookie detected: {}", cookie_str);
                }
            }
        }
    }
}

#[async_trait]
impl Interceptor for PrivacyInterceptor {
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        if self.mode == PrivacyMode::None {
            return Ok(());
        }

        debug!(
            "Applying privacy protection ({:?}) to request: {} {}",
            self.mode,
            request.method(),
            request.uri()
        );

        self.sanitize_request(request);
        Ok(())
    }

    async fn intercept_response(&self, response: &mut Response<Body>) -> InterceptorResult<()> {
        if self.mode == PrivacyMode::None {
            return Ok(());
        }

        trace!("Applying privacy protection to response");

        self.sanitize_response(response);
        Ok(())
    }

    fn name(&self) -> &str {
        "privacy"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn priority(&self) -> i32 {
        100 // High priority - process early
    }
}

/// Fingerprinting protection interceptor
pub struct FingerprintingInterceptor {
    enabled: std::sync::atomic::AtomicBool,
    block_canvas: bool,
    block_webgl: bool,
    block_fonts: bool,
}

impl FingerprintingInterceptor {
    pub fn new() -> Self {
        Self {
            enabled: std::sync::atomic::AtomicBool::new(true),
            block_canvas: true,
            block_webgl: true,
            block_fonts: false,
        }
    }

    pub fn with_canvas_blocking(mut self, block: bool) -> Self {
        self.block_canvas = block;
        self
    }

    pub fn with_webgl_blocking(mut self, block: bool) -> Self {
        self.block_webgl = block;
        self
    }
}

#[async_trait]
impl Interceptor for FingerprintingInterceptor {
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        // Add headers to indicate fingerprinting protection
        // These will be used by the browser engine
        let headers = request.headers_mut();

        if self.block_canvas {
            headers.insert("x-fingerprint-protection-canvas", "1".parse().unwrap());
        }
        if self.block_webgl {
            headers.insert("x-fingerprint-protection-webgl", "1".parse().unwrap());
        }
        if self.block_fonts {
            headers.insert("x-fingerprint-protection-fonts", "1".parse().unwrap());
        }

        Ok(())
    }

    async fn intercept_response(&self, _response: &mut Response<Body>) -> InterceptorResult<()> {
        // Response modifications would be handled by content injection
        Ok(())
    }

    fn name(&self) -> &str {
        "fingerprinting"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Header normalization interceptor
pub struct HeaderNormalizationInterceptor;

impl HeaderNormalizationInterceptor {
    pub fn new() -> Self {
        Self
    }

    /// Normalize accept headers to common values
    fn normalize_accept_headers(&self, request: &mut Request<Body>) {
        let headers = request.headers_mut();

        // Normalize Accept header
        if let Some(accept) = headers.get(header::ACCEPT) {
            if let Ok(accept_str) = accept.to_str() {
                // Simplify to common values
                let normalized = if accept_str.contains("text/html") {
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                } else if accept_str.contains("application/json") {
                    "application/json,*/*;q=0.8"
                } else {
                    "*/*"
                };
                headers.insert(header::ACCEPT, normalized.parse().unwrap());
            }
        }
    }
}

#[async_trait]
impl Interceptor for HeaderNormalizationInterceptor {
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        self.normalize_accept_headers(request);
        Ok(())
    }

    async fn intercept_response(&self, _response: &mut Response<Body>) -> InterceptorResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "header-normalization"
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;

    fn create_test_request() -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri("https://example.com/test")
            .header("User-Agent", "Test/1.0")
            .header("X-Forwarded-For", "192.168.1.1")
            .header("Referer", "https://example.com/previous")
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn test_privacy_interceptor_basic() {
        let interceptor = PrivacyInterceptor::new(PrivacyMode::Basic);
        let mut request = create_test_request();

        // Check that X-Forwarded-For is in the list to remove
        let headers_to_remove = interceptor.get_headers_to_remove();
        assert!(headers_to_remove.contains(&header::HeaderName::from_static("x-forwarded-for")));
    }

    #[test]
    fn test_privacy_interceptor_strict() {
        let interceptor = PrivacyInterceptor::new(PrivacyMode::Strict);
        let headers_to_remove = interceptor.get_headers_to_remove();

        // Should include DNT header in strict mode
        assert!(headers_to_remove.contains(&header::HeaderName::from_static("dnt")));
    }

    #[test]
    fn test_privacy_interceptor_disabled() {
        let interceptor = PrivacyInterceptor::new(PrivacyMode::None);
        let headers_to_remove = interceptor.get_headers_to_remove();
        assert!(headers_to_remove.is_empty());
    }
}
