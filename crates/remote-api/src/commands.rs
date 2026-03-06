//! Remote command definitions

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Commands that can be executed remotely
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RemoteCommand {
    /// Navigate to a URL
    Navigate { url: String },
    
    /// Click an element
    Click { selector: String },
    
    /// Type text into an element
    Type { selector: String, text: String },
    
    /// Select an option
    Select { selector: String, value: String },
    
    /// Scroll the page
    Scroll { direction: ScrollDirection, amount: u32 },
    
    /// Take a screenshot
    Screenshot { full_page: bool, selector: Option<String> },
    
    /// Execute JavaScript
    ExecuteJs { script: String },
    
    /// Get page content
    GetContent,
    
    /// Get page DOM
    GetDom,
    
    /// Wait for a duration
    Wait { duration_ms: u64 },
    
    /// Wait for an element
    WaitForElement { selector: String, timeout_ms: u64 },
    
    /// Extract data using a schema
    Extract { schema_id: Uuid },
    
    /// Run an automation script
    RunAutomation { script_id: Uuid },
    
    /// Get browser info
    GetBrowserInfo,
    
    /// Get all tabs
    GetTabs,
    
    /// Create a new tab
    NewTab { url: Option<String> },
    
    /// Close a tab
    CloseTab { tab_id: Uuid },
    
    /// Switch to a tab
    SwitchTab { tab_id: Uuid },
    
    /// Go back in history
    GoBack,
    
    /// Go forward in history
    GoForward,
    
    /// Reload the page
    Reload,
    
    /// Set viewport size
    SetViewport { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Result of executing a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub execution_time_ms: u64,
}

impl CommandResult {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).ok(),
            error: None,
            timestamp: chrono::Utc::now(),
            execution_time_ms: 0,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            timestamp: chrono::Utc::now(),
            execution_time_ms: 0,
        }
    }

    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }
}

/// Command batch for executing multiple commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBatch {
    pub id: Uuid,
    pub commands: Vec<RemoteCommand>,
    pub continue_on_error: bool,
    pub timeout_ms: Option<u64>,
}

impl CommandBatch {
    pub fn new(commands: Vec<RemoteCommand>) -> Self {
        Self {
            id: Uuid::new_v4(),
            commands,
            continue_on_error: false,
            timeout_ms: None,
        }
    }

    pub fn continue_on_error(mut self) -> Self {
        self.continue_on_error = true;
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }
}

/// Browser information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub version: String,
    pub user_agent: String,
    pub viewport: ViewportInfo,
    pub tabs: Vec<TabInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportInfo {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: Uuid,
    pub title: Option<String>,
    pub url: Option<String>,
    pub active: bool,
}
