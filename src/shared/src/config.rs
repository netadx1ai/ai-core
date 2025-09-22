//! Shared configuration types for the AI-CORE Platform
//!
//! This module provides common configuration structures used across all services
//! in the AI-CORE platform, ensuring consistency and type safety.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Server configuration for HTTP services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Maximum number of concurrent connections
    pub max_connections: Option<u32>,
    /// Request timeout duration in seconds
    pub timeout_seconds: u64,
    /// Enable TLS/HTTPS
    pub tls_enabled: bool,
    /// TLS certificate path (if TLS enabled)
    pub tls_cert_path: Option<String>,
    /// TLS private key path (if TLS enabled)
    pub tls_key_path: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            max_connections: Some(1000),
            timeout_seconds: 30,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_seconds: u64,
    /// Maximum lifetime of a connection in seconds
    pub max_lifetime_seconds: u64,
    /// Enable SSL/TLS for database connections
    pub ssl_mode: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost:5432/aicore".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout_seconds: 30,
            max_lifetime_seconds: 3600,
            ssl_mode: "prefer".to_string(),
        }
    }
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_seconds: u64,
    /// Command timeout in seconds
    pub command_timeout_seconds: u64,
    /// Enable TLS for Redis connections
    pub tls_enabled: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            max_connections: 50,
            connect_timeout_seconds: 30,
            command_timeout_seconds: 5,
            tls_enabled: false,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key for token signing
    pub jwt_secret: String,
    /// JWT token expiration time in seconds
    pub jwt_expiration_seconds: u64,
    /// JWT refresh token expiration time in seconds
    pub jwt_refresh_expiration_seconds: u64,
    /// Enable JWT refresh tokens
    pub enable_refresh_tokens: bool,
    /// JWT issuer
    pub jwt_issuer: String,
    /// JWT audience
    pub jwt_audience: String,
    /// Password minimum length
    pub password_min_length: usize,
    /// Password complexity requirements
    pub password_require_special: bool,
    /// Password complexity requirements
    pub password_require_numbers: bool,
    /// Password complexity requirements
    pub password_require_uppercase: bool,
    /// Maximum login attempts before lockout
    pub max_login_attempts: u32,
    /// Account lockout duration in seconds
    pub lockout_duration_seconds: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-super-secret-jwt-key-change-in-production".to_string(),
            jwt_expiration_seconds: 3600,           // 1 hour
            jwt_refresh_expiration_seconds: 604800, // 7 days
            enable_refresh_tokens: true,
            jwt_issuer: "ai-core-platform".to_string(),
            jwt_audience: "ai-core-users".to_string(),
            password_min_length: 8,
            password_require_special: true,
            password_require_numbers: true,
            password_require_uppercase: true,
            max_login_attempts: 5,
            lockout_duration_seconds: 1800, // 30 minutes
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Default requests per second limit
    pub requests_per_second: u32,
    /// Burst capacity
    pub burst_size: u32,
    /// Rate limiting strategy
    pub strategy: RateLimitStrategy,
    /// Custom rate limits per endpoint
    pub custom_limits: HashMap<String, RateLimitRule>,
    /// Redis connection for distributed rate limiting
    pub redis_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitStrategy {
    /// In-memory rate limiting (single instance)
    InMemory,
    /// Redis-based distributed rate limiting
    Distributed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitRule {
    /// Requests per second for this rule
    pub requests_per_second: u32,
    /// Burst capacity for this rule
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
            strategy: RateLimitStrategy::InMemory,
            custom_limits: HashMap::new(),
            redis_url: None,
        }
    }
}

/// Routing and service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Service discovery enabled
    pub discovery_enabled: bool,
    /// Default timeout for upstream services in seconds
    pub upstream_timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Retry backoff strategy
    pub retry_strategy: RetryStrategy,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
    /// Health check configuration
    pub health_check: HealthCheckConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed { delay_ms: u64 },
    /// Exponential backoff with jitter
    Exponential {
        initial_delay_ms: u64,
        max_delay_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Recovery timeout in seconds
    pub recovery_timeout_seconds: u64,
    /// Minimum number of requests before evaluating circuit state
    pub min_request_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round-robin load balancing
    RoundRobin,
    /// Least connections load balancing
    LeastConnections,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Random selection
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,
    /// Health check interval in seconds
    pub interval_seconds: u64,
    /// Health check timeout in seconds
    pub timeout_seconds: u64,
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
    /// Number of consecutive successes before marking healthy
    pub success_threshold: u32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            discovery_enabled: true,
            upstream_timeout_seconds: 30,
            max_retries: 3,
            retry_strategy: RetryStrategy::Exponential {
                initial_delay_ms: 100,
                max_delay_ms: 5000,
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 10,
                recovery_timeout_seconds: 60,
                min_request_threshold: 5,
            },
            load_balancing: LoadBalancingStrategy::RoundRobin,
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: 30,
                timeout_seconds: 10,
                failure_threshold: 3,
                success_threshold: 2,
            },
        }
    }
}

