//! WebView widget for Iced using IPC-based architecture

use crate::webview_ipc::{IcedWebView, WebViewEvent};
use iced::Element;
use std::sync::Arc;
use tracing::{error, info};

/// Message type for WebView widget events
#[derive(Debug, Clone)]
pub enum WebViewMessage {
    /// Navigate to URL
    Navigate(String),
    /// URL changed
    UrlChanged(String),
    /// Title changed
    TitleChanged(String),
    /// Load started
    LoadStarted,
    /// Load finished
    LoadFinished,
    /// Batch of events from WebView
    Events(Vec<WebViewEvent>),
}

/// WebView widget that displays web content via IPC
pub struct WebViewWidget {
    webview: Arc<IcedWebView>,
    current_url: String,
    webview_url: String,
}

impl WebViewWidget {
    /// Create a new WebView widget
    pub fn new() -> Self {
        match IcedWebView::new() {
            Ok(wv) => {
                info!("WebView widget created successfully");
                Self {
                    webview: Arc::new(wv),
                    current_url: "about:blank".to_string(),
                    webview_url: String::new(),
                }
            }
            Err(e) => {
                error!("Failed to create WebView: {}", e);
                // This shouldn't happen as IcedWebView::new returns Ok in fallback mode
                panic!("WebView initialization failed: {}", e);
            }
        }
    }
    
    /// Navigate to a URL
    pub fn navigate(&mut self, url: &str) {
        info!("Navigating to: {}", url);
        self.webview.navigate(url);
        self.current_url = url.to_string();
        self.webview_url = url.to_string();
    }
    
    /// Get current URL
    pub fn current_url(&self) -> &str {
        // Prefer the webview's reported URL
        let wv_url = self.webview.current_url();
        if !wv_url.is_empty() && wv_url != "about:blank" {
            // We can't return reference to wv_url, so we rely on the stored URL
            // In practice, poll() updates current_url from webview events
            &self.current_url
        } else {
            &self.current_url
        }
    }
    
    /// Get current title
    pub fn current_title(&self) -> String {
        self.webview.current_title()
    }
    
    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.webview.is_loading()
    }

    /// Poll for events from the WebView
    pub fn poll(&self) -> Vec<WebViewEvent> {
        self.webview.poll()
    }
}

impl Default for WebViewWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the WebView widget element
/// Note: The actual WebView renders in a separate window/process
/// This returns a status overlay
pub fn webview<Message: 'static>(
    widget: &WebViewWidget,
) -> Element<Message> {
    // The WebView is rendered in a separate window via IPC
    // We show a status overlay
    iced::widget::container(
        iced::widget::column![
            iced::widget::text("Rusty Browser - Hybrid Mode (IPC)")
                .size(16),
            iced::widget::text(format!("URL: {}", widget.current_url()))
                .size(12),
            iced::widget::text(format!("Loading: {}", widget.is_loading()))
                .size(10),
        ]
        .spacing(4)
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .into()
}
