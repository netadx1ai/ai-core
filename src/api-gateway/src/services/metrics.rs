//! Metrics service for application monitoring and instrumentation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use prometheus::{Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, Opts, Registry};
use tracing::{debug, error, info, warn};

use crate::error::{ApiError, Result};

/// Metrics service for collecting and exposing application metrics
#[derive(Clone)]
pub struct MetricsService {
    registry: Arc<Registry>,

    // HTTP metrics
    pub http_requests_total: CounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub http_requests_in_flight: Gauge,

    // Business metrics
    pub workflow_executions_total: CounterVec,
    pub workflow_execution_duration_seconds: HistogramVec,
    pub workflow_execution_errors_total: CounterVec,

    // System metrics
    pub active_connections: Gauge,
    pub database_connections_active: Gauge,
    pub database_connections_idle: Gauge,
    pub redis_connections_active: Gauge,

    // Rate limiting metrics
    pub rate_limit_hits_total: CounterVec,
    pub rate_limit_blocks_total: CounterVec,

    // Authentication metrics
    pub authentication_attempts_total: CounterVec,
    pub authentication_failures_total: CounterVec,

    // Service health metrics
    pub service_health_status: GaugeVec,
    pub circuit_breaker_state: GaugeVec,

    // Custom metrics storage
    custom_counters: Arc<std::sync::RwLock<HashMap<String, Counter>>>,
    custom_gauges: Arc<std::sync::RwLock<HashMap<String, Gauge>>>,
    custom_histograms: Arc<std::sync::RwLock<HashMap<String, Histogram>>>,
}

