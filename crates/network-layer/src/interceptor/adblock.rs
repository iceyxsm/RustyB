//! Ad blocking interceptor with EasyList filter support
//!
//! This module provides:
//! - EasyList filter parsing
//! - URL pattern matching
//! - Domain-based blocking
//! - Cosmetic filtering support
//! - Filter list management

use super::{Body, Interceptor, InterceptorError, InterceptorResult};
use async_trait::async_trait;
use http::{header, Request, Response};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, trace};
use url::Url;

/// Types of filter rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterRule {
    /// Block URL matching pattern
    Block { pattern: String, options: FilterOptions },
    /// Allow (exception) URL matching pattern
    Allow { pattern: String, options: FilterOptions },
    /// Cosmetic filter (CSS hiding)
    Cosmetic { selector: String, domains: Vec<String> },
    /// Redirect to resource
    Redirect { pattern: String, resource: String },
}

/// Options for filter rules
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterOptions {
    /// Only apply to these domain types
    pub domain_restrictions: Vec<String>,
    /// Don't apply to these domains
    pub domain_exceptions: Vec<String>,
    /// Resource types to block
    pub resource_types: Vec<ResourceType>,
    /// Third-party only
    pub third_party_only: bool,
    /// First-party only
    pub first_party_only: bool,
    /// Case sensitive matching
    pub case_sensitive: bool,
}

/// Resource types for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Script,
    Stylesheet,
    Image,
    Media,
    Font,
    XmlHttpRequest,
    SubFrame,
    Object,
    Ping,
    WebSocket,
    Other,
}

impl ResourceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "script" => Some(Self::Script),
            "stylesheet" | "css" => Some(Self::Stylesheet),
            "image" | "images" => Some(Self::Image),
            "media" => Some(Self::Media),
            "font" | "fonts" => Some(Self::Font),
            "xmlhttprequest" | "xhr" => Some(Self::XmlHttpRequest),
            "subdocument" | "sub_frame" => Some(Self::SubFrame),
            "object" => Some(Self::Object),
            "ping" => Some(Self::Ping),
            "websocket" | "websockets" => Some(Self::WebSocket),
            _ => Some(Self::Other),
        }
    }
}

/// Ad blocking engine
pub struct AdBlockEngine {
    /// Block rules
    block_rules: Vec<FilterRule>,
    /// Allow rules (exceptions)
    allow_rules: Vec<FilterRule>,
    /// Cosmetic filters by domain
    cosmetic_filters: HashMap<String, Vec<String>>,
    /// Compiled regex patterns for performance
    compiled_patterns: HashMap<String, Regex>,
    /// Enabled status
    enabled: std::sync::atomic::AtomicBool,
    /// Statistics
    stats: Arc<RwLock<AdBlockStats>>,
}

/// Statistics for ad blocking
#[derive(Debug, Default)]
pub struct AdBlockStats {
    pub blocked_requests: u64,
    pub allowed_requests: u64,
    pub cosmetic_filters_applied: u64,
}

impl AdBlockEngine {
    /// Create a new ad block engine
    pub fn new() -> Self {
        Self {
            block_rules: Vec::new(),
            allow_rules: Vec::new(),
            cosmetic_filters: HashMap::new(),
            compiled_patterns: HashMap::new(),
            enabled: std::sync::atomic::AtomicBool::new(true),
            stats: Arc::new(RwLock::new(AdBlockStats::default())),
        }
    }

    /// Load EasyList filter list from string
    pub fn load_easylist(&mut self, content: &str) -> anyhow::Result<usize> {
        let mut count = 0;

        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }

