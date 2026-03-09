//! Rusty Browser - Hybrid Mode (Control Panel + WebView)
//!
//! Window 1: Control Panel (Iced) - positioned at left
//! Window 2: WebView (Edge WebView2 via WRY) - positioned at right

use browser_ui::hybrid_app::HybridBrowserApp;
use iced::window;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    iced::application(
        HybridBrowserApp::default,
        HybridBrowserApp::update,
        HybridBrowserApp::view,
    )
    .title(HybridBrowserApp::title)
    .subscription(HybridBrowserApp::subscription)
    .theme(HybridBrowserApp::theme)
    .window(window::Settings {
        size: iced::Size::new(550.0, 800.0),
        position: window::Position::Specific(iced::Point::new(100.0, 100.0)),
        min_size: Some(iced::Size::new(400.0, 600.0)),
        ..Default::default()
    })
    .run()
}
