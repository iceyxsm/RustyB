//! Data extraction engine placeholder

use crate::schema::ExtractionSchema;
use serde_json::Value;

/// Data extractor
pub struct Extractor;

impl Extractor {
    pub async fn extract(&self, _schema: &ExtractionSchema) -> anyhow::Result<Vec<Value>> {
        Ok(vec![])
    }
}
