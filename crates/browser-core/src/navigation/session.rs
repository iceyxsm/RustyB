//! Session storage and management with crash recovery
//!
//! This module provides persistent session management using redb, allowing
//! browser state to be saved and restored across restarts, with automatic
//! crash recovery.

use chrono::{DateTime, Duration, Utc};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::history::{HistoryEntry, HistoryManager, PageMetadata};

/// Current session database schema version
const SESSION_SCHEMA_VERSION: u32 = 1;

/// Auto-save interval in seconds
const DEFAULT_AUTO_SAVE_INTERVAL_SECS: u64 = 30;

/// Maximum number of sessions to keep in history
const MAX_SESSION_HISTORY: usize = 10;

/// Table definitions for redb
const SESSIONS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");
const WINDOWS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("windows");
const TABS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tabs");
const SESSION_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("session_meta");
const CRASH_RECOVERY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("crash_recovery");

/// Errors that can occur in session operations
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Database error: {0}")]
    Database(#[from] redb::Error),
    
    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),
    
    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),
    
    #[error("Transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
    
    #[error("Window not found: {0}")]
    WindowNotFound(Uuid),
    
    #[error("Tab not found: {0}")]
    TabNotFound(Uuid),
    
    #[error("Invalid session state: {0}")]
    InvalidState(String),
    
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Auto-save already running")]
    AutoSaveAlreadyRunning,
}

pub type Result<T> = std::result::Result<T, SessionError>;

/// Window geometry (position and size)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub minimized: bool,
    pub fullscreen: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 720,
            maximized: false,
            minimized: false,
            fullscreen: false,
        }
    }
}

/// State of a single tab
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabState {
    /// Unique tab identifier
    pub id: Uuid,
    /// Current URL
    pub url: String,
    /// Page title
    pub title: String,
    /// Scroll position (x, y)
    pub scroll_position: (f64, f64),
    /// Zoom level (1.0 = 100%)
    pub zoom_level: f64,
    /// Whether the tab is pinned
    pub pinned: bool,
    /// Whether the tab is muted
    pub muted: bool,
    /// Tab history stack
    pub history_stack: Vec<HistoryEntry>,
    /// Current position in history stack
    pub history_position: usize,
    /// Form data for restoration
    pub form_data: Option<HashMap<String, String>>,
    /// Timestamp when tab was created
    pub created_at: SystemTime,
    /// Timestamp of last access
    pub last_accessed: SystemTime,
}

impl TabState {
    /// Create a new tab state
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            url: url.into(),
            title: title.into(),
            scroll_position: (0.0, 0.0),
            zoom_level: 1.0,
            pinned: false,
            muted: false,
            history_stack: Vec::new(),
            history_position: 0,
            form_data: None,
            created_at: now,
            last_accessed: now,
        }
    }
    
    /// Create a new tab with initial history
    pub fn with_history(
        url: impl Into<String>,
        title: impl Into<String>,
        history: Vec<HistoryEntry>,
        position: usize,
    ) -> Self {
        let mut tab = Self::new(url, title);
        tab.history_stack = history;
        tab.history_position = position;
        tab
    }
    
    /// Mark tab as accessed
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
    }
    
    /// Check if tab can go back in history
    pub fn can_go_back(&self) -> bool {
        self.history_position > 0
    }
    
    /// Check if tab can go forward in history
    pub fn can_go_forward(&self) -> bool {
        self.history_position < self.history_stack.len().saturating_sub(1)
    }
    
    /// Navigate back in history
    pub fn go_back(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_back() {
            self.history_position -= 1;
            self.history_stack.get(self.history_position)
        } else {
            None
        }
    }
    
    /// Navigate forward in history
    pub fn go_forward(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_forward() {
            self.history_position += 1;
            self.history_stack.get(self.history_position)
        } else {
            None
        }
    }
    
    /// Navigate to a new entry
    pub fn navigate(&mut self, entry: HistoryEntry) {
        // Remove forward history
        self.history_stack.truncate(self.history_position + 1);
        
        // Add new entry
        self.history_stack.push(entry);
        self.history_position = self.history_stack.len() - 1;
        
        // Update current URL and title
        if let Some(current) = self.history_stack.get(self.history_position) {
            self.url = current.url.clone();
            self.title = current.title.clone();
        }
        
        self.touch();
    }
    
    /// Get current history entry
    pub fn current_entry(&self) -> Option<&HistoryEntry> {
        self.history_stack.get(self.history_position)
    }
}

