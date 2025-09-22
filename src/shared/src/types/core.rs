//! Core type definitions for the AI-CORE Intelligent Automation Platform
//!
//! This module contains shared types used across all microservices to ensure
//! consistency and type safety throughout the platform.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

// ============================================================================
// USER AND AUTHENTICATION TYPES
// ============================================================================

/// User subscription tier levels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Pro,
    Enterprise,
}

impl Default for SubscriptionTier {
    fn default() -> Self {
        SubscriptionTier::Free
    }
}

impl FromStr for SubscriptionTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(SubscriptionTier::Free),
            "pro" => Ok(SubscriptionTier::Pro),
            "enterprise" => Ok(SubscriptionTier::Enterprise),
            _ => Err(format!("Invalid subscription tier: {}", s)),
        }
    }
}

/// User account status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus::Active
    }
}

/// Core user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub is_active: bool,
    pub subscription_tier: SubscriptionTier,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub totp_secret: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub preferences: Option<serde_json::Value>,
}

/// Authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthCredentials {
    EmailPassword { email: String, password: String },
    ApiKey { api_key: String },
    RefreshToken { refresh_token: String },
}

/// JWT token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String, // User ID
    pub iss: String, // Issuer
    pub aud: String, // Audience
    pub exp: i64,    // Expiration timestamp
    pub iat: i64,    // Issued at timestamp
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub subscription_tier: SubscriptionTier,
}

/// API key permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Workflow permissions
    WorkflowsRead,
    WorkflowsCreate,
    WorkflowsUpdate,
    WorkflowsDelete,

    // Content permissions
    ContentRead,
    ContentCreate,
    ContentUpdate,
    ContentDelete,

    // Campaign permissions
    CampaignsRead,
    CampaignsCreate,
    CampaignsUpdate,
    CampaignsDelete,

    // Analytics permissions
    AnalyticsRead,
    AnalyticsExport,

    // Federation permissions
    FederationProxy,
    FederationManage,

    // Admin permissions
    AdminUsers,
    AdminSystem,
    AdminBilling,
}

// ============================================================================
// WORKFLOW TYPES
// ============================================================================

/// Workflow execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Created,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

impl Default for WorkflowStatus {
    fn default() -> Self {
        WorkflowStatus::Created
    }
}

/// Workflow trigger types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowTrigger {
    /// Manual trigger - user initiates workflow
    Manual,

    /// Schedule trigger - runs on a schedule
    Schedule {
        /// Cron expression for scheduling
        cron: String,
        /// Timezone for the schedule
        timezone: Option<String>,
    },

    /// Webhook trigger - external HTTP request
    Webhook {
        /// Webhook URL path
        path: String,
        /// Required HTTP method
        method: String,
        /// Authentication requirements
        auth_required: bool,
    },

    /// Event trigger - system or external event
    Event {
        /// Event source
        source: String,
        /// Event type
        event_type: String,
        /// Event filters
        filters: HashMap<String, serde_json::Value>,
    },

    /// File trigger - file system changes
    File {
        /// File path or pattern
        path: String,
        /// Type of file event (created, modified, deleted)
        event: String,
    },

    /// Email trigger - incoming email
    Email {
        /// Email address to monitor
        address: String,
        /// Subject line filters
        subject_filters: Option<Vec<String>>,
        /// Sender filters
        sender_filters: Option<Vec<String>>,
    },
}

impl Default for WorkflowTrigger {
    fn default() -> Self {
        WorkflowTrigger::Manual
    }
}

/// Workflow priority levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl Default for WorkflowPriority {
    fn default() -> Self {
        WorkflowPriority::Medium
    }
}

/// Workflow types supported by the platform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    ContentGeneration,
    SocialMedia,
    EmailMarketing,
    DataAnalysis,
    Automation,
}

/// Core workflow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workflow_type: WorkflowType,
    pub status: WorkflowStatus,
    pub priority: WorkflowPriority,
    pub estimated_cost_cents: Option<i32>,
    pub actual_cost_cents: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub actual_duration_seconds: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CreateWorkflowRequest {
    NaturalLanguage {
        request: String,
        priority: Option<WorkflowPriority>,
        max_cost_usd: Option<f64>,
        schedule_time: Option<DateTime<Utc>>,
        context: Option<HashMap<String, serde_json::Value>>,
    },
    Structured {
        workflow_type: WorkflowType,
        parameters: HashMap<String, serde_json::Value>,
        priority: Option<WorkflowPriority>,
        schedule_time: Option<DateTime<Utc>>,
        max_cost_usd: Option<f64>,
    },
}

