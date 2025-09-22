//! # Metrics Collection Module
//!
//! This module provides comprehensive metrics collection for the event streaming service.
//! It handles Prometheus metrics, custom metrics, and performance monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use prometheus::{
    histogram_opts, opts, register_counter_with_registry, register_gauge_with_registry,
    register_histogram_with_registry, register_int_counter_with_registry,
    register_int_gauge_with_registry, Counter, Encoder, Gauge, Histogram, IntCounter, IntGauge,
    Registry, TextEncoder,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{EventStreamingError, Result},
    types::{ComponentHealth, EventCategory, EventPriority, MetricsSnapshot, ProcessingStats},
};

/// Metrics collector for the event streaming service
#[derive(Clone)]
pub struct MetricsCollector {
    config: Arc<Config>,
    registry: Arc<Registry>,

    // Event metrics
    events_published_total: IntCounter,
    events_processed_total: IntCounter,
    events_failed_total: IntCounter,
    events_filtered_total: IntCounter,
    events_dead_letter_total: IntCounter,

    // Processing metrics
    processing_duration_seconds: Histogram,
    processing_queue_size: IntGauge,
    processing_active_workers: IntGauge,

    // Kafka metrics
    kafka_publish_success_total: IntCounter,
    kafka_publish_error_total: IntCounter,
    kafka_publish_duration_seconds: Histogram,
    kafka_consumer_lag: IntGauge,

    // Redis metrics
    redis_publish_success_total: IntCounter,
    redis_publish_error_total: IntCounter,
    redis_publish_duration_seconds: Histogram,
    redis_stream_length: IntGauge,

    // Storage metrics
    storage_operations_total: IntCounter,
    storage_operation_duration_seconds: Histogram,
    storage_connection_pool_size: IntGauge,
    storage_connection_pool_active: IntGauge,

    // System metrics
    memory_usage_bytes: Gauge,
    cpu_usage_percent: Gauge,
    goroutines_total: IntGauge,
    gc_duration_seconds: Histogram,

    // Health metrics
    health_check_success_total: IntCounter,
    health_check_failure_total: IntCounter,
    health_check_duration_seconds: Histogram,
    component_health_status: IntGauge,

    // Replay metrics
    replay_jobs_total: IntGauge,
    replay_jobs_active: IntGauge,
    replay_events_processed_total: IntCounter,

    // Custom metrics storage
    custom_metrics: Arc<RwLock<HashMap<String, CustomMetric>>>,
}

