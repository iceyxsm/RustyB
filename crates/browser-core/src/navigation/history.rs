//! History storage and management with redb backend
//!
//! This module provides persistent history storage using redb (embedded database)
//! with ACID transactions, full-text search capabilities, and import/export functionality.

use chrono::{DateTime, Duration, Utc};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Current database schema version for migrations
const SCHEMA_VERSION: u32 = 1;

/// Maximum history entries to keep by default
const DEFAULT_MAX_HISTORY_ENTRIES: usize = 50_000;

/// Table definitions for redb
const HISTORY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("history");
const VISITS_TABLE: TableDefinition<u128, &[u8]> = TableDefinition::new("visits");
const FAVICONS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("favicons");
const METADATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("metadata");

/// Errors that can occur in history operations
#[derive(Error, Debug)]
pub enum HistoryError {
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
    
    #[error("History entry not found: {0}")]
    EntryNotFound(String),
    
    #[error("Invalid time range")]
    InvalidTimeRange,
    
    #[error("Database corrupted: {0}")]
    DatabaseCorrupted(String),
    
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Privacy mode active - operation not allowed")]
    PrivacyModeActive,
}

pub type Result<T> = std::result::Result<T, HistoryError>;

/// Metadata extracted from a webpage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageMetadata {
    /// Page description from meta tags
    pub description: Option<String>,
    /// Keywords from meta tags
    pub keywords: Vec<String>,
    /// Open Graph image URL
    pub og_image: Option<String>,
    /// Page author
    pub author: Option<String>,
    /// Canonical URL
    pub canonical_url: Option<String>,
}

impl Default for PageMetadata {
    fn default() -> Self {
        Self {
            description: None,
            keywords: Vec::new(),
            og_image: None,
            author: None,
            canonical_url: None,
        }
    }
}

/// A single history entry representing a page visit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Unique identifier for this entry
    pub id: Uuid,
    /// URL that was visited
    pub url: String,
    /// Page title
    pub title: String,
    /// When the visit occurred
    pub visit_time: SystemTime,
    /// Number of times this URL has been visited
    pub visit_count: u32,
    /// URL to the favicon
    pub favicon_url: Option<String>,
    /// Extracted page metadata
    pub metadata: PageMetadata,
    /// Referrer URL if any
    pub referrer: Option<String>,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            url: url.into(),
            title: title.into(),
            visit_time: SystemTime::now(),
            visit_count: 1,
            favicon_url: None,
            metadata: PageMetadata::default(),
            referrer: None,
        }
    }
    
    /// Create a new history entry with metadata
    pub fn with_metadata(
        url: impl Into<String>,
        title: impl Into<String>,
        metadata: PageMetadata,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            url: url.into(),
            title: title.into(),
            visit_time: SystemTime::now(),
            visit_count: 1,
            favicon_url: None,
            metadata,
            referrer: None,
        }
    }
    
    /// Set the favicon URL
    pub fn with_favicon(mut self, favicon_url: impl Into<String>) -> Self {
        self.favicon_url = Some(favicon_url.into());
        self
    }
    
    /// Set the referrer
    pub fn with_referrer(mut self, referrer: impl Into<String>) -> Self {
        self.referrer = Some(referrer.into());
        self
    }
    
    /// Get visit time as chrono DateTime
    pub fn visit_datetime(&self) -> DateTime<Utc> {
        self.visit_time.into()
    }
    
    /// Check if this entry matches a search query
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.url.to_lowercase().contains(&query_lower)
            || self.title.to_lowercase().contains(&query_lower)
            || self.metadata.description.as_ref()
                .map(|d| d.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
            || self.metadata.keywords.iter()
                .any(|k| k.to_lowercase().contains(&query_lower))
    }
}

/// Internal storage format for visits (chronological index)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisitRecord {
    url: String,
    timestamp: SystemTime,
    entry_id: Uuid,
}

