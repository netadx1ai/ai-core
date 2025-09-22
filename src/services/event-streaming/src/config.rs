//! # Configuration Module
//!
//! This module defines the configuration structure for the event streaming service.
//! It handles loading configuration from environment variables, files, and defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::types::{
    BackoffStrategy, CompressionType, DeadLetterConfig, EventFilter, EventTransformation,
    ReplayConfig, RetentionConfig, RetryConfig, StreamConfig,
};

/// Main configuration structure for the event streaming service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,

    /// Kafka configuration
    pub kafka: KafkaConfig,

    /// Redis Streams configuration
    pub redis: RedisConfig,

    /// Processing pipeline configuration
    pub processing: ProcessingConfig,

    /// Storage configuration for event persistence
    pub storage: StorageConfig,

    /// Monitoring and metrics configuration
    pub monitoring: MonitoringConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Environment-specific settings
    pub environment: EnvironmentConfig,
}

impl Config {
    /// Load configuration from environment variables and files
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut settings = config::Config::builder()
            // Start with default configuration
            .add_source(config::File::with_name("config/event-streaming").required(false))
            .add_source(config::File::with_name("config/event-streaming.local").required(false))
            // Override with environment variables
            .add_source(
                config::Environment::with_prefix("EVENT_STREAMING")
                    .prefix_separator("_")
                    .separator("__"),
            )
            .build()
            .map_err(|e| ConfigError::LoadError(e.to_string()))?;

        settings
            .try_deserialize()
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // TODO: Implement custom validation logic
        Ok(())
    }

    /// Get database URL for event storage
    pub fn database_url(&self) -> &str {
        &self.storage.database_url
    }

    /// Get Kafka bootstrap servers
    pub fn kafka_brokers(&self) -> &[String] {
        &self.kafka.bootstrap_servers
    }

    /// Get Redis connection URL
    pub fn redis_url(&self) -> &str {
        &self.redis.url
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            kafka: KafkaConfig::default(),
            redis: RedisConfig::default(),

            processing: ProcessingConfig::default(),
            storage: StorageConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            environment: EnvironmentConfig::default(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Maximum number of concurrent connections
    pub max_connections: u32,

    /// Request timeout in seconds
    pub request_timeout_seconds: u64,

    /// Keep-alive timeout in seconds
    pub keep_alive_timeout_seconds: u64,

    /// Enable TLS
    pub tls_enabled: bool,

    /// Graceful shutdown timeout in seconds
    pub graceful_shutdown_timeout_seconds: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            max_connections: 10000,
            request_timeout_seconds: 30,
            keep_alive_timeout_seconds: 60,
            tls_enabled: false,
            graceful_shutdown_timeout_seconds: 30,
        }
    }
}

/// Kafka configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka bootstrap servers
    pub bootstrap_servers: Vec<String>,

    /// Consumer group ID
    pub consumer_group_id: String,

    /// Producer client ID
    pub producer_client_id: String,

    /// Consumer configuration
    pub consumer: KafkaConsumerConfig,

    /// Producer configuration
    pub producer: KafkaProducerConfig,

    /// Topic configurations
    pub topics: HashMap<String, StreamConfig>,

    /// Security settings
    pub security: Option<KafkaSecurityConfig>,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: vec!["localhost:9092".to_string()],
            consumer_group_id: "event-streaming-service".to_string(),
            producer_client_id: "event-streaming-producer".to_string(),
            consumer: KafkaConsumerConfig::default(),
            producer: KafkaProducerConfig::default(),
            topics: HashMap::new(),
            security: None,
        }
    }
}

