//! Rusty Browser - A custom Rust-based browser with Servo rendering
//!
//! This is the main entry point for the Rusty Browser application.
//! It uses Iced 0.14 for the UI and integrates Servo for web rendering.

use browser_ui::integrated_app::{IntegratedBrowserApp, Message};

fn main() -> iced::Result {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Run the integrated browser application
    iced::application(
        IntegratedBrowserApp::default,
        IntegratedBrowserApp::update,
        IntegratedBrowserApp::view,
    )
    .title(IntegratedBrowserApp::title)
    .subscription(IntegratedBrowserApp::subscription)
    .theme(IntegratedBrowserApp::theme)
    .run()
}
