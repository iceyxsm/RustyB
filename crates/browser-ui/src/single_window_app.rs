//! Single Window Browser - WebView embedded inside Iced window
//!
//! This creates a true single-window browser where:
//! - The WebView is embedded as a child of the Iced window
//! - Control panel is drawn by Iced on the right side
//! - Both share the same window frame

use iced::{
    widget::{button, column, container, row, text, text_input},
    Element, Length, Task, Theme, Subscription, Alignment,
};
use tracing::info;

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
// WebView integration temporarily disabled for single-window mode
// use crate::embedded_webview::{EmbeddedWebView, EmbeddedWebViewMessage};

/// Single window browser application
pub struct SingleWindowBrowser {
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
    
    // Panel state
    control_panel_width: u16,
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
    
    // Panel control
    ToggleControlPanel,
}

impl Default for SingleWindowBrowser {
    fn default() -> Self {
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
            
            control_panel_width: 400,
        }
    }
}

impl SingleWindowBrowser {
    pub fn title(&self) -> String {
        match &self.active_tab_title {
            Some(title) if !title.is_empty() => format!("{} - Rusty Browser", title),
            _ => "Rusty Browser".to_string(),
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
                    // WebView navigation would go here
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
                // WebView reload would go here
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
            
            Message::ToggleControlPanel => {
                self.sidebar.update(SidebarMessage::TogglePanel);
                Task::none()
            }
        }
    }
    
    pub fn view(&self) -> Element<'_, Message> {
        // === TOP NAVIGATION BAR ===
        let nav_bar = row![
            button("<-").on_press(Message::GoBack),
            button("->").on_press(Message::GoForward),
            button(if self.is_loading { "X" } else { "R" }).on_press(
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

        // === LEFT: WEBVIEW AREA ===
        // This is where the embedded WebView would render
        let webview_container = container(
            column![
                text("WebView Content")
                    .size(16)
                    .color(iced::Color::WHITE),
                text(&self.current_url)
                    .size(12)
                    .color(iced::Color::from_rgb(0.7, 0.7, 0.7)),
                text(format!("Title: {}", self.active_tab_title.as_deref().unwrap_or("Unknown")))
                    .size(11)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                text(format!("Loading: {}", self.is_loading))
                    .size(11)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            ]
            .spacing(8)
            .align_x(Alignment::Center)
        )
        .width(Length::FillPortion(7))
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        // === RIGHT: CONTROL PANEL ===
        let sidebar = self.sidebar.view().map(Message::Sidebar);
        
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

        // === MAIN CONTENT SPLIT ===
        let main_content = row![
            webview_container,
            control_panel,
        ]
        .height(Length::Fill);

        column![
            nav_bar,
            main_content,
        ]
        .into()
    }
    
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
    
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
