//! Configuration management for the Federation Service
//!
//! This module handles all configuration aspects for the federation service,
//! including YAML file parsing, environment variable overrides, CLI arguments,
//! and configuration validation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Main configuration structure for the Federation Service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Redis configuration
    pub redis: RedisConfig,
    /// Temporal workflow configuration
    pub temporal: TemporalConfig,
    /// Proxy configuration
    pub proxy: ProxyConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Cost optimization configuration
    pub cost_optimization: CostOptimizationConfig,
    /// Telemetry configuration
    pub telemetry: TelemetryConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
    /// Feature flags
    pub features: FeatureFlags,
    /// Environment-specific settings
    pub environment: Environment,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Keep-alive timeout in seconds
    pub keep_alive_timeout: u64,
    /// Maximum request size in bytes
    pub max_request_size: u64,
    /// Enable CORS
    pub enable_cors: bool,
    /// CORS origins
    pub cors_origins: Vec<String>,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    /// Graceful shutdown timeout
    pub shutdown_timeout: u64,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_file: String,
    /// Private key file path
    pub key_file: String,
    /// Enable TLS
    pub enabled: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Maximum number of connections
    pub max_connections: u32,
    /// Minimum number of connections
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Query timeout in seconds
    pub query_timeout: u64,
    /// Idle timeout in seconds
    pub idle_timeout: u64,
    /// Enable query logging
    pub log_queries: bool,
    /// Enable migrations
    pub auto_migrate: bool,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,
    /// Connection pool size
    pub pool_size: u32,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Command timeout in seconds
    pub command_timeout: u64,
    /// Reconnection attempts
    pub reconnect_attempts: u32,
    /// Key prefix
    pub key_prefix: String,
    /// Default TTL in seconds
    pub default_ttl: u64,
}

/// Temporal workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporalConfig {
    /// Temporal server URL
    pub server_url: String,
    /// Namespace
    pub namespace: String,
    /// Task queue name
    pub task_queue: String,
    /// Worker configuration
    pub worker: WorkerConfig,
    /// Workflow defaults
    pub workflow_defaults: WorkflowDefaults,
}

/// Worker configuration for Temporal
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerConfig {
    /// Maximum concurrent workflows
    pub max_concurrent_workflows: u32,
    /// Maximum concurrent activities
    pub max_concurrent_activities: u32,
    /// Worker identity
    pub worker_identity: String,
    /// Enable metrics
    pub enable_metrics: bool,
}

/// Workflow default settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefaults {
    /// Default workflow timeout in seconds
    pub timeout: u64,
    /// Default retry policy
    pub retry_policy: DefaultRetryPolicy,
    /// Default execution environment
    pub execution_environment: String,
}

/// Default retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultRetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial retry delay in milliseconds
    pub initial_delay: u64,
    /// Maximum retry delay in milliseconds
    pub max_delay: u64,
    /// Backoff coefficient
    pub backoff_coefficient: f64,
}

/// Proxy configuration for MCP server integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    /// Enable proxy functionality
    pub enabled: bool,
    /// Connection pool size
    pub connection_pool_size: u32,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Keep-alive settings
    pub keep_alive: KeepAliveConfig,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
}

/// Keep-alive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepAliveConfig {
    /// Enable keep-alive
    pub enabled: bool,
    /// Keep-alive timeout in seconds
    pub timeout: u64,
    /// Keep-alive interval in seconds
    pub interval: u64,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Base delay between retries in milliseconds
    pub base_delay: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Enable jitter
    pub enable_jitter: bool,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold
    pub failure_threshold: u32,
    /// Success threshold for recovery
    pub success_threshold: u32,
    /// Timeout duration in seconds
    pub timeout: u64,
    /// Half-open max calls
    pub half_open_max_calls: u32,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: JwtConfig,
    /// API key configuration
    pub api_key: ApiKeyConfig,
    /// OAuth configuration
    pub oauth: Option<OAuthProviderConfig>,
    /// Session configuration
    pub session: SessionConfig,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JwtConfig {
    /// JWT secret key
    pub secret: String,
    /// Token expiration time in seconds
    pub expiration: u64,
    /// Token issuer
    pub issuer: String,
    /// Token audience
    pub audience: String,
    /// Algorithm used for signing
    pub algorithm: String,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyConfig {
    /// API key length
    pub key_length: u32,
    /// Key prefix
    pub key_prefix: String,
    /// Enable key rotation
    pub enable_rotation: bool,
    /// Rotation interval in days
    pub rotation_interval: u32,
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderConfig {
    /// OAuth providers
    pub providers: HashMap<String, OAuthProvider>,
}

/// Individual OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProvider {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Authorization URL
    pub auth_url: String,
    /// Token URL
    pub token_url: String,
    /// User info URL
    pub user_info_url: String,
    /// Scopes
    pub scopes: Vec<String>,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    /// Session timeout in seconds
    pub timeout: u64,
    /// Session storage type
    pub storage: SessionStorage,
    /// Cookie configuration
    pub cookie: CookieConfig,
}

/// Session storage type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStorage {
    /// In-memory storage
    Memory,
    /// Redis storage
    Redis,
    /// Database storage
    Database,
}

