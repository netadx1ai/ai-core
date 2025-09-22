//! Configuration module for MCP Manager Service
//!
//! This module handles all configuration aspects for the MCP Manager Service,
//! including server settings, database connections, monitoring, and MCP-specific
//! configurations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use url::Url;
use validator::{Validate, ValidationError};

/// Main configuration structure for MCP Manager Service
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Config {
    /// Environment (development, staging, production)
    pub environment: String,

    /// Server configuration
    #[validate]
    pub server: ServerConfig,

    /// Database configuration
    #[validate]
    pub database: DatabaseConfig,

    /// Redis configuration
    #[validate]
    pub redis: RedisConfig,

    /// MCP-specific configuration
    #[validate]
    pub mcp: McpConfig,

    /// Health monitoring configuration
    #[validate]
    pub health: HealthConfig,

    /// Load balancing configuration
    #[validate]
    pub load_balancer: LoadBalancerConfig,

    /// Security configuration
    #[validate]
    pub security: SecurityConfig,

    /// Logging configuration
    #[validate]
    pub logging: LoggingConfig,

    /// Metrics configuration
    #[validate]
    pub metrics: MetricsConfig,

    /// Rate limiting configuration
    #[validate]
    pub rate_limiting: RateLimitingConfig,

    /// Integration configurations
    #[validate]
    pub integrations: IntegrationsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServerConfig {
    /// Server host
    #[validate(length(min = 1))]
    pub host: String,

    /// Server port
    #[validate(range(min = 1024, max = 65535))]
    pub port: u16,

    /// Maximum number of connections
    #[validate(range(min = 1, max = 10000))]
    pub max_connections: u32,

    /// Request timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub timeout_seconds: u64,

    /// Keep-alive timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub keep_alive_seconds: u64,

    /// Enable graceful shutdown
    pub graceful_shutdown: bool,

    /// Graceful shutdown timeout in seconds
    #[validate(range(min = 1, max = 120))]
    pub shutdown_timeout_seconds: u64,

    /// Worker threads (0 = auto-detect)
    pub worker_threads: usize,

    /// Enable HTTP/2
    pub enable_http2: bool,

    /// Enable compression
    pub enable_compression: bool,

    /// CORS settings
    pub cors: CorsConfig,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Allow credentials
    pub allow_credentials: bool,

    /// Max age in seconds
    #[validate(range(min = 0, max = 86400))]
    pub max_age_seconds: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RedisConfig {
    /// Redis URL
    #[validate(custom = "validate_redis_url")]
    pub url: String,

    /// Connection pool size
    #[validate(range(min = 1, max = 100))]
    pub pool_size: u32,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout_seconds: u64,

    /// Key prefix for this service
    #[validate(length(min = 1))]
    pub key_prefix: String,

    /// TTL for cached data in seconds
    #[validate(range(min = 60, max = 86400))]
    pub default_ttl_seconds: u64,
}

/// MCP-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct McpConfig {
    /// Maximum number of MCP servers
    #[validate(range(min = 1, max = 1000))]
    pub max_servers: u32,

    /// Default MCP server timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub default_timeout_seconds: u64,

    /// MCP protocol version
    #[validate(length(min = 1))]
    pub protocol_version: String,

    /// Server startup timeout in seconds
    #[validate(range(min = 5, max = 120))]
    pub startup_timeout_seconds: u64,

    /// Server shutdown timeout in seconds
    #[validate(range(min = 5, max = 60))]
    pub shutdown_timeout_seconds: u64,

    /// Auto-restart failed servers
    pub auto_restart: bool,

    /// Maximum restart attempts
    #[validate(range(min = 0, max = 10))]
    pub max_restart_attempts: u32,

    /// Restart backoff in seconds
    #[validate(range(min = 1, max = 300))]
    pub restart_backoff_seconds: u64,

    /// Server discovery settings
    pub discovery: ServerDiscoveryConfig,

    /// Default server configurations
    pub server_defaults: HashMap<String, serde_json::Value>,
}

/// Server discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServerDiscoveryConfig {
    /// Enable automatic server discovery
    pub enabled: bool,

    /// Discovery interval in seconds
    #[validate(range(min = 10, max = 3600))]
    pub interval_seconds: u64,

    /// Discovery sources
    pub sources: Vec<String>,

    /// Discovery timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout_seconds: u64,
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HealthConfig {
    /// Enable health monitoring
    pub enabled: bool,

    /// Health check interval in seconds
    #[validate(range(min = 1, max = 300))]
    pub check_interval_seconds: u64,

    /// Health check timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub check_timeout_seconds: u64,

    /// Number of failed checks before marking unhealthy
    #[validate(range(min = 1, max = 10))]
    pub failure_threshold: u32,

    /// Number of successful checks before marking healthy
    #[validate(range(min = 1, max = 10))]
    pub success_threshold: u32,

    /// Enable detailed health metrics
    pub detailed_metrics: bool,

    /// Health check endpoints
    pub endpoints: Vec<String>,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy
    pub strategy: LoadBalancingStrategy,

    /// Enable sticky sessions
    pub sticky_sessions: bool,

    /// Session timeout in seconds
    #[validate(range(min = 60, max = 7200))]
    pub session_timeout_seconds: u64,

    /// Maximum requests per server
    #[validate(range(min = 1, max = 10000))]
    pub max_requests_per_server: u32,

    /// Enable circuit breaker
    pub circuit_breaker: bool,

    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,
}

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    Random,
    IpHash,
    ConsistentHash,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CircuitBreakerConfig {
    /// Failure threshold percentage
    #[validate(range(min = 1, max = 100))]
    pub failure_threshold: u32,

    /// Minimum number of requests
    #[validate(range(min = 1, max = 1000))]
    pub min_requests: u32,

    /// Window size in seconds
    #[validate(range(min = 10, max = 3600))]
    pub window_seconds: u64,

    /// Recovery timeout in seconds
    #[validate(range(min = 10, max = 300))]
    pub recovery_timeout_seconds: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SecurityConfig {
    /// Enable JWT authentication
    pub jwt_enabled: bool,

    /// JWT secret key
    #[validate(length(min = 32))]
    pub jwt_secret: String,

    /// JWT expiration in seconds
    #[validate(range(min = 300, max = 86400))]
    pub jwt_expiration_seconds: u64,

    /// Enable API key authentication
    pub api_key_enabled: bool,

    /// Valid API keys
    pub api_keys: Vec<String>,

    /// Enable HTTPS only
    pub https_only: bool,

    /// TLS certificate path
    pub tls_cert_path: Option<String>,

    /// TLS private key path
    pub tls_key_path: Option<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoggingConfig {
    /// Log level
    #[validate(custom = "validate_log_level")]
    pub level: String,

    /// Log format (json, pretty, compact)
    #[validate(custom = "validate_log_format")]
    pub format: String,

    /// Enable console logging
    pub console: bool,

    /// Enable file logging
    pub file_enabled: bool,

    /// Log file path
    pub file_path: Option<String>,

    /// Log file rotation size in MB
    #[validate(range(min = 1, max = 1000))]
    pub file_max_size_mb: u64,

    /// Number of log files to keep
    #[validate(range(min = 1, max = 100))]
    pub file_max_files: u32,

    /// Enable structured logging
    pub structured: bool,

    /// Additional log fields
    pub fields: HashMap<String, String>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Metrics port
    #[validate(range(min = 1024, max = 65535))]
    pub port: u16,

    /// Metrics path
    #[validate(length(min = 1))]
    pub path: String,

    /// Enable Prometheus metrics
    pub prometheus_enabled: bool,

    /// Metrics collection interval in seconds
    #[validate(range(min = 1, max = 300))]
    pub collection_interval_seconds: u64,

    /// Enable custom metrics
    pub custom_metrics: bool,

    /// Metrics labels
    pub labels: HashMap<String, String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Global rate limit (requests per second)
    #[validate(range(min = 1, max = 10000))]
    pub global_rps: u32,

    /// Per-IP rate limit (requests per second)
    #[validate(range(min = 1, max = 1000))]
    pub per_ip_rps: u32,

    /// Rate limit window in seconds
    #[validate(range(min = 1, max = 3600))]
    pub window_seconds: u64,

    /// Enable burst capacity
    pub burst_enabled: bool,

    /// Burst capacity
    #[validate(range(min = 1, max = 1000))]
    pub burst_capacity: u32,
}

/// Integration configurations
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IntegrationsConfig {
    /// Intent Parser Service configuration
    #[validate]
    pub intent_parser: IntentParserConfig,

    /// API Gateway configuration
    #[validate]
    pub api_gateway: ApiGatewayConfig,

    /// External service configurations
    pub external_services: HashMap<String, ExternalServiceConfig>,
}

/// Intent Parser Service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IntentParserConfig {
    /// Intent Parser Service URL
    #[validate(custom = "validate_url")]
    pub url: String,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout_seconds: u64,

    /// Enable authentication
    pub auth_enabled: bool,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Enable health checks
    pub health_check_enabled: bool,

    /// Health check interval in seconds
    #[validate(range(min = 10, max = 300))]
    pub health_check_interval_seconds: u64,
}

/// API Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ApiGatewayConfig {
    /// API Gateway URL
    #[validate(custom = "validate_url")]
    pub url: String,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout_seconds: u64,

    /// Enable authentication
    pub auth_enabled: bool,

    /// API key for authentication
    pub api_key: Option<String>,
}

/// External service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExternalServiceConfig {
    /// Service URL
    #[validate(custom = "validate_url")]
    pub url: String,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 60))]
    pub timeout_seconds: u64,

    /// Enable authentication
    pub auth_enabled: bool,

    /// Authentication type
    pub auth_type: String,

    /// Authentication credentials
    pub auth_credentials: HashMap<String, String>,

    /// Enable health checks
    pub health_check_enabled: bool,

    /// Health check endpoint
    pub health_check_endpoint: Option<String>,
}

