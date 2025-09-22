//! Configuration module for the Data Processing Service
//!
//! This module provides comprehensive configuration management for all components
//! of the data processing service including Kafka, ClickHouse, stream processing,
//! batch processing, and monitoring settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration for the Data Processing Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Kafka configuration
    pub kafka: KafkaConfig,
    /// ClickHouse configuration
    pub clickhouse: ClickHouseConfig,
    /// Stream processing configuration
    pub stream: StreamConfig,
    /// Batch processing configuration
    pub batch: BatchConfig,
    /// Metrics and monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Health check configuration
    pub health: HealthConfig,
    /// Performance tuning configuration
    pub performance: PerformanceConfig,
    /// Security configuration
    pub security: SecurityConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host to bind to
    pub host: String,
    /// Server port to bind to
    pub port: u16,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Keep alive timeout in seconds
    pub keep_alive_secs: u64,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Allowed origins for CORS
    pub cors_origins: Vec<String>,
}

/// Kafka configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka bootstrap servers
    pub bootstrap_servers: String,
    /// Consumer group ID
    pub consumer_group_id: String,
    /// Producer client ID
    pub producer_client_id: String,
    /// Enable auto commit
    pub enable_auto_commit: bool,
    /// Auto commit interval in milliseconds
    pub auto_commit_interval_ms: u64,
    /// Session timeout in milliseconds
    pub session_timeout_ms: u64,
    /// Enable auto offset reset
    pub auto_offset_reset: String,
    /// Maximum poll records
    pub max_poll_records: usize,
    /// Fetch minimum bytes
    pub fetch_min_bytes: usize,
    /// Fetch maximum wait time in milliseconds
    pub fetch_max_wait_ms: u64,
    /// Producer batch size
    pub batch_size: usize,
    /// Producer linger time in milliseconds
    pub linger_ms: u64,
    /// Producer buffer memory
    pub buffer_memory: usize,
    /// Compression type
    pub compression_type: String,
    /// SASL configuration
    pub sasl: Option<SaslConfig>,
    /// SSL configuration
    pub ssl: Option<SslConfig>,
}

/// SASL configuration for Kafka
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaslConfig {
    /// SASL mechanism
    pub mechanism: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
}

/// SSL configuration for Kafka
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// CA certificate path
    pub ca_cert_path: Option<PathBuf>,
    /// Client certificate path
    pub client_cert_path: Option<PathBuf>,
    /// Client key path
    pub client_key_path: Option<PathBuf>,
    /// Verify hostname
    pub verify_hostname: bool,
}

/// ClickHouse configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseConfig {
    /// ClickHouse server URL
    pub url: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Database name
    pub database: String,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Enable compression
    pub compression: bool,
    /// Enable secure connection
    pub secure: bool,
    /// Connection pool size
    pub pool_size: usize,
    /// Maximum idle connections
    pub max_idle: usize,
    /// Connection max lifetime in seconds
    pub max_lifetime_secs: u64,
    /// Batch insert size
    pub batch_insert_size: usize,
    /// Insert timeout in seconds
    pub insert_timeout_secs: u64,
    /// Query timeout in seconds
    pub query_timeout_secs: u64,
}

/// Stream processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Number of stream processing workers
    pub worker_threads: usize,
    /// Buffer size for each worker
    pub buffer_size: usize,
    /// Batch size for processing
    pub batch_size: usize,
    /// Processing timeout per batch in seconds
    pub batch_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: usize,
    /// Retry backoff base in milliseconds
    pub retry_backoff_base_ms: u64,
    /// Maximum retry backoff in milliseconds
    pub retry_backoff_max_ms: u64,
    /// Enable exactly-once processing
    pub exactly_once: bool,
    /// Checkpoint interval in milliseconds
    pub checkpoint_interval_ms: u64,
    /// Window size for streaming aggregations in seconds
    pub window_size_secs: u64,
    /// Window slide interval in seconds
    pub window_slide_secs: u64,
    /// Topics to consume from
    pub input_topics: Vec<String>,
    /// Topics to produce to
    pub output_topics: Vec<String>,
    /// Dead letter topic for failed messages
    pub dead_letter_topic: String,
}

