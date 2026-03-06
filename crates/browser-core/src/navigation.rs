//! Navigation and history management

use shared::{HistoryEntry, LoadState};
use std::collections::VecDeque;
use tokio::sync::RwLock;

/// Maximum history entries to keep per tab
const MAX_HISTORY_ENTRIES: usize = 1000;

/// Manages navigation history
#[derive(Debug)]
pub struct HistoryManager {
    entries: RwLock<VecDeque<HistoryEntry>>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(MAX_HISTORY_ENTRIES)),
        }
    }

    pub async fn add_entry(&self, entry: HistoryEntry) {
        let mut entries = self.entries.write().await;
        
        // Remove oldest if at capacity
        if entries.len() >= MAX_HISTORY_ENTRIES {
            entries.pop_front();
        }
        
        entries.push_back(entry);
    }

    pub async fn get_recent(&self, limit: usize) -> Vec<HistoryEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().take(limit).cloned().collect()
    }

    pub async fn search(&self, query: &str) -> Vec<HistoryEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| {
                e.url.to_lowercase().contains(&query.to_lowercase())
                    || e.title
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&query.to_lowercase()))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn get_count(&self) -> usize {
        self.entries.read().await.len()
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Download manager
#[derive(Debug)]
pub struct DownloadManager {
    downloads: RwLock<Vec<Download>>,
}

#[derive(Debug, Clone)]
pub struct Download {
    pub id: uuid::Uuid,
    pub url: String,
    pub filename: String,
    pub path: std::path::PathBuf,
    pub status: DownloadStatus,
    pub progress: f64,
    pub total_size: Option<u64>,
    pub downloaded_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            downloads: RwLock::new(Vec::new()),
        }
    }

    pub async fn start_download(&self, url: String, filename: String, path: std::path::PathBuf) -> uuid::Uuid {
        let id = uuid::Uuid::new_v4();
        let download = Download {
            id,
            url,
            filename,
            path,
            status: DownloadStatus::Pending,
            progress: 0.0,
            total_size: None,
            downloaded_size: 0,
        };

        let mut downloads = self.downloads.write().await;
        downloads.push(download);

        id
    }

    pub async fn get_download(&self, id: uuid::Uuid) -> Option<Download> {
        let downloads = self.downloads.read().await;
        downloads.iter().find(|d| d.id == id).cloned()
    }

    pub async fn get_all_downloads(&self) -> Vec<Download> {
        self.downloads.read().await.clone()
    }

    pub async fn update_progress(&self, id: uuid::Uuid, downloaded: u64, total: Option<u64>) {
        let mut downloads = self.downloads.write().await;
        if let Some(download) = downloads.iter_mut().find(|d| d.id == id) {
            download.downloaded_size = downloaded;
            download.total_size = total;
            
            if let Some(total) = total {
                download.progress = (downloaded as f64 / total as f64) * 100.0;
            }
            
            download.status = DownloadStatus::InProgress;
        }
    }

    pub async fn complete_download(&self, id: uuid::Uuid) {
        let mut downloads = self.downloads.write().await;
        if let Some(download) = downloads.iter_mut().find(|d| d.id == id) {
            download.status = DownloadStatus::Completed;
            download.progress = 100.0;
        }
    }

    pub async fn fail_download(&self, id: uuid::Uuid) {
        let mut downloads = self.downloads.write().await;
        if let Some(download) = downloads.iter_mut().find(|d| d.id == id) {
            download.status = DownloadStatus::Failed;
        }
    }

    pub async fn cancel_download(&self, id: uuid::Uuid) {
        let mut downloads = self.downloads.write().await;
        if let Some(download) = downloads.iter_mut().find(|d| d.id == id) {
            download.status = DownloadStatus::Cancelled;
        }
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}
