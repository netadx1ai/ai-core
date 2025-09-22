//! Data models for MCP Manager Service
//!
//! This module defines all the data structures used throughout the MCP Manager Service,
//! including server information, protocol messages, health status, and API request/response types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// MCP Server information and metadata
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServerInfo {
    /// Unique server identifier
    pub id: Uuid,

    /// Server name
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// Server description
    #[validate(length(max = 500))]
    pub description: Option<String>,

    /// Server version
    #[validate(length(min = 1, max = 50))]
    pub version: String,

    /// Server type (e.g., "filesystem", "database", "api")
    #[validate(length(min = 1, max = 50))]
    pub server_type: String,

    /// Server configuration
    pub config: ServerConfig,

    /// Server status
    pub status: ServerStatus,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Last health check timestamp
    pub last_health_check: Option<DateTime<Utc>>,

    /// Server tags for categorization
    pub tags: Vec<String>,

    /// Server owner/creator
    pub owner: Option<String>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServerConfig {
    /// Server endpoint URL
    #[validate(url)]
    pub endpoint: String,

    /// Server port
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,

    /// Server host
    #[validate(length(min = 1))]
    pub host: String,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub timeout_seconds: u64,

    /// Maximum concurrent connections
    #[validate(range(min = 1, max = 1000))]
    pub max_connections: u32,

    /// Server-specific configuration
    pub settings: HashMap<String, serde_json::Value>,

    /// Environment variables
    pub environment: HashMap<String, String>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// SSL/TLS configuration
    pub ssl: Option<SslConfig>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,

    /// Authentication credentials
    pub credentials: HashMap<String, String>,
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    None,
    ApiKey,
    Bearer,
    Basic,
    OAuth2,
    Certificate,
}

/// SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// Enable SSL/TLS
    pub enabled: bool,

    /// Certificate path
    pub cert_path: Option<String>,

    /// Private key path
    pub key_path: Option<String>,

    /// CA certificate path
    pub ca_path: Option<String>,

    /// Verify SSL certificates
    pub verify_certs: bool,
}

/// Server status enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    /// Server is starting up
    Starting,
    /// Server is running and healthy
    Running,
    /// Server is running but unhealthy
    Unhealthy,
    /// Server is stopping
    Stopping,
    /// Server is stopped
    Stopped,
    /// Server has failed
    Failed,
    /// Server status is unknown
    Unknown,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Supported MCP protocol version
    pub protocol_version: String,

    /// Supported tools/functions
    pub tools: Vec<ToolInfo>,

    /// Supported resources
    pub resources: Vec<ResourceInfo>,

    /// Supported prompts
    pub prompts: Vec<PromptInfo>,

    /// Server features
    pub features: Vec<String>,

    /// Maximum request size in bytes
    pub max_request_size: Option<u64>,

    /// Maximum response size in bytes
    pub max_response_size: Option<u64>,

    /// Supported content types
    pub content_types: Vec<String>,
}

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Tool schema
    pub schema: serde_json::Value,

    /// Tool tags
    pub tags: Vec<String>,
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource URI
    pub uri: String,

    /// Resource name
    pub name: String,

    /// Resource description
    pub description: String,

    /// Resource MIME type
    pub mime_type: String,

    /// Resource tags
    pub tags: Vec<String>,
}

/// Prompt information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// Prompt name
    pub name: String,

    /// Prompt description
    pub description: String,

    /// Prompt arguments
    pub arguments: Vec<PromptArgument>,

    /// Prompt tags
    pub tags: Vec<String>,
}

/// Prompt argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,

    /// Argument description
    pub description: String,

    /// Argument type
    pub arg_type: String,

    /// Whether argument is required
    pub required: bool,

    /// Default value
    pub default: Option<serde_json::Value>,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Server ID
    pub server_id: Uuid,

    /// Health status
    pub status: HealthStatus,

    /// Health check timestamp
    pub timestamp: DateTime<Utc>,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Health check details
    pub details: HealthDetails,

    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Health status enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
    Unknown,
}

/// Health check details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDetails {
    /// CPU usage percentage
    pub cpu_usage: Option<f64>,

    /// Memory usage in bytes
    pub memory_usage: Option<u64>,

    /// Disk usage percentage
    pub disk_usage: Option<f64>,

    /// Network latency in milliseconds
    pub network_latency: Option<u64>,

    /// Active connections count
    pub active_connections: Option<u32>,

    /// Request rate (requests per second)
    pub request_rate: Option<f64>,

    /// Error rate percentage
    pub error_rate: Option<f64>,

    /// Additional metrics
    pub metrics: HashMap<String, serde_json::Value>,
}