/// State of a browser window
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    /// Unique window identifier
    pub id: Uuid,
    /// Tabs in this window
    pub tabs: Vec<TabState>,
    /// Index of the active tab
    pub active_tab: usize,
    /// Window geometry
    pub geometry: WindowGeometry,
    /// Whether this is a private/incognito window
    pub incognito: bool,
    /// Timestamp when window was created
    pub created_at: SystemTime,
    /// Timestamp of last access
    pub last_accessed: SystemTime,
}

impl WindowState {
    /// Create a new window state
    pub fn new() -> Self {
        let now = SystemTime::now();
        let mut window = Self {
            id: Uuid::new_v4(),
            tabs: Vec::new(),
            active_tab: 0,
            geometry: WindowGeometry::default(),
            incognito: false,
            created_at: now,
            last_accessed: now,
        };
        
        // Create initial tab
        window.tabs.push(TabState::new("about:blank", "New Tab"));
        window
    }
    
    /// Create a new incognito window
    pub fn new_incognito() -> Self {
        let mut window = Self::new();
        window.incognito = true;
        window
    }
    
    /// Add a new tab
    pub fn add_tab(&mut self, tab: TabState) -> usize {
        self.tabs.push(tab);
        self.tabs.len() - 1
    }
    
    /// Remove a tab by index
    pub fn remove_tab(&mut self, index: usize) -> Option<TabState> {
        if index < self.tabs.len() {
            let tab = self.tabs.remove(index);
            
            // Adjust active tab
            if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                self.active_tab -= 1;
            }
            
            Some(tab)
        } else {
            None
        }
    }
    
    /// Get the active tab
    pub fn get_active_tab(&self) -> Option<&TabState> {
        self.tabs.get(self.active_tab)
    }
    
    /// Get the active tab mutably
    pub fn get_active_tab_mut(&mut self) -> Option<&mut TabState> {
        self.tabs.get_mut(self.active_tab)
    }
    
    /// Set active tab
    pub fn set_active_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab = index;
            if let Some(tab) = self.tabs.get_mut(index) {
                tab.touch();
            }
            self.touch();
            true
        } else {
            false
        }
    }
    
    /// Mark window as accessed
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
    }
    
    /// Get tab count
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete browser session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    /// Unique session identifier
    pub id: Uuid,
    /// Windows in this session
    pub windows: Vec<WindowState>,
    /// Index of the active window
    pub active_window: usize,
    /// When the session was created
    pub created_at: SystemTime,
    /// When the session was last accessed
    pub last_accessed: SystemTime,
    /// Session metadata
    pub metadata: SessionMetadata,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMetadata {
    /// Session name (optional)
    pub name: Option<String>,
    /// Whether this is a saved session (not auto-saved)
    pub saved: bool,
    /// Tags for organization
    pub tags: Vec<String>,
    /// User agent string used
    pub user_agent: String,
    /// Browser version
    pub browser_version: String,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            name: None,
            saved: false,
            tags: Vec::new(),
            user_agent: String::new(),
            browser_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Session {
    /// Create a new session
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            windows: vec![WindowState::new()],
            active_window: 0,
            created_at: now,
            last_accessed: now,
            metadata: SessionMetadata::default(),
        }
    }
    
    /// Create a new session with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        let mut session = Self::new();
        session.metadata.name = Some(name.into());
        session
    }
    
    /// Add a window to the session
    pub fn add_window(&mut self, window: WindowState) -> usize {
        self.windows.push(window);
        self.windows.len() - 1
    }
    
    /// Remove a window by index
    pub fn remove_window(&mut self, index: usize) -> Option<WindowState> {
        if index < self.windows.len() {
            let window = self.windows.remove(index);
            
            // Adjust active window
            if self.active_window >= self.windows.len() && self.active_window > 0 {
                self.active_window -= 1;
            }
            
            Some(window)
        } else {
            None
        }
    }
    
    /// Get the active window
    pub fn get_active_window(&self) -> Option<&WindowState> {
        self.windows.get(self.active_window)
    }
    
    /// Get the active window mutably
    pub fn get_active_window_mut(&mut self) -> Option<&mut WindowState> {
        self.windows.get_mut(self.active_window)
    }
    
    /// Set active window
    pub fn set_active_window(&mut self, index: usize) -> bool {
        if index < self.windows.len() {
            self.active_window = index;
            self.touch();
            true
        } else {
            false
        }
    }
    
    /// Mark session as accessed
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
    }
    
    /// Get total tab count across all windows
    pub fn total_tab_count(&self) -> usize {
        self.windows.iter().map(|w| w.tab_count()).sum()
    }
    
    /// Get window count
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Crash recovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrashRecoveryInfo {
    session_id: Uuid,
    timestamp: SystemTime,
    was_clean_shutdown: bool,
    crash_count: u32,
}

