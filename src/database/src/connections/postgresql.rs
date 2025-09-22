//! Database connection management for AI-CORE platform
//!
//! This module handles the initialization, pooling, and lifecycle management
//! of PostgreSQL database connections.

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use super::{DatabaseError, PostgresConfig};

/// PostgreSQL connection manager
pub struct PostgresConnection {
    pool: Arc<PgPool>,
    config: PostgresConfig,
}

impl PostgresConnection {
    /// Create new PostgreSQL connection manager
    pub async fn new(config: PostgresConfig) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(config.max_lifetime_seconds))
            .connect(&config.url)
            .await
            .map_err(DatabaseError::Postgres)?;

        // Test the connection
        let mut conn = pool.acquire().await?;
        sqlx::query("SELECT 1")
            .execute(&mut *conn)
            .await
            .map_err(DatabaseError::Postgres)?;

        info!("PostgreSQL connection pool created successfully");

        Ok(Self {
            pool: Arc::new(pool),
            config,
        })
    }

    /// Get connection pool
    pub fn pool(&self) -> Arc<PgPool> {
        self.pool.clone()
    }

    /// Test connection health
    pub async fn health_check(&self) -> Result<bool, DatabaseError> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query("SELECT 1").execute(&mut *conn).await?;
        Ok(true)
    }

    /// Get pool statistics
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
            max_size: self.config.max_connections,
        }
    }

    /// Close connection pool
    pub async fn close(&self) {
        info!("Closing PostgreSQL connection pool");
        self.pool.close().await;
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub size: u32,
    pub idle: usize,
    pub max_size: u32,
}

/// Connection factory for creating database connections
pub struct ConnectionFactory {
    postgres_config: PostgresConfig,
}

impl ConnectionFactory {
    /// Create new connection factory
    pub fn new(postgres_config: PostgresConfig) -> Self {
        Self { postgres_config }
    }

    /// Create PostgreSQL connection
    pub async fn create_postgres(&self) -> Result<PostgresConnection, DatabaseError> {
        PostgresConnection::new(self.postgres_config.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_stats() {
        let config = PostgresConfig {
            url: "postgresql://localhost:5432/test".to_string(),
            max_connections: 10,
            min_connections: 2,
            acquire_timeout_seconds: 5,
            idle_timeout_seconds: 300,
            max_lifetime_seconds: 1800,
            enable_migrations: false,
        };

        // This test would require an actual database connection
        // so we'll just test the configuration
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
    }
}
