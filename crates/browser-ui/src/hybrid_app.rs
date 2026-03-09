//! Rusty Browser Hybrid Mode - Control Panel + WebView
//!
//! This is the main application that combines:
//! - Left: Control Panel with all tools (Network, AI, Automation, etc.)
//! - Right: WebView for browsing
//!
//! The control panel can be collapsed to give more space to the webview.

use iced::{
    widget::{button, column, container, row, text, text_input, Space},
    Element, Length, Task, Theme, Subscription, Alignment,
};
use tracing::info;

use crate::window_manager::init_window_manager;
use crate::control_panel::{
    sidebar::{Sidebar, SidebarMessage},
    panels::{
        network_panel::{NetworkPanel, NetworkMessage},
        ai_panel::{AiPanel, AiMessage},
        automation_panel::{AutomationPanel, AutomationMessage},
        extraction_panel::{ExtractionPanel, ExtractionMessage},
        devtools_panel::{DevToolsPanel, DevToolsMessage},
        settings_panel::{SettingsPanel, SettingsMessage},
    },
    ToolCategory,
};
use crate::webview_widget::{WebViewWidget, WebViewMessage};
use crate::webview_ipc::WebViewEvent;

/// Main hybrid application
pub struct HybridBrowserApp {
    // Control panel
    sidebar: Sidebar,
    network_panel: NetworkPanel,
    ai_panel: AiPanel,
    automation_panel: AutomationPanel,
    extraction_panel: ExtractionPanel,
    devtools_panel: DevToolsPanel,
    settings_panel: SettingsPanel,
    
    // Browser state
    current_url: String,
    is_loading: bool,
    active_tab_title: Option<String>,
    
    // WebView
    webview: WebViewWidget,
    
    // Panel state
    control_panel_width: u16,
    
    // Window positioning
    windows_positioned: bool,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Control panel
    Sidebar(SidebarMessage),
    NetworkPanel(NetworkMessage),
    AiPanel(AiMessage),
    AutomationPanel(AutomationMessage),
    ExtractionPanel(ExtractionMessage),
    DevToolsPanel(DevToolsMessage),
    SettingsPanel(SettingsMessage),
    
    // Browser navigation
    UrlChanged(String),
    NavigateSubmitted,
    NavigateTo(String),
    GoBack,
    GoForward,
    Reload,
    StopLoading,
    NewTab,
    
    // WebView
    WebView(WebViewMessage),
    PollWebView,
    
    // Panel control
    ToggleControlPanel,
    
    // Window positioning
    PositionWindows,
}

impl Default for HybridBrowserApp {
    fn default() -> Self {
        // Initialize window manager for split view
        init_window_manager();
        
        let mut webview = WebViewWidget::new();
        webview.navigate("https://start.duckduckgo.com");
        
        Self {
            sidebar: Sidebar::new(true, ToolCategory::Network),
            network_panel: NetworkPanel::new(),
            ai_panel: AiPanel::new(),
            automation_panel: AutomationPanel::new(),
            extraction_panel: ExtractionPanel::new(),
            devtools_panel: DevToolsPanel::new(),
            settings_panel: SettingsPanel::new(),
            
            current_url: "https://start.duckduckgo.com".to_string(),
            is_loading: true,
            active_tab_title: Some("New Tab".to_string()),
            
            webview,
            control_panel_width: 500,
            
            windows_positioned: false,
        }
    }
}

impl HybridBrowserApp {
    pub fn title(&self) -> String {
        match &self.active_tab_title {
            Some(title) if !title.is_empty() => format!("{} - Rusty Browser Hybrid", title),
            _ => "Rusty Browser Hybrid".to_string(),
        }
    }
    
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Control panel messages
            Message::Sidebar(msg) => {
                match msg {
                    SidebarMessage::TogglePanel => {
                        self.sidebar.update(msg);
                    }
                    SidebarMessage::CategorySelected(category) => {
                        self.sidebar.update(SidebarMessage::CategorySelected(category));
                    }
                }
                Task::none()
            }
            
            Message::NetworkPanel(msg) => {
                self.network_panel.update(msg);
                Task::none()
            }
            
            Message::AiPanel(msg) => {
                self.ai_panel.update(msg);
                Task::none()
            }
            
            Message::AutomationPanel(msg) => {
                self.automation_panel.update(msg);
                Task::none()
            }
            
            Message::ExtractionPanel(msg) => {
                self.extraction_panel.update(msg);
                Task::none()
            }
            
            Message::DevToolsPanel(msg) => {
                self.devtools_panel.update(msg);
                Task::none()
            }
            
            Message::SettingsPanel(msg) => {
                self.settings_panel.update(msg);
                Task::none()
            }
            
