//! Health Check Handlers
//!
//! This module provides HTTP handlers for health checking functionality,
//! including basic health checks, detailed health reports, and server-specific
//! health monitoring endpoints.

use crate::{
    models::{HealthCheck, HealthStatus},
    server::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;

/// Basic health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Detailed health check response
#[derive(Debug, Serialize)]
pub struct DetailedHealthResponse {
    /// Service status
    pub status: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Registry statistics
    pub registry: RegistryHealth,
    /// Health monitor statistics
    pub health_monitor: HealthMonitorStats,
    /// Load balancer statistics
    pub load_balancer: LoadBalancerHealth,
    /// System resources
    pub system: SystemHealth,
}

/// Registry health information
#[derive(Debug, Serialize)]
pub struct RegistryHealth {
    /// Total servers
    pub total_servers: usize,
    /// Healthy servers
    pub healthy_servers: usize,
    /// Unhealthy servers
    pub unhealthy_servers: usize,
    /// Failed servers
    pub failed_servers: usize,
    /// Status distribution
    pub status_distribution: HashMap<String, usize>,
}

/// Health monitor statistics
#[derive(Debug, Serialize)]
pub struct HealthMonitorStats {
    /// Total health checks performed
    pub total_checks: u64,
    /// Successful checks
    pub successful_checks: u64,
    /// Failed checks
    pub failed_checks: u64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Last check timestamp
    pub last_updated: DateTime<Utc>,
}

/// Load balancer health information
#[derive(Debug, Serialize)]
pub struct LoadBalancerHealth {
    /// Total requests processed
    pub total_requests: u64,
    /// Total errors
    pub total_errors: u64,
    /// Active connections
    pub active_connections: u32,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Error rate percentage
    pub error_rate: f64,
}

/// System health information
#[derive(Debug, Serialize)]
pub struct SystemHealth {
    /// Memory usage in bytes
    pub memory_usage_bytes: Option<u64>,
    /// CPU usage percentage
    pub cpu_usage_percent: Option<f64>,
    /// Available disk space in bytes
    pub disk_available_bytes: Option<u64>,
    /// Network connectivity status
    pub network_status: String,
}

/// Server health check response
#[derive(Debug, Serialize)]
pub struct ServerHealthResponse {
    /// Server ID
    pub server_id: Uuid,
    /// Health check result
    pub health_check: Option<HealthCheck>,
    /// Server status
    pub server_status: String,
    /// Last health check timestamp
    pub last_check: Option<DateTime<Utc>>,
}

/// Basic health check endpoint
///
/// Returns a simple health status for the MCP Manager service.
/// This endpoint is typically used by load balancers and monitoring systems.
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let registry_count = state.registry().count().await;
    let status = if registry_count > 0 {
        "healthy"
    } else {
        "starting"
    };

    let response = HealthResponse {
        status: status.to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
    };

    info!("Health check performed, status: {}", status);
    Ok(Json(response))
}

/// Detailed health check endpoint
///
/// Returns comprehensive health information including registry statistics,
/// health monitor data, load balancer status, and system metrics.
pub async fn detailed_health(
    State(state): State<AppState>,
) -> Result<Json<DetailedHealthResponse>, StatusCode> {
    // Get registry statistics
    let registry_stats = state.registry().get_statistics().await;
    let healthy_count = registry_stats
        .status_counts
        .get(&crate::models::ServerStatus::Running)
        .unwrap_or(&0);
    let unhealthy_count = registry_stats
        .status_counts
        .get(&crate::models::ServerStatus::Unhealthy)
        .unwrap_or(&0);
    let failed_count = registry_stats
        .status_counts
        .get(&crate::models::ServerStatus::Failed)
        .unwrap_or(&0);

    let registry_health = RegistryHealth {
        total_servers: registry_stats.total_servers,
        healthy_servers: *healthy_count,
        unhealthy_servers: *unhealthy_count,
        failed_servers: *failed_count,
        status_distribution: registry_stats
            .status_counts
            .iter()
            .map(|(status, count)| (status.to_string(), *count))
            .collect(),
    };

    // Get health monitor statistics
    let health_stats = state.health_monitor().get_statistics().await;
    let health_monitor_stats = HealthMonitorStats {
        total_checks: health_stats.total_checks,
        successful_checks: health_stats.successful_checks,
        failed_checks: health_stats.failed_checks,
        avg_response_time_ms: health_stats.avg_response_time_ms,
        last_updated: health_stats.last_updated,
    };

    // Get load balancer statistics
    let lb_stats = state.load_balancer().get_statistics().await;
    let error_rate = if lb_stats.total_requests > 0 {
        (lb_stats.total_errors as f64 / lb_stats.total_requests as f64) * 100.0
    } else {
        0.0
    };

    let load_balancer_health = LoadBalancerHealth {
        total_requests: lb_stats.total_requests,
        total_errors: lb_stats.total_errors,
        active_connections: lb_stats.active_connections,
        avg_response_time_ms: lb_stats.avg_response_time_ms,
        error_rate,
    };

    // Get system health (simplified for now)
    let system_health = SystemHealth {
        memory_usage_bytes: None, // TODO: Implement actual system metrics
        cpu_usage_percent: None,
        disk_available_bytes: None,
        network_status: "ok".to_string(),
    };

    // Determine overall status
    let overall_status = if registry_health.failed_servers > registry_health.total_servers / 2 {
        "unhealthy"
    } else if registry_health.unhealthy_servers > 0 {
        "degraded"
    } else {
        "healthy"
    };

    let response = DetailedHealthResponse {
        status: overall_status.to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
        registry: registry_health,
        health_monitor: health_monitor_stats,
        load_balancer: load_balancer_health,
        system: system_health,
    };

    info!(
        "Detailed health check performed, status: {}",
        overall_status
    );
    Ok(Json(response))
}

