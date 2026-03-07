//! WebView widget for Iced that displays WRY-rendered content
//!
//! This widget integrates WRY's WebView output into Iced's UI system.
//! WRY uses the OS native webview (Edge WebView2 on Windows, WebKit on macOS).

use crate::webview_wry::{IcedWebView, WebViewEvent, WebViewController};
use iced::{
    Element, Length, Subscription,
};
use std::sync::Arc;
use tracing::error;

/// Message type for WebView widget events
#[derive(Debug, Clone)]
pub enum WebViewMessage {
    /// Navigate to URL
    Navigate(String),
    /// Navigation completed
    Navigated,
    /// URL changed
    UrlChanged(String),
    /// Title changed  
    TitleChanged(String),
    /// Load started
    LoadStarted,
    /// Load finished
    LoadFinished,
    /// Poll for updates
    Poll,
}

/// WebView widget that displays web content
pub struct WebViewWidget {
    webview: Option<Arc<IcedWebView>>,
    controller: Option<Arc<WebViewController>>,
    current_url: String,
}

impl WebViewWidget {
    /// Create a new WebView widget
    pub fn new() -> Self {
        // Initialize WebView
        match IcedWebView::new() {
            Ok(wv) => {
                let controller = wv.controller();
                Self {
                    webview: Some(Arc::new(wv)),
                    controller: Some(controller),
                    current_url: String::new(),
                }
            }
            Err(e) => {
                error!("Failed to create WebView: {}", e);
                Self {
                    webview: None,
                    controller: None,
                    current_url: String::new(),
                }
            }
        }
    }
    
    /// Navigate to a URL
    pub fn navigate(&mut self, url: &str) {
        if let Some(controller) = &self.controller {
            if let Err(e) = controller.navigate(url) {
                error!("Failed to navigate: {}", e);
            }
        }
        self.current_url = url.to_string();
    }
    
    /// Get current URL
    pub fn current_url(&self) -> &str {
        &self.current_url
    }
    
    /// Get current title
    pub fn current_title(&self) -> String {
        self.controller.as_ref()
            .map(|c| c.current_title())
            .unwrap_or_default()
    }
    
    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.controller.as_ref()
            .map(|c| c.is_loading())
            .unwrap_or(false)
    }
    
    /// Poll for events and convert to messages
    pub fn poll(&self) -> Vec<WebViewMessage> {
        let mut messages = Vec::new();
        
        if let Some(webview) = &self.webview {
            for event in webview.poll_events() {
                match event {
                    WebViewEvent::LoadStarted => messages.push(WebViewMessage::LoadStarted),
                    WebViewEvent::LoadFinished => messages.push(WebViewMessage::LoadFinished),
                    WebViewEvent::UrlChanged(url) => messages.push(WebViewMessage::UrlChanged(url)),
                    WebViewEvent::TitleChanged(title) => messages.push(WebViewMessage::TitleChanged(title)),
                    _ => {}
                }
            }
        }
        
        messages
    }
    
    /// Create a subscription for polling WebView events
    pub fn subscription(&self) -> Subscription<WebViewMessage> {
        iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| WebViewMessage::Poll)
    }
}

impl Default for WebViewWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the WebView widget element
/// 
/// Note: WRY creates its own native window for the webview.
/// This returns a placeholder container since the actual webview
/// is rendered by the OS in a separate window.
pub fn webview<Message: 'static>(
    _webview: &WebViewWidget,
) -> Element<Message> {
    // The actual WebView is rendered by WRY in its own native window
    // We return a placeholder that takes up space in the Iced layout
    iced::widget::container(
        iced::widget::column![
            iced::widget::text("WebView Active")
                .size(20)
                .style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.palette().primary),
                }),
            iced::widget::text("Rendering in native window")
                .size(12),
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// Convert WebViewMessage to your app's Message type
pub fn map_message<M, F>(msg: WebViewMessage, f: F) -> M 
where 
    F: Fn(WebViewMessage) -> M,
{
    f(msg)
}
