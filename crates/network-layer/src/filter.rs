//! Request/Response filtering system

use serde::{Deserialize, Serialize};
use shared::{HttpMethod, Request, Response};
use std::collections::HashMap;
use uuid::Uuid;

/// A filter rule for intercepting and modifying requests/responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub condition: FilterCondition,
    pub action: FilterAction,
}

/// Conditions for matching requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterCondition {
    /// Match any request (always true)
    Any,
    
    /// URL contains substring
    UrlContains { pattern: String },
    
    /// URL matches regex
    UrlMatches { regex: String },
    
    /// Exact URL match
    UrlEquals { url: String },
    
    /// Domain matches
    DomainIs { domain: String },
    
    /// Domain ends with
    DomainEndsWith { suffix: String },
    
    /// HTTP method matches
    MethodIs { method: HttpMethod },
    
    /// Header is present
    HeaderPresent { name: String },
    
    /// Header equals value
    HeaderEquals { name: String, value: String },
    
    /// Header contains value
    HeaderContains { name: String, value: String },
    
    /// Content type matches
    ContentTypeIs { content_type: String },
    
    /// Request body contains
    BodyContains { pattern: String },
    
    /// Response status code
    StatusCodeIs { code: u16 },
    
    /// Status code in range
    StatusCodeInRange { min: u16, max: u16 },
    
    /// Combine multiple conditions with AND
    And { conditions: Vec<FilterCondition> },
    
    /// Combine multiple conditions with OR
    Or { conditions: Vec<FilterCondition> },
    
    /// Negate a condition
    Not { condition: Box<FilterCondition> },
}

/// Actions to take when a filter matches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterAction {
    /// Allow the request/response to proceed
    Allow,
    
    /// Block the request/response
    Block { reason: Option<String> },
    
    /// Block and redirect
    Redirect { url: String },
    
    /// Modify the request/response
    Modify { modifications: Vec<Modification> },
    
    /// Log only (don't modify)
    LogOnly,
    
    /// Delay the request/response
    Delay { milliseconds: u64 },
    
    /// Return a custom response
    CustomResponse {
        status_code: u16,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
}

/// Modifications to apply to requests or responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum Modification {
    /// Add a header
    AddHeader { name: String, value: String },
    
    /// Remove a header
    RemoveHeader { name: String },
    
    /// Set/replace a header
    SetHeader { name: String, value: String },
    
    /// Modify the URL
    SetUrl { url: String },
    
    /// Modify request/response body
    SetBody { body: Vec<u8> },
    
    /// Append to body
    AppendBody { data: Vec<u8> },
    
    /// Prepend to body
    PrependBody { data: Vec<u8> },
    
    /// Replace pattern in body
    ReplaceBody {
        pattern: String,
        replacement: String,
    },
}

/// Filter engine that evaluates rules against requests/responses
#[derive(Debug, Default)]
pub struct FilterEngine {
    rules: Vec<FilterRule>,
}

