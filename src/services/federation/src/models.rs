//! Data models and structures for the Federation Service
//!
//! This module defines all the data structures used throughout the federation service,
//! including client management, schema translation, provider selection, and workflow execution.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

// ================================================================================================
// Client Management Models
// ================================================================================================

/// Represents a federated client in the system
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    /// Unique client identifier
    pub id: Uuid,
    /// Human-readable client name
    pub name: String,
    /// Client description
    pub description: Option<String>,
    /// Client tier for billing and resource allocation
    pub tier: ClientTier,
    /// Client configuration settings
    pub config: ClientConfig,
    /// Authentication credentials
    pub credentials: ClientCredentials,
    /// Current client status
    pub status: ClientStatus,
    /// Resource usage limits
    pub limits: ResourceLimits,
    /// Client metadata and tags
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// Client tier for billing and resource allocation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientTier {
    /// Free tier with basic features
    Free,
    /// Professional tier with advanced features
    Professional,
    /// Enterprise tier with full features
    Enterprise,
    /// Custom tier with negotiated features
    Custom,
}

impl FromStr for ClientTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(ClientTier::Free),
            "professional" => Ok(ClientTier::Professional),
            "enterprise" => Ok(ClientTier::Enterprise),
            "custom" => Ok(ClientTier::Custom),
            _ => Err(format!("Invalid client tier: {}", s)),
        }
    }
}

/// Client configuration settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    /// Preferred providers for different services
    pub preferred_providers: HashMap<String, String>,
    /// Cost optimization settings
    pub cost_optimization: CostOptimizationConfig,
    /// Schema preferences
    pub schema_preferences: SchemaPreferences,
    /// Workflow execution settings
    pub workflow_settings: WorkflowSettings,
    /// Proxy configuration
    pub proxy_config: ProxyConfig,
}

/// Cost optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CostOptimizationConfig {
    /// Enable cost optimization
    pub enabled: bool,
    /// Maximum cost per request
    pub max_cost_per_request: Option<f64>,
    /// Monthly budget limit
    pub monthly_budget_limit: Option<f64>,
    /// Prefer cheaper providers when quality is similar
    pub prefer_cheaper_providers: bool,
    /// Quality vs cost trade-off (0.0 = cheapest, 1.0 = highest quality)
    pub quality_cost_ratio: f64,
}

/// Schema preferences for compatibility layer
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaPreferences {
    /// Preferred schema version
    pub preferred_version: String,
    /// Enable automatic schema translation
    pub auto_translation: bool,
    /// Strict mode for schema validation
    pub strict_validation: bool,
    /// Custom field mappings
    pub custom_mappings: HashMap<String, String>,
}

/// Workflow execution settings
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSettings {
    /// Default workflow timeout
    pub default_timeout: u64,
    /// Maximum concurrent workflows
    pub max_concurrent_workflows: u32,
    /// Retry policy
    pub retry_policy: RetryPolicy,
    /// Enable workflow monitoring
    pub monitoring_enabled: bool,
}

/// Proxy configuration for MCP server integration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    /// Enable proxy mode
    pub enabled: bool,
    /// Proxy timeout settings
    pub timeout: ProxyTimeout,
    /// Connection pooling settings
    pub connection_pool: ConnectionPoolConfig,
    /// Caching settings
    pub caching: CachingConfig,
}