/// Configuration for the session manager
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Path to the database file
    pub db_path: std::path::PathBuf,
    /// Auto-save interval in seconds
    pub auto_save_interval_secs: u64,
    /// Maximum number of sessions to keep in history
    pub max_session_history: usize,
    /// Whether to enable crash recovery
    pub enable_crash_recovery: bool,
    /// Whether to restore sessions on startup
    pub restore_on_startup: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            db_path: dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("rusty_browser")
                .join("sessions.redb"),
            auto_save_interval_secs: DEFAULT_AUTO_SAVE_INTERVAL_SECS,
            max_session_history: MAX_SESSION_HISTORY,
            enable_crash_recovery: true,
            restore_on_startup: true,
        }
    }
}

/// Manages browser sessions with persistent storage
#[derive(Debug)]
pub struct SessionManager {
    db: Arc<Database>,
    config: RwLock<SessionConfig>,
    current_session: RwLock<Session>,
    auto_save_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
    history_manager: Option<Arc<HistoryManager>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionConfig) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let db = Database::create(&config.db_path)?;
        let db = Arc::new(db);
        
        // Initialize tables
        Self::initialize_tables(&db)?;
        
        // Check for crash recovery
        let current_session = if config.enable_crash_recovery {
            Self::check_crash_recovery(&db, config.restore_on_startup)?
        } else {
            Session::new()
        };
        
        let manager = Self {
            db,
            config: RwLock::new(config),
            current_session: RwLock::new(current_session),
            auto_save_handle: RwLock::new(None),
            history_manager: None,
        };
        
        info!("SessionManager initialized");
        Ok(manager)
    }
    
    /// Create a new session manager at the specified path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = SessionConfig::default();
        config.db_path = path.as_ref().to_path_buf();
        Self::new(config)
    }
    
    /// Initialize database tables
    fn initialize_tables(db: &Database) -> Result<()> {
        let txn = db.begin_write()?;
        
        {
            let _ = txn.open_table(SESSIONS_TABLE)?;
            let _ = txn.open_table(WINDOWS_TABLE)?;
            let _ = txn.open_table(TABS_TABLE)?;
            let _ = txn.open_table(SESSION_META_TABLE)?;
            let _ = txn.open_table(CRASH_RECOVERY_TABLE)?;
        }
        
        txn.commit()?;
        Ok(())
    }
    
    /// Check for crash recovery
    fn check_crash_recovery(db: &Database, restore: bool) -> Result<Session> {
        let txn = db.begin_read()?;
        let table = txn.open_table(CRASH_RECOVERY_TABLE)?;
        
        if let Some(value) = table.get("last_session")? {
            let info: CrashRecoveryInfo = serde_json::from_slice(value.value())?;
            
            if !info.was_clean_shutdown && restore {
                warn!("Detected unclean shutdown, attempting recovery");
                
                // Try to restore the session
                let sessions_table = txn.open_table(SESSIONS_TABLE)?;
                if let Some(session_value) = sessions_table.get(info.session_id.to_string().as_str())? {
                    let session: Session = serde_json::from_slice(session_value.value())?;
                    info!("Recovered session {} with {} windows", session.id, session.windows.len());
                    return Ok(session);
                }
            }
        }
        
        Ok(Session::new())
    }
    
    /// Set the history manager for cross-integration
    pub fn set_history_manager(&mut self, history_manager: Arc<HistoryManager>) {
        self.history_manager = Some(history_manager);
    }
    
    /// Get the current session
    pub async fn current_session(&self) -> Session {
        self.current_session.read().await.clone()
    }
    
    /// Get the current session ID
    pub async fn current_session_id(&self) -> Uuid {
        self.current_session.read().await.id
    }
    
    /// Save the current session
    pub async fn save_current_session(&self) -> Result<()> {
        let session = self.current_session.read().await.clone();
        self.save_session(&session).await
    }
    
    /// Save a session to the database
    pub async fn save_session(&self, session: &Session) -> Result<()> {
        let db = self.db.clone();
        let session = session.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            
            {
                let mut table = txn.open_table(SESSIONS_TABLE)?;
                let key = session.id.to_string();
                let value = serde_json::to_vec(&session)?;
                table.insert(key.as_str(), value.as_slice())?;
            }
            
            // Update crash recovery info
            {
                let mut table = txn.open_table(CRASH_RECOVERY_TABLE)?;
                let info = CrashRecoveryInfo {
                    session_id: session.id,
                    timestamp: SystemTime::now(),
                    was_clean_shutdown: false,
                    crash_count: 0,
                };
                let value = serde_json::to_vec(&info)?;
                table.insert("last_session", value.as_slice())?;
            }
            
            txn.commit()?;
            Result::<()>::Ok(())
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))??;
        
        debug!("Saved session {}", session.id);
        Ok(())
    }
    
    /// Load a session by ID
    pub async fn load_session(&self, id: Uuid) -> Result<Session> {
        let db = self.db.clone();
        let id_str = id.to_string();
        
        let session = tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let table = txn.open_table(SESSIONS_TABLE)?;
            
            if let Some(value) = table.get(id_str.as_str())? {
                let session: Session = serde_json::from_slice(value.value())?;
                Ok(Some(session))
            } else {
                Ok(None)
            }
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))?;
        
        session?.ok_or_else(|| SessionError::SessionNotFound(id))
    }
    
    /// Set a new current session
    pub async fn set_current_session(&self, session: Session) -> Result<()> {
        // Save old session
        self.save_current_session().await?;
        
        // Set new session
        let mut current = self.current_session.write().await;
        *current = session;
        
        Ok(())
    }
    
    /// Create a new session and make it current
    pub async fn new_session(&self, name: Option<String>) -> Result<Session> {
        // Save current session first
        self.save_current_session().await?;
        
        // Create new session
        let mut session = Session::new();
        if let Some(name) = name {
            session.metadata.name = Some(name);
        }
        
        // Set as current
        {
            let mut current = self.current_session.write().await;
            *current = session.clone();
        }
        
        // Save new session
        self.save_session(&session).await?;
        
        info!("Created new session {}", session.id);
        Ok(session)
    }
    
    /// Get all saved sessions
    pub async fn get_all_sessions(&self) -> Result<Vec<Session>> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let table = txn.open_table(SESSIONS_TABLE)?;
            
            let mut sessions = Vec::new();
            for item in table.iter()? {
                let (_, value) = item?;
                let session: Session = serde_json::from_slice(value.value())?;
                sessions.push(session);
            }
            
            // Sort by last accessed, most recent first
            sessions.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
            
            Ok(sessions)
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))?
    }
    
    /// Delete a session
    pub async fn delete_session(&self, id: Uuid) -> Result<bool> {
        let db = self.db.clone();
        let id_str = id.to_string();
        
        // Don't allow deleting current session
        let current_id = self.current_session_id().await;
        if id == current_id {
            return Err(SessionError::InvalidState(
                "Cannot delete current session".to_string()
            ));
        }
        
        let deleted = tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            let mut table = txn.open_table(SESSIONS_TABLE)?;
            let existed = table.get(id_str.as_str())?.is_some();
            
            if existed {
                table.remove(id_str.as_str())?;
            }
            
            txn.commit()?;
            Ok(existed)
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))??;
        
        Ok(deleted)
    }
    
    /// Add a window to the current session
    pub async fn add_window(&self, window: WindowState) -> Result<usize> {
        let mut session = self.current_session.write().await;
        let index = session.add_window(window);
        session.touch();
        Ok(index)
    }
    
    /// Remove a window from the current session
    pub async fn remove_window(&self, index: usize) -> Result<Option<WindowState>> {
        let mut session = self.current_session.write().await;
        let window = session.remove_window(index);
        session.touch();
        Ok(window)
    }
    
    /// Get a window from the current session
    pub async fn get_window(&self, index: usize) -> Result<Option<WindowState>> {
        let session = self.current_session.read().await;
        Ok(session.windows.get(index).cloned())
    }
    
    /// Update a window in the current session
    pub async fn update_window(&self, index: usize, window: WindowState) -> Result<()> {
        let mut session = self.current_session.write().await;
        
        if index >= session.windows.len() {
            return Err(SessionError::WindowNotFound(window.id));
        }
        
        session.windows[index] = window;
        session.touch();
        Ok(())
    }
    
    /// Set the active window
    pub async fn set_active_window(&self, index: usize) -> Result<()> {
        let mut session = self.current_session.write().await;
        
        if !session.set_active_window(index) {
            return Err(SessionError::InvalidState(
                format!("Invalid window index: {}", index)
            ));
        }
        
        Ok(())
    }
    
    /// Add a tab to a window
    pub async fn add_tab(&self, window_index: usize, tab: TabState) -> Result<usize> {
        let mut session = self.current_session.write().await;
        
        let window = session.windows.get_mut(window_index)
            .ok_or_else(|| SessionError::WindowNotFound(Uuid::nil()))?;
        
        let index = window.add_tab(tab);
        window.touch();
        session.touch();
        
        Ok(index)
    }
    
    /// Remove a tab from a window
    pub async fn remove_tab(&self, window_index: usize, tab_index: usize) -> Result<Option<TabState>> {
        let mut session = self.current_session.write().await;
        
        let window = session.windows.get_mut(window_index)
            .ok_or_else(|| SessionError::WindowNotFound(Uuid::nil()))?;
        
        let tab = window.remove_tab(tab_index);
        window.touch();
        session.touch();
        
        Ok(tab)
    }
    
    /// Update a tab in a window
    pub async fn update_tab(&self, window_index: usize, tab_index: usize, tab: TabState) -> Result<()> {
        let mut session = self.current_session.write().await;
        
        let window = session.windows.get_mut(window_index)
            .ok_or_else(|| SessionError::WindowNotFound(Uuid::nil()))?;
        
        if tab_index >= window.tabs.len() {
            return Err(SessionError::TabNotFound(tab.id));
        }
        
        window.tabs[tab_index] = tab;
        window.touch();
        session.touch();
        
        Ok(())
    }
    
    /// Navigate to a URL in the active tab of the active window
    pub async fn navigate(&self, url: &str, title: &str) -> Result<()> {
        let mut session = self.current_session.write().await;
        
        let window = session.get_active_window_mut()
            .ok_or_else(|| SessionError::InvalidState("No active window".to_string()))?;
        
        let tab = window.get_active_tab_mut()
            .ok_or_else(|| SessionError::InvalidState("No active tab".to_string()))?;
        
        let entry = HistoryEntry::new(url, title);
        tab.navigate(entry);
        
        session.touch();
        Ok(())
    }
    
    /// Go back in the active tab
    pub async fn go_back(&self) -> Result<Option<HistoryEntry>> {
        let mut session = self.current_session.write().await;
        
        let window = session.get_active_window_mut()
            .ok_or_else(|| SessionError::InvalidState("No active window".to_string()))?;
        
        let tab = window.get_active_tab_mut()
            .ok_or_else(|| SessionError::InvalidState("No active tab".to_string()))?;
        
        let entry = tab.go_back().cloned();
        if entry.is_some() {
            tab.url = tab.current_entry().map(|e| e.url.clone()).unwrap_or_default();
            tab.title = tab.current_entry().map(|e| e.title.clone()).unwrap_or_default();
        }
        
        session.touch();
        Ok(entry)
    }
    
    /// Go forward in the active tab
    pub async fn go_forward(&self) -> Result<Option<HistoryEntry>> {
        let mut session = self.current_session.write().await;
        
        let window = session.get_active_window_mut()
            .ok_or_else(|| SessionError::InvalidState("No active window".to_string()))?;
        
        let tab = window.get_active_tab_mut()
            .ok_or_else(|| SessionError::InvalidState("No active tab".to_string()))?;
        
        let entry = tab.go_forward().cloned();
        if entry.is_some() {
            tab.url = tab.current_entry().map(|e| e.url.clone()).unwrap_or_default();
            tab.title = tab.current_entry().map(|e| e.title.clone()).unwrap_or_default();
        }
        
        session.touch();
        Ok(entry)
    }
    
    /// Check if we can go back
    pub async fn can_go_back(&self) -> bool {
        let session = self.current_session.read().await;
        
        if let Some(window) = session.get_active_window() {
            if let Some(tab) = window.get_active_tab() {
                return tab.can_go_back();
            }
        }
        false
    }
    
    /// Check if we can go forward
    pub async fn can_go_forward(&self) -> bool {
        let session = self.current_session.read().await;
        
        if let Some(window) = session.get_active_window() {
            if let Some(tab) = window.get_active_tab() {
                return tab.can_go_forward();
            }
        }
        false
    }
    
    /// Start auto-saving the session
    pub async fn start_auto_save(&self) -> Result<()> {
        let mut handle = self.auto_save_handle.write().await;
        
        if handle.is_some() {
            return Err(SessionError::AutoSaveAlreadyRunning);
        }
        
        let config = self.config.read().await.clone();
        let this = Arc::new(self.clone()?);
        
        let task = tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(config.auto_save_interval_secs));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = this.save_current_session().await {
                    error!("Auto-save failed: {}", e);
                } else {
                    debug!("Auto-saved session");
                }
            }
        });
        
        *handle = Some(task);
        info!("Started auto-save with {} second interval", config.auto_save_interval_secs);
        
        Ok(())
    }
    
    /// Stop auto-saving
    pub async fn stop_auto_save(&self) {
        let mut handle = self.auto_save_handle.write().await;
        
        if let Some(task) = handle.take() {
            task.abort();
            info!("Stopped auto-save");
        }
    }
    
    /// Mark shutdown as clean (call this on graceful shutdown)
    pub async fn mark_clean_shutdown(&self) -> Result<()> {
        let db = self.db.clone();
        let session_id = self.current_session_id().await;
        
        // Save one last time
        self.save_current_session().await?;
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            let mut table = txn.open_table(CRASH_RECOVERY_TABLE)?;
            
            let info = CrashRecoveryInfo {
                session_id,
                timestamp: SystemTime::now(),
                was_clean_shutdown: true,
                crash_count: 0,
            };
            
            let value = serde_json::to_vec(&info)?;
            table.insert("last_session", value.as_slice())?;
            
            txn.commit()?;
            Result::<()>::Ok(())
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))??;
        
        info!("Marked clean shutdown for session {}", session_id);
        Ok(())
    }
    
    /// Export session to JSON
    pub async fn export_to_json(&self, session_id: Option<Uuid>) -> Result<String> {
        let session = if let Some(id) = session_id {
            self.load_session(id).await?
        } else {
            self.current_session.read().await.clone()
        };
        
        let json = serde_json::to_string_pretty(&session)?;
        Ok(json)
    }
    
    /// Import session from JSON
    pub async fn import_from_json(&self, json: &str) -> Result<Session> {
        let session: Session = serde_json::from_str(json)?;
        self.save_session(&session).await?;
        Ok(session)
    }
    
    /// Get session statistics
    pub async fn get_stats(&self) -> Result<SessionStats> {
        let db = self.db.clone();
        let session = self.current_session.read().await.clone();
        
        let db_stats = tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let sessions_table = txn.open_table(SESSIONS_TABLE)?;
            
            Ok::<_, SessionError>(SessionStats {
                total_saved_sessions: sessions_table.len()?,
                current_session_windows: 0,
                current_session_tabs: 0,
            })
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))??;
        
        Ok(SessionStats {
            current_session_windows: session.window_count(),
            current_session_tabs: session.total_tab_count(),
            ..db_stats
        })
    }
    
    /// Prune old sessions
    pub async fn prune_old_sessions(&self) -> Result<u64> {
        let config = self.config.read().await.clone();
        let db = self.db.clone();
        let current_id = self.current_session_id().await;
        
        let pruned = tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            let mut table = txn.open_table(SESSIONS_TABLE)?;
            
            let mut sessions: Vec<(String, Session)> = Vec::new();
            for item in table.iter()? {
                let (key, value) = item?;
                let session: Session = serde_json::from_slice(value.value())?;
                sessions.push((key.value().to_string(), session));
            }
            
            // Sort by last accessed
            sessions.sort_by(|a, b| b.1.last_accessed.cmp(&a.1.last_accessed));
            
            // Remove old sessions
            let mut removed = 0u64;
            while sessions.len() > config.max_session_history {
                if let Some((key, session)) = sessions.pop() {
                    // Don't remove current session
                    if session.id != current_id {
                        table.remove(key.as_str())?;
                        removed += 1;
                    }
                }
            }
            
            txn.commit()?;
            Ok(removed)
        }).await.map_err(|e| SessionError::InvalidState(e.to_string()))??;
        
        if pruned > 0 {
            info!("Pruned {} old sessions", pruned);
        }
        
        Ok(pruned)
    }
    
    /// Close the session manager gracefully
    pub async fn close(&self) -> Result<()> {
        // Stop auto-save
        self.stop_auto_save().await;
        
        // Mark clean shutdown
        self.mark_clean_shutdown().await?;
        
        info!("SessionManager closed gracefully");
        Ok(())
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            config: RwLock::new(self.config.blocking_read().clone()),
            current_session: RwLock::new(self.current_session.blocking_read().clone()),
            auto_save_handle: RwLock::new(None),
            history_manager: self.history_manager.clone(),
        }
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_saved_sessions: u64,
    pub current_session_windows: usize,
    pub current_session_tabs: usize,
}

