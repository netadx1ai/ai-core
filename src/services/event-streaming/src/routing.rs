//! # Event Routing Module
//!
//! This module provides event routing functionality for the event streaming service.
//! It handles routing events to appropriate destinations based on rules and patterns.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{
    config::Config,
    error::{EventStreamingError, Result},
    events::Event,
    types::EventDestination,
};

/// Event router for determining where events should be sent
#[derive(Clone)]
pub struct EventRouter {
    config: Arc<Config>,
    routing_rules: Arc<Vec<RoutingRule>>,
}

/// Routing rule for determining event destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub name: String,
    pub condition: RoutingCondition,
    pub destination: EventDestination,
    pub priority: u32,
    pub enabled: bool,
}

/// Routing condition for matching events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub event_types: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
    pub tenant_ids: Option<Vec<String>>,
}

impl EventRouter {
    /// Create a new event router
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Initializing Event Router");

        // Default routing rules
        let routing_rules = vec![
            RoutingRule {
                name: "workflow-events".to_string(),
                condition: RoutingCondition {
                    event_types: Some(vec!["workflow.*".to_string()]),
                    categories: None,
                    sources: None,
                    tenant_ids: None,
                },
                destination: EventDestination {
                    target: "kafka:workflow-events".to_string(),
                    routing_key: Some("workflow".to_string()),
                    config: HashMap::new(),
                },
                priority: 100,
                enabled: true,
            },
            RoutingRule {
                name: "system-events".to_string(),
                condition: RoutingCondition {
                    event_types: Some(vec!["system.*".to_string()]),
                    categories: None,
                    sources: None,
                    tenant_ids: None,
                },
                destination: EventDestination {
                    target: "redis:system-events".to_string(),
                    routing_key: Some("system".to_string()),
                    config: HashMap::new(),
                },
                priority: 90,
                enabled: true,
            },
            RoutingRule {
                name: "default-fallback".to_string(),
                condition: RoutingCondition {
                    event_types: None,
                    categories: None,
                    sources: None,
                    tenant_ids: None,
                },
                destination: EventDestination {
                    target: "kafka:all-events".to_string(),
                    routing_key: None,
                    config: HashMap::new(),
                },
                priority: 1,
                enabled: true,
            },
        ];

        Ok(Self {
            config: Arc::new(config.clone()),
            routing_rules: Arc::new(routing_rules),
        })
    }

    /// Route an event to appropriate destinations
    pub async fn route_event(&self, event: &Event) -> Result<Vec<EventDestination>> {
        debug!("Routing event {} of type {}", event.id, event.event_type);

        let mut destinations = Vec::new();
        let mut matched_rules = Vec::new();

        // Find matching rules
        for rule in self.routing_rules.iter() {
            if rule.enabled && self.matches_condition(&rule.condition, event) {
                matched_rules.push(rule.clone());
            }
        }

        // Sort by priority (higher priority first)
        matched_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Add destinations from matching rules
        for rule in matched_rules {
            destinations.push(rule.destination);
        }

        // Ensure at least one destination (fallback)
        if destinations.is_empty() {
            destinations.push(EventDestination {
                target: "kafka:dead-letter".to_string(),
                routing_key: Some("unrouted".to_string()),
                config: HashMap::new(),
            });
        }

        debug!(
            "Event {} routed to {} destinations",
            event.id,
            destinations.len()
        );

        Ok(destinations)
    }

    /// List all available streams
    pub async fn list_streams(&self) -> Result<Vec<String>> {
        let mut streams = Vec::new();

        for rule in self.routing_rules.iter() {
            if let Some(stream) = self.extract_stream_name(&rule.destination.target) {
                if !streams.contains(&stream) {
                    streams.push(stream);
                }
            }
        }

        streams.sort();
        Ok(streams)
    }

    /// Get stream information
    pub async fn get_stream_info(&self, stream_name: &str) -> Result<Option<serde_json::Value>> {
        let mut rules_for_stream = Vec::new();

        for rule in self.routing_rules.iter() {
            if let Some(stream) = self.extract_stream_name(&rule.destination.target) {
                if stream == stream_name {
                    rules_for_stream.push(rule.clone());
                }
            }
        }

        if rules_for_stream.is_empty() {
            return Ok(None);
        }

        Ok(Some(serde_json::json!({
            "name": stream_name,
            "rules": rules_for_stream,
            "rule_count": rules_for_stream.len(),
        })))
    }

    /// Check if event matches routing condition
    fn matches_condition(&self, condition: &RoutingCondition, event: &Event) -> bool {
        // Check event types
        if let Some(event_types) = &condition.event_types {
            let mut matches = false;
            for pattern in event_types {
                if self.matches_pattern(pattern, &event.event_type) {
                    matches = true;
                    break;
                }
            }
            if !matches {
                return false;
            }
        }

        // Check categories
        if let Some(categories) = &condition.categories {
            let category_str = serde_json::to_string(&event.category).unwrap_or_default();
            if !categories.contains(&category_str) {
                return false;
            }
        }

        // Check sources
        if let Some(sources) = &condition.sources {
            if !sources.contains(&event.source.service) {
                return false;
            }
        }

        // Check tenant IDs
        if let Some(tenant_ids) = &condition.tenant_ids {
            match &event.metadata.tenant_id {
                Some(tenant_id) => {
                    if !tenant_ids.contains(tenant_id) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }

    /// Check if a string matches a pattern (supports wildcards)
    fn matches_pattern(&self, pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            return value.starts_with(prefix);
        }

        if pattern.starts_with("*") {
            let suffix = &pattern[1..];
            return value.ends_with(suffix);
        }

        pattern == value
    }

    /// Extract stream name from target
    fn extract_stream_name(&self, target: &str) -> Option<String> {
        if let Some(stream) = target.strip_prefix("kafka:") {
            Some(stream.to_string())
        } else if let Some(stream) = target.strip_prefix("redis:") {
            Some(stream.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::events::{Event, EventPayload};
    use crate::types::{EventCategory, EventSource};

    #[tokio::test]
    async fn test_event_router_creation() {
        let config = Config::default();
        let result = EventRouter::new(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_event_routing() {
        let config = Config::default();
        let router = EventRouter::new(&config).await.unwrap();

        let source = EventSource {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: None,
            hostname: None,
            metadata: std::collections::HashMap::new(),
        };

        let payload = EventPayload::Custom(serde_json::json!({"test": "data"}));
        let event = Event::new("workflow.started", EventCategory::Workflow, source, payload);

        let destinations = router.route_event(&event).await.unwrap();
        assert!(!destinations.is_empty());
    }

    #[test]
    fn test_pattern_matching() {
        let config = Config::default();
        let router = EventRouter::new(&config).await.unwrap();

        assert!(router.matches_pattern("*", "anything"));
        assert!(router.matches_pattern("workflow.*", "workflow.started"));
        assert!(router.matches_pattern("*.error", "system.error"));
        assert!(!router.matches_pattern("workflow.*", "user.login"));
    }

    #[tokio::test]
    async fn test_stream_listing() {
        let config = Config::default();
        let router = EventRouter::new(&config).await.unwrap();

        let streams = router.list_streams().await.unwrap();
        assert!(!streams.is_empty());
    }
}
