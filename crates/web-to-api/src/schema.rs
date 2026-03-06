//! Schema definitions for web-to-API conversion

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Schema definition for extracting data from a website
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSchema {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_url: String,
    pub selectors: Vec<FieldSelector>,
    pub pagination: Option<PaginationConfig>,
    pub refresh_interval: Duration,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Field selector for extracting data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSelector {
    pub field_name: String,
    pub selector: String,
    pub selector_type: SelectorType,
    pub attribute: Option<String>,
    pub transform: Option<TransformRule>,
    pub required: bool,
}

/// Type of selector to use
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectorType {
    Css,
    XPath,
    Regex,
    JsonPath,
}

/// Transformation rules for extracted data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransformRule {
    Trim,
    Lowercase,
    Uppercase,
    Replace { pattern: String, replacement: String },
    RegexExtract { pattern: String, group: usize },
    Split { delimiter: String, index: usize },
    Format { template: String },
    Custom { function: String },
}

/// Pagination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub type_: PaginationType,
    pub max_pages: Option<usize>,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PaginationType {
    NextButton { selector: String },
    PageNumbers { selector: String },
    InfiniteScroll { trigger_selector: String },
    UrlPattern { pattern: String, start: usize },
}

impl ExtractionSchema {
    pub fn new(name: impl Into<String>, target_url: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            target_url: target_url.into(),
            selectors: Vec::new(),
            pagination: None,
            refresh_interval: Duration::from_secs(3600), // 1 hour default
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_selector(mut self, selector: FieldSelector) -> Self {
        self.selectors.push(selector);
        self
    }

    pub fn with_pagination(mut self, config: PaginationConfig) -> Self {
        self.pagination = Some(config);
        self
    }

    pub fn with_refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }
}

impl FieldSelector {
    pub fn new(field_name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            selector: selector.into(),
            selector_type: SelectorType::Css,
            attribute: None,
            transform: None,
            required: false,
        }
    }

    pub fn with_type(mut self, selector_type: SelectorType) -> Self {
        self.selector_type = selector_type;
        self
    }

    pub fn with_attribute(mut self, attr: impl Into<String>) -> Self {
        self.attribute = Some(attr.into());
        self
    }

    pub fn with_transform(mut self, transform: TransformRule) -> Self {
        self.transform = Some(transform);
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Schema registry for managing extraction schemas
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: HashMap<Uuid, ExtractionSchema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn register(&mut self, schema: ExtractionSchema) -> Uuid {
        let id = schema.id;
        self.schemas.insert(id, schema);
        id
    }

    pub fn get(&self, id: Uuid) -> Option<&ExtractionSchema> {
        self.schemas.get(&id)
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut ExtractionSchema> {
        self.schemas.get_mut(&id)
    }

    pub fn remove(&mut self, id: Uuid) -> Option<ExtractionSchema> {
        self.schemas.remove(&id)
    }

    pub fn list(&self) -> Vec<&ExtractionSchema> {
        self.schemas.values().collect()
    }

    pub fn update(&mut self, schema: ExtractionSchema) -> bool {
        if self.schemas.contains_key(&schema.id) {
            self.schemas.insert(schema.id, schema);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = ExtractionSchema::new("Test Schema", "https://example.com")
            .with_description("Test description")
            .with_selector(
                FieldSelector::new("title", "h1")
                    .required()
                    .with_transform(TransformRule::Trim),
            );

        assert_eq!(schema.name, "Test Schema");
        assert_eq!(schema.target_url, "https://example.com");
        assert_eq!(schema.selectors.len(), 1);
        assert!(schema.selectors[0].required);
    }

    #[test]
    fn test_schema_registry() {
        let mut registry = SchemaRegistry::new();
        let schema = ExtractionSchema::new("Test", "https://example.com");
        let id = registry.register(schema);

        assert!(registry.get(id).is_some());
        assert_eq!(registry.list().len(), 1);

        registry.remove(id);
        assert!(registry.get(id).is_none());
    }
}
