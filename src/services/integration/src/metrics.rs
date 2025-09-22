//! Metrics collection and tracking for the AI-CORE Integration Service
//!
//! This module provides comprehensive metrics collection for monitoring integration
//! performance, request rates, error rates, and system health.

use crate::models::IntegrationType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

/// Metrics collector for integration service
#[derive(Debug, Clone)]
pub struct IntegrationMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Requests by integration type
    pub requests_by_integration: HashMap<IntegrationType, u64>,
    /// Errors by type
    pub errors_by_type: HashMap<String, u64>,
    /// Last updated timestamp
    pub timestamp: DateTime<Utc>,
    /// Response time histogram
    pub response_time_histogram: ResponseTimeHistogram,
    /// Request rate tracker
    pub request_rate: RequestRateTracker,
}

/// Response time histogram for tracking latency distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeHistogram {
    /// < 10ms requests
    pub under_10ms: u64,
    /// 10ms - 50ms requests
    pub ms_10_to_50: u64,
    /// 50ms - 100ms requests
    pub ms_50_to_100: u64,
    /// 100ms - 500ms requests
    pub ms_100_to_500: u64,
    /// 500ms - 1000ms requests
    pub ms_500_to_1000: u64,
    /// > 1000ms requests
    pub over_1000ms: u64,
}

/// Request rate tracker for monitoring throughput
#[derive(Debug, Clone)]
pub struct RequestRateTracker {
    /// Current requests per second
    pub requests_per_second: f64,
    /// Peak requests per second
    pub peak_requests_per_second: f64,
    /// Request timestamps for rate calculation
    recent_requests: Vec<DateTime<Utc>>,
}

/// Atomic metrics for thread-safe operations
#[derive(Debug)]
pub struct AtomicMetrics {
    pub total_requests: AtomicU64,
    pub successful_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub total_response_time_ms: AtomicU64,
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Service name
    pub service: String,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Total requests
    pub total_requests: u64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Error rate percentage
    pub error_rate: f64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// 95th percentile response time
    pub p95_response_time_ms: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Integration breakdown
    pub integration_stats: HashMap<String, IntegrationStats>,
    /// Error breakdown
    pub error_breakdown: HashMap<String, u64>,
}

/// Per-integration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStats {
    /// Integration name
    pub name: String,
    /// Total requests for this integration
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Average response time
    pub avg_response_time_ms: f64,
}

impl Default for IntegrationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationMetrics {
    /// Create a new metrics instance
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            requests_by_integration: HashMap::new(),
            errors_by_type: HashMap::new(),
            timestamp: Utc::now(),
            response_time_histogram: ResponseTimeHistogram::new(),
            request_rate: RequestRateTracker::new(),
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, integration: IntegrationType, response_time_ms: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.update_response_time(response_time_ms);
        self.update_integration_count(integration);
        self.response_time_histogram.record(response_time_ms);
        self.request_rate.record_request();
        self.timestamp = Utc::now();

