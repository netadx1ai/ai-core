//! # Configuration Module
//!
//! This module provides configuration management for the database-security
//! integration crate. It handles loading, validation, and management of
//! configuration settings from various sources.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::{
    access_control::AccessControlConfig, audit::AuditConfig,
    encryption_integration::DataEncryptionConfig, error::SecureDatabaseError,
};

/// Main configuration for secure database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureDatabaseConfig {
    /// Database connection settings
    pub database: DatabaseConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Access control configuration
    pub access_control: AccessControlConfig,
    /// Audit logging configuration
    pub audit: AuditConfig,
    /// Encryption configuration
    pub encryption: DataEncryptionConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
    /// Monitoring and metrics settings
    pub monitoring: MonitoringConfig,
    /// Feature flags
    pub features: FeatureFlags,
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection settings
    pub postgres: PostgresConfig,
    /// ClickHouse connection settings
    pub clickhouse: ClickHouseConfig,
    /// MongoDB connection settings
    pub mongodb: MongoConfig,
    /// Redis connection settings
    pub redis: RedisConfig,
    /// Connection pool settings
    pub pool: ConnectionPoolConfig,
    /// Health check settings
    pub health_check: HealthCheckConfig,
}

/// PostgreSQL specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database host
    pub host: String,
    /// Database port
    pub port: u16,
    /// Database name
    pub database: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// SSL mode
    pub ssl_mode: String,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Query timeout in seconds
    pub query_timeout: u64,
    /// Enable prepared statements
    pub enable_prepared_statements: bool,
}

/// ClickHouse specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseConfig {
    /// Database host
    pub host: String,
    /// Database port
    pub port: u16,
    /// Database name
    pub database: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Use compression
    pub compression: bool,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Query timeout in seconds
    pub query_timeout: u64,
    /// Maximum block size for bulk operations
    pub max_block_size: usize,
}

/// MongoDB specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoConfig {
    /// Connection URI
    pub uri: String,
    /// Database name
    pub database: String,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Server selection timeout in seconds
    pub server_selection_timeout: u64,
    /// Socket timeout in seconds
    pub socket_timeout: u64,
    /// Enable TLS
    pub tls: bool,
    /// Read preference
    pub read_preference: String,
    /// Write concern
    pub write_concern: WriteConcern,
}

/// MongoDB write concern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteConcern {
    /// Write concern level
    pub w: String,
    /// Journal acknowledgment
    pub j: bool,
    /// Write timeout in milliseconds
    pub wtimeout: u64,
}

/// Redis specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis host
    pub host: String,
    /// Redis port
    pub port: u16,
    /// Database number
    pub database: u8,
    /// Password
    pub password: Option<String>,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Command timeout in seconds
    pub command_timeout: u64,
    /// Enable TLS
    pub tls: bool,
    /// Connection pool size
    pub pool_size: u32,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Minimum pool size
    pub min_connections: u32,
    /// Maximum pool size
    pub max_connections: u32,
    /// Connection idle timeout in seconds
    pub idle_timeout: u64,
    /// Maximum connection lifetime in seconds
    pub max_lifetime: u64,
    /// Connection acquire timeout in seconds
    pub acquire_timeout: u64,
    /// Enable connection validation
    pub test_on_borrow: bool,
    /// Test query for validation
    pub validation_query: String,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,
    /// Health check interval in seconds
    pub interval: u64,
    /// Health check timeout in seconds
    pub timeout: u64,
    /// Number of retries before marking unhealthy
    pub max_retries: u32,
    /// Recovery check interval in seconds
    pub recovery_interval: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Session configuration
    pub session: SessionConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Security headers configuration
    pub headers: SecurityHeadersConfig,
    /// CORS configuration
    pub cors: CorsConfig,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret: String,
    /// Token expiration in seconds
    pub expiration: u64,
    /// Refresh token expiration in seconds
    pub refresh_expiration: u64,
    /// JWT issuer
    pub issuer: String,
    /// JWT audience
    pub audience: String,
    /// Algorithm to use for signing
    pub algorithm: String,
    /// Enable token blacklisting
    pub enable_blacklist: bool,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout in seconds
    pub timeout: u64,
    /// Session cookie name
    pub cookie_name: String,
    /// Cookie domain
    pub cookie_domain: Option<String>,
    /// Cookie path
    pub cookie_path: String,
    /// Cookie secure flag
    pub cookie_secure: bool,
    /// Cookie HTTP only flag
    pub cookie_http_only: bool,
    /// Cookie SameSite attribute
    pub cookie_same_site: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per minute per IP
    pub requests_per_minute_per_ip: u32,
    /// Requests per minute per user
    pub requests_per_minute_per_user: u32,
    /// Burst limit
    pub burst_limit: u32,
    /// Rate limit window in seconds
    pub window_seconds: u64,
    /// Enable rate limit headers in response
    pub include_headers: bool,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub content_security_policy: String,
    /// X-Frame-Options header
    pub x_frame_options: String,
    /// X-Content-Type-Options header
    pub x_content_type_options: String,
    /// X-XSS-Protection header
    pub x_xss_protection: String,
    /// Strict-Transport-Security header
    pub strict_transport_security: String,
    /// Referrer-Policy header
    pub referrer_policy: String,
}

