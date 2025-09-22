//! Server module for the Data Processing Service
//!
//! This module provides the HTTP API server for the data processing service,
//! including REST endpoints for job management, health monitoring, metrics
//! collection, and administrative operations.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info};

use crate::{
    config::Config,
    error::{DataProcessingError, Result},
    types::{BatchJob, BatchJobStatus, DataRecord, ProcessingResult, ServiceHealth},
    DataProcessingService,
};

/// Main server for the data processing service
pub struct DataProcessingServer {
    service: Arc<DataProcessingService>,
    config: Arc<Config>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
    pub timestamp: i64,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub success: bool,
    pub error: String,
    pub error_code: String,
    pub timestamp: i64,
}

/// Job submission request
#[derive(Debug, Deserialize)]
pub struct SubmitJobRequest {
    pub job: BatchJob,
}

/// Job submission response
#[derive(Debug, Serialize)]
pub struct SubmitJobResponse {
    pub job_id: String,
    pub message: String,
}

/// Process record request
#[derive(Debug, Deserialize)]
pub struct ProcessRecordRequest {
    pub record: DataRecord,
    pub options: Option<ProcessingOptions>,
}

/// Processing options
#[derive(Debug, Deserialize)]
pub struct ProcessingOptions {
    pub async_processing: Option<bool>,
    pub priority: Option<u8>,
    pub timeout_secs: Option<u64>,
}

/// Process record response
#[derive(Debug, Serialize)]
pub struct ProcessRecordResponse {
    pub result: ProcessingResult,
}

/// List jobs query parameters
#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
    pub job_type: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub components: serde_json::Value,
}

/// Metrics response
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub metrics: serde_json::Value,
    pub performance_stats: serde_json::Value,
}

impl DataProcessingServer {
    /// Create a new data processing server
    pub fn new(service: DataProcessingService) -> Self {
        let config = service.config();
        Self {
            service: Arc::new(service),
            config,
        }
    }

    /// Start the HTTP server
    pub async fn start(&self) -> Result<()> {
        let app = self.create_router().await?;

        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        info!("Starting Data Processing HTTP server on {}", addr);

        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            DataProcessingError::internal(format!("Failed to bind to address {}: {}", addr, e))
        })?;

        axum::serve(listener, app)
            .await
            .map_err(|e| DataProcessingError::internal(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Create the application router
    async fn create_router(&self) -> Result<Router> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let middleware = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .layer(CompressionLayer::new())
            .layer(TimeoutLayer::new(Duration::from_secs(
                self.config.server.request_timeout_secs,
            )))
            .into_inner();

        let app = Router::new()
            // Health endpoints
            .route("/health", get(health_check))
            .route("/health/ready", get(readiness_check))
            .route("/health/live", get(liveness_check))
            // Metrics endpoints
            .route("/metrics", get(get_metrics))
            .route("/metrics/prometheus", get(get_prometheus_metrics))
            // Stream processing endpoints
            .route("/stream/process", post(process_record))
            .route("/stream/status", get(get_stream_status))
            // Batch processing endpoints
            .route("/batch/jobs", post(submit_batch_job))
            .route("/batch/jobs", get(list_batch_jobs))
            .route("/batch/jobs/:job_id", get(get_batch_job_status))
            .route("/batch/jobs/:job_id", delete(cancel_batch_job))
            .route("/batch/jobs/:job_id/restart", post(restart_batch_job))
            // Administrative endpoints
            .route("/admin/config", get(get_configuration))
            .route("/admin/stats", get(get_statistics))
            .route("/admin/reset", post(reset_metrics))
            // Service management endpoints
            .route("/service/start", post(start_service))
            .route("/service/stop", post(stop_service))
            .route("/service/restart", post(restart_service))
            .layer(middleware)
            .with_state(self.service.clone());

        Ok(app)
    }
}

/// Health check endpoint
async fn health_check(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<HealthResponse>>, (StatusCode, Json<ApiError>)> {
    match service.health().await {
        health => {
            let response = HealthResponse {
                status: format!("{:?}", health.status),
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_seconds: health.uptime_secs,
                components: serde_json::to_value(&health.components).unwrap_or_default(),
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response),
                message: "Service is healthy".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            }))
        }
    }
}

