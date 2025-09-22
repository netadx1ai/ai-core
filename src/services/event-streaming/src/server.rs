//! # Event Streaming Server
//!
//! Main server implementation for the event streaming service.
//! This module provides the core server functionality including HTTP endpoints,
//! service orchestration, and lifecycle management.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::sync::{broadcast, RwLock};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{EventStreamingError, Result},
    events::Event,
    kafka::KafkaManager,
    metrics::MetricsCollector,
    processing::ProcessingPipeline,
    redis_streams::RedisStreamManager,
    routing::EventRouter,
    storage::EventStorage,
    types::{ComponentHealth, EventCategory, HealthStatus},
};

/// Main event streaming service
#[derive(Clone)]
pub struct EventStreamingService {
    config: Arc<Config>,
    kafka_manager: Arc<KafkaManager>,
    redis_manager: Arc<RedisStreamManager>,
    processing_pipeline: Arc<ProcessingPipeline>,
    event_router: Arc<EventRouter>,
    event_storage: Arc<EventStorage>,
    metrics_collector: Arc<MetricsCollector>,
    shutdown_tx: Arc<RwLock<Option<broadcast::Sender<()>>>>,
    health_status: Arc<RwLock<HealthStatus>>,
}

impl EventStreamingService {
    /// Create a new event streaming service
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing Event Streaming Service");

        // Validate configuration
        config
            .validate()
            .map_err(|e| EventStreamingError::configuration(e.to_string()))?;

        let config = Arc::new(config);

        // Initialize metrics collector
        let metrics_collector = Arc::new(MetricsCollector::new(&config).await?);

        // Initialize storage layer
        let event_storage = Arc::new(EventStorage::new(&config).await?);

        // Initialize Kafka manager
        let kafka_manager = Arc::new(KafkaManager::new(&config, metrics_collector.clone()).await?);

        // Initialize Redis stream manager
        let redis_manager =
            Arc::new(RedisStreamManager::new(&config, metrics_collector.clone()).await?);

        // Initialize event router
        let event_router = Arc::new(EventRouter::new(&config).await?);

