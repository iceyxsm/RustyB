//! WebView abstraction for browser rendering
//!
//! This module provides a trait-based abstraction for web rendering engines.
//! It can be implemented for different backends (Servo, WebKit, etc.)

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Load state for a webview
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState {
    /// Navigation started
    Started,
    /// Navigation committed (response received)
    Committed,
    /// Load complete
    Complete,
    /// Load failed
    Failed(String),
}

/// Input events for webview
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Mouse moved
    MouseMove(MouseMoveEvent),
    /// Mouse button event
    MouseButton(MouseButtonEvent),
    /// Wheel scroll event
    Wheel(WheelEvent),
    /// Keyboard event
    Keyboard(KeyboardEvent),
    /// Touch event
    Touch(TouchEvent),
}

/// Mouse move event
#[derive(Debug, Clone, Copy)]
pub struct MouseMoveEvent {
    pub point: Point2D,
}

/// Mouse button event
#[derive(Debug, Clone, Copy)]
pub struct MouseButtonEvent {
    pub button: MouseButton,
    pub action: MouseButtonAction,
}

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Mouse button actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonAction {
    Down,
    Up,
}

/// Wheel event
#[derive(Debug, Clone, Copy)]
pub struct WheelEvent {
    pub delta: WheelDelta,
    pub mode: WheelMode,
}

/// Wheel delta
#[derive(Debug, Clone, Copy)]
pub struct WheelDelta {
    pub x: f32,
    pub y: f32,
}

/// Wheel mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelMode {
    DeltaPixel,
    DeltaLine,
    DeltaPage,
}

/// Keyboard event
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub key: String,
    pub code: String,
    pub modifiers: Modifiers,
    pub state: KeyState,
}

/// Key modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Key state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Down,
    Up,
}

/// Touch event
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub id: TouchId,
    pub point: Point2D,
    pub event_type: TouchEventType,
}

/// Touch ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TouchId(pub i32);

/// Touch event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchEventType {
    Down,
    Up,
    Move,
    Cancel,
}

/// 2D point
#[derive(Debug, Clone, Copy)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// 2D size
#[derive(Debug, Clone, Copy)]
pub struct Size2D<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size2D<T> {
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

impl<T: Clone> Size2D<T> {
    pub fn clone(&self) -> Self {
        Self {
            width: self.width.clone(),
            height: self.height.clone(),
        }
    }
}

/// WebView delegate trait for handling webview events
pub trait WebViewDelegate: Send + Sync {
    /// Called when a new frame is ready
    fn notify_new_frame_ready(&self);
    /// Called when URL changes
    fn request_load_url(&self, url: String);
    /// Called when title changes
    fn notify_title_changed(&self, title: Option<String>);
    /// Called when load state changes
    fn notify_load_state_changed(&self, load_state: LoadState);
    /// Called when history state changes
    fn notify_history_changed(&self, can_go_back: bool, can_go_forward: bool);
    /// Called when status message changes
    fn notify_status_message(&self, message: Option<String>);
}

/// Servo delegate trait for handling Servo events
pub trait ServoDelegate: Send + Sync {
    /// Called when a new webview is requested (e.g., popup)
    fn notify_new_web_view_requested(&self, url: String) -> Option<Box<dyn WebView>>;
    /// Called when a webview should be closed
    fn notify_close_web_view(&self);
    /// Called when Servo shutdown is complete
    fn notify_shutdown_complete(&self);
}

/// WebView trait - abstraction for web rendering
pub trait WebView: Send + Sync {
    /// Navigate to a URL
    fn navigate(&self, url: &str) -> anyhow::Result<()>;
    /// Go back in history
    fn go_back(&self);
    /// Go forward in history
    fn go_forward(&self);
    /// Reload the current page
    fn reload(&self);
    /// Stop loading
    fn stop(&self);
    /// Handle input event
    fn handle_input_event(&self, event: InputEvent);
    /// Resize the webview
    fn resize(&self, size: Size2D<u32>);
    /// Get the current URL
    fn get_url(&self) -> String;
    /// Get the current title
    fn get_title(&self) -> Option<String>;
    /// Check if can go back
    fn can_go_back(&self) -> bool;
    /// Check if can go forward
    fn can_go_forward(&self) -> bool;
}

/// Window methods required by the webview
pub trait WindowMethods: Send + Sync {
    /// Get window coordinates
    fn get_coordinates(&self) -> EmbedderCoordinates;
    /// Set animation state
    fn set_animation_state(&self, state: AnimationState);
}

/// Embedder coordinates
#[derive(Debug, Clone, Copy)]
pub struct EmbedderCoordinates {
    pub viewport: Rect,
    pub framebuffer: Size2D<u32>,
    pub window: (Size2D<u32>, Point2D),
    pub screen: Size2D<u32>,
    pub screen_avail: Size2D<u32>,
    pub hidpi_factor: f32,
}

/// Rectangle
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub origin: Point2D,
    pub size: Size2D<f32>,
}

