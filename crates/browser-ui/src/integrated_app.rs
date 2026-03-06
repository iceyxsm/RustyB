//! High-performance browser application - optimized for speed
//!
//! Performance optimizations:
//! - No animations, no transitions
//! - Minimal allocations
//! - Direct texture presentation
//! - No frame pacing - VSync handles timing

use crate::{
    servo_renderer::{FpsCounter, ServoRenderer},
    webview_widget::{to_servo_input_event, WebViewMessage},
};
use browser_core::webview::{ServoManager, WebViewUpdate};
use iced::{
    widget::{button, column, container, row, text_input},
    Element, Length, Task, Theme, Subscription,
};
use std::sync::Arc;
use tracing::{error, info};

/// Main browser application - minimal state
pub struct IntegratedBrowserApp {
    current_url: String,
    is_loading: bool,
    active_tab_title: Option<String>,
    servo_renderer: Arc<ServoRenderer>,
    servo_manager: Option<ServoManager>,
    fps_counter: FpsCounter,
}

/// Messages - minimal enum for fast dispatch
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
    ServoTick,
    WebViewInput(WebViewMessage),
}

impl Default for IntegratedBrowserApp {
    fn default() -> Self {
        let servo_renderer = Arc::new(ServoRenderer::new());
        
        let (servo_manager, _, _, _) = 
            ServoManager::new((800, 600), 1.0).expect("Failed to create Servo manager");
        
        Self {
            current_url: String::new(),
            is_loading: false,
            active_tab_title: None,
            servo_renderer,
            servo_manager: Some(servo_manager),
            fps_counter: FpsCounter::new(),
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
                    || self.current_url.starts_with("https://") {
                    self.current_url.clone()
                } else {
                    format!("https://{}", self.current_url)
                };
                Task::done(Message::NavigateTo(url))
            }
            
            Message::NavigateTo(url) => {
                self.current_url = url.clone();
                self.is_loading = true;
                
                if let Some(ref mut servo) = self.servo_manager {
                    if let Err(e) = servo.navigate(&url) {
                        error!("Failed to navigate: {}", e);
                    }
                }
                
                Task::done(Message::LoadingStarted)
            }
            
            Message::GoBack => {
                if let Some(ref mut servo) = self.servo_manager {
                    servo.go_back();
                }
                Task::none()
            }
            
            Message::GoForward => {
                if let Some(ref mut servo) = self.servo_manager {
                    servo.go_forward();
                }
                Task::none()
            }
            
            Message::Reload => {
                if let Some(ref mut servo) = self.servo_manager {
                    servo.reload();
                }
                Task::none()
            }
            
            Message::StopLoading => {
                if let Some(ref mut servo) = self.servo_manager {
                    servo.stop();
                }
                self.is_loading = false;
                Task::none()
            }
            
            Message::NewTab => {
                Task::done(Message::NavigateTo("https://start.duckduckgo.com".to_string()))
            }
            
            Message::CloseTab(_) => Task::none(),
            Message::SwitchTab(_) => Task::none(),
            
            Message::LoadingStarted => {
                self.is_loading = true;
                Task::none()
            }
            
            Message::LoadingFinished => {
                self.is_loading = false;
                Task::none()
            }
            
            Message::LoadingFailed(error) => {
                error!("Loading failed: {}", error);
                self.is_loading = false;
                Task::none()
            }
            
            Message::ServoTick => {
                // Update FPS counter
                self.fps_counter.tick();
                
                // Process Servo events
                if let Some(ref mut servo) = self.servo_manager {
                    servo.tick();
                    
                    if let Some(update) = servo.try_receive_updates() {
                        return self.handle_webview_update(update);
                    }
                }
                Task::none()
            }
            
            Message::WebViewInput(input) => {
                if let Some(ref mut servo) = self.servo_manager {
                    if let Some(event) = to_servo_input_event(&input) {
                        servo.handle_input_event(event);
                    }
                }
                
                if let WebViewMessage::Resize(width, height) = input {
                    self.servo_renderer.set_size(width as u32, height as u32);
                    if let Some(ref mut servo) = self.servo_manager {
                        servo.resize((width as u32, height as u32));
                    }
                }
                
                Task::none()
            }
        }
    }
    
    fn handle_webview_update(&mut self, update: WebViewUpdate) -> Task<Message> {
        if let Some(url) = update.url {
            self.current_url = url;
        }
        
        if let Some(title) = update.title {
            self.active_tab_title = Some(title);
        }
        
        if let Some(load_state) = update.load_state {
            return match load_state {
                browser_core::webview::LoadState::Started => {
                    Task::done(Message::LoadingStarted)
                }
                browser_core::webview::LoadState::Complete => {
                    Task::done(Message::LoadingFinished)
                }
                browser_core::webview::LoadState::Failed(error) => {
                    Task::done(Message::LoadingFailed(error))
                }
                _ => Task::none(),
            };
        }
        
        Task::none()
    }
    
    pub fn view(&self) -> Element<'_, Message> {
        // Toolbar - minimal styling
        let toolbar = row![
            button("←").on_press(Message::GoBack),
            button("→").on_press(Message::GoForward),
            button(if self.is_loading { "✕" } else { "⟳" }).on_press(Message::Reload),
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

        // Content area - simple placeholder
        let content = container(
            iced::widget::text("WebView")
                .size(16)
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        // Main layout - no animations, no fancy styling
        column![toolbar, address_bar, tab_bar, content].into()
    }
    
    pub fn subscription(&self) -> Subscription<Message> {
        // 60 FPS tick - no frame pacing, let VSync handle it
        iced::time::every(std::time::Duration::from_millis(16))
            .map(|_| Message::ServoTick)
    }
    
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
