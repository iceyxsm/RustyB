//! Rusty Browser UI - Iced-based user interface with Servo integration
//!
//! This crate provides the user interface for the Rusty Browser,
//! including:
//! - Main application window using Iced 0.14
//! - Servo WebView rendering with GPU texture sharing
//! - Input event handling (mouse, keyboard, touch)
//! - Navigation controls and address bar

pub mod app;
pub mod event_loop_waker;
pub mod integrated_app;
pub mod servo_renderer;
pub mod webview_widget;

// Re-export main types for convenience
pub use app::{BrowserApp, Message as AppMessage};
pub use integrated_app::{IntegratedBrowserApp, Message as IntegratedMessage};
pub use servo_renderer::ServoRenderer;
pub use webview_widget::{WebViewWidget, WebViewMessage};
