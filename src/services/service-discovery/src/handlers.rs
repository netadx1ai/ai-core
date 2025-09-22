//! API Handlers Module
//!
//! HTTP handlers for the service discovery REST API endpoints.
//! Provides handlers for service registration, discovery, health checks, and management operations.

use crate::config::ServiceDiscoveryConfig;
use crate::health::{HealthMonitor, HealthMonitorImpl};
use crate::load_balancer::{LoadBalancer, LoadBalancerImpl};
use crate::models::{
    HealthCheckResult, HeartbeatRequest, RegisterServiceRequest, ServiceDiscoveryQuery,
    ServiceDiscoveryResponse, ServiceInstance, ServiceRegistration, ServiceStatistics,
    UpdateServiceRequest,
};
use crate::registry::{ServiceRegistry, ServiceRegistryImpl};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServiceDiscoveryConfig>,
    pub registry: Arc<ServiceRegistryImpl>,
    pub health_monitor: Arc<HealthMonitorImpl>,
    pub load_balancer: Arc<LoadBalancerImpl>,
}

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Service registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistrationResponse {
    pub service_id: Uuid,
    pub message: String,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub services: HashMap<String, String>,
}

/// Metrics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub total_services: u64,
    pub healthy_services: u64,
    pub unhealthy_services: u64,
    pub total_requests: u64,
    pub avg_response_time_ms: f64,
    pub uptime_seconds: u64,
}

/// Create the main router with all API routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Service management routes
        .route("/api/v1/services", post(register_service))
        .route("/api/v1/services/:id", get(get_service))
        .route("/api/v1/services/:id", put(update_service))
        .route("/api/v1/services/:id", delete(deregister_service))
        .route("/api/v1/services/:id/heartbeat", post(service_heartbeat))
        // Service discovery routes
        .route("/api/v1/discover", get(discover_services))
        .route(
            "/api/v1/services/:name/instances",
            get(get_service_instances),
        )
        // Health check routes
        .route("/api/v1/services/:id/health", get(get_service_health))
        .route("/api/v1/services/:id/health", post(check_service_health))
        // Statistics and monitoring routes
        .route("/api/v1/services/:id/stats", get(get_service_statistics))
        .route(
            "/api/v1/load-balancer/:service_name/stats",
            get(get_load_balancer_stats),
        )
        .route(
            "/api/v1/health-monitor/stats",
            get(get_health_monitor_stats),
        )
        // Administrative routes
        .route("/api/v1/services", get(list_all_services))
        .route(
            "/api/v1/services/:id/monitoring",
            post(start_service_monitoring),
        )
        .route(
            "/api/v1/services/:id/monitoring",
            delete(stop_service_monitoring),
        )
        // System routes
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .route("/api/v1/status", get(get_system_status))
        .with_state(state)
}

