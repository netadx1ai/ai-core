//! # Metrics Module
//!
//! This module provides comprehensive metrics collection and reporting for the
//! database-security integration crate. It tracks performance, security events,
//! operation counts, and provides insights into system behavior.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use crate::error::SecureDatabaseError;

/// Main metrics collector for secure database operations
pub struct SecureDatabaseMetrics {
    /// Operation metrics by database type
    operation_metrics: Arc<RwLock<HashMap<String, DatabaseOperationMetrics>>>,
    /// Security event metrics
    security_metrics: Arc<RwLock<SecurityEventMetrics>>,
    /// Performance metrics
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Error metrics
    error_metrics: Arc<RwLock<ErrorMetrics>>,
    /// Cache metrics
    cache_metrics: Arc<RwLock<CacheMetrics>>,
    /// Connection metrics
    connection_metrics: Arc<RwLock<ConnectionMetrics>>,
    /// Audit metrics
    audit_metrics: Arc<RwLock<AuditMetrics>>,
    /// System health metrics
    health_metrics: Arc<RwLock<HealthMetrics>>,
    /// Metrics configuration
    config: MetricsConfig,
    /// Metrics start time
    start_time: DateTime<Utc>,
}

/// Database operation metrics for a specific database type
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DatabaseOperationMetrics {
    /// Total number of operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Operations by type
    pub operations_by_type: HashMap<String, u64>,
    /// Total response time in milliseconds
    pub total_response_time_ms: u64,
    /// Minimum response time in milliseconds
    pub min_response_time_ms: u64,
    /// Maximum response time in milliseconds
    pub max_response_time_ms: u64,
    /// Last operation timestamp
    pub last_operation: Option<DateTime<Utc>>,
    /// Operations per minute (sliding window)
    pub operations_per_minute: f64,
}

/// Security-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SecurityEventMetrics {
    /// Total authentication attempts
    pub authentication_attempts: u64,
    /// Successful authentications
    pub successful_authentications: u64,
    /// Failed authentications
    pub failed_authentications: u64,
    /// Authorization checks
    pub authorization_checks: u64,
    /// Authorization grants
    pub authorization_grants: u64,
    /// Authorization denials
    pub authorization_denials: u64,
    /// Permission cache hits
    pub permission_cache_hits: u64,
    /// Permission cache misses
    pub permission_cache_misses: u64,
    /// MFA challenges issued
    pub mfa_challenges: u64,
    /// Security context elevations
    pub context_elevations: u64,
    /// Security violations detected
    pub security_violations: u64,
    /// Rate limit violations
    pub rate_limit_violations: u64,
    /// Suspicious activity events
    pub suspicious_activities: u64,
}

/// Performance-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average query execution time by database
    pub avg_query_time_by_db: HashMap<String, f64>,
    /// Slow queries (above threshold)
    pub slow_queries: u64,
    /// Query timeout occurrences
    pub query_timeouts: u64,
    /// Connection pool utilization
    pub connection_pool_utilization: HashMap<String, f64>,
    /// Thread pool utilization
    pub thread_pool_utilization: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Garbage collection metrics
    pub gc_metrics: GarbageCollectionMetrics,
}

/// Garbage collection metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GarbageCollectionMetrics {
    /// Total GC collections
    pub total_collections: u64,
    /// Total GC time in milliseconds
    pub total_gc_time_ms: u64,
    /// Average GC time in milliseconds
    pub avg_gc_time_ms: f64,
}

/// Error-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,
    /// Errors by category
    pub errors_by_category: HashMap<String, u64>,
    /// Errors by severity
    pub errors_by_severity: HashMap<String, u64>,
    /// Recoverable errors
    pub recoverable_errors: u64,
    /// Retry attempts
    pub retry_attempts: u64,
    /// Successful retries
    pub successful_retries: u64,
    /// Circuit breaker trips
    pub circuit_breaker_trips: u64,
    /// Error rate (errors per minute)
    pub error_rate: f64,
}

/// Cache-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Total cache operations
    pub total_operations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Cache hit rate
    pub hit_rate: f64,
    /// Cache evictions
    pub evictions: u64,
    /// Cache size in entries
    pub cache_size: u64,
    /// Cache memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Average lookup time in microseconds
    pub avg_lookup_time_us: f64,
}