/// Cookie configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieConfig {
    /// Cookie name
    pub name: String,
    /// Cookie domain
    pub domain: Option<String>,
    /// Cookie path
    pub path: String,
    /// Secure flag
    pub secure: bool,
    /// HTTP-only flag
    pub http_only: bool,
    /// SameSite setting
    pub same_site: String,
}

/// Cost optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostOptimizationConfig {
    /// Enable cost optimization
    pub enabled: bool,
    /// Optimization strategy
    pub strategy: OptimizationStrategy,
    /// Cost tracking configuration
    pub cost_tracking: CostTrackingConfig,
    /// Budget alerts
    pub budget_alerts: BudgetAlertsConfig,
    /// Provider scoring weights
    pub scoring_weights: ScoringWeights,
}

/// Optimization strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationStrategy {
    /// Minimize cost
    MinimizeCost,
    /// Maximize quality
    MaximizeQuality,
    /// Balance cost and quality
    Balanced,
    /// Custom strategy
    Custom,
}

/// Cost tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostTrackingConfig {
    /// Enable detailed tracking
    pub detailed_tracking: bool,
    /// Tracking granularity
    pub granularity: TrackingGranularity,
    /// Retention period in days
    pub retention_days: u32,
    /// Export configuration
    pub export: ExportConfig,
}

/// Tracking granularity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingGranularity {
    /// Per request
    Request,
    /// Per minute
    Minute,
    /// Per hour
    Hour,
    /// Per day
    Day,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportConfig {
    /// Enable automatic export
    pub enabled: bool,
    /// Export format
    pub format: String,
    /// Export destination
    pub destination: String,
    /// Export schedule
    pub schedule: String,
}

/// Budget alerts configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetAlertsConfig {
    /// Enable budget alerts
    pub enabled: bool,
    /// Alert thresholds (percentages)
    pub thresholds: Vec<f64>,
    /// Notification channels
    pub channels: Vec<String>,
    /// Alert frequency in hours
    pub frequency: u32,
}

/// Provider scoring weights
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoringWeights {
    /// Cost weight (0.0 - 1.0)
    pub cost: f64,
    /// Quality weight (0.0 - 1.0)
    pub quality: f64,
    /// Performance weight (0.0 - 1.0)
    pub performance: f64,
    /// Reliability weight (0.0 - 1.0)
    pub reliability: f64,
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryConfig {
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Metrics configuration
    pub metrics: MetricsConfig,
    /// Tracing configuration
    pub tracing: TracingConfig,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format
    pub format: String,
    /// Output destination
    pub output: String,
    /// File logging configuration
    pub file: Option<FileLoggingConfig>,
    /// Structured logging
    pub structured: bool,
}

/// File logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLoggingConfig {
    /// Log file path
    pub path: String,
    /// Maximum file size in MB
    pub max_size: u64,
    /// Maximum number of files
    pub max_files: u32,
    /// Enable compression
    pub compress: bool,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,
    /// Metrics endpoint
    pub endpoint: String,
    /// Metrics port
    pub port: u16,
    /// Collection interval in seconds
    pub interval: u64,
    /// Prometheus configuration
    pub prometheus: PrometheusConfig,
}

