//! Rusty Browser WebView Subprocess
//! 
//! This binary runs as a separate process to host the WebView.
//! Communication with the main browser process is via JSON-RPC over stdin/stdout.
//!
//! Usage: rusty-browser-webview --ipc-mode

use std::cell::RefCell;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tao::{
    event::{Event, WindowEvent, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use tracing::{debug, error, info, warn};
use wry::WebViewBuilder;

/// Commands received from parent process via stdin
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "method", content = "params")]
enum Command {
    Navigate { url: String },
    Reload,
    GoBack,
    GoForward,
    ExecuteScript { script: String },
    SetBounds { x: i32, y: i32, width: u32, height: u32 },
    Show,
    Hide,
    Close,
}

/// Events sent to parent process via stdout
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event")]
enum EventMsg {
    LoadStarted { url: String },
    LoadFinished { url: String, success: bool },
    UrlChanged { url: String },
    TitleChanged { title: String },
    NavigationRequested { url: String },
    PageError { error: String },
    #[allow(dead_code)]
    ConsoleMessage { level: String, message: String },
    WindowClosed,
}

fn main() -> Result<()> {
    // Initialize logging to stderr (so stdout is clean for JSON)
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(io::stderr)
        .init();

    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] != "--ipc-mode" {
        eprintln!("Usage: rusty-browser-webview --ipc-mode");
        eprintln!("This binary is meant to be run by the main browser process, not directly.");
        std::process::exit(1);
    }

    info!("WebView subprocess starting in IPC mode");

    // Create channels for communication
    let (cmd_tx, cmd_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel();
    let (evt_tx, evt_rx): (Sender<EventMsg>, Receiver<EventMsg>) = mpsc::channel();

    // Spawn stdin reader thread
    let stdin_handle = thread::spawn(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            match line {
                Ok(json) => {
                    debug!("Received from parent: {}", json);
                    match serde_json::from_str::<Command>(&json) {
                        Ok(cmd) => {
                            if cmd_tx.send(cmd).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse command: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }
        info!("Stdin reader thread exiting");
    });

    // Spawn stdout writer thread
    let stdout_handle = thread::spawn(move || {
        let mut stdout = io::stdout();
        loop {
            match evt_rx.recv() {
                Ok(event) => {
                    let json = serde_json::to_string(&event).expect("Failed to serialize event");
                    debug!("Sending to parent: {}", json);
                    if writeln!(stdout, "{}", json).is_err() {
                        break;
                    }
                    if stdout.flush().is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        info!("Stdout writer thread exiting");
    });

    // Run the WebView on the main thread (required for tao/wry)
    let result = run_webview(cmd_rx, evt_tx);

    // Cleanup
    let _ = stdin_handle.join();
    let _ = stdout_handle.join();

    match result {
        Ok(()) => {
            info!("WebView subprocess exiting normally");
            Ok(())
        }
        Err(e) => {
            error!("WebView error: {}", e);
            Err(e)
        }
    }
}

fn run_webview(
    cmd_rx: Receiver<Command>,
    evt_tx: Sender<EventMsg>,
) -> Result<()> {
    info!("Creating WebView window...");

    // Create event loop
    let event_loop = EventLoopBuilder::new().build();

    // Create window
    let window = WindowBuilder::new()
        .with_title("Rusty Browser - WebView")
        .with_inner_size(tao::dpi::LogicalSize::new(1024, 768))
        .with_visible(true)
        .build(&event_loop)
        .context("Failed to create window")?;

    info!("Window created, building WebView...");

    // Create webview with event handlers
    let evt_tx_nav = evt_tx.clone();
    let evt_tx_load = evt_tx.clone();
    let evt_tx_title = evt_tx.clone();

    let webview = WebViewBuilder::new()
        .with_url("about:blank")
        .with_navigation_handler(move |url: String| {
            let _ = evt_tx_nav.send(EventMsg::NavigationRequested { url: url.clone() });
            let _ = evt_tx_nav.send(EventMsg::UrlChanged { url });
            true // Allow navigation
        })
        .with_on_page_load_handler(move |event, url: String| {
            match event {
                wry::PageLoadEvent::Started => {
                    let _ = evt_tx_load.send(EventMsg::LoadStarted { url });
                }
                wry::PageLoadEvent::Finished => {
                    let _ = evt_tx_load.send(EventMsg::LoadFinished { url, success: true });
                }
            }
        })
        .with_document_title_changed_handler(move |title: String| {
            let _ = evt_tx_title.send(EventMsg::TitleChanged { title });
        })
        .build(&window)
        .context("Failed to build WebView")?;

    info!("WebView created successfully");

    // Store webview in Rc<RefCell> for interior mutability in event loop
    let webview = Rc::new(RefCell::new(webview));
    let window = Rc::new(RefCell::new(window));

    // Run event loop
    info!("Starting event loop...");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) | 
            Event::NewEvents(StartCause::Poll) |
            Event::MainEventsCleared => {
                // Process pending commands from stdin
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        Command::Navigate { url } => {
                            info!("Navigating to: {}", url);
                            let wv = webview.borrow();
                            if let Err(e) = wv.load_url(&url) {
                                let _ = evt_tx.send(EventMsg::PageError {
                                    error: format!("Failed to navigate: {}", e),
                                });
                            }
                        }
                        Command::Reload => {
                            info!("Reloading page");
                            let wv = webview.borrow();
                            if let Err(e) = wv.reload() {
                                let _ = evt_tx.send(EventMsg::PageError {
                                    error: format!("Failed to reload: {}", e),
                                });
                            }
                        }
                        Command::GoBack => {
                            let wv = webview.borrow();
                            let _ = wv.evaluate_script("history.back()");
                        }
                        Command::GoForward => {
                            let wv = webview.borrow();
                            let _ = wv.evaluate_script("history.forward()");
                        }
                        Command::ExecuteScript { script } => {
                            let wv = webview.borrow();
                            if let Err(e) = wv.evaluate_script(&script) {
                                let _ = evt_tx.send(EventMsg::PageError {
                                    error: format!("Script error: {}", e),
                                });
                            }
                        }
                        Command::SetBounds { x, y, width, height } => {
                            let window = window.borrow();
                            window.set_outer_position(tao::dpi::LogicalPosition::new(x, y));
                            window.set_inner_size(tao::dpi::LogicalSize::new(width, height));
                        }
                        Command::Show => {
                            let window = window.borrow();
                            window.set_visible(true);
                        }
                        Command::Hide => {
                            let window = window.borrow();
                            window.set_visible(false);
                        }
                        Command::Close => {
                            info!("Close command received");
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("Window close requested");
                let _ = evt_tx.send(EventMsg::WindowClosed);
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