/// Connection-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Active connections by database
    pub active_connections: HashMap<String, u32>,
    /// Connection pool size by database
    pub pool_sizes: HashMap<String, u32>,
    /// Connection acquisitions
    pub connection_acquisitions: u64,
    /// Connection timeouts
    pub connection_timeouts: u64,
    /// Connection failures
    pub connection_failures: u64,
    /// Average connection acquisition time in milliseconds
    pub avg_acquisition_time_ms: f64,
    /// Connection leaks detected
    pub connection_leaks: u64,
}

/// Audit-related metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuditMetrics {
    /// Total audit events
    pub total_events: u64,
    /// Events by level
    pub events_by_level: HashMap<String, u64>,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Events written to database
    pub events_written_to_db: u64,
    /// Events written to file
    pub events_written_to_file: u64,
    /// Events streamed
    pub events_streamed: u64,
    /// Buffer overflows
    pub buffer_overflows: u64,
    /// Write errors
    pub write_errors: u64,
    /// Average event processing time in microseconds
    pub avg_processing_time_us: f64,
}

/// System health metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Health check results by service
    pub health_checks: HashMap<String, HealthCheckResult>,
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    /// Last health check timestamp
    pub last_health_check: Option<DateTime<Utc>>,
    /// System load average
    pub load_average: [f64; 3], // 1m, 5m, 15m
    /// Disk usage by mount point
    pub disk_usage: HashMap<String, DiskUsage>,
    /// Network statistics
    pub network_stats: NetworkStats,
}

/// Health check result for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub last_check: DateTime<Utc>,
    pub consecutive_failures: u32,
    pub error_message: Option<String>,
}

/// Disk usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

/// Network statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors_sent: u64,
    pub errors_received: u64,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics collection interval in seconds
    pub collection_interval: u64,
    /// Enable detailed operation metrics
    pub detailed_operations: bool,
    /// Enable performance monitoring
    pub performance_monitoring: bool,
    /// Slow query threshold in milliseconds
    pub slow_query_threshold_ms: u64,
    /// Enable cache metrics
    pub cache_metrics: bool,
    /// Enable health monitoring
    pub health_monitoring: bool,
    /// Maximum metrics history to keep
    pub max_history_entries: usize,
    /// Export format (prometheus, json, etc.)
    pub export_format: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: 60,
            detailed_operations: true,
            performance_monitoring: true,
            slow_query_threshold_ms: 1000,
            cache_metrics: true,
            health_monitoring: true,
            max_history_entries: 1000,
            export_format: "prometheus".to_string(),
        }
    }
}

impl SecureDatabaseMetrics {
    /// Create a new metrics collector
    pub fn new() -> Result<Self, SecureDatabaseError> {
        Self::with_config(MetricsConfig::default())
    }

    /// Create a new metrics collector with custom configuration
    pub fn with_config(config: MetricsConfig) -> Result<Self, SecureDatabaseError> {
        info!("Initializing secure database metrics");

        Ok(Self {
            operation_metrics: Arc::new(RwLock::new(HashMap::new())),
            security_metrics: Arc::new(RwLock::new(SecurityEventMetrics::default())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            error_metrics: Arc::new(RwLock::new(ErrorMetrics::default())),
            cache_metrics: Arc::new(RwLock::new(CacheMetrics::default())),
            connection_metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
            audit_metrics: Arc::new(RwLock::new(AuditMetrics::default())),
            health_metrics: Arc::new(RwLock::new(HealthMetrics::default())),
            config,
            start_time: Utc::now(),
        })
    }

    /// Record a database operation
    #[instrument(skip(self), fields(database = %database, operation = %operation))]
    pub async fn record_operation(
        &self,
        database: &str,
        operation: &str,
        duration: Duration,
        success: bool,
    ) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.operation_metrics.write().await;
        let db_metrics = metrics
            .entry(database.to_string())
            .or_insert_with(DatabaseOperationMetrics::default);

        db_metrics.total_operations += 1;
        if success {
            db_metrics.successful_operations += 1;
        } else {
            db_metrics.failed_operations += 1;
        }

        // Update operation type counters
        *db_metrics
            .operations_by_type
            .entry(operation.to_string())
            .or_insert(0) += 1;

        // Update timing metrics
        let duration_ms = duration.as_millis() as u64;
        db_metrics.total_response_time_ms += duration_ms;

        if db_metrics.min_response_time_ms == 0 || duration_ms < db_metrics.min_response_time_ms {
            db_metrics.min_response_time_ms = duration_ms;
        }

        if duration_ms > db_metrics.max_response_time_ms {
            db_metrics.max_response_time_ms = duration_ms;
        }

        db_metrics.last_operation = Some(Utc::now());

        // Update operations per minute (simplified calculation)
        db_metrics.operations_per_minute = self.calculate_operations_per_minute(db_metrics).await;

        // Record slow query if applicable
        if self.config.performance_monitoring && duration_ms > self.config.slow_query_threshold_ms {
            self.record_slow_query().await;
        }

        debug!(
            database = %database,
            operation = %operation,
            duration_ms = duration_ms,
            success = success,
            "Operation recorded"
        );
    }