/// Batch processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Number of batch processing workers
    pub worker_threads: usize,
    /// Maximum concurrent batch jobs
    pub max_concurrent_jobs: usize,
    /// Job queue size
    pub job_queue_size: usize,
    /// Job timeout in seconds
    pub job_timeout_secs: u64,
    /// Chunk size for large datasets
    pub chunk_size: usize,
    /// Temporary directory for batch processing
    pub temp_dir: PathBuf,
    /// Maximum memory usage per job in GB
    pub max_memory_gb: usize,
    /// Enable distributed processing
    pub distributed: bool,
    /// Scheduler cron expression
    pub scheduler_cron: Option<String>,
    /// Data retention days
    pub retention_days: u32,
    /// Cleanup interval in hours
    pub cleanup_interval_hours: u64,
}

/// Monitoring and metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable Prometheus metrics
    pub prometheus_enabled: bool,
    /// Prometheus metrics port
    pub prometheus_port: u16,
    /// Metrics collection interval in seconds
    pub collection_interval_secs: u64,
    /// Enable tracing
    pub tracing_enabled: bool,
    /// Jaeger endpoint for tracing
    pub jaeger_endpoint: Option<String>,
    /// Log level
    pub log_level: String,
    /// Log format (json or text)
    pub log_format: String,
    /// Custom metrics configuration
    pub custom_metrics: HashMap<String, MetricConfig>,
}

