//! Direct WebView2 integration without Tao
//! Uses windows-rs to create WebView2 COM objects directly

use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Arc;
use windows::core::{PCWSTR, HSTRING, Interface};
use windows::w;
use windows::Win32::Foundation::{HWND, RECT, E_FAIL, S_OK};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassW, WS_CHILD, WS_VISIBLE,
    WNDCLASSW, CS_HREDRAW, CS_VREDRAW, MSG, GetMessageW, TranslateMessage, DispatchMessageW,
    WM_SIZE, WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, WM_DESTROY, PostQuitMessage,
};
use webview2_com::{Microsoft::Web::WebView2::Win32::*, *};

pub enum WebViewEvent {
    LoadStarted,
    LoadFinished,
    UrlChanged(String),
    TitleChanged(String),
}

pub enum WebViewCommand {
    Navigate(String),
    Reload,
    GoBack,
    GoForward,
}

/// WebView controller that doesn't use Tao
/// Creates WebView2 directly in an existing window
pub struct DirectWebView {
    controller: Option<ICoreWebView2Controller>,
    webview: Option<ICoreWebView2>,
    event_sender: Sender<WebViewEvent>,
    command_receiver: Receiver<WebViewCommand>,
}

impl DirectWebView {
    /// Create WebView2 embedded in a parent window (from Iced)
    pub fn create_in_window(
        parent_hwnd: isize,
        url: &str,
        event_sender: Sender<WebViewEvent>,
    ) -> anyhow::Result<(Self, Sender<WebViewCommand>)> {
        // Initialize COM
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        }

        let (cmd_tx, cmd_rx) = channel();

        let mut webview = Self {
            controller: None,
            webview: None,
            event_sender,
            command_receiver: cmd_rx,
        };

        webview.initialize(parent_hwnd, url)?;

        Ok((webview, cmd_tx))
    }

    fn initialize(&mut self, parent_hwnd: isize, url: &str) -> anyhow::Result<()> {
        let parent = HWND(parent_hwnd);

        // Create WebView2 environment
        let (tx, rx) = std::sync::mpsc::channel();
        
        let tx_clone = tx.clone();
        let env_options = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |result, environment| {
                if result.is_err() {
                    let _ = tx_clone.send(Err(anyhow::anyhow!("Failed to create WebView2 environment")));
                    return Ok(());
                }
                let _ = tx_clone.send(Ok(environment));
                Ok(())
            },
        ));

        unsafe {
            CreateCoreWebView2Environment(env_options).map_err(|e| anyhow::anyhow!(e))?;
        }

        let environment = rx.recv()?.map_err(|e| anyhow::anyhow!(e))?;

        // Create WebView2 controller
        let tx_clone = tx.clone();
        let env_clone = environment.clone();
        let event_sender = self.event_sender.clone();
        let url_string = url.to_string();

        let controller_created = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |result, controller| {
                if result.is_err() {
                    let _ = tx_clone.send(Err(anyhow::anyhow!("Failed to create controller")));
                    return Ok(());
                }

                let controller = controller.expect("Controller is None");
                let webview = controller.CoreWebView2().expect("Failed to get WebView");

                // Set up navigation handler
                let event_sender_nav = event_sender.clone();
                let nav_handler = WebResourceRequestedEventHandler::create(Box::new(
                    move |_webview, args| {
                        if let Some(args) = args {
                            if let Ok(uri) = args.Request().and_then(|r| r.Uri()) {
                                let uri_string = uri.to_string();
                                let _ = event_sender_nav.send(WebViewEvent::UrlChanged(uri_string));
                            }
                        }
                        Ok(())
                    },
                ));

                // Navigate to initial URL
                let _ = webview.Navigate(&HSTRING::from(&url_string));

                let _ = tx_clone.send(Ok((controller, webview)));
                Ok(())
            },
        ));

        unsafe {
            environment
                .CreateCoreWebView2Controller(parent, controller_created)
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        let (controller, webview) = rx.recv()?.map_err(|e| anyhow::anyhow!(e))?;

        // Set bounds to fill parent
        unsafe {
            let mut rect = RECT::default();
            windows::Win32::UI::WindowsAndMessaging::GetClientRect(parent, &mut rect)?;
            controller.SetBounds(rect)?;
        }

        self.controller = Some(controller);
        self.webview = Some(webview);

        Ok(())
    }

    /// Handle resize - update WebView bounds to match parent
    pub fn resize(&self, width: u32, height: u32) {
        if let Some(controller) = &self.controller {
            unsafe {
                let rect = RECT {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                };
                let _ = controller.SetBounds(rect);
            }
        }
    }

    /// Process any pending commands
    pub fn process_commands(&mut self) {
        while let Ok(cmd) = self.command_receiver.try_recv() {
            if let Some(webview) = &self.webview {
                match cmd {
                    WebViewCommand::Navigate(url) => {
                        let _ = unsafe { webview.Navigate(&HSTRING::from(&url)) };
                    }
                    WebViewCommand::Reload => {
                        let _ = unsafe { webview.Reload() };
                    }
                    WebViewCommand::GoBack => {
                        let _ = unsafe { webview.GoBack() };
                    }
                    WebViewCommand::GoForward => {
                        let _ = unsafe { webview.GoForward() };
                    }
                }
            }
        }
    }
}

// Implement Send/Sync for our handler types
struct CreateCoreWebView2EnvironmentCompletedHandler {
    callback: Box<dyn Fn(windows::core::Result<ICoreWebView2Environment>) -> windows::core::Result<()>>,
}

impl CreateCoreWebView2EnvironmentCompletedHandler {
    fn create<F>(callback: F) -> Self
    where
        F: Fn(windows::core::Result<ICoreWebView2Environment>) -> windows::core::Result<()> + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

// Similar handler for controller creation
struct CreateCoreWebView2ControllerCompletedHandler {
    callback: Box<dyn Fn(windows::core::Result<Option<ICoreWebView2Controller>>) -> windows::core::Result<()>>,
}

impl CreateCoreWebView2ControllerCompletedHandler {
    fn create<F>(callback: F) -> Self
    where
        F: Fn(windows::core::Result<Option<ICoreWebView2Controller>>) -> windows::core::Result<()> + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

struct WebResourceRequestedEventHandler {
    callback: Box<dyn Fn(Option<&ICoreWebView2>, Option<&ICoreWebView2WebResourceRequestedEventArgs>) -> windows::core::Result<()>>,
}

impl WebResourceRequestedEventHandler {
    fn create<F>(callback: F) -> Self
    where
        F: Fn(Option<&ICoreWebView2>, Option<&ICoreWebView2WebResourceRequestedEventArgs>) -> windows::core::Result<()> + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}
