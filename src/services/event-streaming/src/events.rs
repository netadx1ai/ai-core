//! # Event Structures and Definitions
//!
//! This module defines the core Event structures and payload types for the event streaming service.
//! Events are the fundamental unit of data processed by the streaming system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::types::{
    EventCategory, EventCorrelation, EventDestination, EventPriority, EventSource, EventStatus,
};

/// Core event structure that represents all events in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: Uuid,

    /// Event type identifier
    pub event_type: String,

    /// Event category for classification
    pub category: EventCategory,

    /// Event priority level
    pub priority: EventPriority,

    /// Event source information
    pub source: EventSource,

    /// Event correlation data for tracking
    pub correlation: EventCorrelation,

    /// Event destinations
    pub destinations: Vec<EventDestination>,

    /// Event payload data
    pub payload: EventPayload,

    /// Event metadata
    pub metadata: EventMetadata,

    /// Current processing status
    pub status: EventStatus,

    /// Event creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Event expiration timestamp (optional)
    pub expires_at: Option<DateTime<Utc>>,

    /// Processing attempt count
    pub attempt_count: u32,

    /// Error information if processing failed
    pub error: Option<EventError>,

    /// Processing history
    pub processing_history: Vec<ProcessingEvent>,
}

impl Event {
    /// Create a new event with the given parameters
    pub fn new(
        event_type: impl Into<String>,
        category: EventCategory,
        source: EventSource,
        payload: EventPayload,
    ) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let correlation_id = Uuid::new_v4();

        Self {
            id,
            event_type: event_type.into(),
            category,
            priority: EventPriority::Normal,
            source,
            correlation: EventCorrelation {
                correlation_id,
                causation_id: None,
                parent_event_id: None,
                trace_id: None,
                span_id: None,
            },
            destinations: Vec::new(),
            payload,
            metadata: EventMetadata::default(),
            status: EventStatus::Pending,
            created_at: now,
            updated_at: now,
            expires_at: None,
            attempt_count: 0,
            error: None,
            processing_history: Vec::new(),
        }
    }

    /// Create a workflow event
    pub fn workflow_event(
        workflow_id: Uuid,
        action: impl Into<String>,
        source: EventSource,
        data: serde_json::Value,
    ) -> Self {
        let payload = EventPayload::Workflow(WorkflowEventPayload {
            workflow_id,
            action: action.into(),
            data,
            step_id: None,
            user_id: None,
        });

        Self::new("workflow.action", EventCategory::Workflow, source, payload)
    }

    /// Create a system event
    pub fn system_event(
        component: impl Into<String>,
        action: impl Into<String>,
        source: EventSource,
        data: serde_json::Value,
    ) -> Self {
        let payload = EventPayload::System(SystemEventPayload {
            component: component.into(),
            action: action.into(),
            data,
            severity: SystemEventSeverity::Info,
            resource_id: None,
        });

        Self::new("system.action", EventCategory::System, source, payload)
    }

    /// Create a user activity event
    pub fn user_activity_event(
        user_id: Uuid,
        action: impl Into<String>,
        source: EventSource,
        data: serde_json::Value,
    ) -> Self {
        let payload = EventPayload::UserActivity(UserActivityEventPayload {
            user_id,
            action: action.into(),
            data,
            session_id: None,
            ip_address: None,
            user_agent: None,
        });

        Self::new(
            "user.activity",
            EventCategory::UserActivity,
            source,
            payload,
        )
    }

    /// Add a destination to the event
    pub fn add_destination(&mut self, destination: EventDestination) {
        self.destinations.push(destination);
    }

    /// Update event status and add processing history
    pub fn update_status(&mut self, status: EventStatus, message: Option<String>) {
        self.status = status.clone();
        self.updated_at = Utc::now();

        self.processing_history.push(ProcessingEvent {
            timestamp: self.updated_at,
            status,
            message,
            processor: None,
            duration_ms: None,
        });
    }

    /// Mark event as failed with error information
    pub fn mark_failed(&mut self, error: EventError) {
        self.status = EventStatus::Failed;
        self.error = Some(error);
        self.attempt_count += 1;
        self.updated_at = Utc::now();
    }

    /// Check if event should be retried
    pub fn should_retry(&self, max_attempts: u32) -> bool {
        matches!(self.status, EventStatus::Failed) && self.attempt_count < max_attempts
    }

    /// Check if event has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires| Utc::now() > expires)
            .unwrap_or(false)
    }

    /// Get event age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Get processing duration in milliseconds
    pub fn processing_duration_ms(&self) -> Option<i64> {
        self.processing_history
            .iter()
            .find(|h| matches!(h.status, EventStatus::Completed))
            .and_then(|h| h.duration_ms)
    }
}

