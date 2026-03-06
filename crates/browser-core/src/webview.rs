//! Servo WebView integration for rendering web content

use servo::{
    compositing::windowing::{WindowMethods, EmbedderCoordinates},
    euclid::{Point2D, Rect, Scale, Size2D},
    servo_url::ServoUrl,
    webrender_api::units::{DevicePixel, LayoutPixel},
    Servo, 
    WebView,
    WebViewBuilder,
    ServoBuilder,
    WebViewDelegate,
    ServoDelegate,
    InputEvent,
    MouseButton,
    MouseButtonAction,
    MouseButtonEvent,
    TouchEvent,
    TouchAction,
    TouchId,
    TouchType,
    WheelDelta,
    WheelEvent,
    WheelMode,
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use raw_window_handle::{HasRawWindowHandle, HasRawDisplayHandle, RawWindowHandle, RawDisplayHandle};

/// Delegate for handling WebView events
pub struct RustyWebViewDelegate {
    pub url_sender: mpsc::Sender<String>,
    pub title_sender: mpsc::Sender<String>,
    pub load_state_sender: mpsc::Sender<LoadState>,
}

impl WebViewDelegate for RustyWebViewDelegate {
    fn notify_new_frame_ready(&self, _webview: WebView) {
        // Frame is ready to be presented
    }

    fn request_load_url(&self, _webview: WebView, url: ServoUrl) {
        let _ = self.url_sender.try_send(url.to_string());
    }

    fn notify_title_changed(&self, _webview: WebView, title: Option<String>) {
        if let Some(t) = title {
            let _ = self.title_sender.try_send(t);
        }
    }

    fn notify_load_state_changed(&self, _webview: WebView, load_state: LoadState) {
        let _ = self.load_state_sender.try_send(load_state);
    }

    fn notify_history_changed(&self, _webview: WebView, can_go_back: bool, can_go_forward: bool) {
        // Handle history state changes
    }

    fn notify_status_message(&self, _webview: WebView, message: Option<String>) {
        if let Some(msg) = message {
            debug!("Status: {}", msg);
        }
    }
}

/// Delegate for handling Servo events
pub struct RustyServoDelegate;

impl ServoDelegate for RustyServoDelegate {
    fn notify_new_web_view_requested(
        &self,
        servo: &Servo,
        url: ServoUrl,
        _opener: Option<WebView>,
    ) -> Option<WebView> {
        info!("New web view requested for: {}", url);
        // Create a new webview for popups
        let webview = WebViewBuilder::new(servo)
            .url(url)
            .build();
        Some(webview)
    }

    fn notify_close_web_view(&self, _servo: &Servo, webview: WebView) {
        info!("Close web view requested: {:?}", webview.id());
    }

    fn notify_shutdown_complete(&self, _servo: &Servo) {
        info!("Servo shutdown complete");
    }
}

/// Window implementation for Servo
pub struct ServoWindow {
    pub window_size: Size2D<u32, DevicePixel>,
    pub scale_factor: Scale<f32, DevicePixel, LayoutPixel>,
    pub raw_window_handle: RawWindowHandle,
    pub raw_display_handle: RawDisplayHandle,
}

impl ServoWindow {
    pub fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(
        window: &W,
        size: Size2D<u32, DevicePixel>,
        scale_factor: f32,
    ) -> Self {
        Self {
            window_size: size,
            scale_factor: Scale::new(scale_factor),
            raw_window_handle: window.raw_window_handle(),
            raw_display_handle: window.raw_display_handle(),
        }
    }
}

impl WindowMethods for ServoWindow {
    fn get_coordinates(&self) -> EmbedderCoordinates {
        EmbedderCoordinates {
            viewport: Rect::new(
                Point2D::zero(),
                self.window_size.cast(),
            ),
            framebuffer: self.window_size,
            window: (self.window_size, Point2D::zero()),
            screen: self.window_size,
            screen_avail: self.window_size,
            hidpi_factor: self.scale_factor,
        }
    }

    fn set_animation_state(&self, _state: servo::AnimationState) {
        // Handle animation state changes
    }

    fn render(&self, _webrender_surfman: &servo::Surfman, _size: Size2D<u32, DevicePixel>) {
        // Rendering is handled by the compositor
    }
}

/// Manages Servo instances and webviews
pub struct ServoManager {
    servo: Option<Servo>,
    current_webview: Option<WebView>,
    window: Arc<ServoWindow>,
    url_receiver: mpsc::Receiver<String>,
    title_receiver: mpsc::Receiver<String>,
    load_state_receiver: mpsc::Receiver<LoadState>,
}

impl ServoManager {
    pub fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(
        window: &W,
        size: (u32, u32),
        scale_factor: f32,
    ) -> anyhow::Result<(Self, mpsc::Sender<String>, mpsc::Sender<String>, mpsc::Sender<LoadState>)> {
        let (url_tx, url_rx) = mpsc::channel(100);
        let (title_tx, title_rx) = mpsc::channel(100);
        let (load_state_tx, load_state_rx) = mpsc::channel(100);

        let window_size = Size2D::new(size.0, size.1);
        let servo_window = Arc::new(ServoWindow::new(
            window,
            window_size,
            scale_factor,
        ));

        let manager = Self {
            servo: None,
            current_webview: None,
            window: servo_window,
            url_receiver: url_rx,
            title_receiver: title_rx,
            load_state_receiver: load_state_rx,
        };

        Ok((manager, url_tx, title_tx, load_state_tx))
    }

    pub fn initialize(&mut self) -> anyhow::Result<()> {
        info!("Initializing Servo...");

        // Create delegates
        let webview_delegate = Box::new(RustyWebViewDelegate {
            url_sender: self.url_receiver.clone().into(),
            title_sender: self.title_receiver.clone().into(),
            load_state_sender: self.load_state_receiver.clone().into(),
        });

        let servo_delegate = Box::new(RustyServoDelegate);

        // Build Servo instance
        let servo = ServoBuilder::new(self.window.clone())
            .webview_delegate(webview_delegate)
            .servo_delegate(servo_delegate)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Servo: {:?}", e))?;

        self.servo = Some(servo);
        info!("Servo initialized successfully");

        Ok(())
    }

    pub fn create_webview(&mut self, url: &str) -> anyhow::Result<WebView> {
        let servo = self.servo.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Servo not initialized"))?;

        let servo_url = ServoUrl::parse(url)
            .map_err(|e| anyhow::anyhow!("Invalid URL: {:?}", e))?;

        let webview = WebViewBuilder::new(servo)
            .url(servo_url)
            .build();

        self.current_webview = Some(webview.clone());
        info!("Created webview for: {}", url);

        Ok(webview)
    }

    pub fn navigate(&self, url: &str) -> anyhow::Result<()> {
        if let Some(webview) = &self.current_webview {
            let servo_url = ServoUrl::parse(url)
                .map_err(|e| anyhow::anyhow!("Invalid URL: {:?}", e))?;
            webview.navigate(servo_url);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active webview"))
        }
    }

    pub fn go_back(&self) {
        if let Some(webview) = &self.current_webview {
            webview.go_back();
        }
    }

    pub fn go_forward(&self) {
        if let Some(webview) = &self.current_webview {
            webview.go_forward();
        }
    }

    pub fn reload(&self) {
        if let Some(webview) = &self.current_webview {
            webview.reload();
        }
    }

    pub fn stop(&self) {
        if let Some(webview) = &self.current_webview {
            webview.stop();
        }
    }

    pub fn handle_input_event(&self, event: InputEvent) {
        if let Some(webview) = &self.current_webview {
            webview.handle_input_event(event);
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.window.window_size = Size2D::new(size.0, size.1);
        if let Some(webview) = &self.current_webview {
            webview.resize(Size2D::new(size.0, size.1));
        }
    }

    pub fn tick(&self) {
        if let Some(servo) = &self.servo {
            servo.spin();
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(servo) = self.servo.take() {
            servo.shutdown();
        }
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
            })
        } else {
            None
        }
    }
}

/// Update from webview
#[derive(Debug, Clone)]
pub struct WebViewUpdate {
    pub url: Option<String>,
    pub title: Option<String>,
    pub load_state: Option<LoadState>,
}

use servo::LoadState;