        // Initialize processing pipeline
        let processing_pipeline = Arc::new(
            ProcessingPipeline::new(
                &config,
                kafka_manager.clone(),
                redis_manager.clone(),
                event_storage.clone(),
                event_router.clone(),
                metrics_collector.clone(),
            )
            .await?,
        );

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config,
            kafka_manager,
            redis_manager,
            processing_pipeline,
            event_router,
            event_storage,
            metrics_collector,
            shutdown_tx: Arc::new(RwLock::new(Some(shutdown_tx))),
            health_status: Arc::new(RwLock::new(HealthStatus::Healthy)),
        })
    }

    /// Start the event streaming service
    pub async fn start(&self) -> Result<()> {
        info!("Starting Event Streaming Service");

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Healthy;
        }

        // Start all components
        tokio::try_join!(
            self.start_kafka_manager(),
            self.start_redis_manager(),
            self.start_processing_pipeline(),
            self.start_http_server(),
            self.start_health_monitor(),
        )?;

        info!("Event Streaming Service started successfully");
        Ok(())
    }

    /// Stop the event streaming service gracefully
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Event Streaming Service");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(());
        }

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Unhealthy;
        }

        // Stop all components
        tokio::join!(
            self.stop_processing_pipeline(),
            self.stop_kafka_manager(),
            self.stop_redis_manager(),
        );

        info!("Event Streaming Service stopped");
        Ok(())
    }

    /// Get service health status
    pub async fn health(&self) -> Result<serde_json::Value> {
        let overall_health = self.health_status.read().await.clone();

        let component_healths = vec![
            self.check_kafka_health().await,
            self.check_redis_health().await,
            self.check_storage_health().await,
            self.check_processing_health().await,
        ];

        let metrics = self.metrics_collector.get_snapshot().await?;

        Ok(serde_json::json!({
            "status": overall_health,
            "service": "event-streaming-service",
            "version": env!("CARGO_PKG_VERSION"),
            "components": component_healths,
            "metrics": metrics,
            "timestamp": chrono::Utc::now(),
        }))
    }

    /// Create HTTP router for the service
    fn create_router(&self) -> Router {
        Router::new()
            // Health endpoints
            .route("/health", get(health_handler))
            .route("/health/ready", get(readiness_handler))
            .route("/health/live", get(liveness_handler))
            // Event endpoints
            .route("/events", post(publish_event_handler))
            .route("/events/:id", get(get_event_handler))
            .route("/events/:id/status", get(get_event_status_handler))
            // Stream management endpoints
            .route("/streams", get(list_streams_handler))
            .route("/streams/:name/info", get(get_stream_info_handler))
            // Replay endpoints
            .route("/replay/events", post(replay_events_handler))
            .route("/replay/status/:job_id", get(get_replay_status_handler))
            // Metrics endpoint
            .route("/metrics", get(metrics_handler))
            // Administrative endpoints
            .route("/admin/config", get(get_config_handler))
            .route("/admin/stats", get(get_stats_handler))
            .with_state(self.clone())
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(CorsLayer::permissive()),
            )
    }

    /// Start Kafka manager
    async fn start_kafka_manager(&self) -> Result<()> {
        self.kafka_manager.start().await
    }

    /// Start Redis manager
    async fn start_redis_manager(&self) -> Result<()> {
        self.redis_manager.start().await
    }

    /// Start processing pipeline
    async fn start_processing_pipeline(&self) -> Result<()> {
        self.processing_pipeline.start().await
    }

    /// Start HTTP server
    async fn start_http_server(&self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.server.port));
        let router = self.create_router();

        info!("Starting HTTP server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            EventStreamingError::Network {
                message: format!("Failed to bind to {}: {}", addr, e),
                endpoint: Some(addr.to_string()),
                status_code: None,
                retry_after: None,
            }
        })?;

        let shutdown_rx = self.shutdown_tx.read().await.as_ref().unwrap().subscribe();
        axum::serve(listener, router)
            .with_graceful_shutdown(Self::shutdown_signal(shutdown_rx))
            .await
            .map_err(|e| EventStreamingError::internal(format!("Server error: {}", e)))
    }

    /// Start health monitoring
    async fn start_health_monitor(&self) -> Result<()> {
        let service = self.clone();
        let interval = Duration::from_secs(self.config.monitoring.health_check.interval_seconds);

        tokio::spawn(async move {
            let mut shutdown_rx = service
                .shutdown_tx
                .read()
                .await
                .as_ref()
                .unwrap()
                .subscribe();
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = service.perform_health_check().await {
                            error!("Health check failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Health monitor received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop processing pipeline
    async fn stop_processing_pipeline(&self) {
        if let Err(e) = self.processing_pipeline.stop().await {
            error!("Failed to stop processing pipeline: {}", e);
        }
    }

    /// Stop Kafka manager
    async fn stop_kafka_manager(&self) {
        if let Err(e) = self.kafka_manager.stop().await {
            error!("Failed to stop Kafka manager: {}", e);
        }
    }

    /// Stop Redis manager
    async fn stop_redis_manager(&self) {
        if let Err(e) = self.redis_manager.stop().await {
            error!("Failed to stop Redis manager: {}", e);
        }
    }

    /// Wait for shutdown signal
    async fn shutdown_signal(mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        let shutdown = async {
            shutdown_rx.recv().await.ok();
        };

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal");
            }
            _ = terminate => {
                info!("Received terminate signal");
            }
            _ = shutdown => {
                info!("Received shutdown signal");
            }
        }
    }

    /// Perform comprehensive health check
    async fn perform_health_check(&self) -> Result<()> {
        let mut overall_healthy = true;

        // Check all components
        let healths = vec![
            self.check_kafka_health().await,
            self.check_redis_health().await,
            self.check_storage_health().await,
            self.check_processing_health().await,
        ];

        for health in &healths {
            if health.status != HealthStatus::Healthy {
                overall_healthy = false;
            }
        }

        // Update overall health status
        {
            let mut status = self.health_status.write().await;
            *status = if overall_healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            };
        }

        // Update metrics
        self.metrics_collector.record_health_check(&healths).await?;

        Ok(())
    }

    /// Check Kafka health
    async fn check_kafka_health(&self) -> ComponentHealth {
        match self.kafka_manager.health_check().await {
            Ok(health) => health,
            Err(e) => ComponentHealth {
                component: "kafka".to_string(),
                status: HealthStatus::Unhealthy,
                last_check: chrono::Utc::now(),
                response_time_ms: 0,
                details: [("error".to_string(), e.to_string())].into(),
            },
        }
    }

    /// Check Redis health
    async fn check_redis_health(&self) -> ComponentHealth {
        match self.redis_manager.health_check().await {
            Ok(health) => health,
            Err(e) => ComponentHealth {
                component: "redis".to_string(),
                status: HealthStatus::Unhealthy,
                last_check: chrono::Utc::now(),
                response_time_ms: 0,
                details: [("error".to_string(), e.to_string())].into(),
            },
        }
    }

    /// Check storage health
    async fn check_storage_health(&self) -> ComponentHealth {
        match self.event_storage.health_check().await {
            Ok(health) => health,
            Err(e) => ComponentHealth {
                component: "storage".to_string(),
                status: HealthStatus::Unhealthy,
                last_check: chrono::Utc::now(),
                response_time_ms: 0,
                details: [("error".to_string(), e.to_string())].into(),
            },
        }
    }

    /// Check processing pipeline health
    async fn check_processing_health(&self) -> ComponentHealth {
        match self.processing_pipeline.health_check().await {
            Ok(health) => health,
            Err(e) => ComponentHealth {
                component: "processing".to_string(),
                status: HealthStatus::Unhealthy,
                last_check: chrono::Utc::now(),
                response_time_ms: 0,
                details: [("error".to_string(), e.to_string())].into(),
            },
        }
    }
}

// HTTP Handler functions

/// Health check handler
async fn health_handler(
    State(service): State<EventStreamingService>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    match service.health().await {
        Ok(health) => Ok(Json(health)),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Readiness check handler
async fn readiness_handler(
    State(service): State<EventStreamingService>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    let health_status = service.health_status.read().await.clone();

    match health_status {
        HealthStatus::Healthy | HealthStatus::Degraded => Ok(Json(serde_json::json!({
            "status": "ready",
            "timestamp": chrono::Utc::now(),
        }))),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Liveness check handler
async fn liveness_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now(),
    }))
}

/// Publish event handler
#[derive(Debug, Deserialize)]
struct PublishEventRequest {
    event: Event,
    routing_key: Option<String>,
    async_mode: Option<bool>,
}

#[derive(Debug, Serialize)]
struct PublishEventResponse {
    event_id: Uuid,
    status: String,
    message: String,
}

async fn publish_event_handler(
    State(service): State<EventStreamingService>,
    Json(request): Json<PublishEventRequest>,
) -> std::result::Result<Json<PublishEventResponse>, StatusCode> {
    match service
        .processing_pipeline
        .publish_event(request.event.clone())
        .await
    {
        Ok(_) => Ok(Json(PublishEventResponse {
            event_id: request.event.id,
            status: "accepted".to_string(),
            message: "Event queued for processing".to_string(),
        })),
        Err(e) => {
            error!("Failed to publish event: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get event handler
async fn get_event_handler(
    State(service): State<EventStreamingService>,
    Path(event_id): Path<Uuid>,
) -> std::result::Result<Json<Event>, StatusCode> {
    match service.event_storage.get_event(event_id).await {
        Ok(Some(event)) => Ok(Json(event)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get event {}: {}", event_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get event status handler
#[derive(Debug, Serialize)]
struct EventStatusResponse {
    event_id: Uuid,
    status: String,
    processing_history: Vec<serde_json::Value>,
}

async fn get_event_status_handler(
    State(service): State<EventStreamingService>,
    Path(event_id): Path<Uuid>,
) -> std::result::Result<Json<EventStatusResponse>, StatusCode> {
    match service.event_storage.get_event_status(event_id).await {
        Ok(Some((status, history))) => Ok(Json(EventStatusResponse {
            event_id,
            status: status.to_string(),
            processing_history: history,
        })),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get event status {}: {}", event_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List streams handler
async fn list_streams_handler(
    State(service): State<EventStreamingService>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    match service.event_router.list_streams().await {
        Ok(streams) => Ok(Json(serde_json::json!({
            "streams": streams,
            "count": streams.len(),
        }))),
        Err(e) => {
            error!("Failed to list streams: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get stream info handler
async fn get_stream_info_handler(
    State(service): State<EventStreamingService>,
    Path(stream_name): Path<String>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    match service.event_router.get_stream_info(&stream_name).await {
        Ok(Some(info)) => Ok(Json(info)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get stream info for {}: {}", stream_name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Replay events handler
#[derive(Debug, Deserialize)]
struct ReplayEventsRequest {
    from_timestamp: chrono::DateTime<chrono::Utc>,
    to_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    event_types: Option<Vec<String>>,
    categories: Option<Vec<EventCategory>>,
    batch_size: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ReplayEventsResponse {
    job_id: Uuid,
    status: String,
    message: String,
    estimated_events: Option<u64>,
}

async fn replay_events_handler(
    State(service): State<EventStreamingService>,
    Json(request): Json<ReplayEventsRequest>,
) -> std::result::Result<Json<ReplayEventsResponse>, StatusCode> {
    // TODO: Implement replay functionality
    let job_id = uuid::Uuid::new_v4();
    Ok(Json(ReplayEventsResponse {
        job_id,
        status: "started".to_string(),
        message: "Replay job started successfully".to_string(),
        estimated_events: Some(0),
    }))
}

/// Get replay status handler
async fn get_replay_status_handler(
    State(service): State<EventStreamingService>,
    Path(job_id): Path<Uuid>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Implement replay status retrieval
    Ok(Json(serde_json::json!({
        "job_id": job_id,
        "status": "unknown",
        "message": "Replay status not implemented"
    })))
}

/// Metrics handler
async fn metrics_handler(
    State(service): State<EventStreamingService>,
) -> std::result::Result<String, StatusCode> {
    match service.metrics_collector.export_prometheus().await {
        Ok(metrics) => Ok(metrics),
        Err(e) => {
            error!("Failed to export metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get configuration handler
async fn get_config_handler(
    State(service): State<EventStreamingService>,
) -> Json<serde_json::Value> {
    // Return sanitized configuration (without sensitive data)
    Json(serde_json::json!({
        "service": {
            "name": "event-streaming-service",
            "version": env!("CARGO_PKG_VERSION"),
            "environment": service.config.environment.name,
        },
        "server": {
            "host": service.config.server.host,
            "port": service.config.server.port,
        },
        "processing": {
            "worker_threads": service.config.processing.worker_threads,
            "batch_size": service.config.processing.batch_size,
        },
        "monitoring": {
            "metrics_enabled": service.config.monitoring.enable_metrics,
            "tracing_enabled": service.config.monitoring.enable_tracing,
        }
    }))
}

/// Get statistics handler
async fn get_stats_handler(
    State(service): State<EventStreamingService>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    match service.processing_pipeline.get_processing_stats().await {
        Ok(stats) => Ok(Json(serde_json::json!({
            "processing": stats,
            "timestamp": chrono::Utc::now(),
        }))),
        Err(e) => {
            error!("Failed to get processing stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_service_creation() {
        let config = Config::default();
        let result = EventStreamingService::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_router_creation() {
        let config = Config::default();
        let service = EventStreamingService::new(config).await.unwrap();
        let router = service.create_router();
        assert!(router.into_make_service().is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = Config::default();
        let service = EventStreamingService::new(config).await.unwrap();
        let health = service.health().await;
        assert!(health.is_ok());
    }
}