/// Navigation controller for managing navigation state
#[derive(Debug, Clone)]
pub struct NavigationController {
    /// History stack for the current navigation context
    history_stack: Vec<NavigationEntry>,
    /// Current position in the history stack
    current_position: usize,
    /// Reference to history manager for persistence
    history_manager: Option<Arc<HistoryManager>>,
    /// Whether to persist navigation to history
    persist_navigation: bool,
}

/// A single navigation entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NavigationEntry {
    /// URL navigated to
    pub url: String,
    /// Page title
    pub title: String,
    /// Timestamp of navigation
    pub timestamp: SystemTime,
    /// Transition type
    pub transition: TransitionType,
}

/// Type of navigation transition
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransitionType {
    /// User clicked a link
    Link,
    /// User typed the URL
    Typed,
    /// Page was auto-submitted (form, redirect)
    AutoSubmit,
    /// Navigation via back/forward buttons
    BackForward,
    /// Page reload
    Reload,
    /// Navigation from bookmark
    Bookmark,
    /// Navigation from session restore
    SessionRestore,
}

impl Default for NavigationController {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationController {
    /// Create a new navigation controller
    pub fn new() -> Self {
        Self {
            history_stack: Vec::with_capacity(100),
            current_position: 0,
            history_manager: None,
            persist_navigation: true,
        }
    }
    