/// Configuration for individual metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricConfig {
    /// Metric name
    pub name: String,
    /// Metric type (counter, gauge, histogram)
    pub metric_type: String,
    /// Metric description
    pub description: String,
    /// Labels for the metric
    pub labels: Vec<String>,
    /// Buckets for histogram metrics
    pub buckets: Option<Vec<f64>>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Health check interval in seconds
    pub check_interval_secs: u64,
    /// Health check timeout in seconds
    pub check_timeout_secs: u64,
    /// Number of failed checks before marking unhealthy
    pub failure_threshold: usize,
    /// Number of successful checks before marking healthy
    pub success_threshold: usize,
    /// Enable detailed health reporting
    pub detailed_reporting: bool,
    /// Components to monitor
    pub components: Vec<String>,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable auto-scaling
    pub auto_scaling: bool,
    /// Minimum number of workers
    pub min_workers: usize,
    /// Maximum number of workers
    pub max_workers: usize,
    /// CPU usage threshold for scaling up (0-100)
    pub scale_up_cpu_threshold: f64,
    /// CPU usage threshold for scaling down (0-100)
    pub scale_down_cpu_threshold: f64,
    /// Memory usage threshold for scaling up (0-100)
    pub scale_up_memory_threshold: f64,
    /// Memory usage threshold for scaling down (0-100)
    pub scale_down_memory_threshold: f64,
    /// Scaling cooldown period in seconds
    pub scaling_cooldown_secs: u64,
    /// Enable data compression
    pub compression_enabled: bool,
    /// Compression algorithm (lz4, zstd)
    pub compression_algorithm: String,
    /// Cache size in MB
    pub cache_size_mb: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub auth_enabled: bool,
    /// JWT secret key
    pub jwt_secret: Option<String>,
    /// JWT token expiry in hours
    pub jwt_expiry_hours: u64,
    /// API rate limiting
    pub rate_limiting: RateLimitConfig,
    /// Enable data encryption
    pub encryption_enabled: bool,
    /// Encryption key
    pub encryption_key: Option<String>,
    /// Enable audit logging
    pub audit_logging: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per second
    pub requests_per_second: u32,
    /// Burst capacity
    pub burst_capacity: u32,
    /// Rate limit window in seconds
    pub window_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            kafka: KafkaConfig::default(),
            clickhouse: ClickHouseConfig::default(),
            stream: StreamConfig::default(),
            batch: BatchConfig::default(),
            monitoring: MonitoringConfig::default(),
            health: HealthConfig::default(),
            performance: PerformanceConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 10000,
            request_timeout_secs: 30,
            keep_alive_secs: 60,
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            consumer_group_id: "data-processing-service".to_string(),
            producer_client_id: "data-processing-producer".to_string(),
            enable_auto_commit: false,
            auto_commit_interval_ms: 5000,
            session_timeout_ms: 30000,
            auto_offset_reset: "earliest".to_string(),
            max_poll_records: 500,
            fetch_min_bytes: 1024,
            fetch_max_wait_ms: 500,
            batch_size: 16384,
            linger_ms: 0,
            buffer_memory: 33554432,
            compression_type: "lz4".to_string(),
            sasl: None,
            ssl: None,
        }
    }
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            username: "default".to_string(),
            password: "".to_string(),
            database: "aicore".to_string(),
            timeout_seconds: 60,
            compression: true,
            secure: false,
            pool_size: 10,
            max_idle: 5,
            max_lifetime_secs: 3600,
            batch_insert_size: 100000,
            insert_timeout_secs: 300,
            query_timeout_secs: 60,
        }
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            buffer_size: 10000,
            batch_size: 1000,
            batch_timeout_secs: 5,
            max_retries: 3,
            retry_backoff_base_ms: 100,
            retry_backoff_max_ms: 30000,
            exactly_once: false,
            checkpoint_interval_ms: 30000,
            window_size_secs: 60,
            window_slide_secs: 10,
            input_topics: vec!["events".to_string()],
            output_topics: vec!["processed-events".to_string()],
            dead_letter_topic: "failed-events".to_string(),
        }
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            max_concurrent_jobs: 10,
            job_queue_size: 1000,
            job_timeout_secs: 3600,
            chunk_size: 10000,
            temp_dir: std::env::temp_dir().join("data-processing"),
            max_memory_gb: 8,
            distributed: false,
            scheduler_cron: None,
            retention_days: 30,
            cleanup_interval_hours: 24,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            prometheus_enabled: true,
            prometheus_port: 9090,
            collection_interval_secs: 10,
            tracing_enabled: true,
            jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            custom_metrics: HashMap::new(),
        }
    }
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            check_timeout_secs: 5,
            failure_threshold: 3,
            success_threshold: 2,
            detailed_reporting: true,
            components: vec![
                "kafka".to_string(),
                "clickhouse".to_string(),
                "stream_processor".to_string(),
                "batch_processor".to_string(),
            ],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            auto_scaling: true,
            min_workers: 2,
            max_workers: num_cpus::get() * 2,
            scale_up_cpu_threshold: 80.0,
            scale_down_cpu_threshold: 20.0,
            scale_up_memory_threshold: 85.0,
            scale_down_memory_threshold: 30.0,
            scaling_cooldown_secs: 300,
            compression_enabled: true,
            compression_algorithm: "lz4".to_string(),
            cache_size_mb: 1024,
            cache_ttl_secs: 3600,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            jwt_secret: None,
            jwt_expiry_hours: 24,
            rate_limiting: RateLimitConfig::default(),
            encryption_enabled: false,
            encryption_key: None,
            audit_logging: true,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_capacity: 200,
            window_secs: 60,
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::builder();

        // Load from file if specified
        if let Ok(config_file) = std::env::var("DATA_PROCESSING_CONFIG_FILE") {
            cfg = cfg.add_source(config::File::with_name(&config_file));
        }

        // Load from environment variables
        cfg = cfg.add_source(
            config::Environment::with_prefix("DATA_PROCESSING")
                .separator("__")
                .list_separator(","),
        );

        let config: Self = cfg.build()?.try_deserialize()?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate server config
        if self.server.port == 0 {
            return Err("Server port must be greater than 0".to_string());
        }

        // Validate Kafka config
        if self.kafka.bootstrap_servers.is_empty() {
            return Err("Kafka bootstrap servers cannot be empty".to_string());
        }

        // Validate ClickHouse config
        if self.clickhouse.url.is_empty() {
            return Err("ClickHouse URL cannot be empty".to_string());
        }

        // Validate stream config
        if self.stream.worker_threads == 0 {
            return Err("Stream worker threads must be greater than 0".to_string());
        }

        // Validate batch config
        if self.batch.worker_threads == 0 {
            return Err("Batch worker threads must be greater than 0".to_string());
        }

        // Validate performance config
        if self.performance.min_workers > self.performance.max_workers {
            return Err("Min workers cannot be greater than max workers".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Test invalid port
        config.server.port = 0;
        assert!(config.validate().is_err());

        // Reset and test empty Kafka servers
        config = Config::default();
        config.kafka.bootstrap_servers = "".to_string();
        assert!(config.validate().is_err());

        // Reset and test worker thread validation
        config = Config::default();
        config.performance.min_workers = 10;
        config.performance.max_workers = 5;
        assert!(config.validate().is_err());
    }
}