    /// Record a security event
    pub async fn record_security_event(&self, event_type: &str, details: SecurityEventDetails) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.security_metrics.write().await;

        match event_type {
            "authentication_attempt" => {
                metrics.authentication_attempts += 1;
                if details.success {
                    metrics.successful_authentications += 1;
                } else {
                    metrics.failed_authentications += 1;
                }
            }
            "authorization_check" => {
                metrics.authorization_checks += 1;
                if details.success {
                    metrics.authorization_grants += 1;
                } else {
                    metrics.authorization_denials += 1;
                }
            }
            "permission_cache_hit" => {
                metrics.permission_cache_hits += 1;
            }
            "permission_cache_miss" => {
                metrics.permission_cache_misses += 1;
            }
            "mfa_challenge" => {
                metrics.mfa_challenges += 1;
            }
            "context_elevation" => {
                metrics.context_elevations += 1;
            }
            "security_violation" => {
                metrics.security_violations += 1;
            }
            "rate_limit_violation" => {
                metrics.rate_limit_violations += 1;
            }
            "suspicious_activity" => {
                metrics.suspicious_activities += 1;
            }
            _ => {
                debug!("Unknown security event type: {}", event_type);
            }
        }

        debug!(event_type = %event_type, success = details.success, "Security event recorded");
    }

    /// Record an error
    pub async fn record_error(&self, error: &SecureDatabaseError) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.error_metrics.write().await;
        metrics.total_errors += 1;

        // Categorize error
        let category = error.category().to_string();
        *metrics.errors_by_category.entry(category).or_insert(0) += 1;

        // Record severity
        let severity = error.severity().to_string();
        *metrics.errors_by_severity.entry(severity).or_insert(0) += 1;

        // Check if error is recoverable
        if error.is_recoverable() {
            metrics.recoverable_errors += 1;
        }

        // Update error rate
        metrics.error_rate = self.calculate_error_rate(&metrics).await;

        debug!(
            error_category = %error.category(),
            error_severity = %error.severity(),
            "Error recorded"
        );
    }

    /// Record cache operation
    pub async fn record_cache_operation(&self, hit: bool, lookup_time: Duration) {
        if !self.config.enabled || !self.config.cache_metrics {
            return;
        }

        let mut metrics = self.cache_metrics.write().await;
        metrics.total_operations += 1;

        if hit {
            metrics.cache_hits += 1;
        } else {
            metrics.cache_misses += 1;
        }

        // Update hit rate
        if metrics.total_operations > 0 {
            metrics.hit_rate = metrics.cache_hits as f64 / metrics.total_operations as f64;
        }

        // Update average lookup time
        let lookup_time_us = lookup_time.as_micros() as f64;
        metrics.avg_lookup_time_us =
            (metrics.avg_lookup_time_us * (metrics.total_operations - 1) as f64 + lookup_time_us)
                / metrics.total_operations as f64;

        debug!(
            cache_hit = hit,
            lookup_time_us = lookup_time_us,
            "Cache operation recorded"
        );
    }

    /// Record connection metrics
    pub async fn record_connection_event(
        &self,
        database: &str,
        event_type: &str,
        details: ConnectionEventDetails,
    ) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.connection_metrics.write().await;

        match event_type {
            "acquisition" => {
                metrics.connection_acquisitions += 1;
                if let Some(duration) = details.duration {
                    let duration_ms = duration.as_millis() as f64;
                    metrics.avg_acquisition_time_ms = (metrics.avg_acquisition_time_ms
                        * (metrics.connection_acquisitions - 1) as f64
                        + duration_ms)
                        / metrics.connection_acquisitions as f64;
                }
            }
            "timeout" => {
                metrics.connection_timeouts += 1;
            }
            "failure" => {
                metrics.connection_failures += 1;
            }
            "leak" => {
                metrics.connection_leaks += 1;
            }
            _ => {
                debug!("Unknown connection event type: {}", event_type);
            }
        }

        // Update active connections
        if let Some(active_count) = details.active_connections {
            metrics
                .active_connections
                .insert(database.to_string(), active_count);
        }

        debug!(
            database = %database,
            event_type = %event_type,
            "Connection event recorded"
        );
    }

    /// Record audit event
    pub async fn record_audit_event(&self, level: &str, event_type: &str) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.audit_metrics.write().await;
        metrics.total_events += 1;

        *metrics
            .events_by_level
            .entry(level.to_string())
            .or_insert(0) += 1;
        *metrics
            .events_by_type
            .entry(event_type.to_string())
            .or_insert(0) += 1;

        debug!(level = %level, event_type = %event_type, "Audit event recorded");
    }

    /// Update health metrics
    pub async fn update_health_status(
        &self,
        service: &str,
        healthy: bool,
        response_time: Duration,
        error: Option<String>,
    ) {
        if !self.config.enabled || !self.config.health_monitoring {
            return;
        }

        let response_time_ms = response_time.as_millis() as u64;
        let now = Utc::now();

        {
            let mut metrics = self.health_metrics.write().await;

            let health_result = metrics
                .health_checks
                .entry(service.to_string())
                .or_insert_with(|| HealthCheckResult {
                    healthy: true,
                    response_time_ms: 0,
                    last_check: now,
                    consecutive_failures: 0,
                    error_message: None,
                });

            health_result.healthy = healthy;
            health_result.response_time_ms = response_time_ms;
            health_result.last_check = now;
            health_result.error_message = error;

            if healthy {
                health_result.consecutive_failures = 0;
            } else {
                health_result.consecutive_failures += 1;
            }

            metrics.last_health_check = Some(now);
        }

        debug!(
            service = %service,
            healthy = healthy,
            response_time_ms = response_time_ms,
            "Health status updated"
        );
    }

    /// Generate comprehensive metrics report
    pub async fn generate_report(&self) -> Result<MetricsReport, SecureDatabaseError> {
        if !self.config.enabled {
            return Ok(MetricsReport::empty());
        }

        info!("Generating metrics report");

        let operation_metrics = self.operation_metrics.read().await.clone();
        let security_metrics = self.security_metrics.read().await.clone();
        let performance_metrics = self.performance_metrics.read().await.clone();
        let error_metrics = self.error_metrics.read().await.clone();
        let cache_metrics = self.cache_metrics.read().await.clone();
        let connection_metrics = self.connection_metrics.read().await.clone();
        let audit_metrics = self.audit_metrics.read().await.clone();
        let health_metrics = self.health_metrics.read().await.clone();

        let uptime = Utc::now() - self.start_time;

        Ok(MetricsReport {
            timestamp: Utc::now(),
            uptime_seconds: uptime.num_seconds() as u64,
            operation_metrics,
            security_metrics,
            performance_metrics,
            error_metrics,
            cache_metrics,
            connection_metrics,
            audit_metrics,
            health_metrics,
        })
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> Result<String, SecureDatabaseError> {
        let report = self.generate_report().await?;

        let mut output = String::new();

        // Add basic info
        output.push_str(&format!(
            "# HELP secure_database_uptime_seconds Total uptime in seconds\n"
        ));
        output.push_str(&format!("# TYPE secure_database_uptime_seconds counter\n"));
        output.push_str(&format!(
            "secure_database_uptime_seconds {}\n",
            report.uptime_seconds
        ));

        // Operation metrics
        for (database, metrics) in &report.operation_metrics {
            output.push_str(&format!(
                "# HELP secure_database_operations_total Total database operations\n"
            ));
            output.push_str(&format!(
                "# TYPE secure_database_operations_total counter\n"
            ));
            output.push_str(&format!(
                "secure_database_operations_total{{database=\"{}\"}} {}\n",
                database, metrics.total_operations
            ));

            output.push_str(&format!(
                "secure_database_operations_successful{{database=\"{}\"}} {}\n",
                database, metrics.successful_operations
            ));
            output.push_str(&format!(
                "secure_database_operations_failed{{database=\"{}\"}} {}\n",
                database, metrics.failed_operations
            ));
        }

        // Security metrics
        output.push_str(&format!(
            "secure_database_auth_attempts_total {}\n",
            report.security_metrics.authentication_attempts
        ));
        output.push_str(&format!(
            "secure_database_auth_successful_total {}\n",
            report.security_metrics.successful_authentications
        ));
        output.push_str(&format!(
            "secure_database_auth_failed_total {}\n",
            report.security_metrics.failed_authentications
        ));

        // Error metrics
        output.push_str(&format!(
            "secure_database_errors_total {}\n",
            report.error_metrics.total_errors
        ));
        output.push_str(&format!(
            "secure_database_error_rate {}\n",
            report.error_metrics.error_rate
        ));

        // Cache metrics
        output.push_str(&format!(
            "secure_database_cache_hits_total {}\n",
            report.cache_metrics.cache_hits
        ));
        output.push_str(&format!(
            "secure_database_cache_misses_total {}\n",
            report.cache_metrics.cache_misses
        ));
        output.push_str(&format!(
            "secure_database_cache_hit_rate {}\n",
            report.cache_metrics.hit_rate
        ));

        Ok(output)
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        if !self.config.enabled {
            return;
        }

        info!("Resetting all metrics");

        self.operation_metrics.write().await.clear();
        *self.security_metrics.write().await = SecurityEventMetrics::default();
        *self.performance_metrics.write().await = PerformanceMetrics::default();
        *self.error_metrics.write().await = ErrorMetrics::default();
        *self.cache_metrics.write().await = CacheMetrics::default();
        *self.connection_metrics.write().await = ConnectionMetrics::default();
        *self.audit_metrics.write().await = AuditMetrics::default();
        *self.health_metrics.write().await = HealthMetrics::default();
    }

    // Helper methods

    async fn calculate_operations_per_minute(&self, _db_metrics: &DatabaseOperationMetrics) -> f64 {
        // Simplified calculation - in production this would use a sliding window
        60.0 // Placeholder
    }

    async fn record_slow_query(&self) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.slow_queries += 1;
    }

    async fn calculate_error_rate(&self, _error_metrics: &ErrorMetrics) -> f64 {
        // Simplified calculation - in production this would use a time-based window
        1.0 // Placeholder
    }
}

