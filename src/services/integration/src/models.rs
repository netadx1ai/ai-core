//! Data models for the AI-CORE Integration Service
//!
//! This module defines all data structures used across different integrations
//! including webhook payloads, API responses, and internal event representations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Generic webhook payload wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Unique identifier for the webhook request
    pub id: Uuid,
    /// Integration type (zapier, slack, github)
    pub integration: String,
    /// Event type
    pub event_type: String,
    /// Timestamp when the webhook was received
    pub timestamp: DateTime<Utc>,
    /// Raw payload data
    pub data: Value,
    /// Headers from the webhook request
    pub headers: HashMap<String, String>,
    /// Source IP address
    pub source_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
}

/// Unified integration event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// Integration source
    pub integration: IntegrationType,
    /// Event type
    pub event_type: String,
    /// Event metadata
    pub metadata: EventMetadata,
    /// Event payload
    pub payload: EventPayload,
    /// Processing status
    pub status: EventStatus,
    /// Error message if processing failed
    pub error_message: Option<String>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Integration type enumeration
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationType {
    Zapier,
    Slack,
    GitHub,
}

/// Event processing status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Received,
    Processing,
    Completed,
    Failed,
    Retrying,
}

/// Event metadata containing contextual information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Source identifier (e.g., Zap ID, Slack team ID, GitHub repository)
    pub source_id: String,
    /// User or organization identifier
    pub user_id: Option<String>,
    /// Organization or workspace identifier
    pub organization_id: Option<String>,
    /// Request ID for tracing
    pub request_id: String,
    /// Additional tags
    pub tags: HashMap<String, String>,
}

/// Event payload union for different integration types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    Zapier(ZapierEvent),
    Slack(SlackEvent),
    GitHub(GitHubEvent),
}

/// Zapier-specific event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierEvent {
    /// Zap identifier
    pub zap_id: String,
    /// Zap name
    pub zap_name: Option<String>,
    /// Event name as defined in the Zap
    pub event_name: String,
    /// Trigger data from the Zap
    pub trigger_data: Value,
    /// Custom fields mapping
    pub custom_fields: HashMap<String, Value>,
    /// Zap step information
    pub step_info: Option<ZapStepInfo>,
}

/// Zapier step information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapStepInfo {
    /// Step ID
    pub step_id: String,
    /// Step title
    pub title: String,
    /// App name
    pub app: String,
    /// Step type (trigger, action)
    pub step_type: String,
}

/// Slack-specific event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackEvent {
    /// Event type (message, reaction_added, etc.)
    pub event_type: String,
    /// Team ID
    pub team_id: String,
    /// Channel ID
    pub channel_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Message text
    pub text: Option<String>,
    /// Timestamp
    pub ts: Option<String>,
    /// Thread timestamp
    pub thread_ts: Option<String>,
    /// Event data specific to the event type
    pub event_data: Value,
    /// Bot ID if the event was from a bot
    pub bot_id: Option<String>,
}

/// GitHub-specific event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubEvent {
    /// Event action (opened, closed, etc.)
    pub action: Option<String>,
    /// Repository information
    pub repository: GitHubRepository,
    /// Sender information
    pub sender: GitHubUser,
    /// Installation ID (for GitHub Apps)
    pub installation_id: Option<u64>,
    /// Organization information
    pub organization: Option<GitHubOrganization>,
    /// Event-specific data (PR, issue, etc.)
    pub event_data: Value,
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepository {
    /// Repository ID
    pub id: u64,
    /// Repository name
    pub name: String,
    /// Full name (owner/repo)
    pub full_name: String,
    /// Repository URL
    pub html_url: String,
    /// Default branch
    pub default_branch: String,
    /// Repository visibility
    pub private: bool,
}

/// GitHub user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    /// User ID
    pub id: u64,
    /// Username
    pub login: String,
    /// Avatar URL
    pub avatar_url: String,
    /// User type (User, Bot, etc.)
    #[serde(rename = "type")]
    pub user_type: String,
}

/// GitHub organization information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOrganization {
    /// Organization ID
    pub id: u64,
    /// Organization login
    pub login: String,
    /// Avatar URL
    pub avatar_url: String,
}

