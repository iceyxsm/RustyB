//! WebSocket server for real-time remote control

use crate::commands::{RemoteCommand, CommandResult, BrowserInfo, TabInfo};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket command server
pub struct WebSocketServer {
    addr: SocketAddr,
    command_tx: mpsc::Sender<(RemoteCommand, mpsc::Sender<CommandResult>)>,
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    shutdown_tx: broadcast::Sender<()>,
}

struct Connection {
    id: Uuid,
    addr: SocketAddr,
    tx: mpsc::Sender<Message>,
}

impl WebSocketServer {
    pub fn new(
        addr: SocketAddr,
        command_tx: mpsc::Sender<(RemoteCommand, mpsc::Sender<CommandResult>)>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            addr,
            command_tx,
            connections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("WebSocket server listening on {}", self.addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                Ok((stream, addr)) = listener.accept() => {
                    let command_tx = self.command_tx.clone();
                    let connections = Arc::clone(&self.connections);
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            addr,
                            command_tx,
                            connections,
                        ).await {
                            error!("WebSocket connection error from {}: {}", addr, e);
                        }
                    });
                }
                _ = shutdown_rx.recv() => {
                    info!("WebSocket server shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        command_tx: mpsc::Sender<(RemoteCommand, mpsc::Sender<CommandResult>)>,
        connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    ) -> anyhow::Result<()> {
        let ws_stream = accept_async(stream).await?;
        info!("WebSocket connection established from {}", addr);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        
        let connection_id = Uuid::new_v4();
        
        // Store connection
        {
            let mut conns = connections.write().await;
            conns.insert(connection_id, Connection {
                id: connection_id,
                addr,
                tx: tx.clone(),
            });
        }

        // Spawn task to send messages to client
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received from {}: {}", addr, text);
                    
                    // Parse command
                    match serde_json::from_str::<RemoteCommand>(&text) {
                        Ok(command) => {
                            // Create response channel
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            
                            // Send command to handler
                            if command_tx.send((command, resp_tx)).await.is_ok() {
                                // Wait for result
                                if let Some(result) = resp_rx.recv().await {
                                    let response = serde_json::to_string(&result)?;
                                    let _ = tx.send(Message::Text(response.into())).await;
                                }
                            }
                        }
                        Err(e) => {
                            let error_result = CommandResult::error(format!("Invalid command: {}", e));
                            let response = serde_json::to_string(&error_result)?;
                            let _ = tx.send(Message::Text(response.into())).await;
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed from {}", addr);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error from {}: {}", addr, e);
                    break;
                }
                _ => {}
            }
        }

        // Remove connection
        {
            let mut conns = connections.write().await;
            conns.remove(&connection_id);
        }

        send_task.abort();
        Ok(())
    }

    pub async fn broadcast(&self, message: &str) -> anyhow::Result<()> {
        let conns = self.connections.read().await;
        for (_, conn) in conns.iter() {
            let _ = conn.tx.send(Message::Text(message.to_string().into())).await;
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    pub async fn get_connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

/// Command processor that handles commands from WebSocket and REST API
pub struct CommandProcessor {
    command_rx: mpsc::Receiver<(RemoteCommand, mpsc::Sender<CommandResult>)>,
    browser_controller: Arc<dyn BrowserController>,
}

#[async_trait::async_trait]
pub trait BrowserController: Send + Sync {
    async fn navigate(&self, url: &str) -> anyhow::Result<()>;
    async fn go_back(&self) -> anyhow::Result<()>;
    async fn go_forward(&self) -> anyhow::Result<()>;
    async fn reload(&self) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
    async fn get_url(&self) -> anyhow::Result<String>;
    async fn get_title(&self) -> anyhow::Result<String>;
    async fn execute_js(&self, script: &str) -> anyhow::Result<serde_json::Value>;
    async fn click(&self, selector: &str) -> anyhow::Result<()>;
    async fn type_text(&self, selector: &str, text: &str) -> anyhow::Result<()>;
    async fn get_dom(&self) -> anyhow::Result<String>;
    async fn get_content(&self) -> anyhow::Result<String>;
    async fn screenshot(&self, full_page: bool) -> anyhow::Result<Vec<u8>>;
    async fn get_browser_info(&self) -> anyhow::Result<BrowserInfo>;
    async fn get_tabs(&self) -> anyhow::Result<Vec<TabInfo>>;
    async fn new_tab(&self, url: Option<&str>) -> anyhow::Result<Uuid>;
    async fn close_tab(&self, tab_id: Uuid) -> anyhow::Result<()>;
    async fn switch_tab(&self, tab_id: Uuid) -> anyhow::Result<()>;
}

impl CommandProcessor {
    pub fn new(
        command_rx: mpsc::Receiver<(RemoteCommand, mpsc::Sender<CommandResult>)>,
        browser_controller: Arc<dyn BrowserController>,
    ) -> Self {
        Self {
            command_rx,
            browser_controller,
        }
    }

    pub async fn run(mut self) {
        while let Some((command, response_tx)) = self.command_rx.recv().await {
            let result = self.process_command(command).await;
            let _ = response_tx.send(result).await;
        }
    }

    async fn process_command(&self, command: RemoteCommand) -> CommandResult {
        let start = std::time::Instant::now();
        
        let result = match command {
            RemoteCommand::Navigate { url } => {
                match self.browser_controller.navigate(&url).await {
                    Ok(_) => CommandResult::success(serde_json::json!({"navigated": true})),
                    Err(e) => CommandResult::error(format!("Navigation failed: {}", e)),
                }
            }
            RemoteCommand::GoBack => {
                match self.browser_controller.go_back().await {
                    Ok(_) => CommandResult::success(serde_json::json!({"went_back": true})),
                    Err(e) => CommandResult::error(format!("Go back failed: {}", e)),
                }
            }
            RemoteCommand::GoForward => {
                match self.browser_controller.go_forward().await {
                    Ok(_) => CommandResult::success(serde_json::json!({"went_forward": true})),
                    Err(e) => CommandResult::error(format!("Go forward failed: {}", e)),
                }
            }
            RemoteCommand::Reload => {
                match self.browser_controller.reload().await {
                    Ok(_) => CommandResult::success(serde_json::json!({"reloaded": true})),
                    Err(e) => CommandResult::error(format!("Reload failed: {}", e)),
                }
            }
            RemoteCommand::StopLoading => {
                match self.browser_controller.stop().await {
                    Ok(_) => CommandResult::success(serde_json::json!({"stopped": true})),
                    Err(e) => CommandResult::error(format!("Stop failed: {}", e)),
                }
            }
            RemoteCommand::GetContent => {
                match self.browser_controller.get_content().await {
                    Ok(content) => CommandResult::success(serde_json::json!({"content": content})),
                    Err(e) => CommandResult::error(format!("Get content failed: {}", e)),
                }
            }
            RemoteCommand::GetDom => {
                match self.browser_controller.get_dom().await {
                    Ok(dom) => CommandResult::success(serde_json::json!({"dom": dom})),
                    Err(e) => CommandResult::error(format!("Get DOM failed: {}", e)),
                }
            }
            RemoteCommand::ExecuteJs { script } => {
                match self.browser_controller.execute_js(&script).await {
                    Ok(result) => CommandResult::success(result),
                    Err(e) => CommandResult::error(format!("Execute JS failed: {}", e)),
                }
            }
            RemoteCommand::Click { selector } => {
                match self.browser_controller.click(&selector).await {
                    Ok(_) => CommandResult::success(serde_json::json!({"clicked": true})),
                    Err(e) => CommandResult::error(format!("Click failed: {}", e)),
                }
            }
            RemoteCommand::Type { selector, text } => {
                match self.browser_controller.type_text(&selector, &text).await {
                    Ok(_) => CommandResult::success(serde_json::json!({"typed": true})),
                    Err(e) => CommandResult::error(format!("Type failed: {}", e)),
                }
            }
            RemoteCommand::Screenshot { full_page, .. } => {
                match self.browser_controller.screenshot(full_page).await {
                    Ok(data) => {
                        let base64 = base64::encode(&data);
                        CommandResult::success(serde_json::json!({"screenshot": base64}))
                    }
                    Err(e) => CommandResult::error(format!("Screenshot failed: {}", e)),
                }
            }
            RemoteCommand::GetBrowserInfo => {
                match self.browser_controller.get_browser_info().await {
                    Ok(info) => CommandResult::success(serde_json::to_value(info).unwrap()),
                    Err(e) => CommandResult::error(format!("Get browser info failed: {}", e)),
                }
            }
            RemoteCommand::GetTabs => {
                match self.browser_controller.get_tabs().await {
                    Ok(tabs) => CommandResult::success(serde_json::to_value(tabs).unwrap()),
                    Err(e) => CommandResult::error(format!("Get tabs failed: {}", e)),
                }
            }
            RemoteCommand::NewTab { url } => {
                match self.browser_controller.new_tab(url.as_deref()).await {
                    Ok(tab_id) => CommandResult::success(serde_json::json!({"tab_id": tab_id})),
                    Err(e) => CommandResult::error(format!("New tab failed: {}", e)),
                }
            }
            RemoteCommand::CloseTab { tab_id } => {
                match self.browser_controller.close_tab(tab_id).await {
                    Ok(_) => CommandResult::success(serde_json::json!({"closed": true})),
                    Err(e) => CommandResult::error(format!("Close tab failed: {}", e)),
                }
            }
            RemoteCommand::SwitchTab { tab_id } => {
                match self.browser_controller.switch_tab(tab_id).await {
                    Ok(_) => CommandResult::success(serde_json::json!({"switched": true})),
                    Err(e) => CommandResult::error(format!("Switch tab failed: {}", e)),
                }
            }
            _ => CommandResult::error("Command not implemented".to_string()),
        };

        result.with_execution_time(start.elapsed().as_millis() as u64)
    }
}
