use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Request types
#[derive(Debug, Clone, Deserialize)]
pub struct ParseIntentRequest {
    pub user_id: Uuid,
    pub text: String,
    pub context: Option<serde_json::Value>,
    pub federation_context: Option<FederationContext>,
    pub preferred_providers: Option<Vec<String>>,
    pub budget_limit: Option<f64>,
    pub time_limit: Option<chrono::Duration>,
    pub quality_threshold: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchParseRequest {
    pub requests: Vec<ParseIntentRequest>,
    pub parallel_processing: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationContext {
    pub client_id: Option<String>,
    pub available_providers: Vec<ProviderInfo>,
    pub cost_constraints: Option<CostConstraints>,
    pub quality_requirements: Option<QualityRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub provider_id: String,
    pub provider_name: String,
    pub capabilities: Vec<String>,
    pub cost_per_request: Option<f64>,
    pub quality_score: Option<f32>,
    pub response_time_ms: Option<u64>,
    pub availability: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConstraints {
    pub max_total_cost: Option<f64>,
    pub max_cost_per_function: Option<f64>,
    pub preferred_cost_tier: Option<String>, // "budget", "standard", "premium"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirements {
    pub min_confidence_score: Option<f32>,
    pub max_response_time_ms: Option<u64>,
    pub min_availability: Option<f32>,
    pub preferred_quality_tier: Option<String>, // "fast", "balanced", "high_quality"
}

// Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub workflow_id: Uuid,
    pub workflow_type: WorkflowType,
    pub functions: Vec<FunctionCall>,
    pub dependencies: Vec<Dependency>,
    pub estimated_duration: chrono::Duration,
    pub estimated_cost: f64,
    pub confidence_score: f32,
    pub steps: Vec<WorkflowStep>,
    pub required_integrations: Vec<String>,
    pub scheduling_requirements: Option<SchedulingRequirements>,
    pub provider_preferences: Vec<ProviderPreference>,
    pub metadata: IntentMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchParseResponse {
    pub results: Vec<BatchParseResult>,
    pub total_processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchParseResult {
    pub index: usize,
    pub success: bool,
    pub intent: Option<ParsedIntent>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub confidence_score: f32,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
    pub estimated_execution_time: chrono::Duration,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilitiesResponse {
    pub version: String,
    pub supported_functions: Vec<FunctionInfo>,
    pub supported_domains: Vec<String>,
    pub supported_integrations: Vec<String>,
    pub max_complexity_score: f32,
    pub max_functions_per_workflow: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionListResponse {
    pub functions: Vec<FunctionInfo>,
    pub total_count: usize,
    pub filtered_by_domain: Option<String>,
}

// Core types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    ContentCreation,
    MarketingCampaign,
    ScheduledPublishing,
    EcommerceOperation,
    BusinessIntelligence,
    Communication,
    ClientIntegration,
    Analytics,
    HybridWorkflow,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub provider: String,
    pub estimated_cost: f64,
    pub estimated_duration: chrono::Duration,
    pub confidence_score: f32,
    pub required_permissions: Vec<String>,
    pub mcp_server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub prerequisite: String,
    pub dependent: String,
    pub data_transfer: Option<String>,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Sequential,
    DataDependency,
    ResourceDependency,
    TimeDependency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: Uuid,
    pub step_type: StepType,
    pub name: String,
    pub description: String,
    pub function_calls: Vec<FunctionCall>,
    pub parallel_execution: bool,
    pub retry_policy: Option<RetryPolicy>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    ContentCreation,
    MarketingCampaign,
    SchedulingPublishing,
    EcommerceOperation,
    Analytics,
    Communication,
    ClientIntegration,
    DataProcessing,
    FileOperation,
    Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_seconds: u64,
    pub max_delay_seconds: u64,
    pub backoff_multiplier: f64,
    pub retry_on_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingRequirements {
    pub needs_scheduling: bool,
    pub schedule_type: Option<ScheduleType>,
    pub time_sensitivity: Option<TimeSensitivity>,
    pub platforms: Vec<String>,
    pub content_calendar_integration: bool,
    pub recurring_pattern: Option<RecurringPattern>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    Immediate,
    Delayed,
    Recurring,
    OptimalTiming,
    FollowerActivity,
    EventBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeSensitivity {
    High,
    Medium,
    Low,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringPattern {
    pub frequency: String, // "daily", "weekly", "monthly", "custom"
    pub interval: u32,
    pub cron_expression: Option<String>,
    pub end_date: Option<DateTime<Utc>>,
    pub max_occurrences: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreference {
    pub domain: String,
    pub preferred_providers: Vec<String>,
    pub fallback_providers: Vec<String>,
    pub cost_weight: f32,
    pub quality_weight: f32,
    pub speed_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMetadata {
    pub created_at: DateTime<Utc>,
    pub complexity_score: f32,
    pub language: String,
    pub domain_scores: HashMap<String, f32>,
    pub user_preferences: Option<UserPreferences>,
    pub context_variables: HashMap<String, serde_json::Value>,
}

// User context types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserContext {
    pub user_id: Uuid,
    pub preferences: UserPreferences,
    pub history: Vec<IntentHistory>,
    pub integrations: Vec<UserIntegration>,
    pub subscription_tier: SubscriptionTier,
    pub usage_statistics: UsageStatistics,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub preferred_providers: HashMap<String, String>,
    pub cost_sensitivity: f32, // 0.0 (cost-conscious) to 1.0 (quality-focused)
    pub speed_preference: f32, // 0.0 (thorough) to 1.0 (fast)
    pub default_timezone: String,
    pub notification_preferences: NotificationPreferences,
    pub content_preferences: ContentPreferences,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationPreferences {
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub slack_notifications: bool,
    pub webhook_url: Option<String>,
    pub notification_frequency: String, // "immediate", "hourly", "daily"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentPreferences {
    pub default_tone: String, // "professional", "casual", "friendly", "formal"
    pub brand_voice: Option<String>,
    pub target_audience: Option<String>,
    pub content_length_preference: String, // "short", "medium", "long"
    pub include_hashtags: bool,
    pub include_emojis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentHistory {
    pub intent_id: Uuid,
    pub text: String,
    pub parsed_intent: ParsedIntent,
    pub execution_result: Option<ExecutionResult>,
    pub created_at: DateTime<Utc>,
    pub feedback_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub duration: chrono::Duration,
    pub cost: f64,
    pub error_message: Option<String>,
    pub output_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntegration {
    pub integration_id: String,
    pub integration_type: String, // "social_media", "email", "crm", "ecommerce"
    pub platform: String,         // "facebook", "twitter", "gmail", "shopify", etc.
    pub credentials_valid: bool,
    pub permissions: Vec<String>,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTier {
    Free,
    Basic,
    Professional,
    Enterprise,
    Custom(String),
}

impl Default for SubscriptionTier {
    fn default() -> Self {
        SubscriptionTier::Free
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStatistics {
    pub total_intents_parsed: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_cost_spent: f64,
    pub total_time_saved_hours: f64,
    pub favorite_functions: Vec<String>,
    pub most_used_domains: Vec<String>,
    pub average_confidence_score: f32,
}

// Function metadata types
#[derive(Debug, Clone, Serialize)]
pub struct FunctionInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub cost_range: CostRange,
    pub estimated_duration: chrono::Duration,
    pub complexity_score: f32,
    pub popularity_score: f32,
    pub success_rate: f32,
    pub required_permissions: Vec<String>,
    pub supported_parameters: Vec<ParameterInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostRange {
    pub min_cost: f64,
    pub max_cost: f64,
    pub average_cost: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterInfo {
    pub name: String,
    pub parameter_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDetails {
    pub info: FunctionInfo,
    pub examples: Vec<FunctionExample>,
    pub integration_requirements: Vec<String>,
    pub rate_limits: Option<RateLimit>,
    pub documentation_url: Option<String>,
    pub changelog: Vec<ChangelogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionExample {
    pub name: String,
    pub description: String,
    pub input: serde_json::Value,
    pub expected_output: serde_json::Value,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub burst_limit: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: DateTime<Utc>,
    pub changes: Vec<String>,
    pub breaking_changes: Vec<String>,
}

// Validation types
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub confidence_score: f32,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
    pub estimated_execution_time: chrono::Duration,
    pub estimated_cost: f64,
    pub missing_permissions: Vec<String>,
    pub invalid_parameters: Vec<String>,
}

// Capability types
#[derive(Debug, Clone)]
pub struct AvailableCapabilities {
    pub functions: Vec<FunctionInfo>,
    pub domains: Vec<String>,
    pub integrations: Vec<String>,
    pub max_complexity_score: f32,
    pub max_functions_per_workflow: usize,
}

// Error types for parser
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("LLM API error: {0}")]
    LlmError(String),

    #[error("Invalid request format: {0}")]
    InvalidRequest(String),

    #[error("Unsupported function: {0}")]
    UnsupportedFunction(String),

    #[error("Context processing error: {0}")]
    ContextError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitError(String),

    #[error("Provider selection failed: {0}")]
    ProviderSelectionError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),
}