    /// Create with a history manager
    pub fn with_history_manager(history_manager: Arc<HistoryManager>) -> Self {
        Self {
            history_stack: Vec::with_capacity(100),
            current_position: 0,
            history_manager: Some(history_manager),
            persist_navigation: true,
        }
    }
    
    /// Navigate to a new URL
    pub async fn navigate(&mut self, url: impl Into<String>, title: impl Into<String>) {
        let url = url.into();
        let title = title.into();
        
        // Remove forward history
        self.history_stack.truncate(self.current_position + 1);
        
        // Add new entry
        let entry = NavigationEntry {
            url: url.clone(),
            title: title.clone(),
            timestamp: SystemTime::now(),
            transition: TransitionType::Typed,
        };
        
        self.history_stack.push(entry);
        self.current_position = self.history_stack.len() - 1;
        
        // Persist to history if enabled
        if self.persist_navigation {
            if let Some(ref manager) = self.history_manager {
                let history_entry = HistoryEntry::new(url, title);
                let _ = manager.add_visit(history_entry).await;
            }
        }
    }
    
    /// Navigate back
    pub fn back(&mut self) -> Option<&NavigationEntry> {
        if self.can_go_back() {
            self.current_position -= 1;
            self.history_stack.get(self.current_position)
        } else {
            None
        }
    }
    
