//! Main browser application using Iced 0.14

use browser_core::session::BrowserSession;
use iced::{
    widget::{button, column, container, row, text, text_input},
    Center, Element, Length, Task, Theme,
};
use shared::BrowserConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Main browser application state
#[derive(Debug)]
pub struct BrowserApp {
    session: Arc<RwLock<BrowserSession>>,
    current_url: String,
    is_loading: bool,
    active_tab_title: Option<String>,
}

/// Messages for the application
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    UrlChanged(String),
    NavigateSubmitted,
    NavigateTo(String),
    GoBack,
    GoForward,
    Reload,
    StopLoading,
    
    // Tabs
    NewTab,
    CloseTab(uuid::Uuid),
    SwitchTab(uuid::Uuid),
    
    // Loading state
    LoadingStarted,
    LoadingFinished,
    LoadingFailed(String),
}

impl Default for BrowserApp {
    fn default() -> Self {
        let config = BrowserConfig::default();
        let session = Arc::new(RwLock::new(BrowserSession::new(config)));
        
        Self {
            session,
            current_url: String::new(),
            is_loading: false,
            active_tab_title: None,
        }
    }
}

impl BrowserApp {
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
                    || self.current_url.starts_with("https://") {
                    self.current_url.clone()
                } else {
                    format!("https://{}", self.current_url)
                };
                Task::done(Message::NavigateTo(url))
            }
            
            Message::NavigateTo(url) => {
                debug!("Navigating to: {}", url);
                self.current_url = url.clone();
                self.is_loading = true;
                
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            if let Some(tab) = window.tab_manager.get_active_tab().await {
                                let _ = tab.navigate(&url).await;
                            }
                        }
                    },
                    |_| Message::LoadingStarted,
                )
            }
            
            Message::GoBack => {
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            if let Some(tab) = window.tab_manager.get_active_tab().await {
                                tab.go_back().await
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    },
                    |url| {
                        if let Some(u) = url {
                            Message::NavigateTo(u)
                        } else {
                            Message::LoadingFinished
                        }
                    },
                )
            }
            
            Message::GoForward => {
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            if let Some(tab) = window.tab_manager.get_active_tab().await {
                                tab.go_forward().await
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    },
                    |url| {
                        if let Some(u) = url {
                            Message::NavigateTo(u)
                        } else {
                            Message::LoadingFinished
                        }
                    },
                )
            }
            
            Message::Reload => {
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            if let Some(tab) = window.tab_manager.get_active_tab().await {
                                tab.reload().await
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    },
                    |url| {
                        if let Some(u) = url {
                            Message::NavigateTo(u)
                        } else {
                            Message::LoadingFinished
                        }
                    },
                )
            }
            
            Message::StopLoading => {
                self.is_loading = false;
                Task::none()
            }
            
            Message::NewTab => {
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            let tab = window.create_tab().await;
                            tab.id.0
                        } else {
                            uuid::Uuid::nil()
                        }
                    },
                    |tab_id| {
                        if tab_id != uuid::Uuid::nil() {
                            Message::NavigateTo("https://start.duckduckgo.com".to_string())
                        } else {
                            Message::LoadingFinished
                        }
                    },
                )
            }
            
            Message::CloseTab(tab_id) => {
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            window.tab_manager.close_tab(shared::TabId(tab_id)).await
                        } else {
                            None
                        }
                    },
                    |_| Message::LoadingFinished,
                )
            }
            
            Message::SwitchTab(tab_id) => {
                let session = Arc::clone(&self.session);
                Task::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            window.tab_manager.set_active_tab(shared::TabId(tab_id)).await;
                            if let Some(tab) = window.tab_manager.get_active_tab().await {
                                let state = tab.get_state().await;
                                (state.url, state.title)
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        }
                    },
                    |(url, title)| {
                        if let Some(u) = url {
                            Message::UrlChanged(u)
                        } else {
                            Message::LoadingFinished
                        }
                    },
                )
            }
            
            Message::LoadingStarted => {
                self.is_loading = true;
                Task::none()
            }
            
            Message::LoadingFinished => {
                self.is_loading = false;
                Task::none()
            }
            
            Message::LoadingFailed(error) => {
                debug!("Loading failed: {}", error);
                self.is_loading = false;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Toolbar with navigation buttons
        let toolbar = row![
            button("←").on_press(Message::GoBack),
            button("→").on_press(Message::GoForward),
            button(if self.is_loading { "✕" } else { "⟳" }).on_press(Message::Reload),
        ]
        .spacing(8)
        .padding(8);

        // Address bar
        let address_bar = text_input("Enter URL or search...", &self.current_url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::NavigateSubmitted)
            .padding(10);

        // Tab bar
        let tab_bar = row![
            button("Tab 1"),
            button("+").on_press(Message::NewTab),
        ]
        .spacing(8)
        .padding(8);

        // Content area (placeholder for now)
        let content = container(
            text("Content Area - Servo WebView will be embedded here").size(16)
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        // Main layout
        column![toolbar, address_bar, tab_bar, content].into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