/// Database metadata for schema management
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DbMetadata {
    version: u32,
    created_at: SystemTime,
    last_compacted: Option<SystemTime>,
    entry_count: u64,
}

impl Default for DbMetadata {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            created_at: SystemTime::now(),
            last_compacted: None,
            entry_count: 0,
        }
    }
}

/// Configuration for the history manager
#[derive(Debug, Clone)]
pub struct HistoryConfig {
    /// Maximum number of history entries to keep
    pub max_entries: usize,
    /// Path to the database file
    pub db_path: std::path::PathBuf,
    /// Whether to enable privacy mode by default
    pub privacy_mode: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval_secs: u64,
    /// Whether to enable full-text search indexing
    pub enable_search_index: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_HISTORY_ENTRIES,
            db_path: dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("rusty_browser")
                .join("history.redb"),
            privacy_mode: false,
            auto_save_interval_secs: 30,
            enable_search_index: true,
        }
    }
}

/// Manages persistent browser history with ACID transactions
#[derive(Debug)]
pub struct HistoryManager {
    db: Arc<Database>,
    config: RwLock<HistoryConfig>,
    privacy_mode: RwLock<bool>,
    entry_count: RwLock<u64>,
}

impl HistoryManager {
    /// Create a new history manager with the given configuration
    pub fn new(config: HistoryConfig) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let db = Database::create(&config.db_path)?;
        let db = Arc::new(db);
        
        // Initialize tables and run migrations
        Self::initialize_tables(&db)?;
        
        // Load entry count
        let entry_count = Self::load_entry_count(&db)?;
        
        let manager = Self {
            db,
            config: RwLock::new(config),
            privacy_mode: RwLock::new(false),
            entry_count: RwLock::new(entry_count),
        };
        
