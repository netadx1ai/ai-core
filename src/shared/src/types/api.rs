//! API request and response type definitions for the AI-CORE platform
//!
//! This module contains all the HTTP API models used for communication between
//! clients and the various microservices in the platform.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::core::*;

// =============================================================================
// Authentication API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub subscription_tier: Option<SubscriptionTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub user: User,
    pub verification_required: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub session_token: Option<String>,
    pub logout_all_sessions: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

// =============================================================================
// Workflow Management API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub workflow_type: WorkflowType,
    pub steps: Vec<WorkflowStepRequest>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepRequest {
    pub name: String,
    pub step_type: StepType,
    pub function_name: String,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<Uuid>,
    pub retry_config: Option<RetryConfig>,
    pub timeout_seconds: Option<u32>,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowResponse {
    pub workflow: WorkflowDefinition,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub steps: Option<Vec<WorkflowStepRequest>>,
    pub status: Option<WorkflowDefinitionStatus>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteWorkflowRequest {
    pub workflow_id: Uuid,
    pub input_parameters: Option<serde_json::Value>,
    pub priority: Option<WorkflowPriority>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub budget_limit_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteWorkflowResponse {
    pub execution: WorkflowExecution,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkflowsQuery {
    pub status: Option<WorkflowDefinitionStatus>,
    pub workflow_type: Option<WorkflowType>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkflowsResponse {
    pub workflows: Vec<WorkflowDefinition>,
    pub total_count: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatusResponse {
    pub execution: WorkflowExecution,
    pub progress_updates: Vec<ProgressUpdate>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

// =============================================================================
// Automation Request API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAutomationRequest {
    pub request: String,
    pub context: Option<serde_json::Value>,
    pub priority: Option<WorkflowPriority>,
    pub client_id: Option<String>,
    pub preferred_providers: Option<Vec<String>>,
    pub budget_limit_usd: Option<f64>,
    pub deadline: Option<DateTime<Utc>>,
    pub approval_required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAutomationResponse {
    pub workflow_id: Uuid,
    pub execution_id: Uuid,
    pub status: String,
    pub estimated_duration_seconds: u64,
    pub estimated_cost_usd: f64,
    pub steps: Vec<WorkflowStep>,
    pub provider_allocation: Option<ProviderAllocation>,
    pub requires_approval: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseIntentRequest {
    pub text: String,
    pub context: Option<serde_json::Value>,
    pub user_preferences: Option<UserPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseIntentResponse {
    pub intent: ParsedIntent,
    pub alternatives: Vec<ParsedIntent>,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub preferred_providers: Vec<String>,
    pub budget_constraints: Option<BudgetConstraints>,
    pub quality_preferences: QualityPreferences,
    pub notification_preferences: NotificationPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConstraints {
    pub max_cost_per_workflow: f64,
    pub max_monthly_spend: f64,
    pub cost_approval_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPreferences {
    pub prefer_quality_over_speed: bool,
    pub prefer_quality_over_cost: bool,
    pub minimum_success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_notifications: bool,
    pub sms_notifications: bool,
    pub push_notifications: bool,
    pub webhook_notifications: bool,
    pub websocket_notifications: bool,
    pub webhook_url: Option<String>,
    pub notification_frequency: NotificationFrequency,
    pub quiet_hours: Option<QuietHours>,
    pub channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationFrequency {
    RealTime,
    Hourly,
    Daily,
    Weekly,
    OnCompletionOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_time: String, // Format: "HH:MM"
    pub end_time: String,   // Format: "HH:MM"
    pub timezone: String,   // IANA timezone identifier
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    Webhook,
    Websocket,
}

// =============================================================================
// Notification Service API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationRequest {
    pub recipient_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub channels: Vec<NotificationChannel>,
    pub priority: NotificationPriority,
    pub template_id: Option<String>,
    pub template_data: Option<serde_json::Value>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub recipient_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub channels: Vec<NotificationChannel>,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub delivery_attempts: Vec<DeliveryAttempt>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowPaused,
    SystemAlert,
    SecurityAlert,
    AccountUpdate,
    BillingAlert,
    MaintenanceNotice,
    FeatureAnnouncement,
    Custom,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkflowStarted => write!(f, "workflow_started"),
            Self::WorkflowCompleted => write!(f, "workflow_completed"),
            Self::WorkflowFailed => write!(f, "workflow_failed"),
            Self::WorkflowPaused => write!(f, "workflow_paused"),
            Self::SystemAlert => write!(f, "system_alert"),
            Self::SecurityAlert => write!(f, "security_alert"),
            Self::AccountUpdate => write!(f, "account_update"),
            Self::BillingAlert => write!(f, "billing_alert"),
            Self::MaintenanceNotice => write!(f, "maintenance_notice"),
            Self::FeatureAnnouncement => write!(f, "feature_announcement"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Pending,
    Queued,
    Processing,
    Delivered,
    PartiallyDelivered,
    Failed,
    Expired,
    Cancelled,
}

impl std::fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Queued => write!(f, "queued"),
            Self::Processing => write!(f, "processing"),
            Self::Delivered => write!(f, "delivered"),
            Self::PartiallyDelivered => write!(f, "partially_delivered"),
            Self::Failed => write!(f, "failed"),
            Self::Expired => write!(f, "expired"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAttempt {
    pub id: String,
    pub channel: NotificationChannel,
    pub attempted_at: chrono::DateTime<chrono::Utc>,
    pub status: DeliveryStatus,
    pub response: Option<String>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Success,
    Failed,
    Retry,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub notification_type: NotificationType,
    pub channels: Vec<NotificationChannel>,
    pub subject_template: String,
    pub content_template: String,
    pub variables: Vec<TemplateVariable>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub variable_type: VariableType,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Date,
    Url,
    Email,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub notification_type: NotificationType,
    pub channels: Vec<NotificationChannel>,
    pub subject_template: String,
    pub content_template: String,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub channels: Option<Vec<NotificationChannel>>,
    pub subject_template: Option<String>,
    pub content_template: Option<String>,
    pub variables: Option<Vec<TemplateVariable>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubscription {
    pub id: String,
    pub user_id: String,
    pub notification_types: Vec<NotificationType>,
    pub channels: Vec<NotificationChannel>,
    pub preferences: NotificationPreferences,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub notification_types: Vec<NotificationType>,
    pub channels: Vec<NotificationChannel>,
    pub preferences: NotificationPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub notification_types: Option<Vec<NotificationType>>,
    pub channels: Option<Vec<NotificationChannel>>,
    pub preferences: Option<NotificationPreferences>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStats {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub delivery_rate: f32,
    pub average_delivery_time: Option<f32>, // in seconds
    pub channel_stats: std::collections::HashMap<NotificationChannel, ChannelStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub sent: u64,
    pub delivered: u64,
    pub failed: u64,
    pub delivery_rate: f32,
    pub average_delivery_time: Option<f32>, // in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationWebSocketMessage {
    pub message_type: WebSocketMessageType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebSocketMessageType {
    Notification,
    NotificationUpdate,
    DeliveryStatus,
    ConnectionStatus,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkNotificationRequest {
    pub notifications: Vec<CreateNotificationRequest>,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkNotificationResponse {
    pub batch_id: String,
    pub total_notifications: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BulkNotificationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkNotificationResult {
    pub index: usize,
    pub status: BulkOperationStatus,
    pub notification_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BulkOperationStatus {
    Success,
    Failed,
    Skipped,
}

// =============================================================================
// MCP and Integration API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMCPServerRequest {
    pub name: String,
    pub description: String,
    pub endpoint: String,
    pub version: String,
    pub tools: Vec<MCPToolRequest>,
    pub auth_config: AuthConfig,
    pub cost_per_request: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolRequest {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub cost_per_call: Option<f64>,
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMCPServerResponse {
    pub server: McpServer,
    pub registration_token: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMCPServersQuery {
    pub status: Option<McpServerStatus>,
    pub provider_type: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMCPServersResponse {
    pub servers: Vec<McpServer>,
    pub total_count: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteMCPToolRequest {
    pub server_id: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteMCPToolResponse {
    pub result: serde_json::Value,
    pub execution_time_ms: u64,
    pub cost_usd: Option<f64>,
    pub success: bool,
    pub error_message: Option<String>,
}

// =============================================================================
// Federation API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterClientRequest {
    pub client_name: String,
    pub api_endpoint: String,
    pub auth_config: AuthConfig,
    pub mcp_servers: Vec<McpServerInfo>,
    pub capabilities: Vec<String>,
    pub webhook_url: Option<String>,
    pub rate_limits: RateLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterClientResponse {
    pub client: ClientInfo,
    pub client_secret: String,
    pub registration_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedAutomationRequest {
    pub client_id: String,
    pub automation_request: CreateAutomationRequest,
    pub federation_context: FederationContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationContext {
    pub client_capabilities: Vec<String>,
    pub preferred_execution_mode: ExecutionMode,
    pub data_residency_requirements: Option<Vec<String>>,
    pub compliance_requirements: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Hybrid,
    ClientPrimary,
    PlatformPrimary,
    Balanced,
}

// =============================================================================
// Analytics and Reporting API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub metrics: Vec<AnalyticsMetric>,
    pub group_by: Option<Vec<String>>,
    pub filters: Option<HashMap<String, serde_json::Value>>,
    pub aggregation: Option<AggregationType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsMetric {
    WorkflowCount,
    WorkflowSuccessRate,
    AverageExecutionTime,
    TotalCost,
    ApiRequests,
    ErrorRate,
    UserActivity,
    ProviderPerformance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationType {
    Sum,
    Average,
    Count,
    Min,
    Max,
    Percentile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub results: Vec<AnalyticsDataPoint>,
    pub total_count: u64,
    pub summary: AnalyticsSummary,
    pub query_execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsDataPoint {
    pub timestamp: DateTime<Utc>,
    pub dimensions: HashMap<String, String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_workflows: u64,
    pub success_rate: f64,
    pub average_cost_usd: f64,
    pub total_cost_usd: f64,
    pub top_providers: Vec<ProviderSummary>,
    pub trend_analysis: TrendAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub provider_id: String,
    pub usage_count: u64,
    pub success_rate: f64,
    pub average_cost: f64,
    pub average_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub workflow_volume_trend: TrendDirection,
    pub cost_trend: TrendDirection,
    pub performance_trend: TrendDirection,
    pub error_rate_trend: TrendDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Volatile,
}

// =============================================================================
// System Health and Monitoring API Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: HealthStatus,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub checks: HashMap<String, ComponentHealth>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub service_metrics: Vec<PerformanceMetrics>,
    pub system_metrics: SystemMetrics,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_bytes_in: u64,
    pub network_bytes_out: u64,
    pub active_connections: u32,
}

// =============================================================================
// Common Query and Response Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total_count: u64,
    pub page: u32,
    pub per_page: u32,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub success: bool,
    pub message: Option<String>,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ApiError,
    pub success: bool,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorResponse {
    pub errors: Vec<ValidationError>,
    pub success: bool,
    pub message: String,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// WebSocket Message Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum WebSocketMessage {
    WorkflowProgress {
        execution_id: Uuid,
        progress: ProgressUpdate,
    },
    WorkflowCompleted {
        execution_id: Uuid,
        final_status: WorkflowExecutionStatus,
        total_cost: f64,
    },
    SystemAlert {
        alert_type: AlertType,
        message: String,
        severity: AlertSeverity,
    },
    KeepAlive {
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    RateLimitExceeded,
    QuotaExceeded,
    ServiceDegraded,
    MaintenanceScheduled,
    SecurityEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

// =============================================================================
// Default Implementations
// =============================================================================

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Asc
    }
}

impl Default for NotificationFrequency {
    fn default() -> Self {
        NotificationFrequency::RealTime
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Balanced
    }
}

impl Default for AggregationType {
    fn default() -> Self {
        AggregationType::Average
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data,
            success: true,
            message: None,
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            data,
            success: true,
            message: Some(message),
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
        }
    }
}

impl ErrorResponse {
    pub fn new(error: ApiError) -> Self {
        Self {
            error,
            success: false,
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
        }
    }
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total_count: u64, page: u32, per_page: u32) -> Self {
        let has_more = (page * per_page) < total_count as u32;
        Self {
            data,
            total_count,
            page,
            per_page,
            has_more,
        }
    }
}
