//! # Event Router
//!
//! The event router provides intelligent routing of webhook events to appropriate
//! processors based on configurable rules, patterns, and load balancing strategies.
//! It supports dynamic routing configuration and real-time routing decisions.

use super::{EventRouter, WebhookEvent};
use crate::error::{IntegrationError, IntegrationResult};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

/// Routing rule types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingRuleType {
    /// Route based on integration type
    Integration,
    /// Route based on event type pattern
    EventType,
    /// Route based on payload content
    PayloadContent,
    /// Route based on webhook headers
    Headers,
    /// Route based on source IP
    SourceIp,
    /// Route based on priority level
    Priority,
    /// Custom routing logic
    Custom,
}

/// Routing condition operators
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Regex,
    In,
    NotIn,
    GreaterThan,
    LessThan,
    Exists,
    NotExists,
}

/// Routing condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    /// Field to evaluate
    pub field: String,
    /// Operator to apply
    pub operator: ConditionOperator,
    /// Value to compare against
    pub value: serde_json::Value,
}

impl RoutingCondition {
    /// Evaluate condition against webhook event
    pub fn evaluate(&self, event: &WebhookEvent) -> bool {
        let field_value = self.extract_field_value(event);

        match self.operator {
            ConditionOperator::Equals => field_value == self.value,
            ConditionOperator::NotEquals => field_value != self.value,
            ConditionOperator::Contains => {
                if let (Some(field_str), Some(value_str)) =
                    (field_value.as_str(), self.value.as_str())
                {
                    field_str.contains(value_str)
                } else {
                    false
                }
            }
            ConditionOperator::NotContains => {
                if let (Some(field_str), Some(value_str)) =
                    (field_value.as_str(), self.value.as_str())
                {
                    !field_str.contains(value_str)
                } else {
                    true
                }
            }
            ConditionOperator::StartsWith => {
                if let (Some(field_str), Some(value_str)) =
                    (field_value.as_str(), self.value.as_str())
                {
                    field_str.starts_with(value_str)
                } else {
                    false
                }
            }
            ConditionOperator::EndsWith => {
                if let (Some(field_str), Some(value_str)) =
                    (field_value.as_str(), self.value.as_str())
                {
                    field_str.ends_with(value_str)
                } else {
                    false
                }
            }
            ConditionOperator::Regex => {
                if let (Some(field_str), Some(pattern_str)) =
                    (field_value.as_str(), self.value.as_str())
                {
                    regex::Regex::new(pattern_str)
                        .map(|re| re.is_match(field_str))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            ConditionOperator::In => {
                if let Some(values) = self.value.as_array() {
                    values.contains(&field_value)
                } else {
                    false
                }
            }
            ConditionOperator::NotIn => {
                if let Some(values) = self.value.as_array() {
                    !values.contains(&field_value)
                } else {
                    true
                }
            }
            ConditionOperator::GreaterThan => self.compare_numbers(&field_value, |a, b| a > b),
            ConditionOperator::LessThan => self.compare_numbers(&field_value, |a, b| a < b),
            ConditionOperator::Exists => !field_value.is_null(),
            ConditionOperator::NotExists => field_value.is_null(),
        }
    }

    fn extract_field_value(&self, event: &WebhookEvent) -> serde_json::Value {
        match self.field.as_str() {
            "integration" => serde_json::Value::String(event.payload.integration.clone()),
            "event_type" => serde_json::Value::String(event.payload.event_type.clone()),
            "priority" => serde_json::Value::String(format!("{:?}", event.priority)),
            "source_ip" => event
                .payload
                .source_ip
                .as_ref()
                .map(|ip| serde_json::Value::String(ip.clone()))
                .unwrap_or(serde_json::Value::Null),
            "user_agent" => event
                .payload
                .user_agent
                .as_ref()
                .map(|ua| serde_json::Value::String(ua.clone()))
                .unwrap_or(serde_json::Value::Null),
            _ => {
                // Check headers
                if self.field.starts_with("headers.") {
                    let header_name = &self.field[8..];
                    event
                        .payload
                        .headers
                        .get(header_name)
                        .map(|v| serde_json::Value::String(v.clone()))
                        .unwrap_or(serde_json::Value::Null)
                } else if self.field.starts_with("data.") {
                    // Navigate payload data using JSON pointer
                    let pointer = &self.field[5..];
                    event
                        .payload
                        .data
                        .pointer(&format!("/{}", pointer.replace('.', "/")))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
        }
    }

    fn compare_numbers<F>(&self, field_value: &serde_json::Value, comparator: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        if let (Some(field_num), Some(value_num)) = (field_value.as_f64(), self.value.as_f64()) {
            comparator(field_num, value_num)
        } else {
            false
        }
    }
}

/// Load balancing strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Route to first available processor
    FirstAvailable,
    /// Round-robin distribution
    RoundRobin,
    /// Random selection
    Random,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Least connections
    LeastConnections,
    /// Hash-based routing (consistent hashing)
    HashBased,
}

/// Routing rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    /// Unique rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule priority (higher numbers processed first)
    pub priority: u32,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Rule type
    pub rule_type: RoutingRuleType,
    /// Conditions that must be met
    pub conditions: Vec<RoutingCondition>,
    /// Target processors
    pub target_processors: Vec<String>,
    /// Load balancing strategy for multiple targets
    pub load_balancing: LoadBalancingStrategy,
    /// Processor weights (for weighted strategies)
    pub processor_weights: HashMap<String, u32>,
    /// Whether to continue processing other rules after this rule matches
    pub continue_on_match: bool,
}

impl RoutingRule {
    /// Check if this rule matches the given event
    pub fn matches(&self, event: &WebhookEvent) -> bool {
        if !self.enabled {
            return false;
        }

        // All conditions must be true for the rule to match
        self.conditions
            .iter()
            .all(|condition| condition.evaluate(event))
    }

    /// Select processor based on load balancing strategy
    pub fn select_processor(&self, routing_stats: &RoutingStats) -> Option<String> {
        if self.target_processors.is_empty() {
            return None;
        }

        match self.load_balancing {
            LoadBalancingStrategy::FirstAvailable => self.target_processors.first().cloned(),
            LoadBalancingStrategy::RoundRobin => {
                let count = routing_stats.get_processor_count(&self.id);
                let index = (count as usize) % self.target_processors.len();
                self.target_processors.get(index).cloned()
            }
            LoadBalancingStrategy::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                self.target_processors.choose(&mut rng).cloned()
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_processor(routing_stats)
            }
            LoadBalancingStrategy::LeastConnections => {
                // Find processor with least active connections
                self.target_processors
                    .iter()
                    .min_by_key(|processor| routing_stats.get_active_connections(processor))
                    .cloned()
            }
            LoadBalancingStrategy::HashBased => {
                // Use consistent hashing based on event ID
                // Need event parameter - using a placeholder hash for now
                let hash = self.calculate_hash("placeholder");
                let index = (hash as usize) % self.target_processors.len();
                self.target_processors.get(index).cloned()
            }
        }
    }

    fn select_weighted_processor(&self, routing_stats: &RoutingStats) -> Option<String> {
        let total_weight: u32 = self.processor_weights.values().sum();
        if total_weight == 0 {
            return self.target_processors.first().cloned();
        }

        let count = routing_stats.get_processor_count(&self.id);
        let position = (count % total_weight as u64) as u32;
        let mut current_weight = 0;

        for processor in &self.target_processors {
            let weight = self.processor_weights.get(processor).unwrap_or(&1);
            current_weight += weight;
            if position < current_weight {
                return Some(processor.clone());
            }
        }

        self.target_processors.first().cloned()
    }

    fn calculate_hash(&self, input: &str) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        hasher.finish() as u32
    }
}