/// CORS configuration
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
    /// Exposed headers
    pub expose_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight requests
    pub max_age: u64,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Query timeout in seconds
    pub query_timeout: u64,
    /// Bulk operation batch size
    pub batch_size: usize,
    /// Maximum concurrent operations
    pub max_concurrent_operations: u32,
    /// Enable query result caching
    pub enable_query_cache: bool,
    /// Query cache TTL in seconds
    pub query_cache_ttl: u64,
    /// Maximum query cache size
    pub max_query_cache_size: usize,
    /// Enable connection pooling
    pub enable_connection_pooling: bool,
    /// Database operation retry settings
    pub retry: RetryConfig,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Enable retries
    pub enabled: bool,
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics export interval in seconds
    pub metrics_interval: u64,
    /// Enable tracing
    pub enable_tracing: bool,
    /// Tracing sample rate (0.0 to 1.0)
    pub tracing_sample_rate: f64,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Performance monitoring threshold in milliseconds
    pub performance_threshold_ms: u64,
    /// Enable health monitoring
    pub enable_health_monitoring: bool,
    /// Health check endpoints
    pub health_endpoints: Vec<String>,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable experimental features
    pub experimental_features: bool,
    /// Enable debug mode
    pub debug_mode: bool,
    /// Enable development features
    pub development_features: bool,
    /// Feature-specific flags
    pub features: HashMap<String, bool>,
}

impl Default for SecureDatabaseConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
            access_control: AccessControlConfig::default(),
            audit: AuditConfig::default(),
            encryption: DataEncryptionConfig::default(),
            performance: PerformanceConfig::default(),
            monitoring: MonitoringConfig::default(),
            features: FeatureFlags::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres: PostgresConfig::default(),
            clickhouse: ClickHouseConfig::default(),
            mongodb: MongoConfig::default(),
            redis: RedisConfig::default(),
            pool: ConnectionPoolConfig::default(),
            health_check: HealthCheckConfig::default(),
        }
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "ai_core".to_string(),
            username: "ai_core".to_string(),
            password: "password".to_string(),
            ssl_mode: "prefer".to_string(),
            connect_timeout: 30,
            query_timeout: 60,
            enable_prepared_statements: true,
        }
    }
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8123,
            database: "ai_core".to_string(),
            username: "default".to_string(),
            password: "".to_string(),
            compression: true,
            connect_timeout: 30,
            query_timeout: 300,
            max_block_size: 100000,
        }
    }
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            uri: "mongodb://localhost:27017".to_string(),
            database: "ai_core".to_string(),
            connect_timeout: 30,
            server_selection_timeout: 30,
            socket_timeout: 60,
            tls: false,
            read_preference: "primary".to_string(),
            write_concern: WriteConcern::default(),
        }
    }
}

