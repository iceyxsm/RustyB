//! Rusty Browser UI - Iced-based user interface with WebView integration
//!
//! This crate provides the user interface for the Rusty Browser,
//! including:
//! - Main application window using Iced 0.14
//! - WRY WebView rendering (Edge WebView2 on Windows, WebKit on macOS)
//! - Input event handling (mouse, keyboard, touch)
//! - Navigation controls and address bar
//! - Comprehensive theme system with dark/light/high-contrast modes

pub mod app;
pub mod control_panel;
pub mod embedded_webview;
pub mod event_loop_waker;
pub mod gpu_renderer;
pub mod hybrid_app;
pub mod input_system;
pub mod integrated_app;
pub mod servo_integration;
pub mod servo_renderer;
pub mod theme;
pub mod webview_widget;
pub mod webview_ipc;
pub mod window_manager;

// Re-export main types for convenience
pub mod single_window_app;
pub mod two_window_app;

pub use app::{BrowserApp, Message as AppMessage};
pub use gpu_renderer::{GpuRenderer, RenderMode, InputEvent as GpuInputEvent, InputBatcher, FpsProfiler, GpuFrame};
pub use hybrid_app::{HybridBrowserApp, Message as HybridMessage};
pub use input_system::{
    InputManager, InputState, InputEvent, GestureRecognizer, Gesture,
    FocusManager, FocusId, FocusableElement,
    MouseButton, Key, NamedKey, Modifiers, ScrollDelta,
    AccessibilitySupport, AccessibilityAnnouncement,
    from_iced_mouse_event, from_iced_keyboard_event,
};
pub use integrated_app::{IntegratedBrowserApp, Message as IntegratedMessage};
pub use servo_integration::{
    ServoWebView, ServoConfig, ServoWebViewBuilder, ServoWebViewEvent,
    TouchPhase, CursorType, ServoIntegrationError,
    platform, cpu_fallback, renderer_integration, input,
    to_servo_mouse_button,
};
pub use browser_core::webview::MouseButton as ServoMouseButton;
pub use servo_renderer::ServoRenderer;
pub use webview_widget::{WebViewWidget, WebViewMessage};
pub use webview_ipc::{WebViewController, WebViewCommand, WebViewEvent, WebViewState};
