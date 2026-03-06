//! Rusty Browser - A custom Rust-based browser

mod app;
mod views;
mod widgets;

use app::{BrowserApp, Message};
use tracing_subscriber;

fn main() -> iced::Result {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    iced::run("Rusty Browser", update, view)
}

fn update(app: &mut BrowserApp, message: Message) -> iced::Task<Message> {
    app.update(message)
}

fn view(app: &BrowserApp) -> iced::Element<Message> {
    app.view()
}