impl FilterEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: FilterRule) {
        // Insert in priority order (higher priority first)
        let pos = self
            .rules
            .binary_search_by(|r| r.priority.cmp(&rule.priority).reverse())
            .unwrap_or_else(|e| e);
        self.rules.insert(pos, rule);
    }

    pub fn remove_rule(&mut self, id: Uuid) -> Option<FilterRule> {
        if let Some(pos) = self.rules.iter().position(|r| r.id == id) {
            Some(self.rules.remove(pos))
        } else {
            None
        }
    }

    pub fn get_rules(&self) -> &[FilterRule] {
        &self.rules
    }

    pub fn get_rule(&self, id: Uuid) -> Option<&FilterRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    pub fn update_rule(&mut self, rule: FilterRule) -> bool {
        if let Some(pos) = self.rules.iter().position(|r| r.id == rule.id) {
            self.rules[pos] = rule;
            true
        } else {
            false
        }
    }

    /// Evaluate a request against all rules
    pub fn evaluate_request(&self, request: &Request) -> FilterResult {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if self.matches_condition(&rule.condition, request, None) {
                return FilterResult {
                    matched_rule: Some(rule.id),
                    action: rule.action.clone(),
                };
            }
        }

        FilterResult {
            matched_rule: None,
            action: FilterAction::Allow,
        }
    }

    /// Evaluate a response against all rules
    pub fn evaluate_response(&self, request: &Request, response: &Response) -> FilterResult {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if self.matches_condition(&rule.condition, request, Some(response)) {
                return FilterResult {
                    matched_rule: Some(rule.id),
                    action: rule.action.clone(),
                };
            }
        }

        FilterResult {
            matched_rule: None,
            action: FilterAction::Allow,
        }
    }

    fn matches_condition(
        &self,
        condition: &FilterCondition,
        request: &Request,
        response: Option<&Response>,
    ) -> bool {
        match condition {
            FilterCondition::Any => true,
            
            FilterCondition::UrlContains { pattern } => {
                request.url.to_lowercase().contains(&pattern.to_lowercase())
            }
            
            FilterCondition::UrlMatches { regex } => {
                regex::Regex::new(regex)
                    .map(|re| re.is_match(&request.url))
                    .unwrap_or(false)
            }
            
            FilterCondition::UrlEquals { url } => request.url == *url,
            
            FilterCondition::DomainIs { domain } => {
                url::Url::parse(&request.url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h == domain))
                    .unwrap_or(false)
            }
            
            FilterCondition::DomainEndsWith { suffix } => {
                url::Url::parse(&request.url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.ends_with(suffix)))
                    .unwrap_or(false)
            }
            
            FilterCondition::MethodIs { method } => request.method == *method,
            
            FilterCondition::HeaderPresent { name } => {
                request.headers.contains_key(&name.to_lowercase())
            }
            
            FilterCondition::HeaderEquals { name, value } => {
                request
                    .headers
                    .get(&name.to_lowercase())
                    .map(|v| v == value)
                    .unwrap_or(false)
            }
            
            FilterCondition::HeaderContains { name, value } => {
                request
                    .headers
                    .get(&name.to_lowercase())
                    .map(|v| v.contains(value))
                    .unwrap_or(false)
            }
            
            FilterCondition::ContentTypeIs { content_type } => {
                request
                    .headers
                    .get("content-type")
                    .map(|ct| ct.contains(content_type))
                    .unwrap_or(false)
            }
            
            FilterCondition::BodyContains { pattern } => {
                request
                    .body
                    .as_ref()
                    .and_then(|b| String::from_utf8(b.clone()).ok())
                    .map(|s| s.contains(pattern))
                    .unwrap_or(false)
            }
            
            FilterCondition::StatusCodeIs { code } => {
                response.map(|r| r.status_code == *code).unwrap_or(false)
            }
            
            FilterCondition::StatusCodeInRange { min, max } => {
                response
                    .map(|r| r.status_code >= *min && r.status_code <= *max)
                    .unwrap_or(false)
            }
            
            FilterCondition::And { conditions } => {
                conditions.iter().all(|c| self.matches_condition(c, request, response))
            }
            
            FilterCondition::Or { conditions } => {
                conditions.iter().any(|c| self.matches_condition(c, request, response))
            }
            
            FilterCondition::Not { condition } => {
                !self.matches_condition(condition, request, response)
            }
        }
    }
}

/// Result of filter evaluation
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub matched_rule: Option<Uuid>,
    pub action: FilterAction,
}

impl FilterRule {
    pub fn new(name: impl Into<String>, condition: FilterCondition, action: FilterAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            enabled: true,
            priority: 0,
            condition,
            action,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_request(url: &str) -> Request {
        Request {
            id: Uuid::new_v4(),
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_url_contains_condition() {
        let engine = FilterEngine::new();
        let request = create_test_request("https://example.com/api/test");

        let condition = FilterCondition::UrlContains {
            pattern: "api".to_string(),
        };

        assert!(engine.matches_condition(&condition, &request, None));
    }

    #[test]
    fn test_domain_is_condition() {
        let engine = FilterEngine::new();
        let request = create_test_request("https://example.com/page");

        let condition = FilterCondition::DomainIs {
            domain: "example.com".to_string(),
        };

        assert!(engine.matches_condition(&condition, &request, None));
    }

    #[test]
    fn test_and_condition() {
        let engine = FilterEngine::new();
        let request = create_test_request("https://api.example.com/test");

        let condition = FilterCondition::And {
            conditions: vec![
                FilterCondition::DomainIs {
                    domain: "api.example.com".to_string(),
                },
                FilterCondition::UrlContains {
                    pattern: "test".to_string(),
                },
            ],
        };

        assert!(engine.matches_condition(&condition, &request, None));
    }
}