impl Config {
    /// Load configuration from file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path.as_ref())
            .await
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config =
            serde_yaml::from_str(&content).with_context(|| "Failed to parse configuration YAML")?;

        config
            .validate()
            .with_context(|| "Configuration validation failed")?;

        Ok(config)
    }

    /// Create default configuration
    pub fn default() -> Self {
        Self {
            environment: "development".to_string(),
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            mcp: McpConfig::default(),
            health: HealthConfig::default(),
            load_balancer: LoadBalancerConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            metrics: MetricsConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            integrations: IntegrationsConfig::default(),
        }
    }

    /// Get server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Get request timeout
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.server.timeout_seconds)
    }

    /// Get graceful shutdown timeout
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.server.shutdown_timeout_seconds)
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8083,
            max_connections: 1000,
            timeout_seconds: 30,
            keep_alive_seconds: 60,
            graceful_shutdown: true,
            shutdown_timeout_seconds: 30,
            worker_threads: 0,
            enable_http2: true,
            enable_compression: true,
            cors: CorsConfig::default(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Requested-With".to_string(),
            ],
            allow_credentials: true,
            max_age_seconds: 3600,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            timeout_seconds: 5,
            key_prefix: "mcp_manager".to_string(),
            default_ttl_seconds: 3600,
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            max_servers: 100,
            default_timeout_seconds: 30,
            protocol_version: "2024-11-05".to_string(),
            startup_timeout_seconds: 30,
            shutdown_timeout_seconds: 15,
            auto_restart: true,
            max_restart_attempts: 3,
            restart_backoff_seconds: 5,
            discovery: ServerDiscoveryConfig::default(),
            server_defaults: HashMap::new(),
        }
    }
}

