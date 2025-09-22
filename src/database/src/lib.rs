//! Database Layer for AI-CORE Intelligent Automation Platform
//!
//! This module provides a unified database abstraction layer with initial support for PostgreSQL.
//! Additional database support (MongoDB, ClickHouse, Redis) will be added incrementally.

pub mod connections;
pub mod health;
pub mod migrations;
pub mod repositories;
pub mod seeders;

#[cfg(feature = "clickhouse")]
pub mod analytics;

use anyhow::{Context, Result};
// Removed unused imports DateTime and Utc
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;

// Re-export specific items to avoid ambiguity
pub use connections::{ConnectionFactory, ConnectionHealth, PostgresConfig};
pub use health::*;
pub use migrations::*;
pub use repositories::*;
pub use seeders::*;

#[cfg(feature = "clickhouse")]
pub use connections::ClickHouseConfig;

#[cfg(feature = "mongodb")]
pub use connections::MongoConfig;

#[cfg(feature = "redis")]
pub use connections::RedisConfig;

#[cfg(feature = "clickhouse")]
pub use analytics::*;

/// Database configuration for all supported databases
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub postgresql: PostgresConfig,
    pub monitoring: MonitoringConfig,

    #[cfg(feature = "clickhouse")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse: Option<ClickHouseConfig>,

    #[cfg(feature = "mongodb")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<MongoConfig>,

    #[cfg(feature = "redis")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<RedisConfig>,
}

// Re-export configuration types from connections module
pub use connections::MonitoringConfig;

/// Main database manager that orchestrates all database connections
#[derive(Clone)]
pub struct DatabaseManager {
    pub postgres: Arc<PgPool>,
    pub config: DatabaseConfig,

    #[cfg(feature = "clickhouse")]
    pub clickhouse: Option<Arc<connections::ClickHouseConnection>>,

    #[cfg(feature = "mongodb")]
    pub mongodb: Option<Arc<connections::MongoConnection>>,

    #[cfg(feature = "redis")]
    pub redis: Option<Arc<connections::RedisConnection>>,
}

impl DatabaseManager {
    /// Initialize database connections
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        tracing::info!("Initializing database connections...");

        // Initialize PostgreSQL connection pool
        let postgres = Self::init_postgres(&config.postgresql)
            .await
            .context("Failed to initialize PostgreSQL connection")?;

        // Initialize ClickHouse connection (if configured)
        #[cfg(feature = "clickhouse")]
        let clickhouse = if let Some(ch_config) = &config.clickhouse {
            Some(Arc::new(
                connections::ClickHouseConnection::new(ch_config.clone())
                    .await
                    .context("Failed to initialize ClickHouse connection")?,
            ))
        } else {
            None
        };

        // Initialize MongoDB connection (if configured)
        #[cfg(feature = "mongodb")]
        let mongodb = if let Some(mongo_config) = &config.mongodb {
            Some(Arc::new(
                connections::MongoConnection::new(mongo_config.clone())
                    .await
                    .context("Failed to initialize MongoDB connection")?,
            ))
        } else {
            None
        };

        // Initialize Redis connection (if configured)
        #[cfg(feature = "redis")]
        let redis = if let Some(redis_config) = &config.redis {
            Some(Arc::new(
                connections::RedisConnection::new(redis_config.clone())
                    .await
                    .context("Failed to initialize Redis connection")?,
            ))
        } else {
            None
        };

        let manager = DatabaseManager {
            postgres,
            config,
            #[cfg(feature = "clickhouse")]
            clickhouse,
            #[cfg(feature = "mongodb")]
            mongodb,
            #[cfg(feature = "redis")]
            redis,
        };

