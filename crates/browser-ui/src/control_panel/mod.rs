//! Rusty Control Panel - Unified interface for all browser tools
//!
//! This module provides a side panel containing all the browser's powerful tools:
//! - Network: Proxy settings, ad blocker, DNS configuration
//! - AI Engine: LLM chat, RAG queries, embeddings
//! - Automation: Scripts, remote commands, macros
//! - Extraction: Web-to-API schemas, data extraction
//! - DevTools: Inspector, console, network monitor
//! - Settings: General preferences, themes

pub mod tools;
pub mod sidebar;
pub mod panels;

use serde::{Deserialize, Serialize};

/// Categories of tools in the control panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    /// Network tools (proxy, adblock, DNS)
    Network,
    /// AI Engine (LLM, RAG, embeddings)
    AiEngine,
    /// Automation (scripts, commands, macros)
    Automation,
    /// Data extraction (web-to-API, scraping)
    Extraction,
    /// Developer tools (inspector, console)
    DevTools,
    /// Browser settings
    Settings,
}

impl ToolCategory {
    pub fn name(&self) -> &'static str {
        match self {
            ToolCategory::Network => "Network",
            ToolCategory::AiEngine => "AI Engine",
            ToolCategory::Automation => "Automation",
            ToolCategory::Extraction => "Extraction",
            ToolCategory::DevTools => "DevTools",
            ToolCategory::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ToolCategory::Network => "NET",
            ToolCategory::AiEngine => "AI",
            ToolCategory::Automation => "AUTO",
            ToolCategory::Extraction => "DATA",
            ToolCategory::DevTools => "DEV",
            ToolCategory::Settings => "SET",
        }
    }
}

impl Default for ToolCategory {
    fn default() -> Self {
        ToolCategory::Network
    }
}

/// State of the control panel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPanelState {
    /// Whether the panel is expanded
    pub is_expanded: bool,
    /// Current selected category
    pub selected_category: ToolCategory,
    /// Width of the panel when expanded (pixels)
    pub panel_width: u16,
}

impl Default for ControlPanelState {
    fn default() -> Self {
        Self {
            is_expanded: true,
            selected_category: ToolCategory::Network,
            panel_width: 350,
        }
    }
}

impl ControlPanelState {
    pub fn toggle(&mut self) {
        self.is_expanded = !self.is_expanded;
    }

    pub fn select_category(&mut self, category: ToolCategory) {
        self.selected_category = category;
        // Auto-expand when selecting a category
        if !self.is_expanded {
            self.is_expanded = true;
        }
    }
}
