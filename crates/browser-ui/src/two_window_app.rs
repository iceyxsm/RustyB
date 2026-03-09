//! Two-Window Browser
//!
//! Window 1: Main browser with WebView
//! Window 2: Tools/Control Panel (opens via button)

use iced::{
    widget::{button, column, container, row, text, text_input},
    Element, Length, Task, Theme, Subscription, Alignment, window,
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

/// Window IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowIds {
    pub main: window::Id,
    pub tools: Option<window::Id>,
}

/// Two-window browser application
pub struct TwoWindowBrowser {
    // Window management
    windows: WindowIds,
    
    // Control panel (shown in tools window)
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
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Window management
    OpenToolsWindow,
    CloseToolsWindow,
    ToolsWindowOpened(window::Id),
    ToolsWindowClosed(window::Id),
    
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
}

impl TwoWindowBrowser {
    pub fn new(main_window: window::Id) -> Self {
        Self {
            windows: WindowIds {
                main: main_window,
                tools: None,
            },
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
        }
    }
    
    pub fn title(&self, window_id: window::Id) -> String {
        if window_id == self.windows.main {
            match &self.active_tab_title {
                Some(title) if !title.is_empty() => format!("{} - Rusty Browser", title),
                _ => "Rusty Browser".to_string(),
            }
        } else if Some(window_id) == self.windows.tools {
            "Tools - Rusty Browser".to_string()
        } else {
            "Rusty Browser".to_string()
        }
    }
    
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Window management
            Message::OpenToolsWindow => {
                if self.windows.tools.is_none() {
                    info!("Opening tools window...");
                    let tools_window = window::open(window::Settings {
                        size: iced::Size::new(500.0, 700.0),
                        position: window::Position::Default,
                        min_size: Some(iced::Size::new(350.0, 400.0)),
                        max_size: None,
                        visible: true,
                        resizable: true,
                        decorations: true,
                        transparent: false,
                        level: window::Level::Normal,
                        icon: None,
                        exit_on_close_request: false,
                        ..Default::default()
                    });
                    
                    let (id, _) = tools_window;
                    return Task::done(Message::ToolsWindowOpened(id));
                }
                Task::none()
            }
            
            Message::CloseToolsWindow => {
                if let Some(tools_id) = self.windows.tools.take() {
                    return window::close(tools_id);
                }
                Task::none()
            }
            
            Message::ToolsWindowOpened(id) => {
                info!("Tools window opened: {:?}", id);
                self.windows.tools = Some(id);
                Task::none()
            }
            
            Message::ToolsWindowClosed(id) => {
                info!("Tools window closed: {:?}", id);
                if self.windows.tools == Some(id) {
                    self.windows.tools = None;
                }
                Task::none()
            }
            
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
        }
    }
    
    pub fn view(&self, window_id: window::Id) -> Element<Message> {
        if window_id == self.windows.main {
            self.view_main_window()
        } else if Some(window_id) == self.windows.tools {
            self.view_tools_window()
        } else {
            container(text("Unknown window")).into()
        }
    }
    
    /// Main browser window view
    fn view_main_window(&self) -> Element<Message> {
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
            // Button to open tools window
            button("🔧 Tools").on_press(Message::OpenToolsWindow),
        ]
        .spacing(8)
        .padding(8)
        .align_y(Alignment::Center);

        // === MAIN: WEBVIEW AREA (full window) ===
        let webview_container = container(
            column![
                text("WebView Content")
                    .size(24)
                    .color(iced::Color::WHITE),
                text(&self.current_url)
                    .size(14)
                    .color(iced::Color::from_rgb(0.7, 0.7, 0.7)),
                text(format!("Title: {}", self.active_tab_title.as_deref().unwrap_or("Unknown")))
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                text(format!("Loading: {}", self.is_loading))
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                text("Click 'Tools' button to open control panel")
                    .size(11)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        column![
            nav_bar,
            webview_container,
        ]
        .into()
    }
    
    /// Tools/Control Panel window view
    fn view_tools_window(&self) -> Element<Message> {
        let header = row![
            text("Control Panel")
                .size(18),
            container(iced::widget::Space::new().width(Length::Fill)),
            button("✕ Close").on_press(Message::CloseToolsWindow),
        ]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center);

        let sidebar = self.sidebar.view().map(Message::Sidebar);
        
        let tool_panel: Element<Message> = match self.sidebar.selected() {
            ToolCategory::Network => self.network_panel.view().map(Message::NetworkPanel),
            ToolCategory::AiEngine => self.ai_panel.view().map(Message::AiPanel),
            ToolCategory::Automation => self.automation_panel.view().map(Message::AutomationPanel),
            ToolCategory::Extraction => self.extraction_panel.view().map(Message::ExtractionPanel),
            ToolCategory::DevTools => self.devtools_panel.view().map(Message::DevToolsPanel),
            ToolCategory::Settings => self.settings_panel.view().map(Message::SettingsPanel),
        };

        let content = row![
            sidebar,
            container(tool_panel)
                .width(Length::Fill)
                .padding(10),
        ]
        .height(Length::Fill);

        column![
            header,
            content,
        ]
        .into()
    }
    
    pub fn subscription(&self) -> Subscription<Message> {
        // Subscribe to window close events
        window::close_events().map(|id| Message::ToolsWindowClosed(id))
    }
    
    pub fn theme(&self, _window_id: window::Id) -> Theme {
        Theme::Dark
    }
}
