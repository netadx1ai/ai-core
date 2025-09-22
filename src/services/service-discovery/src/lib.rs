//! Service Discovery Library
//!
//! AI-CORE Service Discovery and Registry Service
//!
//! This library provides comprehensive service discovery capabilities including:
//! - Service registration and deregistration
//! - Health monitoring with multiple check types
//! - Load balancing with various strategies
//! - Service mesh integration
//! - Circuit breaker patterns
//! - Configuration management
//!
//! # Features
//!
//! - **Multi-protocol support**: HTTP, HTTPS, gRPC, TCP
//! - **Health checking**: HTTP, TCP, gRPC, and script-based checks
//! - **Load balancing**: Round-robin, least connections, weighted, consistent hash, random, IP hash
//! - **Service mesh**: Integration with Consul, etcd, Kubernetes
//! - **Circuit breakers**: Automatic failover and recovery
//! - **Observability**: Metrics, tracing, and comprehensive logging
//! - **High availability**: Redis and PostgreSQL backend with clustering support
//!
//! # Quick Start
//!
//! ```no_run
//! use service_discovery::{
//!     config::ServiceDiscoveryConfig,
//!     registry::{ServiceRegistry, ServiceRegistryImpl},
//!     health::{HealthMonitor, HealthMonitorImpl},
//!     load_balancer::{LoadBalancer, LoadBalancerImpl},
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = Arc::new(ServiceDiscoveryConfig::default());
//!
//!     // Initialize services
//!     // let registry = ServiceRegistryImpl::new(db_pool, redis_pool, config.clone());
//!     // let health_monitor = HealthMonitorImpl::new(config.clone(), registry.clone());
//!     // let load_balancer = LoadBalancerImpl::new(config.clone());
//!
//!     // Start services
//!     // registry.initialize().await?;
//!     // health_monitor.start_monitoring().await?;
//!
//!     Ok(())
//! }
//! ```

use thiserror::Error;

pub mod config;
pub mod handlers;
pub mod health;
pub mod load_balancer;
pub mod models;
pub mod registry;

// Re-export commonly used types
pub use config::{Args, ServiceDiscoveryConfig};
pub use health::{HealthMonitor, HealthMonitorImpl, HealthMonitoringStats};
pub use load_balancer::{LoadBalancer, LoadBalancerImpl};
pub use models::{
    CircuitBreakerConfig, HealthCheckConfig, HealthCheckResult, HealthCheckType,
    HealthCheckTypeConfig, HealthStatus, LoadBalancerStats, LoadBalancingStrategy,
    RegisterServiceRequest, ServiceDiscoveryQuery, ServiceDiscoveryResponse, ServiceInstance,
    ServiceRegistration, ServiceStatistics, ServiceStatus, UpdateServiceRequest,
};
pub use registry::{ServiceRegistry, ServiceRegistryImpl};

