//! Hot Reload Server for Rust UI Development
//! 
//! Provides Electron-like hot reload by:
//! 1. Serving static files (HTML/CSS/JS) with live reload
//! 2. Watching for file changes and auto-refreshing the WebView
//! 3. Providing WebSocket for real-time communication
//!
//! Usage in your Rust app:
//! ```rust
//! use hot_reload::{HotReloadServer, DevServerConfig};
//! 
//! let server = HotReloadServer::new(DevServerConfig {
//!     port: 8080,
//!     workspace_dir: "./frontend".into(),
//!     ..Default::default()
//! }).await?;
//! 
//! // Get the URL to load in WebView
//! let url = server.url();
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Configuration for the hot reload development server
#[derive(Debug, Clone)]
pub struct DevServerConfig {
    /// Port to run the server on
    pub port: u16,
    /// Directory containing static files (HTML/CSS/JS)
    pub workspace_dir: PathBuf,
    /// Enable live reload (inject script into HTML)
    pub live_reload: bool,
    /// Enable sourcemaps
    pub sourcemap: bool,
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            workspace_dir: PathBuf::from("."),
            live_reload: true,
            sourcemap: true,
        }
    }
}

/// File change event for hot reload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileChangeEvent {
    #[serde(rename = "change")]
    Change { path: String },
    #[serde(rename = "add")]
    Add { path: String },
    #[serde(rename = "unlink")]
    Remove { path: String },
}

/// Hot reload server that serves files and notifies clients of changes
pub struct HotReloadServer {
    config: DevServerConfig,
    listener: TcpListener,
    /// Active WebSocket connections for live reload
    clients: Arc<RwLock<Vec<tokio::sync::mpsc::Sender<String>>>>,
}

impl HotReloadServer {
    /// Create and start the hot reload server
    pub async fn new(config: DevServerConfig) -> anyhow::Result<Self> {
        let addr = format!("127.0.0.1:{}", config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("Hot reload server listening on http://{}", addr);
        
        let server = Self {
            config,
            listener,
            clients: Arc::new(RwLock::new(Vec::new())),
        };
        
        Ok(server)
    }
    
    /// Get the base URL for the server
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.config.port)
    }
    
    /// Get WebSocket URL for live reload
    pub fn ws_url(&self) -> String {
        format!("ws://127.0.0.1:{}/livereload", self.config.port)
    }
    
    /// Handle HTTP requests
    pub async fn handle_request(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let mut buffer = [0u8; 4096];
        let bytes_read = stream.read(&mut buffer).await?;
        
        if bytes_read == 0 {
            return Ok(());
        }
        
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        let lines: Vec<&str> = request.lines().collect();
        
        if let Some(request_line) = lines.first() {
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            
            if parts.len() >= 2 {
                let method = parts[0];
                let path = parts[1];
                
                match (method, path) {
                    ("GET", "/livereload") => {
                        // Serve live reload info page
                        self.serve_livereload_info(stream).await?;
                    }
                    ("GET", "/__livereload__") => {
                        // Livereload script injection endpoint
                        self.serve_livereload_script(stream).await?;
                    }
                    _ => {
                        // Serve static file
                        self.serve_static_file(stream, path).await?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle live reload endpoint - serves info about live reload
    async fn serve_livereload_info(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let html = r#"
<!DOCTYPE html>
<html>
<head><title>Live Reload</title></head>
<body>
<h1>Live Reload Active</h1>
<p>Edit files in the frontend directory to see changes.</p>
</body>
</html>
"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}",
            html.len(),
            html
        );
        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
    
    /// Serve the livereload script that gets injected into HTML
    async fn serve_livereload_script(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let script = r#"
(function() {
    var ws = new WebSocket('ws://127.0.0.1:__PORT__/__livereload__');
    
    ws.onmessage = function(event) {
        var data = JSON.parse(event.data);
        console.log('[Hot Reload] Received:', data.type, data.path);
        
        if (data.type === 'change' || data.type === 'add') {
            if (data.path.endsWith('.html') || data.path.endsWith('.htm')) {
                location.reload();
            } else if (data.path.endsWith('.css')) {
                // Reload CSS without page refresh
                var links = document.querySelectorAll('link[rel="stylesheet"]');
                links.forEach(function(link) {
                    link.href = link.href.split('?')[0] + '?t=' + Date.now();
                });
            } else if (data.path.endsWith('.js')) {
                // Reload JS - simplest is to reload
                location.reload();
            }
        } else if (data.type === 'unlink') {
            // File was removed
            console.log('[Hot Reload] File removed:', data.path);
        }
    };
    
    ws.onclose = function() {
        console.log('[Hot Reload] Connection closed, retrying...');
        setTimeout(function() {
            location.reload();
        }, 1000);
    };
    
    console.log('[Hot Reload] Connected');
})();
"#.replace("__PORT__", &self.config.port.to_string());
        
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: application/javascript\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            \r\n\
            {}",
            script.len(),
            script
        );
        
        stream.write_all(response.as_bytes()).await?;
        
        Ok(())
    }
    
    /// Serve a static file from the workspace directory
    async fn serve_static_file(&self, mut stream: TcpStream, path: &str) -> anyhow::Result<()> {
        // Parse query string for cache busting
        let path = path.split('?').next().unwrap_or(path);
        
        // Build file path
        let mut file_path = self.config.workspace_dir.clone();
        
        if path == "/" || path.is_empty() {
            file_path.push("index.html");
        } else {
            // Prevent directory traversal
            let clean_path = path.trim_start_matches('/');
            if clean_path.contains("..") {
                stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n").await?;
                return Ok(());
            }
            file_path.push(clean_path);
        }
        
        // Read file
        let content = match fs::read(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                let response = format!(
                    "HTTP/1.1 404 Not Found\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    File not found: {}",
                    e
                );
                stream.write_all(response.as_bytes()).await?;
                return Ok(());
            }
        };
        
        // Determine content type
        let content_type = match file_path.extension().and_then(|e| e.to_str()) {
            Some("html") | Some("htm") => {
                // Inject livereload script into HTML
                let mut html = String::from_utf8_lossy(&content).to_string();
                
                if self.config.live_reload && !html.contains("__livereload__") {
                    let script = format!(
                        "<script src=\"/__livereload__\"></script>",
                    );
                    
                    if let Some(head_end) = html.find("</head>") {
                        html.insert_str(head_end, &script);
                    } else if let Some(body_end) = html.find("</body>") {
                        html.insert_str(body_end, &script);
                    }
                }
                
                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: text/html\r\n\
                    Content-Length: {}\r\n\
                    \r\n\
                    {}",
                    html.len(),
                    html
                );
                stream.write_all(response.as_bytes()).await?;
                return Ok(());
            }
            Some("css") => "text/css",
            Some("js") => "application/javascript",
            Some("json") => "application/json",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("svg") => "image/svg+xml",
            _ => "application/octet-stream",
        };
        
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: {}\r\n\
            Content-Length: {}\r\n\
            Cache-Control: no-cache\r\n\
            \r\n",
            content_type,
            content.len()
        );
        
        stream.write_all(response.as_bytes()).await?;
        stream.write_all(&content).await?;
        
        Ok(())
    }
    
    /// Broadcast file change to all connected clients
    pub async fn notify_change(&self, path: &str, event_type: &str) {
        let _event = FileChangeEvent::Change { 
            path: path.to_string() 
        };
        
        // Use the correct event type
        let json = match event_type {
            "add" => serde_json::to_string(&FileChangeEvent::Add { path: path.to_string() }).unwrap(),
            "unlink" => serde_json::to_string(&FileChangeEvent::Remove { path: path.to_string() }).unwrap(),
            _ => serde_json::to_string(&FileChangeEvent::Change { path: path.to_string() }).unwrap(),
        };
        
        let clients = self.clients.read().await;
        for client in clients.iter() {
            let _ = client.send(json.clone()).await;
        }
        
        info!("Broadcast: {} - {}", event_type, path);
    }
    
    /// Run the server (blocking)
    pub async fn run(self: Arc<Self>) {
        info!("Hot reload server running at {}", self.url());
        
        while let Ok((stream, addr)) = self.listener.accept().await {
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_request(stream).await {
                    warn!("Request error from {}: {}", addr, e);
                }
            });
        }
    }
}