/// Event payload that can contain different types of event data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    /// Workflow-related event data
    Workflow(WorkflowEventPayload),

    /// System operation event data
    System(SystemEventPayload),

    /// User activity event data
    UserActivity(UserActivityEventPayload),

    /// Security event data
    Security(SecurityEventPayload),

    /// Integration event data
    Integration(IntegrationEventPayload),

    /// Data processing event data
    DataProcessing(DataProcessingEventPayload),

    /// Notification event data
    Notification(NotificationEventPayload),

    /// Error event data
    Error(ErrorEventPayload),

    /// Audit event data
    Audit(AuditEventPayload),

    /// Custom event data
    Custom(serde_json::Value),
}

/// Workflow event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEventPayload {
    /// Workflow identifier
    pub workflow_id: Uuid,

    /// Workflow action or step
    pub action: String,

    /// Workflow-specific data
    pub data: serde_json::Value,

    /// Workflow step identifier (optional)
    pub step_id: Option<String>,

    /// User who triggered the workflow (optional)
    pub user_id: Option<Uuid>,
}

/// System event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemEventPayload {
    /// System component name
    pub component: String,

    /// System action or operation
    pub action: String,

    /// System event data
    pub data: serde_json::Value,

    /// Event severity level
    pub severity: SystemEventSeverity,

    /// Resource identifier (optional)
    pub resource_id: Option<String>,
}

/// System event severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// User activity event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserActivityEventPayload {
    /// User identifier
    pub user_id: Uuid,

    /// User action
    pub action: String,

    /// Activity-specific data
    pub data: serde_json::Value,

    /// Session identifier (optional)
    pub session_id: Option<String>,

    /// User IP address (optional)
    pub ip_address: Option<String>,

    /// User agent (optional)
    pub user_agent: Option<String>,
}

/// Security event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityEventPayload {
    /// Security event type
    pub security_event_type: String,

    /// User involved (optional)
    pub user_id: Option<Uuid>,

    /// Resource accessed
    pub resource: String,

    /// Action attempted
    pub action: String,

    /// Event outcome
    pub outcome: SecurityEventOutcome,

    /// Security-specific data
    pub data: serde_json::Value,
}

/// Security event outcomes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventOutcome {
    Success,
    Failure,
    Blocked,
    Suspicious,
}

/// Integration event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationEventPayload {
    /// External service name
    pub service: String,

    /// Integration operation
    pub operation: String,

    /// Request/response data
    pub data: serde_json::Value,

    /// Integration status
    pub status: IntegrationStatus,

    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
}

/// Integration status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationStatus {
    Success,
    Failure,
    Timeout,
    RateLimited,
}

/// Data processing event payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataProcessingEventPayload {
    /// Processing job identifier
    pub job_id: Uuid,

    /// Processing operation
    pub operation: String,

    /// Input data information
    pub input_info: DataInfo,

    /// Output data information (optional)
    pub output_info: Option<DataInfo>,

    /// Processing metrics
    pub metrics: ProcessingMetrics,

    /// Processing-specific data
    pub data: serde_json::Value,
}

/// Data information structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataInfo {
    /// Data type
    pub data_type: String,

    /// Data size in bytes
    pub size_bytes: u64,

    /// Number of records
    pub record_count: Option<u64>,

    /// Data source
    pub source: String,
}

/// Processing metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingMetrics {
    /// Processing start time
    pub start_time: DateTime<Utc>,

    /// Processing end time (optional)
    pub end_time: Option<DateTime<Utc>>,

    /// Records processed
    pub records_processed: u64,

    /// Records failed
    pub records_failed: u64,

    /// Processing rate (records per second)
    pub processing_rate: f64,
}

/// Notification event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationEventPayload {
    /// Notification channel
    pub channel: NotificationChannel,

    /// Recipient information
    pub recipient: String,

    /// Notification subject
    pub subject: String,

    /// Notification content
    pub content: String,

    /// Notification status
    pub status: NotificationStatus,

    /// Delivery attempt count
    pub delivery_attempts: u32,
}

