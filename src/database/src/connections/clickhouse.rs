//! ClickHouse connection management for AI-CORE platform
//!
//! This module handles high-performance analytics database connections,
//! bulk data insertion, and real-time query optimization.

use anyhow::Result;
use clickhouse::{inserter::Inserter, Client, Row};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::{ClickHouseConfig, ConnectionHealth, ConnectionStats, DatabaseError};

/// ClickHouse connection manager optimized for analytics workloads
pub struct ClickHouseConnection {
    client: Arc<Client>,
    config: ClickHouseConfig,
    stats: Arc<RwLock<ClickHouseStats>>,
}

impl ClickHouseConnection {
    /// Create new ClickHouse connection manager
    pub async fn new(config: ClickHouseConfig) -> Result<Self, DatabaseError> {
        info!("Initializing ClickHouse connection to {}", config.url);

        let mut client_builder = Client::default()
            .with_url(&config.url)
            .with_user(&config.username)
            .with_password(&config.password)
            .with_database(&config.database)
            .with_option("max_execution_time", &config.timeout_seconds.to_string())
            .with_option("max_memory_usage", "4000000000"); // 4GB limit

        if config.compression {
            client_builder = client_builder.with_compression(clickhouse::Compression::Lz4);
        }

        if config.secure {
            client_builder = client_builder.with_option("secure", "1");
        }

        let client = client_builder;

        // Test connection
        let start_time = Instant::now();
        let test_result = client
            .query("SELECT 1 as test")
            .fetch_one::<u8>()
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("ClickHouse connection test failed: {}", e))
            })?;

        let connection_time = start_time.elapsed();

        if test_result != 1 {
            return Err(DatabaseError::Connection(
                "ClickHouse connection test returned unexpected result".to_string(),
            ));
        }

        info!(
            "ClickHouse connection established successfully in {:?}",
            connection_time
        );

        let stats = ClickHouseStats {
            queries_executed: 0,
            total_query_time_ms: 0,
            bulk_inserts: 0,
            total_rows_inserted: 0,
            connection_errors: 0,
        };

        Ok(Self {
            client: Arc::new(client),
            config,
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Get ClickHouse client
    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    /// Execute a query and return results
    pub async fn query<T>(&self, sql: &str) -> Result<Vec<T>, DatabaseError>
    where
        T: Row + for<'b> serde::Deserialize<'b>,
    {
        let start_time = Instant::now();
        debug!("Executing ClickHouse query: {}", sql);

        let result = self.client.query(sql).fetch_all::<T>().await.map_err(|e| {
            error!("ClickHouse query failed: {}", e);
            DatabaseError::Connection(format!("Query execution failed: {}", e))
        });

        let query_time = start_time.elapsed();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.queries_executed += 1;
            stats.total_query_time_ms += query_time.as_millis() as u64;
        }

        if query_time.as_millis() > 1000 {
            warn!(
                "Slow ClickHouse query detected: {}ms",
                query_time.as_millis()
            );
        }

        debug!("Query executed in {:?}", query_time);
        result
    }

    /// Execute a query and return a single result
    pub async fn query_one<T>(&self, sql: &str) -> Result<T, DatabaseError>
    where
        T: Row + for<'b> serde::Deserialize<'b>,
    {
        let start_time = Instant::now();
        debug!("Executing ClickHouse single query: {}", sql);

        let result = self.client.query(sql).fetch_one::<T>().await.map_err(|e| {
            error!("ClickHouse single query failed: {}", e);
            DatabaseError::Connection(format!("Single query execution failed: {}", e))
        });

        let query_time = start_time.elapsed();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.queries_executed += 1;
            stats.total_query_time_ms += query_time.as_millis() as u64;
        }

        debug!("Single query executed in {:?}", query_time);
        result
    }

    /// Execute a query without returning results (for DDL, DML)
    pub async fn execute(&self, sql: &str) -> Result<(), DatabaseError> {
        let start_time = Instant::now();
        debug!("Executing ClickHouse command: {}", sql);

        let result = self.client.query(sql).execute().await.map_err(|e| {
            error!("ClickHouse command execution failed: {}", e);
            DatabaseError::Connection(format!("Command execution failed: {}", e))
        });

        let query_time = start_time.elapsed();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.queries_executed += 1;
            stats.total_query_time_ms += query_time.as_millis() as u64;
        }

        debug!("Command executed in {:?}", query_time);
        result
    }

    /// Create a bulk inserter for high-performance data insertion
    pub async fn create_inserter<T>(&self, table: &str) -> Result<Inserter<T>, DatabaseError>
    where
        T: Row + serde::Serialize,
    {
        info!("Creating bulk inserter for table: {}", table);

        let inserter = self
            .client
            .inserter(table)
            .map_err(|e| DatabaseError::Connection(format!("Failed to create inserter: {}", e)))?
            .with_max_bytes(100_000) // Batch size for optimal performance
            .with_period(Some(Duration::from_secs(30))); // Auto-commit after 30 seconds

        Ok(inserter)
    }

    /// Bulk insert data with automatic batching and error handling
    pub async fn bulk_insert<T>(&self, table: &str, data: Vec<T>) -> Result<u64, DatabaseError>
    where
        T: Row + serde::Serialize + Send,
    {
        let start_time = Instant::now();
        let row_count = data.len() as u64;

        info!(
            "Starting bulk insert of {} rows to table: {}",
            row_count, table
        );

        let mut inserter = self.create_inserter::<T>(table).await?;

        // Insert data in batches for memory efficiency
        let batch_size = 10_000;
        let mut inserted_rows = 0u64;

        for chunk in data.chunks(batch_size) {
            for row in chunk {
                inserter.write(row).map_err(|e| {
                    DatabaseError::Connection(format!("Failed to write row: {}", e))
                })?;
            }
            inserted_rows += chunk.len() as u64;

            // Log progress for large inserts
            if row_count > 100_000 && inserted_rows % 50_000 == 0 {
                info!("Bulk insert progress: {}/{} rows", inserted_rows, row_count);
            }
        }

        // Commit the batch
        inserter.commit().await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to commit bulk insert: {}", e))
        })?;

        let insert_time = start_time.elapsed();
        let rows_per_second = row_count as f64 / insert_time.as_secs_f64();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.bulk_inserts += 1;
            stats.total_rows_inserted += row_count;
        }

        info!(
            "Bulk insert completed: {} rows in {:?} ({:.0} rows/sec)",
            row_count, insert_time, rows_per_second
        );

        Ok(row_count)
    }

    /// Create materialized view for real-time analytics
    pub async fn create_materialized_view(
        &self,
        view_name: &str,
        query: &str,
        engine: &str,
    ) -> Result<(), DatabaseError> {
        let sql = format!(
            "CREATE MATERIALIZED VIEW IF NOT EXISTS {} ENGINE = {} AS {}",
            view_name, engine, query
        );

        info!("Creating materialized view: {}", view_name);
        self.execute(&sql).await?;
        info!("Materialized view created successfully: {}", view_name);

        Ok(())
    }

    /// Drop materialized view
    pub async fn drop_materialized_view(&self, view_name: &str) -> Result<(), DatabaseError> {
        let sql = format!("DROP VIEW IF EXISTS {}", view_name);

        info!("Dropping materialized view: {}", view_name);
        self.execute(&sql).await?;
        info!("Materialized view dropped successfully: {}", view_name);

        Ok(())
    }

    /// Optimize table performance (merge parts, update statistics)
    pub async fn optimize_table(&self, table: &str) -> Result<(), DatabaseError> {
        let sql = format!("OPTIMIZE TABLE {} FINAL", table);

        info!("Optimizing table: {}", table);
        self.execute(&sql).await?;
        info!("Table optimization completed: {}", table);

        Ok(())
    }

    /// Get table statistics
    pub async fn get_table_stats(&self, table: &str) -> Result<TableStats, DatabaseError> {
        let sql = format!(
            r#"
            SELECT
                table,
                total_rows,
                total_bytes,
                parts,
                active_parts
            FROM system.parts
            WHERE table = '{}' AND active = 1
            GROUP BY table
            "#,
            table
        );

        #[derive(Row, Deserialize)]
        struct RawStats {
            table: String,
            total_rows: u64,
            total_bytes: u64,
            parts: u64,
            active_parts: u64,
        }

        let raw_stats: Vec<RawStats> = self.query(&sql).await?;

        if let Some(stats) = raw_stats.first() {
            Ok(TableStats {
                table_name: stats.table.clone(),
                total_rows: stats.total_rows,
                total_bytes: stats.total_bytes,
                parts: stats.parts,
                active_parts: stats.active_parts,
            })
        } else {
            Err(DatabaseError::Connection(format!(
                "Table not found: {}",
                table
            )))
        }
    }

    /// Test connection health
    pub async fn health_check(&self) -> Result<ConnectionHealth, DatabaseError> {
        let start_time = Instant::now();

        let result = self
            .client
            .query("SELECT 1 as health_check")
            .fetch_one::<u8>()
            .await;

        let response_time = start_time.elapsed();

        match result {
            Ok(1) => Ok(ConnectionHealth {
                healthy: true,
                response_time_ms: response_time.as_millis() as u64,
                error_message: None,
            }),
            Ok(_) => Ok(ConnectionHealth {
                healthy: false,
                response_time_ms: response_time.as_millis() as u64,
                error_message: Some("Health check returned unexpected result".to_string()),
            }),
            Err(e) => {
                // Update error stats
                {
                    let mut stats = self.stats.write().await;
                    stats.connection_errors += 1;
                }

                Ok(ConnectionHealth {
                    healthy: false,
                    response_time_ms: response_time.as_millis() as u64,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> ClickHouseStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ClickHouseStats::default();
        info!("ClickHouse connection statistics reset");
    }
}

impl ConnectionStats for ClickHouseConnection {
    fn connection_count(&self) -> u32 {
        self.config.pool_size
    }

    fn active_connections(&self) -> u32 {
        1 // ClickHouse client manages internal connection pooling
    }

    fn idle_connections(&self) -> u32 {
        0
    }
}

/// ClickHouse connection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClickHouseStats {
    pub queries_executed: u64,
    pub total_query_time_ms: u64,
    pub bulk_inserts: u64,
    pub total_rows_inserted: u64,
    pub connection_errors: u64,
}

impl ClickHouseStats {
    pub fn average_query_time_ms(&self) -> f64 {
        if self.queries_executed > 0 {
            self.total_query_time_ms as f64 / self.queries_executed as f64
        } else {
            0.0
        }
    }

    pub fn rows_per_insert(&self) -> f64 {
        if self.bulk_inserts > 0 {
            self.total_rows_inserted as f64 / self.bulk_inserts as f64
        } else {
            0.0
        }
    }
}

/// Table statistics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub table_name: String,
    pub total_rows: u64,
    pub total_bytes: u64,
    pub parts: u64,
    pub active_parts: u64,
}

impl TableStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.total_rows > 0 {
            self.total_bytes as f64 / self.total_rows as f64
        } else {
            0.0
        }
    }

    pub fn fragmentation_ratio(&self) -> f64 {
        if self.parts > 0 {
            self.active_parts as f64 / self.parts as f64
        } else {
            0.0
        }
    }
}