        debug!(
            integration = %integration,
            response_time_ms = response_time_ms,
            total_requests = self.total_requests,
            "Recorded successful request"
        );
    }

    /// Record a failed request
    pub fn record_failure(&mut self, integration: IntegrationType, error_type: &str) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.update_integration_count(integration);
        self.increment_error_count(error_type);
        self.request_rate.record_request();
        self.timestamp = Utc::now();

        debug!(
            integration = %integration,
            error_type = error_type,
            total_requests = self.total_requests,
            "Recorded failed request"
        );
    }

    /// Update average response time
    fn update_response_time(&mut self, response_time_ms: u64) {
        if self.successful_requests == 1 {
            self.avg_response_time_ms = response_time_ms as f64;
        } else {
            self.avg_response_time_ms = (self.avg_response_time_ms
                * (self.successful_requests - 1) as f64
                + response_time_ms as f64)
                / self.successful_requests as f64;
        }
    }

    /// Update integration request count
    fn update_integration_count(&mut self, integration: IntegrationType) {
        *self.requests_by_integration.entry(integration).or_insert(0) += 1;
    }

    /// Increment error count by type
    fn increment_error_count(&mut self, error_type: &str) {
        *self
            .errors_by_type
            .entry(error_type.to_string())
            .or_insert(0) += 1;
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            100.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Get error rate as percentage
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.failed_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Create a metrics snapshot for reporting
    pub fn snapshot(&self) -> MetricsSnapshot {
        let mut integration_stats = HashMap::new();

        for (integration_type, &requests) in &self.requests_by_integration {
            let integration_name = integration_type.as_str().to_string();

            // For detailed per-integration stats, we'd need to track more granular data
            // For now, we'll provide basic stats
            integration_stats.insert(
                integration_name.clone(),
                IntegrationStats {
                    name: integration_name,
                    total_requests: requests,
                    successful_requests: requests, // Simplified - would need separate tracking
                    failed_requests: 0,            // Simplified - would need separate tracking
                    success_rate: 100.0,           // Simplified - would need separate tracking
                    avg_response_time_ms: self.avg_response_time_ms,
                },
            );
        }

        MetricsSnapshot {
            service: "integration-service".to_string(),
            timestamp: Utc::now(),
            total_requests: self.total_requests,
            success_rate: self.success_rate(),
            error_rate: self.error_rate(),
            avg_response_time_ms: self.avg_response_time_ms,
            p95_response_time_ms: self.response_time_histogram.percentile_95(),
            requests_per_second: self.request_rate.current_rate(),
            integration_stats,
            error_breakdown: self.errors_by_type.clone(),
        }
    }

    /// Reset all metrics (useful for testing or periodic resets)
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Export metrics in Prometheus format
    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::new();

        // Total requests
        output.push_str("# HELP integration_requests_total Total number of integration requests\n");
        output.push_str("# TYPE integration_requests_total counter\n");
        output.push_str(&format!(
            "integration_requests_total {}\n\n",
            self.total_requests
        ));

        // Successful requests
        output.push_str("# HELP integration_requests_successful_total Total number of successful integration requests\n");
        output.push_str("# TYPE integration_requests_successful_total counter\n");
        output.push_str(&format!(
            "integration_requests_successful_total {}\n\n",
            self.successful_requests
        ));

        // Failed requests
        output.push_str("# HELP integration_requests_failed_total Total number of failed integration requests\n");
        output.push_str("# TYPE integration_requests_failed_total counter\n");
        output.push_str(&format!(
            "integration_requests_failed_total {}\n\n",
            self.failed_requests
        ));

        // Average response time
        output.push_str(
            "# HELP integration_response_time_seconds Average response time in seconds\n",
        );
        output.push_str("# TYPE integration_response_time_seconds gauge\n");
        output.push_str(&format!(
            "integration_response_time_seconds {}\n\n",
            self.avg_response_time_ms / 1000.0
        ));

        // Requests per second
        output.push_str("# HELP integration_requests_per_second Current requests per second\n");
        output.push_str("# TYPE integration_requests_per_second gauge\n");
        output.push_str(&format!(
            "integration_requests_per_second {}\n\n",
            self.request_rate.current_rate()
        ));

        // Per-integration metrics
        for (integration_type, &count) in &self.requests_by_integration {
            let integration_name = integration_type.as_str();
            output.push_str(&format!(
                "integration_requests_by_type{{integration=\"{}\"}} {}\n",
                integration_name, count
            ));
        }
        output.push('\n');

        // Error metrics
        for (error_type, &count) in &self.errors_by_type {
            output.push_str(&format!(
                "integration_errors_by_type{{error_type=\"{}\"}} {}\n",
                error_type, count
            ));
        }

        output
    }
}

impl ResponseTimeHistogram {
    /// Create a new response time histogram
    pub fn new() -> Self {
        Self {
            under_10ms: 0,
            ms_10_to_50: 0,
            ms_50_to_100: 0,
            ms_100_to_500: 0,
            ms_500_to_1000: 0,
            over_1000ms: 0,
        }
    }