/// Notification channels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    Webhook,
    Slack,
    Teams,
}

/// Notification status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
    Bounced,
}

/// Error event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEventPayload {
    /// Error type
    pub error_type: String,

    /// Error message
    pub message: String,

    /// Error code (optional)
    pub code: Option<String>,

    /// Stack trace (optional)
    pub stack_trace: Option<String>,

    /// Context where error occurred
    pub context: serde_json::Value,

    /// User involved (optional)
    pub user_id: Option<Uuid>,

    /// Request identifier (optional)
    pub request_id: Option<String>,
}

/// Audit event payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEventPayload {
    /// Actor who performed the action
    pub actor: AuditActor,

    /// Action performed
    pub action: String,

    /// Resource affected
    pub resource: AuditResource,

    /// Audit outcome
    pub outcome: AuditOutcome,

    /// Additional audit data
    pub data: serde_json::Value,
}

/// Audit actor information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditActor {
    /// Actor identifier
    pub id: String,

    /// Actor type (user, service, system)
    pub actor_type: String,

    /// Actor name
    pub name: String,

    /// Actor roles
    pub roles: Vec<String>,
}

/// Audit resource information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditResource {
    /// Resource identifier
    pub id: String,

    /// Resource type
    pub resource_type: String,

    /// Resource name
    pub name: String,

    /// Resource attributes
    pub attributes: HashMap<String, String>,
}

/// Audit event outcomes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    Unknown,
}

/// Event metadata for additional information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Custom tags for event classification
    pub tags: Vec<String>,

    /// Additional custom properties
    pub properties: HashMap<String, String>,

    /// Event schema version
    pub schema_version: String,

    /// Tenant or organization identifier
    pub tenant_id: Option<String>,

    /// Environment (dev, staging, prod)
    pub environment: Option<String>,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            properties: HashMap::new(),
            schema_version: "1.0.0".to_string(),
            tenant_id: None,
            environment: None,
        }
    }
}

/// Event error information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventError {
    /// Error type
    pub error_type: String,

    /// Error message
    pub message: String,

    /// Error code (optional)
    pub code: Option<String>,

    /// Retry after seconds (optional)
    pub retry_after: Option<u64>,

    /// Whether error is retryable
    pub retryable: bool,

    /// Error occurred at timestamp
    pub occurred_at: DateTime<Utc>,
}

/// Processing event for history tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessingEvent {
    /// Processing timestamp
    pub timestamp: DateTime<Utc>,

    /// Processing status
    pub status: EventStatus,

    /// Processing message (optional)
    pub message: Option<String>,

    /// Processor identifier (optional)
    pub processor: Option<String>,

    /// Processing duration in milliseconds (optional)
    pub duration_ms: Option<i64>,
}

/// Event type enumeration for common event types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    // Workflow events
    WorkflowCreated,
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowPaused,
    WorkflowResumed,
    WorkflowCancelled,

    // System events
    ServiceStarted,
    ServiceStopped,
    ServiceHealthCheck,
    ResourceCreated,
    ResourceUpdated,
    ResourceDeleted,

    // User events
    UserLoggedIn,
    UserLoggedOut,
    UserRegistered,
    UserProfileUpdated,
    UserActionPerformed,

    // Security events
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationFailure,
    SecurityThreatDetected,

    // Integration events
    ExternalServiceCall,
    WebhookReceived,
    DataSynchronized,

    // Custom event type
    Custom(String),
}