/// Custom metric types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustomMetric {
    Counter(f64),
    Gauge(f64),
    Histogram {
        sum: f64,
        count: u64,
        buckets: Vec<(f64, u64)>,
    },
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Initializing Metrics Collector");

        let registry = Registry::new();

        // Create event metrics
        let events_published_total = register_int_counter_with_registry!(
            opts!("events_published_total", "Total number of events published"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let events_processed_total = register_int_counter_with_registry!(
            opts!(
                "events_processed_total",
                "Total number of events processed successfully"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let events_failed_total = register_int_counter_with_registry!(
            opts!(
                "events_failed_total",
                "Total number of events that failed processing"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let events_filtered_total = register_int_counter_with_registry!(
            opts!(
                "events_filtered_total",
                "Total number of events filtered out"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let events_dead_letter_total = register_int_counter_with_registry!(
            opts!(
                "events_dead_letter_total",
                "Total number of events sent to dead letter queue"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create processing metrics
        let processing_duration_seconds = register_histogram_with_registry!(
            histogram_opts!(
                "processing_duration_seconds",
                "Time spent processing events",
                vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let processing_queue_size = register_int_gauge_with_registry!(
            opts!(
                "processing_queue_size",
                "Current size of the processing queue"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let processing_active_workers = register_int_gauge_with_registry!(
            opts!(
                "processing_active_workers",
                "Number of active worker threads"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create Kafka metrics
        let kafka_publish_success_total = register_int_counter_with_registry!(
            opts!(
                "kafka_publish_success_total",
                "Total number of successful Kafka publishes"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let kafka_publish_error_total = register_int_counter_with_registry!(
            opts!(
                "kafka_publish_error_total",
                "Total number of failed Kafka publishes"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let kafka_publish_duration_seconds = register_histogram_with_registry!(
            histogram_opts!(
                "kafka_publish_duration_seconds",
                "Time spent publishing to Kafka",
                vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let kafka_consumer_lag = register_int_gauge_with_registry!(
            opts!("kafka_consumer_lag", "Current Kafka consumer lag"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create Redis metrics
        let redis_publish_success_total = register_int_counter_with_registry!(
            opts!(
                "redis_publish_success_total",
                "Total number of successful Redis publishes"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let redis_publish_error_total = register_int_counter_with_registry!(
            opts!(
                "redis_publish_error_total",
                "Total number of failed Redis publishes"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let redis_publish_duration_seconds = register_histogram_with_registry!(
            histogram_opts!(
                "redis_publish_duration_seconds",
                "Time spent publishing to Redis",
                vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let redis_stream_length = register_int_gauge_with_registry!(
            opts!("redis_stream_length", "Current Redis stream length"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create storage metrics
        let storage_operations_total = register_int_counter_with_registry!(
            opts!(
                "storage_operations_total",
                "Total number of storage operations"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let storage_operation_duration_seconds = register_histogram_with_registry!(
            histogram_opts!(
                "storage_operation_duration_seconds",
                "Time spent on storage operations",
                vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let storage_connection_pool_size = register_int_gauge_with_registry!(
            opts!(
                "storage_connection_pool_size",
                "Size of the storage connection pool"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let storage_connection_pool_active = register_int_gauge_with_registry!(
            opts!(
                "storage_connection_pool_active",
                "Active connections in the storage pool"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create system metrics
        let memory_usage_bytes = register_gauge_with_registry!(
            opts!("memory_usage_bytes", "Current memory usage in bytes"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let cpu_usage_percent = register_gauge_with_registry!(
            opts!("cpu_usage_percent", "Current CPU usage percentage"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let goroutines_total = register_int_gauge_with_registry!(
            opts!("goroutines_total", "Total number of goroutines"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let gc_duration_seconds = register_histogram_with_registry!(
            histogram_opts!(
                "gc_duration_seconds",
                "Time spent in garbage collection",
                vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5]
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create health metrics
        let health_check_success_total = register_int_counter_with_registry!(
            opts!(
                "health_check_success_total",
                "Total number of successful health checks"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let health_check_failure_total = register_int_counter_with_registry!(
            opts!(
                "health_check_failure_total",
                "Total number of failed health checks"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let health_check_duration_seconds = register_histogram_with_registry!(
            histogram_opts!(
                "health_check_duration_seconds",
                "Time spent on health checks",
                vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let component_health_status = register_int_gauge_with_registry!(
            opts!(
                "component_health_status",
                "Health status of components (1=healthy, 0=unhealthy)"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        // Create replay metrics
        let replay_jobs_total = register_int_gauge_with_registry!(
            opts!("replay_jobs_total", "Total number of replay jobs"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let replay_jobs_active = register_int_gauge_with_registry!(
            opts!("replay_jobs_active", "Number of active replay jobs"),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        let replay_events_processed_total = register_int_counter_with_registry!(
            opts!(
                "replay_events_processed_total",
                "Total number of events processed during replay"
            ),
            &registry
        )
        .map_err(|e| EventStreamingError::internal(format!("Failed to register metric: {}", e)))?;

        Ok(Self {
            config: Arc::new(config.clone()),
            registry: Arc::new(registry),
            events_published_total,
            events_processed_total,
            events_failed_total,
            events_filtered_total,
            events_dead_letter_total,
            processing_duration_seconds,
            processing_queue_size,
            processing_active_workers,
            kafka_publish_success_total,
            kafka_publish_error_total,
            kafka_publish_duration_seconds,
            kafka_consumer_lag,
            redis_publish_success_total,
            redis_publish_error_total,
            redis_publish_duration_seconds,
            redis_stream_length,
            storage_operations_total,
            storage_operation_duration_seconds,
            storage_connection_pool_size,
            storage_connection_pool_active,
            memory_usage_bytes,
            cpu_usage_percent,
            goroutines_total,
            gc_duration_seconds,
            health_check_success_total,
            health_check_failure_total,
            health_check_duration_seconds,
            component_health_status,
            replay_jobs_total,
            replay_jobs_active,
            replay_events_processed_total,
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Record event published
    pub async fn record_event_published(&self, duration: Duration) -> Result<()> {
        self.events_published_total.inc();
        self.processing_duration_seconds
            .observe(duration.as_secs_f64());
        Ok(())
    }

    /// Record event processed
    pub async fn record_event_processed(
        &self,
        _event_id: Uuid,
        duration: Duration,
        success: bool,
    ) -> Result<()> {
        if success {
            self.events_processed_total.inc();
        } else {
            self.events_failed_total.inc();
        }
        self.processing_duration_seconds
            .observe(duration.as_secs_f64());
        Ok(())
    }

    /// Record Kafka publish success
    pub async fn record_kafka_publish_success(
        &self,
        _topic: &str,
        duration: Duration,
    ) -> Result<()> {
        self.kafka_publish_success_total.inc();
        self.kafka_publish_duration_seconds
            .observe(duration.as_secs_f64());
        Ok(())
    }

    /// Record Kafka publish error
    pub async fn record_kafka_publish_error(
        &self,
        _topic: &str,
        duration: Duration,
        _error: &str,
    ) -> Result<()> {
        self.kafka_publish_error_total.inc();
        self.kafka_publish_duration_seconds
            .observe(duration.as_secs_f64());
        Ok(())
    }

    /// Record Redis publish success
    pub async fn record_redis_publish_success(
        &self,
        _stream: &str,
        duration: Duration,
    ) -> Result<()> {
        self.redis_publish_success_total.inc();
        self.redis_publish_duration_seconds
            .observe(duration.as_secs_f64());
        Ok(())
    }

    /// Record Redis read success
    pub async fn record_redis_read_success(
        &self,
        _stream: &str,
        _count: usize,
        duration: Duration,
    ) -> Result<()> {
        self.redis_publish_duration_seconds
            .observe(duration.as_secs_f64());
        Ok(())
    }

    /// Record health check
    pub async fn record_health_check(&self, healths: &[ComponentHealth]) -> Result<()> {
        for health in healths {
            match health.status {
                crate::types::HealthStatus::Healthy => {
                    self.health_check_success_total.inc();
                    self.component_health_status.set(1);
                }
                _ => {
                    self.health_check_failure_total.inc();
                    self.component_health_status.set(0);
                }
            }
            self.health_check_duration_seconds
                .observe(health.response_time_ms as f64 / 1000.0);
        }
        Ok(())
    }

    /// Record processing stats
    pub async fn record_processing_stats(&self, stats: &ProcessingStats) -> Result<()> {
        self.events_processed_total.inc_by(stats.total_processed);
        self.events_failed_total.inc_by(stats.total_failed);
        self.events_filtered_total.inc_by(stats.total_filtered);
        self.events_dead_letter_total
            .inc_by(stats.total_dead_letter);
        Ok(())
    }

    /// Record replay jobs
    pub async fn record_replay_jobs(&self, total: usize, active: usize) -> Result<()> {
        self.replay_jobs_total.set(total as i64);
        self.replay_jobs_active.set(active as i64);
        Ok(())
    }

    /// Get metrics snapshot
    pub async fn get_snapshot(&self) -> Result<MetricsSnapshot> {
        Ok(MetricsSnapshot {
            events_processed: self.events_processed_total.get(),
            events_per_second: 0.0,      // Would be calculated from rate
            avg_processing_time_ms: 0.0, // Would be calculated from histogram
            error_rate: 0.0,             // Would be calculated from counters
            memory_usage_bytes: self.memory_usage_bytes.get() as u64,
            cpu_usage_percent: self.cpu_usage_percent.get(),
            active_connections: 0, // Would be from connection pool metrics
            timestamp: chrono::Utc::now(),
        })
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();

        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).map_err(|e| {
            EventStreamingError::internal(format!("Failed to encode metrics: {}", e))
        })?;

        String::from_utf8(buffer).map_err(|e| {
            EventStreamingError::internal(format!("Failed to convert metrics to string: {}", e))
        })
    }

    /// Record custom metric
    pub async fn record_custom_metric(&self, name: String, metric: CustomMetric) -> Result<()> {
        let mut custom_metrics = self.custom_metrics.write().await;
        custom_metrics.insert(name, metric);
        Ok(())
    }

    /// Get custom metric
    pub async fn get_custom_metric(&self, name: &str) -> Option<CustomMetric> {
        let custom_metrics = self.custom_metrics.read().await;
        custom_metrics.get(name).cloned()
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self) -> Result<()> {
        // Get system information
        let memory_usage = self.get_memory_usage().await?;
        let cpu_usage = self.get_cpu_usage().await?;

        self.memory_usage_bytes.set(memory_usage);
        self.cpu_usage_percent.set(cpu_usage);

        Ok(())
    }

    /// Get current memory usage
    async fn get_memory_usage(&self) -> Result<f64> {
        // This would typically use system APIs to get actual memory usage
        // For now, return a placeholder value
        Ok(1024.0 * 1024.0 * 100.0) // 100MB
    }

    /// Get current CPU usage
    async fn get_cpu_usage(&self) -> Result<f64> {
        // This would typically use system APIs to get actual CPU usage
        // For now, return a placeholder value
        Ok(15.5) // 15.5%
    }

    /// Set processing queue size
    pub fn set_processing_queue_size(&self, size: i64) {
        self.processing_queue_size.set(size);
    }

    /// Set active workers
    pub fn set_active_workers(&self, count: i64) {
        self.processing_active_workers.set(count);
    }

    /// Set Kafka consumer lag
    pub fn set_kafka_consumer_lag(&self, lag: i64) {
        self.kafka_consumer_lag.set(lag);
    }

    /// Set Redis stream length
    pub fn set_redis_stream_length(&self, length: i64) {
        self.redis_stream_length.set(length);
    }

    /// Set storage connection pool metrics
    pub fn set_storage_pool_metrics(&self, size: i64, active: i64) {
        self.storage_connection_pool_size.set(size);
        self.storage_connection_pool_active.set(active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = Config::default();
        let result = MetricsCollector::new(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).await.unwrap();

        let duration = Duration::from_millis(100);
        let event_id = Uuid::new_v4();

        // Test event metrics
        let result = collector.record_event_published(duration).await;
        assert!(result.is_ok());

        let result = collector
            .record_event_processed(event_id, duration, true)
            .await;
        assert!(result.is_ok());

        // Test Kafka metrics
        let result = collector
            .record_kafka_publish_success("test-topic", duration)
            .await;
        assert!(result.is_ok());

        // Test Redis metrics
        let result = collector
            .record_redis_publish_success("test-stream", duration)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_prometheus_export() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).await.unwrap();

        let result = collector.export_prometheus().await;
        assert!(result.is_ok());

        let metrics_text = result.unwrap();
        assert!(!metrics_text.is_empty());
        assert!(metrics_text.contains("events_published_total"));
    }

    #[tokio::test]
    async fn test_custom_metrics() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).await.unwrap();

        let custom_metric = CustomMetric::Counter(42.0);
        let result = collector
            .record_custom_metric("test_counter".to_string(), custom_metric.clone())
            .await;
        assert!(result.is_ok());

        let retrieved = collector.get_custom_metric("test_counter").await;
        assert!(retrieved.is_some());

        match retrieved.unwrap() {
            CustomMetric::Counter(value) => assert_eq!(value, 42.0),
            _ => panic!("Expected counter metric"),
        }
    }

    #[tokio::test]
    async fn test_metrics_snapshot() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).await.unwrap();

        let snapshot = collector.get_snapshot().await.unwrap();
        assert_eq!(snapshot.events_processed, 0);
        assert!(snapshot.timestamp <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_gauge_metrics() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).await.unwrap();

        collector.set_processing_queue_size(100);
        collector.set_active_workers(8);
        collector.set_kafka_consumer_lag(50);
        collector.set_redis_stream_length(1000);
        collector.set_storage_pool_metrics(20, 15);

        // Verify metrics were set (in a real test, we'd export and check)
        let metrics = collector.export_prometheus().await.unwrap();
        assert!(metrics.contains("processing_queue_size"));
        assert!(metrics.contains("processing_active_workers"));
    }
}