/// Service-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service description
    pub description: String,
    /// Service tags for discovery
    pub tags: Vec<String>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Service health check endpoint
    pub health_endpoint: String,
    /// Service metrics endpoint
    pub metrics_endpoint: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "ai-core-service".to_string(),
            version: "1.0.0".to_string(),
            description: "AI-CORE Platform Service".to_string(),
            tags: vec!["ai-core".to_string()],
            metadata: HashMap::new(),
            health_endpoint: "/health".to_string(),
            metrics_endpoint: "/metrics".to_string(),
        }
    }
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable distributed tracing
    pub tracing_enabled: bool,
    /// Jaeger endpoint for trace collection
    pub jaeger_endpoint: Option<String>,
    /// Enable metrics collection
    pub metrics_enabled: bool,
    /// Prometheus metrics endpoint
    pub metrics_endpoint: String,
    /// Log level
    pub log_level: String,
    /// Log format (json or pretty)
    pub log_format: LogFormat,
    /// Enable structured logging
    pub structured_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    /// JSON formatted logs
    Json,
    /// Pretty formatted logs for development
    Pretty,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            jaeger_endpoint: Some("http://jaeger:14268/api/traces".to_string()),
            metrics_enabled: true,
            metrics_endpoint: "/metrics".to_string(),
            log_level: "info".to_string(),
            log_format: LogFormat::Json,
            structured_logging: true,
        }
    }
}

/// External service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceConfig {
    /// Service name
    pub name: String,
    /// Base URL for the external service
    pub base_url: String,
    /// API key or token for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Enable retries for failed requests
    pub retry_enabled: bool,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Custom headers to include in requests
    pub headers: HashMap<String, String>,
}

impl Default for ExternalServiceConfig {
    fn default() -> Self {
        Self {
            name: "external-service".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: None,
            timeout_seconds: 30,
            retry_enabled: true,
            max_retries: 3,
            headers: HashMap::new(),
        }
    }
}

/// Temporal workflow engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    /// Temporal server address
    pub server_address: String,
    /// Namespace to use
    pub namespace: String,
    /// Task queue name
    pub task_queue: String,
    /// Worker configuration
    pub worker: TemporalWorkerConfig,
    /// Client configuration
    pub client: TemporalClientConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalWorkerConfig {
    /// Maximum number of concurrent workflow executions
    pub max_concurrent_workflows: usize,
    /// Maximum number of concurrent activity executions
    pub max_concurrent_activities: usize,
    /// Worker identity
    pub identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalClientConfig {
    /// Client timeout in seconds
    pub timeout_seconds: u64,
    /// Enable TLS
    pub tls_enabled: bool,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            server_address: "http://localhost:7233".to_string(),
            namespace: "default".to_string(),
            task_queue: "ai-core-task-queue".to_string(),
            worker: TemporalWorkerConfig {
                max_concurrent_workflows: 100,
                max_concurrent_activities: 200,
                identity: "ai-core-worker".to_string(),
            },
            client: TemporalClientConfig {
                timeout_seconds: 30,
                tls_enabled: false,
            },
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security features
    pub enabled: bool,
    /// CORS configuration
    pub cors: CorsConfig,
    /// Content Security Policy
    pub csp_header: Option<String>,
    /// Enable HSTS (HTTP Strict Transport Security)
    pub hsts_enabled: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u64,
    /// Enable X-Frame-Options header
    pub x_frame_options: Option<String>,
    /// Enable X-Content-Type-Options header
    pub x_content_type_options: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight requests in seconds
    pub max_age_seconds: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cors: CorsConfig {
                enabled: true,
                allowed_origins: vec!["*".to_string()],
                allowed_methods: vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "DELETE".to_string(),
                    "PATCH".to_string(),
                    "OPTIONS".to_string(),
                ],
                allowed_headers: vec![
                    "Content-Type".to_string(),
                    "Authorization".to_string(),
                    "Accept".to_string(),
                    "Origin".to_string(),
                    "X-Requested-With".to_string(),
                ],
                allow_credentials: true,
                max_age_seconds: 3600,
            },
            csp_header: Some("default-src 'self'".to_string()),
            hsts_enabled: true,
            hsts_max_age: 31536000, // 1 year
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: true,
        }
    }
}

/// Configuration utilities
impl ServerConfig {
    /// Get the full server address
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get request timeout as Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }
}

impl DatabaseConfig {
    /// Get connect timeout as Duration
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_seconds)
    }

    /// Get max lifetime as Duration
    pub fn max_lifetime(&self) -> Duration {
        Duration::from_secs(self.max_lifetime_seconds)
    }
}

impl RedisConfig {
    /// Get connect timeout as Duration
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_seconds)
    }

    /// Get command timeout as Duration
    pub fn command_timeout(&self) -> Duration {
        Duration::from_secs(self.command_timeout_seconds)
    }
}

impl AuthConfig {
    /// Get JWT expiration as Duration
    pub fn jwt_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt_expiration_seconds)
    }

    /// Get JWT refresh expiration as Duration
    pub fn jwt_refresh_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt_refresh_expiration_seconds)
    }

    /// Get lockout duration as Duration
    pub fn lockout_duration(&self) -> Duration {
        Duration::from_secs(self.lockout_duration_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8000);
        assert_eq!(config.address(), "127.0.0.1:8000");
    }

    #[test]
    fn test_auth_config_defaults() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt_expiration_seconds, 3600);
        assert_eq!(config.password_min_length, 8);
        assert!(config.enable_refresh_tokens);
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.requests_per_second, 100);
        assert_eq!(config.burst_size, 200);
    }

    #[test]
    fn test_routing_config_defaults() {
        let config = RoutingConfig::default();
        assert!(config.discovery_enabled);
        assert_eq!(config.upstream_timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_security_config_defaults() {
        let config = SecurityConfig::default();
        assert!(config.enabled);
        assert!(config.cors.enabled);
        assert!(config.hsts_enabled);
    }

    #[test]
    fn test_serde_serialization() {
        let config = ServerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.host, deserialized.host);
        assert_eq!(config.port, deserialized.port);
    }
}