/// Routing statistics
#[derive(Debug, Default)]
pub struct RoutingStats {
    /// Rule usage counters
    rule_counters: HashMap<String, AtomicU64>,
    /// Processor usage counters
    processor_counters: HashMap<String, AtomicU64>,
    /// Active connections per processor
    active_connections: HashMap<String, AtomicU64>,
}

impl RoutingStats {
    pub fn new() -> Self {
        Self {
            rule_counters: HashMap::new(),
            processor_counters: HashMap::new(),
            active_connections: HashMap::new(),
        }
    }

    pub fn increment_rule_counter(&self, rule_id: &str) {
        if let Some(counter) = self.rule_counters.get(rule_id) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn increment_processor_counter(&self, processor: &str) {
        if let Some(counter) = self.processor_counters.get(processor) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn get_processor_count(&self, rule_id: &str) -> u64 {
        self.rule_counters
            .get(rule_id)
            .map(|counter| counter.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    pub fn get_active_connections(&self, processor: &str) -> u64 {
        self.active_connections
            .get(processor)
            .map(|counter| counter.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    pub fn add_rule(&mut self, rule_id: String) {
        self.rule_counters.insert(rule_id, AtomicU64::new(0));
    }

    pub fn add_processor(&mut self, processor: String) {
        self.processor_counters
            .insert(processor.clone(), AtomicU64::new(0));
        self.active_connections.insert(processor, AtomicU64::new(0));
    }
}

/// Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Default processor when no rules match
    pub default_processor: Option<String>,
    /// Whether to log routing decisions
    pub log_routing_decisions: bool,
    /// Maximum number of processors per rule
    pub max_processors_per_rule: usize,
    /// Enable routing metrics collection
    pub enable_metrics: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            default_processor: None,
            log_routing_decisions: true,
            max_processors_per_rule: 10,
            enable_metrics: true,
        }
    }
}

/// Configurable event router implementation
pub struct ConfigurableEventRouter {
    config: RouterConfig,
    rules: Arc<RwLock<Vec<RoutingRule>>>,
    stats: Arc<RwLock<RoutingStats>>,
}

impl ConfigurableEventRouter {
    /// Create a new configurable event router
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            rules: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(RoutingStats::new())),
        }
    }

