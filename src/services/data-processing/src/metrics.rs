//! Metrics collection module for the Data Processing Service
//!
//! This module provides comprehensive metrics collection and reporting including:
//! - Prometheus metrics integration
//! - Custom metrics definitions
//! - Performance monitoring
//! - Health metrics tracking
//! - Resource utilization metrics
//! - Business logic metrics

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{
    config::{Config, MonitoringConfig},
    error::{DataProcessingError, Result},
    types::HealthStatus,
};

/// Main metrics collector for the data processing service
pub struct MetricsCollector {
    registry: Registry,
    config: Arc<MonitoringConfig>,

    // Counter metrics
    records_processed_total: IntCounterVec,
    records_failed_total: IntCounterVec,
    kafka_messages_produced_total: IntCounterVec,
    kafka_messages_consumed_total: IntCounterVec,
    kafka_produce_errors_total: IntCounterVec,
    kafka_consume_errors_total: IntCounterVec,
    batch_jobs_started_total: IntCounterVec,
    batch_jobs_completed_total: IntCounterVec,
    batch_jobs_failed_total: IntCounterVec,
    stream_records_processed_total: IntCounter,
    worker_tasks_processed_total: IntCounterVec,
    checkpoints_created_total: IntCounter,
    watermarks_updated_total: IntCounter,

    // Gauge metrics
    active_batch_jobs: IntGauge,
    active_stream_workers: IntGauge,
    kafka_consumer_lag: IntGaugeVec,
    memory_usage_bytes: IntGauge,
    cpu_usage_percent: Gauge,
    disk_usage_bytes: IntGauge,
    network_connections: IntGauge,
    cache_size_bytes: IntGauge,
    queue_size: IntGaugeVec,

    // Histogram metrics
    processing_duration_seconds: HistogramVec,
    kafka_produce_latency_seconds: HistogramVec,
    kafka_consume_latency_seconds: HistogramVec,
    batch_job_duration_seconds: HistogramVec,
    stream_processing_duration_seconds: Histogram,
    worker_task_duration_seconds: HistogramVec,
    database_query_duration_seconds: HistogramVec,
    http_request_duration_seconds: HistogramVec,

    // Custom metrics registry
    custom_counters: Arc<RwLock<HashMap<String, Counter>>>,
    custom_gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    custom_histograms: Arc<RwLock<HashMap<String, Histogram>>>,

    // Performance tracking
    start_time: SystemTime,
    last_reset_time: Arc<RwLock<SystemTime>>,
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub counters: HashMap<String, f64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, HistogramData>,
    pub custom_metrics: HashMap<String, f64>,
}

/// Histogram data for snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    pub count: u64,
    pub sum: f64,
    pub buckets: Vec<(f64, u64)>,
    pub quantiles: HashMap<String, f64>,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub records_per_second: f64,
    pub avg_processing_time_ms: f64,
    pub error_rate: f64,
    pub throughput_bytes_per_second: f64,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub disk_io_per_second: f64,
    pub network_io_per_second: f64,
}

/// Health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub status: HealthStatus,
    pub components: HashMap<String, ComponentHealthMetrics>,
    pub overall_score: f64,
    pub last_check_timestamp: u64,
}

