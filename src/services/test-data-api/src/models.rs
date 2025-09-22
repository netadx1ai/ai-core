// AI-CORE Test Data API Models
// FAANG-Enhanced Testing Infrastructure with Comprehensive Data Management
// Backend Agent Implementation - T2.2

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

// ============================================================================
// Core Test Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Validate)]
pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
    pub permissions: Vec<String>,
    pub metadata: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub test_environment: String,
    pub cleanup_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
    Viewer,
    Manager,
    Developer,
    Tester,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTestUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
    pub permissions: Vec<String>,
    pub metadata: Option<serde_json::Value>,
    pub test_environment: String,
    pub ttl_hours: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TestWorkflow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub workflow_definition: serde_json::Value,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub test_cases: Vec<TestCase>,
    pub status: WorkflowStatus,
    pub version: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub test_environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "workflow_status", rename_all = "lowercase")]
pub enum WorkflowStatus {
    Draft,
    Active,
    Testing,
    Disabled,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub input_data: serde_json::Value,
    pub expected_output: serde_json::Value,
    pub assertions: Vec<TestAssertion>,
    pub setup_steps: Vec<String>,
    pub cleanup_steps: Vec<String>,
    pub timeout_seconds: i32,
    pub retry_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAssertion {
    pub field_path: String,
    pub assertion_type: AssertionType,
    pub expected_value: serde_json::Value,
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertionType {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Matches,
    NotMatches,
    IsNull,
    IsNotNull,
    IsEmpty,
    IsNotEmpty,
}

// ============================================================================
// Test Environment Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TestEnvironment {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub environment_type: EnvironmentType,
    pub configuration: EnvironmentConfig,
    pub database_configs: HashMap<String, DatabaseConfig>,
    pub service_configs: HashMap<String, ServiceConfig>,
    pub status: EnvironmentStatus,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub auto_cleanup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "environment_type", rename_all = "lowercase")]
pub enum EnvironmentType {
    Development,
    Testing,
    Staging,
    Integration,
    Performance,
    Chaos,
    Sandbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub base_url: String,
    pub api_endpoints: HashMap<String, String>,
    pub authentication: AuthenticationConfig,
    pub feature_flags: HashMap<String, bool>,
    pub resource_limits: ResourceLimits,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub database_type: String,
    pub host: String,
    pub port: i32,
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub connection_pool_size: i32,
    pub ssl_enabled: bool,
    pub migrations: Vec<String>,
    pub seed_data: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub image: String,
    pub version: String,
    pub ports: Vec<i32>,
    pub environment_variables: HashMap<String, String>,
    pub health_check_endpoint: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub jwt_secret: String,
    pub token_expiry_hours: i32,
    pub refresh_token_expiry_days: i32,
    pub multi_factor_enabled: bool,
    pub oauth_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_limit: String,
    pub memory_limit: String,
    pub disk_limit: String,
    pub network_bandwidth_limit: String,
    pub concurrent_users: i32,
    pub api_rate_limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub logging_level: String,
    pub trace_sampling_rate: f64,
    pub alert_endpoints: Vec<String>,
    pub dashboard_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "environment_status", rename_all = "lowercase")]
pub enum EnvironmentStatus {
    Provisioning,
    Ready,
    InUse,
    Maintenance,
    Error,
    Destroying,
}

