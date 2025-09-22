// AI-CORE Test Data API Metrics Service
// Comprehensive metrics collection and monitoring for test data operations
// Backend Agent Implementation - T2.2

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

// ============================================================================
// Metrics Service - Performance and operational metrics collection
// ============================================================================

pub struct MetricsService {
    counters: Arc<RwLock<HashMap<String, AtomicU64>>>,
    gauges: Arc<RwLock<HashMap<String, AtomicI64>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    timers: Arc<RwLock<HashMap<String, Timer>>>,
    custom_metrics: Arc<Mutex<HashMap<String, CustomMetric>>>,
    start_time: Instant,
    system_start_time: SystemTime,
}

#[derive(Debug, Clone)]
struct Histogram {
    buckets: Vec<(f64, AtomicU64)>, // (upper_bound, count)
    sum: AtomicU64,
    count: AtomicU64,
}

#[derive(Debug, Clone)]
struct Timer {
    total_duration_ms: AtomicU64,
    count: AtomicU64,
    min_duration_ms: AtomicU64,
    max_duration_ms: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomMetric {
    name: String,
    value: f64,
    labels: HashMap<String, String>,
    timestamp: DateTime<Utc>,
    unit: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, i64>,
    pub histograms: HashMap<String, HistogramSnapshot>,
    pub timers: HashMap<String, TimerSnapshot>,
    pub custom_metrics: Vec<CustomMetric>,
    pub system_metrics: SystemMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramSnapshot {
    pub buckets: Vec<(f64, u64)>,
    pub sum: u64,
    pub count: u64,
    pub average: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSnapshot {
    pub count: u64,
    pub total_duration_ms: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_bytes_in: u64,
    pub network_bytes_out: u64,
    pub open_file_descriptors: u64,
    pub thread_count: u64,
    pub gc_count: u64,
    pub gc_time_ms: u64,
}

impl MetricsService {
    pub async fn new() -> Result<Self> {
        info!("Initializing MetricsService");

        let service = Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            timers: Arc::new(RwLock::new(HashMap::new())),
            custom_metrics: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
            system_start_time: SystemTime::now(),
        };

        // Initialize default metrics
        service.initialize_default_metrics().await?;

        info!("MetricsService initialized successfully");
        Ok(service)
    }

    // ========================================================================
    // Counter Operations
    // ========================================================================

    pub async fn increment_counter(&self, name: &str) -> Result<()> {
        self.add_to_counter(name, 1).await
    }

    pub async fn add_to_counter(&self, name: &str, value: u64) -> Result<()> {
        let counters = self.counters.read().unwrap();

        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(counters);
            let mut counters = self.counters.write().unwrap();
            let counter = AtomicU64::new(value);
            counters.insert(name.to_string(), counter);
        }

        debug!("Counter '{}' incremented by {}", name, value);
        Ok(())
    }

    pub async fn get_counter(&self, name: &str) -> Result<u64> {
        let counters = self.counters.read().unwrap();
        if let Some(counter) = counters.get(name) {
            Ok(counter.load(Ordering::Relaxed))
        } else {
            Ok(0)
        }
    }

    pub async fn reset_counter(&self, name: &str) -> Result<()> {
        let counters = self.counters.read().unwrap();
        if let Some(counter) = counters.get(name) {
            counter.store(0, Ordering::Relaxed);
            debug!("Counter '{}' reset to 0", name);
        }
        Ok(())
    }

    // ========================================================================
    // Gauge Operations
    // ========================================================================

    pub async fn set_gauge(&self, name: &str, value: i64) -> Result<()> {
        let gauges = self.gauges.read().unwrap();

        if let Some(gauge) = gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        } else {
            drop(gauges);
            let mut gauges = self.gauges.write().unwrap();
            let gauge = AtomicI64::new(value);
            gauges.insert(name.to_string(), gauge);
        }

        debug!("Gauge '{}' set to {}", name, value);
        Ok(())
    }