impl Rect {
    pub fn new(origin: Point2D, size: Size2D<f32>) -> Self {
        Self { origin, size }
    }
}

/// Animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Idle,
    Animating,
}

/// WebView update from renderer
#[derive(Debug, Clone)]
pub struct WebViewUpdate {
    pub url: Option<String>,
    pub title: Option<String>,
    pub load_state: Option<LoadState>,
    pub can_go_back: Option<bool>,
    pub can_go_forward: Option<bool>,
}

/// Servo window implementation
pub struct ServoWindow {
    pub window_size: Size2D<u32>,
    pub scale_factor: f32,
}

impl ServoWindow {
    pub fn new(
        size: Size2D<u32>,
        scale_factor: f32,
    ) -> Self {
        Self {
            window_size: size,
            scale_factor,
        }
    }
}

impl WindowMethods for ServoWindow {
    fn get_coordinates(&self) -> EmbedderCoordinates {
        EmbedderCoordinates {
            viewport: Rect::new(
                Point2D::zero(),
                Size2D::new(self.window_size.width as f32, self.window_size.height as f32),
            ),
            framebuffer: self.window_size.clone(),
            window: (self.window_size.clone(), Point2D::zero()),
            screen: self.window_size.clone(),
            screen_avail: self.window_size.clone(),
            hidpi_factor: self.scale_factor,
        }
    }

    fn set_animation_state(&self, _state: AnimationState) {
        // Handle animation state changes
    }
}

/// Default webview delegate implementation
pub struct DefaultWebViewDelegate {
    pub url_sender: mpsc::Sender<String>,
    pub title_sender: mpsc::Sender<String>,
    pub load_state_sender: mpsc::Sender<LoadState>,
}

impl WebViewDelegate for DefaultWebViewDelegate {
    fn notify_new_frame_ready(&self) {
        // Frame is ready to be presented
    }

    fn request_load_url(&self, _url: String) {
        // Handle URL load request
    }

    fn notify_title_changed(&self, title: Option<String>) {
        if let Some(t) = title {
            let _ = self.title_sender.try_send(t);
        }
    }

    fn notify_load_state_changed(&self, load_state: LoadState) {
        let _ = self.load_state_sender.try_send(load_state);
    }

    fn notify_history_changed(&self, _can_go_back: bool, _can_go_forward: bool) {
        // Handle history state changes
    }

    fn notify_status_message(&self, message: Option<String>) {
        if let Some(msg) = message {
            debug!("Status: {}", msg);
        }
    }
}

/// Default Servo delegate implementation
pub struct DefaultServoDelegate;

impl ServoDelegate for DefaultServoDelegate {
    fn notify_new_web_view_requested(&self, url: String) -> Option<Box<dyn WebView>> {
        info!("New web view requested for: {}", url);
        None // Return None for now - would create new webview in full implementation
    }

