//! Data models and structures for the AI-CORE Test Database Service
//!
//! This module contains all the data structures, enums, and types used throughout
//! the test database service for request/response handling, database interactions,
//! and internal state management.
//!
//! Version: 1.0.0
//! Created: 2025-01-11
//! Backend Agent: backend_agent
//! Classification: P0 Critical Path Foundation

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ===== CORE ENUMS AND TYPES =====

/// Supported database types in the testing infrastructure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    PostgreSQL,
    ClickHouse,
    MongoDB,
    Redis,
}

/// Database operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Health status for services and dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Environment types for test isolation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Local,
    CI,
    Staging,
    Performance,
    Security,
    Production,
}

/// Data seeding strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SeedingStrategy {
    Minimal,
    Standard,
    Comprehensive,
    Custom,
}

// ===== REQUEST MODELS =====

/// Request to setup a new test database
#[derive(Debug, Deserialize, Validate)]
pub struct SetupDatabaseRequest {
    #[validate(length(min = 1, max = 100))]
    pub database_type: DatabaseType,

    #[validate(length(min = 1, max = 255))]
    pub description: Option<String>,

    pub environment: Environment,

    pub configuration: DatabaseConfiguration,

    /// Whether to run migrations after setup
    pub run_migrations: bool,

    /// Whether to seed initial test data
    pub seed_data: bool,

    /// Custom tags for database identification
    pub tags: HashMap<String, String>,

    /// TTL for temporary databases (in seconds)
    pub ttl_seconds: Option<u64>,
}

/// Database configuration options
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DatabaseConfiguration {
    /// Connection pool settings
    pub pool_config: Option<PoolConfiguration>,

    /// Performance tuning parameters
    pub performance_config: Option<PerformanceConfiguration>,

    /// Security settings
    pub security_config: Option<SecurityConfiguration>,

    /// Backup and recovery settings
    pub backup_config: Option<BackupConfiguration>,

    /// Custom configuration parameters
    pub custom_params: HashMap<String, serde_json::Value>,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PoolConfiguration {
    #[validate(range(min = 1, max = 100))]
    pub min_connections: u32,

    #[validate(range(min = 1, max = 1000))]
    pub max_connections: u32,

    #[validate(range(min = 1, max = 3600))]
    pub connection_timeout_seconds: u64,

    #[validate(range(min = 1, max = 86400))]
    pub idle_timeout_seconds: u64,

    pub acquire_timeout_seconds: Option<u64>,
}

/// Performance configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfiguration {
    pub enable_query_cache: bool,
    pub cache_size_mb: Option<u64>,
    pub enable_prepared_statements: bool,
    pub statement_cache_size: Option<u32>,
    pub enable_connection_pooling: bool,
    pub max_query_execution_time_ms: Option<u64>,
}

/// Security configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    pub enable_ssl: bool,
    pub ssl_mode: Option<String>,
    pub enable_encryption_at_rest: bool,
    pub enable_audit_logging: bool,
    pub allowed_ip_ranges: Vec<String>,
    pub authentication_method: String,
}

/// Backup configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfiguration {
    pub enable_auto_backup: bool,
    pub backup_interval_hours: Option<u32>,
    pub retention_days: Option<u32>,
    pub backup_location: Option<String>,
    pub compression_enabled: bool,
}

/// Request to create a test dataset
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTestDatasetRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    pub description: Option<String>,

    pub dataset_type: TestDatasetType,

    pub environment: Environment,

    pub seeding_strategy: SeedingStrategy,

    pub data_volume: DataVolume,

    /// Schema version for compatibility tracking
    pub schema_version: Option<String>,

    /// Custom data generation rules
    pub generation_rules: Option<DataGenerationRules>,

    /// Tags for categorization
    pub tags: HashMap<String, String>,
}

/// Test dataset types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestDatasetType {
    Users,
    Workflows,
    Organizations,
    Sessions,
    Analytics,
    Synthetic,
    Custom,
}

/// Data volume specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataVolume {
    pub record_count: u64,
    pub size_estimate_mb: Option<f64>,
    pub complexity_level: ComplexityLevel,
}

/// Data generation complexity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComplexityLevel {
    Simple,
    Medium,
    Complex,
    Enterprise,
}

/// Rules for generating test data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataGenerationRules {
    pub use_realistic_data: bool,
    pub locale: Option<String>,
    pub date_range: Option<DateRange>,
    pub custom_generators: HashMap<String, serde_json::Value>,
    pub relationship_rules: Vec<RelationshipRule>,
}

/// Date range for temporal data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Rules for generating related data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRule {
    pub source_table: String,
    pub target_table: String,
    pub relationship_type: RelationshipType,
    pub cardinality: String, // e.g., "1:1", "1:many", "many:many"
}