impl Default for WriteConcern {
    fn default() -> Self {
        Self {
            w: "majority".to_string(),
            j: true,
            wtimeout: 10000,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
            database: 0,
            password: None,
            connect_timeout: 30,
            command_timeout: 30,
            tls: false,
            pool_size: 10,
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            idle_timeout: 600,  // 10 minutes
            max_lifetime: 3600, // 1 hour
            acquire_timeout: 30,
            test_on_borrow: true,
            validation_query: "SELECT 1".to_string(),
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: 30,
            timeout: 10,
            max_retries: 3,
            recovery_interval: 60,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt: JwtConfig::default(),
            session: SessionConfig::default(),
            rate_limiting: RateLimitConfig::default(),
            headers: SecurityHeadersConfig::default(),
            cors: CorsConfig::default(),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-secret-key-change-this-in-production".to_string(),
            expiration: 3600,          // 1 hour
            refresh_expiration: 86400, // 24 hours
            issuer: "ai-core".to_string(),
            audience: "ai-core-users".to_string(),
            algorithm: "HS256".to_string(),
            enable_blacklist: true,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: 3600, // 1 hour
            cookie_name: "ai_core_session".to_string(),
            cookie_domain: None,
            cookie_path: "/".to_string(),
            cookie_secure: true,
            cookie_http_only: true,
            cookie_same_site: "Strict".to_string(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute_per_ip: 60,
            requests_per_minute_per_user: 100,
            burst_limit: 10,
            window_seconds: 60,
            include_headers: true,
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_security_policy: "default-src 'self'".to_string(),
            x_frame_options: "DENY".to_string(),
            x_content_type_options: "nosniff".to_string(),
            x_xss_protection: "1; mode=block".to_string(),
            strict_transport_security: "max-age=31536000; includeSubDomains".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
            ],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            expose_headers: vec!["X-Request-ID".to_string()],
            allow_credentials: true,
            max_age: 3600,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            query_timeout: 30,
            batch_size: 1000,
            max_concurrent_operations: 100,
            enable_query_cache: true,
            query_cache_ttl: 300, // 5 minutes
            max_query_cache_size: 10000,
            enable_connection_pooling: true,
            retry: RetryConfig::default(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_interval: 60,
            enable_tracing: true,
            tracing_sample_rate: 0.1,
            enable_performance_monitoring: true,
            performance_threshold_ms: 1000,
            enable_health_monitoring: true,
            health_endpoints: vec![
                "/health".to_string(),
                "/ready".to_string(),
                "/metrics".to_string(),
            ],
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            experimental_features: false,
            debug_mode: false,
            development_features: false,
            features: HashMap::new(),
        }
    }
}

impl SecureDatabaseConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SecureDatabaseError> {
        let path = path.as_ref();
        info!("Loading configuration from: {}", path.display());

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))
            .map_err(|e| SecureDatabaseError::Configuration(e.to_string()))?;

        let config: Self = match path.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => serde_yaml::from_str(&contents).map_err(|e| {
                SecureDatabaseError::Configuration(format!("YAML parse error: {}", e))
            })?,
            Some("json") => serde_json::from_str(&contents).map_err(|e| {
                SecureDatabaseError::Configuration(format!("JSON parse error: {}", e))
            })?,
            Some("toml") => toml::from_str(&contents).map_err(|e| {
                SecureDatabaseError::Configuration(format!("TOML parse error: {}", e))
            })?,
            _ => {
                return Err(SecureDatabaseError::Configuration(
                    "Unsupported config file format. Use .yaml, .json, or .toml".to_string(),
                ))
            }
        };

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, SecureDatabaseError> {
        info!("Loading configuration from environment variables");

        let mut config = Self::default();

        // Database configuration from environment
        if let Ok(postgres_host) = std::env::var("POSTGRES_HOST") {
            config.database.postgres.host = postgres_host;
        }
        if let Ok(postgres_port) = std::env::var("POSTGRES_PORT") {
            config.database.postgres.port = postgres_port.parse().map_err(|e| {
                SecureDatabaseError::Configuration(format!("Invalid POSTGRES_PORT: {}", e))
            })?;
        }
        if let Ok(postgres_db) = std::env::var("POSTGRES_DATABASE") {
            config.database.postgres.database = postgres_db;
        }
        if let Ok(postgres_user) = std::env::var("POSTGRES_USERNAME") {
            config.database.postgres.username = postgres_user;
        }
        if let Ok(postgres_pass) = std::env::var("POSTGRES_PASSWORD") {
            config.database.postgres.password = postgres_pass;
        }

        // Redis configuration from environment
        if let Ok(redis_host) = std::env::var("REDIS_HOST") {
            config.database.redis.host = redis_host;
        }
        if let Ok(redis_port) = std::env::var("REDIS_PORT") {
            config.database.redis.port = redis_port.parse().map_err(|e| {
                SecureDatabaseError::Configuration(format!("Invalid REDIS_PORT: {}", e))
            })?;
        }
        if let Ok(redis_pass) = std::env::var("REDIS_PASSWORD") {
            config.database.redis.password = Some(redis_pass);
        }

        // JWT configuration from environment
        if let Ok(jwt_secret) = std::env::var("JWT_SECRET") {
            config.security.jwt.secret = jwt_secret;
        }

        // Feature flags from environment
        if let Ok(debug_mode) = std::env::var("DEBUG_MODE") {
            config.features.debug_mode = debug_mode.parse().unwrap_or(false);
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), SecureDatabaseError> {
        debug!("Validating configuration");

        // Validate database configurations
        if self.database.postgres.host.is_empty() {
            return Err(SecureDatabaseError::Configuration(
                "PostgreSQL host cannot be empty".to_string(),
            ));
        }

        if self.database.postgres.port == 0 {
            return Err(SecureDatabaseError::Configuration(
                "PostgreSQL port must be greater than 0".to_string(),
            ));
        }

        if self.database.postgres.username.is_empty() {
            return Err(SecureDatabaseError::Configuration(
                "PostgreSQL username cannot be empty".to_string(),
            ));
        }

        // Validate security configurations
        if self.security.jwt.secret.len() < 32 {
            warn!("JWT secret is shorter than recommended 32 characters");
        }

        if self.security.jwt.expiration == 0 {
            return Err(SecureDatabaseError::Configuration(
                "JWT expiration must be greater than 0".to_string(),
            ));
        }

        // Validate performance configurations
        if self.performance.batch_size == 0 {
            return Err(SecureDatabaseError::Configuration(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        if self.performance.max_concurrent_operations == 0 {
            return Err(SecureDatabaseError::Configuration(
                "Max concurrent operations must be greater than 0".to_string(),
            ));
        }

        // Validate connection pool configurations
        if self.database.pool.min_connections > self.database.pool.max_connections {
            return Err(SecureDatabaseError::Configuration(
                "Min connections cannot be greater than max connections".to_string(),
            ));
        }

        info!("Configuration validation passed");
        Ok(())
    }

    /// Create a test configuration for development/testing
    pub fn test_config() -> Self {
        let mut config = Self::default();

        // Use test database names
        config.database.postgres.database = "ai_core_test".to_string();
        config.database.clickhouse.database = "ai_core_test".to_string();
        config.database.mongodb.database = "ai_core_test".to_string();

        // Reduce timeouts for faster tests
        config.database.postgres.connect_timeout = 5;
        config.database.postgres.query_timeout = 10;
        config.database.clickhouse.connect_timeout = 5;
        config.database.clickhouse.query_timeout = 10;

        // Disable some features for tests
        config.audit.enabled = false;
        config.security.rate_limiting.enabled = false;
        config.features.debug_mode = true;

        // Smaller pool sizes for tests
        config.database.pool.min_connections = 1;
        config.database.pool.max_connections = 5;

        config
    }

    /// Get feature flag value
    pub fn feature_enabled(&self, feature: &str) -> bool {
        self.features
            .features
            .get(feature)
            .copied()
            .unwrap_or(false)
    }

    /// Enable a feature flag
    pub fn enable_feature(&mut self, feature: &str) {
        self.features.features.insert(feature.to_string(), true);
    }

    /// Disable a feature flag
    pub fn disable_feature(&mut self, feature: &str) {
        self.features.features.insert(feature.to_string(), false);
    }

    /// Get configuration as JSON string
    pub fn to_json(&self) -> Result<String, SecureDatabaseError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SecureDatabaseError::SerializationError(e.to_string()))
    }

    /// Get configuration as YAML string
    pub fn to_yaml(&self) -> Result<String, SecureDatabaseError> {
        serde_yaml::to_string(self)
            .map_err(|e| SecureDatabaseError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config_creation() {
        let config = SecureDatabaseConfig::default();
        assert_eq!(config.database.postgres.host, "localhost");
        assert_eq!(config.database.postgres.port, 5432);
        assert_eq!(config.security.jwt.algorithm, "HS256");
        assert!(config.audit.enabled);
    }

    #[test]
    fn test_config_validation_success() {
        let config = SecureDatabaseConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_failure() {
        let mut config = SecureDatabaseConfig::default();
        config.database.postgres.host = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_test_config_creation() {
        let config = SecureDatabaseConfig::test_config();
        assert_eq!(config.database.postgres.database, "ai_core_test");
        assert!(!config.audit.enabled);
        assert!(config.features.debug_mode);
    }

    #[test]
    fn test_feature_flags() {
        let mut config = SecureDatabaseConfig::default();

        assert!(!config.feature_enabled("test_feature"));

        config.enable_feature("test_feature");
        assert!(config.feature_enabled("test_feature"));

        config.disable_feature("test_feature");
        assert!(!config.feature_enabled("test_feature"));
    }

    #[test]
    fn test_json_serialization() {
        let config = SecureDatabaseConfig::test_config();
        let json = config.to_json();
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("ai_core_test"));
    }

    #[test]
    fn test_yaml_serialization() {
        let config = SecureDatabaseConfig::test_config();
        let yaml = config.to_yaml();
        assert!(yaml.is_ok());

        let yaml_str = yaml.unwrap();
        assert!(yaml_str.contains("ai_core_test"));
    }

    #[test]
    fn test_config_from_json_file() {
        let config = SecureDatabaseConfig::test_config();
        let json_content = config.to_json().unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), json_content).unwrap();

        // Rename to .json extension
        let json_path = temp_file.path().with_extension("json");
        fs::copy(temp_file.path(), &json_path).unwrap();

        let loaded_config = SecureDatabaseConfig::from_file(&json_path);
        assert!(loaded_config.is_ok());

        let loaded = loaded_config.unwrap();
        assert_eq!(loaded.database.postgres.database, "ai_core_test");

        // Cleanup
        fs::remove_file(json_path).ok();
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("POSTGRES_HOST", "test-host");
        std::env::set_var("POSTGRES_PORT", "5433");
        std::env::set_var("DEBUG_MODE", "true");

        let config = SecureDatabaseConfig::from_env().unwrap();

        assert_eq!(config.database.postgres.host, "test-host");
        assert_eq!(config.database.postgres.port, 5433);
        assert!(config.features.debug_mode);

        // Cleanup environment variables
        std::env::remove_var("POSTGRES_HOST");
        std::env::remove_var("POSTGRES_PORT");
        std::env::remove_var("DEBUG_MODE");
    }
}
