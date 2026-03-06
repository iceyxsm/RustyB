//! AI features placeholder

use serde_json::Value;

/// AI-powered browser features
pub struct AiFeatures;

impl AiFeatures {
    pub async fn summarize_page(&self, _content: &str) -> anyhow::Result<String> {
        Ok("Summary placeholder".to_string())
    }

    pub async fn ask_about_page(&self, _content: &str, _question: &str) -> anyhow::Result<String> {
        Ok("Answer placeholder".to_string())
    }

    pub async fn smart_extract(&self, _html: &str, _description: &str) -> anyhow::Result<Value> {
        Ok(Value::Null)
    }
}