/// Component-specific health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealthMetrics {
    pub status: HealthStatus,
    pub response_time_ms: f64,
    pub error_count: u64,
    pub success_rate: f64,
    pub last_success_timestamp: Option<u64>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: &Config) -> Result<Self> {
        let registry = Registry::new();
        let monitoring_config = Arc::new(config.monitoring.clone());

        info!("Initializing metrics collector with Prometheus integration");

        // Initialize counter metrics
        let records_processed_total = IntCounterVec::new(
            Opts::new(
                "data_processing_records_processed_total",
                "Total number of records processed",
            ),
            &["source", "status"],
        )?;

        let records_failed_total = IntCounterVec::new(
            Opts::new(
                "data_processing_records_failed_total",
                "Total number of records failed",
            ),
            &["source", "error_type"],
        )?;

        let kafka_messages_produced_total = IntCounterVec::new(
            Opts::new(
                "kafka_messages_produced_total",
                "Total Kafka messages produced",
            ),
            &["topic"],
        )?;

        let kafka_messages_consumed_total = IntCounterVec::new(
            Opts::new(
                "kafka_messages_consumed_total",
                "Total Kafka messages consumed",
            ),
            &["topic"],
        )?;

        let kafka_produce_errors_total = IntCounterVec::new(
            Opts::new("kafka_produce_errors_total", "Total Kafka produce errors"),
            &["topic", "error_type"],
        )?;

        let kafka_consume_errors_total = IntCounterVec::new(
            Opts::new("kafka_consume_errors_total", "Total Kafka consume errors"),
            &["topic", "error_type"],
        )?;

        let batch_jobs_started_total = IntCounterVec::new(
            Opts::new("batch_jobs_started_total", "Total batch jobs started"),
            &["job_type"],
        )?;

        let batch_jobs_completed_total = IntCounterVec::new(
            Opts::new("batch_jobs_completed_total", "Total batch jobs completed"),
            &["job_type", "status"],
        )?;

        let batch_jobs_failed_total = IntCounterVec::new(
            Opts::new("batch_jobs_failed_total", "Total batch jobs failed"),
            &["job_type", "error_type"],
        )?;

        let stream_records_processed_total = IntCounter::new(
            "stream_records_processed_total",
            "Total stream records processed",
        )?;

        let worker_tasks_processed_total = IntCounterVec::new(
            Opts::new(
                "worker_tasks_processed_total",
                "Total worker tasks processed",
            ),
            &["worker"],
        )?;

        let checkpoints_created_total =
            IntCounter::new("checkpoints_created_total", "Total checkpoints created")?;

        let watermarks_updated_total =
            IntCounter::new("watermarks_updated_total", "Total watermarks updated")?;

        // Initialize gauge metrics
        let active_batch_jobs =
            IntGauge::new("active_batch_jobs", "Number of currently active batch jobs")?;

        let active_stream_workers =
            IntGauge::new("active_stream_workers", "Number of active stream workers")?;

        let kafka_consumer_lag = IntGaugeVec::new(
            Opts::new("kafka_consumer_lag", "Kafka consumer lag"),
            &["topic", "partition"],
        )?;

        let memory_usage_bytes =
            IntGauge::new("memory_usage_bytes", "Current memory usage in bytes")?;

        let cpu_usage_percent = Gauge::new("cpu_usage_percent", "Current CPU usage percentage")?;

        let disk_usage_bytes = IntGauge::new("disk_usage_bytes", "Current disk usage in bytes")?;

        let network_connections = IntGauge::new(
            "network_connections",
            "Number of active network connections",
        )?;

        let cache_size_bytes = IntGauge::new("cache_size_bytes", "Current cache size in bytes")?;

        let queue_size = IntGaugeVec::new(
            Opts::new("queue_size", "Current queue size"),
            &["queue_name"],
        )?;

        // Initialize histogram metrics
        let processing_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "processing_duration_seconds",
                "Processing duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
            &["operation"],
        )?;

        let kafka_produce_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "kafka_produce_latency_seconds",
                "Kafka produce latency in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            &["topic"],
        )?;

        let kafka_consume_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "kafka_consume_latency_seconds",
                "Kafka consume latency in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            &["topic"],
        )?;

        let batch_job_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "batch_job_duration_seconds",
                "Batch job duration in seconds",
            )
            .buckets(vec![1.0, 10.0, 60.0, 300.0, 1800.0, 3600.0, 7200.0]),
            &["job_type"],
        )?;

        let stream_processing_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "stream_processing_duration_seconds",
                "Stream processing duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
        )?;

        let worker_task_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "worker_task_duration_seconds",
                "Worker task duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            &["worker"],
        )?;

        let database_query_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "database_query_duration_seconds",
                "Database query duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["database", "operation"],
        )?;

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["method", "endpoint", "status"],
        )?;

        // Register all metrics with Prometheus registry
        registry.register(Box::new(records_processed_total.clone()))?;
        registry.register(Box::new(records_failed_total.clone()))?;
        registry.register(Box::new(kafka_messages_produced_total.clone()))?;
        registry.register(Box::new(kafka_messages_consumed_total.clone()))?;
        registry.register(Box::new(kafka_produce_errors_total.clone()))?;
        registry.register(Box::new(kafka_consume_errors_total.clone()))?;
        registry.register(Box::new(batch_jobs_started_total.clone()))?;
        registry.register(Box::new(batch_jobs_completed_total.clone()))?;
        registry.register(Box::new(batch_jobs_failed_total.clone()))?;
        registry.register(Box::new(stream_records_processed_total.clone()))?;
        registry.register(Box::new(worker_tasks_processed_total.clone()))?;
        registry.register(Box::new(checkpoints_created_total.clone()))?;
        registry.register(Box::new(watermarks_updated_total.clone()))?;

        registry.register(Box::new(active_batch_jobs.clone()))?;
        registry.register(Box::new(active_stream_workers.clone()))?;
        registry.register(Box::new(kafka_consumer_lag.clone()))?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;
        registry.register(Box::new(disk_usage_bytes.clone()))?;
        registry.register(Box::new(network_connections.clone()))?;
        registry.register(Box::new(cache_size_bytes.clone()))?;
        registry.register(Box::new(queue_size.clone()))?;

        registry.register(Box::new(processing_duration_seconds.clone()))?;
        registry.register(Box::new(kafka_produce_latency_seconds.clone()))?;
        registry.register(Box::new(kafka_consume_latency_seconds.clone()))?;
        registry.register(Box::new(batch_job_duration_seconds.clone()))?;
        registry.register(Box::new(stream_processing_duration_seconds.clone()))?;
        registry.register(Box::new(worker_task_duration_seconds.clone()))?;
        registry.register(Box::new(database_query_duration_seconds.clone()))?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;

        let collector = Self {
            registry,
            config: monitoring_config,
            records_processed_total,
            records_failed_total,
            kafka_messages_produced_total,
            kafka_messages_consumed_total,
            kafka_produce_errors_total,
            kafka_consume_errors_total,
            batch_jobs_started_total,
            batch_jobs_completed_total,
            batch_jobs_failed_total,
            stream_records_processed_total,
            worker_tasks_processed_total,
            checkpoints_created_total,
            watermarks_updated_total,
            active_batch_jobs,
            active_stream_workers,
            kafka_consumer_lag,
            memory_usage_bytes,
            cpu_usage_percent,
            disk_usage_bytes,
            network_connections,
            cache_size_bytes,
            queue_size,
            processing_duration_seconds,
            kafka_produce_latency_seconds,
            kafka_consume_latency_seconds,
            batch_job_duration_seconds,
            stream_processing_duration_seconds,
            worker_task_duration_seconds,
            database_query_duration_seconds,
            http_request_duration_seconds,
            custom_counters: Arc::new(RwLock::new(HashMap::new())),
            custom_gauges: Arc::new(RwLock::new(HashMap::new())),
            custom_histograms: Arc::new(RwLock::new(HashMap::new())),
            start_time: SystemTime::now(),
            last_reset_time: Arc::new(RwLock::new(SystemTime::now())),
        };

        info!("Metrics collector initialized successfully");
        Ok(collector)
    }

    /// Get Prometheus registry for HTTP endpoint
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Increment a counter metric
    pub fn increment_counter(&self, name: &str, labels: &[(&str, &str)]) {
        match name {
            "kafka_messages_produced_total" => {
                if let Some(topic) = labels.iter().find(|(k, _)| *k == "topic").map(|(_, v)| *v) {
                    self.kafka_messages_produced_total
                        .with_label_values(&[topic])
                        .inc();
                }
            }
            "kafka_messages_consumed_total" => {
                if let Some(topic) = labels.iter().find(|(k, _)| *k == "topic").map(|(_, v)| *v) {
                    self.kafka_messages_consumed_total
                        .with_label_values(&[topic])
                        .inc();
                }
            }
            "kafka_produce_errors_total" => {
                if let Some(topic) = labels.iter().find(|(k, _)| *k == "topic").map(|(_, v)| *v) {
                    let error_type = labels
                        .iter()
                        .find(|(k, _)| *k == "error_type")
                        .map(|(_, v)| *v)
                        .unwrap_or("unknown");
                    self.kafka_produce_errors_total
                        .with_label_values(&[topic, error_type])
                        .inc();
                }
            }
            "kafka_consume_errors_total" => {
                let error_type = labels
                    .iter()
                    .find(|(k, _)| *k == "error_type")
                    .map(|(_, v)| *v)
                    .unwrap_or("unknown");
                self.kafka_consume_errors_total
                    .with_label_values(&[error_type])
                    .inc();
            }
            "stream_records_processed_total" => {
                self.stream_records_processed_total.inc();
            }
            "worker_tasks_processed_total" => {
                if let Some(worker) = labels.iter().find(|(k, _)| *k == "worker").map(|(_, v)| *v) {
                    self.worker_tasks_processed_total
                        .with_label_values(&[worker])
                        .inc();
                }
            }
            "checkpoints_created_total" => {
                self.checkpoints_created_total.inc();
            }
            "watermarks_updated_total" => {
                self.watermarks_updated_total.inc();
            }
            _ => {
                debug!("Unknown counter metric: {}", name);
            }
        }
    }

    /// Set a gauge metric value
    pub fn set_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        match name {
            "active_batch_jobs" => {
                self.active_batch_jobs.set(value as i64);
            }
            "active_stream_workers" => {
                self.active_stream_workers.set(value as i64);
            }
            "memory_usage_bytes" => {
                self.memory_usage_bytes.set(value as i64);
            }
            "cpu_usage_percent" => {
                self.cpu_usage_percent.set(value);
            }
            "disk_usage_bytes" => {
                self.disk_usage_bytes.set(value as i64);
            }
            "network_connections" => {
                self.network_connections.set(value as i64);
            }
            "cache_size_bytes" => {
                self.cache_size_bytes.set(value as i64);
            }
            "queue_size" => {
                if let Some(queue_name) = labels
                    .iter()
                    .find(|(k, _)| *k == "queue_name")
                    .map(|(_, v)| *v)
                {
                    self.queue_size
                        .with_label_values(&[queue_name])
                        .set(value as i64);
                }
            }
            _ => {
                debug!("Unknown gauge metric: {}", name);
            }
        }
    }

    /// Record a histogram observation
    pub fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        match name {
            "kafka_produce_latency_seconds" => {
                if let Some(topic) = labels.iter().find(|(k, _)| *k == "topic").map(|(_, v)| *v) {
                    self.kafka_produce_latency_seconds
                        .with_label_values(&[topic])
                        .observe(value);
                }
            }
            "kafka_consume_latency_seconds" => {
                if let Some(topic) = labels.iter().find(|(k, _)| *k == "topic").map(|(_, v)| *v) {
                    self.kafka_consume_latency_seconds
                        .with_label_values(&[topic])
                        .observe(value);
                }
            }
            "stream_processing_duration_seconds" => {
                self.stream_processing_duration_seconds.observe(value);
            }
            "worker_task_duration_seconds" => {
                if let Some(worker) = labels.iter().find(|(k, _)| *k == "worker").map(|(_, v)| *v) {
                    self.worker_task_duration_seconds
                        .with_label_values(&[worker])
                        .observe(value);
                }
            }
            "database_query_duration_seconds" => {
                let database = labels
                    .iter()
                    .find(|(k, _)| *k == "database")
                    .map(|(_, v)| *v)
                    .unwrap_or("unknown");
                let operation = labels
                    .iter()
                    .find(|(k, _)| *k == "operation")
                    .map(|(_, v)| *v)
                    .unwrap_or("unknown");
                self.database_query_duration_seconds
                    .with_label_values(&[database, operation])
                    .observe(value);
            }
            "http_request_duration_seconds" => {
                let method = labels
                    .iter()
                    .find(|(k, _)| *k == "method")
                    .map(|(_, v)| *v)
                    .unwrap_or("unknown");
                let endpoint = labels
                    .iter()
                    .find(|(k, _)| *k == "endpoint")
                    .map(|(_, v)| *v)
                    .unwrap_or("unknown");
                let status = labels
                    .iter()
                    .find(|(k, _)| *k == "status")
                    .map(|(_, v)| *v)
                    .unwrap_or("unknown");
                self.http_request_duration_seconds
                    .with_label_values(&[method, endpoint, status])
                    .observe(value);
            }
            _ => {
                debug!("Unknown histogram metric: {}", name);
            }
        }
    }

    /// Create a custom counter metric
    pub async fn create_custom_counter(&self, name: String, description: String) -> Result<()> {
        let counter = Counter::new(name.clone(), description)?;
        self.registry.register(Box::new(counter.clone()))?;

        let mut custom_counters = self.custom_counters.write().await;
        custom_counters.insert(name.clone(), counter);

        info!("Created custom counter metric: {}", name);
        Ok(())
    }

    /// Create a custom gauge metric
    pub async fn create_custom_gauge(&self, name: String, description: String) -> Result<()> {
        let gauge = Gauge::new(name.clone(), description)?;
        self.registry.register(Box::new(gauge.clone()))?;

        let mut custom_gauges = self.custom_gauges.write().await;
        custom_gauges.insert(name.clone(), gauge);

        info!("Created custom gauge metric: {}", name);
        Ok(())
    }

    /// Create a custom histogram metric
    pub async fn create_custom_histogram(
        &self,
        name: String,
        description: String,
        buckets: Vec<f64>,
    ) -> Result<()> {
        let histogram =
            Histogram::with_opts(HistogramOpts::new(name.clone(), description).buckets(buckets))?;
        self.registry.register(Box::new(histogram.clone()))?;

        let mut custom_histograms = self.custom_histograms.write().await;
        custom_histograms.insert(name.clone(), histogram);

        info!("Created custom histogram metric: {}", name);
        Ok(())
    }

    /// Increment a custom counter
    pub async fn increment_custom_counter(&self, name: &str, amount: f64) -> Result<()> {
        let custom_counters = self.custom_counters.read().await;
        if let Some(counter) = custom_counters.get(name) {
            counter.inc_by(amount);
        } else {
            warn!("Custom counter '{}' not found", name);
        }
        Ok(())
    }

    /// Set a custom gauge value
    pub async fn set_custom_gauge(&self, name: &str, value: f64) -> Result<()> {
        let custom_gauges = self.custom_gauges.read().await;
        if let Some(gauge) = custom_gauges.get(name) {
            gauge.set(value);
        } else {
            warn!("Custom gauge '{}' not found", name);
        }
        Ok(())
    }

    /// Record a custom histogram observation
    pub async fn record_custom_histogram(&self, name: &str, value: f64) -> Result<()> {
        let custom_histograms = self.custom_histograms.read().await;
        if let Some(histogram) = custom_histograms.get(name) {
            histogram.observe(value);
        } else {
            warn!("Custom histogram '{}' not found", name);
        }
        Ok(())
    }

    /// Get current metrics snapshot
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let uptime = SystemTime::now()
            .duration_since(self.start_time)
            .unwrap()
            .as_secs();

        // Collect metrics from Prometheus registry
        let metric_families = self.registry.gather();
        let mut counters = HashMap::new();
        let mut gauges = HashMap::new();
        let mut histograms = HashMap::new();

        for mf in metric_families {
            let name = mf.get_name();
            for metric in mf.get_metric() {
                match mf.get_field_type() {
                    prometheus::proto::MetricType::COUNTER => {
                        counters.insert(name.to_string(), metric.get_counter().get_value());
                    }
                    prometheus::proto::MetricType::GAUGE => {
                        gauges.insert(name.to_string(), metric.get_gauge().get_value());
                    }
                    prometheus::proto::MetricType::HISTOGRAM => {
                        let hist = metric.get_histogram();
                        let mut buckets = Vec::new();
                        for bucket in hist.get_bucket() {
                            buckets.push((bucket.get_upper_bound(), bucket.get_cumulative_count()));
                        }
                        let hist_data = HistogramData {
                            count: hist.get_sample_count(),
                            sum: hist.get_sample_sum(),
                            buckets,
                            quantiles: HashMap::new(), // TODO: calculate quantiles
                        };
                        histograms.insert(name.to_string(), hist_data);
                    }
                    _ => {}
                }
            }
        }

        MetricsSnapshot {
            timestamp: now,
            uptime_seconds: uptime,
            counters,
            gauges,
            histograms,
            custom_metrics: HashMap::new(), // TODO: collect custom metrics
        }
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let snapshot = self.get_snapshot().await;

        // Calculate performance metrics based on snapshot
        let records_per_second = snapshot
            .counters
            .get("stream_records_processed_total")
            .unwrap_or(&0.0)
            / snapshot.uptime_seconds as f64;

        let avg_processing_time_ms = snapshot
            .histograms
            .get("stream_processing_duration_seconds")
            .map(|h| (h.sum / h.count as f64) * 1000.0)
            .unwrap_or(0.0);

        let total_errors = snapshot
            .counters
            .get("kafka_produce_errors_total")
            .unwrap_or(&0.0)
            + snapshot
                .counters
                .get("kafka_consume_errors_total")
                .unwrap_or(&0.0);
        let total_operations = snapshot
            .counters
            .get("stream_records_processed_total")
            .unwrap_or(&0.0);
        let error_rate = if *total_operations > 0.0 {
            total_errors / *total_operations
        } else {
            0.0
        };

        PerformanceStats {
            records_per_second,
            avg_processing_time_ms,
            error_rate,
            throughput_bytes_per_second: 0.0, // TODO: implement
            cpu_utilization: snapshot
                .gauges
                .get("cpu_usage_percent")
                .copied()
                .unwrap_or(0.0),
            memory_utilization: snapshot
                .gauges
                .get("memory_usage_bytes")
                .copied()
                .unwrap_or(0.0),
            disk_io_per_second: 0.0,    // TODO: implement
            network_io_per_second: 0.0, // TODO: implement
        }
    }

    /// Reset all metrics
    pub async fn reset_metrics(&self) -> Result<()> {
        info!("Resetting all metrics");

        // Reset timestamp
        let mut last_reset = self.last_reset_time.write().await;
        *last_reset = SystemTime::now();

        // Note: Prometheus metrics cannot be reset directly
        // In a production system, you would typically restart the service
        // or use metric labels with timestamps to create new metric series

        Ok(())
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        use prometheus::TextEncoder;
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode_to_string(&metric_families)
            .unwrap_or_else(|e| {
                error!("Failed to encode metrics: {}", e);
                String::new()
            })
    }
}

