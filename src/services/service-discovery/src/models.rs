//! Service Discovery Models
//!
//! Core data structures and models for the service discovery and registry service.
//! Provides types for service registration, health monitoring, load balancing, and configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Service registration information
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServiceRegistration {
    /// Unique service identifier
    pub id: Uuid,

    /// Service name (e.g., "user-service", "auth-service")
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// Service version using semantic versioning
    #[validate(length(min = 1, max = 50))]
    pub version: String,

    /// Network address where service is running
    #[validate(length(min = 1, max = 255))]
    pub address: String,

    /// Port number where service is listening
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,

    /// Service protocol (http, https, grpc, tcp)
    pub protocol: ServiceProtocol,

    /// Service health check configuration
    pub health_check: Option<HealthCheckConfig>,

    /// Service metadata and tags
    pub metadata: HashMap<String, String>,

    /// Load balancing weight (1-1000)
    #[validate(range(min = 1, max = 1000))]
    pub weight: u32,

    /// Service status
    pub status: ServiceStatus,

    /// Registration timestamp
    pub registered_at: DateTime<Utc>,

    /// Last heartbeat timestamp
    pub last_heartbeat: Option<DateTime<Utc>>,

    /// Service TTL in seconds
    #[validate(range(min = 10, max = 3600))]
    pub ttl: u32,

    /// Service dependencies
    pub dependencies: Vec<ServiceDependency>,

    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

/// Service protocol enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceProtocol {
    Http,
    Https,
    Grpc,
    Tcp,
    Udp,
}

/// Service status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    /// Service is healthy and ready to serve traffic
    Healthy,
    /// Service is unhealthy but still registered
    Unhealthy,
    /// Service is starting up
    Starting,
    /// Service is shutting down gracefully
    Stopping,
    /// Service registration has expired
    Expired,
    /// Service is in maintenance mode
    Maintenance,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HealthCheckConfig {
    /// Health check type
    pub check_type: HealthCheckType,

    /// Health check interval in seconds
    #[validate(range(min = 5, max = 300))]
    pub interval: u32,

    /// Health check timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout: u32,

    /// Number of consecutive failures before marking unhealthy
    #[validate(range(min = 1, max = 10))]
    pub failure_threshold: u32,

    /// Number of consecutive successes before marking healthy
    #[validate(range(min = 1, max = 10))]
    pub success_threshold: u32,

    /// Health check specific configuration
    pub config: HealthCheckTypeConfig,
}

/// Health check type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckType {
    Http,
    Tcp,
    Grpc,
    Script,
}

/// Health check type-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HealthCheckTypeConfig {
    Http {
        path: String,
        method: String,
        headers: HashMap<String, String>,
        expected_status: u16,
        expected_body: Option<String>,
    },
    Tcp,
    Grpc {
        service_name: String,
    },
    Script {
        command: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
}

/// Service dependency information
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServiceDependency {
    /// Name of the dependent service
    #[validate(length(min = 1, max = 100))]
    pub service_name: String,

    /// Version constraint (e.g., ">=1.0.0", "~1.2.0")
    pub version_constraint: Option<String>,

    /// Whether this dependency is required or optional
    pub required: bool,

    /// Dependency relationship type
    pub relationship: DependencyRelationship,
}

/// Dependency relationship type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyRelationship {
    /// Service cannot function without this dependency
    Required,
    /// Service can function with reduced capabilities
    Optional,
    /// Service provides data to this dependency
    Producer,
    /// Service consumes data from this dependency
    Consumer,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    #[validate(range(min = 1, max = 100))]
    pub failure_threshold: u32,

    /// Recovery timeout in seconds
    #[validate(range(min = 5, max = 300))]
    pub recovery_timeout: u32,

    /// Number of successful calls to close circuit
    #[validate(range(min = 1, max = 10))]
    pub success_threshold: u32,

    /// Request timeout in seconds
    #[validate(range(min = 1, max = 120))]
    pub timeout: u32,

    /// Maximum number of retries
    #[validate(range(min = 0, max = 10))]
    pub max_retries: u32,
}

/// Service discovery query
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServiceDiscoveryQuery {
    /// Service name to discover
    #[validate(length(min = 1, max = 100))]
    pub service_name: String,

    /// Version constraint
    pub version: Option<String>,

    /// Required tags/metadata
    pub tags: HashMap<String, String>,

    /// Load balancing strategy
    pub load_balancing_strategy: Option<LoadBalancingStrategy>,

    /// Include unhealthy services
    pub include_unhealthy: bool,

    /// Maximum number of services to return
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

/// Load balancing strategy enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    ConsistentHash,
    Random,
    IpHash,
}

/// Service discovery response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryResponse {
    /// List of matching services
    pub services: Vec<ServiceInstance>,

    /// Total number of matching services
    pub total: u32,

    /// Load balancing strategy used
    pub strategy: LoadBalancingStrategy,

    /// Response timestamp
    pub timestamp: DateTime<Utc>,

    /// Cache TTL in seconds
    pub cache_ttl: u32,
}