    /// Navigate forward
    pub fn forward(&mut self) -> Option<&NavigationEntry> {
        if self.can_go_forward() {
            self.current_position += 1;
            self.history_stack.get(self.current_position)
        } else {
            None
        }
    }
    
    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        self.current_position > 0
    }
    
    /// Check if we can go forward
    pub fn can_go_forward(&self) -> bool {
        self.current_position < self.history_stack.len().saturating_sub(1)
    }
    
    /// Get current entry
    pub fn current_entry(&self) -> Option<&NavigationEntry> {
        self.history_stack.get(self.current_position)
    }
    
    /// Get the history stack
    pub fn history_stack(&self) -> &[NavigationEntry] {
        &self.history_stack
    }
    
    /// Get current position
    pub fn current_position(&self) -> usize {
        self.current_position
    }
    
    /// Set persist navigation
    pub fn set_persist_navigation(&mut self, persist: bool) {
        self.persist_navigation = persist;
    }
    
    /// Clear history
    pub fn clear(&mut self) {
        self.history_stack.clear();
        self.current_position = 0;
    }
    
    /// Replace the current entry (for redirects)
    pub fn replace_current(&mut self, url: impl Into<String>, title: impl Into<String>) {
        if let Some(entry) = self.history_stack.get_mut(self.current_position) {
            entry.url = url.into();
            entry.title = title.into();
            entry.timestamp = SystemTime::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_manager() -> (SessionManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_sessions.redb");
        
        let mut config = SessionConfig::default();
        config.db_path = db_path;
        config.enable_crash_recovery = false;
        
        let manager = SessionManager::new(config).unwrap();
        (manager, temp_dir)
    }
    
    #[tokio::test]
    async fn test_new_session() {
        let (manager, _temp) = create_test_manager().await;
        
        let session = manager.current_session().await;
        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].tabs.len(), 1);
    }
    