/// MCP Protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    /// Message ID
    pub id: Option<String>,

    /// Message method
    pub method: String,

    /// Message parameters
    pub params: Option<serde_json::Value>,

    /// Message result (for responses)
    pub result: Option<serde_json::Value>,

    /// Message error (for error responses)
    pub error: Option<McpError>,

    /// Protocol version
    pub jsonrpc: String,
}

/// MCP Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code
    pub code: i32,

    /// Error message
    pub message: String,

    /// Additional error data
    pub data: Option<serde_json::Value>,
}

/// Server registration request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterServerRequest {
    /// Server name
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// Server description
    #[validate(length(max = 500))]
    pub description: Option<String>,

    /// Server version
    #[validate(length(min = 1, max = 50))]
    pub version: String,

    /// Server type
    #[validate(length(min = 1, max = 50))]
    pub server_type: String,

    /// Server configuration
    #[validate]
    pub config: ServerConfig,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Server tags
    pub tags: Option<Vec<String>>,

    /// Server owner
    pub owner: Option<String>,
}

/// Server registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterServerResponse {
    /// Server ID
    pub server_id: Uuid,

    /// Registration status
    pub status: String,

    /// Registration message
    pub message: String,

    /// Server endpoint for communication
    pub endpoint: String,
}

/// Server update request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateServerRequest {
    /// Server name
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    /// Server description
    #[validate(length(max = 500))]
    pub description: Option<String>,

    /// Server version
    #[validate(length(min = 1, max = 50))]
    pub version: Option<String>,

    /// Server configuration
    pub config: Option<ServerConfig>,

    /// Server capabilities
    pub capabilities: Option<ServerCapabilities>,

    /// Server metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Server tags
    pub tags: Option<Vec<String>>,
}

/// Server list request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ListServersRequest {
    /// Filter by server status
    pub status: Option<ServerStatus>,

    /// Filter by server type
    pub server_type: Option<String>,

    /// Filter by tags
    pub tags: Option<Vec<String>>,

    /// Filter by owner
    pub owner: Option<String>,

    /// Page number
    #[validate(range(min = 1))]
    pub page: Option<u32>,

    /// Page size
    #[validate(range(min = 1, max = 100))]
    pub page_size: Option<u32>,

    /// Sort by field
    pub sort_by: Option<String>,

    /// Sort order (asc, desc)
    pub sort_order: Option<String>,
}

/// Server list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListServersResponse {
    /// List of servers
    pub servers: Vec<ServerInfo>,

    /// Total number of servers
    pub total: u64,

    /// Current page
    pub page: u32,

    /// Page size
    pub page_size: u32,

    /// Total pages
    pub total_pages: u32,
}

/// Server metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    /// Server ID
    pub server_id: Uuid,

    /// Metrics timestamp
    pub timestamp: DateTime<Utc>,

    /// Request count
    pub request_count: u64,

    /// Error count
    pub error_count: u64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// Request rate (requests per second)
    pub request_rate: f64,

    /// Error rate percentage
    pub error_rate: f64,

    /// CPU usage percentage
    pub cpu_usage: Option<f64>,

    /// Memory usage in bytes
    pub memory_usage: Option<u64>,

    /// Network throughput in bytes per second
    pub network_throughput: Option<u64>,

    /// Additional metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Load balancer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    /// Total requests processed
    pub total_requests: u64,

    /// Total errors
    pub total_errors: u64,

    /// Active connections
    pub active_connections: u32,

    /// Server distribution
    pub server_distribution: HashMap<Uuid, u64>,

    /// Current strategy
    pub current_strategy: String,

    /// Circuit breaker status
    pub circuit_breaker_status: HashMap<Uuid, String>,

    /// Statistics timestamp
    pub timestamp: DateTime<Utc>,
}

