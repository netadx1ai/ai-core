//! Application Configuration
//!
//! This module defines the configuration structure for the AI-PLATFORM API Gateway.
//! It uses the `config` crate to load settings from a YAML file and environment
//! variables, providing a unified and flexible configuration system.

use serde::Deserialize;
use std::collections::HashMap;

/// Main configuration for the application
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub environment: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub rate_limiting: RateLimitConfig,
    pub routing: RoutingConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)] // Use default if 'integrations' is missing
    pub integrations: IntegrationsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub max_connections: u32,
    pub timeout_seconds: u64,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout_seconds: u64,
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiry_seconds: i64,
    pub refresh_token_expiry_seconds: i64,
    pub bcrypt_cost: u32,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub redis_key_prefix: String,
    pub default_burst_size: u32,
    pub cleanup_interval_seconds: u64,
}

/// Service routing configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RoutingConfig {
    pub services: HashMap<String, ServiceConfig>,
    pub circuit_breaker_enabled: bool,
    pub circuit_breaker_failure_threshold: f64,
    pub circuit_breaker_timeout_seconds: u64,
    pub health_check_interval_seconds: u64,
}

/// Individual service configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub url: String,
    pub timeout_seconds: u64,
    pub retries: u32,
    pub enabled: bool,
}

/// Observability (metrics and tracing) configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
    pub jaeger_endpoint: String,
    pub metrics_port: u16,
}

/// Configuration for third-party integrations
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IntegrationsConfig {
    #[serde(default)] // Use default if 'zapier' is missing
    pub zapier: ZapierConfig,
}

/// Zapier-specific configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ZapierConfig {
    /// The secret key used to validate incoming webhooks from Zapier.
    /// If not set, signature validation will be skipped (not recommended for production).
    pub secret_key: Option<String>,
}

impl Config {
    /// Load configuration from environment variables and config files
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let environment = std::env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "development".into());

        let builder = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(
                config::File::with_name(&format!("config/environments/{}", environment))
                    .required(false),
            )
            .add_source(config::Environment::with_prefix("APP").separator("__"));

        builder.build()?.try_deserialize()
    }

    /// Check if the environment is development
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// Check if the environment is production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.auth.jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT secret must be at least 32 characters long"
            ));
        }
        Ok(())
    }
}

// Default implementations for sub-configurations

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: num_cpus::get(),
            max_connections: 1024,
            timeout_seconds: 30,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://user:password@localhost/ai_core".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            idle_timeout_seconds: 300,
            max_lifetime_seconds: 1800,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1/".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "a_very_secure_default_secret_key_that_is_long_enough".to_string(),
            jwt_expiry_seconds: 3600,             // 1 hour
            refresh_token_expiry_seconds: 604800, // 7 days
            bcrypt_cost: 12,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            redis_key_prefix: "rate_limit:".to_string(),
            default_burst_size: 100,
            cleanup_interval_seconds: 300,
        }
    }
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            services: HashMap::new(),
            circuit_breaker_enabled: true,
            circuit_breaker_failure_threshold: 0.5,
            circuit_breaker_timeout_seconds: 30,
            health_check_interval_seconds: 60,
        }
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            tracing_enabled: true,
            jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            metrics_port: 9090,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env_with_defaults() {
        // This test requires a temporary config file, which is complex to set up here.
        // It's better to test this in an integration test where file IO is more appropriate.
        // For now, we'll just check that the default config can be built.
        let config = Config::from_env();
        assert!(config.is_ok());
    }

    #[test]
    fn test_config_validation_jwt_secret() {
        let mut config = Config::from_env().unwrap();
        config.auth.jwt_secret = "short".to_string();
        assert!(config.validate().is_err());

        config.auth.jwt_secret = "a_much_longer_and_therefore_more_secure_secret_key".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_environment_detection() {
        let mut config = Config::from_env().unwrap();
        config.environment = "development".to_string();
        assert!(config.is_development());
        assert!(!config.is_production());

        config.environment = "production".to_string();
        assert!(!config.is_development());
        assert!(config.is_production());
    }

    #[test]
    fn test_default_configurations() {
        let server_config = ServerConfig::default();
        assert_eq!(server_config.port, 8080);

        let auth_config = AuthConfig::default();
        assert_eq!(auth_config.bcrypt_cost, 12);

        let integrations_config = IntegrationsConfig::default();
        assert!(integrations_config.zapier.secret_key.is_none());
    }
}