    pub async fn increment_gauge(&self, name: &str) -> Result<()> {
        self.add_to_gauge(name, 1).await
    }

    pub async fn decrement_gauge(&self, name: &str) -> Result<()> {
        self.add_to_gauge(name, -1).await
    }

    pub async fn add_to_gauge(&self, name: &str, value: i64) -> Result<()> {
        let gauges = self.gauges.read().unwrap();

        if let Some(gauge) = gauges.get(name) {
            gauge.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(gauges);
            let mut gauges = self.gauges.write().unwrap();
            let gauge = AtomicI64::new(value);
            gauges.insert(name.to_string(), gauge);
        }

        debug!("Gauge '{}' modified by {}", name, value);
        Ok(())
    }

    pub async fn get_gauge(&self, name: &str) -> Result<i64> {
        let gauges = self.gauges.read().unwrap();
        if let Some(gauge) = gauges.get(name) {
            Ok(gauge.load(Ordering::Relaxed))
        } else {
            Ok(0)
        }
    }

    // ========================================================================
    // Histogram Operations
    // ========================================================================

    pub async fn record_histogram(&self, name: &str, value: f64) -> Result<()> {
        let histograms = self.histograms.read().unwrap();

        if let Some(histogram) = histograms.get(name) {
            histogram.record(value);
        } else {
            drop(histograms);
            let mut histograms = self.histograms.write().unwrap();
            let histogram = Histogram::new();
            histogram.record(value);
            histograms.insert(name.to_string(), histogram);
        }

        debug!("Histogram '{}' recorded value: {}", name, value);
        Ok(())
    }

    pub async fn get_histogram_snapshot(&self, name: &str) -> Result<Option<HistogramSnapshot>> {
        let histograms = self.histograms.read().unwrap();
        if let Some(histogram) = histograms.get(name) {
            Ok(Some(histogram.snapshot()))
        } else {
            Ok(None)
        }
    }

    // ========================================================================
    // Timer Operations
    // ========================================================================

    pub async fn start_timer(&self, name: &str) -> TimerHandle {
        TimerHandle {
            name: name.to_string(),
            start_time: Instant::now(),
            service: Arc::new(self.clone()),
        }
    }

    pub async fn record_timer(&self, name: &str, duration_ms: u64) -> Result<()> {
        let timers = self.timers.read().unwrap();

        if let Some(timer) = timers.get(name) {
            timer.record(duration_ms);
        } else {
            drop(timers);
            let mut timers = self.timers.write().unwrap();
            let timer = Timer::new();
            timer.record(duration_ms);
            timers.insert(name.to_string(), timer);
        }

        debug!("Timer '{}' recorded duration: {}ms", name, duration_ms);
        Ok(())
    }

    pub async fn get_timer_snapshot(&self, name: &str) -> Result<Option<TimerSnapshot>> {
        let timers = self.timers.read().unwrap();
        if let Some(timer) = timers.get(name) {
            Ok(Some(timer.snapshot()))
        } else {
            Ok(None)
        }
    }

    // ========================================================================
    // Custom Metrics Operations
    // ========================================================================

    pub async fn record_custom_metric(
        &self,
        name: &str,
        value: f64,
        labels: HashMap<String, String>,
        unit: &str,
        description: &str,
    ) -> Result<()> {
        let metric = CustomMetric {
            name: name.to_string(),
            value,
            labels,
            timestamp: Utc::now(),
            unit: unit.to_string(),
            description: description.to_string(),
        };

        let mut custom_metrics = self.custom_metrics.lock().await;
        custom_metrics.insert(name.to_string(), metric);

        debug!("Custom metric '{}' recorded: {} {}", name, value, unit);
        Ok(())
    }

    pub async fn get_custom_metric(&self, name: &str) -> Result<Option<CustomMetric>> {
        let custom_metrics = self.custom_metrics.lock().await;
        Ok(custom_metrics.get(name).cloned())
    }