            if let Some(rule) = self.parse_filter_rule(line) {
                match rule {
                    FilterRule::Block { .. } => {
                        self.block_rules.push(rule);
                    }
                    FilterRule::Allow { .. } => {
                        self.allow_rules.push(rule);
                    }
                    FilterRule::Cosmetic { ref selector, ref domains } => {
                        if domains.is_empty() {
                            // Global cosmetic filter
                            self.cosmetic_filters
                                .entry("*".to_string())
                                .or_default()
                                .push(selector.clone());
                        } else {
                            for domain in domains {
                                self.cosmetic_filters
                                    .entry(domain.clone())
                                    .or_default()
                                    .push(selector.clone());
                            }
                        }
                    }
                    FilterRule::Redirect { .. } => {
                        // TODO: Implement redirect rules
                    }
                }
                count += 1;
            }
        }

        info!("Loaded {} filter rules from EasyList", count);
        Ok(count)
    }

    /// Parse a single filter rule
    fn parse_filter_rule(&mut self, line: &str) -> Option<FilterRule> {
        // Check for cosmetic filters
        if line.contains("##") {
            return self.parse_cosmetic_filter(line);
        }

        // Check for exception rules
        if line.starts_with("@@") {
            return self.parse_allow_rule(&line[2..]);
        }

        // Parse block rule
        self.parse_block_rule(line)
    }

    /// Parse a block rule
    fn parse_block_rule(&mut self, line: &str) -> Option<FilterRule> {
        let (pattern, options_str) = if let Some(pos) = line.find('$') {
            (&line[..pos], &line[pos + 1..])
        } else {
            (line, "")
        };

        let options = if options_str.is_empty() {
            FilterOptions::default()
        } else {
            self.parse_options(options_str)
        };

        Some(FilterRule::Block {
            pattern: pattern.to_string(),
            options,
        })
    }

    /// Parse an allow (exception) rule
    fn parse_allow_rule(&mut self, line: &str) -> Option<FilterRule> {
        let (pattern, options_str) = if let Some(pos) = line.find('$') {
            (&line[..pos], &line[pos + 1..])
        } else {
            (line, "")
        };

        let options = if options_str.is_empty() {
            FilterOptions::default()
        } else {
            self.parse_options(options_str)
        };

        Some(FilterRule::Allow {
            pattern: pattern.to_string(),
            options,
        })
    }

    /// Parse a cosmetic filter
    fn parse_cosmetic_filter(&self, line: &str) -> Option<FilterRule> {
        let parts: Vec<&str> = line.split("##").collect();
        if parts.len() != 2 {
            return None;
        }

        let domains_str = parts[0];
        let selector = parts[1].to_string();

        let domains: Vec<String> = if domains_str.is_empty() {
            vec![]
        } else {
            domains_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        };

        Some(FilterRule::Cosmetic { selector, domains })
    }

    /// Parse filter options
    fn parse_options(&self, options_str: &str) -> FilterOptions {
        let mut options = FilterOptions::default();

        for opt in options_str.split(',') {
            let opt = opt.trim();

            if opt == "third-party" || opt == "thirdParty" {
                options.third_party_only = true;
            } else if opt == "~third-party" || opt == "~thirdParty" {
                options.first_party_only = true;
            } else if opt == "match-case" {
                options.case_sensitive = true;
            } else if opt.starts_with("domain=") {
                let domains = &opt[7..];
                for domain in domains.split('|') {
                    if domain.starts_with('~') {
                        options.domain_exceptions.push(domain[1..].to_string());
                    } else {
                        options.domain_restrictions.push(domain.to_string());
                    }
                }
            } else if opt.starts_with("script") 
                || opt.starts_with("image") 
                || opt.starts_with("stylesheet")
                || opt.starts_with("xmlhttprequest")
                || opt.starts_with("subdocument") {
                if let Some(resource_type) = ResourceType::from_str(opt) {
                    options.resource_types.push(resource_type);
                }
            }
        }

        options
    }

    /// Check if a URL should be blocked
    pub fn should_block(&self, url: &str, source_domain: Option<&str>) -> bool {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return false;
        }

        // Check allow rules first (exceptions)
        for rule in &self.allow_rules {
            if let FilterRule::Allow { pattern, options } = rule {
                if self.matches_pattern(url, pattern, options) {
                    if self.matches_domain_restrictions(options, source_domain) {
                        trace!("URL allowed by exception rule: {}", pattern);
                        return false;
                    }
                }
            }
        }

        // Check block rules
        for rule in &self.block_rules {
            if let FilterRule::Block { pattern, options } = rule {
                if self.matches_pattern(url, pattern, options) {
                    if self.matches_domain_restrictions(options, source_domain) {
                        trace!("URL blocked by rule: {}", pattern);
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get cosmetic filters for a domain
    pub fn get_cosmetic_filters(&self, domain: &str) -> Vec<String> {
        let mut filters = Vec::new();

        // Add global filters
        if let Some(global) = self.cosmetic_filters.get("*") {
            filters.extend(global.clone());
        }

        // Add domain-specific filters
        // Check exact domain and parent domains
        let parts: Vec<&str> = domain.split('.').collect();
        for i in 0..parts.len() {
            let check_domain = parts[i..].join(".");
            if let Some(domain_filters) = self.cosmetic_filters.get(&check_domain) {
                filters.extend(domain_filters.clone());
            }
        }

        filters
    }

    /// Check if URL matches a pattern
    fn matches_pattern(&self, url: &str, pattern: &str, options: &FilterOptions) -> bool {
        let url_to_check = if options.case_sensitive {
            url.to_string()
        } else {
            url.to_lowercase()
        };

        let pattern_to_check = if options.case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        // Simple pattern matching
        // Handle wildcards
        if pattern_to_check.starts_with("||") {
            // Domain anchor - match domain and subdomains
            let domain_pattern = &pattern_to_check[2..];
            self.matches_domain_anchor(&url_to_check, domain_pattern)
        } else if pattern_to_check.starts_with('|') {
            // Anchor at start
            let start_pattern = &pattern_to_check[1..];
            url_to_check.starts_with(start_pattern)
        } else if pattern_to_check.ends_with('|') {
            // Anchor at end
            let end_pattern = &pattern_to_check[..pattern_to_check.len() - 1];
            url_to_check.ends_with(end_pattern)
        } else if pattern_to_check.starts_with('/') && pattern_to_check.ends_with('/') {
            // Regex pattern
            self.matches_regex(&url_to_check, &pattern_to_check[1..pattern_to_check.len() - 1])
        } else {
            // Simple substring match
            url_to_check.contains(&pattern_to_check)
        }
    }

    /// Check domain anchor pattern (||)
    fn matches_domain_anchor(&self, url: &str, pattern: &str) -> bool {
        // Extract domain from URL
        if let Ok(url_parsed) = Url::parse(url) {
            if let Some(host) = url_parsed.host_str() {
                let host_lower = host.to_lowercase();
                let pattern_lower = pattern.to_lowercase();

                // Match exact domain or subdomain
                if host_lower == pattern_lower {
                    return true;
                }
                if host_lower.ends_with(&format!(".{}", pattern_lower)) {
                    return true;
                }
            }
        }

        // Fallback: check if pattern appears after ://
        url.contains(&format!("://{}", pattern)) ||
        url.contains(&format!("//www.{}", pattern))
    }

    /// Check regex pattern
    fn matches_regex(&self, url: &str, pattern: &str) -> bool {
        // Use cached regex if available
        if let Some(regex) = self.compiled_patterns.get(pattern) {
            return regex.is_match(url);
        }

        // Compile and cache new regex
        if let Ok(regex) = Regex::new(pattern) {
            let result = regex.is_match(url);
            // Note: In production, we'd cache this, but we can't mutate self here
            return result;
        }

        false
    }

    /// Check domain restrictions
    fn matches_domain_restrictions(
        &self,
        options: &FilterOptions,
        source_domain: Option<&str>,
    ) -> bool {
        // Check domain exceptions
        if let Some(domain) = source_domain {
            for exception in &options.domain_exceptions {
                if domain.ends_with(exception) {
                    return false;
                }
            }
        }

        // Check domain restrictions
        if !options.domain_restrictions.is_empty() {
            if let Some(domain) = source_domain {
                for restriction in &options.domain_restrictions {
                    if domain.ends_with(restriction) {
                        return true;
                    }
                }
                return false; // No restriction matched
            } else {
                return false; // No source domain but restrictions exist
            }
        }

        true
    }

    /// Get statistics
    pub async fn stats(&self) -> AdBlockStats {
        self.stats.read().await.clone()
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.block_rules.clear();
        self.allow_rules.clear();
        self.cosmetic_filters.clear();
        self.compiled_patterns.clear();
        info!("All filter rules cleared");
    }

    /// Get rule counts
    pub fn rule_counts(&self) -> (usize, usize, usize) {
        (
            self.block_rules.len(),
            self.allow_rules.len(),
            self.cosmetic_filters.values().map(|v| v.len()).sum(),
        )
    }
}

impl Default for AdBlockEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AdBlockStats {
    fn clone(&self) -> Self {
        Self {
            blocked_requests: self.blocked_requests,
            allowed_requests: self.allowed_requests,
            cosmetic_filters_applied: self.cosmetic_filters_applied,
        }
    }
}

/// Ad blocking interceptor
pub struct AdBlockInterceptor {
    engine: Arc<RwLock<AdBlockEngine>>,
    enabled: std::sync::atomic::AtomicBool,
}

impl AdBlockInterceptor {
    /// Create a new ad block interceptor
    pub fn new() -> Self {
        Self {
            engine: Arc::new(RwLock::new(AdBlockEngine::new())),
            enabled: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Load filter list from string
    pub async fn load_filters(&self, content: &str) -> anyhow::Result<usize> {
        let mut engine = self.engine.write().await;
        engine.load_easylist(content)
    }

    /// Load default filter list
    pub async fn load_default_filters(&self) -> anyhow::Result<usize> {
        // Basic ad blocking rules
        let default_filters = r#"
! Basic ad blocking filters
||googleadservices.com^
||googlesyndication.com^
||google-analytics.com^
||doubleclick.net^
||facebook.com/tr^
||analytics.twitter.com^
||adsystem.amazon.com^
||advertising.com^
||adnxs.com^
||adsrvr.org^
||adsafeprotected.com^
||moatads.com^
||scorecardresearch.com^
||quantserve.com^
||googletagmanager.com^
||hotjar.com^
||segment.io^
||mixpanel.com^
||amplitude.com^
||googletagservices.com^
||amazon-adsystem.com^

! Common ad serving patterns
/advertisement.
/ads.js
/analytics.js
/tracking.js
/pixel.gif
/beacon.js
/telemetry.
/metrics.

! Social media tracking
/facebook.com/plugins/
/platform.twitter.com/widgets/
/linkedin.com/countserv/
"#;

        self.load_filters(default_filters).await
    }

    /// Get the underlying engine
    pub fn engine(&self) -> &Arc<RwLock<AdBlockEngine>> {
        &self.engine
    }

    /// Extract domain from URI
    fn extract_domain(&self, uri: &http::Uri) -> Option<String> {
        uri.host().map(|h| h.to_lowercase())
    }
}

#[async_trait]
impl Interceptor for AdBlockInterceptor {
    async fn intercept_request(&self, request: &mut Request<Body>) -> InterceptorResult<()> {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        let uri = request.uri().to_string();
        let domain = self.extract_domain(request.uri());
        let source_domain = request.headers()
            .get(header::REFERER)
            .and_then(|h| h.to_str().ok())
            .and_then(|r| Url::parse(r).ok())
            .and_then(|u| u.host_str().map(|h| h.to_string()));

        let engine = self.engine.read().await;
        
        if engine.should_block(&uri, source_domain.as_deref().or(domain.as_deref())) {
            debug!("Blocking ad/tracker: {}", uri);
            return Err(InterceptorError::Blocked(format!(
                "Blocked by ad filter: {}",
                uri
            )));
        }

        Ok(())
    }

    async fn intercept_response(&self, _response: &mut Response<Body>) -> InterceptorResult<()> {
        // Response handling would inject cosmetic filters
        Ok(())
    }

    fn name(&self) -> &str {
        "adblock"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn priority(&self) -> i32 {
        200 // Very high priority - block before other processing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_block_rule() {
        let mut engine = AdBlockEngine::new();
        let rule = engine.parse_filter_rule("||ads.example.com^");
        
        assert!(matches!(rule, Some(FilterRule::Block { .. })));
    }

    #[test]
    fn test_parse_allow_rule() {
        let mut engine = AdBlockEngine::new();
        let rule = engine.parse_filter_rule("@@||example.com/ads.js");
        
        assert!(matches!(rule, Some(FilterRule::Allow { .. })));
    }

    #[test]
    fn test_parse_cosmetic_filter() {
        let mut engine = AdBlockEngine::new();
        let rule = engine.parse_filter_rule("example.com##.ad-banner");
        
        assert!(matches!(rule, Some(FilterRule::Cosmetic { .. })));
    }

    #[test]
    fn test_domain_anchor_matching() {
        let engine = AdBlockEngine::new();
        
        assert!(engine.matches_domain_anchor(
            "https://ads.example.com/banner.js",
            "ads.example.com"
        ));
        
        assert!(engine.matches_domain_anchor(
            "https://sub.ads.example.com/banner.js",
            "ads.example.com"
        ));
        
        assert!(!engine.matches_domain_anchor(
            "https://example.com/ads.js",
            "ads.example.com"
        ));
    }

    #[test]
    fn test_should_block() {
        let mut engine = AdBlockEngine::new();
        
        engine.load_easylist(r#"
||ads.example.com^
||tracker.com/analytics.js
@@||example.com/allowed-ad.js
"#).unwrap();

        assert!(engine.should_block("https://ads.example.com/banner.js", None));
        assert!(engine.should_block("https://tracker.com/analytics.js", None));
        assert!(!engine.should_block("https://example.com/allowed-ad.js", None));
        assert!(!engine.should_block("https://example.com/normal.js", None));
    }

    #[test]
    fn test_cosmetic_filters() {
        let mut engine = AdBlockEngine::new();
        
        engine.load_easylist(r#"
example.com##.ad-banner
example.com##.tracking-pixel
global##.advertisement
"#).unwrap();

        let filters = engine.get_cosmetic_filters("example.com");
        assert!(filters.contains(&".ad-banner".to_string()));
        assert!(filters.contains(&".tracking-pixel".to_string()));
    }
}