/// Prometheus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusConfig {
    /// Enable Prometheus metrics
    pub enabled: bool,
    /// Metrics namespace
    pub namespace: String,
    /// Additional labels
    pub labels: HashMap<String, String>,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,
    /// Tracing endpoint
    pub endpoint: Option<String>,
    /// Service name
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Sample rate (0.0 - 1.0)
    pub sample_rate: f64,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Global rate limits
    pub global: GlobalRateLimits,
    /// Per-client rate limits
    pub per_client: PerClientRateLimits,
    /// Rate limit storage
    pub storage: RateLimitStorage,
}

/// Global rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalRateLimits {
    /// Requests per second
    pub requests_per_second: u32,
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Requests per hour
    pub requests_per_hour: u32,
    /// Concurrent requests
    pub concurrent_requests: u32,
}

/// Per-client rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerClientRateLimits {
    /// Default limits for new clients
    pub default_limits: HashMap<String, u32>,
    /// Tier-based limits
    pub tier_limits: HashMap<String, HashMap<String, u32>>,
}

/// Rate limit storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStorage {
    /// In-memory storage
    Memory,
    /// Redis storage
    Redis,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    /// Enable schema translation
    pub schema_translation: bool,
    /// Enable cost optimization
    pub cost_optimization: bool,
    /// Enable advanced analytics
    pub advanced_analytics: bool,
    /// Enable A/B testing
    pub ab_testing: bool,
    /// Enable experimental features
    pub experimental_features: bool,
}

/// Environment settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    /// Development environment
    Development,
    /// Testing environment
    Testing,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8084,
                request_timeout: 30,
                keep_alive_timeout: 60,
                max_request_size: 10 * 1024 * 1024, // 10MB
                enable_cors: true,
                cors_origins: vec!["*".to_string()],
                tls: None,
                shutdown_timeout: 30,
            },
            database: DatabaseConfig {
                url: "postgresql://federation:federation@localhost:5432/federation".to_string(),
                max_connections: 10,
                min_connections: 2,
                connect_timeout: 10,
                query_timeout: 30,
                idle_timeout: 300,
                log_queries: false,
                auto_migrate: true,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: 10,
                connect_timeout: 5,
                command_timeout: 5,
                reconnect_attempts: 3,
                key_prefix: "federation:".to_string(),
                default_ttl: 3600,
            },
            temporal: TemporalConfig {
                server_url: "http://localhost:7233".to_string(),
                namespace: "default".to_string(),
                task_queue: "federation-tasks".to_string(),
                worker: WorkerConfig {
                    max_concurrent_workflows: 100,
                    max_concurrent_activities: 100,
                    worker_identity: format!("federation-worker-{}", Uuid::new_v4()),
                    enable_metrics: true,
                },
                workflow_defaults: WorkflowDefaults {
                    timeout: 3600,
                    retry_policy: DefaultRetryPolicy {
                        max_attempts: 3,
                        initial_delay: 1000,
                        max_delay: 60000,
                        backoff_coefficient: 2.0,
                    },
                    execution_environment: "production".to_string(),
                },
            },
            proxy: ProxyConfig {
                enabled: true,
                connection_pool_size: 100,
                request_timeout: 30,
                connection_timeout: 10,
                keep_alive: KeepAliveConfig {
                    enabled: true,
                    timeout: 90,
                    interval: 30,
                },
                retry: RetryConfig {
                    max_attempts: 3,
                    base_delay: 1000,
                    max_delay: 30000,
                    backoff_multiplier: 2.0,
                    enable_jitter: true,
                },
                circuit_breaker: CircuitBreakerConfig {
                    enabled: true,
                    failure_threshold: 5,
                    success_threshold: 3,
                    timeout: 60,
                    half_open_max_calls: 3,
                },
            },
            auth: AuthConfig {
                jwt: JwtConfig {
                    secret: "your-jwt-secret-key".to_string(),
                    expiration: 86400, // 24 hours
                    issuer: "federation-service".to_string(),
                    audience: "federation-clients".to_string(),
                    algorithm: "HS256".to_string(),
                },
                api_key: ApiKeyConfig {
                    key_length: 32,
                    key_prefix: "fed_".to_string(),
                    enable_rotation: false,
                    rotation_interval: 90,
                },
                oauth: None,
                session: SessionConfig {
                    timeout: 3600,
                    storage: SessionStorage::Redis,
                    cookie: CookieConfig {
                        name: "federation_session".to_string(),
                        domain: None,
                        path: "/".to_string(),
                        secure: false,
                        http_only: true,
                        same_site: "lax".to_string(),
                    },
                },
            },
            cost_optimization: CostOptimizationConfig {
                enabled: true,
                strategy: OptimizationStrategy::Balanced,
                cost_tracking: CostTrackingConfig {
                    detailed_tracking: true,
                    granularity: TrackingGranularity::Request,
                    retention_days: 90,
                    export: ExportConfig {
                        enabled: false,
                        format: "json".to_string(),
                        destination: "s3://cost-reports/".to_string(),
                        schedule: "0 0 * * *".to_string(), // Daily at midnight
                    },
                },
                budget_alerts: BudgetAlertsConfig {
                    enabled: true,
                    thresholds: vec![0.5, 0.8, 0.95],
                    channels: vec!["email".to_string(), "slack".to_string()],
                    frequency: 1,
                },
                scoring_weights: ScoringWeights {
                    cost: 0.3,
                    quality: 0.3,
                    performance: 0.2,
                    reliability: 0.2,
                },
            },
            telemetry: TelemetryConfig {
                logging: LoggingConfig {
                    level: "info".to_string(),
                    format: "json".to_string(),
                    output: "stdout".to_string(),
                    file: None,
                    structured: true,
                },
                metrics: MetricsConfig {
                    enabled: true,
                    endpoint: "/metrics".to_string(),
                    port: 9090,
                    interval: 15,
                    prometheus: PrometheusConfig {
                        enabled: true,
                        namespace: "federation".to_string(),
                        labels: HashMap::new(),
                    },
                },
                tracing: TracingConfig {
                    enabled: true,
                    endpoint: None,
                    service_name: "federation-service".to_string(),
                    service_version: env!("CARGO_PKG_VERSION").to_string(),
                    sample_rate: 0.1,
                },
            },
            rate_limiting: RateLimitingConfig {
                enabled: true,
                global: GlobalRateLimits {
                    requests_per_second: 1000,
                    requests_per_minute: 60000,
                    requests_per_hour: 3600000,
                    concurrent_requests: 500,
                },
                per_client: PerClientRateLimits {
                    default_limits: {
                        let mut limits = HashMap::new();
                        limits.insert("requests_per_second".to_string(), 10);
                        limits.insert("requests_per_minute".to_string(), 600);
                        limits.insert("requests_per_hour".to_string(), 36000);
                        limits
                    },
                    tier_limits: HashMap::new(),
                },
                storage: RateLimitStorage::Redis,
            },
            features: FeatureFlags {
                schema_translation: true,
                cost_optimization: true,
                advanced_analytics: false,
                ab_testing: false,
                experimental_features: false,
            },
            environment: Environment::Development,
        }
    }
}

