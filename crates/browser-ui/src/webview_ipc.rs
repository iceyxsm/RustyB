//! Production-grade WebView IPC implementation
//! 
//! Architecture: Separate process for WebView to avoid:
//! - Event loop conflicts with Iced
//! - Memory bloat in main process
//! - Crash propagation from WebView
//!
//! Communication: JSON-RPC over stdin/stdout with WebView subprocess

use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Arc;
use std::thread;
use parking_lot::Mutex;
use serde::{Serialize, Deserialize};
use tracing::{info, error, debug};

/// WebView subprocess path - will be compiled as separate binary
const WEBVIEW_SUBPROCESS: &str = "rusty-browser-webview";

/// JSON-RPC messages to WebView
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum WebViewCommand {
    Navigate { url: String },
    Reload,
    GoBack,
    GoForward,
    ExecuteScript { script: String },
    SetBounds { x: i32, y: i32, width: u32, height: u32 },
    Show,
    Hide,
    Close,
}

/// JSON-RPC messages from WebView
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum WebViewEvent {
    LoadStarted { url: String },
    LoadFinished { url: String, success: bool },
    UrlChanged { url: String },
    TitleChanged { title: String },
    NavigationRequested { url: String },
    PageError { error: String },
    ConsoleMessage { level: String, message: String },
    WindowClosed,
}

/// Shared state between UI and controller
pub struct WebViewState {
    pub url: String,
    pub title: String,
    pub is_loading: bool,
    pub is_visible: bool,
}

impl WebViewState {
    pub fn new() -> Self {
        Self {
            url: "about:blank".to_string(),
            title: "New Tab".to_string(),
            is_loading: false,
            is_visible: true,
        }
    }
}

/// Production WebView controller using IPC
pub struct WebViewController {
    state: Arc<Mutex<WebViewState>>,
    command_sender: Sender<WebViewCommand>,
    event_receiver: Receiver<WebViewEvent>,
    #[allow(dead_code)]
    subprocess: Arc<Mutex<Option<Child>>>,
}

impl WebViewController {
    /// Create and spawn WebView subprocess
    pub fn new(initial_url: &str) -> anyhow::Result<Self> {
        info!("Starting WebView subprocess...");

        // Try to spawn WebView subprocess
        let mut child = match Self::spawn_subprocess() {
            Ok(child) => {
                info!("WebView subprocess started successfully");
                child
            }
            Err(e) => {
                error!("Failed to spawn WebView subprocess: {}", e);
                error!("Falling back to headless mode - WebView UI only");
                // Return controller without subprocess - UI will still work
                let (cmd_tx, _cmd_rx) = channel();
                let (_evt_tx, evt_rx) = channel();
                
                return Ok(Self {
                    state: Arc::new(Mutex::new(WebViewState::new())),
                    command_sender: cmd_tx,
                    event_receiver: evt_rx,
                    subprocess: Arc::new(Mutex::new(None)),
                });
            }
        };

        let stdin = child.stdin.take().expect("Failed to get stdin");
        let stdout = child.stdout.take().expect("Failed to get stdout");

        let (cmd_tx, cmd_rx) = channel::<WebViewCommand>();
        let (evt_tx, evt_rx) = channel::<WebViewEvent>();

        let state = Arc::new(Mutex::new(WebViewState::new()));
        let state_clone = state.clone();

        // Spawn writer thread
        thread::spawn(move || {
            let mut stdin = stdin;
            while let Ok(cmd) = cmd_rx.recv() {
                let json = match serde_json::to_string(&cmd) {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Failed to serialize command: {}", e);
                        continue;
                    }
                };
                debug!("Sending to WebView: {}", json);
                if writeln!(stdin, "{}", json).is_err() {
                    error!("Failed to write to WebView stdin");
                    break;
                }
            }
        });