/// Service instance (simplified view for discovery)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// Service ID
    pub id: Uuid,

    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Service address
    pub address: String,

    /// Service port
    pub port: u16,

    /// Service protocol
    pub protocol: ServiceProtocol,

    /// Service status
    pub status: ServiceStatus,

    /// Load balancing weight
    pub weight: u32,

    /// Service metadata
    pub metadata: HashMap<String, String>,

    /// Last health check timestamp
    pub last_health_check: Option<DateTime<Utc>>,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Service ID
    pub service_id: Uuid,

    /// Health check status
    pub status: HealthStatus,

    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Check timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional check details
    pub details: HashMap<String, String>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
    Timeout,
}

/// Load balancer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    /// Service name
    pub service_name: String,

    /// Total requests handled
    pub total_requests: u64,

    /// Active connections per instance
    pub active_connections: HashMap<Uuid, u32>,

    /// Response time percentiles per instance
    pub response_times: HashMap<Uuid, ResponseTimeStats>,

    /// Error rate per instance
    pub error_rates: HashMap<Uuid, f64>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Response time statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeStats {
    /// Average response time in milliseconds
    pub avg_ms: f64,

    /// 50th percentile
    pub p50_ms: f64,

    /// 95th percentile
    pub p95_ms: f64,

    /// 99th percentile
    pub p99_ms: f64,

    /// Maximum response time
    pub max_ms: f64,
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServiceConfiguration {
    /// Service name
    #[validate(length(min = 1, max = 100))]
    pub service_name: String,

    /// Configuration version
    pub version: u32,

    /// Configuration data as JSON
    pub config_data: serde_json::Value,

    /// Configuration schema version
    pub schema_version: String,

    /// Environment (dev, staging, prod)
    pub environment: String,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Created by user/service
    pub created_by: String,
}

/// Service mesh route
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServiceRoute {
    /// Route ID
    pub id: Uuid,

    /// Source service pattern
    #[validate(length(min = 1, max = 100))]
    pub source_pattern: String,

    /// Destination service
    #[validate(length(min = 1, max = 100))]
    pub destination_service: String,

    /// Path rewrite rules
    pub path_rewrites: Vec<PathRewrite>,

    /// Header modifications
    pub header_modifications: Vec<HeaderModification>,

    /// Route weight for traffic splitting
    #[validate(range(min = 0, max = 100))]
    pub weight: u8,

    /// Route priority (lower number = higher priority)
    pub priority: u32,

    /// Route conditions
    pub conditions: Vec<RouteCondition>,

    /// Route timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub timeout: u32,

    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
}

/// Path rewrite rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRewrite {
    /// Pattern to match (regex)
    pub pattern: String,

    /// Replacement string
    pub replacement: String,
}

/// Header modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderModification {
    /// Operation type
    pub operation: HeaderOperation,

    /// Header name
    pub header_name: String,

    /// Header value (for add/set operations)
    pub header_value: Option<String>,
}

/// Header operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HeaderOperation {
    Add,
    Set,
    Remove,
}

/// Route condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCondition {
    /// Condition type
    pub condition_type: ConditionType,

    /// Condition value
    pub value: String,

    /// Whether condition should be negated
    pub negate: bool,
}

/// Route condition type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    HeaderMatch,
    PathMatch,
    QueryParam,
    Method,
    SourceIp,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RetryPolicy {
    /// Maximum number of retries
    #[validate(range(min = 0, max = 10))]
    pub max_retries: u32,

    /// Initial retry interval in milliseconds
    #[validate(range(min = 100, max = 30000))]
    pub initial_interval_ms: u32,

    /// Maximum retry interval in milliseconds
    #[validate(range(min = 1000, max = 60000))]
    pub max_interval_ms: u32,

    /// Backoff multiplier
    #[validate(range(min = 1.0, max = 5.0))]
    pub multiplier: f64,

    /// Retryable status codes
    pub retryable_status_codes: Vec<u16>,
}

/// API request/response models

/// Register service request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterServiceRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 1, max = 50))]
    pub version: String,

    #[validate(length(min = 1, max = 255))]
    pub address: String,

    #[validate(range(min = 1, max = 65535))]
    pub port: u16,

    pub protocol: ServiceProtocol,

    pub health_check: Option<HealthCheckConfig>,

    pub metadata: Option<HashMap<String, String>>,

    #[validate(range(min = 1, max = 1000))]
    pub weight: Option<u32>,

    #[validate(range(min = 10, max = 3600))]
    pub ttl: Option<u32>,

    pub dependencies: Option<Vec<ServiceDependency>>,

    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

/// Service heartbeat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub service_id: Uuid,
    pub status: Option<ServiceStatus>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Update service request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateServiceRequest {
    pub status: Option<ServiceStatus>,
    pub weight: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
    pub health_check: Option<HealthCheckConfig>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

/// Service statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatistics {
    pub service_id: Uuid,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub last_24h_requests: u64,
    pub uptime_percentage: f64,
    pub last_updated: DateTime<Utc>,
}