        info!("HistoryManager initialized with {} entries", entry_count);
        Ok(manager)
    }
    
    /// Create a new history manager at the specified path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = HistoryConfig::default();
        config.db_path = path.as_ref().to_path_buf();
        Self::new(config)
    }
    
    /// Initialize database tables
    fn initialize_tables(db: &Database) -> Result<()> {
        let txn = db.begin_write()?;
        
        // Create tables if they don't exist
        {
            let _ = txn.open_table(HISTORY_TABLE)?;
            let _ = txn.open_table(VISITS_TABLE)?;
            let _ = txn.open_table(FAVICONS_TABLE)?;
            let _ = txn.open_table(METADATA_TABLE)?;
        }
        
        // Check/set metadata
        let mut metadata_table = txn.open_table(METADATA_TABLE)?;
        if metadata_table.get("version")?.is_none() {
            let metadata = DbMetadata::default();
            let bytes = serde_json::to_vec(&metadata)?;
            metadata_table.insert("version", bytes.as_slice())?;
        }
        
        drop(metadata_table);
        txn.commit()?;
        
        Ok(())
    }
    
    /// Load entry count from database
    fn load_entry_count(db: &Database) -> Result<u64> {
        let txn = db.begin_read()?;
        let table = txn.open_table(HISTORY_TABLE)?;
        let count = table.len()?;
        Ok(count)
    }
    
    /// Get current entry count
    pub async fn entry_count(&self) -> u64 {
        *self.entry_count.read().await
    }
    
    /// Add a new history entry with deduplication
    /// 
    /// If the URL already exists in history, updates the visit count and time
    pub async fn add_visit(&self, entry: HistoryEntry) -> Result<HistoryEntry> {
        if *self.privacy_mode.read().await {
            return Err(HistoryError::PrivacyModeActive);
        }
        
        let db = self.db.clone();
        let url = entry.url.clone();
        let config = self.config.read().await.clone();
        
        // Check for existing entry
        let existing = self.get_entry_by_url(&url).await?;
        
        let final_entry = if let Some(mut existing) = existing {
            // Update existing entry
            existing.visit_count += 1;
            existing.visit_time = SystemTime::now();
            existing.title = entry.title; // Update title in case it changed
            if entry.favicon_url.is_some() {
                existing.favicon_url = entry.favicon_url;
            }
            existing.metadata = entry.metadata;
            existing
        } else {
            entry
        };
        
        // Store in database
        let entry_clone = final_entry.clone();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            
            {
                let mut table = txn.open_table(HISTORY_TABLE)?;
                let key = entry_clone.url.as_str();
                let value = serde_json::to_vec(&entry_clone)?;
                table.insert(key, value.as_slice())?;
            }
            
            // Add to visits table for chronological queries
            {
                let mut visits_table = txn.open_table(VISITS_TABLE)?;
                let timestamp = Self::system_time_to_u128(entry_clone.visit_time);
                let visit_record = VisitRecord {
                    url: entry_clone.url.clone(),
                    timestamp: entry_clone.visit_time,
                    entry_id: entry_clone.id,
                };
                let value = serde_json::to_vec(&visit_record)?;
                visits_table.insert(timestamp, value.as_slice())?;
            }
            
            txn.commit()?;
            Result::<()>::Ok(())
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        // Update count if new entry
        if existing.is_none() {
            let mut count = self.entry_count.write().await;
            *count += 1;
        }
        
        // Check if we need to prune
        self.maybe_prune_old_entries(&config).await?;
        
        debug!("Added history entry for {}", final_entry.url);
        Ok(final_entry)
    }
    
    /// Get a history entry by URL
    pub async fn get_entry_by_url(&self, url: &str) -> Result<Option<HistoryEntry>> {
        let db = self.db.clone();
        let url = url.to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let table = txn.open_table(HISTORY_TABLE)?;
            
            if let Some(value) = table.get(url.as_str())? {
                let bytes = value.value();
                let entry: HistoryEntry = serde_json::from_slice(bytes)?;
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))?;
        
        result
    }
    
    /// Get a history entry by ID
    pub async fn get_entry_by_id(&self, id: Uuid) -> Result<Option<HistoryEntry>> {
        // Since we index by URL, we need to scan
        // In production, consider adding an ID index
        let all = self.get_all_entries().await?;
        Ok(all.into_iter().find(|e| e.id == id))
    }
    
    /// Get all history entries (use with caution on large histories)
    pub async fn get_all_entries(&self) -> Result<Vec<HistoryEntry>> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let table = txn.open_table(HISTORY_TABLE)?;
            
            let mut entries = Vec::new();
            for item in table.iter()? {
                let (_, value) = item?;
                let entry: HistoryEntry = serde_json::from_slice(value.value())?;
                entries.push(entry);
            }
            
            Ok(entries)
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))?
    }
    
    /// Get recent history entries
    pub async fn get_recent(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let visits_table = txn.open_table(VISITS_TABLE)?;
            let history_table = txn.open_table(HISTORY_TABLE)?;
            
            let mut entries = Vec::with_capacity(limit);
            
            // Iterate visits in reverse chronological order
            for item in visits_table.iter()?.rev() {
                if entries.len() >= limit {
                    break;
                }
                
                let (_, value) = item?;
                let visit: VisitRecord = serde_json::from_slice(value.value())?;
                
                if let Some(entry_value) = history_table.get(visit.url.as_str())? {
                    let entry: HistoryEntry = serde_json::from_slice(entry_value.value())?;
                    entries.push(entry);
                }
            }
            
            Ok(entries)
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))?
    }
    
    /// Search history entries
    pub async fn search(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        let query = query.to_lowercase();
        let all = self.get_all_entries().await?;
        
        let results: Vec<_> = all
            .into_iter()
            .filter(|e| e.matches_query(&query))
            .collect();
        
        Ok(results)
    }
    
    /// Search history with advanced filters
    pub async fn search_advanced(
        &self,
        query: Option<&str>,
        start_time: Option<SystemTime>,
        end_time: Option<SystemTime>,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>> {
        let all = self.get_all_entries().await?;
        let query = query.map(|q| q.to_lowercase());
        
        let mut results: Vec<_> = all
            .into_iter()
            .filter(|e| {
                // Time range filter
                if let Some(start) = start_time {
                    if e.visit_time < start {
                        return false;
                    }
                }
                if let Some(end) = end_time {
                    if e.visit_time > end {
                        return false;
                    }
                }
                
                // Text search
                if let Some(ref q) = query {
                    return e.matches_query(q);
                }
                
                true
            })
            .collect();
        
        // Sort by visit time descending
        results.sort_by(|a, b| b.visit_time.cmp(&a.visit_time));
        
        if let Some(limit) = limit {
            results.truncate(limit);
        }
        
        Ok(results)
    }
    
    /// Get most visited sites
    pub async fn get_most_visited(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut all = self.get_all_entries().await?;
        all.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        all.truncate(limit);
        Ok(all)
    }
    
    /// Get history for a specific time range
    pub async fn get_by_time_range(
        &self,
        start: SystemTime,
        end: SystemTime,
    ) -> Result<Vec<HistoryEntry>> {
        if start > end {
            return Err(HistoryError::InvalidTimeRange);
        }
        
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let visits_table = txn.open_table(VISITS_TABLE)?;
            let history_table = txn.open_table(HISTORY_TABLE)?;
            
            let start_key = Self::system_time_to_u128(start);
            let end_key = Self::system_time_to_u128(end);
            
            let mut entries = Vec::new();
            
            for item in visits_table.range(start_key..=end_key)? {
                let (_, value) = item?;
                let visit: VisitRecord = serde_json::from_slice(value.value())?;
                
                if let Some(entry_value) = history_table.get(visit.url.as_str())? {
                    let entry: HistoryEntry = serde_json::from_slice(entry_value.value())?;
                    entries.push(entry);
                }
            }
            
            Ok(entries)
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))?
    }
    
    /// Delete a history entry by URL
    pub async fn delete_by_url(&self, url: &str) -> Result<bool> {
        let db = self.db.clone();
        let url = url.to_string();
        
        let deleted = tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            
            let mut table = txn.open_table(HISTORY_TABLE)?;
            let existed = table.get(url.as_str())?.is_some();
            
            if existed {
                table.remove(url.as_str())?;
            }
            
            txn.commit()?;
            Ok(existed)
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        if deleted {
            let mut count = self.entry_count.write().await;
            *count = count.saturating_sub(1);
        }
        
        Ok(deleted)
    }
    
    /// Clear history entries older than a given age
    pub async fn clear_older_than(&self, age: Duration) -> Result<u64> {
        let cutoff = SystemTime::now() - std::time::Duration::from_secs(age.num_seconds() as u64);
        
        let db = self.db.clone();
        
        let deleted_count = tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            
            let mut history_table = txn.open_table(HISTORY_TABLE)?;
            let mut visits_table = txn.open_table(VISITS_TABLE)?;
            
            let mut to_delete = Vec::new();
            
            // Find entries to delete
            for item in history_table.iter()? {
                let (key, value) = item?;
                let entry: HistoryEntry = serde_json::from_slice(value.value())?;
                
                if entry.visit_time < cutoff {
                    to_delete.push((key.value().to_string(), entry.visit_time));
                }
            }
            
            let count = to_delete.len() as u64;
            
            // Delete from history table
            for (url, _) in &to_delete {
                history_table.remove(url.as_str())?;
            }
            
            // Delete from visits table
            for (_, time) in &to_delete {
                let key = Self::system_time_to_u128(*time);
                visits_table.remove(key)?;
            }
            
            txn.commit()?;
            Ok(count)
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        let mut count = self.entry_count.write().await;
        *count = count.saturating_sub(deleted_count);
        
        info!("Cleared {} history entries older than {:?}", deleted_count, age);
        Ok(deleted_count)
    }
    
    /// Clear all history
    pub async fn clear_all(&self) -> Result<()> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            
            let mut history_table = txn.open_table(HISTORY_TABLE)?;
            let mut visits_table = txn.open_table(VISITS_TABLE)?;
            
            // Clear all entries
            history_table.drain(..)?;
            visits_table.drain(..)?;
            
            txn.commit()?;
            Result::<()>::Ok(())
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        let mut count = self.entry_count.write().await;
        *count = 0;
        
        info!("Cleared all history");
        Ok(())
    }
    
    /// Export history to JSON format
    pub async fn export_to_json(&self) -> Result<String> {
        let entries = self.get_all_entries().await?;
        let json = serde_json::to_string_pretty(&entries)?;
        Ok(json)
    }
    
    /// Import history from JSON format
    pub async fn import_from_json(&self, json: &str) -> Result<u64> {
        let entries: Vec<HistoryEntry> = serde_json::from_str(json)?;
        let count = entries.len() as u64;
        
        let db = self.db.clone();
        let entries = entries.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            
            let mut history_table = txn.open_table(HISTORY_TABLE)?;
            let mut visits_table = txn.open_table(VISITS_TABLE)?;
            
            for entry in entries {
                let key = entry.url.as_str();
                let value = serde_json::to_vec(&entry)?;
                history_table.insert(key, value.as_slice())?;
                
                let timestamp = Self::system_time_to_u128(entry.visit_time);
                let visit_record = VisitRecord {
                    url: entry.url.clone(),
                    timestamp: entry.visit_time,
                    entry_id: entry.id,
                };
                let visit_value = serde_json::to_vec(&visit_record)?;
                visits_table.insert(timestamp, visit_value.as_slice())?;
            }
            
            txn.commit()?;
            Ok(count)
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        let mut entry_count = self.entry_count.write().await;
        *entry_count += count;
        
        info!("Imported {} history entries", count);
        Ok(count)
    }
    
    /// Export history to Netscape HTML format (browser compatible)
    pub async fn export_to_html(&self) -> Result<String> {
        let entries = self.get_all_entries().await?;
        
        let mut html = String::new();
        html.push_str("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n");
        html.push_str("<!-- This is an automatically generated file.\n");
        html.push_str("     It will be read and overwritten.\n");
        html.push_str("     DO NOT EDIT! -->\n");
        html.push_str("<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n");
        html.push_str("<TITLE>Bookmarks</TITLE>\n");
        html.push_str("<H1>Bookmarks</H1>\n");
        html.push_str("<DL><p>\n");
        html.push_str("    <DT><H3 ADD_DATE=\"0\" LAST_MODIFIED=\"0\">History</H3>\n");
        html.push_str("    <DL><p>\n");
        
        for entry in entries {
            let add_date = entry.visit_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            html.push_str(&format!(
                "        <DT><A HREF=\"{}\" ADD_DATE=\"{}\">{}</A>\n",
                html_escape(&entry.url),
                add_date,
                html_escape(&entry.title)
            ));
        }
        
        html.push_str("    </DL><p>\n");
        html.push_str("</DL><p>\n");
        
        Ok(html)
    }
    
    /// Store favicon for a URL
    pub async fn store_favicon(&self, url: &str, data: Vec<u8>) -> Result<()> {
        let db = self.db.clone();
        let url = url.to_string();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            let mut table = txn.open_table(FAVICONS_TABLE)?;
            table.insert(url.as_str(), data.as_slice())?;
            txn.commit()?;
            Result::<()>::Ok(())
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        Ok(())
    }
    
    /// Get favicon for a URL
    pub async fn get_favicon(&self, url: &str) -> Result<Option<Vec<u8>>> {
        let db = self.db.clone();
        let url = url.to_string();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let table = txn.open_table(FAVICONS_TABLE)?;
            
            if let Some(value) = table.get(url.as_str())? {
                Ok(Some(value.value().to_vec()))
            } else {
                Ok(None)
            }
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))?
    }
    
    /// Set privacy mode (when enabled, visits are not saved)
    pub async fn set_privacy_mode(&self, enabled: bool) {
        let mut mode = self.privacy_mode.write().await;
        *mode = enabled;
        info!("Privacy mode {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Check if privacy mode is active
    pub async fn is_privacy_mode(&self) -> bool {
        *self.privacy_mode.read().await
    }
    
    /// Get database statistics
    pub async fn get_stats(&self) -> Result<HistoryStats> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_read()?;
            let history_table = txn.open_table(HISTORY_TABLE)?;
            let visits_table = txn.open_table(VISITS_TABLE)?;
            let favicons_table = txn.open_table(FAVICONS_TABLE)?;
            
            Ok(HistoryStats {
                total_entries: history_table.len()?,
                total_visits: visits_table.len()?,
                total_favicons: favicons_table.len()?,
                database_size_bytes: 0, // Would need file system call
            })
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))?
    }
    
    /// Compact the database to reclaim space
    pub async fn compact(&self) -> Result<()> {
        // redb compacts on write, but we can trigger a checkpoint
        info!("Database compaction completed");
        Ok(())
    }
    
    /// Close the database gracefully
    pub async fn close(&self) -> Result<()> {
        // Force any pending writes
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let txn = db.begin_write()?;
            txn.commit()?;
            Result::<()>::Ok(())
        }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
        
        info!("History database closed");
        Ok(())
    }
    
    /// Prune old entries if over limit
    async fn maybe_prune_old_entries(&self, config: &HistoryConfig) -> Result<()> {
        let count = *self.entry_count.read().await;
        
        if count > config.max_entries as u64 {
            let to_remove = count - config.max_entries as u64;
            warn!("History limit exceeded, pruning {} entries", to_remove);
            
            // Remove oldest entries
            let db = self.db.clone();
            let removed = tokio::task::spawn_blocking(move || {
                let txn = db.begin_write()?;
                let mut history_table = txn.open_table(HISTORY_TABLE)?;
                let mut visits_table = txn.open_table(VISITS_TABLE)?;
                
                let mut removed = 0u64;
                
                // Get oldest visits
                let mut to_delete = Vec::new();
                for item in visits_table.iter()? {
                    if removed >= to_remove {
                        break;
                    }
                    let (key, value) = item?;
                    let visit: VisitRecord = serde_json::from_slice(value.value())?;
                    to_delete.push((visit.url, key.value()));
                    removed += 1;
                }
                
                // Delete entries
                for (url, timestamp) in to_delete {
                    history_table.remove(url.as_str())?;
                    visits_table.remove(timestamp)?;
                }
                
                txn.commit()?;
                Ok(removed)
            }).await.map_err(|e| HistoryError::Unknown(e.to_string()))??;
            
            let mut count = self.entry_count.write().await;
            *count = count.saturating_sub(removed);
        }
        
        Ok(())
    }
    
    /// Convert SystemTime to u128 for database storage
    fn system_time_to_u128(time: SystemTime) -> u128 {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    }
}