/// Proxy timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTimeout {
    /// Connect timeout in milliseconds
    pub connect_timeout: u64,
    /// Request timeout in milliseconds
    pub request_timeout: u64,
    /// Keep-alive timeout in milliseconds
    pub keep_alive_timeout: u64,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPoolConfig {
    /// Maximum connections per host
    pub max_connections_per_host: u32,
    /// Connection idle timeout
    pub idle_timeout: u64,
    /// Connection keep-alive
    pub keep_alive: bool,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CachingConfig {
    /// Enable caching
    pub enabled: bool,
    /// Cache TTL in seconds
    pub ttl: u64,
    /// Maximum cache size
    pub max_size: u64,
    /// Cache strategy
    pub strategy: CacheStrategy,
}

/// Cache strategy
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CacheStrategy {
    /// Least recently used
    Lru,
    /// Least frequently used
    Lfu,
    /// Time to live
    Ttl,
    /// First in, first out
    Fifo,
}

/// Client authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentials {
    /// API key for authentication
    pub api_key: String,
    /// JWT secret for token validation
    pub jwt_secret: Option<String>,
    /// OAuth configuration
    pub oauth_config: Option<OAuthConfig>,
    /// Webhook secret for secure callbacks
    pub webhook_secret: Option<String>,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfig {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
}

/// Client status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientStatus {
    /// Client is active and operational
    Active,
    /// Client is suspended due to violations or non-payment
    Suspended,
    /// Client is inactive but not suspended
    Inactive,
    /// Client is pending approval
    Pending,
    /// Client is being migrated
    Migrating,
}

impl FromStr for ClientStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(ClientStatus::Active),
            "suspended" => Ok(ClientStatus::Suspended),
            "inactive" => Ok(ClientStatus::Inactive),
            "pending" => Ok(ClientStatus::Pending),
            "migrating" => Ok(ClientStatus::Migrating),
            _ => Err(format!("Invalid client status: {}", s)),
        }
    }
}

/// Resource usage limits for client
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimits {
    /// Maximum requests per minute
    pub max_requests_per_minute: u32,
    /// Maximum requests per hour
    pub max_requests_per_hour: u32,
    /// Maximum requests per day
    pub max_requests_per_day: u32,
    /// Maximum concurrent connections
    pub max_concurrent_connections: u32,
    /// Maximum data transfer per day (in bytes)
    pub max_data_transfer_per_day: u64,
    /// Maximum storage usage (in bytes)
    pub max_storage_usage: u64,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries (milliseconds)
    pub initial_delay: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Enable exponential backoff
    pub exponential_backoff: bool,
}

// ================================================================================================
// Provider Management Models
// ================================================================================================

/// Represents a service provider in the federation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    /// Unique provider identifier
    pub id: Uuid,
    /// Provider name
    pub name: String,
    /// Provider type (e.g., LLM, Storage, Compute)
    pub provider_type: ProviderType,
    /// Provider configuration
    pub config: ProviderConfig,
    /// Cost information
    pub cost_info: CostInfo,
    /// Quality metrics
    pub quality_metrics: QualityMetrics,
    /// Provider status
    pub status: ProviderStatus,
    /// Supported capabilities
    pub capabilities: Vec<String>,
    /// Health check endpoint
    pub health_endpoint: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Provider type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    /// Large Language Model provider
    Llm,
    /// Storage provider
    Storage,
    /// Compute provider
    Compute,
    /// Database provider
    Database,
    /// Message queue provider
    MessageQueue,
    /// Monitoring provider
    Monitoring,
    /// Custom provider type
    Custom(String),
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    /// Provider endpoint URL
    pub endpoint: String,
    /// Authentication method
    pub auth_method: AuthMethod,
    /// Request timeout
    pub timeout: u64,
    /// Rate limit information
    pub rate_limits: RateLimits,
    /// Custom headers
    pub headers: HashMap<String, String>,
}

/// Authentication method for providers
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication required
    None,
    /// API key authentication
    ApiKey { key: String },
    /// Bearer token authentication
    Bearer { token: String },
    /// Basic authentication
    Basic { username: String, password: String },
    /// OAuth 2.0 authentication
    OAuth { config: OAuthConfig },
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimits {
    /// Requests per second
    pub requests_per_second: Option<u32>,
    /// Requests per minute
    pub requests_per_minute: Option<u32>,
    /// Requests per hour
    pub requests_per_hour: Option<u32>,
    /// Concurrent requests
    pub concurrent_requests: Option<u32>,
}

/// Cost information for provider
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CostInfo {
    /// Cost per request
    pub cost_per_request: f64,
    /// Cost per token (for LLM providers)
    pub cost_per_token: Option<f64>,
    /// Cost per GB (for storage providers)
    pub cost_per_gb: Option<f64>,
    /// Cost per compute hour
    pub cost_per_compute_hour: Option<f64>,
    /// Minimum cost threshold
    pub minimum_cost: f64,
    /// Currency code
    pub currency: String,
}