    #[tokio::test]
    async fn test_add_window() {
        let (manager, _temp) = create_test_manager().await;
        
        let window = WindowState::new();
        let index = manager.add_window(window).await.unwrap();
        
        let session = manager.current_session().await;
        assert_eq!(session.windows.len(), 2);
        assert_eq!(index, 1);
    }
    
    #[tokio::test]
    async fn test_add_tab() {
        let (manager, _temp) = create_test_manager().await;
        
        let tab = TabState::new("https://example.com", "Example");
        let index = manager.add_tab(0, tab).await.unwrap();
        
        let session = manager.current_session().await;
        assert_eq!(session.windows[0].tabs.len(), 2);
        assert_eq!(index, 1);
    }
    
    #[tokio::test]
    async fn test_save_and_load_session() {
        let (manager, _temp) = create_test_manager().await;
        
        // Add some data
        let tab = TabState::new("https://example.com", "Example");
        manager.add_tab(0, tab).await.unwrap();
        
        // Save
        manager.save_current_session().await.unwrap();
        let session_id = manager.current_session_id().await;
        
        // Load
        let loaded = manager.load_session(session_id).await.unwrap();
        assert_eq!(loaded.windows[0].tabs.len(), 2);
    }
    
    #[tokio::test]
    async fn test_new_session_creates_fresh() {
        let (manager, _temp) = create_test_manager().await;
        
        // Add data to current session
        let tab = TabState::new("https://example.com", "Example");
        manager.add_tab(0, tab).await.unwrap();
        
        // Create new session
        let new_session = manager.new_session(Some("Test Session".to_string())).await.unwrap();
        
        assert_eq!(new_session.windows.len(), 1);
        assert_eq!(new_session.windows[0].tabs.len(), 1);
        assert_eq!(new_session.metadata.name, Some("Test Session".to_string()));
    }
    