/// File watcher for hot reload
pub struct FileWatcher {
    workspace_dir: PathBuf,
    server: Arc<HotReloadServer>,
}

impl FileWatcher {
    pub fn new(workspace_dir: PathBuf, server: Arc<HotReloadServer>) -> Self {
        Self { workspace_dir, server }
    }
    
    /// Start watching for file changes
    pub async fn watch(&self) -> anyhow::Result<()> {
        use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config};
        
        let server = self.server.clone();
        let workspace = self.workspace_dir.clone();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    let event_type = match event.kind {
                        notify::EventKind::Create(_) => "add",
                        notify::EventKind::Modify(_) => "change",
                        notify::EventKind::Remove(_) => "unlink",
                        _ => return,
                    };
                    
                    for path in event.paths {
                        // Get relative path
                        if let Ok(rel_path) = path.strip_prefix(&workspace) {
                            let path_str = rel_path.to_string_lossy().replace("\\", "/");
                            
                            // Skip node_modules, .git, etc.
                            if !path_str.contains("node_modules") && 
                               !path_str.contains(".git") &&
                               !path_str.starts_with(".") {
                                
                                let server_clone = server.clone();
                                let path_str = path_str.clone();
                                tokio::spawn(async move {
                                    server_clone.notify_change(&path_str, event_type).await;
                                });
                            }
                        }
                    }
                }
            },
            Config::default(),
        )?;
        
        watcher.watch(&self.workspace_dir, RecursiveMode::Recursive)?;
        
        info!("Watching {} for changes", self.workspace_dir.display());
        
        // Keep watcher alive
        tokio::time::sleep(tokio::time::Duration::MAX).await;
        
        Ok(())
    }
}

/// Start hot reload development environment
pub async fn start_dev_server(
    workspace_dir: PathBuf,
    port: u16,
) -> anyhow::Result<Arc<HotReloadServer>> {
    let config = DevServerConfig {
        port,
        workspace_dir: workspace_dir.clone(),
        live_reload: true,
        sourcemap: true,
    };
    
    let server = Arc::new(HotReloadServer::new(config).await?);
    let server_clone = server.clone();
    
    // Start HTTP server
    tokio::spawn(async move {
        server_clone.run().await;
    });
    
    // Start file watcher
    let watcher = FileWatcher::new(workspace_dir, server.clone());
    let watcher = Arc::new(watcher);
    let watcher_clone = watcher.clone();
    
    tokio::spawn(async move {
        if let Err(e) = watcher_clone.watch().await {
            error!("File watcher error: {}", e);
        }
    });
    
    Ok(server)
}
