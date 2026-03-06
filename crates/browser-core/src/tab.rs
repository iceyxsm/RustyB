//! Browser tab management

use chrono::Utc;
use shared::{HistoryEntry, LoadState, TabId, Url};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Represents a browser tab
#[derive(Debug)]
pub struct Tab {
    pub id: TabId,
    pub window_id: Option<uuid::Uuid>,
    pub state: Arc<RwLock<TabState>>,
    navigation: Arc<RwLock<NavigationState>>,
}

#[derive(Debug, Clone)]
pub struct TabState {
    pub title: Option<String>,
    pub url: Option<String>,
    pub load_state: LoadState,
    pub favicon: Option<Vec<u8>>,
    pub zoom_level: f64,
    pub muted: bool,
}

#[derive(Debug, Clone)]
pub struct NavigationState {
    pub history: Vec<HistoryEntry>,
    pub current_index: usize,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

impl Tab {
    pub fn new(window_id: Option<uuid::Uuid>) -> Self {
        let id = TabId::new();
        debug!("Creating new tab: {:?}", id);

        Self {
            id,
            window_id,
            state: Arc::new(RwLock::new(TabState {
                title: None,
                url: None,
                load_state: LoadState::Idle,
                favicon: None,
                zoom_level: 1.0,
                muted: false,
            })),
            navigation: Arc::new(RwLock::new(NavigationState {
                history: Vec::new(),
                current_index: 0,
                can_go_back: false,
                can_go_forward: false,
            })),
        }
    }

    pub async fn navigate(&self, url: &str) -> anyhow::Result<()> {
        let parsed = Url::parse(url)?;
        info!("Tab {:?} navigating to: {}", self.id, parsed.raw);

        // Update state
        {
            let mut state = self.state.write().await;
            state.load_state = LoadState::Loading;
            state.url = Some(parsed.raw.clone());
        }

        // Add to history
        {
            let mut nav = self.navigation.write().await;
            let entry = HistoryEntry {
                id: Uuid::new_v4(),
                url: parsed.raw,
                title: None,
                timestamp: Utc::now(),
                visit_count: 1,
            };

            // Remove forward history if navigating from middle
            let current_index = nav.current_index;
            if current_index < nav.history.len().saturating_sub(1) {
                nav.history.truncate(current_index + 1);
            }

            nav.history.push(entry);
            nav.current_index = nav.history.len() - 1;
            nav.update_navigation_state();
        }

        Ok(())
    }

    pub async fn go_back(&self) -> Option<String> {
        let mut nav = self.navigation.write().await;
        
        if nav.current_index > 0 {
            nav.current_index -= 1;
            nav.update_navigation_state();
            
            let url = nav.history[nav.current_index].url.clone();
            drop(nav);
            
            let _ = self.navigate(&url).await;
            Some(url)
        } else {
            None
        }
    }

    pub async fn go_forward(&self) -> Option<String> {
        let mut nav = self.navigation.write().await;
        
        if nav.current_index < nav.history.len() - 1 {
            nav.current_index += 1;
            nav.update_navigation_state();
            
            let url = nav.history[nav.current_index].url.clone();
            drop(nav);
            
            let _ = self.navigate(&url).await;
            Some(url)
        } else {
            None
        }
    }

    pub async fn reload(&self) -> Option<String> {
        let state = self.state.read().await;
        state.url.clone()
    }

    pub async fn update_title(&self, title: String) {
        let mut state = self.state.write().await;
        state.title = Some(title.clone());
        
        // Also update history
        let mut nav = self.navigation.write().await;
        let current_index = nav.current_index;
        if let Some(entry) = nav.history.get_mut(current_index) {
            entry.title = Some(title);
        }
    }

    pub async fn set_load_state(&self, state: LoadState) {
        let mut tab_state = self.state.write().await;
        tab_state.load_state = state;
    }

    pub async fn get_state(&self) -> TabState {
        self.state.read().await.clone()
    }

    pub async fn get_navigation_state(&self) -> NavigationState {
        self.navigation.read().await.clone()
    }

    pub async fn set_zoom(&self, level: f64) {
        let mut state = self.state.write().await;
        state.zoom_level = level.clamp(0.25, 5.0);
    }

    pub async fn toggle_mute(&self) -> bool {
        let mut state = self.state.write().await;
        state.muted = !state.muted;
        state.muted
    }
}

impl NavigationState {
    fn update_navigation_state(&mut self) {
        self.can_go_back = self.current_index > 0;
        self.can_go_forward = self.current_index < self.history.len() - 1;
    }
}

/// Manages all tabs
#[derive(Debug)]
pub struct TabManager {
    tabs: Arc<RwLock<Vec<Tab>>>,
    active_tab: Arc<RwLock<Option<TabId>>>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(RwLock::new(Vec::new())),
            active_tab: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn create_tab(&self, window_id: Option<uuid::Uuid>) -> TabId {
        let tab = Tab::new(window_id);
        let id = tab.id;
        
        let mut tabs = self.tabs.write().await;
        tabs.push(tab);
        
        // Set as active if first tab
        let mut active = self.active_tab.write().await;
        if active.is_none() {
            *active = Some(id);
        }
        
        id
    }

    pub async fn close_tab(&self, id: TabId) -> Option<TabId> {
        let mut tabs = self.tabs.write().await;
        
        if let Some(index) = tabs.iter().position(|t| t.id == id) {
            tabs.remove(index);
            
            // Update active tab
            let mut active = self.active_tab.write().await;
            if *active == Some(id) {
                *active = tabs.get(index.saturating_sub(1)).map(|t| t.id);
            }
            
            *active
        } else {
            None
        }
    }

    pub async fn get_tab(&self, id: TabId) -> Option<Tab> {
        let tabs = self.tabs.read().await;
        tabs.iter().find(|t| t.id == id).cloned()
    }

    pub async fn get_active_tab(&self) -> Option<Tab> {
        let active = self.active_tab.read().await;
        if let Some(id) = *active {
            self.get_tab(id).await
        } else {
            None
        }
    }

    pub async fn set_active_tab(&self, id: TabId) -> bool {
        let tabs = self.tabs.read().await;
        if tabs.iter().any(|t| t.id == id) {
            let mut active = self.active_tab.write().await;
            *active = Some(id);
            true
        } else {
            false
        }
    }

    pub async fn get_all_tabs(&self) -> Vec<Tab> {
        self.tabs.read().await.clone()
    }

    pub async fn get_tab_count(&self) -> usize {
        self.tabs.read().await.len()
    }
}

impl Clone for Tab {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            window_id: self.window_id,
            state: Arc::clone(&self.state),
            navigation: Arc::clone(&self.navigation),
        }
    }
}