/// Get health status for a specific server
///
/// Returns the latest health check result for the specified server.
pub async fn get_server_health(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ServerHealthResponse>, StatusCode> {
    // Check if server exists
    let server = state
        .registry()
        .get(&server_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get health check result
    let health_check = state.health_monitor().get_server_health(&server_id).await;

    let response = ServerHealthResponse {
        server_id,
        health_check,
        server_status: server.status.to_string(),
        last_check: server.last_health_check,
    };

    Ok(Json(response))
}

/// Trigger a manual health check for a specific server
///
/// Forces an immediate health check for the specified server and returns the result.
pub async fn check_server_health(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<HealthCheck>, StatusCode> {
    // Check if server exists
    let _server = state
        .registry()
        .get(&server_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Trigger health check
    match state.health_monitor().check_server(&server_id).await {
        Ok(health_check) => {
            info!(server_id = %server_id, "Manual health check completed");
            Ok(Json(health_check))
        }
        Err(e) => {
            error!(server_id = %server_id, error = %e, "Manual health check failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get health status for all servers
///
/// Returns health status information for all registered servers.
pub async fn get_all_server_health(
    State(state): State<AppState>,
) -> Result<Json<HashMap<Uuid, HealthCheck>>, StatusCode> {
    let health_status = state.health_monitor().get_all_health_status().await;
    Ok(Json(health_status))
}

/// Health check summary for multiple servers
#[derive(Debug, Serialize)]
pub struct HealthSummary {
    /// Total servers checked
    pub total_servers: usize,
    /// Healthy servers
    pub healthy_count: usize,
    /// Unhealthy servers
    pub unhealthy_count: usize,
    /// Degraded servers
    pub degraded_count: usize,
    /// Unknown status servers
    pub unknown_count: usize,
    /// Overall health percentage
    pub health_percentage: f64,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Get health summary for all servers
///
/// Returns a summary of health status across all registered servers.
pub async fn get_health_summary(
    State(state): State<AppState>,
) -> Result<Json<HealthSummary>, StatusCode> {
    let health_status = state.health_monitor().get_all_health_status().await;

    let total_servers = health_status.len();
    let mut healthy_count = 0;
    let mut unhealthy_count = 0;
    let mut degraded_count = 0;
    let mut unknown_count = 0;

    for (_, health_check) in &health_status {
        match health_check.status {
            HealthStatus::Healthy => healthy_count += 1,
            HealthStatus::Unhealthy => unhealthy_count += 1,
            HealthStatus::Degraded => degraded_count += 1,
            HealthStatus::Unknown => unknown_count += 1,
        }
    }

    let health_percentage = if total_servers > 0 {
        (healthy_count as f64 / total_servers as f64) * 100.0
    } else {
        0.0
    };

    let summary = HealthSummary {
        total_servers,
        healthy_count,
        unhealthy_count,
        degraded_count,
        unknown_count,
        health_percentage,
        last_updated: Utc::now(),
    };

    Ok(Json(summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: Utc::now(),
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_health_summary_calculation() {
        let summary = HealthSummary {
            total_servers: 10,
            healthy_count: 8,
            unhealthy_count: 1,
            degraded_count: 1,
            unknown_count: 0,
            health_percentage: 80.0,
            last_updated: Utc::now(),
        };

        assert_eq!(summary.health_percentage, 80.0);
        assert_eq!(
            summary.total_servers,
            summary.healthy_count
                + summary.unhealthy_count
                + summary.degraded_count
                + summary.unknown_count
        );
    }
}
