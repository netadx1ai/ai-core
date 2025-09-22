//! Metrics Module
//!
//! This module provides metrics collection and aggregation for the MCP Manager Service.
//! It integrates with the telemetry system to provide detailed operational metrics.

use crate::server::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info};

/// Metrics summary response
#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    /// Service metrics
    pub service: ServiceMetrics,
    /// Server metrics
    pub servers: ServerMetrics,
    /// Request metrics
    pub requests: RequestMetrics,
    /// Health check metrics
    pub health_checks: HealthCheckMetrics,
    /// Load balancer metrics
    pub load_balancer: LoadBalancerMetrics,
}

/// Service-level metrics
#[derive(Debug, Serialize)]
pub struct ServiceMetrics {
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: Option<u64>,
    /// CPU usage percentage
    pub cpu_usage_percent: Option<f64>,
    /// Active goroutines/tasks
    pub active_tasks: u32,
}

/// Server-related metrics
#[derive(Debug, Serialize)]
pub struct ServerMetrics {
    /// Total registered servers
    pub total_servers: u64,
    /// Healthy servers
    pub healthy_servers: u64,
    /// Unhealthy servers
    pub unhealthy_servers: u64,
    /// Failed servers
    pub failed_servers: u64,
    /// Servers by type
    pub servers_by_type: HashMap<String, u64>,
}

/// Request-related metrics
#[derive(Debug, Serialize)]
pub struct RequestMetrics {
    /// Total requests processed
    pub total_requests: u64,
    /// Total errors
    pub total_errors: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Requests per second (current rate)
    pub requests_per_second: f64,
    /// Error rate percentage
    pub error_rate_percent: f64,
}

/// Health check metrics
#[derive(Debug, Serialize)]
pub struct HealthCheckMetrics {
    /// Total health checks performed
    pub total_checks: u64,
    /// Successful health checks
    pub successful_checks: u64,
    /// Failed health checks
    pub failed_checks: u64,
    /// Average health check duration in milliseconds
    pub avg_check_duration_ms: f64,
}

/// Load balancer metrics
#[derive(Debug, Serialize)]
pub struct LoadBalancerMetrics {
    /// Total requests routed
    pub total_routed_requests: u64,
    /// Active connections
    pub active_connections: u32,
    /// Current strategy
    pub current_strategy: String,
    /// Request distribution by server
    pub request_distribution: HashMap<String, u64>,
}

/// Custom metrics query request
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    /// Metric name pattern
    pub metric_pattern: Option<String>,
    /// Time range start (ISO 8601)
    pub start_time: Option<String>,
    /// Time range end (ISO 8601)
    pub end_time: Option<String>,
    /// Aggregation type
    pub aggregation: Option<String>,
    /// Tags filter
    pub tags: Option<HashMap<String, String>>,
}

/// Get metrics summary
pub async fn get_metrics_summary(
    State(state): State<AppState>,
) -> Result<Json<MetricsSummary>, StatusCode> {
    // Get registry statistics
    let registry_stats = state.registry().get_statistics().await;

    // Get load balancer statistics
    let lb_stats = state.load_balancer().get_statistics().await;

    // Get health monitor statistics
    let health_stats = state.health_monitor().get_statistics().await;

    let metrics = MetricsSummary {
        service: ServiceMetrics {
            uptime_seconds: 0,        // TODO: Track actual uptime
            memory_usage_bytes: None, // TODO: Get from system
            cpu_usage_percent: None,  // TODO: Get from system
            active_tasks: 0,          // TODO: Track active tasks
        },
        servers: ServerMetrics {
            total_servers: registry_stats.total_servers as u64,
            healthy_servers: registry_stats
                .status_counts
                .get(&crate::models::ServerStatus::Running)
                .unwrap_or(&0)
                .clone() as u64,
            unhealthy_servers: registry_stats
                .status_counts
                .get(&crate::models::ServerStatus::Unhealthy)
                .unwrap_or(&0)
                .clone() as u64,
            failed_servers: registry_stats
                .status_counts
                .get(&crate::models::ServerStatus::Failed)
                .unwrap_or(&0)
                .clone() as u64,
            servers_by_type: registry_stats
                .type_counts
                .iter()
                .map(|(k, v)| (k.clone(), *v as u64))
                .collect(),
        },
        requests: RequestMetrics {
            total_requests: lb_stats.total_requests,
            total_errors: lb_stats.total_errors,
            avg_response_time_ms: lb_stats.avg_response_time_ms,
            requests_per_second: 0.0, // TODO: Calculate current rate
            error_rate_percent: if lb_stats.total_requests > 0 {
                (lb_stats.total_errors as f64 / lb_stats.total_requests as f64) * 100.0
            } else {
                0.0
            },
        },
        health_checks: HealthCheckMetrics {
            total_checks: health_stats.total_checks,
            successful_checks: health_stats.successful_checks,
            failed_checks: health_stats.failed_checks,
            avg_check_duration_ms: health_stats.avg_response_time_ms,
        },
        load_balancer: LoadBalancerMetrics {
            total_routed_requests: lb_stats.total_requests,
            active_connections: lb_stats.active_connections,
            current_strategy: "round_robin".to_string(), // TODO: Get actual strategy
            request_distribution: lb_stats
                .requests_by_server
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
        },
    };

    info!("Metrics summary generated");
    Ok(Json(metrics))
}

/// Query custom metrics
pub async fn query_metrics(
    State(_state): State<AppState>,
    Json(_query): Json<MetricsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Implement custom metrics querying
    error!("Custom metrics querying not yet implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Reset metrics counters
pub async fn reset_metrics(State(_state): State<AppState>) -> Result<StatusCode, StatusCode> {
    // TODO: Implement metrics reset
    error!("Metrics reset not yet implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_summary_serialization() {
        let metrics = MetricsSummary {
            service: ServiceMetrics {
                uptime_seconds: 3600,
                memory_usage_bytes: Some(1024 * 1024 * 100), // 100MB
                cpu_usage_percent: Some(25.5),
                active_tasks: 10,
            },
            servers: ServerMetrics {
                total_servers: 5,
                healthy_servers: 4,
                unhealthy_servers: 1,
                failed_servers: 0,
                servers_by_type: [("api".to_string(), 3), ("database".to_string(), 2)]
                    .iter()
                    .cloned()
                    .collect(),
            },
            requests: RequestMetrics {
                total_requests: 1000,
                total_errors: 10,
                avg_response_time_ms: 150.5,
                requests_per_second: 10.5,
                error_rate_percent: 1.0,
            },
            health_checks: HealthCheckMetrics {
                total_checks: 500,
                successful_checks: 490,
                failed_checks: 10,
                avg_check_duration_ms: 50.0,
            },
            load_balancer: LoadBalancerMetrics {
                total_routed_requests: 950,
                active_connections: 25,
                current_strategy: "round_robin".to_string(),
                request_distribution: HashMap::new(),
            },
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("uptime_seconds"));
        assert!(json.contains("total_servers"));
    }

    #[test]
    fn test_metrics_query_deserialization() {
        let query_json = r#"
        {
            "metric_pattern": "request_*",
            "start_time": "2024-01-01T00:00:00Z",
            "end_time": "2024-01-02T00:00:00Z",
            "aggregation": "sum",
            "tags": {"service": "mcp-manager"}
        }
        "#;

        let query: MetricsQuery = serde_json::from_str(query_json).unwrap();
        assert_eq!(query.metric_pattern, Some("request_*".to_string()));
        assert_eq!(query.aggregation, Some("sum".to_string()));
        assert!(query.tags.is_some());
    }
}
