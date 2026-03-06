//! Utility functions

use std::path::PathBuf;

/// Get the browser's data directory
pub fn data_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rusty-browser");
    
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    
    dir
}

/// Get the browser's config directory
pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rusty-browser");
    
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    
    dir
}

/// Get the cache directory
pub fn cache_dir() -> PathBuf {
    let dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rusty-browser");
    
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    
    dir
}

/// Sanitize a string for use as a filename
pub fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | ' ' => '_',
            _ => c,
        })
        .collect()
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let exp = (bytes as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
    let value = bytes as f64 / 1024f64.powi(exp as i32);
    
    format!("{:.2} {}", value, UNITS[exp])
}

/// Truncate a string with ellipsis
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Check if a URL is valid
pub fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// Normalize a URL (add https:// if missing)
pub fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

/// Generate a short ID (for display purposes)
pub fn short_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello world"), "hello_world");
        assert_eq!(sanitize_filename("file:name"), "file_name");
        assert_eq!(sanitize_filename("test/file"), "test_file");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(
            normalize_url("https://example.com"),
            "https://example.com"
        );
    }
}
