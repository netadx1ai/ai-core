//! Database health monitoring for PostgreSQL
//!
//! This module provides health checking functionality for PostgreSQL connections,
//! monitoring connection pool status, and providing health metrics.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use crate::DatabaseError;

/// Health checker for PostgreSQL database
#[derive(Clone)]
pub struct HealthChecker {
    postgres_pool: Arc<PgPool>,
    config: HealthConfig,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub check_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub max_response_time_ms: u64,
    pub enable_detailed_checks: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 30,
            timeout_seconds: 5,
            max_response_time_ms: 1000,
            enable_detailed_checks: false,
        }
    }
}

/// Overall health status
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub overall_healthy: bool,
    pub postgres: PostgresHealth,
    #[cfg(feature = "redis")]
    pub redis: Option<RedisHealth>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// PostgreSQL health status
#[derive(Debug, Clone, Serialize)]
pub struct PostgresHealth {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub connection_pool: PoolHealth,
    pub error_message: Option<String>,
    pub last_successful_connection: Option<chrono::DateTime<chrono::Utc>>,
}

/// Connection pool health information
#[derive(Debug, Clone, Serialize)]
pub struct PoolHealth {
    pub total_connections: u32,
    pub idle_connections: usize,
    pub active_connections: u32,
    pub pool_utilization_percent: f32,
}

/// Redis health status
#[cfg(feature = "redis")]
#[derive(Debug, Clone, Serialize)]
pub struct RedisHealth {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_ratio: f32,
    pub error_message: Option<String>,
    pub last_successful_connection: Option<chrono::DateTime<chrono::Utc>>,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(postgres_pool: Arc<PgPool>, config: HealthConfig) -> Self {
        Self {
            postgres_pool,
            config,
        }
    }

    /// Perform comprehensive health check
    pub async fn check_health(&self) -> Result<HealthStatus, DatabaseError> {
        debug!("Starting health check");
        let start_time = Instant::now();

        let postgres_health = self.check_postgres_health().await;
        let overall_healthy = postgres_health.healthy;

        let status = HealthStatus {
            overall_healthy,
            postgres: postgres_health,
            #[cfg(feature = "redis")]
            redis: None,
            last_check: chrono::Utc::now(),
        };

        let check_duration = start_time.elapsed();
        debug!("Health check completed in {:?}", check_duration);

        if status.overall_healthy {
            debug!("All systems healthy");
        } else {
            warn!("Health check detected issues: {:?}", status);
        }

        Ok(status)
    }

    /// Check PostgreSQL health
    async fn check_postgres_health(&self) -> PostgresHealth {
        let start_time = Instant::now();

        // Test basic connectivity
        let connection_result = sqlx::query("SELECT 1 as health_check")
            .fetch_one(&*self.postgres_pool)
            .await;

        let response_time = start_time.elapsed().as_millis() as u64;

        match connection_result {
            Ok(_) => {
                let pool_health = self.get_pool_health();

                let healthy = response_time <= self.config.max_response_time_ms
                    && pool_health.pool_utilization_percent < 95.0;

                if !healthy {
                    warn!(
                        "PostgreSQL health degraded: response_time={}ms, utilization={}%",
                        response_time, pool_health.pool_utilization_percent
                    );
                }

                PostgresHealth {
                    healthy,
                    response_time_ms: response_time,
                    connection_pool: pool_health,
                    error_message: None,
                    last_successful_connection: Some(chrono::Utc::now()),
                }
            }
            Err(e) => {
                error!("PostgreSQL health check failed: {}", e);

                PostgresHealth {
                    healthy: false,
                    response_time_ms: response_time,
                    connection_pool: self.get_pool_health(),
                    error_message: Some(e.to_string()),
                    last_successful_connection: None,
                }
            }
        }
    }

    /// Get connection pool health metrics
    fn get_pool_health(&self) -> PoolHealth {
        let total_connections = self.postgres_pool.size();
        let idle_connections = self.postgres_pool.num_idle();
        let active_connections = total_connections.saturating_sub(idle_connections as u32);

        let pool_utilization_percent = if total_connections > 0 {
            (active_connections as f32 / total_connections as f32) * 100.0
        } else {
            0.0
        };

        PoolHealth {
            total_connections,
            idle_connections,
            active_connections,
            pool_utilization_percent,
        }
    }

