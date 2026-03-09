//! Tool panels for each category

pub mod network_panel;
pub mod ai_panel;
pub mod automation_panel;
pub mod extraction_panel;
pub mod devtools_panel;
pub mod settings_panel;

pub use network_panel::{NetworkPanel, NetworkMessage};
pub use ai_panel::{AiPanel, AiMessage};
pub use automation_panel::{AutomationPanel, AutomationMessage};
pub use extraction_panel::{ExtractionPanel, ExtractionMessage};
pub use devtools_panel::{DevToolsPanel, DevToolsMessage};
pub use settings_panel::{SettingsPanel, SettingsMessage};
