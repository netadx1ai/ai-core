//! Database connection management for AI-CORE platform
//!
//! This module handles the initialization, pooling, and lifecycle management
//! of all database connections including PostgreSQL, ClickHouse, MongoDB, and Redis.

pub mod postgresql;

#[cfg(feature = "clickhouse")]
pub mod clickhouse;

#[cfg(feature = "mongodb")]
pub mod mongodb;

#[cfg(feature = "redis")]
pub mod redis;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::DatabaseError;

// Re-export connection types
pub use postgresql::{PoolStats, PostgresConnection};

#[cfg(feature = "clickhouse")]
pub use clickhouse::ClickHouseConnection;

#[cfg(feature = "mongodb")]
pub use mongodb::{AggregationOps, DatabaseStats, DocumentOps, MongoConnection, MongoStats};

#[cfg(feature = "redis")]
pub use redis::{RedisConfig, RedisConnection};

/// Connection factory for creating database connections
pub struct ConnectionFactory {
    postgres_config: Option<PostgresConfig>,

    #[cfg(feature = "clickhouse")]
    clickhouse_config: Option<ClickHouseConfig>,

    #[cfg(feature = "mongodb")]
    mongodb_config: Option<MongoConfig>,

    #[cfg(feature = "redis")]
    redis_config: Option<RedisConfig>,
}

impl ConnectionFactory {
    /// Create new connection factory with PostgreSQL configuration
    pub fn with_postgres(postgres_config: PostgresConfig) -> Self {
        Self {
            postgres_config: Some(postgres_config),

            #[cfg(feature = "clickhouse")]
            clickhouse_config: None,

            #[cfg(feature = "mongodb")]
            mongodb_config: None,

            #[cfg(feature = "redis")]
            redis_config: None,
        }
    }

    /// Add ClickHouse configuration
    #[cfg(feature = "clickhouse")]
    pub fn with_clickhouse(mut self, clickhouse_config: ClickHouseConfig) -> Self {
        self.clickhouse_config = Some(clickhouse_config);
        self
    }

    /// Add MongoDB configuration
    #[cfg(feature = "mongodb")]
    pub fn with_mongodb(mut self, mongodb_config: MongoConfig) -> Self {
        self.mongodb_config = Some(mongodb_config);
        self
    }

    /// Add Redis configuration
    #[cfg(feature = "redis")]
    pub fn with_redis(mut self, redis_config: RedisConfig) -> Self {
        self.redis_config = Some(redis_config);
        self
    }

    /// Create PostgreSQL connection
    pub async fn create_postgres(&self) -> Result<PostgresConnection, DatabaseError> {
        match &self.postgres_config {
            Some(config) => PostgresConnection::new(config.clone()).await,
            None => Err(DatabaseError::Connection(
                "PostgreSQL configuration not provided".to_string(),
            )),
        }
    }

    /// Create ClickHouse connection
    #[cfg(feature = "clickhouse")]
    pub async fn create_clickhouse(&self) -> Result<ClickHouseConnection, DatabaseError> {
        match &self.clickhouse_config {
            Some(config) => ClickHouseConnection::new(config.clone()).await,
            None => Err(DatabaseError::Connection(
                "ClickHouse configuration not provided".to_string(),
            )),
        }
    }

    /// Create MongoDB connection
    #[cfg(feature = "mongodb")]
    pub async fn create_mongodb(&self) -> Result<MongoConnection, DatabaseError> {
        match &self.mongodb_config {
            Some(config) => MongoConnection::new(config.clone()).await,
            None => Err(DatabaseError::Connection(
                "MongoDB configuration not provided".to_string(),
            )),
        }
    }

    /// Create Redis connection
    #[cfg(feature = "redis")]
    pub async fn create_redis(&self) -> Result<RedisConnection, DatabaseError> {
        match &self.redis_config {
            Some(config) => RedisConnection::new(config.clone()).await,
            None => Err(DatabaseError::Connection(
                "Redis configuration not provided".to_string(),
            )),
        }
    }
}

/// PostgreSQL configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub enable_migrations: bool,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost:5432/ai_core".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 10,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
            enable_migrations: true,
        }
    }
}

/// ClickHouse configuration
#[cfg(feature = "clickhouse")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClickHouseConfig {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub pool_size: u32,
    pub timeout_seconds: u64,
    pub compression: bool,
    pub secure: bool,
}

#[cfg(feature = "clickhouse")]
impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            database: "automation_analytics".to_string(),
            username: "default".to_string(),
            password: "".to_string(),
            pool_size: 10,
            timeout_seconds: 30,
            compression: true,
            secure: false,
        }
    }
}

/// MongoDB configuration
#[cfg(feature = "mongodb")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MongoConfig {
    pub url: String,
    pub database: String,
    pub max_pool_size: u32,
    pub min_pool_size: u32,
    pub max_idle_time_seconds: u64,
    pub connect_timeout_seconds: u64,
    pub server_selection_timeout_seconds: u64,
}

#[cfg(feature = "mongodb")]
impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            url: "mongodb://localhost:27017".to_string(),
            database: "ai_core_content".to_string(),
            max_pool_size: 20,
            min_pool_size: 5,
            max_idle_time_seconds: 600,
            connect_timeout_seconds: 10,
            server_selection_timeout_seconds: 30,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_interval_seconds: u64,
    pub slow_query_threshold_ms: u64,
    pub health_check_interval_seconds: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval_seconds: 60,
            slow_query_threshold_ms: 1000,
            health_check_interval_seconds: 30,
        }
    }
}

/// Health check result for connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
}

/// Connection statistics
pub trait ConnectionStats {
    fn connection_count(&self) -> u32;
    fn active_connections(&self) -> u32;
    fn idle_connections(&self) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_config_default() {
        let config = PostgresConfig::default();
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert!(config.enable_migrations);
    }

    #[cfg(feature = "clickhouse")]
    #[test]
    fn test_clickhouse_config_default() {
        let config = ClickHouseConfig::default();
        assert_eq!(config.database, "automation_analytics");
        assert_eq!(config.pool_size, 10);
        assert!(config.compression);
    }

    #[cfg(feature = "mongodb")]
    #[test]
    fn test_mongodb_config_default() {
        let config = MongoConfig::default();
        assert_eq!(config.database, "ai_core_content");
        assert_eq!(config.max_pool_size, 20);
    }

    #[cfg(feature = "redis")]
    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.max_connections, 20);
        assert!(!config.enable_cluster);
    }

    #[test]
    fn test_connection_factory_postgres() {
        let config = PostgresConfig::default();
        let factory = ConnectionFactory::with_postgres(config);
        assert!(factory.postgres_config.is_some());
    }
}