    /// Record a response time
    pub fn record(&mut self, response_time_ms: u64) {
        match response_time_ms {
            0..=9 => self.under_10ms += 1,
            10..=49 => self.ms_10_to_50 += 1,
            50..=99 => self.ms_50_to_100 += 1,
            100..=499 => self.ms_100_to_500 += 1,
            500..=999 => self.ms_500_to_1000 += 1,
            _ => self.over_1000ms += 1,
        }
    }

    /// Get total number of requests
    pub fn total(&self) -> u64 {
        self.under_10ms
            + self.ms_10_to_50
            + self.ms_50_to_100
            + self.ms_100_to_500
            + self.ms_500_to_1000
            + self.over_1000ms
    }

    /// Calculate 95th percentile (approximate)
    pub fn percentile_95(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }

        let p95_index = (total as f64 * 0.95) as u64;
        let mut cumulative = 0;

        if cumulative + self.under_10ms >= p95_index {
            return 9.5;
        }
        cumulative += self.under_10ms;

        if cumulative + self.ms_10_to_50 >= p95_index {
            return 49.5;
        }
        cumulative += self.ms_10_to_50;

        if cumulative + self.ms_50_to_100 >= p95_index {
            return 99.5;
        }
        cumulative += self.ms_50_to_100;

        if cumulative + self.ms_100_to_500 >= p95_index {
            return 499.5;
        }
        cumulative += self.ms_100_to_500;

        if cumulative + self.ms_500_to_1000 >= p95_index {
            return 999.5;
        }

        1500.0 // Approximate for over 1000ms bucket
    }
}

impl Default for ResponseTimeHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestRateTracker {
    /// Create a new request rate tracker
    pub fn new() -> Self {
        Self {
            requests_per_second: 0.0,
            peak_requests_per_second: 0.0,
            recent_requests: Vec::new(),
        }
    }

    /// Record a new request
    pub fn record_request(&mut self) {
        let now = Utc::now();
        self.recent_requests.push(now);

        // Keep only requests from the last minute for rate calculation
        let one_minute_ago = now - chrono::Duration::minutes(1);
        self.recent_requests
            .retain(|&timestamp| timestamp > one_minute_ago);

        // Calculate current rate (requests per second)
        self.requests_per_second = self.recent_requests.len() as f64 / 60.0;

        // Update peak rate
        if self.requests_per_second > self.peak_requests_per_second {
            self.peak_requests_per_second = self.requests_per_second;
        }
    }

    /// Get current requests per second
    pub fn current_rate(&self) -> f64 {
        self.requests_per_second
    }

    /// Get peak requests per second
    pub fn peak_rate(&self) -> f64 {
        self.peak_requests_per_second
    }
}

impl Default for RequestRateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicMetrics {
    /// Create new atomic metrics
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
        }
    }

    /// Record a successful request atomically
    pub fn record_success(&self, response_time_ms: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms
            .fetch_add(response_time_ms, Ordering::Relaxed);
    }

    /// Record a failed request atomically
    pub fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current values
    pub fn get_values(&self) -> (u64, u64, u64, u64) {
        (
            self.total_requests.load(Ordering::Relaxed),
            self.successful_requests.load(Ordering::Relaxed),
            self.failed_requests.load(Ordering::Relaxed),
            self.total_response_time_ms.load(Ordering::Relaxed),
        )
    }

    /// Calculate average response time
    pub fn avg_response_time(&self) -> f64 {
        let successful = self.successful_requests.load(Ordering::Relaxed);
        if successful == 0 {
            0.0
        } else {
            self.total_response_time_ms.load(Ordering::Relaxed) as f64 / successful as f64
        }
    }
}