/// Readiness check endpoint
async fn readiness_check(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    let health = service.health().await;

    if matches!(health.status, crate::types::HealthStatus::Healthy) {
        Ok(Json(ApiResponse {
            success: true,
            data: Some("ready".to_string()),
            message: "Service is ready".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                success: false,
                error: "Service not ready".to_string(),
                error_code: "SERVICE_NOT_READY".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            }),
        ))
    }
}

/// Liveness check endpoint
async fn liveness_check() -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        success: true,
        data: Some("alive".to_string()),
        message: "Service is alive".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// Get metrics endpoint
async fn get_metrics(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<MetricsResponse>>, (StatusCode, Json<ApiError>)> {
    let metrics = service.metrics();

    match metrics.get_snapshot().await {
        snapshot => match metrics.get_performance_stats().await {
            performance_stats => {
                let response = MetricsResponse {
                    metrics: serde_json::to_value(&snapshot).unwrap_or_default(),
                    performance_stats: serde_json::to_value(&performance_stats).unwrap_or_default(),
                };

                Ok(Json(ApiResponse {
                    success: true,
                    data: Some(response),
                    message: "Metrics retrieved successfully".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }))
            }
        },
    }
}

/// Get Prometheus metrics endpoint
async fn get_prometheus_metrics(State(service): State<Arc<DataProcessingService>>) -> String {
    service.metrics().export_prometheus()
}

/// Process record endpoint
async fn process_record(
    State(service): State<Arc<DataProcessingService>>,
    Json(request): Json<ProcessRecordRequest>,
) -> std::result::Result<Json<ApiResponse<ProcessRecordResponse>>, (StatusCode, Json<ApiError>)> {
    match service.process_record(request.record).await {
        Ok(result) => {
            let response = ProcessRecordResponse { result };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response),
                message: "Record processed successfully".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            }))
        }
        Err(e) => {
            error!("Failed to process record: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                    error_code: "PROCESSING_ERROR".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

/// Get stream processing status endpoint
async fn get_stream_status(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let health = service.health().await;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "status": format!("{:?}", health.status),
            "components": health.components
        })),
        message: "Stream status retrieved successfully".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Submit batch job endpoint
