//! Error types for the browser

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("Navigation failed: {0}")]
    NavigationError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Tab not found: {0}")]
    TabNotFound(uuid::Uuid),

    #[error("Window not found: {0}")]
    WindowNotFound(uuid::Uuid),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Proxy error: {0}")]
    ProxyError(String),

    #[error("AI engine error: {0}")]
    AiError(String),

    #[error("Automation error: {0}")]
    AutomationError(String),

    #[error("Schema error: {0}")]
    SchemaError(String),

    #[error("Remote command error: {0}")]
    RemoteCommandError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, BrowserError>;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Request blocked by filter")]
    BlockedByFilter,

    #[error("Invalid certificate")]
    InvalidCertificate,

    #[error("Proxy connection failed: {0}")]
    ProxyConnectionFailed(String),
}

#[derive(Error, Debug)]
pub enum AiError {
    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Device not available")]
    DeviceNotAvailable,
}