impl Default for AtomicMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = IntegrationMetrics::new();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.avg_response_time_ms, 0.0);
    }

    #[test]
    fn test_record_success() {
        let mut metrics = IntegrationMetrics::new();
        metrics.record_success(IntegrationType::Zapier, 100);

        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.avg_response_time_ms, 100.0);
        assert_eq!(metrics.success_rate(), 100.0);
        assert_eq!(metrics.error_rate(), 0.0);
    }

    #[test]
    fn test_record_failure() {
        let mut metrics = IntegrationMetrics::new();
        metrics.record_failure(IntegrationType::Slack, "authentication_error");

        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.error_rate(), 100.0);
        assert_eq!(metrics.errors_by_type.get("authentication_error"), Some(&1));
    }

    #[test]
    fn test_mixed_requests() {
        let mut metrics = IntegrationMetrics::new();

        metrics.record_success(IntegrationType::Zapier, 50);
        metrics.record_success(IntegrationType::Slack, 150);
        metrics.record_failure(IntegrationType::GitHub, "rate_limit");

        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.avg_response_time_ms, 100.0); // (50 + 150) / 2
        assert_eq!(metrics.success_rate(), 66.66666666666666);
        assert_eq!(metrics.error_rate(), 33.33333333333333);
    }

    #[test]
    fn test_response_time_histogram() {
        let mut histogram = ResponseTimeHistogram::new();

        histogram.record(5); // under_10ms
        histogram.record(25); // ms_10_to_50
        histogram.record(75); // ms_50_to_100
        histogram.record(250); // ms_100_to_500
        histogram.record(750); // ms_500_to_1000
        histogram.record(1500); // over_1000ms

        assert_eq!(histogram.under_10ms, 1);
        assert_eq!(histogram.ms_10_to_50, 1);
        assert_eq!(histogram.ms_50_to_100, 1);
        assert_eq!(histogram.ms_100_to_500, 1);
        assert_eq!(histogram.ms_500_to_1000, 1);
        assert_eq!(histogram.over_1000ms, 1);
        assert_eq!(histogram.total(), 6);
    }

    #[test]
    fn test_request_rate_tracker() {
        let mut tracker = RequestRateTracker::new();

        // Record some requests
        tracker.record_request();
        tracker.record_request();
        tracker.record_request();

        assert!(tracker.current_rate() > 0.0);
        assert_eq!(tracker.peak_rate(), tracker.current_rate());
    }

    #[test]
    fn test_atomic_metrics() {
        let metrics = AtomicMetrics::new();

        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_failure();

        let (total, successful, failed, total_time) = metrics.get_values();
        assert_eq!(total, 3);
        assert_eq!(successful, 2);
        assert_eq!(failed, 1);
        assert_eq!(total_time, 300);
        assert_eq!(metrics.avg_response_time(), 150.0);
    }

    #[test]
    fn test_prometheus_format() {
        let mut metrics = IntegrationMetrics::new();
        metrics.record_success(IntegrationType::Zapier, 100);
        metrics.record_failure(IntegrationType::Slack, "test_error");

        let prometheus_output = metrics.to_prometheus_format();

        assert!(prometheus_output.contains("integration_requests_total 2"));
        assert!(prometheus_output.contains("integration_requests_successful_total 1"));
        assert!(prometheus_output.contains("integration_requests_failed_total 1"));
        assert!(prometheus_output.contains("integration=\"zapier\""));
        assert!(prometheus_output.contains("error_type=\"test_error\""));
    }

    #[test]
    fn test_metrics_snapshot() {
        let mut metrics = IntegrationMetrics::new();
        metrics.record_success(IntegrationType::Zapier, 50);
        metrics.record_success(IntegrationType::Slack, 150);

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.service, "integration-service");
        assert_eq!(snapshot.total_requests, 2);
        assert_eq!(snapshot.success_rate, 100.0);
        assert_eq!(snapshot.error_rate, 0.0);
        assert_eq!(snapshot.avg_response_time_ms, 100.0);
        assert!(!snapshot.integration_stats.is_empty());
    }

    #[test]
    fn test_metrics_reset() {
        let mut metrics = IntegrationMetrics::new();
        metrics.record_success(IntegrationType::Zapier, 100);
        metrics.record_failure(IntegrationType::Slack, "error");

        assert_ne!(metrics.total_requests, 0);

        metrics.reset();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.avg_response_time_ms, 0.0);
        assert!(metrics.requests_by_integration.is_empty());
        assert!(metrics.errors_by_type.is_empty());
    }
}