    /// Add a routing rule
    pub fn add_rule(&self, rule: RoutingRule) -> Result<(), String> {
        if rule.target_processors.len() > self.config.max_processors_per_rule {
            return Err(format!(
                "Rule has too many processors: {} > {}",
                rule.target_processors.len(),
                self.config.max_processors_per_rule
            ));
        }

        let mut rules = self.rules.write();
        let mut stats = self.stats.write();

        // Remove existing rule with same ID
        rules.retain(|r| r.id != rule.id);

        // Add rule stats tracking
        stats.add_rule(rule.id.clone());
        for processor in &rule.target_processors {
            stats.add_processor(processor.clone());
        }

        // Insert rule in priority order (higher priority first)
        let insert_pos = rules
            .iter()
            .position(|r| r.priority < rule.priority)
            .unwrap_or(rules.len());
        rules.insert(insert_pos, rule);

        Ok(())
    }

    /// Remove a routing rule
    pub fn remove_rule(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write();
        let initial_len = rules.len();
        rules.retain(|rule| rule.id != rule_id);
        rules.len() != initial_len
    }

    /// Get all routing rules
    pub fn get_rules(&self) -> Vec<RoutingRule> {
        self.rules.read().clone()
    }

    /// Update rule enabled status
    pub fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> bool {
        let mut rules = self.rules.write();
        if let Some(rule) = rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get routing statistics
    pub fn get_routing_stats(&self) -> HashMap<String, u64> {
        let stats = self.stats.read();
        let mut result = HashMap::new();

        for (rule_id, counter) in &stats.rule_counters {
            result.insert(rule_id.clone(), counter.load(Ordering::SeqCst));
        }

        result
    }

    /// Get processor statistics
    pub fn get_processor_stats(&self) -> HashMap<String, u64> {
        let stats = self.stats.read();
        let mut result = HashMap::new();

        for (processor, counter) in &stats.processor_counters {
            result.insert(processor.clone(), counter.load(Ordering::SeqCst));
        }

        result
    }
}

#[async_trait]
impl EventRouter for ConfigurableEventRouter {
    async fn route_event(&self, event: &WebhookEvent) -> IntegrationResult<Vec<String>> {
        let rules = self.rules.read().clone();
        let mut selected_processors = Vec::new();

        for rule in &rules {
            if rule.matches(event) {
                if self.config.log_routing_decisions {
                    debug!(
                        event_id = %event.id,
                        rule_id = %rule.id,
                        rule_name = %rule.name,
                        "Event matched routing rule"
                    );
                }

                if let Some(processor) = {
                    let stats = self.stats.read();
                    rule.select_processor(&stats)
                } {
                    selected_processors.push(processor.clone());

                    // Update statistics
                    if self.config.enable_metrics {
                        let stats = self.stats.read();
                        stats.increment_rule_counter(&rule.id);
                        stats.increment_processor_counter(&processor);
                    }
                }

                // Stop processing if rule doesn't allow continuation
                if !rule.continue_on_match {
                    break;
                }
            }
        }

        // Use default processor if no rules matched
        if selected_processors.is_empty() {
            if let Some(default_processor) = &self.config.default_processor {
                selected_processors.push(default_processor.clone());

                if self.config.log_routing_decisions {
                    debug!(
                        event_id = %event.id,
                        processor = %default_processor,
                        "Using default processor"
                    );
                }
            } else {
                warn!(
                    event_id = %event.id,
                    "No routing rules matched and no default processor configured"
                );
                return Err(IntegrationError::webhook_processing(
                    "No suitable processor found for event",
                ));
            }
        }

        Ok(selected_processors)
    }