/// Kafka consumer configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaConsumerConfig {
    /// Session timeout in milliseconds
    pub session_timeout_ms: u32,

    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u32,

    /// Auto offset reset strategy
    pub auto_offset_reset: AutoOffsetReset,

    /// Enable auto commit
    pub enable_auto_commit: bool,

    /// Auto commit interval in milliseconds
    pub auto_commit_interval_ms: u32,

    /// Maximum poll records
    pub max_poll_records: u32,

    /// Fetch minimum bytes
    pub fetch_min_bytes: u32,

    /// Fetch maximum wait time in milliseconds
    pub fetch_max_wait_ms: u32,
}

impl Default for KafkaConsumerConfig {
    fn default() -> Self {
        Self {
            session_timeout_ms: 30000,
            heartbeat_interval_ms: 3000,
            auto_offset_reset: AutoOffsetReset::Latest,
            enable_auto_commit: false,
            auto_commit_interval_ms: 5000,
            max_poll_records: 500,
            fetch_min_bytes: 1024,
            fetch_max_wait_ms: 5000,
        }
    }
}

/// Kafka producer configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaProducerConfig {
    /// Acknowledgments required
    pub acks: KafkaAcks,

    /// Retries count
    pub retries: u32,

    /// Batch size in bytes
    pub batch_size: u32,

    /// Linger time in milliseconds
    pub linger_ms: u32,

    /// Buffer memory in bytes
    pub buffer_memory: u64,

    /// Compression type
    pub compression_type: CompressionType,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u32,

    /// Delivery timeout in milliseconds
    pub delivery_timeout_ms: u32,
}

impl Default for KafkaProducerConfig {
    fn default() -> Self {
        Self {
            acks: KafkaAcks::All,
            retries: 3,
            batch_size: 16384,
            linger_ms: 5,
            buffer_memory: 33554432,
            compression_type: CompressionType::Lz4,
            request_timeout_ms: 30000,
            delivery_timeout_ms: 120000,
        }
    }
}

/// Kafka acknowledgment settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KafkaAcks {
    /// No acknowledgments
    None,
    /// Leader acknowledgment only
    Leader,
    /// All in-sync replicas acknowledgment
    All,
}

/// Auto offset reset strategies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoOffsetReset {
    /// Reset to earliest offset
    Earliest,
    /// Reset to latest offset
    Latest,
    /// Throw exception
    None,
}

/// Kafka security configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaSecurityConfig {
    /// Security protocol
    pub protocol: KafkaSecurityProtocol,

    /// SASL mechanism
    pub sasl_mechanism: Option<String>,

    /// SASL username
    pub sasl_username: Option<String>,

    /// SASL password
    pub sasl_password: Option<String>,

    /// SSL configuration
    pub ssl: Option<KafkaSslConfig>,
}

/// Kafka security protocols
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KafkaSecurityProtocol {
    Plaintext,
    Ssl,
    SaslPlaintext,
    SaslSsl,
}

/// Kafka SSL configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaSslConfig {
    /// CA certificate file path
    pub ca_cert_path: Option<String>,

    /// Client certificate file path
    pub cert_path: Option<String>,

    /// Client private key file path
    pub key_path: Option<String>,

    /// Key password
    pub key_password: Option<String>,

    /// Verify SSL certificates
    pub verify_certificates: bool,
}

/// Redis Streams configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Connection pool configuration
    pub pool: RedisPoolConfig,

    /// Streams configuration
    pub streams: HashMap<String, RedisStreamConfig>,

    /// Consumer group configurations
    pub consumer_groups: HashMap<String, RedisConsumerGroupConfig>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool: RedisPoolConfig::default(),
            streams: HashMap::new(),
            consumer_groups: HashMap::new(),
        }
    }
}

/// Redis connection pool configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedisPoolConfig {
    /// Maximum pool size
    pub max_size: u32,

    /// Minimum idle connections
    pub min_idle: u32,

    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,

    /// Idle timeout in seconds
    pub idle_timeout_seconds: u64,
}

impl Default for RedisPoolConfig {
    fn default() -> Self {
        Self {
            max_size: 50,
            min_idle: 5,
            connection_timeout_seconds: 5,
            idle_timeout_seconds: 600,
        }
    }
}