impl Default for SecureDatabaseMetrics {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| panic!("Failed to create default SecureDatabaseMetrics"))
    }
}

/// Details for security events
#[derive(Debug, Clone)]
pub struct SecurityEventDetails {
    pub success: bool,
    pub user_id: Option<String>,
    pub resource: Option<String>,
    pub permission: Option<String>,
}

/// Details for connection events
#[derive(Debug, Clone)]
pub struct ConnectionEventDetails {
    pub duration: Option<Duration>,
    pub active_connections: Option<u32>,
    pub error_message: Option<String>,
}

/// Comprehensive metrics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReport {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub operation_metrics: HashMap<String, DatabaseOperationMetrics>,
    pub security_metrics: SecurityEventMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub error_metrics: ErrorMetrics,
    pub cache_metrics: CacheMetrics,
    pub connection_metrics: ConnectionMetrics,
    pub audit_metrics: AuditMetrics,
    pub health_metrics: HealthMetrics,
}

impl MetricsReport {
    /// Create an empty metrics report
    pub fn empty() -> Self {
        Self {
            timestamp: Utc::now(),
            uptime_seconds: 0,
            operation_metrics: HashMap::new(),
            security_metrics: SecurityEventMetrics::default(),
            performance_metrics: PerformanceMetrics::default(),
            error_metrics: ErrorMetrics::default(),
            cache_metrics: CacheMetrics::default(),
            connection_metrics: ConnectionMetrics::default(),
            audit_metrics: AuditMetrics::default(),
            health_metrics: HealthMetrics::default(),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, SecureDatabaseError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SecureDatabaseError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = SecureDatabaseMetrics::new();
        assert!(metrics.is_ok());
    }

    #[tokio::test]
    async fn test_operation_recording() {
        let metrics = SecureDatabaseMetrics::new().unwrap();

        metrics
            .record_operation("postgres", "select", Duration::from_millis(100), true)
            .await;

        let report = metrics.generate_report().await.unwrap();
        let postgres_metrics = report.operation_metrics.get("postgres").unwrap();

        assert_eq!(postgres_metrics.total_operations, 1);
        assert_eq!(postgres_metrics.successful_operations, 1);
        assert_eq!(postgres_metrics.failed_operations, 0);
    }

    #[tokio::test]
    async fn test_security_event_recording() {
        let metrics = SecureDatabaseMetrics::new().unwrap();

        let details = SecurityEventDetails {
            success: true,
            user_id: Some("user123".to_string()),
            resource: None,
            permission: None,
        };

        metrics
            .record_security_event("authentication_attempt", details)
            .await;

        let report = metrics.generate_report().await.unwrap();
        assert_eq!(report.security_metrics.authentication_attempts, 1);
        assert_eq!(report.security_metrics.successful_authentications, 1);
    }

    #[tokio::test]
    async fn test_error_recording() {
        let metrics = SecureDatabaseMetrics::new().unwrap();
        let error = SecureDatabaseError::database_operation("Test error");

        metrics.record_error(&error).await;

        let report = metrics.generate_report().await.unwrap();
        assert_eq!(report.error_metrics.total_errors, 1);
        assert!(report
            .error_metrics
            .errors_by_category
            .contains_key("database"));
    }

    #[tokio::test]
    async fn test_cache_operation_recording() {
        let metrics = SecureDatabaseMetrics::new().unwrap();

        metrics
            .record_cache_operation(true, Duration::from_micros(500))
            .await;
        metrics
            .record_cache_operation(false, Duration::from_micros(1000))
            .await;

        let report = metrics.generate_report().await.unwrap();
        assert_eq!(report.cache_metrics.total_operations, 2);
        assert_eq!(report.cache_metrics.cache_hits, 1);
        assert_eq!(report.cache_metrics.cache_misses, 1);
        assert_eq!(report.cache_metrics.hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_prometheus_export() {
        let metrics = SecureDatabaseMetrics::new().unwrap();

        // Record some test data
        metrics
            .record_operation("postgres", "select", Duration::from_millis(100), true)
            .await;

        let prometheus_output = metrics.export_prometheus().await;
        assert!(prometheus_output.is_ok());

        let output = prometheus_output.unwrap();
        assert!(output.contains("secure_database_uptime_seconds"));
        assert!(output.contains("secure_database_operations_total"));
    }

    #[tokio::test]
    async fn test_health_status_update() {
        let metrics = SecureDatabaseMetrics::new().unwrap();

        metrics
            .update_health_status("postgres", true, Duration::from_millis(50), None)
            .await;

        let report = metrics.generate_report().await.unwrap();
        let health_result = report.health_metrics.health_checks.get("postgres").unwrap();
        assert!(health_result.healthy);
        assert_eq!(health_result.response_time_ms, 50);
        assert_eq!(health_result.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let metrics = SecureDatabaseMetrics::new().unwrap();

        // Record some data
        metrics
            .record_operation("postgres", "select", Duration::from_millis(100), true)
            .await;

        let report_before = metrics.generate_report().await.unwrap();
        assert_eq!(
            report_before
                .operation_metrics
                .get("postgres")
                .unwrap()
                .total_operations,
            1
        );

        // Reset metrics
        metrics.reset().await;

        let report_after = metrics.generate_report().await.unwrap();
        assert!(report_after.operation_metrics.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_report_json() {
        let metrics = SecureDatabaseMetrics::new().unwrap();
        let report = metrics.generate_report().await.unwrap();

        let json = report.to_json();
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("timestamp"));
        assert!(json_str.contains("uptime_seconds"));
    }
}