    fn get_routing_config(&self) -> HashMap<String, Vec<String>> {
        let rules = self.rules.read();
        let mut config = HashMap::new();

        for rule in rules.iter() {
            config.insert(rule.id.clone(), rule.target_processors.clone());
        }

        config
    }
}

/// Simple static event router for basic use cases
pub struct StaticEventRouter {
    routing_map: HashMap<String, Vec<String>>,
    default_processors: Vec<String>,
}

impl StaticEventRouter {
    /// Create a new static event router
    pub fn new(routing_map: HashMap<String, Vec<String>>, default_processors: Vec<String>) -> Self {
        Self {
            routing_map,
            default_processors,
        }
    }

    /// Add or update routing for an integration
    pub fn add_routing(&mut self, integration: String, processors: Vec<String>) {
        self.routing_map.insert(integration, processors);
    }

    /// Remove routing for an integration
    pub fn remove_routing(&mut self, integration: &str) -> Option<Vec<String>> {
        self.routing_map.remove(integration)
    }
}

#[async_trait]
impl EventRouter for StaticEventRouter {
    async fn route_event(&self, event: &WebhookEvent) -> IntegrationResult<Vec<String>> {
        // Try to find processors for the integration
        if let Some(processors) = self.routing_map.get(&event.payload.integration) {
            if !processors.is_empty() {
                return Ok(processors.clone());
            }
        }

        // Try to find processors based on event type
        let event_prefix = event.payload.event_type.split('.').next().unwrap_or("");
        if let Some(processors) = self.routing_map.get(event_prefix) {
            if !processors.is_empty() {
                return Ok(processors.clone());
            }
        }

        // Use default processors
        if !self.default_processors.is_empty() {
            Ok(self.default_processors.clone())
        } else {
            Err(IntegrationError::webhook_processing(
                "No routing configuration found",
            ))
        }
    }