        // Spawn reader thread
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(json) => {
                        debug!("Received from WebView: {}", json);
                        match serde_json::from_str::<WebViewEvent>(&json) {
                            Ok(event) => {
                                // Update shared state
                                let mut state = state_clone.lock();
                                match &event {
                                    WebViewEvent::UrlChanged { url } => {
                                        state.url = url.clone();
                                    }
                                    WebViewEvent::TitleChanged { title } => {
                                        state.title = title.clone();
                                    }
                                    WebViewEvent::LoadStarted { .. } => {
                                        state.is_loading = true;
                                    }
                                    WebViewEvent::LoadFinished { .. } => {
                                        state.is_loading = false;
                                    }
                                    _ => {}
                                }
                                drop(state);
                                
                                if evt_tx.send(event).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from WebView: {}", e);
                        break;
                    }
                }
            }
        });

        // Send initial navigation
        let _ = cmd_tx.send(WebViewCommand::Navigate { 
            url: initial_url.to_string() 
        });

        Ok(Self {
            state,
            command_sender: cmd_tx,
            event_receiver: evt_rx,
            subprocess: Arc::new(Mutex::new(Some(child))),
        })
    }

    fn spawn_subprocess() -> anyhow::Result<Child> {
        // Check for env var from build script first
        let webview_exe = if let Ok(path) = std::env::var("WEBVIEW_SUBPROCESS_PATH") {
            PathBuf::from(path)
        } else {
            // Look in same directory as current executable
            let exe_path = std::env::current_exe()?;
            exe_path
                .parent()
                .map(|p| p.join(WEBVIEW_SUBPROCESS))
                .unwrap_or_else(|| WEBVIEW_SUBPROCESS.into())
        };

        info!("Spawning WebView subprocess: {}", webview_exe.display());

        let child = Command::new(&webview_exe)
            .arg("--ipc-mode")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn {}: {}", webview_exe.display(), e))?;

        Ok(child)
    }

    pub fn navigate(&self, url: &str) {
        let _ = self.command_sender.send(WebViewCommand::Navigate {
            url: url.to_string(),
        });
        self.state.lock().url = url.to_string();
    }

    pub fn reload(&self) {
        let _ = self.command_sender.send(WebViewCommand::Reload);
    }

    pub fn go_back(&self) {
        let _ = self.command_sender.send(WebViewCommand::GoBack);
    }

    pub fn go_forward(&self) {
        let _ = self.command_sender.send(WebViewCommand::GoForward);
    }

    pub fn set_bounds(&self, x: i32, y: i32, width: u32, height: u32) {
        let _ = self.command_sender.send(WebViewCommand::SetBounds { x, y, width, height });
    }

    pub fn show(&self) {
        let _ = self.command_sender.send(WebViewCommand::Show);
        self.state.lock().is_visible = true;
    }

    pub fn hide(&self) {
        let _ = self.command_sender.send(WebViewCommand::Hide);
        self.state.lock().is_visible = false;
    }

    pub fn current_url(&self) -> String {
        self.state.lock().url.clone()
    }

    pub fn current_title(&self) -> String {
        self.state.lock().title.clone()
    }

    pub fn is_loading(&self) -> bool {
        self.state.lock().is_loading
    }

    pub fn poll_events(&self) -> Vec<WebViewEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Drop for WebViewController {
    fn drop(&mut self) {
        let _ = self.command_sender.send(WebViewCommand::Close);
        if let Some(mut child) = self.subprocess.lock().take() {
            let _ = child.wait();
        }
    }
}

/// Iced-compatible wrapper
pub struct IcedWebView {
    controller: Arc<WebViewController>,
}

impl IcedWebView {
    pub fn new() -> Result<Self, String> {
        // Start with about:blank, will navigate when ready
        let controller = WebViewController::new("about:blank")
            .map_err(|e| format!("Failed to create WebView: {}", e))?;
        
        Ok(Self {
            controller: Arc::new(controller),
        })
    }

    pub fn navigate(&self, url: &str) {
        self.controller.navigate(url);
    }

    pub fn controller(&self) -> Arc<WebViewController> {
        self.controller.clone()
    }

    pub fn current_url(&self) -> String {
        self.controller.current_url()
    }

    pub fn current_title(&self) -> String {
        self.controller.current_title()
    }

    pub fn is_loading(&self) -> bool {
        self.controller.is_loading()
    }

    pub fn poll(&self) -> Vec<WebViewEvent> {
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