            // Browser navigation
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
            
            // WebView messages
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
                                _ => {}
                            }
                        }
                        Task::none()
                    }
                    _ => Task::none()
                }
            }
            
            Message::PollWebView => {
                let events = self.webview.poll();
                if events.is_empty() {
                    Task::none()
                } else {
                    Task::done(Message::WebView(WebViewMessage::Events(events)))
                }
            }
            
            Message::ToggleControlPanel => {
                self.sidebar.update(SidebarMessage::TogglePanel);
                Task::none()
            }
            
            Message::PositionWindows => {
                // Only position windows once
                if !self.windows_positioned {
                    // Position webview side-by-side with control panel
                    // Control panel at left (x=100), webview at right (x=650)
                    self.webview.controller().set_bounds(
                        650,   // x: to the right of control panel
                        100,   // y: top position
                        1100,  // width
                        800    // height
                    );
                    
                    info!("WebView positioned at x=650 (control panel is at x=100, width=550)");
                    info!("=== BOTH WINDOWS SHOULD BE VISIBLE ===");
                    info!("Look for: 'New Tab - Rusty Browser Hybrid' and 'Rusty Browser - WebView'");
                    self.windows_positioned = true;
                }
                Task::none()
            }
        }
    }
    
    pub fn view(&self) -> Element<'_, Message> {
        // === TOP NAVIGATION BAR ===
        let nav_bar = row![
            button("←").on_press(Message::GoBack),
            button("→").on_press(Message::GoForward),
            button(if self.is_loading { "✕" } else { "⟳" }).on_press(
                if self.is_loading { Message::StopLoading } else { Message::Reload }
            ),
            text_input("Enter URL...", &self.current_url)
                .on_input(Message::UrlChanged)
                .on_submit(Message::NavigateSubmitted)
                .padding(8)
                .width(Length::FillPortion(4)),
            button("+").on_press(Message::NewTab),
        ]
        .spacing(8)
        .padding(8)
        .align_y(Alignment::Center);

        // === LEFT: CONTROL PANEL ===
        let sidebar = self.sidebar.view().map(Message::Sidebar);
        
        // Tool panel based on selected category
        let tool_panel: Element<Message> = match self.sidebar.selected() {
            ToolCategory::Network => self.network_panel.view().map(Message::NetworkPanel),
            ToolCategory::AiEngine => self.ai_panel.view().map(Message::AiPanel),
            ToolCategory::Automation => self.automation_panel.view().map(Message::AutomationPanel),
            ToolCategory::Extraction => self.extraction_panel.view().map(Message::ExtractionPanel),
            ToolCategory::DevTools => self.devtools_panel.view().map(Message::DevToolsPanel),
            ToolCategory::Settings => self.settings_panel.view().map(Message::SettingsPanel),
        };

        let control_panel = row![sidebar, tool_panel]
            .width(Length::Fixed(if self.sidebar.is_expanded() { 
                self.control_panel_width as f32 
            } else { 
                50.0 
            }));

        // === RIGHT: WEBVIEW CONTENT ===
        // The webview renders as a native window behind the UI
        // We show a status overlay
        let webview_overlay = container(
            column![
                text("Rusty Browser - Hybrid Mode")
                    .size(20)
                    .color(iced::Color::WHITE),
                text(format!("Current URL: {}", self.current_url))
                    .size(12)
                    .color(iced::Color::from_rgb(0.8, 0.8, 0.8)),
                text(format!("Title: {}", self.active_tab_title.as_deref().unwrap_or("Unknown")))
                    .size(12)
                    .color(iced::Color::from_rgb(0.8, 0.8, 0.8)),
                text(format!("Loading: {}", self.is_loading))
                    .size(12)
                    .color(iced::Color::from_rgb(0.8, 0.8, 0.8)),
                Space::new().height(20),
                button("Toggle Control Panel")
                    .on_press(Message::ToggleControlPanel),
            ]
            .spacing(10)
            .align_x(Alignment::Center)
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill);

        // === MAIN LAYOUT ===
        // Split view: Control Panel | WebView
        let main_content = row![
            control_panel,
            webview_overlay,
        ]
        .height(Length::Fill);

        column![
            nav_bar,
            main_content,
        ]
        .into()
    }
    
    pub fn subscription(&self) -> Subscription<Message> {
        // Poll WebView events every 100ms
        let webview_poll = iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::PollWebView);
        
        // Position windows after startup (after 500ms delay to let windows create)
        let position_windows = iced::time::every(std::time::Duration::from_millis(500))
            .map(|_| Message::PositionWindows);
        
        Subscription::batch(vec![webview_poll, position_windows])
    }
    
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