impl Config {
    /// Load configuration from a YAML file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read configuration file")?;

        let config: Config =
            serde_yaml::from_str(&content).context("Failed to parse configuration file")?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Config::default();

        // Override with environment variables
        if let Ok(host) = std::env::var("FEDERATION_HOST") {
            config.server.host = host;
        }

        if let Ok(port) = std::env::var("FEDERATION_PORT") {
            config.server.port = port.parse().context("Invalid port number")?;
        }

        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            config.database.url = db_url;
        }

        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis.url = redis_url;
        }

        if let Ok(temporal_url) = std::env::var("TEMPORAL_SERVER_URL") {
            config.temporal.server_url = temporal_url;
        }

        if let Ok(jwt_secret) = std::env::var("JWT_SECRET") {
            config.auth.jwt.secret = jwt_secret;
        }

        if let Ok(log_level) = std::env::var("LOG_LEVEL") {
            config.telemetry.logging.level = log_level;
        }

        config.validate()?;
        Ok(config)
    }

    /// Merge configuration with environment variables and CLI arguments
    pub fn merge_with_overrides(mut self, overrides: ConfigOverrides) -> Result<Self> {
        if let Some(host) = overrides.host {
            self.server.host = host;
        }

        if let Some(port) = overrides.port {
            self.server.port = port;
        }

        if let Some(log_level) = overrides.log_level {
            self.telemetry.logging.level = log_level;
        }

        if let Some(db_url) = overrides.database_url {
            self.database.url = db_url;
        }

        if let Some(redis_url) = overrides.redis_url {
            self.redis.url = redis_url;
        }

        self.validate()?;
        Ok(self)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port must be greater than 0"));
        }

        if self.server.request_timeout == 0 {
            return Err(anyhow::anyhow!("Request timeout must be greater than 0"));
        }

        // Validate database configuration
        if self.database.url.is_empty() {
            return Err(anyhow::anyhow!("Database URL is required"));
        }

        if self.database.max_connections == 0 {
            return Err(anyhow::anyhow!(
                "Max database connections must be greater than 0"
            ));
        }

        // Validate Redis configuration
        if self.redis.url.is_empty() {
            return Err(anyhow::anyhow!("Redis URL is required"));
        }

        // Validate JWT configuration
        if self.auth.jwt.secret.len() < 16 {
            return Err(anyhow::anyhow!(
                "JWT secret must be at least 16 characters long"
            ));
        }

        // Validate scoring weights sum to approximately 1.0
        let weights_sum = self.cost_optimization.scoring_weights.cost
            + self.cost_optimization.scoring_weights.quality
            + self.cost_optimization.scoring_weights.performance
            + self.cost_optimization.scoring_weights.reliability;

        if (weights_sum - 1.0).abs() > 0.1 {
            return Err(anyhow::anyhow!(
                "Scoring weights must sum to approximately 1.0, got {}",
                weights_sum
            ));
        }

        Ok(())
    }

    /// Save configuration to a YAML file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(self).context("Failed to serialize configuration")?;

        tokio::fs::write(path, content)
            .await
            .context("Failed to write configuration file")?;

        Ok(())
    }

    /// Get environment-specific configuration
    pub fn for_environment(&self, env: Environment) -> Self {
        let mut config = self.clone();
        config.environment = env.clone();

        match env {
            Environment::Development => {
                config.telemetry.logging.level = "debug".to_string();
                config.database.log_queries = true;
                config.features.experimental_features = true;
            }
            Environment::Testing => {
                config.telemetry.logging.level = "warn".to_string();
                config.database.log_queries = false;
                config.features.experimental_features = false;
            }
            Environment::Staging => {
                config.telemetry.logging.level = "info".to_string();
                config.database.log_queries = false;
                config.features.experimental_features = false;
            }
            Environment::Production => {
                config.telemetry.logging.level = "warn".to_string();
                config.database.log_queries = false;
                config.features.experimental_features = false;
                config.auth.session.cookie.secure = true;
            }
        }

        config
    }
}