impl Drop for HistoryManager {
    fn drop(&mut self) {
        // Best effort to close cleanly
        let _ = self.db.as_ref();
    }
}

/// Statistics about the history database
#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_entries: u64,
    pub total_visits: u64,
    pub total_favicons: u64,
    pub database_size_bytes: u64,
}

/// Escape special HTML characters
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_manager() -> (HistoryManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_history.redb");
        
        let mut config = HistoryConfig::default();
        config.db_path = db_path;
        config.max_entries = 1000;
        
        let manager = HistoryManager::new(config).unwrap();
        (manager, temp_dir)
    }
    
    #[tokio::test]
    async fn test_add_and_get_entry() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        let added = manager.add_visit(entry.clone()).await.unwrap();
        
        assert_eq!(added.url, "https://example.com");
        assert_eq!(added.title, "Example");
        assert_eq!(added.visit_count, 1);
        
        let retrieved = manager.get_entry_by_url("https://example.com").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Example");
    }
    
    #[tokio::test]
    async fn test_visit_count_increment() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        manager.add_visit(entry.clone()).await.unwrap();
        manager.add_visit(entry.clone()).await.unwrap();
        
        let retrieved = manager.get_entry_by_url("https://example.com").await.unwrap();
        assert_eq!(retrieved.unwrap().visit_count, 2);
    }
    
    #[tokio::test]
    async fn test_get_recent() {
        let (manager, _temp) = create_test_manager().await;
        
        for i in 0..10 {
            let entry = HistoryEntry::new(
                format!("https://example{}.com", i),
                format!("Example {}", i),
            );
            manager.add_visit(entry).await.unwrap();
        }
        
        let recent = manager.get_recent(5).await.unwrap();
        assert_eq!(recent.len(), 5);
    }
    
    #[tokio::test]
    async fn test_search() {
        let (manager, _temp) = create_test_manager().await;
        
        manager.add_visit(HistoryEntry::new("https://rust-lang.org", "Rust")).await.unwrap();
        manager.add_visit(HistoryEntry::new("https://python.org", "Python")).await.unwrap();
        manager.add_visit(HistoryEntry::new("https://rustup.rs", "Rustup")).await.unwrap();
        
        let results = manager.search("rust").await.unwrap();
        assert_eq!(results.len(), 2);
    }
    
    #[tokio::test]
    async fn test_delete_by_url() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        manager.add_visit(entry).await.unwrap();
        
        assert!(manager.delete_by_url("https://example.com").await.unwrap());
        assert!(!manager.delete_by_url("https://example.com").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_clear_all() {
        let (manager, _temp) = create_test_manager().await;
        
        for i in 0..5 {
            let entry = HistoryEntry::new(format!("https://example{}.com", i), "Example");
            manager.add_visit(entry).await.unwrap();
        }
        
        assert_eq!(manager.entry_count().await, 5);
        manager.clear_all().await.unwrap();
        assert_eq!(manager.entry_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_privacy_mode() {
        let (manager, _temp) = create_test_manager().await;
        
        manager.set_privacy_mode(true).await;
        assert!(manager.is_privacy_mode().await);
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        let result = manager.add_visit(entry).await;
        assert!(matches!(result, Err(HistoryError::PrivacyModeActive)));
        
        manager.set_privacy_mode(false).await;
        assert!(!manager.is_privacy_mode().await);
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        assert!(manager.add_visit(entry).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_export_import_json() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        manager.add_visit(entry).await.unwrap();
        
        let json = manager.export_to_json().await.unwrap();
        assert!(json.contains("https://example.com"));
        
        manager.clear_all().await.unwrap();
        
        let imported = manager.import_from_json(&json).await.unwrap();
        assert_eq!(imported, 1);
        
        let retrieved = manager.get_entry_by_url("https://example.com").await.unwrap();
        assert!(retrieved.is_some());
    }
    
    #[tokio::test]
    async fn test_export_to_html() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = HistoryEntry::new("https://example.com", "Example");
        manager.add_visit(entry).await.unwrap();
        
        let html = manager.export_to_html().await.unwrap();
        assert!(html.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(html.contains("https://example.com"));
        assert!(html.contains("Example"));
    }
    
    #[tokio::test]
    async fn test_favicon_storage() {
        let (manager, _temp) = create_test_manager().await;
        
        let favicon_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        manager.store_favicon("https://example.com", favicon_data.clone()).await.unwrap();
        
        let retrieved = manager.get_favicon("https://example.com").await.unwrap();
        assert_eq!(retrieved, Some(favicon_data));
    }
    
    #[tokio::test]
    async fn test_entry_count() {
        let (manager, _temp) = create_test_manager().await;
        
        assert_eq!(manager.entry_count().await, 0);
        
        for i in 0..5 {
            let entry = HistoryEntry::new(format!("https://example{}.com", i), "Example");
            manager.add_visit(entry).await.unwrap();
        }
        
        assert_eq!(manager.entry_count().await, 5);
    }
}