/// Types of relationships between data entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// Request to update an existing test dataset
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTestDatasetRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,

    pub description: Option<String>,

    pub seeding_strategy: Option<SeedingStrategy>,

    pub data_volume: Option<DataVolume>,

    pub generation_rules: Option<DataGenerationRules>,

    pub tags: Option<HashMap<String, String>>,
}

/// Request to update database schema
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSchemaRequest {
    #[validate(length(min = 1))]
    pub schema_definition: String,

    pub migration_strategy: MigrationStrategy,

    pub validate_before_apply: bool,

    pub backup_before_migration: bool,

    pub rollback_on_failure: bool,
}

/// Schema migration strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStrategy {
    Immediate,
    Scheduled,
    BlueGreen,
    RollingUpdate,
}

/// Request to create a backup
#[derive(Debug, Deserialize, Validate)]
pub struct CreateBackupRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    pub description: Option<String>,

    pub backup_type: BackupType,

    pub include_data: bool,

    pub include_schema: bool,

    pub compression_level: Option<u8>,

    pub encryption_enabled: bool,
}

/// Types of database backups
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
    Schema,
    Data,
}

// ===== RESPONSE MODELS =====

/// Standard operation response
#[derive(Debug, Serialize)]
pub struct OperationResponse {
    pub success: bool,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Database setup response
#[derive(Debug, Serialize)]
pub struct DatabaseSetupResponse {
    pub database_id: Uuid,
    pub database_name: String,
    pub database_type: DatabaseType,
    pub connection_string: String, // Sanitized, no credentials
    pub status: OperationStatus,
    pub setup_duration_ms: u64,
    pub migrations_applied: u32,
    pub initial_data_seeded: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Test dataset response
#[derive(Debug, Serialize)]
pub struct TestDatasetResponse {
    pub dataset_id: Uuid,
    pub name: String,
    pub dataset_type: TestDatasetType,
    pub status: OperationStatus,
    pub records_created: u64,
    pub size_bytes: u64,
    pub generation_duration_ms: u64,
    pub created_at: DateTime<Utc>,
}

/// Basic health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Detailed health check response
#[derive(Debug, Serialize)]
pub struct DetailedHealthResponse {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub dependencies: HashMap<String, DependencyHealth>,
    pub system_metrics: SystemMetrics,
    pub database_connections: HashMap<String, ConnectionHealth>,
}

/// Health status of a dependency
#[derive(Debug, Serialize)]
pub struct DependencyHealth {
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub last_check: DateTime<Utc>,
    pub error_message: Option<String>,
}

/// System performance metrics
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub active_connections: u32,
    pub requests_per_minute: u32,
}

/// Database connection health information
#[derive(Debug, Serialize)]
pub struct ConnectionHealth {
    pub status: HealthStatus,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub average_query_time_ms: f64,
    pub slow_query_count: u32,
}

/// Version information response
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
    pub build_date: String,
    pub git_commit: String,
    pub rust_version: String,
}

/// Schema validation response
#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub is_valid: bool,
    pub validation_errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub schema_version: String,
    pub validated_at: DateTime<Utc>,
}

/// Schema validation error
#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub error_type: String,
    pub message: String,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub severity: ErrorSeverity,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Schema migration response