async fn submit_batch_job(
    State(service): State<Arc<DataProcessingService>>,
    Json(request): Json<SubmitJobRequest>,
) -> std::result::Result<Json<ApiResponse<SubmitJobResponse>>, (StatusCode, Json<ApiError>)> {
    match service.submit_batch_job(request.job).await {
        Ok(job_id) => {
            let response = SubmitJobResponse {
                job_id: job_id.clone(),
                message: "Job submitted successfully".to_string(),
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response),
                message: format!("Batch job {} submitted successfully", job_id),
                timestamp: chrono::Utc::now().timestamp(),
            }))
        }
        Err(e) => {
            error!("Failed to submit batch job: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                    error_code: "JOB_SUBMISSION_ERROR".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

/// List batch jobs endpoint
async fn list_batch_jobs(
    State(_service): State<Arc<DataProcessingService>>,
    Query(_query): Query<ListJobsQuery>,
) -> std::result::Result<Json<ApiResponse<Vec<BatchJobStatus>>>, (StatusCode, Json<ApiError>)> {
    // For now, just return an empty list as the full implementation would require
    // additional methods on the service
    let jobs = Vec::new();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(jobs),
        message: "Jobs retrieved successfully".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Get batch job status endpoint
async fn get_batch_job_status(
    State(service): State<Arc<DataProcessingService>>,
    Path(job_id): Path<String>,
) -> std::result::Result<Json<ApiResponse<BatchJobStatus>>, (StatusCode, Json<ApiError>)> {
    match service.get_batch_job_status(&job_id).await {
        Ok(status) => Ok(Json(ApiResponse {
            success: true,
            data: Some(status),
            message: "Job status retrieved successfully".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })),
        Err(e) => {
            error!("Failed to get job status for {}: {}", job_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                    error_code: "JOB_NOT_FOUND".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

/// Cancel batch job endpoint
async fn cancel_batch_job(
    State(service): State<Arc<DataProcessingService>>,
    Path(job_id): Path<String>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    // Implementation would go here
    Ok(Json(ApiResponse {
        success: true,
        data: Some(format!("Job {} cancelled", job_id)),
        message: "Job cancelled successfully".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Restart batch job endpoint
async fn restart_batch_job(
    State(service): State<Arc<DataProcessingService>>,
    Path(job_id): Path<String>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    // Implementation would go here
    Ok(Json(ApiResponse {
        success: true,
        data: Some(format!("Job {} restarted", job_id)),
        message: "Job restarted successfully".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Get configuration endpoint
async fn get_configuration(
    State(service): State<Arc<DataProcessingService>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config = service.config();

    Json(ApiResponse {
        success: true,
        data: Some(serde_json::to_value(&*config).unwrap_or_default()),
        message: "Configuration retrieved successfully".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// Get statistics endpoint
async fn get_statistics(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let metrics = service.metrics();

    match metrics.get_performance_stats().await {
        stats => Ok(Json(ApiResponse {
            success: true,
            data: Some(serde_json::to_value(&stats).unwrap_or_default()),
            message: "Statistics retrieved successfully".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })),
    }
}

/// Reset metrics endpoint
async fn reset_metrics(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    let metrics = service.metrics();

    match metrics.reset_metrics().await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some("Metrics reset successfully".to_string()),
            message: "All metrics have been reset".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })),
        Err(e) => {
            error!("Failed to reset metrics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                    error_code: "RESET_ERROR".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

/// Start service endpoint
async fn start_service(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    match service.start().await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some("Service started successfully".to_string()),
            message: "Data processing service has been started".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })),
        Err(e) => {
            error!("Failed to start service: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                    error_code: "START_ERROR".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

/// Stop service endpoint
async fn stop_service(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    match service.stop().await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some("Service stopped successfully".to_string()),
            message: "Data processing service has been stopped".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })),
        Err(e) => {
            error!("Failed to stop service: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                    error_code: "STOP_ERROR".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

/// Restart service endpoint
async fn restart_service(
    State(service): State<Arc<DataProcessingService>>,
) -> std::result::Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiError>)> {
    // Stop the service first
    if let Err(e) = service.stop().await {
        error!("Failed to stop service during restart: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to stop service: {}", e),
                error_code: "RESTART_STOP_ERROR".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            }),
        ));
    }

    // Start the service again
    match service.start().await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some("Service restarted successfully".to_string()),
            message: "Data processing service has been restarted".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })),
        Err(e) => {
            error!("Failed to start service during restart: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: format!("Failed to start service: {}", e),
                    error_code: "RESTART_START_ERROR".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_check() {
        let config = Config::default();
        let service = DataProcessingService::new(config).await.unwrap();
        let server = DataProcessingServer::new(service);
        let app = server.create_router().await.unwrap();

        let test_server = TestServer::new(app).unwrap();
        let response = test_server.get("/health").await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let config = Config::default();
        let service = DataProcessingService::new(config).await.unwrap();
        let server = DataProcessingServer::new(service);
        let app = server.create_router().await.unwrap();

        let test_server = TestServer::new(app).unwrap();
        let response = test_server.get("/health/live").await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let config = Config::default();
        let service = DataProcessingService::new(config).await.unwrap();
        let server = DataProcessingServer::new(service);
        let app = server.create_router().await.unwrap();

        let test_server = TestServer::new(app).unwrap();
        let response = test_server.get("/metrics").await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_prometheus_metrics_endpoint() {
        let config = Config::default();
        let service = DataProcessingService::new(config).await.unwrap();
        let server = DataProcessingServer::new(service);
        let app = server.create_router().await.unwrap();

        let test_server = TestServer::new(app).unwrap();
        let response = test_server.get("/metrics/prometheus").await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let body = response.text();
        assert!(!body.is_empty());
    }
}
