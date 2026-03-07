//! Production-grade browser application with WRY WebView integration
//!
//! Uses wry to create a native WebView (Edge WebView2 on Windows, WebKit on macOS/Linux)
//! embedded in the Iced window

use crate::webview_widget::{WebViewWidget, WebViewMessage};
use crate::webview_ipc::WebViewEvent;
use iced::{
    widget::{button, column, container, row, text_input},
    Element, Length, Task, Theme, Subscription,
};
use tracing::{error, info};

/// Main browser application
pub struct IntegratedBrowserApp {
    current_url: String,
    is_loading: bool,
    active_tab_title: Option<String>,
    webview: WebViewWidget,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    UrlChanged(String),
    NavigateSubmitted,
    NavigateTo(String),
    GoBack,
    GoForward,
    Reload,
    StopLoading,
    NewTab,
    CloseTab(uuid::Uuid),
    SwitchTab(uuid::Uuid),
    LoadingStarted,
    LoadingFinished,
    LoadingFailed(String),
    /// WebView-specific messages
    WebView(WebViewMessage),
    /// Poll for WebView updates
    PollWebView,
}

impl Default for IntegratedBrowserApp {
    fn default() -> Self {
        let mut webview = WebViewWidget::new();
        
        // Navigate to start page
        webview.navigate("https://start.duckduckgo.com");
        
        Self {
            current_url: "https://start.duckduckgo.com".to_string(),
            is_loading: true,
            active_tab_title: Some("New Tab".to_string()),
            webview,
        }
    }
}

impl IntegratedBrowserApp {
    pub fn title(&self) -> String {
        match &self.active_tab_title {
            Some(title) if !title.is_empty() => format!("{} - Rusty Browser", title),
            _ => "Rusty Browser".to_string(),
        }
    }
    
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(url) => {
                self.current_url = url;
                Task::none()
            }
            
            Message::NavigateSubmitted => {
                let url = if self.current_url.starts_with("http://") 
                    || self.current_url.starts_with("https://") 
                    || self.current_url.starts_with("about:") {
                    self.current_url.clone()
                } else {
                    format!("https://{}", self.current_url)
                };
                Task::done(Message::NavigateTo(url))
            }
            
            Message::NavigateTo(url) => {
                info!("Navigating to: {}", url);
                // Prevent duplicate navigation to same URL
                if self.current_url != url {
                    self.current_url = url.clone();
                    self.is_loading = true;
                    self.webview.navigate(&url);
                }
                Task::none()
            }
            
            Message::GoBack => {
                info!("Going back");
                Task::none()
            }
            
            Message::GoForward => {
                info!("Going forward");
                Task::none()
            }
            
            Message::Reload => {
                info!("Reloading");
                self.webview.navigate(&self.current_url);
                Task::none()
            }
            
            Message::StopLoading => {
                info!("Stopping load");
                self.is_loading = false;
                Task::none()
            }
            
            Message::NewTab => {
                Task::done(Message::NavigateTo("https://start.duckduckgo.com".to_string()))
            }
            
            Message::CloseTab(_) => {
                Task::none()
            }
            
            Message::SwitchTab(_) => {
                Task::none()
            }
            
            Message::LoadingStarted => {
                self.is_loading = true;
                Task::none()
            }
            
            Message::LoadingFinished => {
                self.is_loading = false;
                // Update URL from webview
                self.current_url = self.webview.current_url().to_string();
                Task::none()
            }
            
            Message::LoadingFailed(error) => {
                error!("Loading failed: {}", error);
                self.is_loading = false;
                Task::none()
            }
            
            Message::WebView(msg) => {
                match msg {
                    WebViewMessage::Events(events) => {
                        for event in events {
                            match event {
                                WebViewEvent::UrlChanged { url } => {
                                    self.current_url = url;
                                }
                                WebViewEvent::TitleChanged { title } => {
                                    self.active_tab_title = Some(title);
                                }
                                WebViewEvent::LoadStarted { .. } => {
                                    self.is_loading = true;
                                }
                                WebViewEvent::LoadFinished { .. } => {
                                    self.is_loading = false;
                                }
                                WebViewEvent::WindowClosed => {
                                    info!("WebView window closed by user");
                                    // Optionally exit the application or open a new tab
                                }
                                _ => {}
                            }
                        }
                        Task::none()
                    }
                    _ => Task::none()
                }
            }
            
            Message::PollWebView => {
                // Poll for WebView events
                let events = self.webview.poll();
                if events.is_empty() {
                    Task::none()
                } else {
                    Task::done(Message::WebView(WebViewMessage::Events(events)))
                }
            }
        }
    }
    
    pub fn view(&self) -> Element<'_, Message> {
        // Navigation toolbar
        let toolbar = row![
            button("←").on_press(Message::GoBack),
            button("→").on_press(Message::GoForward),
            button(if self.is_loading { "✕" } else { "⟳" }).on_press(
                if self.is_loading { Message::StopLoading } else { Message::Reload }
            ),
        ]
        .spacing(4)
        .padding(4);

        // Address bar
        let address_bar = text_input("Enter URL...", &self.current_url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::NavigateSubmitted)
            .padding(8);

        // Tab bar
        let tab_bar = row![
            button("Tab 1"),
            button("+").on_press(Message::NewTab),
        ]
        .spacing(4)
        .padding(4);

        // Content area - the WebView renders as a native window behind this
        // We show a semi-transparent overlay with status
        let content = container(
            column![
                iced::widget::text("Rusty Browser - Hybrid Mode")
                    .size(16),
                iced::widget::text(format!("Current URL: {}", self.current_url))
                    .size(12),
                iced::widget::text(format!("Title: {}", self.active_tab_title.as_deref().unwrap_or("Unknown")))
                    .size(12),
                iced::widget::text(format!("Loading: {}", self.is_loading))
                    .size(12),
            ]
            .spacing(10)
            .align_x(iced::Alignment::Center)
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill);

        // Main layout
        column![toolbar, address_bar, tab_bar, content].into()
    }
    
    pub fn subscription(&self) -> Subscription<Message> {
        // Poll WebView events every 100ms
        iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::PollWebView)
    }
    
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