// ============================================================================
// Test Data Generation Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataGenerationRequest {
    pub data_type: DataType,
    pub count: i32,
    pub template: Option<serde_json::Value>,
    pub constraints: Option<DataConstraints>,
    pub relationships: Vec<DataRelationship>,
    pub output_format: OutputFormat,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Users,
    Workflows,
    TestCases,
    Organizations,
    Projects,
    Documents,
    Events,
    Metrics,
    Logs,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConstraints {
    pub field_constraints: HashMap<String, FieldConstraint>,
    pub business_rules: Vec<BusinessRule>,
    pub uniqueness_constraints: Vec<String>,
    pub referential_integrity: Vec<ForeignKeyConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConstraint {
    pub field_name: String,
    pub data_type: String,
    pub min_value: Option<serde_json::Value>,
    pub max_value: Option<serde_json::Value>,
    pub pattern: Option<String>,
    pub enum_values: Option<Vec<String>>,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRule {
    pub name: String,
    pub description: String,
    pub condition: String,
    pub action: String,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyConstraint {
    pub source_field: String,
    pub target_table: String,
    pub target_field: String,
    pub cascade_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRelationship {
    pub parent_type: DataType,
    pub child_type: DataType,
    pub relationship_type: RelationshipType,
    pub cardinality: Cardinality,
    pub foreign_key_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToMany,
    Hierarchical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Cardinality {
    Required,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Csv,
    Sql,
    Excel,
    Yaml,
    Xml,
}

// ============================================================================
// Test Execution Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TestExecution {
    pub id: Uuid,
    pub test_case_id: Uuid,
    pub environment_id: Uuid,
    pub executor_id: Uuid,
    pub status: ExecutionStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub result: Option<TestResult>,
    pub error_message: Option<String>,
    pub logs: Vec<String>,
    pub metrics: serde_json::Value,
    pub artifacts: Vec<TestArtifact>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "execution_status", rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    Timeout,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub status: ExecutionStatus,
    pub assertions_passed: i32,
    pub assertions_failed: i32,
    pub output_data: serde_json::Value,
    pub performance_metrics: PerformanceMetrics,
    pub screenshots: Vec<String>,
    pub error_details: Option<ErrorDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub response_time_ms: i64,
    pub throughput_rps: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub database_query_count: i32,
    pub database_query_time_ms: i64,
    pub network_requests: i32,
    pub network_time_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: String,
    pub error_code: Option<String>,
    pub message: String,
    pub stack_trace: Option<String>,
    pub context: serde_json::Value,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestArtifact {
    pub id: Uuid,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub file_path: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    Screenshot,
    Video,
    Log,
    Report,
    Trace,
    Heap,
    Network,
    Database,
    Config,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateEnvironmentRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub environment_type: EnvironmentType,
    pub configuration: EnvironmentConfig,
    pub database_configs: HashMap<String, DatabaseConfig>,
    pub service_configs: HashMap<String, ServiceConfig>,
    pub expires_after_hours: Option<i32>,
    pub auto_cleanup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentResponse {
    pub environment: TestEnvironment,
    pub status_url: String,
    pub dashboard_url: Option<String>,
    pub api_endpoints: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GenerateDataRequest {
    pub data_generation: DataGenerationRequest,
    pub target_environment: String,
    pub cleanup_strategy: CleanupStrategy,
    pub notification_webhook: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupStrategy {
    Immediate,
    AfterTest,
    AfterHours(i32),
    Manual,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataGenerationResponse {
    pub generation_id: Uuid,
    pub status: String,
    pub estimated_completion_time: DateTime<Utc>,
    pub progress_url: String,
    pub generated_count: i32,
    pub total_count: i32,
    pub data_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRequest {
    pub environment_ids: Vec<Uuid>,
    pub cleanup_type: CleanupType,
    pub force: bool,
    pub backup_before_cleanup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupType {
    Users,
    Workflows,
    TestData,
    Environments,
    Artifacts,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResponse {
    pub cleanup_id: Uuid,
    pub status: String,
    pub items_to_cleanup: i32,
    pub estimated_duration_seconds: i32,
    pub progress_url: String,
}

// ============================================================================
// Health and Status Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service_name: String,
    pub version: String,
    pub status: ServiceHealthStatus,
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: i64,
    pub database_connections: DatabaseHealthStatus,
    pub external_services: Vec<ExternalServiceHealth>,
    pub metrics: ServiceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealthStatus {
    pub postgresql: ConnectionHealth,
    pub mongodb: ConnectionHealth,
    pub redis: ConnectionHealth,
    pub clickhouse: ConnectionHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub status: ServiceHealthStatus,
    pub connection_count: i32,
    pub max_connections: i32,
    pub response_time_ms: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceHealth {
    pub service_name: String,
    pub url: String,
    pub status: ServiceHealthStatus,
    pub response_time_ms: i64,
    pub last_check: DateTime<Utc>,
    pub error_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub active_connections: i32,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

// ============================================================================
// Error Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub error_code: String,
    pub message: String,
    pub rejected_value: Option<serde_json::Value>,
}

// ============================================================================
// Implementation helpers
// ============================================================================

impl Default for TestUser {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: String::new(),
            email: String::new(),
            password_hash: String::new(),
            first_name: None,
            last_name: None,
            role: UserRole::User,
            permissions: Vec::new(),
            metadata: serde_json::Value::Null,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
            test_environment: "default".to_string(),
            cleanup_after: None,
        }
    }
}

impl TestEnvironment {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn should_cleanup(&self) -> bool {
        self.auto_cleanup && (self.is_expired() || self.status == EnvironmentStatus::Error)
    }
}

impl TestExecution {
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Passed
                | ExecutionStatus::Failed
                | ExecutionStatus::Skipped
                | ExecutionStatus::Timeout
                | ExecutionStatus::Error
                | ExecutionStatus::Cancelled
        )
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        if let Some(end_time) = self.end_time {
            Some(end_time - self.start_time)
        } else {
            None
        }
    }
}

// ============================================================================
// Display implementations for better debugging
// ============================================================================

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::User => write!(f, "user"),
            UserRole::Viewer => write!(f, "viewer"),
            UserRole::Manager => write!(f, "manager"),
            UserRole::Developer => write!(f, "developer"),
            UserRole::Tester => write!(f, "tester"),
            UserRole::Guest => write!(f, "guest"),
        }
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "pending"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Passed => write!(f, "passed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Skipped => write!(f, "skipped"),
            ExecutionStatus::Timeout => write!(f, "timeout"),
            ExecutionStatus::Error => write!(f, "error"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}
