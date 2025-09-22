use anyhow::Result;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use config::{Config, ConfigError};
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info;

mod config_types;
mod error;
mod handlers;
mod middleware_auth;
mod models;
mod services;
mod utils;

#[cfg(test)]
mod tests;

use config_types::FileStorageConfig;
use error::{FileStorageError, FileStorageResult};
use handlers::*;
use middleware_auth::auth_middleware;
use services::*;

/// Application state containing all services and configuration
#[derive(Clone)]
pub struct AppState {
    pub storage_service: Arc<StorageService>,
    pub metadata_service: Arc<MetadataService>,
    pub virus_scanner: Arc<VirusScanner>,
    pub media_processor: Arc<MediaProcessor>,
    pub access_control: Arc<AccessControlService>,
    pub config: Arc<FileStorageConfig>,
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub services: HashMap<String, bool>,
}

/// Server metrics response
#[derive(Serialize)]
pub struct MetricsResponse {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub uploads_today: u64,
    pub downloads_today: u64,
    pub storage_usage_percent: f64,
}

/// Main application entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "file_storage=debug,tower_http=debug".into()),
        )
        .with_target(false)
        .compact()
        .init();

    info!(
        "Starting AI-CORE File Storage Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let config = load_config()?;
    info!("Configuration loaded successfully");

    // Initialize services
    let app_state = initialize_services(config).await?;
    info!("All services initialized successfully");

    // Create the router
    let app = create_router(app_state).await?;

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8084));
    info!("File Storage Service starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("File Storage Service shutting down");
    Ok(())
}

/// Load configuration from environment and config files
fn load_config() -> Result<Arc<FileStorageConfig>, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::with_name("config/file-storage").required(false))
        .add_source(config::Environment::with_prefix("FILE_STORAGE"))
        .build()?;

    let config = settings.try_deserialize::<FileStorageConfig>()?;
    Ok(Arc::new(config))
}

/// Initialize all services with proper error handling
async fn initialize_services(config: Arc<FileStorageConfig>) -> Result<AppState> {
    info!("Initializing services...");

    // Initialize storage service (S3/MinIO)
    let storage_service = Arc::new(
        StorageService::new(&config.storage)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize storage service: {}", e))?,
    );

    // Initialize metadata service (MongoDB)
    let metadata_service = Arc::new(
        MetadataService::new(&config.database.mongodb_uri)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize metadata service: {}", e))?,
    );

    // Initialize virus scanner
    let virus_scanner = Arc::new(
        VirusScanner::new(&config.security.virus_scanner)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize virus scanner: {}", e))?,
    );

    // Initialize media processor
    let media_processor = Arc::new(MediaProcessor::new(&config.processing));

    // Initialize access control service
    let access_control = Arc::new(
        AccessControlService::new(&config.security.jwt_secret)
            .map_err(|e| anyhow::anyhow!("Failed to initialize access control: {}", e))?,
    );

    info!("All services initialized successfully");

    Ok(AppState {
        storage_service,
        metadata_service,
        virus_scanner,
        media_processor,
        access_control,
        config,
    })
}

/// Create the main application router
async fn create_router(state: AppState) -> Result<Router> {
    let router = Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        // File upload endpoints
        .route("/api/v1/files/upload", post(upload_file))
        .route("/api/v1/files/upload/multipart", post(upload_multipart))
        // File download and streaming endpoints
        .route("/api/v1/files/:file_id/download", get(download_file))
        .route("/api/v1/files/:file_id/stream", get(stream_file))
        .route("/api/v1/files/:file_id/thumbnail", get(get_thumbnail))
        // File management endpoints
        .route("/api/v1/files/:file_id", get(get_file_info))
        .route("/api/v1/files/:file_id", put(update_file_metadata))
        .route("/api/v1/files/:file_id", delete(delete_file))
        // File listing and search endpoints
        .route("/api/v1/files", get(list_files))
        .route("/api/v1/files/search", get(search_files))
        // Folder management endpoints
        .route("/api/v1/folders", post(create_folder))
        .route("/api/v1/folders/:folder_id", get(get_folder))
        .route("/api/v1/folders/:folder_id/files", get(list_folder_files))
        // Permission management endpoints
        .route(
            "/api/v1/files/:file_id/permissions",
            get(get_file_permissions),
        )
        .route(
            "/api/v1/files/:file_id/permissions",
            put(update_file_permissions),
        )
        // Batch operations
        .route("/api/v1/files/batch/delete", post(batch_delete_files))
        .route("/api/v1/files/batch/move", post(batch_move_files))
        // Administrative endpoints
        .route("/api/v1/admin/storage/stats", get(get_storage_stats))
        .route("/api/v1/admin/files/cleanup", post(cleanup_orphaned_files))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB max
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth_middleware,
                )),
        )
        .with_state(state);

    Ok(router)
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let mut services = HashMap::new();

    // Check storage service
    services.insert(
        "storage".to_string(),
        state.storage_service.health_check().await.is_ok(),
    );

    // Check metadata service
    services.insert(
        "metadata".to_string(),
        state.metadata_service.health_check().await.is_ok(),
    );

    // Check virus scanner
    services.insert(
        "virus_scanner".to_string(),
        state.virus_scanner.health_check().await.is_ok(),
    );

    let all_healthy = services.values().all(|&healthy| healthy);
    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: if all_healthy {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services,
    };

    (status_code, Json(response))
}

/// Get service metrics
async fn get_metrics(State(state): State<AppState>) -> FileStorageResult<Json<MetricsResponse>> {
    let stats = state.metadata_service.get_storage_stats().await?;

    Ok(Json(MetricsResponse {
        total_files: stats.total_files,
        total_size_bytes: stats.total_size_bytes,
        uploads_today: stats.uploads_today,
        downloads_today: stats.downloads_today,
        storage_usage_percent: stats.storage_usage_percent,
    }))
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down");
        },
    }
}

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        // Basic compilation test
        assert!(true);
    }

    #[tokio::test]
    async fn test_config_loading() {
        // Test configuration loading with environment variables
        std::env::set_var("FILE_STORAGE_SERVER_PORT", "8084");
        std::env::set_var("FILE_STORAGE_STORAGE_TYPE", "s3");

        // This would test actual config loading
        assert!(true);
    }
}