/// Quality metrics for provider
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityMetrics {
    /// Average response time (milliseconds)
    pub avg_response_time: f64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Availability (0.0 - 1.0)
    pub availability: f64,
    /// Quality score (0.0 - 1.0)
    pub quality_score: f64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Provider status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// Provider is active and healthy
    Active,
    /// Provider is experiencing issues
    Degraded,
    /// Provider is temporarily unavailable
    Unavailable,
    /// Provider is under maintenance
    Maintenance,
    /// Provider is permanently disabled
    Disabled,
}

// ================================================================================================
// Schema Translation Models
// ================================================================================================

/// Schema translation configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaTranslation {
    /// Unique translation ID
    pub id: Uuid,
    /// Source schema version
    pub source_version: String,
    /// Target schema version
    pub target_version: String,
    /// Field mappings
    pub field_mappings: HashMap<String, FieldMapping>,
    /// Transformation rules
    pub transformation_rules: Vec<TransformationRule>,
    /// Validation rules
    pub validation_rules: Vec<ValidationRule>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Field mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldMapping {
    /// Source field path
    pub source_field: String,
    /// Target field path
    pub target_field: String,
    /// Data type conversion
    pub type_conversion: Option<TypeConversion>,
    /// Default value if source field is missing
    pub default_value: Option<serde_json::Value>,
    /// Whether the field is required
    pub required: bool,
}

/// Type conversion configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypeConversion {
    /// Convert string to number
    StringToNumber,
    /// Convert number to string
    NumberToString,
    /// Convert boolean to string
    BooleanToString,
    /// Convert string to boolean
    StringToBoolean,
    /// Convert array to string (JSON)
    ArrayToString,
    /// Convert string to array (JSON)
    StringToArray,
    /// Convert object to string (JSON)
    ObjectToString,
    /// Convert string to object (JSON)
    StringToObject,
    /// Custom conversion function
    Custom(String),
}

/// Transformation rule
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransformationRule {
    /// Rule name
    pub name: String,
    /// Field selector
    pub field_selector: String,
    /// Transformation function
    pub transformation: TransformationFunction,
    /// Rule conditions
    pub conditions: Vec<RuleCondition>,
}

/// Transformation function
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransformationFunction {
    /// Convert to uppercase
    ToUpperCase,
    /// Convert to lowercase
    ToLowerCase,
    /// Trim whitespace
    Trim,
    /// Format as date
    FormatDate,
    /// Calculate hash
    Hash,
    /// Encrypt value
    Encrypt,
    /// Decrypt value
    Decrypt,
    /// Custom function
    Custom(String),
}

/// Rule condition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuleCondition {
    /// Field to check
    pub field: String,
    /// Condition operator
    pub operator: ConditionOperator,
    /// Expected value
    pub value: serde_json::Value,
}

/// Condition operator
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Equal to
    Equal,
    /// Not equal to
    NotEqual,
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Contains
    Contains,
    /// Starts with
    StartsWith,
    /// Ends with
    EndsWith,
    /// Matches regex
    Regex,
}

/// Validation rule
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Field to validate
    pub field: String,
    /// Validation type
    pub validation_type: ValidationType,
    /// Error message if validation fails
    pub error_message: String,
}

/// Validation type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidationType {
    /// Required field
    Required,
    /// Minimum length
    MinLength(usize),
    /// Maximum length
    MaxLength(usize),
    /// Pattern match
    Pattern(String),
    /// Email format
    Email,
    /// URL format
    Url,
    /// UUID format
    Uuid,
    /// Date format
    Date,
    /// Number range
    NumberRange { min: f64, max: f64 },
    /// Custom validation
    Custom(String),
}

// ================================================================================================
// Workflow Execution Models
// ================================================================================================

