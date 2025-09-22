// AI-CORE Test Data Management API Server
// FAANG-Enhanced Testing Infrastructure - Backend Agent Implementation T2.2
// Complete HTTP API server with multi-database support and comprehensive endpoints

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware::from_fn,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Pool, Postgres};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::MakeRequestId,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    timeout::TimeoutLayer,
};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

mod models;
mod database;
mod generators;
mod cleanup;
mod auth;
mod health;
mod metrics;

use models::*;
use database::DatabaseManager;
use generators::DataGenerator;
use cleanup::CleanupService;
use auth::AuthService;
use health::HealthService;
use metrics::MetricsService;

// ============================================================================
// Application State and Configuration
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    pub database: Arc<DatabaseManager>,
    pub data_generator: Arc<DataGenerator>,
    pub cleanup_service: Arc<CleanupService>,
    pub auth_service: Arc<AuthService>,
    pub health_service: Arc<HealthService>,
    pub metrics_service: Arc<MetricsService>,
    pub config: Arc<AppConfig>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_port: u16,
    pub server_host: String,
    pub database_url: String,
    pub redis_url: String,
    pub mongodb_url: String,
    pub clickhouse_url: String,
    pub jwt_secret: String,
    pub cors_origins: Vec<String>,
    pub request_timeout_seconds: u64,
    pub max_request_size: usize,
    pub rate_limit_per_second: u64,
    pub cleanup_interval_hours: u64,
    pub data_generation_batch_size: usize,
    pub environment_ttl_hours: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_port: 8002,
            server_host: "0.0.0.0".to_string(),
            database_url: "postgresql://localhost:5432/aicore_test".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            mongodb_url: "mongodb://localhost:27017".to_string(),
            clickhouse_url: "http://localhost:8123".to_string(),
            jwt_secret: "test-secret-key-change-in-production".to_string(),
            cors_origins: vec!["http://localhost:3000".to_string()],
            request_timeout_seconds: 30,
            max_request_size: 16 * 1024 * 1024, // 16MB
            rate_limit_per_second: 100,
            cleanup_interval_hours: 24,
            data_generation_batch_size: 1000,
            environment_ttl_hours: 72,
        }
    }
}

// ============================================================================
// Request ID Generation
// ============================================================================

#[derive(Clone)]
struct RequestIdGenerator;

impl MakeRequestId for RequestIdGenerator {
    fn make_request_id<B>(&mut self, _: &axum::http::Request<B>) -> Option<axum::http::HeaderValue> {
        let request_id = Uuid::new_v4().to_string();
        axum::http::HeaderValue::from_str(&request_id).ok()
    }
}

// ============================================================================
// Middleware for Authentication and Metrics
// ============================================================================

async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next<axum::body::Body>,
) -> impl IntoResponse {
    // Extract JWT token from Authorization header
    let auth_header = headers.get("Authorization");

    if let Some(auth_value) = auth_header {
        if let Ok(auth_str) = auth_value.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                match state.auth_service.validate_token(token).await {
                    Ok(user_claims) => {
                        request.extensions_mut().insert(user_claims);
                        return next.run(request).await;
                    }
                    Err(e) => {
                        error!("Token validation failed: {}", e);
                        return (StatusCode::UNAUTHORIZED, Json(ApiError {
                            error_code: "INVALID_TOKEN".to_string(),
                            message: "Invalid or expired authentication token".to_string(),
                            details: Some(serde_json::json!({"error": e.to_string()})),
                            timestamp: Utc::now(),
                            request_id: Uuid::new_v4().to_string(),
                            suggestions: vec![
                                "Check your authentication token".to_string(),
                                "Ensure token hasn't expired".to_string(),
                                "Re-authenticate if necessary".to_string(),
                            ],
                        })).into_response();
                    }
                }
            }
        }
    }

    // For testing endpoints, allow access without authentication
    let path = request.uri().path();
    if path.starts_with("/health") || path.starts_with("/metrics") || path.starts_with("/api/test-data") {
        return next.run(request).await;
    }

    (StatusCode::UNAUTHORIZED, Json(ApiError {
        error_code: "MISSING_AUTH".to_string(),
        message: "Authentication required".to_string(),
        details: None,
        timestamp: Utc::now(),
        request_id: Uuid::new_v4().to_string(),
        suggestions: vec![
            "Include Authorization header with Bearer token".to_string(),
            "Authenticate first to get a valid token".to_string(),
        ],
    })).into_response()
}

// ============================================================================
// Test User Management Endpoints
// ============================================================================

