//! Browser session management

use crate::window::{Window, WindowManager};
use shared::BrowserConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a browser session
#[derive(Debug)]
pub struct BrowserSession {
    pub config: Arc<RwLock<BrowserConfig>>,
    pub window_manager: WindowManager,
    pub is_running: Arc<RwLock<bool>>,
}

impl BrowserSession {
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            window_manager: WindowManager::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> anyhow::Result<Window> {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        
        // Create initial window
        let window = self.window_manager.create_window(false).await;
        
        // Navigate to homepage
        let config = self.config.read().await;
        let homepage = config.homepage.clone();
        drop(config);
        
        if let Some(tab) = window.tab_manager.get_active_tab().await {
            tab.navigate(&homepage).await?;
        }
        
        Ok(window)
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Save session state
        // TODO: Implement session persistence
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn update_config(&self, config: BrowserConfig) {
        let mut c = self.config.write().await;
        *c = config;
    }

    pub async fn get_config(&self) -> BrowserConfig {
        self.config.read().await.clone()
    }
}

/// Session persistence
#[derive(Debug)]
pub struct SessionStorage {
    data_dir: std::path::PathBuf,
}

impl SessionStorage {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        Self { data_dir }
    }

    pub async fn save_session(&self, session: &BrowserSession) -> anyhow::Result<()> {
        let config = session.get_config().await;
        let config_path = self.data_dir.join("config.json");
        
        let config_json = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(config_path, config_json).await?;
        
        Ok(())
    }

    pub async fn load_session(&self) -> anyhow::Result<BrowserSession> {
        let config_path = self.data_dir.join("config.json");
        
        let config = if config_path.exists() {
            let config_json = tokio::fs::read_to_string(config_path).await?;
            serde_json::from_str(&config_json)?
        } else {
            BrowserConfig::default()
        };
        
        Ok(BrowserSession::new(config))
    }
}