impl Default for ServerDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 60,
            sources: vec![],
            timeout_seconds: 10,
        }
    }
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_seconds: 30,
            check_timeout_seconds: 5,
            failure_threshold: 3,
            success_threshold: 2,
            detailed_metrics: true,
            endpoints: vec!["/health".to_string()],
        }
    }
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            sticky_sessions: false,
            session_timeout_seconds: 1800,
            max_requests_per_server: 1000,
            circuit_breaker: true,
            circuit_breaker_config: CircuitBreakerConfig::default(),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 50,
            min_requests: 10,
            window_seconds: 60,
            recovery_timeout_seconds: 30,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_enabled: true,
            jwt_secret: "your-secret-key-here-must-be-at-least-32-characters-long".to_string(),
            jwt_expiration_seconds: 3600,
            api_key_enabled: false,
            api_keys: vec![],
            https_only: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            console: true,
            file_enabled: false,
            file_path: None,
            file_max_size_mb: 100,
            file_max_files: 5,
            structured: true,
            fields: HashMap::new(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9092,
            path: "/metrics".to_string(),
            prometheus_enabled: true,
            collection_interval_seconds: 15,
            custom_metrics: true,
            labels: HashMap::new(),
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_rps: 1000,
            per_ip_rps: 100,
            window_seconds: 60,
            burst_enabled: true,
            burst_capacity: 50,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DatabaseConfig {
    /// Database host
    #[validate(length(min = 1))]
    pub host: String,

    /// Database port
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,

    /// Database name
    #[validate(length(min = 1))]
    pub database: String,

    /// Database username
    #[validate(length(min = 1))]
    pub username: String,

    /// Database password
    pub password: String,

    /// Connection pool size
    #[validate(range(min = 1, max = 100))]
    pub pool_size: u32,

    /// Connection timeout in seconds
    #[validate(range(min = 1, max = 300))]
    pub timeout_seconds: u64,

    /// SSL mode
    pub ssl_mode: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "mcp_manager".to_string(),
            username: "postgres".to_string(),
            password: "password".to_string(),
            pool_size: 10,
            timeout_seconds: 30,
            ssl_mode: "prefer".to_string(),
        }
    }
}

impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            intent_parser: IntentParserConfig::default(),
            api_gateway: ApiGatewayConfig::default(),
            external_services: HashMap::new(),
        }
    }
}

impl Default for IntentParserConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8082".to_string(),
            timeout_seconds: 30,
            auth_enabled: false,
            api_key: None,
            health_check_enabled: true,
            health_check_interval_seconds: 60,
        }
    }
}