/// Redis stream configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedisStreamConfig {
    /// Stream name
    pub name: String,

    /// Maximum stream length
    pub max_length: Option<u64>,

    /// Approximate maximum length
    pub max_length_approx: Option<u64>,

    /// Retention configuration
    pub retention: Option<RetentionConfig>,
}

/// Redis consumer group configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedisConsumerGroupConfig {
    /// Consumer group name
    pub group_name: String,

    /// Consumer name
    pub consumer_name: String,

    /// Block time in milliseconds
    pub block_time_ms: u64,

    /// Count of messages to read
    pub count: u32,
}

/// RabbitMQ configuration (optional messaging backend)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RabbitMQConfig {
    /// RabbitMQ connection URL
    pub url: String,

    /// Exchange configurations
    pub exchanges: HashMap<String, RabbitMQExchangeConfig>,

    /// Queue configurations
    pub queues: HashMap<String, RabbitMQQueueConfig>,
}

/// RabbitMQ exchange configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RabbitMQExchangeConfig {
    /// Exchange name
    pub name: String,

    /// Exchange type
    pub exchange_type: String,

    /// Durable exchange
    pub durable: bool,

    /// Auto-delete exchange
    pub auto_delete: bool,
}

/// RabbitMQ queue configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RabbitMQQueueConfig {
    /// Queue name
    pub name: String,

    /// Durable queue
    pub durable: bool,

    /// Exclusive queue
    pub exclusive: bool,

    /// Auto-delete queue
    pub auto_delete: bool,
}

/// Processing pipeline configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Number of worker threads
    pub worker_threads: usize,

    /// Processing batch size
    pub batch_size: u32,

    /// Processing timeout in seconds
    pub timeout_seconds: u64,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Dead letter queue configuration
    pub dead_letter: DeadLetterConfig,

    /// Event filters
    pub filters: Vec<EventFilter>,

    /// Event transformations
    pub transformations: Vec<EventTransformation>,

    /// Enable event replay
    pub enable_replay: bool,

    /// Replay configuration
    pub replay: Option<ReplayConfig>,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            batch_size: 100,
            timeout_seconds: 300,
            retry: RetryConfig::default(),
            dead_letter: DeadLetterConfig {
                queue_name: "event-streaming-dlq".to_string(),
                retention_seconds: 86400 * 7, // 7 days
                auto_replay: false,
                replay_config: None,
            },
            filters: Vec::new(),
            transformations: Vec::new(),
            enable_replay: true,
            replay: Some(ReplayConfig {
                batch_size: 50,
                batch_delay_ms: 1000,
                max_concurrent: 10,
                preserve_timestamps: false,
            }),
        }
    }
}

/// Storage configuration for event persistence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database URL for event storage
    pub database_url: String,

    /// Maximum connection pool size
    pub max_connections: u32,

    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,

    /// Query timeout in seconds
    pub query_timeout_seconds: u64,

    /// Enable event archiving
    pub enable_archiving: bool,

    /// Archive after days
    pub archive_after_days: u32,

    /// Archive storage configuration
    pub archive_storage: Option<ArchiveStorageConfig>,

    /// Compression for stored events
    pub compression: CompressionType,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost:5432/event_streaming".to_string(),
            max_connections: 50,
            connection_timeout_seconds: 5,
            query_timeout_seconds: 30,
            enable_archiving: false,
            archive_after_days: 90,
            archive_storage: None,
            compression: CompressionType::Lz4,
        }
    }
}

/// Archive storage configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveStorageConfig {
    /// Storage type (s3, gcs, azure)
    pub storage_type: String,

    /// Storage bucket/container name
    pub bucket: String,

    /// Storage path prefix
    pub prefix: String,

    /// Storage credentials
    pub credentials: HashMap<String, String>,
}