#[derive(Debug, Serialize)]
pub struct MigrationResponse {
    pub migration_id: Uuid,
    pub status: OperationStatus,
    pub migrations_applied: u32,
    pub migration_duration_ms: u64,
    pub rollback_available: bool,
    pub migration_log: Vec<MigrationLogEntry>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Migration log entry
#[derive(Debug, Serialize)]
pub struct MigrationLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Log levels for migration operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "uppercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Seed operation response
#[derive(Debug, Serialize)]
pub struct SeedResponse {
    pub seed_id: Uuid,
    pub status: OperationStatus,
    pub records_created: HashMap<String, u64>, // table_name -> count
    pub total_records: u64,
    pub seed_duration_ms: u64,
    pub data_size_bytes: u64,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Connection test response
#[derive(Debug, Serialize)]
pub struct ConnectionTestResponse {
    pub connection_name: String,
    pub is_successful: bool,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub connection_details: ConnectionDetails,
    pub tested_at: DateTime<Utc>,
}

/// Connection details (sanitized)
#[derive(Debug, Serialize)]
pub struct ConnectionDetails {
    pub database_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub ssl_enabled: bool,
    pub pool_size: Option<u32>,
}

/// Backup operation response
#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub backup_id: Uuid,
    pub backup_name: String,
    pub backup_type: BackupType,
    pub status: OperationStatus,
    pub file_size_bytes: Option<u64>,
    pub backup_duration_ms: u64,
    pub storage_location: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Restore operation response
#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    pub restore_id: Uuid,
    pub backup_id: Uuid,
    pub status: OperationStatus,
    pub restore_duration_ms: u64,
    pub records_restored: Option<u64>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Environment provisioning response
#[derive(Debug, Serialize)]
pub struct ProvisionResponse {
    pub environment_id: Uuid,
    pub environment_name: String,
    pub status: OperationStatus,
    pub services_provisioned: Vec<String>,
    pub provision_duration_ms: u64,
    pub endpoint_urls: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

// ===== INFO MODELS =====

/// Database information
#[derive(Debug, Serialize)]
pub struct DatabaseInfo {
    pub id: Uuid,
    pub name: String,
    pub database_type: DatabaseType,
    pub environment: Environment,
    pub status: OperationStatus,
    pub size_bytes: Option<u64>,
    pub table_count: Option<u32>,
    pub connection_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tags: HashMap<String, String>,
}

/// Database status information
#[derive(Debug, Serialize)]
pub struct DatabaseStatus {
    pub database_name: String,
    pub status: HealthStatus,
    pub connection_status: HealthStatus,
    pub performance_metrics: DatabasePerformanceMetrics,
    pub storage_info: StorageInfo,
    pub recent_operations: Vec<RecentOperation>,
    pub last_backup: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
}

/// Database performance metrics
#[derive(Debug, Serialize)]
pub struct DatabasePerformanceMetrics {
    pub queries_per_second: f64,
    pub average_response_time_ms: f64,
    pub slow_query_count: u32,
    pub cache_hit_ratio: Option<f64>,
    pub connection_utilization: f64,
    pub error_rate: f64,
}

/// Storage information
#[derive(Debug, Serialize)]
pub struct StorageInfo {
    pub total_size_bytes: u64,
    pub used_size_bytes: u64,
    pub available_size_bytes: u64,
    pub usage_percentage: f64,
    pub index_size_bytes: u64,
    pub data_size_bytes: u64,
}

/// Recent database operations
#[derive(Debug, Serialize)]
pub struct RecentOperation {
    pub operation_id: Uuid,
    pub operation_type: String,
    pub status: OperationStatus,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

/// Schema information
#[derive(Debug, Serialize)]
pub struct SchemaInfo {
    pub name: String,
    pub version: String,
    pub database_type: DatabaseType,
    pub table_count: u32,
    pub last_modified: DateTime<Utc>,
    pub checksum: String,
}

/// Schema definition
#[derive(Debug, Serialize)]
pub struct SchemaDefinition {
    pub name: String,
    pub version: String,
    pub database_type: DatabaseType,
    pub definition: String, // SQL DDL or JSON schema
    pub tables: Vec<TableDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub constraints: Vec<ConstraintDefinition>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

/// Table definition
#[derive(Debug, Serialize)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKeyDefinition>,
}

/// Column definition
#[derive(Debug, Serialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub constraints: Vec<String>,
}

/// Index definition
#[derive(Debug, Serialize)]
pub struct IndexDefinition {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub index_type: String,
}

/// Constraint definition
#[derive(Debug, Serialize)]
pub struct ConstraintDefinition {
    pub name: String,
    pub constraint_type: String,
    pub table_name: String,
    pub definition: String,
}

/// Foreign key definition
#[derive(Debug, Serialize)]
pub struct ForeignKeyDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Test dataset information
#[derive(Debug, Serialize)]
pub struct TestDatasetInfo {
    pub id: Uuid,
    pub name: String,
    pub dataset_type: TestDatasetType,
    pub environment: Environment,
    pub record_count: u64,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub tags: HashMap<String, String>,
}

/// Complete test dataset
#[derive(Debug, Serialize)]
pub struct TestDataset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub dataset_type: TestDatasetType,
    pub environment: Environment,
    pub seeding_strategy: SeedingStrategy,
    pub data_volume: DataVolume,
    pub generation_rules: Option<DataGenerationRules>,
    pub schema_version: Option<String>,
    pub status: OperationStatus,
    pub record_count: u64,
    pub size_bytes: u64,
    pub tables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u32,
    pub tags: HashMap<String, String>,
}

/// Connection information
#[derive(Debug, Serialize)]
pub struct ConnectionInfo {
    pub name: String,
    pub database_type: DatabaseType,
    pub environment: Environment,
    pub status: HealthStatus,
    pub connection_details: ConnectionDetails,
    pub pool_stats: Option<PoolStats>,
    pub created_at: DateTime<Utc>,
    pub last_health_check: DateTime<Utc>,
}

/// Connection pool statistics
#[derive(Debug, Serialize)]
pub struct PoolStats {
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub pending_connections: u32,
    pub total_created: u64,
    pub total_closed: u64,
}

/// Connection statistics
#[derive(Debug, Serialize)]
pub struct ConnectionStats {
    pub connection_name: String,
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub average_query_time_ms: f64,
    pub max_query_time_ms: u64,
    pub min_query_time_ms: u64,
    pub queries_per_second: f64,
    pub data_transferred_bytes: u64,
    pub last_query_time: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
}

/// Query performance statistics
#[derive(Debug, Serialize)]
pub struct QueryPerformanceStats {
    pub total_queries: u64,
    pub average_execution_time_ms: f64,
    pub p95_execution_time_ms: f64,
    pub p99_execution_time_ms: f64,
    pub slowest_query_time_ms: u64,
    pub fastest_query_time_ms: u64,
    pub queries_per_second: f64,
    pub error_rate: f64,
    pub cache_hit_rate: Option<f64>,
    pub top_slow_queries: Vec<SlowQuery>,
}

/// Slow query information
#[derive(Debug, Serialize)]
pub struct SlowQuery {
    pub query_hash: String,
    pub query_text: String,
    pub execution_time_ms: u64,
    pub execution_count: u32,
    pub average_time_ms: f64,
    pub last_executed: DateTime<Utc>,
    pub database_name: String,
    pub table_names: Vec<String>,
}

/// Monitoring alert
#[derive(Debug, Serialize)]
pub struct MonitoringAlert {
    pub alert_id: Uuid,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub source_component: String,
    pub metric_name: String,
    pub threshold_value: f64,
    pub current_value: f64,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub status: AlertStatus,
}

/// Alert types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    Performance,
    Availability,
    Security,
    Storage,
    Connection,
    ErrorRate,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Active,
    Resolved,
    Acknowledged,
    Suppressed,
}

/// Backup information
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: Uuid,
    pub name: String,
    pub backup_type: BackupType,
    pub database_name: String,
    pub size_bytes: u64,
    pub status: OperationStatus,
    pub storage_location: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

/// Environment information
#[derive(Debug, Serialize)]
pub struct EnvironmentInfo {
    pub id: Uuid,
    pub name: String,
    pub environment_type: Environment,
    pub status: OperationStatus,
    pub services: Vec<ServiceInfo>,
    pub databases: Vec<String>,
    pub resource_usage: ResourceUsage,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Service information within an environment
#[derive(Debug, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: String,
    pub status: HealthStatus,
    pub endpoint_url: String,
    pub version: String,
    pub resource_usage: ResourceUsage,
}

/// Resource usage information
#[derive(Debug, Serialize)]
pub struct ResourceUsage {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub disk_usage_mb: u64,
    pub network_io_mb: u64,
}

// ===== ERROR TYPES =====

/// Application-specific errors
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("MongoDB error: {0}")]
    MongoDB(String),

    #[error("ClickHouse error: {0}")]
    ClickHouse(String),

    #[error("Validation error: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Feature not implemented")]
    NotImplemented,
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, error_message) = match self {
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            AppError::Timeout(_) => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            AppError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            AppError::NotImplemented => (StatusCode::NOT_IMPLEMENTED, self.to_string()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
            "timestamp": Utc::now(),
            "status_code": status.as_u16()
        }));

        (status, body).into_response()
    }
}