/// Federated workflow definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FederatedWorkflow {
    /// Unique workflow ID
    pub id: Uuid,
    /// Client ID that owns this workflow
    pub client_id: Uuid,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Workflow configuration
    pub config: WorkflowConfig,
    /// Current status
    pub status: WorkflowStatus,
    /// Execution history
    pub execution_history: Vec<WorkflowExecution>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Individual workflow step
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    /// Step ID
    pub id: String,
    /// Step name
    pub name: String,
    /// Step type
    pub step_type: StepType,
    /// Provider to use for this step
    pub provider_id: Option<Uuid>,
    /// Step configuration
    pub config: StepConfig,
    /// Input mapping
    pub input_mapping: HashMap<String, String>,
    /// Output mapping
    pub output_mapping: HashMap<String, String>,
    /// Dependencies (steps that must complete first)
    pub dependencies: Vec<String>,
    /// Retry configuration
    pub retry_config: Option<RetryPolicy>,
}

/// Workflow step type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// LLM inference step
    LlmInference,
    /// Data transformation step
    DataTransformation,
    /// API call step
    ApiCall,
    /// Database operation step
    DatabaseOperation,
    /// File operation step
    FileOperation,
    /// Notification step
    Notification,
    /// Conditional step
    Conditional,
    /// Loop step
    Loop,
    /// Parallel execution step
    Parallel,
    /// Custom step
    Custom(String),
}

/// Step configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StepConfig {
    /// Step-specific parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Timeout for step execution
    pub timeout: Option<u64>,
    /// Enable step monitoring
    pub monitoring_enabled: bool,
    /// Cost budget for this step
    pub cost_budget: Option<f64>,
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfig {
    /// Workflow timeout
    pub timeout: u64,
    /// Maximum parallel executions
    pub max_parallel_executions: u32,
    /// Retry policy
    pub retry_policy: RetryPolicy,
    /// Cost budget
    pub cost_budget: Option<f64>,
    /// Priority level
    pub priority: WorkflowPriority,
    /// Execution environment
    pub environment: ExecutionEnvironment,
}

/// Workflow priority
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Execution environment
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEnvironment {
    /// Development environment
    Development,
    /// Testing environment
    Testing,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is pending execution
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
    /// Workflow is paused
    Paused,
    /// Workflow timed out
    TimedOut,
}

/// Workflow execution record
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecution {
    /// Execution ID
    pub id: Uuid,
    /// Workflow ID
    pub workflow_id: Uuid,
    /// Execution status
    pub status: WorkflowStatus,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Execution result
    pub result: Option<serde_json::Value>,
    /// Error information if failed
    pub error: Option<ExecutionError>,
    /// Step executions
    pub step_executions: Vec<StepExecution>,
    /// Total cost
    pub total_cost: f64,
    /// Resource usage
    pub resource_usage: ResourceUsage,
}

/// Step execution record
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StepExecution {
    /// Step ID
    pub step_id: String,
    /// Execution status
    pub status: WorkflowStatus,
    /// Provider used
    pub provider_id: Option<Uuid>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Step result
    pub result: Option<serde_json::Value>,
    /// Error information if failed
    pub error: Option<ExecutionError>,
    /// Step cost
    pub cost: f64,
    /// Retry attempts
    pub retry_attempts: u32,
}

/// Execution error information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<serde_json::Value>,
    /// Stack trace
    pub stack_trace: Option<String>,
    /// Timestamp when error occurred
    pub occurred_at: DateTime<Utc>,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsage {
    /// CPU time used (milliseconds)
    pub cpu_time: u64,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Network I/O (bytes)
    pub network_io: u64,
    /// Disk I/O (bytes)
    pub disk_io: u64,
    /// API calls made
    pub api_calls: u32,
}

// ================================================================================================
// API Request/Response Models
// ================================================================================================

/// Client registration request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientRegistrationRequest {
    /// Client name
    pub name: String,
    /// Client description
    pub description: Option<String>,
    /// Desired client tier
    pub tier: ClientTier,
    /// Initial configuration
    pub config: ClientConfig,
    /// Client metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Client registration response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientRegistrationResponse {
    /// Registered client
    pub client: Client,
    /// Generated API key
    pub api_key: String,
    /// Welcome message
    pub message: String,
}