/// Service Discovery library errors
#[derive(Error, Debug)]
pub enum ServiceDiscoveryError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Load balancer error: {0}")]
    LoadBalancer(String),

    #[error("Circuit breaker open for service: {0}")]
    CircuitBreakerOpen(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for the service discovery library
pub type Result<T> = std::result::Result<T, ServiceDiscoveryError>;

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default service discovery port
pub const DEFAULT_PORT: u16 = 8080;

/// Default health check interval in seconds
pub const DEFAULT_HEALTH_CHECK_INTERVAL: u32 = 30;

/// Default service TTL in seconds
pub const DEFAULT_SERVICE_TTL: u32 = 30;

/// Maximum number of concurrent health checks
pub const MAX_CONCURRENT_HEALTH_CHECKS: usize = 100;

/// Maximum number of services per discovery query
pub const MAX_DISCOVERY_LIMIT: u32 = 100;

/// Circuit breaker default settings
pub mod circuit_breaker {
    pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
    pub const DEFAULT_RECOVERY_TIMEOUT: u32 = 60;
    pub const DEFAULT_SUCCESS_THRESHOLD: u32 = 3;
    pub const DEFAULT_REQUEST_TIMEOUT: u32 = 30;
    pub const DEFAULT_MAX_RETRIES: u32 = 3;
}

/// Load balancer default settings
pub mod lb_defaults {
    pub const DEFAULT_WEIGHT: u32 = 100;
    pub const DEFAULT_VIRTUAL_NODES: u32 = 150;
    pub const MAX_RESPONSE_TIME_SAMPLES: usize = 1000;
}

/// Health check default settings
pub mod health_check {
    pub const DEFAULT_TIMEOUT: u32 = 5;
    pub const DEFAULT_FAILURE_THRESHOLD: u32 = 3;
    pub const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;
    pub const DEFAULT_HTTP_PATH: &str = "/health";
    pub const DEFAULT_GRPC_SERVICE: &str = "health";
}

/// Utility functions and helpers
pub mod utils {
    use crate::ServiceDiscoveryError;
    use std::time::Duration;

    /// Parse duration from string (e.g., "30s", "5m", "1h")
    pub fn parse_duration(s: &str) -> Result<Duration, ServiceDiscoveryError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ServiceDiscoveryError::Config(
                "Empty duration string".to_string(),
            ));
        }

        let (value_str, unit) = if s.ends_with("ms") {
            (&s[..s.len() - 2], "ms")
        } else if s.ends_with('s') {
            (&s[..s.len() - 1], "s")
        } else if s.ends_with('m') {
            (&s[..s.len() - 1], "m")
        } else if s.ends_with('h') {
            (&s[..s.len() - 1], "h")
        } else {
            (s, "s") // Default to seconds
        };

        let value: u64 = value_str
            .parse()
            .map_err(|_| ServiceDiscoveryError::Config(format!("Invalid duration value: {}", s)))?;

        let duration = match unit {
            "ms" => Duration::from_millis(value),
            "s" => Duration::from_secs(value),
            "m" => Duration::from_secs(value * 60),
            "h" => Duration::from_secs(value * 3600),
            _ => {
                return Err(ServiceDiscoveryError::Config(format!(
                    "Invalid duration unit: {}",
                    unit
                )))
            }
        };

        Ok(duration)
    }

    /// Validate service name according to RFC 1123
    pub fn validate_service_name(name: &str) -> Result<(), ServiceDiscoveryError> {
        if name.is_empty() || name.len() > 63 {
            return Err(ServiceDiscoveryError::Validation(
                validator::ValidationErrors::new(), // Simplified for example
            ));
        }

        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(ServiceDiscoveryError::Validation(
                validator::ValidationErrors::new(),
            ));
        }

        if name.starts_with('-') || name.ends_with('-') {
            return Err(ServiceDiscoveryError::Validation(
                validator::ValidationErrors::new(),
            ));
        }

        Ok(())
    }

    /// Generate consistent hash for a key
    pub fn consistent_hash(key: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Calculate weighted selection based on service weights
    pub fn calculate_weighted_selection(weights: &[u32]) -> Option<usize> {
        let total_weight: u32 = weights.iter().sum();
        if total_weight == 0 {
            return None;
        }

        let mut rng = rand::thread_rng();
        let random_weight = rand::Rng::gen_range(&mut rng, 0..total_weight);

        let mut current_weight = 0;
        for (index, &weight) in weights.iter().enumerate() {
            current_weight += weight;
            if random_weight < current_weight {
                return Some(index);
            }
        }

        None
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        config::{Args, ServiceDiscoveryConfig},
        health::{HealthMonitor, HealthMonitorImpl, HealthMonitoringStats},
        load_balancer::{LoadBalancer, LoadBalancerImpl},
        models::{
            HealthCheckConfig, HealthCheckResult, HealthStatus, LoadBalancingStrategy,
            RegisterServiceRequest, ServiceDiscoveryQuery, ServiceDiscoveryResponse,
            ServiceInstance, ServiceRegistration, ServiceStatus, UpdateServiceRequest,
        },
        registry::{ServiceRegistry, ServiceRegistryImpl},
        utils, Result, ServiceDiscoveryError,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_version_available() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_PORT, 8080);
        assert_eq!(DEFAULT_HEALTH_CHECK_INTERVAL, 30);
        assert_eq!(DEFAULT_SERVICE_TTL, 30);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            utils::parse_duration("30s").unwrap(),
            Duration::from_secs(30)
        );
        assert_eq!(
            utils::parse_duration("5m").unwrap(),
            Duration::from_secs(300)
        );
        assert_eq!(
            utils::parse_duration("1h").unwrap(),
            Duration::from_secs(3600)
        );
        assert_eq!(
            utils::parse_duration("500ms").unwrap(),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn test_validate_service_name() {
        assert!(utils::validate_service_name("test-service").is_ok());
        assert!(utils::validate_service_name("service123").is_ok());
        assert!(utils::validate_service_name("").is_err());
        assert!(utils::validate_service_name("-invalid").is_err());
        assert!(utils::validate_service_name("invalid-").is_err());
        assert!(utils::validate_service_name("invalid_name").is_err());
    }

    #[test]
    fn test_consistent_hash() {
        let hash1 = utils::consistent_hash("test-key");
        let hash2 = utils::consistent_hash("test-key");
        let hash3 = utils::consistent_hash("different-key");

        assert_eq!(hash1, hash2); // Same key should produce same hash
        assert_ne!(hash1, hash3); // Different keys should produce different hashes
    }
}