/// Monitoring and metrics configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Metrics port
    pub port: u16,

    /// Metrics path
    pub metrics_path: String,

    /// Enable distributed tracing
    pub enable_tracing: bool,

    /// Tracing configuration
    pub tracing: TracingConfig,

    /// Health check configuration
    pub health_check: HealthCheckConfig,

    /// Log level
    pub log_level: String,

    /// Log format (json or text)
    pub log_format: String,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            port: 9090,
            metrics_path: "/metrics".to_string(),
            enable_tracing: true,
            tracing: TracingConfig::default(),
            health_check: HealthCheckConfig::default(),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
        }
    }
}

/// Distributed tracing configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Tracing backend (jaeger, zipkin)
    pub backend: String,

    /// Tracing endpoint
    pub endpoint: String,

    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            backend: "jaeger".to_string(),
            endpoint: "http://localhost:14268/api/traces".to_string(),
            sample_rate: 0.1,
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval in seconds
    pub interval_seconds: u64,

    /// Health check timeout in seconds
    pub timeout_seconds: u64,

    /// Failure threshold before marking unhealthy
    pub failure_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 30,
            timeout_seconds: 5,
            failure_threshold: 3,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,

    /// JWT configuration
    pub jwt: Option<JwtConfig>,

    /// API key configuration
    pub api_key: Option<ApiKeyConfig>,

    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,

    /// CORS configuration
    pub cors: CorsConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: false,
            jwt: None,
            api_key: None,
            rate_limiting: RateLimitingConfig::default(),
            cors: CorsConfig::default(),
        }
    }
}

/// JWT configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret or public key
    pub secret: String,

    /// JWT algorithm
    pub algorithm: String,

    /// Token expiration in seconds
    pub expiration_seconds: u64,

    /// Token issuer
    pub issuer: String,
}

/// API key configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Valid API keys
    pub keys: Vec<String>,

    /// API key header name
    pub header_name: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Requests per minute
    pub requests_per_minute: u32,

    /// Burst size
    pub burst_size: u32,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 1000,
            burst_size: 100,
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["*".to_string()],
            allow_credentials: false,
        }
    }
}

/// Environment-specific configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment name (dev, staging, prod)
    pub name: String,

    /// Service version
    pub version: String,

    /// Service instance ID
    pub instance_id: String,

    /// Debug mode
    pub debug: bool,

    /// Feature flags
    pub features: HashMap<String, bool>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            name: "development".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            instance_id: uuid::Uuid::new_v4().to_string(),
            debug: cfg!(debug_assertions),
            features: HashMap::new(),
        }
    }
}

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadError(String),

    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    #[error("Configuration validation failed: {0}")]
    ValidationError(String),

    #[error("Missing required configuration: {0}")]
    MissingRequired(String),

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert!(!config.kafka.bootstrap_servers.is_empty());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_server_config_validation() {
        let config = ServerConfig {
            host: "".to_string(), // Invalid: empty host
            port: 8080,
            max_connections: 10000,
            request_timeout_seconds: 30,
            keep_alive_timeout_seconds: 60,
            graceful_shutdown: true,
            graceful_shutdown_timeout_seconds: 30,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kafka_config_defaults() {
        let config = KafkaConfig::default();
        assert_eq!(config.bootstrap_servers, vec!["localhost:9092".to_string()]);
        assert_eq!(config.consumer_group_id, "event-streaming-service");
        assert_eq!(config.producer.acks, KafkaAcks::All);
    }

    #[test]
    fn test_processing_config_defaults() {
        let config = ProcessingConfig::default();
        assert_eq!(config.worker_threads, num_cpus::get());
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.retry.max_attempts, 3);
    }

    #[test]
    fn test_environment_config_defaults() {
        let config = EnvironmentConfig::default();
        assert_eq!(config.name, "development");
        assert_eq!(config.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(config.debug, cfg!(debug_assertions));
    }
}