/// Predefined analytics events for common use cases
#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub event_id: String,
    pub workflow_id: String,
    pub user_id: String,
    pub service_name: String,
    pub event_type: String,
    pub event_category: String,
    pub duration_ms: u64,
    pub cost_usd: f64,
    pub success: bool,
    pub error_code: String,
    pub error_message: String,
    pub provider_id: String,
    pub mcp_server_id: String,
    pub request_size: u32,
    pub response_size: u32,
    pub timestamp: String, // ISO format timestamp
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct ApiRequest {
    pub request_id: String,
    pub user_id: String,
    pub api_key_prefix: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub response_time_ms: u32,
    pub request_size: u32,
    pub response_size: u32,
    pub ip_address: String,
    pub user_agent: String,
    pub rate_limit_remaining: u32,
    pub success: bool,
    pub error_type: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct SystemMetric {
    pub metric_id: String,
    pub service_name: String,
    pub metric_name: String,
    pub metric_type: String,
    pub value: f64,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clickhouse_stats() {
        let mut stats = ClickHouseStats::default();
        stats.queries_executed = 10;
        stats.total_query_time_ms = 1000;
        stats.bulk_inserts = 2;
        stats.total_rows_inserted = 20000;

        assert_eq!(stats.average_query_time_ms(), 100.0);
        assert_eq!(stats.rows_per_insert(), 10000.0);
    }

    #[test]
    fn test_table_stats() {
        let stats = TableStats {
            table_name: "test_table".to_string(),
            total_rows: 1000,
            total_bytes: 50000,
            parts: 10,
            active_parts: 8,
        };

        assert_eq!(stats.compression_ratio(), 50.0);
        assert_eq!(stats.fragmentation_ratio(), 0.8);
    }

    #[tokio::test]
    async fn test_clickhouse_config_default() {
        let config = ClickHouseConfig::default();
        assert_eq!(config.database, "automation_analytics");
        assert_eq!(config.pool_size, 10);
        assert!(config.compression);
    }

    #[test]
    fn test_workflow_event_creation() {
        let event = WorkflowEvent {
            event_id: "test-id".to_string(),
            workflow_id: "workflow-123".to_string(),
            user_id: "user-456".to_string(),
            service_name: "test-service".to_string(),
            event_type: "workflow_completed".to_string(),
            event_category: "workflow".to_string(),
            duration_ms: 1500,
            cost_usd: 0.05,
            success: true,
            error_code: "".to_string(),
            error_message: "".to_string(),
            provider_id: "openai".to_string(),
            mcp_server_id: "mcp-1".to_string(),
            request_size: 1024,
            response_size: 2048,
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        };

        assert_eq!(event.workflow_id, "workflow-123");
        assert!(event.success);
    }
}