/// MCP request routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRouting {
    /// Request ID
    pub request_id: String,

    /// Target server ID
    pub server_id: Uuid,

    /// Request method
    pub method: String,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Request priority
    pub priority: u32,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Batch operation request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchOperationRequest {
    /// List of operations
    #[validate(length(min = 1, max = 100))]
    pub operations: Vec<BatchOperation>,

    /// Batch execution mode
    pub execution_mode: BatchExecutionMode,

    /// Continue on error
    pub continue_on_error: bool,

    /// Batch timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub timeout_seconds: u64,
}

/// Individual batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation {
    /// Operation ID
    pub operation_id: String,

    /// Target server ID
    pub server_id: Uuid,

    /// Operation method
    pub method: String,

    /// Operation parameters
    pub params: Option<serde_json::Value>,

    /// Operation priority
    pub priority: u32,
}

/// Batch execution modes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchExecutionMode {
    Sequential,
    Parallel,
    Pipeline,
}

/// Batch operation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResponse {
    /// Batch ID
    pub batch_id: String,

    /// Operation results
    pub results: Vec<BatchOperationResult>,

    /// Overall batch status
    pub status: BatchStatus,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Success count
    pub success_count: u32,

    /// Error count
    pub error_count: u32,
}

/// Individual batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    /// Operation ID
    pub operation_id: String,

    /// Operation status
    pub status: OperationStatus,

    /// Operation result
    pub result: Option<serde_json::Value>,

    /// Operation error
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Success,
    Error,
    Timeout,
    Cancelled,
}

/// Batch status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Default for AuthType {
    fn default() -> Self {
        Self::None
    }
}

impl ServerInfo {
    /// Create a new server info
    pub fn new(
        name: String,
        version: String,
        server_type: String,
        config: ServerConfig,
        capabilities: ServerCapabilities,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            version,
            server_type,
            config,
            status: ServerStatus::Unknown,
            capabilities,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            last_health_check: None,
            tags: Vec::new(),
            owner: None,
        }
    }

    /// Check if server is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, ServerStatus::Running)
    }

    /// Check if server is available for requests
    pub fn is_available(&self) -> bool {
        matches!(self.status, ServerStatus::Running | ServerStatus::Unhealthy)
    }

    /// Get server endpoint URL
    pub fn endpoint_url(&self) -> String {
        self.config.endpoint.clone()
    }

    /// Update server status
    pub fn update_status(&mut self, status: ServerStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Update last health check timestamp
    pub fn update_health_check(&mut self) {
        self.last_health_check = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

impl HealthCheck {
    /// Create a new health check result
    pub fn new(server_id: Uuid, status: HealthStatus, response_time_ms: u64) -> Self {
        Self {
            server_id,
            status,
            timestamp: Utc::now(),
            response_time_ms,
            details: HealthDetails::default(),
            error: None,
        }
    }

    /// Create a failed health check
    pub fn failed(server_id: Uuid, error: String) -> Self {
        Self {
            server_id,
            status: HealthStatus::Unhealthy,
            timestamp: Utc::now(),
            response_time_ms: 0,
            details: HealthDetails::default(),
            error: Some(error),
        }
    }
}

impl Default for HealthDetails {
    fn default() -> Self {
        Self {
            cpu_usage: None,
            memory_usage: None,
            disk_usage: None,
            network_latency: None,
            active_connections: None,
            request_rate: None,
            error_rate: None,
            metrics: HashMap::new(),
        }
    }
}

impl McpMessage {
    /// Create a new request message
    pub fn request(id: String, method: String, params: Option<serde_json::Value>) -> Self {
        Self {
            id: Some(id),
            method,
            params,
            result: None,
            error: None,
            jsonrpc: "2.0".to_string(),
        }
    }

    /// Create a new response message
    pub fn response(id: String, result: serde_json::Value) -> Self {
        Self {
            id: Some(id),
            method: String::new(),
            params: None,
            result: Some(result),
            error: None,
            jsonrpc: "2.0".to_string(),
        }
    }

    /// Create a new error response message
    pub fn error_response(id: String, error: McpError) -> Self {
        Self {
            id: Some(id),
            method: String::new(),
            params: None,
            result: None,
            error: Some(error),
            jsonrpc: "2.0".to_string(),
        }
    }

    /// Check if this is a request message
    pub fn is_request(&self) -> bool {
        !self.method.is_empty() && self.result.is_none() && self.error.is_none()
    }

    /// Check if this is a response message
    pub fn is_response(&self) -> bool {
        self.method.is_empty() && (self.result.is_some() || self.error.is_some())
    }
}