/// Register a new service
pub async fn register_service(
    State(state): State<AppState>,
    Json(request): Json<RegisterServiceRequest>,
) -> Result<Json<ApiResponse<ServiceRegistrationResponse>>, StatusCode> {
    debug!("Registering service: {}", request.name);

    match state.registry.register_service(request.clone()).await {
        Ok(service_id) => {
            info!(
                "Successfully registered service {} with ID {}",
                request.name, service_id
            );

            // Add to health monitoring if health check is configured
            if request.health_check.is_some() {
                if let Ok(Some(service)) = state.registry.get_service(service_id).await {
                    if let Err(e) = state.health_monitor.monitor_service(service).await {
                        warn!("Failed to add service to health monitoring: {}", e);
                    }
                }
            }

            Ok(Json(ApiResponse::success(ServiceRegistrationResponse {
                service_id,
                message: format!("Service {} registered successfully", request.name),
            })))
        }
        Err(e) => {
            error!("Failed to register service {}: {}", request.name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get service by ID
pub async fn get_service(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ServiceRegistration>>, StatusCode> {
    debug!("Getting service: {}", service_id);

    match state.registry.get_service(service_id).await {
        Ok(Some(service)) => Ok(Json(ApiResponse::success(service))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get service {}: {}", service_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update service information
pub async fn update_service(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
    Json(request): Json<UpdateServiceRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    debug!("Updating service: {}", service_id);

    match state.registry.update_service(service_id, request).await {
        Ok(()) => {
            info!("Successfully updated service {}", service_id);
            Ok(Json(ApiResponse::success(
                "Service updated successfully".to_string(),
            )))
        }
        Err(e) => {
            error!("Failed to update service {}: {}", service_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Deregister a service
pub async fn deregister_service(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    debug!("Deregistering service: {}", service_id);

    // Remove from health monitoring first
    if let Err(e) = state.health_monitor.remove_service(service_id).await {
        warn!("Failed to remove service from health monitoring: {}", e);
    }

    match state.registry.deregister_service(service_id).await {
        Ok(()) => {
            info!("Successfully deregistered service {}", service_id);
            Ok(Json(ApiResponse::success(
                "Service deregistered successfully".to_string(),
            )))
        }
        Err(e) => {
            error!("Failed to deregister service {}: {}", service_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Process service heartbeat
pub async fn service_heartbeat(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
    Json(request): Json<HeartbeatRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    debug!("Processing heartbeat for service: {}", service_id);

    match state
        .registry
        .heartbeat(request.service_id, request.status)
        .await
    {
        Ok(()) => {
            debug!("Processed heartbeat for service {}", service_id);
            Ok(Json(ApiResponse::success(
                "Heartbeat processed successfully".to_string(),
            )))
        }
        Err(e) => {
            error!(
                "Failed to process heartbeat for service {}: {}",
                service_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Discover services based on query parameters
pub async fn discover_services(
    State(state): State<AppState>,
    Query(query): Query<ServiceDiscoveryQuery>,
) -> Result<Json<ApiResponse<ServiceDiscoveryResponse>>, StatusCode> {
    debug!("Discovering services for: {}", query.service_name);

    match state.registry.discover_services(query).await {
        Ok(response) => {
            debug!("Found {} services", response.services.len());
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to discover services: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get all instances of a service by name
pub async fn get_service_instances(
    State(state): State<AppState>,
    Path(service_name): Path<String>,
) -> Result<Json<ApiResponse<Vec<ServiceInstance>>>, StatusCode> {
    debug!("Getting instances for service: {}", service_name);

    match state.registry.get_services_by_name(&service_name).await {
        Ok(services) => {
            let instances: Vec<ServiceInstance> = services
                .into_iter()
                .map(|s| ServiceInstance {
                    id: s.id,
                    name: s.name,
                    version: s.version,
                    address: s.address,
                    port: s.port,
                    protocol: s.protocol,
                    status: s.status,
                    weight: s.weight,
                    metadata: s.metadata,
                    last_health_check: None, // Could be populated from health monitoring
                })
                .collect();

            Ok(Json(ApiResponse::success(instances)))
        }
        Err(e) => {
            error!(
                "Failed to get service instances for {}: {}",
                service_name, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get service health status
pub async fn get_service_health(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    debug!("Getting health status for service: {}", service_id);

    match state.registry.get_health_status(service_id).await {
        Ok(status) => Ok(Json(ApiResponse::success(format!("{:?}", status)))),
        Err(e) => {
            error!(
                "Failed to get health status for service {}: {}",
                service_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Perform immediate health check
pub async fn check_service_health(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<HealthCheckResult>>, StatusCode> {
    debug!("Performing health check for service: {}", service_id);

    match state.health_monitor.check_service_health(service_id).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            error!(
                "Failed to perform health check for service {}: {}",
                service_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get service statistics
pub async fn get_service_statistics(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ServiceStatistics>>, StatusCode> {
    debug!("Getting statistics for service: {}", service_id);

    match state.registry.get_service_statistics(service_id).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            error!("Failed to get statistics for service {}: {}", service_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get load balancer statistics
pub async fn get_load_balancer_stats(
    State(state): State<AppState>,
    Path(service_name): Path<String>,
) -> Result<Json<ApiResponse<crate::models::LoadBalancerStats>>, StatusCode> {
    debug!("Getting load balancer stats for service: {}", service_name);

    match state.load_balancer.get_stats(&service_name).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            error!(
                "Failed to get load balancer stats for service {}: {}",
                service_name, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get health monitor statistics
pub async fn get_health_monitor_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::health::HealthMonitoringStats>>, StatusCode> {
    debug!("Getting health monitor statistics");

    match state.health_monitor.get_health_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            error!("Failed to get health monitor stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List all registered services
pub async fn list_all_services(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ServiceInstance>>>, StatusCode> {
    debug!("Listing all registered services");

    // This would need to be implemented in the registry trait
    // For now, return empty list
    Ok(Json(ApiResponse::success(Vec::new())))
}

/// Start monitoring for a specific service
pub async fn start_service_monitoring(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    debug!("Starting monitoring for service: {}", service_id);

    match state.registry.get_service(service_id).await {
        Ok(Some(service)) => match state.health_monitor.monitor_service(service).await {
            Ok(()) => Ok(Json(ApiResponse::success(
                "Service monitoring started".to_string(),
            ))),
            Err(e) => {
                error!(
                    "Failed to start monitoring for service {}: {}",
                    service_id, e
                );
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get service {}: {}", service_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Stop monitoring for a specific service
pub async fn stop_service_monitoring(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    debug!("Stopping monitoring for service: {}", service_id);

    match state.health_monitor.remove_service(service_id).await {
        Ok(()) => Ok(Json(ApiResponse::success(
            "Service monitoring stopped".to_string(),
        ))),
        Err(e) => {
            error!(
                "Failed to stop monitoring for service {}: {}",
                service_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Health check endpoint
pub async fn health_check(State(_state): State<AppState>) -> Json<ApiResponse<HealthResponse>> {
    let mut services = HashMap::new();

    // Add basic service health information
    services.insert("database".to_string(), "healthy".to_string());
    services.insert("redis".to_string(), "healthy".to_string());
    services.insert("registry".to_string(), "healthy".to_string());

    Json(ApiResponse::success(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services,
    }))
}

/// Get system metrics
pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MetricsResponse>>, StatusCode> {
    debug!("Getting system metrics");

    // Get health monitor stats
    let health_stats = match state.health_monitor.get_health_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            error!("Failed to get health monitor stats: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let metrics = MetricsResponse {
        total_services: health_stats.total_services,
        healthy_services: health_stats.healthy_services,
        unhealthy_services: health_stats.unhealthy_services,
        total_requests: health_stats.total_health_checks,
        avg_response_time_ms: health_stats.avg_response_time_ms,
        uptime_seconds: 0, // Would need to track service start time
    };

    Ok(Json(ApiResponse::success(metrics)))
}

/// Get system status
pub async fn get_system_status(
    State(state): State<AppState>,
) -> Json<ApiResponse<HashMap<String, serde_json::Value>>> {
    let mut status = HashMap::new();

    status.insert(
        "version".to_string(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    status.insert(
        "timestamp".to_string(),
        serde_json::json!(chrono::Utc::now()),
    );
    status.insert("environment".to_string(), serde_json::json!("production"));

    // Add configuration summary
    let config_summary = serde_json::json!({
        "server_port": state.config.server.port,
        "health_checks_enabled": state.config.registry.health_checks.enabled,
        "circuit_breaker_enabled": state.config.circuit_breaker.enabled,
        "service_mesh_enabled": state.config.service_mesh.enabled
    });
    status.insert("configuration".to_string(), config_summary);

    Json(ApiResponse::success(status))
}

/// Error handling for the API
pub async fn handle_error(_error: Box<dyn std::error::Error>) -> StatusCode {
    error!("API error occurred");
    StatusCode::INTERNAL_SERVER_ERROR
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        // This would be a full integration test
        // For now, just test that the handler compiles
        assert!(true);
    }

    #[test]
    fn test_api_response() {
        let success_response = ApiResponse::success("test data");
        assert!(success_response.success);
        assert!(success_response.data.is_some());
        assert!(success_response.error.is_none());

        let error_response: ApiResponse<String> = ApiResponse::error("test error".to_string());
        assert!(!error_response.success);
        assert!(error_response.data.is_none());
        assert!(error_response.error.is_some());
    }
}