    // ========================================================================
    // System Metrics Collection
    // ========================================================================

    pub async fn collect_system_metrics(&self) -> Result<()> {
        debug!("Collecting system metrics");

        // Memory usage
        let memory_usage_mb = self.get_memory_usage_mb().await;
        self.set_gauge("system_memory_usage_mb", memory_usage_mb as i64).await?;

        // CPU usage
        let cpu_usage_percent = self.get_cpu_usage_percent().await;
        self.set_gauge("system_cpu_usage_percent", (cpu_usage_percent * 100.0) as i64).await?;

        // Disk usage
        let disk_usage_percent = self.get_disk_usage_percent().await;
        self.set_gauge("system_disk_usage_percent", (disk_usage_percent * 100.0) as i64).await?;

        // Network statistics
        let (bytes_in, bytes_out) = self.get_network_stats().await;
        self.set_gauge("system_network_bytes_in", bytes_in as i64).await?;
        self.set_gauge("system_network_bytes_out", bytes_out as i64).await?;

        // Process statistics
        let open_fds = self.get_open_file_descriptors().await;
        self.set_gauge("system_open_file_descriptors", open_fds as i64).await?;

        let thread_count = self.get_thread_count().await;
        self.set_gauge("system_thread_count", thread_count as i64).await?;

        debug!("System metrics collection completed");
        Ok(())
    }

    // ========================================================================
    // Metrics Export
    // ========================================================================

    pub async fn get_metrics_snapshot(&self) -> Result<MetricsSnapshot> {
        let timestamp = Utc::now();
        let uptime_seconds = self.start_time.elapsed().as_secs();

        // Collect counters
        let counters = {
            let counters = self.counters.read().unwrap();
            counters.iter()
                .map(|(name, counter)| (name.clone(), counter.load(Ordering::Relaxed)))
                .collect()
        };

        // Collect gauges
        let gauges = {
            let gauges = self.gauges.read().unwrap();
            gauges.iter()
                .map(|(name, gauge)| (name.clone(), gauge.load(Ordering::Relaxed)))
                .collect()
        };

        // Collect histograms
        let histograms = {
            let histograms = self.histograms.read().unwrap();
            histograms.iter()
                .map(|(name, histogram)| (name.clone(), histogram.snapshot()))
                .collect()
        };

        // Collect timers
        let timers = {
            let timers = self.timers.read().unwrap();
            timers.iter()
                .map(|(name, timer)| (name.clone(), timer.snapshot()))
                .collect()
        };

        // Collect custom metrics
        let custom_metrics = {
            let custom_metrics = self.custom_metrics.lock().await;
            custom_metrics.values().cloned().collect()
        };

        // Collect system metrics
        let system_metrics = SystemMetrics {
            memory_usage_mb: self.get_memory_usage_mb().await,
            cpu_usage_percent: self.get_cpu_usage_percent().await,
            disk_usage_percent: self.get_disk_usage_percent().await,
            network_bytes_in: self.get_network_stats().await.0,
            network_bytes_out: self.get_network_stats().await.1,
            open_file_descriptors: self.get_open_file_descriptors().await,
            thread_count: self.get_thread_count().await,
            gc_count: self.get_gc_count().await,
            gc_time_ms: self.get_gc_time_ms().await,
        };

        Ok(MetricsSnapshot {
            timestamp,
            uptime_seconds,
            counters,
            gauges,
            histograms,
            timers,
            custom_metrics,
            system_metrics,
        })
    }