/// Configuration overrides from CLI arguments or environment
#[derive(Debug, Default)]
pub struct ConfigOverrides {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub log_level: Option<String>,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config_validation() {
        let mut config = Config::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_file_roundtrip() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();

        config.save_to_file(temp_file.path()).await.unwrap();
        let loaded_config = Config::from_file(temp_file.path()).await.unwrap();

        assert_eq!(config.server.port, loaded_config.server.port);
        assert_eq!(config.database.url, loaded_config.database.url);
    }

    #[test]
    fn test_environment_specific_config() {
        let base_config = Config::default();
        let dev_config = base_config.for_environment(Environment::Development);
        let prod_config = base_config.for_environment(Environment::Production);

        assert_eq!(dev_config.telemetry.logging.level, "debug");
        assert_eq!(prod_config.telemetry.logging.level, "warn");
        assert!(dev_config.features.experimental_features);
        assert!(!prod_config.features.experimental_features);
    }

    #[test]
    fn test_config_overrides() {
        let base_config = Config::default();
        let overrides = ConfigOverrides {
            host: Some("127.0.0.1".to_string()),
            port: Some(9000),
            log_level: Some("debug".to_string()),
            database_url: Some("postgresql://test".to_string()),
            redis_url: Some("redis://test".to_string()),
        };

        let config = base_config.merge_with_overrides(overrides).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.telemetry.logging.level, "debug");
        assert_eq!(config.database.url, "postgresql://test");
        assert_eq!(config.redis.url, "redis://test");
    }

    #[test]
    fn test_scoring_weights_validation() {
        let mut config = Config::default();
        config.cost_optimization.scoring_weights = ScoringWeights {
            cost: 0.5,
            quality: 0.3,
            performance: 0.1,
            reliability: 0.05, // Sum = 0.95, should fail
        };

        assert!(config.validate().is_err());

        config.cost_optimization.scoring_weights.reliability = 0.1; // Sum = 1.0, should pass
        assert!(config.validate().is_ok());
    }
}