async fn create_test_user(
    State(state): State<AppState>,
    Json(request): Json<CreateTestUserRequest>,
) -> Result<Json<TestUser>, (StatusCode, Json<ApiError>)> {
    debug!("Creating test user: {}", request.username);

    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error_code: "VALIDATION_ERROR".to_string(),
                message: "Request validation failed".to_string(),
                details: Some(serde_json::json!(validation_errors)),
                timestamp: Utc::now(),
                request_id: Uuid::new_v4().to_string(),
                suggestions: vec!["Fix validation errors and retry".to_string()],
            }),
        ));
    }

    match state.database.create_test_user(request).await {
        Ok(user) => {
            info!("Created test user: {} ({})", user.username, user.id);
            state.metrics_service.increment_counter("test_users_created").await;
            Ok(Json(user))
        }
        Err(e) => {
            error!("Failed to create test user: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "USER_CREATION_FAILED".to_string(),
                    message: "Failed to create test user".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec![
                        "Check if username/email is already taken".to_string(),
                        "Verify database connectivity".to_string(),
                    ],
                }),
            ))
        }
    }
}

async fn get_test_users(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<TestUser>>, (StatusCode, Json<ApiError>)> {
    let environment = params.get("environment").cloned().unwrap_or_default();
    let limit = params.get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100);

    debug!("Fetching test users for environment: {}, limit: {}", environment, limit);

    match state.database.get_test_users(&environment, limit).await {
        Ok(users) => {
            info!("Retrieved {} test users", users.len());
            Ok(Json(users))
        }
        Err(e) => {
            error!("Failed to fetch test users: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "FETCH_USERS_FAILED".to_string(),
                    message: "Failed to fetch test users".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check database connectivity".to_string()],
                }),
            ))
        }
    }
}

async fn delete_test_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    debug!("Deleting test user: {}", user_id);

    match state.database.delete_test_user(user_id).await {
        Ok(deleted) => {
            if deleted {
                info!("Deleted test user: {}", user_id);
                state.metrics_service.increment_counter("test_users_deleted").await;
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiError {
                        error_code: "USER_NOT_FOUND".to_string(),
                        message: "Test user not found".to_string(),
                        details: Some(serde_json::json!({"user_id": user_id})),
                        timestamp: Utc::now(),
                        request_id: Uuid::new_v4().to_string(),
                        suggestions: vec!["Verify the user ID is correct".to_string()],
                    }),
                ))
            }
        }
        Err(e) => {
            error!("Failed to delete test user: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "USER_DELETION_FAILED".to_string(),
                    message: "Failed to delete test user".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check database connectivity".to_string()],
                }),
            ))
        }
    }
}

// ============================================================================
// Test Environment Management Endpoints
// ============================================================================

async fn create_test_environment(
    State(state): State<AppState>,
    Json(request): Json<CreateEnvironmentRequest>,
) -> Result<Json<EnvironmentResponse>, (StatusCode, Json<ApiError>)> {
    debug!("Creating test environment: {}", request.name);

    if let Err(validation_errors) = request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error_code: "VALIDATION_ERROR".to_string(),
                message: "Request validation failed".to_string(),
                details: Some(serde_json::json!(validation_errors)),
                timestamp: Utc::now(),
                request_id: Uuid::new_v4().to_string(),
                suggestions: vec!["Fix validation errors and retry".to_string()],
            }),
        ));
    }

    match state.database.create_test_environment(request).await {
        Ok(environment) => {
            info!("Created test environment: {} ({})", environment.name, environment.id);

            let response = EnvironmentResponse {
                status_url: format!("/api/environments/{}/status", environment.id),
                dashboard_url: Some(format!("/dashboard/environments/{}", environment.id)),
                api_endpoints: environment.configuration.api_endpoints.clone(),
                environment,
            };

            state.metrics_service.increment_counter("test_environments_created").await;
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to create test environment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "ENVIRONMENT_CREATION_FAILED".to_string(),
                    message: "Failed to create test environment".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check resource availability and configuration".to_string()],
                }),
            ))
        }
    }
}

async fn get_test_environments(
    State(state): State<AppState>,
) -> Result<Json<Vec<TestEnvironment>>, (StatusCode, Json<ApiError>)> {
    debug!("Fetching all test environments");

    match state.database.get_test_environments().await {
        Ok(environments) => {
            info!("Retrieved {} test environments", environments.len());
            Ok(Json(environments))
        }
        Err(e) => {
            error!("Failed to fetch test environments: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "FETCH_ENVIRONMENTS_FAILED".to_string(),
                    message: "Failed to fetch test environments".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check database connectivity".to_string()],
                }),
            ))
        }
    }
}