/// Provider selection request
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSelectionRequest {
    /// Client ID making the request
    pub client_id: Uuid,
    /// Service type needed
    pub service_type: ProviderType,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Cost constraints
    pub cost_constraints: Option<CostConstraints>,
    /// Quality requirements
    pub quality_requirements: Option<QualityRequirements>,
}

/// Cost constraints for provider selection
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CostConstraints {
    /// Maximum cost per request
    pub max_cost_per_request: Option<f64>,
    /// Maximum total cost
    pub max_total_cost: Option<f64>,
    /// Prefer cheaper options
    pub prefer_cheaper: bool,
}

/// Quality requirements for provider selection
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityRequirements {
    /// Minimum success rate required
    pub min_success_rate: Option<f64>,
    /// Maximum acceptable response time
    pub max_response_time: Option<f64>,
    /// Minimum availability required
    pub min_availability: Option<f64>,
    /// Minimum quality score required
    pub min_quality_score: Option<f64>,
}

/// Provider selection response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSelectionResponse {
    /// Selected provider
    pub provider: Provider,
    /// Selection reasoning
    pub reasoning: String,
    /// Estimated cost
    pub estimated_cost: f64,
    /// Expected quality metrics
    pub expected_quality: QualityMetrics,
}

/// Schema translation request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaTranslationRequest {
    /// Source data to translate
    pub source_data: serde_json::Value,
    /// Source schema version
    pub source_version: String,
    /// Target schema version
    pub target_version: String,
    /// Client ID for custom mappings
    pub client_id: Option<Uuid>,
}

/// Schema translation response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaTranslationResponse {
    /// Translated data
    pub translated_data: serde_json::Value,
    /// Translation metadata
    pub translation_metadata: TranslationMetadata,
    /// Any validation warnings
    pub warnings: Vec<String>,
}

/// Translation metadata
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranslationMetadata {
    /// Translation ID used
    pub translation_id: Uuid,
    /// Fields that were mapped
    pub mapped_fields: Vec<String>,
    /// Fields that were dropped
    pub dropped_fields: Vec<String>,
    /// Fields that used default values
    pub defaulted_fields: Vec<String>,
    /// Translation duration
    pub duration_ms: u64,
}

// ================================================================================================
// Error Models
// ================================================================================================

/// Federation service error types
#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    /// Client not found
    #[error("Client not found: {id}")]
    ClientNotFound { id: Uuid },

    /// Provider not found
    #[error("Provider not found: {id}")]
    ProviderNotFound { id: Uuid },

    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    /// Authorization failed
    #[error("Authorization failed: {reason}")]
    AuthorizationFailed { reason: String },

    /// Schema translation failed
    #[error("Schema translation failed: {reason}")]
    SchemaTranslationFailed { reason: String },

    /// Provider selection failed
    #[error("Provider selection failed: {reason}")]
    ProviderSelectionFailed { reason: String },

    /// Workflow execution failed
    #[error("Workflow execution failed: {reason}")]
    WorkflowExecutionFailed { reason: String },

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {limit_type}")]
    ResourceLimitExceeded { limit_type: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    /// External service error
    #[error("External service error: {service} - {message}")]
    ExternalServiceError { service: String, message: String },

    /// Database error
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    /// Cache error
    #[error("Cache error: {message}")]
    CacheError { message: String },

    /// Validation error
    #[error("Validation error: {field} - {message}")]
    ValidationError { field: String, message: String },

    /// Internal server error
    #[error("Internal server error: {message}")]
    InternalError { message: String },
}

impl FederationError {
    /// Convert error to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            FederationError::ClientNotFound { .. } => 404,
            FederationError::ProviderNotFound { .. } => 404,
            FederationError::AuthenticationFailed { .. } => 401,
            FederationError::AuthorizationFailed { .. } => 403,
            FederationError::ResourceLimitExceeded { .. } => 429,
            FederationError::ValidationError { .. } => 400,
            FederationError::ConfigurationError { .. } => 400,
            _ => 500,
        }
    }
}