    pub async fn get_prometheus_metrics(&self) -> Result<String> {
        let snapshot = self.get_metrics_snapshot().await?;
        let mut output = String::new();

        // Add metadata
        output.push_str("# HELP test_data_api_info Service information\n");
        output.push_str("# TYPE test_data_api_info gauge\n");
        output.push_str(&format!("test_data_api_info{{version=\"1.0.0\"}} 1\n"));

        // Add uptime
        output.push_str("# HELP test_data_api_uptime_seconds Service uptime in seconds\n");
        output.push_str("# TYPE test_data_api_uptime_seconds counter\n");
        output.push_str(&format!("test_data_api_uptime_seconds {}\n", snapshot.uptime_seconds));

        // Add counters
        for (name, value) in &snapshot.counters {
            let metric_name = format!("test_data_api_{}", name.replace("-", "_"));
            output.push_str(&format!("# HELP {} Counter metric\n", metric_name));
            output.push_str(&format!("# TYPE {} counter\n", metric_name));
            output.push_str(&format!("{} {}\n", metric_name, value));
        }

        // Add gauges
        for (name, value) in &snapshot.gauges {
            let metric_name = format!("test_data_api_{}", name.replace("-", "_"));
            output.push_str(&format!("# HELP {} Gauge metric\n", metric_name));
            output.push_str(&format!("# TYPE {} gauge\n", metric_name));
            output.push_str(&format!("{} {}\n", metric_name, value));
        }

        // Add histograms
        for (name, histogram) in &snapshot.histograms {
            let metric_name = format!("test_data_api_{}", name.replace("-", "_"));
            output.push_str(&format!("# HELP {} Histogram metric\n", metric_name));
            output.push_str(&format!("# TYPE {} histogram\n", metric_name));

            for (upper_bound, count) in &histogram.buckets {
                output.push_str(&format!("{}{{le=\"{}\"}} {}\n",
                    format!("{}_bucket", metric_name), upper_bound, count));
            }

            output.push_str(&format!("{}_sum {}\n", metric_name, histogram.sum));
            output.push_str(&format!("{}_count {}\n", metric_name, histogram.count));
        }

        // Add timers as summaries
        for (name, timer) in &snapshot.timers {
            let metric_name = format!("test_data_api_{}_seconds", name.replace("-", "_"));
            output.push_str(&format!("# HELP {} Timer metric in seconds\n", metric_name));
            output.push_str(&format!("# TYPE {} summary\n", metric_name));
            output.push_str(&format!("{}_sum {}\n", metric_name, timer.total_duration_ms as f64 / 1000.0));
            output.push_str(&format!("{}_count {}\n", metric_name, timer.count));
        }

        // Add system metrics
        output.push_str(&format!("test_data_api_memory_usage_bytes {}\n",
            snapshot.system_metrics.memory_usage_mb * 1024.0 * 1024.0));
        output.push_str(&format!("test_data_api_cpu_usage_ratio {}\n",
            snapshot.system_metrics.cpu_usage_percent / 100.0));
        output.push_str(&format!("test_data_api_disk_usage_ratio {}\n",
            snapshot.system_metrics.disk_usage_percent / 100.0));

        Ok(output)
    }

    // ========================================================================
    // System Resource Collection
    // ========================================================================

    async fn get_memory_usage_mb(&self) -> f64 {
        // Mock implementation - in production would use system APIs
        128.5
    }

    async fn get_cpu_usage_percent(&self) -> f64 {
        // Mock implementation - in production would calculate actual CPU usage
        15.3
    }

    async fn get_disk_usage_percent(&self) -> f64 {
        // Mock implementation - in production would check actual disk usage
        45.2
    }

    async fn get_network_stats(&self) -> (u64, u64) {
        // Mock implementation - in production would read from /proc/net/dev
        (1024 * 1024 * 50, 1024 * 1024 * 25) // 50MB in, 25MB out
    }

    async fn get_open_file_descriptors(&self) -> u64 {
        // Mock implementation - in production would read from /proc/self/fd
        45
    }

    async fn get_thread_count(&self) -> u64 {
        // Mock implementation - in production would count actual threads
        12
    }

    async fn get_gc_count(&self) -> u64 {
        // Not applicable to Rust, but useful for other runtimes
        0
    }

    async fn get_gc_time_ms(&self) -> u64 {
        // Not applicable to Rust, but useful for other runtimes
        0
    }