async fn reset_test_environment(
    State(state): State<AppState>,
    Path(environment_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    debug!("Resetting test environment: {}", environment_id);

    match state.cleanup_service.reset_environment(environment_id).await {
        Ok(()) => {
            info!("Reset test environment: {}", environment_id);
            state.metrics_service.increment_counter("test_environments_reset").await;
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Failed to reset test environment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "ENVIRONMENT_RESET_FAILED".to_string(),
                    message: "Failed to reset test environment".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check if environment exists and is accessible".to_string()],
                }),
            ))
        }
    }
}

// ============================================================================
// Data Generation Endpoints
// ============================================================================

async fn generate_test_data(
    State(state): State<AppState>,
    Json(request): Json<GenerateDataRequest>,
) -> Result<Json<DataGenerationResponse>, (StatusCode, Json<ApiError>)> {
    debug!("Generating test data: {:?}", request.data_generation.data_type);

    match state.data_generator.generate_data(request).await {
        Ok(response) => {
            info!("Started data generation: {}", response.generation_id);
            state.metrics_service.increment_counter("data_generation_started").await;
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to generate test data: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "DATA_GENERATION_FAILED".to_string(),
                    message: "Failed to start data generation".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check data generation parameters".to_string()],
                }),
            ))
        }
    }
}

async fn get_generation_status(
    State(state): State<AppState>,
    Path(generation_id): Path<Uuid>,
) -> Result<Json<DataGenerationResponse>, (StatusCode, Json<ApiError>)> {
    debug!("Getting generation status: {}", generation_id);

    match state.data_generator.get_generation_status(generation_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            error!("Failed to get generation status: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    error_code: "GENERATION_NOT_FOUND".to_string(),
                    message: "Data generation not found".to_string(),
                    details: Some(serde_json::json!({"generation_id": generation_id})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Verify the generation ID is correct".to_string()],
                }),
            ))
        }
    }
}

// ============================================================================
// Cleanup Endpoints
// ============================================================================

async fn cleanup_test_data(
    State(state): State<AppState>,
    Json(request): Json<CleanupRequest>,
) -> Result<Json<CleanupResponse>, (StatusCode, Json<ApiError>)> {
    debug!("Starting cleanup: {:?}", request.cleanup_type);

    match state.cleanup_service.cleanup(request).await {
        Ok(response) => {
            info!("Started cleanup: {}", response.cleanup_id);
            state.metrics_service.increment_counter("cleanup_operations_started").await;
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to start cleanup: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "CLEANUP_FAILED".to_string(),
                    message: "Failed to start cleanup".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check cleanup parameters".to_string()],
                }),
            ))
        }
    }
}

async fn get_cleanup_status(
    State(state): State<AppState>,
    Path(cleanup_id): Path<Uuid>,
) -> Result<Json<CleanupResponse>, (StatusCode, Json<ApiError>)> {
    debug!("Getting cleanup status: {}", cleanup_id);

    match state.cleanup_service.get_cleanup_status(cleanup_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            error!("Failed to get cleanup status: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    error_code: "CLEANUP_NOT_FOUND".to_string(),
                    message: "Cleanup operation not found".to_string(),
                    details: Some(serde_json::json!({"cleanup_id": cleanup_id})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Verify the cleanup ID is correct".to_string()],
                }),
            ))
        }
    }
}

// ============================================================================
// Health and Metrics Endpoints
// ============================================================================

async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthStatus>, (StatusCode, Json<ApiError>)> {
    match state.health_service.get_health_status().await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            error!("Health check failed: {}", e);
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiError {
                    error_code: "HEALTH_CHECK_FAILED".to_string(),
                    message: "Service health check failed".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check service dependencies".to_string()],
                }),
            ))
        }
    }
}

async fn get_metrics(
    State(state): State<AppState>,
) -> Result<String, (StatusCode, Json<ApiError>)> {
    match state.metrics_service.get_prometheus_metrics().await {
        Ok(metrics) => Ok(metrics),
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error_code: "METRICS_FAILED".to_string(),
                    message: "Failed to retrieve metrics".to_string(),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                    timestamp: Utc::now(),
                    request_id: Uuid::new_v4().to_string(),
                    suggestions: vec!["Check metrics service".to_string()],
                }),
            ))
        }
    }
}

// ============================================================================
// Router Setup
// ============================================================================