impl EventType {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            EventType::WorkflowCreated => "workflow.created",
            EventType::WorkflowStarted => "workflow.started",
            EventType::WorkflowCompleted => "workflow.completed",
            EventType::WorkflowFailed => "workflow.failed",
            EventType::WorkflowPaused => "workflow.paused",
            EventType::WorkflowResumed => "workflow.resumed",
            EventType::WorkflowCancelled => "workflow.cancelled",
            EventType::ServiceStarted => "service.started",
            EventType::ServiceStopped => "service.stopped",
            EventType::ServiceHealthCheck => "service.health_check",
            EventType::ResourceCreated => "resource.created",
            EventType::ResourceUpdated => "resource.updated",
            EventType::ResourceDeleted => "resource.deleted",
            EventType::UserLoggedIn => "user.logged_in",
            EventType::UserLoggedOut => "user.logged_out",
            EventType::UserRegistered => "user.registered",
            EventType::UserProfileUpdated => "user.profile_updated",
            EventType::UserActionPerformed => "user.action_performed",
            EventType::AuthenticationSuccess => "auth.success",
            EventType::AuthenticationFailure => "auth.failure",
            EventType::AuthorizationFailure => "auth.authorization_failure",
            EventType::SecurityThreatDetected => "security.threat_detected",
            EventType::ExternalServiceCall => "integration.service_call",
            EventType::WebhookReceived => "integration.webhook_received",
            EventType::DataSynchronized => "integration.data_synchronized",
            EventType::Custom(ref name) => name,
        }
    }
}

impl From<EventType> for String {
    fn from(event_type: EventType) -> Self {
        event_type.as_str().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let source = EventSource {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: None,
            hostname: None,
            metadata: HashMap::new(),
        };

        let payload = EventPayload::Custom(serde_json::json!({
            "test": "data"
        }));

        let event = Event::new("test.event", EventCategory::System, source, payload);

        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.category, EventCategory::System);
        assert_eq!(event.status, EventStatus::Pending);
        assert_eq!(event.attempt_count, 0);
    }

    #[test]
    fn test_workflow_event_creation() {
        let source = EventSource {
            service: "workflow-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: None,
            hostname: None,
            metadata: HashMap::new(),
        };

        let workflow_id = Uuid::new_v4();
        let data = serde_json::json!({"step": "validation"});

        let event = Event::workflow_event(workflow_id, "start", source, data);

        assert_eq!(event.event_type, "workflow.action");
        assert_eq!(event.category, EventCategory::Workflow);

        if let EventPayload::Workflow(payload) = &event.payload {
            assert_eq!(payload.workflow_id, workflow_id);
            assert_eq!(payload.action, "start");
        } else {
            panic!("Expected workflow payload");
        }
    }

    #[test]
    fn test_event_status_update() {
        let mut event = create_test_event();

        event.update_status(
            EventStatus::Processing,
            Some("Started processing".to_string()),
        );

        assert_eq!(event.status, EventStatus::Processing);
        assert_eq!(event.processing_history.len(), 1);
        assert_eq!(
            event.processing_history[0].message,
            Some("Started processing".to_string())
        );
    }

    #[test]
    fn test_event_retry_logic() {
        let mut event = create_test_event();

        // Event should not retry when pending
        assert!(!event.should_retry(3));

        // Mark as failed
        let error = EventError {
            error_type: "processing_error".to_string(),
            message: "Test error".to_string(),
            code: None,
            retry_after: None,
            retryable: true,
            occurred_at: Utc::now(),
        };

        event.mark_failed(error);

        assert_eq!(event.status, EventStatus::Failed);
        assert_eq!(event.attempt_count, 1);
        assert!(event.should_retry(3));

        // After max attempts, should not retry
        event.attempt_count = 3;
        assert!(!event.should_retry(3));
    }

    #[test]
    fn test_event_expiration() {
        let mut event = create_test_event();

        // Event without expiration should not be expired
        assert!(!event.is_expired());

        // Set expiration in the past
        event.expires_at = Some(Utc::now() - chrono::Duration::minutes(1));
        assert!(event.is_expired());

        // Set expiration in the future
        event.expires_at = Some(Utc::now() + chrono::Duration::minutes(1));
        assert!(!event.is_expired());
    }

    #[test]
    fn test_event_type_string_conversion() {
        assert_eq!(EventType::WorkflowCreated.as_str(), "workflow.created");
        assert_eq!(EventType::UserLoggedIn.as_str(), "user.logged_in");

        let custom = EventType::Custom("my.custom.event".to_string());
        assert_eq!(custom.as_str(), "my.custom.event");

        let event_type_str: String = EventType::WorkflowStarted.into();
        assert_eq!(event_type_str, "workflow.started");
    }

    fn create_test_event() -> Event {
        let source = EventSource {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: None,
            hostname: None,
            metadata: HashMap::new(),
        };

        let payload = EventPayload::Custom(serde_json::json!({"test": "data"}));

        Event::new("test.event", EventCategory::System, source, payload)
    }
}