    fn notify_close_web_view(&self) {
        info!("Close web view requested");
    }

    fn notify_shutdown_complete(&self) {
        info!("Servo shutdown complete");
    }
}

/// Manages webview instances
pub struct ServoManager {
    /// Current webview
    current_webview: Option<Arc<dyn WebView>>,
    /// Window reference
    window: Arc<std::sync::RwLock<ServoWindow>>,
    /// URL receiver
    url_receiver: mpsc::Receiver<String>,
    /// Title receiver
    title_receiver: mpsc::Receiver<String>,
    /// Load state receiver
    load_state_receiver: mpsc::Receiver<LoadState>,
}

impl ServoManager {
    pub fn new(
        size: (u32, u32),
        scale_factor: f32,
    ) -> anyhow::Result<(Self, mpsc::Sender<String>, mpsc::Sender<String>, mpsc::Sender<LoadState>)> {
        let (url_tx, url_rx) = mpsc::channel(100);
        let (title_tx, title_rx) = mpsc::channel(100);
        let (load_state_tx, load_state_rx) = mpsc::channel(100);

        let window_size = Size2D::new(size.0, size.1);
        let servo_window = Arc::new(std::sync::RwLock::new(ServoWindow::new(
            window_size,
            scale_factor,
        )));

        let manager = Self {
            current_webview: None,
            window: servo_window,
            url_receiver: url_rx,
            title_receiver: title_rx,
            load_state_receiver: load_state_rx,
        };

        Ok((manager, url_tx, title_tx, load_state_tx))
    }

    pub fn initialize(&mut self) -> anyhow::Result<()> {
        info!("Initializing Servo manager...");
        // In full implementation, this would create the Servo instance
        info!("Servo manager initialized successfully");
        Ok(())
    }

    pub fn create_webview(&mut self, url: &str) -> anyhow::Result<()> {
        info!("Creating webview for: {}", url);
        // In full implementation, this would create a real webview
        // For now, we just log the request
        Ok(())
    }

    pub fn navigate(&self, url: &str) -> anyhow::Result<()> {
        if let Some(ref webview) = self.current_webview {
            webview.navigate(url)?;
        } else {
            return Err(anyhow::anyhow!("No active webview"));
        }
        Ok(())
    }

    pub fn go_back(&self) {
        if let Some(ref webview) = self.current_webview {
            webview.go_back();
        }
    }

    pub fn go_forward(&self) {
        if let Some(ref webview) = self.current_webview {
            webview.go_forward();
        }
    }

    pub fn reload(&self) {
        if let Some(ref webview) = self.current_webview {
            webview.reload();
        }
    }

    pub fn stop(&self) {
        if let Some(ref webview) = self.current_webview {
            webview.stop();
        }
    }

    pub fn handle_input_event(&self, event: InputEvent) {
        if let Some(ref webview) = self.current_webview {
            webview.handle_input_event(event);
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if let Ok(mut window) = self.window.write() {
            window.window_size = Size2D::new(size.0, size.1);
        }
        if let Some(ref webview) = self.current_webview {
            webview.resize(Size2D::new(size.0, size.1));
        }
    }

    pub fn tick(&self) {
        // Process Servo events
        // In full implementation, this would call servo.spin()
    }

    pub fn shutdown(&mut self) {
        info!("Shutting down Servo manager");
        self.current_webview = None;
    }

    pub fn try_receive_updates(&mut self) -> Option<WebViewUpdate> {
        // Try to receive updates from channels
        let url = self.url_receiver.try_recv().ok();
        let title = self.title_receiver.try_recv().ok();
        let load_state = self.load_state_receiver.try_recv().ok();

        if url.is_some() || title.is_some() || load_state.is_some() {
            Some(WebViewUpdate {
                url,
                title,
                load_state,
                can_go_back: None,
                can_go_forward: None,
            })
        } else {
            None
        }
    }
}