impl MetricsService {
    /// Create new metrics service
    pub fn new() -> Result<Self> {
        let registry = Arc::new(Registry::new());

        // HTTP metrics
        let http_requests_total = CounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total number of HTTP requests processed",
            ),
            &["method", "path", "status_code", "user_tier"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create http_requests_total metric: {}",
                e
            ))
        })?;

        let http_request_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "Duration of HTTP requests in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "path", "user_tier"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create http_request_duration_seconds metric: {}",
                e
            ))
        })?;

        let http_requests_in_flight = Gauge::new(
            "http_requests_in_flight",
            "Number of HTTP requests currently being processed",
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create http_requests_in_flight metric: {}",
                e
            ))
        })?;

        // Business metrics
        let workflow_executions_total = CounterVec::new(
            Opts::new(
                "workflow_executions_total",
                "Total number of workflow executions",
            ),
            &["workflow_id", "user_id", "user_tier", "status"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create workflow_executions_total metric: {}",
                e
            ))
        })?;

        let workflow_execution_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "workflow_execution_duration_seconds",
                "Duration of workflow executions in seconds",
            )
            .buckets(vec![
                1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0,
            ]),
            &["workflow_id", "user_tier", "complexity"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create workflow_execution_duration_seconds metric: {}",
                e
            ))
        })?;

        let workflow_execution_errors_total = CounterVec::new(
            Opts::new(
                "workflow_execution_errors_total",
                "Total number of workflow execution errors",
            ),
            &["workflow_id", "user_id", "user_tier", "error_type"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create workflow_execution_errors_total metric: {}",
                e
            ))
        })?;

        // System metrics
        let active_connections =
            Gauge::new("active_connections_total", "Number of active connections").map_err(
                |e| {
                    ApiError::internal(format!("Failed to create active_connections metric: {}", e))
                },
            )?;

        let database_connections_active = Gauge::new(
            "database_connections_active",
            "Number of active database connections",
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create database_connections_active metric: {}",
                e
            ))
        })?;

        let database_connections_idle = Gauge::new(
            "database_connections_idle",
            "Number of idle database connections",
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create database_connections_idle metric: {}",
                e
            ))
        })?;

        let redis_connections_active = Gauge::new(
            "redis_connections_active",
            "Number of active Redis connections",
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create redis_connections_active metric: {}",
                e
            ))
        })?;

        // Rate limiting metrics
        let rate_limit_hits_total = CounterVec::new(
            Opts::new("rate_limit_hits_total", "Total number of rate limit hits"),
            &["user_id", "user_tier", "endpoint"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create rate_limit_hits_total metric: {}",
                e
            ))
        })?;

        let rate_limit_blocks_total = CounterVec::new(
            Opts::new(
                "rate_limit_blocks_total",
                "Total number of rate limit blocks",
            ),
            &["user_id", "user_tier", "endpoint"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create rate_limit_blocks_total metric: {}",
                e
            ))
        })?;

        // Authentication metrics
        let authentication_attempts_total = CounterVec::new(
            Opts::new(
                "authentication_attempts_total",
                "Total number of authentication attempts",
            ),
            &["method", "user_tier", "success"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create authentication_attempts_total metric: {}",
                e
            ))
        })?;

        let authentication_failures_total = CounterVec::new(
            Opts::new(
                "authentication_failures_total",
                "Total number of authentication failures",
            ),
            &["method", "failure_reason"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create authentication_failures_total metric: {}",
                e
            ))
        })?;

        // Service health metrics
        let service_health_status = GaugeVec::new(
            Opts::new(
                "service_health_status",
                "Health status of services (1 = healthy, 0 = unhealthy)",
            ),
            &["service_name", "service_type"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create service_health_status metric: {}",
                e
            ))
        })?;

        let circuit_breaker_state = GaugeVec::new(
            Opts::new(
                "circuit_breaker_state",
                "Circuit breaker state (0 = closed, 1 = half-open, 2 = open)",
            ),
            &["service_name"],
        )
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to create circuit_breaker_state metric: {}",
                e
            ))
        })?;

        // Register all metrics
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;
        registry.register(Box::new(http_requests_in_flight.clone()))?;
        registry.register(Box::new(workflow_executions_total.clone()))?;
        registry.register(Box::new(workflow_execution_duration_seconds.clone()))?;
        registry.register(Box::new(workflow_execution_errors_total.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(database_connections_active.clone()))?;
        registry.register(Box::new(database_connections_idle.clone()))?;
        registry.register(Box::new(redis_connections_active.clone()))?;
        registry.register(Box::new(rate_limit_hits_total.clone()))?;
        registry.register(Box::new(rate_limit_blocks_total.clone()))?;
        registry.register(Box::new(authentication_attempts_total.clone()))?;
        registry.register(Box::new(authentication_failures_total.clone()))?;
        registry.register(Box::new(service_health_status.clone()))?;
        registry.register(Box::new(circuit_breaker_state.clone()))?;

        info!(
            "Metrics service initialized with {} collectors",
            registry.gather().len()
        );

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
            workflow_executions_total,
            workflow_execution_duration_seconds,
            workflow_execution_errors_total,
            active_connections,
            database_connections_active,
            database_connections_idle,
            redis_connections_active,
            rate_limit_hits_total,
            rate_limit_blocks_total,
            authentication_attempts_total,
            authentication_failures_total,
            service_health_status,
            circuit_breaker_state,
            custom_counters: Arc::new(std::sync::RwLock::new(HashMap::new())),
            custom_gauges: Arc::new(std::sync::RwLock::new(HashMap::new())),
            custom_histograms: Arc::new(std::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Record HTTP request metrics
    pub fn record_http_request(
        &self,
        method: &str,
        path: &str,
        status_code: u16,
        duration: Duration,
        user_tier: &str,
    ) {
        self.http_requests_total
            .with_label_values(&[method, path, &status_code.to_string(), user_tier])
            .inc();

        self.http_request_duration_seconds
            .with_label_values(&[method, path, user_tier])
            .observe(duration.as_secs_f64());

        debug!(
            "Recorded HTTP request: {} {} {} ({:.3}s)",
            method,
            path,
            status_code,
            duration.as_secs_f64()
        );
    }

    /// Increment in-flight requests counter
    pub fn increment_in_flight_requests(&self) {
        self.http_requests_in_flight.inc();
    }

    /// Decrement in-flight requests counter
    pub fn decrement_in_flight_requests(&self) {
        self.http_requests_in_flight.dec();
    }

    /// Record workflow execution
    pub fn record_workflow_execution(
        &self,
        workflow_id: &str,
        user_id: &str,
        user_tier: &str,
        status: &str,
        duration: Option<Duration>,
        complexity: &str,
    ) {
        self.workflow_executions_total
            .with_label_values(&[workflow_id, user_id, user_tier, status])
            .inc();

        if let Some(dur) = duration {
            self.workflow_execution_duration_seconds
                .with_label_values(&[workflow_id, user_tier, complexity])
                .observe(dur.as_secs_f64());
        }

        debug!(
            "Recorded workflow execution: {} by {} ({}) - {}",
            workflow_id, user_id, user_tier, status
        );
    }

    /// Record workflow execution error
    pub fn record_workflow_error(
        &self,
        workflow_id: &str,
        user_id: &str,
        user_tier: &str,
        error_type: &str,
    ) {
        self.workflow_execution_errors_total
            .with_label_values(&[workflow_id, user_id, user_tier, error_type])
            .inc();

        debug!(
            "Recorded workflow error: {} by {} - {}",
            workflow_id, user_id, error_type
        );
    }

    /// Record workflow created
    pub fn record_workflow_created(
        &self,
        workflow_id: &str,
        user_id: &str,
        user_tier: &str,
        workflow_type: &str,
    ) {
        self.workflow_executions_total
            .with_label_values(&[workflow_type, user_tier])
            .inc();

        debug!(
            "Recorded workflow created: {} by {} ({}) - type: {}",
            workflow_id, user_id, user_tier, workflow_type
        );
    }

    /// Record workflow executed
    pub fn record_workflow_executed(
        &self,
        workflow_id: &str,
        user_id: &str,
        user_tier: &str,
        workflow_type: &str,
    ) {
        self.workflow_executions_total
            .with_label_values(&[workflow_type, user_tier])
            .inc();

        debug!(
            "Recorded workflow executed: {} by {} ({}) - type: {}",
            workflow_id, user_id, user_tier, workflow_type
        );
    }

    /// Record user login
    pub fn record_user_login(&self, user_id: &str, user_tier: &str) {
        self.authentication_attempts_total
            .with_label_values(&["password", user_tier, "true"])
            .inc();

        debug!("Recorded user login: {} ({})", user_id, user_tier);
    }

    /// Record API key login
    pub fn record_api_key_login(&self, user_id: &str, user_tier: &str) {
        self.authentication_attempts_total
            .with_label_values(&["api_key", user_tier, "true"])
            .inc();

        debug!("Recorded API key login: {} ({})", user_id, user_tier);
    }

    /// Record user logout
    pub fn record_user_logout(&self, user_id: &str) {
        debug!("Recorded user logout: {}", user_id);
        // Could add logout-specific metrics if needed
    }

    /// Record authentication failure
    pub fn record_authentication_failure(&self, method: &str, reason: &str) {
        self.authentication_attempts_total
            .with_label_values(&[method, "unknown", "false"])
            .inc();

        self.authentication_failures_total
            .with_label_values(&[method, reason])
            .inc();

        warn!("Recorded authentication failure: {} - {}", method, reason);
    }

    /// Record rate limit hit
    pub fn record_rate_limit_hit(&self, user_id: &str, user_tier: &str) {
        self.rate_limit_hits_total
            .with_label_values(&[user_id, user_tier, "general"])
            .inc();

        debug!("Recorded rate limit hit: {} ({})", user_id, user_tier);
    }

    /// Record rate limit block
    pub fn record_rate_limit_block(&self, user_id: &str, user_tier: &str, endpoint: &str) {
        self.rate_limit_blocks_total
            .with_label_values(&[user_id, user_tier, endpoint])
            .inc();

        warn!(
            "Recorded rate limit block: {} ({}) at {}",
            user_id, user_tier, endpoint
        );
    }

    /// Set active connections count
    pub fn set_active_connections(&self, count: f64) {
        self.active_connections.set(count);
    }

    /// Set database connection metrics
    pub fn set_database_connections(&self, active: f64, idle: f64) {
        self.database_connections_active.set(active);
        self.database_connections_idle.set(idle);
    }

    /// Set Redis connection metrics
    pub fn set_redis_connections(&self, active: f64) {
        self.redis_connections_active.set(active);
    }

    /// Set service health status
    pub fn set_service_health(&self, service_name: &str, service_type: &str, is_healthy: bool) {
        let value = if is_healthy { 1.0 } else { 0.0 };
        self.service_health_status
            .with_label_values(&[service_name, service_type])
            .set(value);

        debug!(
            "Set service health: {} ({}) = {}",
            service_name, service_type, is_healthy
        );
    }

    /// Set circuit breaker state
    pub fn set_circuit_breaker_state(&self, service_name: &str, state: CircuitBreakerState) {
        let value = match state {
            CircuitBreakerState::Closed => 0.0,
            CircuitBreakerState::HalfOpen => 1.0,
            CircuitBreakerState::Open => 2.0,
        };

        self.circuit_breaker_state
            .with_label_values(&[service_name])
            .set(value);

        debug!("Set circuit breaker state: {} = {:?}", service_name, state);
    }

    /// Create and register a custom counter
    pub fn create_custom_counter(&self, name: &str, help: &str) -> Result<()> {
        let counter = Counter::new(name, help)
            .map_err(|e| ApiError::internal(format!("Failed to create custom counter: {}", e)))?;

        self.registry.register(Box::new(counter.clone()))?;

        let mut counters = self.custom_counters.write().unwrap();
        counters.insert(name.to_string(), counter);

        info!("Created custom counter: {}", name);
        Ok(())
    }

    /// Increment a custom counter
    pub fn increment_custom_counter(&self, name: &str) -> Result<()> {
        let counters = self.custom_counters.read().unwrap();
        if let Some(counter) = counters.get(name) {
            counter.inc();
            Ok(())
        } else {
            Err(ApiError::not_found(format!(
                "Custom counter not found: {}",
                name
            )))
        }
    }

    /// Create and register a custom gauge
    pub fn create_custom_gauge(&self, name: &str, help: &str) -> Result<()> {
        let gauge = Gauge::new(name, help)
            .map_err(|e| ApiError::internal(format!("Failed to create custom gauge: {}", e)))?;

        self.registry.register(Box::new(gauge.clone()))?;

        let mut gauges = self.custom_gauges.write().unwrap();
        gauges.insert(name.to_string(), gauge);

        info!("Created custom gauge: {}", name);
        Ok(())
    }

    /// Set a custom gauge value
    pub fn set_custom_gauge(&self, name: &str, value: f64) -> Result<()> {
        let gauges = self.custom_gauges.read().unwrap();
        if let Some(gauge) = gauges.get(name) {
            gauge.set(value);
            Ok(())
        } else {
            Err(ApiError::not_found(format!(
                "Custom gauge not found: {}",
                name
            )))
        }
    }

    /// Get metrics in Prometheus format
    pub fn get_prometheus_metrics(&self) -> Result<String> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();

        encoder
            .encode_to_string(&metric_families)
            .map_err(|e| ApiError::internal(format!("Failed to encode metrics: {}", e)))
    }

    /// Get metrics summary for health checks
    pub fn get_metrics_summary(&self) -> MetricsSummary {
        let metric_families = self.registry.gather();

        MetricsSummary {
            total_collectors: metric_families.len(),
            total_metrics: metric_families
                .iter()
                .map(|family| family.get_metric().len())
                .sum(),
            last_updated: chrono::Utc::now(),
        }
    }

    /// Record custom timing
    pub fn record_custom_timing<T, F>(&self, name: &str, operation: F) -> Result<T>
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();

        // Create or get custom histogram
        let histograms = self.custom_histograms.read().unwrap();
        if let Some(histogram) = histograms.get(name) {
            histogram.observe(duration.as_secs_f64());
        } else {
            drop(histograms);
            // Create the histogram if it doesn't exist
            let histogram = Histogram::with_opts(prometheus::HistogramOpts::new(
                &format!("{}_duration_seconds", name),
                &format!("Duration of {} operations in seconds", name),
            ))
            .map_err(|e| ApiError::internal(format!("Failed to create custom histogram: {}", e)))?;

            self.registry.register(Box::new(histogram.clone()))?;
            histogram.observe(duration.as_secs_f64());

            let mut histograms = self.custom_histograms.write().unwrap();
            histograms.insert(name.to_string(), histogram);
        }

        debug!(
            "Recorded custom timing: {} = {:.3}s",
            name,
            duration.as_secs_f64()
        );
        Ok(result)
    }
}