/// OAuth token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Token type (Bearer, etc.)
    pub token_type: String,
    /// Token expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Token scopes
    pub scopes: Vec<String>,
    /// User ID associated with the token
    pub user_id: String,
    /// Integration type
    pub integration: IntegrationType,
}

/// Integration configuration for a specific user/organization
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IntegrationConfiguration {
    /// Configuration ID
    pub id: Uuid,
    /// User ID
    pub user_id: String,
    /// Organization ID
    pub organization_id: Option<String>,
    /// Integration type
    pub integration: IntegrationType,
    /// Configuration settings
    pub settings: IntegrationSettings,
    /// OAuth tokens
    pub tokens: Option<OAuthToken>,
    /// Active status
    pub active: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Integration-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum IntegrationSettings {
    Zapier(ZapierSettings),
    Slack(SlackSettings),
    GitHub(GitHubSettings),
}

/// Zapier integration settings
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ZapierSettings {
    /// Webhook URL for receiving Zapier events
    #[validate(url)]
    pub webhook_url: String,
    /// Allowed Zap IDs
    pub allowed_zap_ids: Vec<String>,
    /// Custom field mappings
    pub field_mappings: HashMap<String, String>,
    /// Enable request logging
    pub log_requests: bool,
}

/// Slack integration settings
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SlackSettings {
    /// Slack team/workspace ID
    pub team_id: String,
    /// Bot user ID
    pub bot_user_id: String,
    /// Subscribed event types
    pub event_subscriptions: Vec<String>,
    /// Channel filters
    pub channel_filters: Vec<String>,
    /// User filters
    pub user_filters: Vec<String>,
    /// Enable socket mode
    pub socket_mode: bool,
}

/// GitHub integration settings
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GitHubSettings {
    /// Installation ID
    pub installation_id: u64,
    /// Repository IDs to monitor
    pub repository_ids: Vec<u64>,
    /// Event types to subscribe to
    pub event_types: Vec<String>,
    /// Branch filters
    pub branch_filters: Vec<String>,
    /// Enable issue tracking
    pub issue_tracking: bool,
    /// Enable PR tracking
    pub pr_tracking: bool,
}

/// Workflow trigger request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WorkflowTriggerRequest {
    /// Workflow template ID
    pub workflow_id: String,
    /// Trigger type
    pub trigger_type: String,
    /// Integration event that triggered the workflow
    pub event: IntegrationEvent,
    /// Input parameters for the workflow
    pub parameters: HashMap<String, Value>,
    /// User context
    pub user_context: UserContext,
}

/// User context for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID
    pub user_id: String,
    /// Organization ID
    pub organization_id: Option<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// User preferences
    pub preferences: HashMap<String, Value>,
}

/// API response for webhook endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    /// Request ID for tracking
    pub request_id: String,
    /// Response status
    pub status: String,
    /// Response message
    pub message: String,
    /// Processing timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional data
    pub data: Option<Value>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Overall status
    pub status: HealthStatus,
    /// Integration statuses
    pub integrations: HashMap<String, IntegrationHealth>,
    /// System information
    pub system: SystemHealth,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Integration health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationHealth {
    /// Health status
    pub status: HealthStatus,
    /// Last check timestamp
    pub last_check: DateTime<Utc>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Database connection status
    pub database: bool,
    /// Redis connection status
    pub redis: bool,
    /// Memory usage percentage
    pub memory_usage_percent: f32,
    /// CPU usage percentage
    pub cpu_usage_percent: f32,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Metrics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationMetrics {
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Requests per integration type
    pub requests_by_integration: HashMap<IntegrationType, u64>,
    /// Error counts by type
    pub errors_by_type: HashMap<String, u64>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl Default for WebhookPayload {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            integration: String::new(),
            event_type: String::new(),
            timestamp: Utc::now(),
            data: Value::Null,
            headers: HashMap::new(),
            source_ip: None,
            user_agent: None,
        }
    }
}

