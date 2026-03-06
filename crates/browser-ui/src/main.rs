//! Rusty Browser - A custom Rust-based browser

mod app;
mod views;
mod widgets;

use app::BrowserApp;

fn main() -> iced::Result {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    iced::application(
        BrowserApp::default,
        BrowserApp::update,
        BrowserApp::view,
    )
    .title(BrowserApp::title)
    .run()
}