// ===== UTILITY FUNCTIONS =====

impl Default for PoolConfiguration {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 50,
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            acquire_timeout_seconds: Some(30),
        }
    }
}

impl Default for PerformanceConfiguration {
    fn default() -> Self {
        Self {
            enable_query_cache: true,
            cache_size_mb: Some(128),
            enable_prepared_statements: true,
            statement_cache_size: Some(100),
            enable_connection_pooling: true,
            max_query_execution_time_ms: Some(30000),
        }
    }
}

impl Default for SecurityConfiguration {
    fn default() -> Self {
        Self {
            enable_ssl: true,
            ssl_mode: Some("require".to_string()),
            enable_encryption_at_rest: true,
            enable_audit_logging: true,
            allowed_ip_ranges: vec!["0.0.0.0/0".to_string()],
            authentication_method: "password".to_string(),
        }
    }
}

impl Default for BackupConfiguration {
    fn default() -> Self {
        Self {
            enable_auto_backup: true,
            backup_interval_hours: Some(24),
            retention_days: Some(7),
            backup_location: None,
            compression_enabled: true,
        }
    }
}

impl Default for DatabaseConfiguration {
    fn default() -> Self {
        Self {
            pool_config: Some(PoolConfiguration::default()),
            performance_config: Some(PerformanceConfiguration::default()),
            security_config: Some(SecurityConfiguration::default()),
            backup_config: Some(BackupConfiguration::default()),
            custom_params: HashMap::new(),
        }
    }
}