impl Default for IntegrationEvent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            integration: IntegrationType::Zapier,
            event_type: String::new(),
            metadata: EventMetadata::default(),
            payload: EventPayload::Zapier(ZapierEvent::default()),
            status: EventStatus::Received,
            error_message: None,
            retry_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            source_id: String::new(),
            user_id: None,
            organization_id: None,
            request_id: Uuid::new_v4().to_string(),
            tags: HashMap::new(),
        }
    }
}

impl Default for ZapierEvent {
    fn default() -> Self {
        Self {
            zap_id: String::new(),
            zap_name: None,
            event_name: String::new(),
            trigger_data: Value::Null,
            custom_fields: HashMap::new(),
            step_info: None,
        }
    }
}

impl WebhookResponse {
    /// Create a successful webhook response
    pub fn success(request_id: String, message: String) -> Self {
        Self {
            request_id,
            status: "success".to_string(),
            message,
            timestamp: Utc::now(),
            data: None,
        }
    }

    /// Create a successful webhook response with data
    pub fn success_with_data(request_id: String, message: String, data: Value) -> Self {
        Self {
            request_id,
            status: "success".to_string(),
            message,
            timestamp: Utc::now(),
            data: Some(data),
        }
    }

    /// Create an error webhook response
    pub fn error(request_id: String, message: String) -> Self {
        Self {
            request_id,
            status: "error".to_string(),
            message,
            timestamp: Utc::now(),
            data: None,
        }
    }
}

impl IntegrationType {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrationType::Zapier => "zapier",
            IntegrationType::Slack => "slack",
            IntegrationType::GitHub => "github",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "zapier" => Ok(IntegrationType::Zapier),
            "slack" => Ok(IntegrationType::Slack),
            "github" => Ok(IntegrationType::GitHub),
            _ => Err(format!("Unknown integration type: {}", s)),
        }
    }
}

impl std::fmt::Display for IntegrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            EventStatus::Received => "received",
            EventStatus::Processing => "processing",
            EventStatus::Completed => "completed",
            EventStatus::Failed => "failed",
            EventStatus::Retrying => "retrying",
        };
        write!(f, "{}", status)
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        };
        write!(f, "{}", status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_type_conversions() {
        assert_eq!(IntegrationType::Zapier.as_str(), "zapier");
        assert_eq!(IntegrationType::Slack.as_str(), "slack");
        assert_eq!(IntegrationType::GitHub.as_str(), "github");

        assert_eq!(
            IntegrationType::from_str("zapier").unwrap(),
            IntegrationType::Zapier
        );
        assert_eq!(
            IntegrationType::from_str("SLACK").unwrap(),
            IntegrationType::Slack
        );
        assert_eq!(
            IntegrationType::from_str("GitHub").unwrap(),
            IntegrationType::GitHub
        );

        assert!(IntegrationType::from_str("invalid").is_err());
    }

    #[test]
    fn test_webhook_response_creation() {
        let success_response =
            WebhookResponse::success("req-123".to_string(), "Processed successfully".to_string());
        assert_eq!(success_response.status, "success");
        assert_eq!(success_response.request_id, "req-123");

        let error_response =
            WebhookResponse::error("req-456".to_string(), "Processing failed".to_string());
        assert_eq!(error_response.status, "error");
        assert_eq!(error_response.request_id, "req-456");
    }

    #[test]
    fn test_default_implementations() {
        let event = IntegrationEvent::default();
        assert_eq!(event.integration, IntegrationType::Zapier);
        assert_eq!(event.status, EventStatus::Received);
        assert_eq!(event.retry_count, 0);

        let metadata = EventMetadata::default();
        assert!(!metadata.request_id.is_empty());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_serialization() {
        let event = IntegrationEvent::default();
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: IntegrationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.integration, deserialized.integration);
        assert_eq!(event.status, deserialized.status);
    }

    #[test]
    fn test_event_status_display() {
        assert_eq!(EventStatus::Received.to_string(), "received");
        assert_eq!(EventStatus::Processing.to_string(), "processing");
        assert_eq!(EventStatus::Completed.to_string(), "completed");
        assert_eq!(EventStatus::Failed.to_string(), "failed");
        assert_eq!(EventStatus::Retrying.to_string(), "retrying");
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }
}