        tracing::info!("Database connections initialized successfully");
        Ok(manager)
    }

    /// Initialize PostgreSQL connection pool
    async fn init_postgres(config: &PostgresConfig) -> Result<Arc<PgPool>> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(config.max_lifetime_seconds))
            .connect(&config.url)
            .await
            .context("Failed to create PostgreSQL connection pool")?;

        // Test connection
        let row = sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .context("Failed to test PostgreSQL connection")?;

        let value: i32 = row.try_get(0)?;
        if value != 1 {
            return Err(anyhow::anyhow!("PostgreSQL connection test failed"));
        }

        Ok(Arc::new(pool))
    }

    /// Get repository factory for data access
    pub fn repositories(&self) -> RepositoryFactory {
        RepositoryFactory::new(self.postgres.clone())
    }

    /// Execute a PostgreSQL transaction
    pub async fn execute_transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: for<'a> FnOnce(
                &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<R>> + Send + 'a>,
            > + Send,
        R: Send,
    {
        let mut tx = self.postgres.begin().await?;

        match f(&mut tx).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }

    /// Health check for database connections
    pub async fn health_check(&self) -> Result<health::HealthStatus> {
        let pg_health = self.check_postgres_health().await?;
        let mut overall_healthy = pg_health.healthy;

        // Check Redis health if available
        #[cfg(feature = "redis")]
        let redis_health = if let Some(redis) = &self.redis {
            match redis.health_check().await {
                Ok(is_healthy) => {
                    overall_healthy = overall_healthy && is_healthy;
                    let stats = redis.get_stats().await;
                    Some(health::RedisHealth {
                        healthy: is_healthy,
                        response_time_ms: 0, // Redis health check doesn't return response time
                        cache_hits: stats.cache_hits,
                        cache_misses: stats.cache_misses,
                        cache_hit_ratio: if stats.cache_hits + stats.cache_misses > 0 {
                            stats.cache_hits as f32 / (stats.cache_hits + stats.cache_misses) as f32
                                * 100.0
                        } else {
                            0.0
                        },
                        error_message: stats.last_error,
                        last_successful_connection: if is_healthy {
                            Some(chrono::Utc::now())
                        } else {
                            None
                        },
                    })
                }
                Err(e) => {
                    overall_healthy = false;
                    Some(health::RedisHealth {
                        healthy: false,
                        response_time_ms: 0,
                        cache_hits: 0,
                        cache_misses: 0,
                        cache_hit_ratio: 0.0,
                        error_message: Some(e.to_string()),
                        last_successful_connection: None,
                    })
                }
            }
        } else {
            None
        };

        Ok(health::HealthStatus {
            postgres: health::PostgresHealth {
                healthy: pg_health.healthy,
                response_time_ms: pg_health.response_time_ms,
                connection_pool: health::PoolHealth {
                    total_connections: pg_health.connection_pool_size,
                    idle_connections: self.postgres.num_idle(),
                    active_connections: pg_health.active_connections,
                    pool_utilization_percent: (pg_health.active_connections as f32
                        / pg_health.connection_pool_size as f32)
                        * 100.0,
                },
                error_message: pg_health.error_message.clone(),
                last_successful_connection: if pg_health.healthy {
                    Some(chrono::Utc::now())
                } else {
                    None
                },
            },
            #[cfg(feature = "redis")]
            redis: redis_health,
            overall_healthy,
            last_check: chrono::Utc::now(),
        })
    }

    async fn check_postgres_health(&self) -> Result<DatabaseHealthStatus> {
        let start_time = std::time::Instant::now();

        let result = sqlx::query("SELECT 1").fetch_one(&*self.postgres).await;

        let response_time = start_time.elapsed();

        match result {
            Ok(_) => Ok(DatabaseHealthStatus {
                healthy: true,
                response_time_ms: response_time.as_millis() as u64,
                error_message: None,
                connection_pool_size: self.postgres.size(),
                active_connections: self.postgres.size() - self.postgres.num_idle() as u32,
            }),
            Err(e) => Ok(DatabaseHealthStatus {
                healthy: false,
                response_time_ms: response_time.as_millis() as u64,
                error_message: Some(e.to_string()),
                connection_pool_size: self.postgres.size(),
                active_connections: 0,
            }),
        }
    }

    /// Graceful shutdown of database connections
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down database connections...");

        self.postgres.close().await;

        #[cfg(feature = "clickhouse")]
        if let Some(_ch) = &self.clickhouse {
            // ClickHouse connection doesn't need explicit cleanup
            tracing::info!("ClickHouse connection cleaned up");
        }

        tracing::info!("Database connections closed");
        Ok(())
    }
}

/// Overall health status
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub postgres: Option<health::PostgresHealth>,
    #[cfg(feature = "clickhouse")]
    pub clickhouse: Option<connections::ConnectionHealth>,
    pub overall_healthy: bool,
}

/// Health status for individual database
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseHealthStatus {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub connection_pool_size: u32,
    pub active_connections: u32,
}

/// Common database error types
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("PostgreSQL error: {0}")]
    Postgres(#[from] sqlx::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgresql: PostgresConfig::default(),
            monitoring: MonitoringConfig::default(),
            #[cfg(feature = "clickhouse")]
            clickhouse: None,
            #[cfg(feature = "mongodb")]
            mongodb: None,
            #[cfg(feature = "redis")]
            redis: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.postgresql.max_connections, 20);
        assert_eq!(config.postgresql.min_connections, 5);
        assert!(config.monitoring.enabled);
    }
}