    // ========================================================================
    // Initialization
    // ========================================================================

    async fn initialize_default_metrics(&self) -> Result<()> {
        debug!("Initializing default metrics");

        // Initialize request counters
        self.add_to_counter("requests_total", 0).await?;
        self.add_to_counter("requests_success", 0).await?;
        self.add_to_counter("requests_error", 0).await?;

        // Initialize operation counters
        self.add_to_counter("test_users_created", 0).await?;
        self.add_to_counter("test_users_deleted", 0).await?;
        self.add_to_counter("test_environments_created", 0).await?;
        self.add_to_counter("test_environments_reset", 0).await?;
        self.add_to_counter("data_generation_started", 0).await?;
        self.add_to_counter("cleanup_operations_started", 0).await?;
        self.add_to_counter("scheduled_cleanups_executed", 0).await?;

        // Initialize connection gauges
        self.set_gauge("active_connections", 0).await?;
        self.set_gauge("database_connections", 0).await?;
        self.set_gauge("redis_connections", 0).await?;

        // Initialize response time histograms
        let response_time_hist = Histogram::new_with_buckets(vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0
        ]);
        {
            let mut histograms = self.histograms.write().unwrap();
            histograms.insert("http_request_duration_ms".to_string(), response_time_hist);
        }

        info!("Default metrics initialized");
        Ok(())
    }
}

// ============================================================================
// Timer Handle for automatic timing
// ============================================================================

pub struct TimerHandle {
    name: String,
    start_time: Instant,
    service: Arc<MetricsService>,
}

impl Drop for TimerHandle {
    fn drop(&mut self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        let service = self.service.clone();
        let name = self.name.clone();

        tokio::spawn(async move {
            let _ = service.record_timer(&name, duration_ms).await;
        });
    }
}

// ============================================================================
// Histogram Implementation
// ============================================================================

impl Histogram {
    fn new() -> Self {
        Self::new_with_buckets(vec![
            1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0
        ])
    }

    fn new_with_buckets(bucket_bounds: Vec<f64>) -> Self {
        let buckets = bucket_bounds.into_iter()
            .map(|bound| (bound, AtomicU64::new(0)))
            .collect();

        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    fn record(&self, value: f64) {
        // Update count and sum
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add((value * 1000.0) as u64, Ordering::Relaxed); // Store as microseconds

        // Update buckets
        for (bound, counter) in &self.buckets {
            if value <= *bound {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn snapshot(&self) -> HistogramSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum = self.sum.load(Ordering::Relaxed);

        let buckets = self.buckets.iter()
            .map(|(bound, counter)| (*bound, counter.load(Ordering::Relaxed)))
            .collect();

        let average = if count > 0 {
            (sum as f64 / 1000.0) / count as f64
        } else {
            0.0
        };

        HistogramSnapshot {
            buckets,
            sum,
            count,
            average,
        }
    }
}

// ============================================================================
// Timer Implementation
// ============================================================================

impl Timer {
    fn new() -> Self {
        Self {
            total_duration_ms: AtomicU64::new(0),
            count: AtomicU64::new(0),
            min_duration_ms: AtomicU64::new(u64::MAX),
            max_duration_ms: AtomicU64::new(0),
        }
    }

    fn record(&self, duration_ms: u64) {
        self.total_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update min
        let current_min = self.min_duration_ms.load(Ordering::Relaxed);
        if duration_ms < current_min {
            self.min_duration_ms.store(duration_ms, Ordering::Relaxed);
        }

        // Update max
        let current_max = self.max_duration_ms.load(Ordering::Relaxed);
        if duration_ms > current_max {
            self.max_duration_ms.store(duration_ms, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> TimerSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let total_duration_ms = self.total_duration_ms.load(Ordering::Relaxed);
        let min_duration_ms = self.min_duration_ms.load(Ordering::Relaxed);
        let max_duration_ms = self.max_duration_ms.load(Ordering::Relaxed);

        let average_duration_ms = if count > 0 {
            total_duration_ms as f64 / count as f64
        } else {
            0.0
        };

        TimerSnapshot {
            count,
            total_duration_ms,
            average_duration_ms,
            min_duration_ms: if min_duration_ms == u64::MAX { 0 } else { min_duration_ms },
            max_duration_ms,
        }
    }
}

// ============================================================================
// Clone implementation for shared usage
// ============================================================================

impl Clone for MetricsService {
    fn clone(&self) -> Self {
        Self {
            counters: self.counters.clone(),
            gauges: self.gauges.clone(),
            histograms: self.histograms.clone(),
            timers: self.timers.clone(),
            custom_metrics: self.custom_metrics.clone(),
            start_time: self.start_time,
            system_start_time: self.system_start_time,
        }
    }
}

// ============================================================================
// Utility macros for easier metric recording
// ============================================================================

#[macro_export]
macro_rules! increment_counter {
    ($service:expr, $name:expr) => {
        $service.increment_counter($name).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to increment counter '{}': {}", $name, e);
        })
    };
}

#[macro_export]
macro_rules! set_gauge {
    ($service:expr, $name:expr, $value:expr) => {
        $service.set_gauge($name, $value).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to set gauge '{}': {}", $name, e);
        })
    };
}

#[macro_export]
macro_rules! record_histogram {
    ($service:expr, $name:expr, $value:expr) => {
        $service.record_histogram($name, $value).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to record histogram '{}': {}", $name, e);
        })
    };
}

