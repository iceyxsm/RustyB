//! Rusty Browser - A custom Rust-based browser

mod app;
mod views;
mod widgets;

use app::BrowserApp;
use iced::{Application, Settings};
use tracing_subscriber;

fn main() -> iced::Result {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    BrowserApp::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1280.0, 720.0),
            position: iced::window::Position::Default,
            min_size: Some(iced::Size::new(800.0, 600.0)),
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            always_on_top: false,
            icon: None,
            ..Default::default()
        },
        ..Default::default()
    })
}
