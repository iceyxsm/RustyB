//! Caching system placeholder

use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

/// Cache for extracted data
pub struct Cache;

impl Cache {
    pub async fn get(&self, _schema_id: Uuid) -> Option<Vec<Value>> {
        None
    }

    pub async fn set(&self, _schema_id: Uuid, _data: Vec<Value>, _ttl: Duration) {
    }
}