#[macro_export]
macro_rules! time_operation {
    ($service:expr, $name:expr, $operation:expr) => {{
        let timer = $service.start_timer($name).await;
        let result = $operation;
        drop(timer);
        result
    }};
}

// ============================================================================
// Testing utilities
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_counter_operations() {
        let service = MetricsService::new().await.unwrap();

        service.increment_counter("test_counter").await.unwrap();
        service.add_to_counter("test_counter", 5).await.unwrap();

        let value = service.get_counter("test_counter").await.unwrap();
        assert_eq!(value, 6);
    }

    #[tokio::test]
    async fn test_gauge_operations() {
        let service = MetricsService::new().await.unwrap();

        service.set_gauge("test_gauge", 10).await.unwrap();
        service.increment_gauge("test_gauge").await.unwrap();
        service.add_to_gauge("test_gauge", 5).await.unwrap();

        let value = service.get_gauge("test_gauge").await.unwrap();
        assert_eq!(value, 16);
    }

    #[tokio::test]
    async fn test_histogram_operations() {
        let service = MetricsService::new().await.unwrap();

        service.record_histogram("test_histogram", 5.0).await.unwrap();
        service.record_histogram("test_histogram", 15.0).await.unwrap();
        service.record_histogram("test_histogram", 25.0).await.unwrap();

        let snapshot = service.get_histogram_snapshot("test_histogram").await.unwrap();
        assert!(snapshot.is_some());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.count, 3);
        assert_eq!(snapshot.average, 15.0);
    }

    #[tokio::test]
    async fn test_timer_operations() {
        let service = MetricsService::new().await.unwrap();

        {
            let _timer = service.start_timer("test_timer").await;
            sleep(TokioDuration::from_millis(10)).await;
        }

        let snapshot = service.get_timer_snapshot("test_timer").await.unwrap();
        assert!(snapshot.is_some());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.count, 1);
        assert!(snapshot.average_duration_ms >= 10.0);
    }

    #[tokio::test]
    async fn test_prometheus_export() {
        let service = MetricsService::new().await.unwrap();

        service.increment_counter("test_requests").await.unwrap();
        service.set_gauge("test_connections", 5).await.unwrap();

        let prometheus_output = service.get_prometheus_metrics().await.unwrap();
        assert!(prometheus_output.contains("test_data_api_test_requests"));
        assert!(prometheus_output.contains("test_data_api_test_connections"));
    }
}