impl From<prometheus::Error> for DataProcessingError {
    fn from(err: prometheus::Error) -> Self {
        DataProcessingError::Metrics {
            message: format!("Prometheus error: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_metrics_collector_creation() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config);
        assert!(collector.is_ok());
    }

    #[test]
    fn test_counter_increment() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).unwrap();

        // This should not panic
        collector.increment_counter("stream_records_processed_total", &[]);
    }

    #[test]
    fn test_gauge_set() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).unwrap();

        // This should not panic
        collector.set_gauge("active_batch_jobs", 5.0, &[]);
    }

    #[test]
    fn test_histogram_record() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).unwrap();

        // This should not panic
        collector.record_histogram("stream_processing_duration_seconds", 0.1, &[]);
    }

    #[tokio::test]
    async fn test_custom_metrics() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).unwrap();

        // Create custom metrics
        let result = collector
            .create_custom_counter(
                "test_counter".to_string(),
                "Test counter metric".to_string(),
            )
            .await;
        assert!(result.is_ok());

        let result = collector
            .create_custom_gauge("test_gauge".to_string(), "Test gauge metric".to_string())
            .await;
        assert!(result.is_ok());

        // Use custom metrics
        let result = collector
            .increment_custom_counter("test_counter", 1.0)
            .await;
        assert!(result.is_ok());

        let result = collector.set_custom_gauge("test_gauge", 42.0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_snapshot() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).unwrap();

        // Generate some metrics
        collector.increment_counter("stream_records_processed_total", &[]);
        collector.set_gauge("active_batch_jobs", 3.0, &[]);
        collector.record_histogram("stream_processing_duration_seconds", 0.05, &[]);

        let snapshot = collector.get_snapshot().await;
        assert!(snapshot.uptime_seconds > 0);
        assert!(snapshot.timestamp > 0);
    }

    #[test]
    fn test_prometheus_export() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).unwrap();

        // Generate some metrics
        collector.increment_counter("stream_records_processed_total", &[]);

        let export = collector.export_prometheus();
        assert!(!export.is_empty());
        assert!(export.contains("stream_records_processed_total"));
    }
}
