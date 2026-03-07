//! Navigation and history management module
//!
//! This module provides comprehensive navigation, history, and session management
//! capabilities for the browser, including:
//!
//! - **History Management**: Persistent history storage with redb database,
//!   full-text search, import/export, and privacy mode
//!
//! - **Session Management**: Save and restore browser sessions with crash recovery,
//!   auto-save functionality, and session history
//!
//! - **Navigation Control**: Back/forward navigation with history stack management
//!
//! # Example Usage
//!
//! ```rust
//! use browser_core::navigation::{HistoryManager, HistoryConfig, HistoryEntry};
//! use browser_core::navigation::{SessionManager, SessionConfig, Session, WindowState, TabState};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create history manager
//! let history_config = HistoryConfig::default();
//! let history_manager = HistoryManager::new(history_config)?;
//!
//! // Add a history entry
//! let entry = HistoryEntry::new("https://example.com", "Example Site");
//! history_manager.add_visit(entry).await?;
//!
//! // Search history
//! let results = history_manager.search("example").await?;
//!
//! // Create session manager
//! let session_config = SessionConfig::default();
//! let session_manager = SessionManager::new(session_config)?;
//!
//! // Start auto-save
//! session_manager.start_auto_save().await?;
//!
//! // Add a window with tabs
//! let mut window = WindowState::new();
//! let tab = TabState::new("https://example.com", "Example");
//! window.add_tab(tab);
//! session_manager.add_window(window).await?;
//! # Ok(())
//! # }
//! ```

// History management
pub mod history;

// Session management
pub mod session;

// Re-export history types
pub use history::{
    HistoryConfig,
    HistoryEntry,
    HistoryError,
    HistoryManager,
    HistoryStats,
    PageMetadata,
};

// Re-export session types
pub use session::{
    NavigationController,
    NavigationEntry,
    Session,
    SessionConfig,
    SessionError,
    SessionManager,
    SessionMetadata,
    SessionStats,
    TabState,
    TransitionType,
    WindowGeometry,
    WindowState,
};

/// Result type for navigation operations
pub type Result<T> = std::result::Result<T, NavigationError>;

/// Combined error type for navigation operations
#[derive(Debug, thiserror::Error)]
pub enum NavigationError {
    #[error("History error: {0}")]
    History(#[from] HistoryError),
    
    #[error("Session error: {0}")]
    Session(#[from] SessionError),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Re-export history error
pub use history::HistoryError;

/// Re-export session error
pub use session::SessionError;

/// Version of the navigation module
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the navigation module with default configuration
/// 
/// This creates the necessary directories and databases for history
/// and session management.
pub fn init(data_dir: &std::path::Path) -> anyhow::Result<(HistoryManager, SessionManager)> {
    // Create history manager
    let history_config = HistoryConfig {
        db_path: data_dir.join("history.redb"),
        ..Default::default()
    };
    let history_manager = HistoryManager::new(history_config)?;
    
    // Create session manager
    let session_config = SessionConfig {
        db_path: data_dir.join("sessions.redb"),
        ..Default::default()
    };
    let session_manager = SessionManager::new(session_config)?;
    
    Ok((history_manager, session_manager))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_init() {
        let temp_dir = TempDir::new().unwrap();
        let (history, session) = init(temp_dir.path()).unwrap();
        
        // Just verify they were created successfully
        drop(history);
        drop(session);
    }
}
