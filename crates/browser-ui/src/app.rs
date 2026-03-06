//! Main browser application using Iced

use crate::views::{address_bar::AddressBar, tab_bar::TabBar, toolbar::Toolbar};
use browser_core::session::BrowserSession;
use iced::{
    widget::{column, container, row, text},
    Application, Command, Element, Length, Theme,
};
use shared::BrowserConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Main browser application state
pub struct BrowserApp {
    session: Arc<RwLock<BrowserSession>>,
    current_url: String,
    is_loading: bool,
    can_go_back: bool,
    can_go_forward: bool,
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
    TabTitleChanged(uuid::Uuid, String),
    
    // Window
    WindowResized(u32, u32),
    
    // Loading state
    LoadingStarted,
    LoadingFinished,
    LoadingFailed(String),
}

impl Application for BrowserApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        info!("Initializing Rusty Browser");
        
        let config = BrowserConfig::default();
        let session = Arc::new(RwLock::new(BrowserSession::new(config)));
        
        let app = Self {
            session,
            current_url: String::new(),
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
            active_tab_title: None,
        };
        
        // Start the session
        let session_clone = Arc::clone(&app.session);
        let init_cmd = Command::perform(
            async move {
                let session = session_clone.read().await;
                let _ = session.start().await;
            },
            |_| Message::NavigateTo("https://start.duckduckgo.com".to_string()),
        );
        
        (app, init_cmd)
    }

    fn title(&self) -> String {
        match &self.active_tab_title {
            Some(title) if !title.is_empty() => format!("{} - Rusty Browser", title),
            _ => "Rusty Browser".to_string(),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::UrlChanged(url) => {
                self.current_url = url;
                Command::none()
            }
            
            Message::NavigateSubmitted => {
                let url = if self.current_url.starts_with("http://") 
                    || self.current_url.starts_with("https://") {
                    self.current_url.clone()
                } else {
                    format!("https://{}", self.current_url)
                };
                
                Command::perform(async move { url }, Message::NavigateTo)
            }
            
            Message::NavigateTo(url) => {
                debug!("Navigating to: {}", url);
                self.current_url = url.clone();
                self.is_loading = true;
                
                let session = Arc::clone(&self.session);
                Command::perform(
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
                Command::perform(
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
                Command::perform(
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
                Command::perform(
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
                Command::none()
            }
            
            Message::NewTab => {
                let session = Arc::clone(&self.session);
                Command::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            let tab = window.create_tab().await;
                            tab.id
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
                Command::perform(
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
                Command::perform(
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
            
            Message::TabTitleChanged(tab_id, title) => {
                if self.active_tab_title.as_ref() != Some(&title) {
                    self.active_tab_title = Some(title);
                }
                Command::none()
            }
            
            Message::WindowResized(width, height) => {
                let session = Arc::clone(&self.session);
                Command::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            window.set_size(width, height).await;
                        }
                    },
                    |_| Message::LoadingFinished,
                )
            }
            
            Message::LoadingStarted => {
                self.is_loading = true;
                Command::none()
            }
            
            Message::LoadingFinished => {
                self.is_loading = false;
                
                // Update navigation state
                let session = Arc::clone(&self.session);
                Command::perform(
                    async move {
                        let session = session.read().await;
                        if let Some(window) = session.window_manager.get_active_window().await {
                            if let Some(tab) = window.tab_manager.get_active_tab().await {
                                let nav_state = tab.get_navigation_state().await;
                                (nav_state.can_go_back, nav_state.can_go_forward)
                            } else {
                                (false, false)
                            }
                        } else {
                            (false, false)
                        }
                    },
                    |(back, forward)| {
                        // We need to update these in the state
                        // For now, just return a no-op
                        Message::LoadingFinished
                    },
                )
            }
            
            Message::LoadingFailed(error) => {
                debug!("Loading failed: {}", error);
                self.is_loading = false;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        // Toolbar with navigation buttons
        let toolbar = Toolbar::new()
            .on_back(Message::GoBack)
            .on_forward(Message::GoForward)
            .on_reload(Message::Reload)
            .on_stop(Message::StopLoading)
            .can_go_back(self.can_go_back)
            .can_go_forward(self.can_go_forward)
            .is_loading(self.is_loading);

        // Address bar
        let address_bar = AddressBar::new(&self.current_url)
            .on_change(Message::UrlChanged)
            .on_submit(Message::NavigateSubmitted);

        // Tab bar
        let tab_bar = TabBar::new()
            .on_new_tab(Message::NewTab)
            .on_close_tab(Message::CloseTab)
            .on_switch_tab(Message::SwitchTab);

        // Content area (placeholder for now)
        let content = container(
            text("Content Area - Servo WebView will be embedded here")
                .size(16)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y();

        // Main layout
        column![
            toolbar.view(),
            address_bar.view(),
            tab_bar.view(),
            content,
        ]
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}
