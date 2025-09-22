//! Event schema definitions for the AI-CORE Intelligent Automation Platform
//!
//! This module defines all event types used throughout the platform for
//! event-driven architecture, pub/sub messaging, and workflow coordination.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::core::*;

// =============================================================================
// Base Event Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: Uuid,
    pub event_type: String,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub version: u64,
    pub data: serde_json::Value,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub user_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub source_service: String,
    pub source_version: String,
    pub trace_id: Option<String>,
    pub session_id: Option<String>,
}

// =============================================================================
// User and Authentication Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum UserEvent {
    UserRegistered {
        user_id: Uuid,
        email: String,
        username: String,
        subscription_tier: SubscriptionTier,
    },
    UserLoggedIn {
        user_id: Uuid,
        session_id: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },
    UserLoggedOut {
        user_id: Uuid,
        session_id: String,
        logout_type: LogoutType,
    },
    UserEmailVerified {
        user_id: Uuid,
        email: String,
    },
    UserProfileUpdated {
        user_id: Uuid,
        updated_fields: Vec<String>,
    },
    UserSubscriptionChanged {
        user_id: Uuid,
        from_tier: SubscriptionTier,
        to_tier: SubscriptionTier,
        effective_date: DateTime<Utc>,
    },
    UserStatusChanged {
        user_id: Uuid,
        from_status: UserStatus,
        to_status: UserStatus,
        reason: Option<String>,
    },
    UserDeleted {
        user_id: Uuid,
        deletion_type: DeletionType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogoutType {
    Manual,
    Timeout,
    ForceLogout,
    SecurityBreach,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeletionType {
    SoftDelete,
    HardDelete,
    Anonymization,
}

// =============================================================================
// Workflow Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEvent {
    WorkflowDefinitionCreated {
        workflow_id: Uuid,
        user_id: Uuid,
        workflow_type: WorkflowType,
        name: String,
        step_count: u32,
    },
    WorkflowDefinitionUpdated {
        workflow_id: Uuid,
        user_id: Uuid,
        updated_fields: Vec<String>,
        version: u64,
    },
    WorkflowDefinitionDeleted {
        workflow_id: Uuid,
        user_id: Uuid,
        deletion_reason: String,
    },
    WorkflowExecutionStarted {
        execution_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        temporal_workflow_id: String,
        priority: WorkflowPriority,
        estimated_cost: f64,
        estimated_duration: u64,
    },
    WorkflowStepStarted {
        execution_id: Uuid,
        step_id: Uuid,
        step_name: String,
        step_type: StepType,
        provider_id: Option<String>,
        estimated_cost: f64,
    },
    WorkflowStepCompleted {
        execution_id: Uuid,
        step_id: Uuid,
        step_name: String,
        duration_ms: u64,
        cost_usd: f64,
        output_size_bytes: u32,
        success: bool,
        error_message: Option<String>,
    },
    WorkflowStepRetried {
        execution_id: Uuid,
        step_id: Uuid,
        step_name: String,
        attempt_number: u32,
        error_message: String,
        next_retry_at: DateTime<Utc>,
    },
    WorkflowExecutionPaused {
        execution_id: Uuid,
        user_id: Uuid,
        reason: PauseReason,
        current_step: Option<String>,
    },
    WorkflowExecutionResumed {
        execution_id: Uuid,
        user_id: Uuid,
        resumed_from_step: Option<String>,
    },
    WorkflowExecutionCompleted {
        execution_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        total_duration_ms: u64,
        total_cost_usd: f64,
        success_rate: f32,
        steps_completed: u32,
        steps_failed: u32,
    },
    WorkflowExecutionFailed {
        execution_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        failure_reason: String,
        failed_step: Option<String>,
        total_cost_usd: f64,
        recovery_possible: bool,
    },
    WorkflowExecutionCancelled {
        execution_id: Uuid,
        user_id: Uuid,
        cancellation_reason: String,
        cancelled_by: CancelledBy,
        partial_cost: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PauseReason {
    UserRequested,
    BudgetExceeded,
    ErrorThreshold,
    SystemMaintenance,
    ApprovalRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CancelledBy {
    User,
    System,
    Admin,
    Timeout,
    BudgetLimit,
}

// =============================================================================
// Intent Parsing Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum IntentEvent {
    IntentParseRequested {
        request_id: Uuid,
        user_id: Uuid,
        request_text: String,
        context_size: u32,
        llm_provider: String,
    },
    IntentParseCompleted {
        request_id: Uuid,
        user_id: Uuid,
        workflow_type: WorkflowType,
        confidence_score: f32,
        function_count: u32,
        estimated_cost: f64,
        parse_duration_ms: u64,
        tokens_used: u32,
    },
    IntentParseFailed {
        request_id: Uuid,
        user_id: Uuid,
        error_type: ParseErrorType,
        error_message: String,
        tokens_used: u32,
    },
    LowConfidenceIntentDetected {
        request_id: Uuid,
        user_id: Uuid,
        confidence_score: f32,
        threshold: f32,
        suggested_clarifications: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParseErrorType {
    AmbiguousIntent,
    UnsupportedRequest,
    InsufficientContext,
    LlmServiceError,
    ValidationError,
    BudgetExceeded,
}

// =============================================================================
// MCP and Integration Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum IntegrationEvent {
    McpServerRegistered {
        server_id: String,
        name: String,
        endpoint: String,
        tool_count: u32,
        registering_user: Uuid,
    },
    McpServerHealthCheckFailed {
        server_id: String,
        endpoint: String,
        error_message: String,
        consecutive_failures: u32,
    },
    McpServerDeregistered {
        server_id: String,
        reason: DeregistrationReason,
        final_success_rate: f32,
    },
    McpToolExecutionStarted {
        execution_id: Uuid,
        server_id: String,
        tool_name: String,
        user_id: Uuid,
        workflow_execution_id: Option<Uuid>,
        parameters_size: u32,
    },
    McpToolExecutionCompleted {
        execution_id: Uuid,
        server_id: String,
        tool_name: String,
        duration_ms: u64,
        cost_usd: Option<f64>,
        success: bool,
        response_size: u32,
        error_code: Option<String>,
    },
    ExternalApiCallStarted {
        call_id: Uuid,
        provider: String,
        endpoint: String,
        user_id: Uuid,
        workflow_execution_id: Option<Uuid>,
    },
    ExternalApiCallCompleted {
        call_id: Uuid,
        provider: String,
        endpoint: String,
        status_code: u16,
        duration_ms: u64,
        request_size: u32,
        response_size: u32,
        success: bool,
    },
    WebhookReceived {
        webhook_id: Uuid,
        source: String,
        webhook_event_type: String,
        payload_size: u32,
        signature_valid: bool,
    },
    WebhookProcessed {
        webhook_id: Uuid,
        source: String,
        processing_duration_ms: u64,
        actions_triggered: u32,
        success: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeregistrationReason {
    UserRequested,
    HealthCheckFailures,
    SecurityViolation,
    ContractExpired,
    SystemMaintenance,
}

// =============================================================================
// Federation Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum FederationEvent {
    ClientRegistered {
        client_id: String,
        client_name: String,
        capabilities: Vec<String>,
        mcp_server_count: u32,
        registered_by: Uuid,
    },
    ClientDeregistered {
        client_id: String,
        reason: DeregistrationReason,
        final_success_rate: f32,
        total_requests: u64,
    },
    FederatedWorkflowStarted {
        execution_id: Uuid,
        client_id: String,
        platform_steps: u32,
        client_steps: u32,
        estimated_cost: f64,
    },
    FederatedWorkflowCompleted {
        execution_id: Uuid,
        client_id: String,
        total_duration_ms: u64,
        platform_cost: f64,
        client_cost: f64,
        success: bool,
    },
    CrossClientCollaboration {
        collaboration_id: Uuid,
        participating_clients: Vec<String>,
        workflow_type: WorkflowType,
        initiating_user: Uuid,
    },
    ClientHealthDegraded {
        client_id: String,
        success_rate: f32,
        threshold: f32,
        recent_errors: Vec<String>,
    },
}

// =============================================================================
// Security Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum SecurityEvent {
    LoginAttempt {
        user_id: Option<Uuid>,
        email: String,
        ip_address: String,
        user_agent: String,
        success: bool,
        failure_reason: Option<String>,
    },
    MultipleFailedLogins {
        email: String,
        ip_address: String,
        attempt_count: u32,
        time_window_minutes: u32,
    },
    SuspiciousActivity {
        user_id: Uuid,
        activity_type: SuspiciousActivityType,
        risk_score: f32,
        details: HashMap<String, String>,
    },
    RateLimitExceeded {
        user_id: Option<Uuid>,
        api_key: Option<String>,
        ip_address: String,
        endpoint: String,
        limit_type: RateLimitType,
        current_rate: f64,
        limit_threshold: f64,
    },
    UnauthorizedAccess {
        user_id: Option<Uuid>,
        resource: String,
        action: String,
        ip_address: String,
        required_permission: String,
    },
    TokenExpired {
        user_id: Uuid,
        token_type: TokenType,
        expired_at: DateTime<Utc>,
    },
    SecurityPolicyViolation {
        user_id: Uuid,
        policy: String,
        violation_type: String,
        severity: SecuritySeverity,
    },
    DataExfiltrationAttempt {
        user_id: Uuid,
        data_type: String,
        volume: u64,
        blocked: bool,
        detection_method: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SuspiciousActivityType {
    UnusualLoginLocation,
    RapidApiCalls,
    LargeDataDownload,
    OffHoursActivity,
    PrivilegeEscalation,
    UnusualWorkflowPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitType {
    RequestsPerMinute,
    RequestsPerHour,
    RequestsPerDay,
    BandwidthLimit,
    ConcurrentRequests,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    AccessToken,
    RefreshToken,
    ApiKey,
    SessionToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

// =============================================================================
// System Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
#[serde(rename_all = "snake_case")]
pub enum SystemEvent {
    ServiceStarted {
        service_name: String,
        version: String,
        startup_duration_ms: u64,
    },
    ServiceStopped {
        service_name: String,
        version: String,
        uptime_seconds: u64,
        shutdown_reason: ShutdownReason,
    },
    ServiceHealthDegraded {
        service_name: String,
        health_score: f32,
        threshold: f32,
        affected_components: Vec<String>,
    },
    ServiceHealthRestored {
        service_name: String,
        health_score: f32,
        recovery_duration_ms: u64,
    },
    DatabaseConnectionLost {
        database_type: DatabaseType,
        database_name: String,
        error_message: String,
        retry_count: u32,
    },
    DatabaseConnectionRestored {
        database_type: DatabaseType,
        database_name: String,
        downtime_duration_ms: u64,
    },
    HighResourceUsage {
        service_name: String,
        resource_type: ResourceType,
        current_usage: f64,
        threshold: f64,
        unit: String,
    },
    BackupCompleted {
        backup_type: BackupType,
        database_name: String,
        backup_size_bytes: u64,
        duration_ms: u64,
        success: bool,
    },
    MaintenanceScheduled {
        maintenance_id: String,
        scheduled_start: DateTime<Utc>,
        estimated_duration_minutes: u32,
        affected_services: Vec<String>,
        maintenance_type: MaintenanceType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownReason {
    Graceful,
    Emergency,
    Maintenance,
    Deployment,
    Error,
    OutOfMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseType {
    PostgreSQL,
    ClickHouse,
    MongoDB,
    Redis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
    Network,
    FileDescriptors,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
    TransactionLog,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceType {
    Routine,
    SecurityPatch,
    FeatureDeployment,
    InfrastructureUpgrade,
    Emergency,
}

// =============================================================================
// Event Envelope for Message Queues
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event: DomainEvent,
    pub routing_key: String,
    pub message_id: Uuid,
    pub retry_count: u32,
    pub max_retries: u32,
    pub delay_seconds: Option<u32>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBatch {
    pub batch_id: Uuid,
    pub events: Vec<DomainEvent>,
    pub batch_size: u32,
    pub created_at: DateTime<Utc>,
    pub source_service: String,
}

// =============================================================================
// Event Handlers and Subscriptions
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub subscription_id: Uuid,
    pub event_types: Vec<String>,
    pub handler_endpoint: String,
    pub handler_type: HandlerType,
    pub filter_conditions: Option<serde_json::Value>,
    pub retry_config: RetryConfig,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandlerType {
    Webhook,
    MessageQueue,
    Database,
    Function,
}

// =============================================================================
// Utility Functions
// =============================================================================

impl DomainEvent {
    pub fn new<T>(
        event_type: String,
        aggregate_id: String,
        aggregate_type: String,
        version: u64,
        data: T,
        metadata: EventMetadata,
    ) -> Self
    where
        T: Serialize,
    {
        Self {
            event_id: Uuid::new_v4(),
            event_type,
            aggregate_id,
            aggregate_type,
            version,
            data: serde_json::to_value(data).unwrap(),
            metadata,
            timestamp: Utc::now(),
        }
    }
}

impl EventMetadata {
    pub fn new(source_service: String, source_version: String) -> Self {
        Self {
            user_id: None,
            correlation_id: None,
            causation_id: None,
            source_service,
            source_version,
            trace_id: None,
            session_id: None,
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_causation(mut self, causation_id: Uuid) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    pub fn with_trace(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
}

impl EventEnvelope {
    pub fn new(event: DomainEvent, routing_key: String) -> Self {
        Self {
            event,
            routing_key,
            message_id: Uuid::new_v4(),
            retry_count: 0,
            max_retries: 3,
            delay_seconds: None,
            expires_at: None,
        }
    }

    pub fn with_retry_config(mut self, max_retries: u32, delay_seconds: u32) -> Self {
        self.max_retries = max_retries;
        self.delay_seconds = Some(delay_seconds);
        self
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| exp < Utc::now())
    }
}
