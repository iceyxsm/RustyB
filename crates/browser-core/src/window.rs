//! Browser window management

use crate::tab::{Tab, TabManager};
use shared::WindowId;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Represents a browser window
#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub title: Arc<RwLock<String>>,
    pub position: Arc<RwLock<WindowPosition>>,
    pub size: Arc<RwLock<WindowSize>>,
    pub state: Arc<RwLock<WindowState>>,
    pub tab_manager: TabManager,
    pub is_private: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

impl Window {
    pub fn new(is_private: bool) -> Self {
        let id = WindowId::new();
        debug!("Creating new window: {:?}", id);

        Self {
            id,
            title: Arc::new(RwLock::new("Rusty Browser".to_string())),
            position: Arc::new(RwLock::new(WindowPosition { x: 100, y: 100 })),
            size: Arc::new(RwLock::new(WindowSize {
                width: 1280,
                height: 720,
            })),
            state: Arc::new(RwLock::new(WindowState::Normal)),
            tab_manager: TabManager::new(),
            is_private,
        }
    }

    pub async fn create_tab(&self) -> Tab {
        let id = self.tab_manager.create_tab(Some(self.id.0)).await;
        self.tab_manager.get_tab(id).await.unwrap()
    }

    pub async fn update_title(&self) {
        let active_tab = self.tab_manager.get_active_tab().await;
        let tab_title = active_tab
            .as_ref()
            .and_then(|t| t.state.read().await.title.clone())
            .unwrap_or_else(|| "New Tab".to_string());

        let mut title = self.title.write().await;
        *title = format!("{} - Rusty Browser", tab_title);
    }

    pub async fn set_position(&self, x: i32, y: i32) {
        let mut pos = self.position.write().await;
        pos.x = x;
        pos.y = y;
    }

    pub async fn set_size(&self, width: u32, height: u32) {
        let mut size = self.size.write().await;
        size.width = width;
        size.height = height;
    }

    pub async fn set_state(&self, state: WindowState) {
        let mut s = self.state.write().await;
        *s = state;
    }

    pub async fn get_position(&self) -> WindowPosition {
        *self.position.read().await
    }

    pub async fn get_size(&self) -> WindowSize {
        *self.size.read().await
    }

    pub async fn get_state(&self) -> WindowState {
        *self.state.read().await
    }
}

/// Manages all windows
#[derive(Debug)]
pub struct WindowManager {
    windows: Arc<RwLock<Vec<Window>>>,
    active_window: Arc<RwLock<Option<WindowId>>>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Arc::new(RwLock::new(Vec::new())),
            active_window: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn create_window(&self, is_private: bool) -> Window {
        let window = Window::new(is_private);
        let id = window.id;

        // Create initial tab
        let _ = window.create_tab().await;

        let mut windows = self.windows.write().await;
        windows.push(window);

        // Set as active
        let mut active = self.active_window.write().await;
        *active = Some(id);

        self.get_window(id).await.unwrap()
    }

    pub async fn close_window(&self, id: WindowId) -> Option<WindowId> {
        let mut windows = self.windows.write().await;

        if let Some(index) = windows.iter().position(|w| w.id == id) {
            windows.remove(index);

            // Update active window
            let mut active = self.active_window.write().await;
            if *active == Some(id) {
                *active = windows.get(index.saturating_sub(1)).map(|w| w.id);
            }

            *active
        } else {
            None
        }
    }

    pub async fn get_window(&self, id: WindowId) -> Option<Window> {
        let windows = self.windows.read().await;
        windows.iter().find(|w| w.id == id).cloned()
    }

    pub async fn get_active_window(&self) -> Option<Window> {
        let active = self.active_window.read().await;
        if let Some(id) = *active {
            self.get_window(id).await
        } else {
            None
        }
    }

    pub async fn set_active_window(&self, id: WindowId) -> bool {
        let windows = self.windows.read().await;
        if windows.iter().any(|w| w.id == id) {
            let mut active = self.active_window.write().await;
            *active = Some(id);
            true
        } else {
            false
        }
    }

    pub async fn get_all_windows(&self) -> Vec<Window> {
        self.windows.read().await.clone()
    }

    pub async fn get_window_count(&self) -> usize {
        self.windows.read().await.len()
    }
}

impl Clone for Window {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            title: Arc::clone(&self.title),
            position: Arc::clone(&self.position),
            size: Arc::clone(&self.size),
            state: Arc::clone(&self.state),
            tab_manager: TabManager::new(), // Each clone gets its own tab manager
            is_private: self.is_private,
        }
    }
}
