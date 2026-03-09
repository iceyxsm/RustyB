//! Embedded WebView for Iced - Stub implementation
//!
//! The actual WebView is handled via IPC in webview_ipc.rs
//! This module provides placeholder types for potential future embedded use.

use iced::{widget::container, Element, Length, Rectangle};
use std::sync::Arc;
use tracing::info;

/// Message type for embedded WebView
#[derive(Debug, Clone)]
pub enum EmbeddedWebViewMessage {
    Navigate(String),
    UrlChanged(String),
    TitleChanged(String),
    LoadStarted,
    LoadFinished,
}

/// WebView controller stub
pub struct WebViewController {
    current_url: String,
    current_title: String,
    is_loading: bool,
}

impl WebViewController {
    pub fn new() -> Self {
        Self {
            current_url: "about:blank".to_string(),
            current_title: String::new(),
            is_loading: false,
        }
    }

    pub fn navigate(&self, url: &str) {
        info!("Navigate requested to: {}", url);
    }

    pub fn current_url(&self) -> &str {
        &self.current_url
    }

    pub fn current_title(&self) -> &str {
        &self.current_title
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn set_bounds(&self, _bounds: Rectangle) {
        // Stub
    }
}

impl Default for WebViewController {
    fn default() -> Self {
        Self::new()
    }
}

/// Embedded WebView widget state
pub struct EmbeddedWebView {
    controller: Arc<std::sync::Mutex<WebViewController>>,
    bounds: Rectangle,
}

impl EmbeddedWebView {
    pub fn new() -> Self {
        Self {
            controller: Arc::new(std::sync::Mutex::new(WebViewController::new())),
            bounds: Rectangle::default(),
        }
    }

    pub fn controller(&self) -> Arc<std::sync::Mutex<WebViewController>> {
        self.controller.clone()
    }

    pub fn navigate(&self, url: &str) {
        if let Ok(controller) = self.controller.lock() {
            controller.navigate(url);
        }
    }

    pub fn current_url(&self) -> String {
        self.controller
            .lock()
            .map(|c| c.current_url().to_string())
            .unwrap_or_default()
    }

    pub fn current_title(&self) -> String {
        self.controller
            .lock()
            .map(|c| c.current_title().to_string())
            .unwrap_or_default()
    }

    pub fn is_loading(&self) -> bool {
        self.controller
            .lock()
            .map(|c| c.is_loading())
            .unwrap_or(false)
    }

    pub fn update_bounds(&mut self, bounds: Rectangle) {
        self.bounds = bounds;
        if let Ok(controller) = self.controller.lock() {
            controller.set_bounds(bounds);
        }
    }
}

impl Default for EmbeddedWebView {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the WebView widget element (placeholder)
pub fn embedded_webview<'a, Message: 'a>(
    _webview: &'a EmbeddedWebView,
) -> Element<'a, Message> {
    container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
