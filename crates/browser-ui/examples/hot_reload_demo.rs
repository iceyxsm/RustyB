//! Hot Reload Demo - Electron-like development experience
//! 
//! This example demonstrates how to use the hot reload system:
//! 1. Frontend (HTML/CSS/JS) - Changes auto-reload without full app restart
//! 2. Rust code - Uses cargo-watch for auto-recompile
//!
//! Run with: cargo run --example hot_reload_demo
//! Or use the dev script: ./scripts/dev.bat

use browser_ui::hot_reload::{start_dev_server, HotReloadServer, DevServerConfig};
use browser_ui::hybrid_app::{HybridBrowserApp, Message};
use iced::window;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    
    // Determine frontend directory
    let frontend_dir = PathBuf::from(std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("frontend"));
    
    // Check if we're in development mode
    let dev_mode = std::env::var("DEV_MODE").unwrap_or_default() == "1";
    
    let server = if dev_mode && frontend_dir.exists() {
        println!(" reload server...");
        
 Starting hot        let config = DevServerConfig {
            port: 8080,
            workspace_dir: frontend_dir.clone(),
            live_reload: true,
            sourcemap: true,
        };
        
        Some(start_dev_server(frontend_dir, 8080).await.unwrap())
    } else {
        None
    };
    
    // Get the URL to load in WebView
    let webview_url = server
        .as_ref()
        .map(|s| s.url())
        .unwrap_or_else(|| "https://start.duckduckgo.com".to_string());
    
    println!("🌐 Loading WebView with: {}", webview_url);
    
    // Run the Iced application
    // The WebView will load from our hot reload server in dev mode
    iced::application(
        HybridBrowserApp::default,
        HybridBrowserApp::update,
        HybridBrowserApp::view,
    )
    .title("Rusty Browser - Hot Reload Demo")
    .subscription(HybridBrowserApp::subscription)
    .theme(HybridBrowserApp::theme)
    .window(window::Settings {
        size: iced::Size::new(1400.0, 900.0),
        position: window::Position::Specific(iced::Point::new(50.0, 50.0)),
        min_size: Some(iced::Size::new(800.0, 600.0)),
        ..Default::default()
    })
    .run()
}

/// Alternative: Use hot reload server in your app state
/// 
/// ```ignore
/// use browser_ui::hot_reload::{HotReloadServer, DevServerConfig};
/// 
/// struct AppState {
///     hot_reload: Option<Arc<HotReloadServer>>,
/// }
/// 
/// impl AppState {
///     fn new() -> Self {
///         // Check for DEV_MODE environment variable
///         if std::env::var("DEV_MODE").unwrap_or_default() == "1" {
///             let config = DevServerConfig {
///                 port: 8080,
///                 workspace_dir: PathBuf::from("./frontend"),
///                 ..Default::default()
///             };
///             
///             let server = tokio::runtime::Runtime::new()
///                 .unwrap()
///                 .block_on(HotReloadServer::new(config))
///                 .unwrap();
///             
///             Self { hot_reload: Some(Arc::new(server)) }
///         } else {
///             Self { hot_reload: None }
///         }
///     }
/// }
/// ```

/// Quick Start:
/// 
/// 1. Create your frontend files in `frontend/` folder:
///    - frontend/index.html
///    - frontend/styles.css  
///    - frontend/app.js
/// 
/// 2. Run in development mode:
///    DEV_MODE=1 cargo run --example hot_reload_demo
/// 
///    Or use the dev script:
///    ./scripts/dev.bat
/// 
/// 3. Edit any file in frontend/ and see instant changes!
/// 
/// For Rust code changes, the dev script uses cargo-watch to auto-recompile.