/// Workflow step status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Individual workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: String,
    pub status: StepStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub cost_usd: Option<f64>,
    pub retry_count: i32,
    pub error_message: Option<String>,
    pub results: Option<HashMap<String, serde_json::Value>>,
}

/// Workflow progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgress {
    pub workflow_id: Uuid,
    pub status: WorkflowStatus,
    pub progress_percent: f32,
    pub current_step: Option<String>,
    pub current_step_progress: Option<f32>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub elapsed_time_seconds: Option<i64>,
    pub cost_so_far_usd: Option<f64>,
    pub steps_completed: i32,
    pub total_steps: i32,
    pub last_updated: DateTime<Utc>,
}

// ============================================================================
// CONTENT TYPES
// ============================================================================

/// Content types supported by the platform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Blog,
    SocialPost,
    Image,
    Video,
    Infographic,
    Carousel,
    Story,
    Reel,
    Email,
    LandingPage,
}

/// Content status in the workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus {
    Draft,
    PendingReview,
    Approved,
    Published,
    Archived,
    Rejected,
}

/// AI-generated content metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMetadata {
    pub model_used: String,
    pub prompt_version: String,
    pub generation_params: HashMap<String, serde_json::Value>,
    pub quality_score: f64,
    pub confidence_score: f64,
    pub content_category: String,
    pub sentiment: ContentSentiment,
    pub readability_score: f64,
}

/// Content sentiment analysis
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentSentiment {
    Positive,
    Neutral,
    Negative,
}

/// SEO metadata for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoMetadata {
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub keywords: Vec<String>,
    pub slug: Option<String>,
    pub canonical_url: Option<String>,
}

/// Content performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetrics {
    pub impressions: Option<i64>,
    pub reach: Option<i64>,
    pub clicks: Option<i64>,
    pub shares: Option<i64>,
    pub likes: Option<i64>,
    pub comments: Option<i64>,
    pub engagement_rate: Option<f64>,
    pub cost_per_engagement: Option<f64>,
}

/// Core content item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub id: String,
    pub workflow_id: Option<String>,
    pub campaign_id: Option<String>,
    pub content_type: ContentType,
    pub title: String,
    pub body: Option<String>,
    pub summary: Option<String>,
    pub media_urls: Vec<String>,
    pub hashtags: Vec<String>,
    pub mentions: Vec<String>,
    pub call_to_action: Option<String>,
    pub target_platforms: Vec<String>,
    pub seo_metadata: Option<SeoMetadata>,
    pub performance_metrics: Option<ContentMetrics>,
    pub ai_metadata: Option<AiMetadata>,
    pub status: ContentStatus,
    pub version: i32,
    pub language: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

// ============================================================================
// CAMPAIGN TYPES
// ============================================================================

/// Campaign types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignType {
    ProductLaunch,
    BrandAwareness,
    LeadGeneration,
    Retargeting,
    Seasonal,
    EventPromotion,
}

/// Campaign status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CampaignStatus {
    Draft,
    Scheduled,
    Active,
    Paused,
    Completed,
    Cancelled,
}

/// Campaign budget information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignBudget {
    pub total_budget: f64,
    pub daily_budget: Option<f64>,
    pub spent_amount: f64,
    pub currency: String,
    pub budget_allocation: HashMap<String, f64>,
}

/// Campaign timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignTimeline {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub launch_date: Option<DateTime<Utc>>,
    pub milestones: Vec<CampaignMilestone>,
}

/// Campaign milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMilestone {
    pub name: String,
    pub date: DateTime<Utc>,
    pub status: String,
    pub description: Option<String>,
}

/// Campaign performance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignPerformance {
    pub impressions: i64,
    pub reach: i64,
    pub clicks: i64,
    pub conversions: i64,
    pub cost_per_click: f64,
    pub cost_per_conversion: f64,
    pub return_on_ad_spend: f64,
    pub engagement_rate: f64,
    pub last_updated: DateTime<Utc>,
}

// ============================================================================
// FEDERATION TYPES
// ============================================================================