    #[tokio::test]
    async fn test_delete_session() {
        let (manager, _temp) = create_test_manager().await;
        
        // Save current session
        manager.save_current_session().await.unwrap();
        
        // Create and save another session
        let new_session = manager.new_session(Some("To Delete".to_string())).await.unwrap();
        manager.save_session(&new_session).await.unwrap();
        let id_to_delete = new_session.id;
        
        // Delete it
        assert!(manager.delete_session(id_to_delete).await.unwrap());
        assert!(!manager.delete_session(id_to_delete).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_navigation() {
        let (manager, _temp) = create_test_manager().await;
        
        manager.navigate("https://example.com", "Example").await.unwrap();
        manager.navigate("https://example.org", "Example Org").await.unwrap();
        
        let session = manager.current_session().await;
        let window = session.get_active_window().unwrap();
        let tab = window.get_active_tab().unwrap();
        
        assert_eq!(tab.url, "https://example.org");
        assert_eq!(tab.history_stack.len(), 2);
        assert_eq!(tab.history_position, 1);
    }
    
    #[tokio::test]
    async fn test_go_back_forward() {
        let (manager, _temp) = create_test_manager().await;
        
        manager.navigate("https://example.com", "Example").await.unwrap();
        manager.navigate("https://example.org", "Example Org").await.unwrap();
        
        assert!(manager.can_go_back().await);
        assert!(!manager.can_go_forward().await);
        
        let entry = manager.go_back().await.unwrap();
        assert_eq!(entry.url, "https://example.com");
        
        assert!(manager.can_go_forward().await);
        
        let entry = manager.go_forward().await.unwrap();
        assert_eq!(entry.url, "https://example.org");
    }
    
    #[tokio::test]
    async fn test_export_import_json() {
        let (manager, _temp) = create_test_manager().await;
        
        let tab = TabState::new("https://example.com", "Example");
        manager.add_tab(0, tab).await.unwrap();
        
        let json = manager.export_to_json(None).await.unwrap();
        assert!(json.contains("https://example.com"));
        
        let imported = manager.import_from_json(&json).await.unwrap();
        assert_eq!(imported.windows[0].tabs.len(), 2);
    }
    
    #[tokio::test]
    async fn test_session_stats() {
        let (manager, _temp) = create_test_manager().await;
        
        let tab = TabState::new("https://example.com", "Example");
        manager.add_tab(0, tab).await.unwrap();
        manager.add_window(WindowState::new()).await.unwrap();
        
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.current_session_windows, 2);
        assert_eq!(stats.current_session_tabs, 2); // 1 original + 1 added per window
    }
    
    #[tokio::test]
    async fn test_navigation_controller() {
        let mut controller = NavigationController::new();
        
        controller.navigate("https://example.com", "Example").await;
        controller.navigate("https://example.org", "Example Org").await;
        
        assert!(controller.can_go_back());
        assert!(!controller.can_go_forward());
        
        let entry = controller.back().unwrap();
        assert_eq!(entry.url, "https://example.com");
        
        let entry = controller.forward().unwrap();
        assert_eq!(entry.url, "https://example.org");
    }
    
    #[tokio::test]
    async fn test_tab_state_history() {
        let mut tab = TabState::new("https://example.com", "Example");
        
        tab.navigate(HistoryEntry::new("https://page1.com", "Page 1"));
        tab.navigate(HistoryEntry::new("https://page2.com", "Page 2"));
        tab.navigate(HistoryEntry::new("https://page3.com", "Page 3"));
        
        assert_eq!(tab.history_stack.len(), 3);
        assert_eq!(tab.history_position, 2);
        
        tab.go_back();
        tab.go_back();
        assert_eq!(tab.history_position, 0);
        
        // Navigate to new page should clear forward history
        tab.navigate(HistoryEntry::new("https://page4.com", "Page 4"));
        assert_eq!(tab.history_stack.len(), 2);
        assert_eq!(tab.history_position, 1);
    }
}
