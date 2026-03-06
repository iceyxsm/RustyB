//! Automation scripts

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Automation script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationScript {
    pub id: Uuid,
    pub name: String,
    pub steps: Vec<AutomationStep>,
}

/// Automation step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationStep {
    pub action: ActionType,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, value: String },
    Wait { duration_ms: u64 },
}
