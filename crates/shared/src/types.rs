//! Common types used across the browser

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for browser tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for browser windows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub Uuid);

impl WindowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a URL with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Url {
    pub raw: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl Url {
    pub fn parse(url: &str) -> anyhow::Result<Self> {
        let parsed = url::Url::parse(url)?;
        Ok(Self {
            raw: url.to_string(),
            scheme: parsed.scheme().to_string(),
            host: parsed.host_str().unwrap_or("").to_string(),
            path: parsed.path().to_string(),
            query: parsed.query().map(|s| s.to_string()),
            fragment: parsed.fragment().map(|s| s.to_string()),
        })
    }
}

/// Navigation entry for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: Uuid,
    pub url: String,
    pub title: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub visit_count: u32,
}

/// Cookie storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<DateTime<Utc>>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Connect,
    Trace,
}

/// Represents an HTTP request for interception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Uuid,
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timestamp: DateTime<Utc>,
}

/// Represents an HTTP response for interception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub request_id: Uuid,
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timestamp: DateTime<Utc>,
}

/// Page load state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadState {
    Idle,
    Loading,
    Loaded,
    Failed,
}

/// Security information for a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub secure: bool,
    pub protocol: Option<String>,
    pub cipher_suite: Option<String>,
    pub certificate: Option<CertificateInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub fingerprint: String,
}

/// Browser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub homepage: String,
    pub search_engine: String,
    pub download_path: String,
    pub user_agent: String,
    pub proxy: Option<ProxyConfig>,
    pub enable_adblock: bool,
    pub enable_tracking_protection: bool,
    pub enable_do_not_track: bool,
    pub accept_languages: Vec<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            homepage: "https://start.duckduckgo.com".to_string(),
            search_engine: "https://duckduckgo.com/?q={}".to_string(),
            download_path: "~/Downloads".to_string(),
            user_agent: format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) RustyBrowser/{} Safari/537.36",
                env!("CARGO_PKG_VERSION")
            ),
            proxy: None,
            enable_adblock: true,
            enable_tracking_protection: true,
            enable_do_not_track: true,
            accept_languages: vec!["en-US".to_string(), "en".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub proxy_type: ProxyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyType {
    Http,
    Https,
    Socks5,
}