    /// Perform detailed health checks (when enabled)
    pub async fn detailed_health_check(&self) -> Result<DetailedHealthStatus, DatabaseError> {
        if !self.config.enable_detailed_checks {
            return Err(DatabaseError::Validation(
                "Detailed health checks not enabled".to_string(),
            ));
        }

        let basic_status = self.check_health().await?;

        // Perform additional checks
        let table_check = self.check_table_access().await;
        let transaction_check = self.check_transaction_capability().await;
        let performance_metrics = self.collect_performance_metrics().await;

        Ok(DetailedHealthStatus {
            basic_status,
            table_access: table_check.is_ok(),
            transaction_capability: transaction_check.is_ok(),
            performance_metrics,
            detailed_errors: vec![
                table_check.err().map(|e| e.to_string()),
                transaction_check.err().map(|e| e.to_string()),
            ]
            .into_iter()
            .flatten()
            .collect(),
        })
    }

    /// Test basic table access
    async fn check_table_access(&self) -> Result<(), DatabaseError> {
        sqlx::query("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'")
            .fetch_one(&*self.postgres_pool)
            .await?;
        Ok(())
    }

    /// Test transaction capability
    async fn check_transaction_capability(&self) -> Result<(), DatabaseError> {
        let mut tx = self.postgres_pool.begin().await?;
        sqlx::query("SELECT 1").execute(&mut *tx).await?;
        tx.rollback().await?;
        Ok(())
    }

    /// Collect performance metrics
    async fn collect_performance_metrics(&self) -> PerformanceMetrics {
        // Collect basic performance metrics
        let query_count_result = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(calls), 0) FROM pg_stat_user_functions",
        )
        .fetch_optional(&*self.postgres_pool)
        .await;

        let active_connections_result = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM pg_stat_activity WHERE state = 'active'",
        )
        .fetch_optional(&*self.postgres_pool)
        .await;

        PerformanceMetrics {
            total_queries: query_count_result.unwrap_or(Some(0)).unwrap_or(0),
            active_connections: active_connections_result.unwrap_or(Some(0)).unwrap_or(0),
            cache_hit_ratio: 0.0,       // Would require more complex query
            average_query_time_ms: 0.0, // Would require query history
        }
    }

    /// Start background health monitoring
    pub async fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let checker = self.clone();
        let interval = Duration::from_secs(checker.config.check_interval_seconds);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;

                match checker.check_health().await {
                    Ok(status) => {
                        if status.overall_healthy {
                            debug!("Periodic health check: All systems healthy");
                        } else {
                            warn!("Periodic health check: Issues detected - {:?}", status);
                        }
                    }
                    Err(e) => {
                        error!("Health check failed: {}", e);
                    }
                }
            }
        })
    }
}

/// Detailed health status with additional metrics
#[derive(Debug, Clone, Serialize)]
pub struct DetailedHealthStatus {
    pub basic_status: HealthStatus,
    pub table_access: bool,
    pub transaction_capability: bool,
    pub performance_metrics: PerformanceMetrics,
    pub detailed_errors: Vec<String>,
}

/// Performance metrics for database operations
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceMetrics {
    pub total_queries: i64,
    pub active_connections: i64,
    pub cache_hit_ratio: f64,
    pub average_query_time_ms: f64,
}

/// Health check result for HTTP endpoints
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<HealthStatus>,
}

impl HealthCheckResponse {
    /// Create a healthy response
    pub fn healthy(details: Option<HealthStatus>) -> Self {
        Self {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            details,
        }
    }

    /// Create an unhealthy response
    pub fn unhealthy(details: Option<HealthStatus>) -> Self {
        Self {
            status: "unhealthy".to_string(),
            timestamp: chrono::Utc::now(),
            details,
        }
    }

    /// Create a degraded response
    pub fn degraded(details: Option<HealthStatus>) -> Self {
        Self {
            status: "degraded".to_string(),
            timestamp: chrono::Utc::now(),
            details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_config_default() {
        let config = HealthConfig::default();
        assert_eq!(config.check_interval_seconds, 30);
        assert_eq!(config.timeout_seconds, 5);
        assert_eq!(config.max_response_time_ms, 1000);
        assert!(!config.enable_detailed_checks);
    }

    #[test]
    fn test_pool_health_utilization() {
        let pool_health = PoolHealth {
            total_connections: 20,
            idle_connections: 5,
            active_connections: 15,
            pool_utilization_percent: 75.0,
        };

        assert_eq!(pool_health.active_connections, 15);
        assert_eq!(pool_health.pool_utilization_percent, 75.0);
    }

    #[test]
    fn test_health_check_response() {
        let healthy = HealthCheckResponse::healthy(None);
        assert_eq!(healthy.status, "healthy");

        let unhealthy = HealthCheckResponse::unhealthy(None);
        assert_eq!(unhealthy.status, "unhealthy");

        let degraded = HealthCheckResponse::degraded(None);
        assert_eq!(degraded.status, "degraded");
    }
}