/// Circuit breaker state enumeration
#[derive(Debug, Clone, Copy)]
pub enum CircuitBreakerState {
    Closed,
    HalfOpen,
    Open,
}

/// Metrics summary for health checks
#[derive(Debug, serde::Serialize)]
pub struct MetricsSummary {
    pub total_collectors: usize,
    pub total_metrics: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new().expect("Failed to create default metrics service")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_service_creation() {
        let metrics = MetricsService::new().unwrap();
        let summary = metrics.get_metrics_summary();

        assert!(summary.total_collectors > 0);
        assert!(summary.total_metrics > 0);
    }

    #[test]
    fn test_http_request_recording() {
        let metrics = MetricsService::new().unwrap();

        metrics.record_http_request(
            "GET",
            "/api/v1/workflows",
            200,
            Duration::from_millis(150),
            "pro",
        );

        // Verify metrics were recorded by checking Prometheus output
        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("http_requests_total"));
        assert!(prometheus_output.contains("http_request_duration_seconds"));
    }

    #[test]
    fn test_workflow_execution_recording() {
        let metrics = MetricsService::new().unwrap();

        metrics.record_workflow_execution(
            "workflow-123",
            "user-456",
            "enterprise",
            "completed",
            Some(Duration::from_secs(30)),
            "moderate",
        );

        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("workflow_executions_total"));
        assert!(prometheus_output.contains("workflow_execution_duration_seconds"));
    }

    #[test]
    fn test_custom_counter() {
        let metrics = MetricsService::new().unwrap();

        metrics
            .create_custom_counter("test_counter", "Test counter")
            .unwrap();
        metrics.increment_custom_counter("test_counter").unwrap();

        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("test_counter"));
    }

    #[test]
    fn test_custom_gauge() {
        let metrics = MetricsService::new().unwrap();

        metrics
            .create_custom_gauge("test_gauge", "Test gauge")
            .unwrap();
        metrics.set_custom_gauge("test_gauge", 42.5).unwrap();

        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("test_gauge"));
    }

    #[test]
    fn test_custom_timing() {
        let metrics = MetricsService::new().unwrap();

        let result: i32 = metrics
            .record_custom_timing("test_operation", || {
                std::thread::sleep(Duration::from_millis(10));
                42
            })
            .unwrap();

        assert_eq!(result, 42);

        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("test_operation_duration_seconds"));
    }

    #[test]
    fn test_service_health_tracking() {
        let metrics = MetricsService::new().unwrap();

        metrics.set_service_health("database", "postgresql", true);
        metrics.set_service_health("redis", "cache", false);

        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("service_health_status"));
    }

    #[test]
    fn test_circuit_breaker_state() {
        let metrics = MetricsService::new().unwrap();

        metrics.set_circuit_breaker_state("external-api", CircuitBreakerState::Open);
        metrics.set_circuit_breaker_state("payment-service", CircuitBreakerState::HalfOpen);

        let prometheus_output = metrics.get_prometheus_metrics().unwrap();
        assert!(prometheus_output.contains("circuit_breaker_state"));
    }
}