async fn create_router(state: AppState) -> Router {
    Router::new()
        // Test User Management Routes
        .route("/api/test-users", post(create_test_user))
        .route("/api/test-users", get(get_test_users))
        .route("/api/test-users/:id", delete(delete_test_user))

        // Test Environment Management Routes
        .route("/api/environments", post(create_test_environment))
        .route("/api/environments", get(get_test_environments))
        .route("/api/environments/:id/reset", post(reset_test_environment))

        // Data Generation Routes
        .route("/api/generate-data", post(generate_test_data))
        .route("/api/generate-data/:id/status", get(get_generation_status))

        // Cleanup Routes
        .route("/api/cleanup", post(cleanup_test_data))
        .route("/api/cleanup/:id/status", get(get_cleanup_status))

        // Health and Metrics Routes
        .route("/health", get(health_check))
        .route("/health/detailed", get(health_check))
        .route("/metrics", get(get_metrics))

        // Add middleware layers
        .layer(
            ServiceBuilder::new()
                .layer(TimeoutLayer::new(Duration::from_secs(state.config.request_timeout_seconds)))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
                        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
                )
                .layer(from_fn(auth_middleware))
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

// ============================================================================
// Application Initialization and Main Function
// ============================================================================

async fn initialize_services(config: AppConfig) -> anyhow::Result<AppState> {
    info!("Initializing Test Data API services...");

    // Initialize database manager
    let database = Arc::new(DatabaseManager::new(&config).await?);

    // Initialize other services
    let data_generator = Arc::new(DataGenerator::new(database.clone()).await?);
    let cleanup_service = Arc::new(CleanupService::new(database.clone()).await?);
    let auth_service = Arc::new(AuthService::new(config.jwt_secret.clone()).await?);
    let health_service = Arc::new(HealthService::new(database.clone()).await?);
    let metrics_service = Arc::new(MetricsService::new().await?);

    info!("All services initialized successfully");

    Ok(AppState {
        database,
        data_generator,
        cleanup_service,
        auth_service,
        health_service,
        metrics_service,
        config: Arc::new(config),
    })
}

async fn start_background_tasks(state: AppState) {
    let cleanup_interval = Duration::from_secs(state.config.cleanup_interval_hours * 3600);
    let cleanup_service = state.cleanup_service.clone();
    let metrics_service = state.metrics_service.clone();

    // Start cleanup task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(cleanup_interval);
        loop {
            interval.tick().await;
            info!("Running scheduled cleanup task");

            if let Err(e) = cleanup_service.run_scheduled_cleanup().await {
                error!("Scheduled cleanup failed: {}", e);
            }

            metrics_service.increment_counter("scheduled_cleanups_executed").await;
        }
    });

    // Start metrics collection task
    let metrics_service = state.metrics_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = metrics_service.collect_system_metrics().await {
                error!("Failed to collect system metrics: {}", e);
            }
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("test_data_api=debug,tower_http=debug,axum=debug")
        .with_target(false)
        .compact()
        .init();

    info!("Starting AI-CORE Test Data Management API Server");

    // Load configuration
    let config = AppConfig::default(); // In production, load from environment/config files

    // Initialize services
    let state = initialize_services(config.clone()).await?;

    // Start background tasks
    start_background_tasks(state.clone()).await;

    // Create the application router
    let app = create_router(state.clone()).await;

    // Create server address
    let addr = format!("{}:{}", config.server_host, config.server_port);
    info!("Server starting on {}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Test Data API Server listening on {}", addr);

    // Log available endpoints
    info!("Available endpoints:");
    info!("  POST /api/test-users - Create test user");
    info!("  GET  /api/test-users - List test users");
    info!("  DELETE /api/test-users/:id - Delete test user");
    info!("  POST /api/environments - Create test environment");
    info!("  GET  /api/environments - List test environments");
    info!("  POST /api/environments/:id/reset - Reset environment");
    info!("  POST /api/generate-data - Generate test data");
    info!("  GET  /api/generate-data/:id/status - Get generation status");
    info!("  POST /api/cleanup - Start cleanup operation");
    info!("  GET  /api/cleanup/:id/status - Get cleanup status");
    info!("  GET  /health - Health check");
    info!("  GET  /metrics - Prometheus metrics");

    // Start serving requests with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Test Data API Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully");
        },
    }
}

// ============================================================================
// Tests Module
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;

    #[tokio::test]
    async fn test_health_endpoint() {
        // This is a placeholder test - in a real implementation,
        // you would set up a test database and proper test state
        let config = AppConfig::default();
        // let state = initialize_services(config).await.unwrap();
        // let app = create_router(state).await;
        // let server = TestServer::new(app).unwrap();

        // let response = server.get("/health").await;
        // assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn test_create_test_user() {
        // Placeholder for user creation test
        let user_request = CreateTestUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            role: UserRole::User,
            permissions: vec!["read".to_string()],
            test_environment: "test".to_string(),
            ttl_hours: Some(24),
        };

        assert!(user_request.validate().is_ok());
    }
}
