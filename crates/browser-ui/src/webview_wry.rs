//! Production-grade WebView implementation using WRY
//!
//! WRY is a cross-platform WebView library that uses:
//! - Windows: Edge WebView2
//! - macOS: WebKit WKWebView
//! - Linux: WebKitGTK

use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use tracing::{error, info};

/// Messages that can be sent to the WebView
#[derive(Debug, Clone)]
pub enum WebViewCommand {
    Navigate(String),
    GoBack,
    GoForward,
    Reload,
}

/// Messages received from the WebView
#[derive(Debug, Clone)]
pub enum WebViewEvent {
    LoadStarted,
    LoadFinished,
    UrlChanged(String),
    TitleChanged(String),
}

/// WebView controller
pub struct WebViewController {
    command_tx: Sender<WebViewCommand>,
    event_rx: Receiver<WebViewEvent>,
    current_url: Arc<Mutex<String>>,
    current_title: Arc<Mutex<String>>,
    is_loading: Arc<Mutex<bool>>,
}

impl WebViewController {
    /// Create a new WebView controller
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (command_tx, command_rx) = channel::<WebViewCommand>();
        let (event_tx, event_rx) = channel::<WebViewEvent>();
        
        let current_url = Arc::new(Mutex::new("about:blank".to_string()));
        let current_title = Arc::new(Mutex::new("New Tab".to_string()));
        let is_loading = Arc::new(Mutex::new(false));
        
        // Start WebView thread
        std::thread::spawn(move || {
            run_wry_thread(command_rx, event_tx);
        });
        
        info!("WebView controller created");
        
        Ok(Self {
            command_tx,
            event_rx,
            current_url,
            current_title,
            is_loading,
        })
    }
    
    /// Navigate to a URL
    pub fn navigate(&self, url: &str) -> Result<(), String> {
        info!("Navigating to: {}", url);
        *self.current_url.lock() = url.to_string();
        *self.is_loading.lock() = true;
        self.command_tx
            .send(WebViewCommand::Navigate(url.to_string()))
            .map_err(|e| format!("Failed to send navigate: {}", e))?;
        Ok(())
    }
    
    /// Get current URL
    pub fn current_url(&self) -> String {
        self.current_url.lock().clone()
    }
    
    /// Get current title
    pub fn current_title(&self) -> String {
        self.current_title.lock().clone()
    }
    
    /// Check if page is loading
    pub fn is_loading(&self) -> bool {
        *self.is_loading.lock()
    }
    
    /// Poll for events
    pub fn poll_events(&self) -> Vec<WebViewEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            match &event {
                WebViewEvent::UrlChanged(url) => {
                    *self.current_url.lock() = url.clone();
                }
                WebViewEvent::TitleChanged(title) => {
                    *self.current_title.lock() = title.clone();
                }
                WebViewEvent::LoadStarted => {
                    *self.is_loading.lock() = true;
                }
                WebViewEvent::LoadFinished => {
                    *self.is_loading.lock() = false;
                }
            }
            events.push(event);
        }
        events
    }
}

/// Run the WRY WebView in a dedicated thread
fn run_wry_thread(
    command_rx: Receiver<WebViewCommand>,
    event_tx: Sender<WebViewEvent>,
) {
    use tao::{
        event::{Event, WindowEvent, StartCause},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;
    
    #[cfg(target_os = "windows")]
    use tao::platform::windows::EventLoopBuilderExtWindows;
    
    info!("Starting WRY WebView thread...");
    
    // On Windows, allow event loop on any thread
    #[cfg(target_os = "windows")]
    let event_loop = EventLoopBuilder::new().with_any_thread(true).build();
    
    #[cfg(not(target_os = "windows"))]
    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("Rusty Browser - WebView")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 720.0))
        .with_visible(true)
        .build(&event_loop)
        .expect("Failed to create window");
    
    let event_tx_nav = event_tx.clone();
    
    let webview_builder = WebViewBuilder::new()
        .with_url("about:blank")
        .with_devtools(true)
        .with_navigation_handler(move |url| {
            info!("Navigation: {}", url);
            let _ = event_tx_nav.send(WebViewEvent::LoadStarted);
            let _ = event_tx_nav.send(WebViewEvent::UrlChanged(url.to_string()));
            true
        });
    
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let webview = webview_builder.build(&window).expect("Failed to create webview");
    
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get vbox");
        webview_builder.build_gtk(vbox).expect("Failed to create webview")
    };
    
    // Send initial events
    let _ = event_tx.send(WebViewEvent::UrlChanged("about:blank".to_string()));
    let _ = event_tx.send(WebViewEvent::TitleChanged("New Tab".to_string()));
    let _ = event_tx.send(WebViewEvent::LoadFinished);
    
    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::NewEvents(StartCause::Init) => {
                info!("WebView initialized");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Process commands
                while let Ok(cmd) = command_rx.try_recv() {
                    match cmd {
                        WebViewCommand::Navigate(url) => {
                            info!("Navigating to: {}", url);
                            let _ = webview.load_url(&url);
                        }
                        WebViewCommand::Reload => {
                            webview.reload();
                        }
                        WebViewCommand::GoBack => {
                            let _ = webview.evaluate_script("history.back()");
                        }
                        WebViewCommand::GoForward => {
                            let _ = webview.evaluate_script("history.forward()");
                        }
                    }
                }
            }
            _ => {}
        }
    });
}

/// Iced-compatible WebView widget
pub struct IcedWebView {
    controller: Arc<WebViewController>,
}

impl IcedWebView {
    /// Create a new WebView widget
    pub fn new() -> Result<Self, String> {
        let controller = WebViewController::new()
            .map_err(|e| format!("Failed to create WebView: {}", e))?;
        
        Ok(Self {
            controller: Arc::new(controller),
        })
    }
    
    /// Navigate to URL
    pub fn navigate(&self, url: &str) -> Result<(), String> {
        self.controller.navigate(url)
    }
    
    /// Get controller
    pub fn controller(&self) -> Arc<WebViewController> {
        self.controller.clone()
    }
    
    /// Poll for events
    pub fn poll_events(&self) -> Vec<WebViewEvent> {
        self.controller.poll_events()
    }
}

impl Default for IcedWebView {
    fn default() -> Self {
        Self::new().expect("Failed to create WebView")
    }
}

unsafe impl Send for IcedWebView {}
unsafe impl Sync for IcedWebView {}