    fn get_routing_config(&self) -> HashMap<String, Vec<String>> {
        self.routing_map.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WebhookPayload;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_event(integration: &str, event_type: &str) -> WebhookEvent {
        use uuid::Uuid;

        let payload = WebhookPayload {
            id: Uuid::new_v4(),
            integration: integration.to_string(),
            event_type: event_type.to_string(),
            timestamp: Utc::now(),
            data: json!({"test": "data", "priority": "high"}),
            headers: {
                let mut headers = HashMap::new();
                headers.insert("content-type".to_string(), "application/json".to_string());
                headers.insert("x-webhook-source".to_string(), "test".to_string());
                headers
            },
            source_ip: Some("192.168.1.100".to_string()),
            user_agent: Some("test-agent/1.0".to_string()),
        };
        WebhookEvent::new(payload, super::super::EventPriority::High)
    }

    #[test]
    fn test_routing_condition_evaluation() {
        let event = create_test_event("zapier", "zap.trigger");

        // Test equals condition
        let condition = RoutingCondition {
            field: "integration".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("zapier"),
        };
        assert!(condition.evaluate(&event));

        // Test contains condition
        let condition = RoutingCondition {
            field: "event_type".to_string(),
            operator: ConditionOperator::Contains,
            value: json!("trigger"),
        };
        assert!(condition.evaluate(&event));

        // Test header condition
        let condition = RoutingCondition {
            field: "headers.content-type".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("application/json"),
        };
        assert!(condition.evaluate(&event));

        // Test payload data condition
        let condition = RoutingCondition {
            field: "data.priority".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("high"),
        };
        assert!(condition.evaluate(&event));
    }

    #[test]
    fn test_routing_rule_matching() {
        let event = create_test_event("slack", "message.channels");

        let rule = RoutingRule {
            id: "slack-rule".to_string(),
            name: "Slack Message Rule".to_string(),
            priority: 100,
            enabled: true,
            rule_type: RoutingRuleType::Integration,
            conditions: vec![
                RoutingCondition {
                    field: "integration".to_string(),
                    operator: ConditionOperator::Equals,
                    value: json!("slack"),
                },
                RoutingCondition {
                    field: "event_type".to_string(),
                    operator: ConditionOperator::StartsWith,
                    value: json!("message"),
                },
            ],
            target_processors: vec!["slack-processor".to_string()],
            load_balancing: LoadBalancingStrategy::FirstAvailable,
            processor_weights: HashMap::new(),
            continue_on_match: false,
        };

        assert!(rule.matches(&event));
    }

    #[test]
    fn test_load_balancing_strategies() {
        let stats = RoutingStats::new();

        let rule = RoutingRule {
            id: "test-rule".to_string(),
            name: "Test Rule".to_string(),
            priority: 100,
            enabled: true,
            rule_type: RoutingRuleType::Integration,
            conditions: vec![],
            target_processors: vec![
                "processor-1".to_string(),
                "processor-2".to_string(),
                "processor-3".to_string(),
            ],
            load_balancing: LoadBalancingStrategy::FirstAvailable,
            processor_weights: HashMap::new(),
            continue_on_match: false,
        };

        // Test first available
        let processor = rule.select_processor(&stats).unwrap();
        assert_eq!(processor, "processor-1");

        // Test round robin (would need multiple calls to see rotation)
        let mut rule_rr = rule.clone();
        rule_rr.load_balancing = LoadBalancingStrategy::RoundRobin;
        let processor = rule_rr.select_processor(&stats).unwrap();
        assert!(rule_rr.target_processors.contains(&processor));

        // Test weighted round robin
        let mut rule_weighted = rule.clone();
        rule_weighted.load_balancing = LoadBalancingStrategy::WeightedRoundRobin;
        rule_weighted
            .processor_weights
            .insert("processor-1".to_string(), 3);
        rule_weighted
            .processor_weights
            .insert("processor-2".to_string(), 2);
        rule_weighted
            .processor_weights
            .insert("processor-3".to_string(), 1);

        let processor = rule_weighted.select_processor(&stats).unwrap();
        assert!(rule_weighted.target_processors.contains(&processor));
    }

    #[tokio::test]
    async fn test_configurable_router() {
        let config = RouterConfig::default();
        let router = ConfigurableEventRouter::new(config);

        // Add a routing rule
        let rule = RoutingRule {
            id: "github-rule".to_string(),
            name: "GitHub Events".to_string(),
            priority: 100,
            enabled: true,
            rule_type: RoutingRuleType::Integration,
            conditions: vec![RoutingCondition {
                field: "integration".to_string(),
                operator: ConditionOperator::Equals,
                value: json!("github"),
            }],
            target_processors: vec!["github-processor".to_string()],
            load_balancing: LoadBalancingStrategy::FirstAvailable,
            processor_weights: HashMap::new(),
            continue_on_match: false,
        };

        router.add_rule(rule).unwrap();

        // Test routing
        let event = create_test_event("github", "push");
        let processors = router.route_event(&event).await.unwrap();
        assert_eq!(processors, vec!["github-processor"]);

        // Test rule management
        let rules = router.get_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "github-rule");

        // Test disabling rule
        assert!(router.set_rule_enabled("github-rule", false));

        // Should not match when disabled
        let result = router.route_event(&event).await;
        assert!(result.is_err());

        // Test removing rule
        assert!(router.remove_rule("github-rule"));
        let rules = router.get_rules();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn test_static_router() {
        let mut routing_map = HashMap::new();
        routing_map.insert("zapier".to_string(), vec!["zapier-processor".to_string()]);
        routing_map.insert("slack".to_string(), vec!["slack-processor".to_string()]);

        let default_processors = vec!["default-processor".to_string()];
        let router = StaticEventRouter::new(routing_map, default_processors);

        // Test integration-based routing
        let event = create_test_event("zapier", "zap.trigger");
        let processors = router.route_event(&event).await.unwrap();
        assert_eq!(processors, vec!["zapier-processor"]);

        // Test default routing
        let event = create_test_event("unknown", "unknown.event");
        let processors = router.route_event(&event).await.unwrap();
        assert_eq!(processors, vec!["default-processor"]);
    }

    #[test]
    fn test_routing_stats() {
        let mut stats = RoutingStats::new();
        stats.add_rule("rule-1".to_string());
        stats.add_processor("processor-1".to_string());

        // Test counters
        assert_eq!(stats.get_processor_count("rule-1"), 0);
        stats.increment_rule_counter("rule-1");
        assert_eq!(stats.get_processor_count("rule-1"), 1);

        assert_eq!(stats.get_active_connections("processor-1"), 0);
    }

    #[test]
    fn test_complex_routing_conditions() {
        let event = create_test_event("custom", "data.process");

        // Test regex condition
        let condition = RoutingCondition {
            field: "event_type".to_string(),
            operator: ConditionOperator::Regex,
            value: json!(r"^data\.\w+$"),
        };
        assert!(condition.evaluate(&event));

        // Test in condition
        let condition = RoutingCondition {
            field: "integration".to_string(),
            operator: ConditionOperator::In,
            value: json!(["custom", "zapier", "slack"]),
        };
        assert!(condition.evaluate(&event));

        // Test not in condition
        let condition = RoutingCondition {
            field: "integration".to_string(),
            operator: ConditionOperator::NotIn,
            value: json!(["github", "webhook"]),
        };
        assert!(condition.evaluate(&event));

        // Test exists condition
        let condition = RoutingCondition {
            field: "source_ip".to_string(),
            operator: ConditionOperator::Exists,
            value: json!(null),
        };
        assert!(condition.evaluate(&event));
    }

    #[tokio::test]
    async fn test_rule_priority_ordering() {
        let config = RouterConfig::default();
        let router = ConfigurableEventRouter::new(config);

        // Add rules with different priorities
        let high_priority_rule = RoutingRule {
            id: "high-priority".to_string(),
            name: "High Priority Rule".to_string(),
            priority: 200,
            enabled: true,
            rule_type: RoutingRuleType::Integration,
            conditions: vec![RoutingCondition {
                field: "integration".to_string(),
                operator: ConditionOperator::Equals,
                value: json!("test"),
            }],
            target_processors: vec!["high-priority-processor".to_string()],
            load_balancing: LoadBalancingStrategy::FirstAvailable,
            processor_weights: HashMap::new(),
            continue_on_match: false,
        };

        let low_priority_rule = RoutingRule {
            id: "low-priority".to_string(),
            name: "Low Priority Rule".to_string(),
            priority: 100,
            enabled: true,
            rule_type: RoutingRuleType::Integration,
            conditions: vec![RoutingCondition {
                field: "integration".to_string(),
                operator: ConditionOperator::Equals,
                value: json!("test"),
            }],
            target_processors: vec!["low-priority-processor".to_string()],
            load_balancing: LoadBalancingStrategy::FirstAvailable,
            processor_weights: HashMap::new(),
            continue_on_match: false,
        };

        router.add_rule(low_priority_rule).unwrap();
        router.add_rule(high_priority_rule).unwrap();

        // Test that high priority rule is processed first
        let event = create_test_event("test", "test.event");
        let processors = router.route_event(&event).await.unwrap();
        assert_eq!(processors, vec!["high-priority-processor"]);

        let rules = router.get_rules();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].priority, 200); // High priority first
        assert_eq!(rules[1].priority, 100); // Low priority second
    }
}