impl Default for ApiGatewayConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
            timeout_seconds: 30,
            auth_enabled: false,
            api_key: None,
        }
    }
}

// Validation functions
fn validate_redis_url(url: &str) -> Result<(), ValidationError> {
    if url.starts_with("redis://") || url.starts_with("rediss://") {
        Ok(())
    } else {
        Err(ValidationError::new("Invalid Redis URL format"))
    }
}

fn validate_url(url: &str) -> Result<(), ValidationError> {
    Url::parse(url)
        .map(|_| ())
        .map_err(|_| ValidationError::new("Invalid URL format"))
}

fn validate_log_level(level: &str) -> Result<(), ValidationError> {
    match level.to_lowercase().as_str() {
        "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
        _ => Err(ValidationError::new("Invalid log level")),
    }
}

fn validate_log_format(format: &str) -> Result<(), ValidationError> {
    match format.to_lowercase().as_str() {
        "json" | "pretty" | "compact" => Ok(()),
        _ => Err(ValidationError::new("Invalid log format")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_config_from_file() {
        let yaml_content = r#"
environment: "test"
server:
  host: "127.0.0.1"
  port: 8083
  max_connections: 1000
  timeout_seconds: 30
  keep_alive_seconds: 60
  graceful_shutdown: true
  shutdown_timeout_seconds: 30
  worker_threads: 0
  enable_http2: true
  enable_compression: true
  cors:
    allowed_origins: ["*"]
    allowed_methods: ["GET", "POST"]
    allowed_headers: ["Content-Type"]
    allow_credentials: true
    max_age_seconds: 3600
database:
  host: "localhost"
  port: 5432
  database: "test_db"
  username: "test_user"
  password: "test_pass"
  pool_size: 10
  timeout_seconds: 30
  ssl_mode: "prefer"
redis:
  url: "redis://localhost:6379"
  pool_size: 10
  timeout_seconds: 5
  key_prefix: "test"
  default_ttl_seconds: 3600
mcp:
  max_servers: 50
  default_timeout_seconds: 30
  protocol_version: "2024-11-05"
  startup_timeout_seconds: 30
  shutdown_timeout_seconds: 15
  auto_restart: true
  max_restart_attempts: 3
  restart_backoff_seconds: 5
  discovery:
    enabled: false
    interval_seconds: 60
    sources: []
    timeout_seconds: 10
  server_defaults: {}
health:
  enabled: true
  check_interval_seconds: 30
  check_timeout_seconds: 5
  failure_threshold: 3
  success_threshold: 2
  detailed_metrics: true
  endpoints: ["/health"]
load_balancer:
  strategy: "round_robin"
  sticky_sessions: false
  session_timeout_seconds: 1800
  max_requests_per_server: 1000
  circuit_breaker: true
  circuit_breaker_config:
    failure_threshold: 50
    min_requests: 10
    window_seconds: 60
    recovery_timeout_seconds: 30
security:
  jwt_enabled: true
  jwt_secret: "test-secret-key-that-is-long-enough"
  jwt_expiration_seconds: 3600
  api_key_enabled: false
  api_keys: []
  https_only: false
  tls_cert_path: null
  tls_key_path: null
logging:
  level: "info"
  format: "json"
  console: true
  file_enabled: false
  file_path: null
  file_max_size_mb: 100
  file_max_files: 5
  structured: true
  fields: {}
metrics:
  enabled: true
  port: 9092
  path: "/metrics"
  prometheus_enabled: true
  collection_interval_seconds: 15
  custom_metrics: true
  labels: {}
rate_limiting:
  enabled: true
  global_rps: 1000
  per_ip_rps: 100
  window_seconds: 60
  burst_enabled: true
  burst_capacity: 50
integrations:
  intent_parser:
    url: "http://localhost:8082"
    timeout_seconds: 30
    auth_enabled: false
    api_key: null
    health_check_enabled: true
    health_check_interval_seconds: 60
  api_gateway:
    url: "http://localhost:8080"
    timeout_seconds: 30
    auth_enabled: false
    api_key: null
  external_services: {}
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let config = Config::from_file(temp_file.path()).await.unwrap();
        assert_eq!(config.environment, "test");
        assert_eq!(config.server.port, 8083);
    }

    #[test]
    fn test_validation_functions() {
        assert!(validate_redis_url("redis://localhost:6379").is_ok());
        assert!(validate_redis_url("rediss://localhost:6379").is_ok());
        assert!(validate_redis_url("http://localhost:6379").is_err());

        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("invalid-url").is_err());

        assert!(validate_log_level("info").is_ok());
        assert!(validate_log_level("debug").is_ok());
        assert!(validate_log_level("invalid").is_err());

        assert!(validate_log_format("json").is_ok());
        assert!(validate_log_format("pretty").is_ok());
        assert!(validate_log_format("invalid").is_err());
    }
}