/// Authentication types for federated clients
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    OAuth2,
    Jwt,
    BasicAuth,
}

/// Federated client status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientStatus {
    Active,
    Inactive,
    Suspended,
}

/// Federated client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedClient {
    pub id: Uuid,
    pub client_name: String,
    pub client_id: String,
    pub api_endpoint: String,
    pub auth_type: AuthType,
    pub auth_config: HashMap<String, serde_json::Value>,
    pub webhook_url: Option<String>,
    pub status: ClientStatus,
    pub rate_limit_per_minute: i32,
    pub rate_limit_per_hour: i32,
    pub sla_uptime_percent: f64,
    pub sla_response_time_ms: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_health_check: Option<DateTime<Utc>>,
}

/// MCP server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: Uuid,
    pub client_id: Option<Uuid>,
    pub server_id: String,
    pub server_name: String,
    pub description: Option<String>,
    pub endpoint: String,
    pub version: Option<String>,
    pub auth_required: bool,
    pub cost_per_request_cents: i32,
    pub status: String,
    pub capabilities: Vec<String>,
    pub tools: Vec<McpTool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub cost_cents: Option<i32>,
}

/// MCP request/response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub method: String,
    pub params: serde_json::Value,
    pub id: Option<String>,
    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
    pub response_time_ms: f64,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// ============================================================================
// ANALYTICS TYPES
// ============================================================================

/// Time frame for analytics queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeFrame {
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// Analytics trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Workflow analytics metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalytics {
    pub total_workflows: i64,
    pub successful_workflows: i64,
    pub failed_workflows: i64,
    pub success_rate_percent: f64,
    pub total_cost_usd: f64,
    pub average_cost_per_workflow: f64,
    pub total_execution_time_seconds: i64,
    pub average_execution_time_seconds: i64,
    pub cost_trends: Vec<TrendDataPoint>,
    pub workflow_trends: Vec<TrendDataPoint>,
}

/// Usage analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    pub total_requests: i64,
    pub total_data_processed_bytes: i64,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub rate_limit_hits: i64,
    pub peak_usage_time: Option<DateTime<Utc>>,
}

// ============================================================================
// BILLING TYPES
// ============================================================================

/// Billing cycle options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BillingCycle {
    Monthly,
    Yearly,
}

/// Subscription status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    Expired,
    Suspended,
}

/// Invoice status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

/// Resource usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub resource_type: String,
    pub quantity: i32,
    pub unit_cost_cents: Option<i32>,
    pub total_cost_cents: Option<i32>,
    pub billing_period: chrono::NaiveDate,
    pub workflow_id: Option<Uuid>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub recorded_at: DateTime<Utc>,
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Standard error types across the platform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    NotFoundError,
    ConflictError,
    RateLimitError,
    ExternalApiError,
    DatabaseError,
    NetworkError,
    InternalServerError,
    ServiceUnavailableError,
}

/// Standardized error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<Vec<ErrorDetail>>,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Error detail for validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub field: String,
    pub error: String,
    pub value: Option<String>,
}

// ============================================================================
// SYSTEM TYPES
// ============================================================================

/// System health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub name: String,
    pub status: HealthStatus,
    pub response_time_ms: Option<f64>,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub platform_name: String,
    pub version: String,
    pub build_date: DateTime<Utc>,
    pub environment: String,
    pub api_version: String,
    pub documentation_url: Option<String>,
    pub support_email: Option<String>,
    pub features: Vec<String>,
}

// ============================================================================
// CONFIGURATION TYPES
// ============================================================================

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub per_minute: i32,
    pub per_hour: i32,
    pub per_day: i32,
    pub burst_multiplier: f64,
}

/// External API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiConfig {
    pub base_url: String,
    pub timeout_seconds: i32,
    pub max_retries: i32,
    pub rate_limit_rpm: i32,
    pub circuit_breaker_enabled: bool,
}

// ============================================================================
// UTILITY TYPES AND IMPLEMENTATIONS
// ============================================================================

/// Pagination information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: i32,
    pub limit: i32,
    pub total_items: i64,
    pub total_pages: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

/// Standard success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }
}

// ============================================================================
// ADDITIONAL MISSING TYPES FOR API LAYER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    LlmCall,
    WebSearch,
    EmailSend,
    DataTransform,
    ApiCall,
    FileOperation,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_seconds: u32,
    pub max_delay_seconds: u32,
    pub backoff_multiplier: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_seconds: 1,
            max_delay_seconds: 60,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub workflow_type: WorkflowType,
    pub steps: Vec<WorkflowStepDefinition>,
    pub status: WorkflowDefinitionStatus,
    pub version: u32,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowDefinitionStatus {
    Draft,
    Active,
    Deprecated,
    Archived,
}

impl Default for WorkflowDefinitionStatus {
    fn default() -> Self {
        WorkflowDefinitionStatus::Draft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepDefinition {
    pub id: Uuid,
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
pub struct WorkflowExecution {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub workflow_definition: WorkflowDefinition,
    pub status: WorkflowExecutionStatus,
    pub progress: WorkflowProgress,
    pub input_parameters: serde_json::Value,
    pub output_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowExecutionStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl Default for WorkflowExecutionStatus {
    fn default() -> Self {
        WorkflowExecutionStatus::Queued
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub step_id: Uuid,
    pub step_name: String,
    pub status: StepStatus,
    pub progress_percent: f32,
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAllocation {
    pub provider_id: String,
    pub provider_type: String,
    pub allocated_steps: Vec<Uuid>,
    pub estimated_cost: f64,
    pub priority_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub intent_type: String,
    pub confidence: f32,
    pub parameters: serde_json::Value,
    pub workflow_type: WorkflowType,
    pub estimated_steps: Vec<String>,
    pub estimated_cost: f64,
    pub estimated_duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub api_key: Option<String>,
    pub oauth_config: Option<serde_json::Value>,
    pub jwt_config: Option<serde_json::Value>,
    pub basic_auth: Option<BasicAuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuthConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub server_id: Uuid,
    pub name: String,
    pub endpoint: String,
    pub tools: Vec<String>,
    pub status: McpServerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpServerStatus {
    Active,
    Inactive,
    Maintenance,
    Error,
}

impl Default for McpServerStatus {
    fn default() -> Self {
        McpServerStatus::Inactive
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub concurrent_requests: u32,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            concurrent_requests: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_id: Uuid,
    pub client_name: String,
    pub status: ClientStatus,
    pub registration_date: chrono::DateTime<chrono::Utc>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub service_name: String,
    pub average_response_time_ms: f64,
    pub requests_per_second: f64,
    pub error_rate_percent: f32,
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

// ============================================================================
// TRAIT IMPLEMENTATIONS
// ============================================================================

impl std::fmt::Display for SubscriptionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionTier::Free => write!(f, "free"),
            SubscriptionTier::Pro => write!(f, "pro"),
            SubscriptionTier::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStatus::Created => write!(f, "created"),
            WorkflowStatus::Queued => write!(f, "queued"),
            WorkflowStatus::Running => write!(f, "running"),
            WorkflowStatus::Completed => write!(f, "completed"),
            WorkflowStatus::Failed => write!(f, "failed"),
            WorkflowStatus::Cancelled => write!(f, "cancelled"),
            WorkflowStatus::Paused => write!(f, "paused"),
        }
    }
}

impl std::fmt::Display for WorkflowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowType::ContentGeneration => write!(f, "content_generation"),
            WorkflowType::SocialMedia => write!(f, "social_media"),
            WorkflowType::EmailMarketing => write!(f, "email_marketing"),
            WorkflowType::DataAnalysis => write!(f, "data_analysis"),
            WorkflowType::Automation => write!(f, "automation"),
        }
    }
}

impl std::str::FromStr for Permission {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "workflows:read" => Ok(Permission::WorkflowsRead),
            "workflows:create" => Ok(Permission::WorkflowsCreate),
            "workflows:update" => Ok(Permission::WorkflowsUpdate),
            "workflows:delete" => Ok(Permission::WorkflowsDelete),
            "content:read" => Ok(Permission::ContentRead),
            "content:create" => Ok(Permission::ContentCreate),
            "content:update" => Ok(Permission::ContentUpdate),
            "content:delete" => Ok(Permission::ContentDelete),
            "campaigns:read" => Ok(Permission::CampaignsRead),
            "campaigns:create" => Ok(Permission::CampaignsCreate),
            "campaigns:update" => Ok(Permission::CampaignsUpdate),
            "campaigns:delete" => Ok(Permission::CampaignsDelete),
            "analytics:read" => Ok(Permission::AnalyticsRead),
            "analytics:export" => Ok(Permission::AnalyticsExport),
            "federation:proxy" => Ok(Permission::FederationProxy),
            "federation:manage" => Ok(Permission::FederationManage),
            "admin:users" => Ok(Permission::AdminUsers),
            "admin:system" => Ok(Permission::AdminSystem),
            "admin:billing" => Ok(Permission::AdminBilling),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default rate limits for different subscription tiers
pub mod rate_limits {
    use super::RateLimitConfig;
    use std::collections::HashMap;

    pub fn default_limits() -> HashMap<super::SubscriptionTier, RateLimitConfig> {
        let mut limits = HashMap::new();

        limits.insert(
            super::SubscriptionTier::Free,
            RateLimitConfig {
                per_minute: 10,
                per_hour: 100,
                per_day: 500,
                burst_multiplier: 1.5,
            },
        );

        limits.insert(
            super::SubscriptionTier::Pro,
            RateLimitConfig {
                per_minute: 60,
                per_hour: 1000,
                per_day: 10000,
                burst_multiplier: 2.0,
            },
        );

        limits.insert(
            super::SubscriptionTier::Enterprise,
            RateLimitConfig {
                per_minute: 300,
                per_hour: 5000,
                per_day: 50000,
                burst_multiplier: 3.0,
            },
        );

        limits
    }
}

/// Validation constants
pub mod validation {
    pub const MIN_PASSWORD_LENGTH: usize = 8;
    pub const MAX_EMAIL_LENGTH: usize = 255;
    pub const MAX_USERNAME_LENGTH: usize = 100;
    pub const MAX_CONTENT_TITLE_LENGTH: usize = 500;
    pub const MAX_CONTENT_SUMMARY_LENGTH: usize = 1000;
    pub const MAX_REQUEST_SIZE_MB: usize = 50;
    pub const MAX_FILE_SIZE_MB: usize = 100;

    pub const ALLOWED_FILE_EXTENSIONS: &[&str] = &[
        ".jpg", ".jpeg", ".png", ".gif", ".mp4", ".mp3", ".pdf", ".txt", ".doc", ".docx", ".csv",
        ".json", ".xml",
    ];
}

/// Default timeouts in seconds
pub mod timeouts {
    pub const HTTP_REQUEST_TIMEOUT: u64 = 30;
    pub const DATABASE_QUERY_TIMEOUT: u64 = 10;
    pub const EXTERNAL_API_TIMEOUT: u64 = 60;
    pub const WORKFLOW_TIMEOUT: u64 = 3600;
    pub const WEBSOCKET_TIMEOUT: u64 = 300;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_subscription_tier_serialization() {
        let tier = SubscriptionTier::Pro;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"pro\"");

        let deserialized: SubscriptionTier = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, tier);
    }

    #[test]
    fn test_workflow_status_display() {
        assert_eq!(WorkflowStatus::Running.to_string(), "running");
        assert_eq!(WorkflowStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse {
            error: "validation_failed".to_string(),
            message: "Invalid request parameters".to_string(),
            details: None,
            request_id: Some("req_123".to_string()),
            timestamp: Utc::now(),
        };

        assert_eq!(error.error, "validation_failed");
        assert!(error.request_id.is_some());
    }

    #[test]
    fn test_success_response_creation() {
        let response = SuccessResponse::new("Operation completed successfully");
        assert!(response.success);
        assert_eq!(response.message, "Operation completed successfully");
        assert!(response.data.is_none());

        let response_with_data =
            SuccessResponse::with_data("Data retrieved", serde_json::json!({"count": 42}));
        assert!(response_with_data.success);
        assert!(response_with_data.data.is_some());
    }

    #[test]
    fn test_rate_limits_defaults() {
        let limits = rate_limits::default_limits();

        let free_limits = limits.get(&SubscriptionTier::Free).unwrap();
        assert_eq!(free_limits.per_minute, 10);

        let pro_limits = limits.get(&SubscriptionTier::Pro).unwrap();
        assert_eq!(pro_limits.per_minute, 60);

        let enterprise_limits = limits.get(&SubscriptionTier::Enterprise).unwrap();
        assert_eq!(enterprise_limits.per_minute, 300);
    }
}
