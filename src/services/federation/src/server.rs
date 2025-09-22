//! Federation Service HTTP Server
//!
//! This module implements the main HTTP server for the Federation Service,
//! providing REST API endpoints for client management, provider selection,
//! schema translation, workflow execution, and MCP server integration.

use crate::{
    client::ClientManager,
    config::Config,
    cost_optimizer::CostOptimizer,
    handlers,
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    models::FederationError,
    provider::ProviderManager,
    proxy::McpProxy,
    schema_translator::SchemaTranslationService,
    workflow::WorkflowEngine,
};
use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

/// Federation server state shared across all handlers
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Service configuration
    pub config: Arc<Config>,
    /// Client management
    pub client_manager: Arc<ClientManager>,
    /// Provider management
    pub provider_manager: Arc<ProviderManager>,
    /// Schema translation
    pub schema_translator: Arc<SchemaTranslationService>,
    /// Workflow execution engine
    pub workflow_engine: Arc<WorkflowEngine>,
    /// MCP proxy
    pub mcp_proxy: Arc<McpProxy>,
    /// Cost optimization
    pub cost_optimizer: Arc<CostOptimizer>,
}

/// Federation HTTP server
#[derive(Debug)]
pub struct FederationServer {
    /// Server state
    state: ServerState,
    /// HTTP router
    router: Router,
}

impl FederationServer {
    /// Create a new federation server
    pub async fn new(
        config: Arc<Config>,
        client_manager: Arc<ClientManager>,
        provider_manager: Arc<ProviderManager>,
        schema_translator: Arc<SchemaTranslationService>,
        workflow_engine: Arc<WorkflowEngine>,
        mcp_proxy: Arc<McpProxy>,
        cost_optimizer: Arc<CostOptimizer>,
    ) -> Result<Self, FederationError> {
        let state = ServerState {
            config: config.clone(),
            client_manager,
            provider_manager,
            schema_translator,
            workflow_engine,
            mcp_proxy,
            cost_optimizer,
        };

        let router = create_router(state.clone(), &config).await?;

        Ok(Self { state, router })
    }

    /// Start the HTTP server
    pub async fn start(&self) -> Result<(), FederationError> {
        let addr = format!(
            "{}:{}",
            self.state.config.server.host, self.state.config.server.port
        );

        info!("Starting Federation Service HTTP server on {}", addr);

        let listener =
            TcpListener::bind(&addr)
                .await
                .map_err(|e| FederationError::InternalError {
                    message: format!("Failed to bind to address {}: {}", addr, e),
                })?;

        info!("🚀 Federation Service is ready and listening on {}", addr);

        axum::serve(listener, self.router.clone())
            .await
            .map_err(|e| FederationError::InternalError {
                message: format!("Server error: {}", e),
            })?;

        Ok(())
    }

    /// Get server health information
    pub async fn health(&self) -> Result<serde_json::Value, FederationError> {
        Ok(serde_json::json!({
            "service": "federation",
            "status": "healthy",
            "timestamp": chrono::Utc::now(),
            "version": env!("CARGO_PKG_VERSION")
        }))
    }
}

/// Create the HTTP router with all routes and middleware
async fn create_router(state: ServerState, config: &Config) -> Result<Router, FederationError> {
    // Create middleware layers
    let auth_middleware = AuthMiddleware::new(&config.auth).await?;
    let rate_limit_middleware = RateLimitMiddleware::new(&config.rate_limiting).await?;

    // Create CORS layer
    let cors_layer = if config.server.enable_cors {
        CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
            .allow_origin(Any)
    } else {
        CorsLayer::permissive()
    };

    // Build the router with all routes
    let app = Router::new()
        // Health and status endpoints
        .route("/health", get(handlers::health::health_check))
        .route("/health/detailed", get(handlers::health::detailed_health))
        .route("/status", get(handlers::status::service_status))
        .route("/metrics", get(handlers::metrics::prometheus_metrics))
        // Blog API endpoints (EARLY-LAUNCH integration)
        .route(
            "/v1/blog/generate",
            post(handlers::blog_api::generate_blog_post),
        )
        .route(
            "/v1/workflows/:id",
            get(handlers::blog_api::get_workflow_status),
        )
        .route(
            "/v1/workflows/:id/cancel",
            post(handlers::blog_api::cancel_workflow),
        )
        .route("/v1/workflows", get(handlers::blog_api::list_workflows))
        .route(
            "/v1/clients/register",
            post(handlers::blog_api::register_client),
        )
        .route(
            "/v1/clients/profile",
            get(handlers::blog_api::get_client_profile),
        )
        .route(
            "/v1/clients/profile",
            put(handlers::blog_api::update_client_profile),
        )
        .route(
            "/v1/capabilities",
            get(handlers::blog_api::get_capabilities),
        )
        // Client management endpoints
        .route("/clients", post(handlers::clients::register_client))
        .route("/clients", get(handlers::clients::list_clients))
        .route("/clients/:id", get(handlers::clients::get_client))
        .route("/clients/:id", put(handlers::clients::update_client))
        .route("/clients/:id", delete(handlers::clients::delete_client))
        .route(
            "/clients/:id/usage",
            get(handlers::clients::get_client_usage),
        )
        // Provider management endpoints
        .route("/providers", post(handlers::providers::register_provider))
        .route("/providers", get(handlers::providers::list_providers))
        .route("/providers/:id", get(handlers::providers::get_provider))
        .route("/providers/:id", put(handlers::providers::update_provider))
        .route(
            "/providers/:id",
            delete(handlers::providers::delete_provider),
        )
        .route(
            "/providers/select",
            post(handlers::providers::select_provider),
        )
        // Schema translation endpoints
        .route(
            "/schema/translate",
            post(handlers::schema::translate_schema),
        )
        .route(
            "/schema/translations",
            get(handlers::schema::list_translations),
        )
        .route(
            "/schema/translations/:id",
            get(handlers::schema::get_translation),
        )
        // Workflow execution endpoints
        .route("/workflows", post(handlers::workflows::create_workflow))
        .route("/workflows", get(handlers::workflows::list_workflows))
        .route("/workflows/:id", get(handlers::workflows::get_workflow))
        .route("/workflows/:id", put(handlers::workflows::update_workflow))
        .route(
            "/workflows/:id",
            delete(handlers::workflows::delete_workflow),
        )
        .route(
            "/workflows/:id/execute",
            post(handlers::workflows::execute_workflow),
        )
        .route(
            "/workflows/:id/status",
            get(handlers::workflows::get_workflow_status),
        )
        .route(
            "/workflows/:id/cancel",
            post(handlers::workflows::cancel_workflow),
        )
        // MCP proxy endpoints
        .route(
            "/proxy/mcp/:server_id/*path",
            post(handlers::proxy::proxy_mcp_request),
        )
        .route(
            "/proxy/mcp/:server_id/*path",
            get(handlers::proxy::proxy_mcp_request),
        )
        .route(
            "/proxy/mcp/:server_id/*path",
            put(handlers::proxy::proxy_mcp_request),
        )
        .route(
            "/proxy/mcp/:server_id/*path",
            delete(handlers::proxy::proxy_mcp_request),
        )
        // Cost optimization endpoints
        .route("/cost/optimize", post(handlers::cost::optimize_selection))
        .route("/cost/reports", get(handlers::cost::get_cost_reports))
        .route(
            "/cost/reports/:client_id",
            get(handlers::cost::get_client_cost_report),
        )
        // Authentication endpoints
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/refresh", post(handlers::auth::refresh_token))
        .route("/auth/logout", post(handlers::auth::logout))
        // Add state
        .with_state(state)
        // Add middleware layers
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors_layer)
                .layer(middleware::from_fn_with_state(
                    rate_limit_middleware,
                    crate::middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    auth_middleware,
                    crate::middleware::auth::auth_middleware,
                ))
                .layer(DefaultBodyLimit::max(
                    config.server.max_request_size as usize,
                )),
        );

    Ok(app)
}

/// Global error handler for the application
impl IntoResponse for FederationError {
    fn into_response(self) -> Response {
        let status_code = match self {
            FederationError::ClientNotFound { .. } => StatusCode::NOT_FOUND,
            FederationError::ProviderNotFound { .. } => StatusCode::NOT_FOUND,
            FederationError::AuthenticationFailed { .. } => StatusCode::UNAUTHORIZED,
            FederationError::AuthorizationFailed { .. } => StatusCode::FORBIDDEN,
            FederationError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            FederationError::ResourceLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            FederationError::ConfigurationError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error_response = serde_json::json!({
            "error": {
                "type": format!("{:?}", self).split('(').next().unwrap_or("UnknownError"),
                "message": self.to_string(),
                "timestamp": chrono::Utc::now(),
                "status": status_code.as_u16()
            }
        });

        (status_code, Json(error_response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_server_creation() {
        let config = Arc::new(Config::default());

        // This would require proper mocks in a real test
        // For now, just test that the function signature works
        assert!(config.server.port > 0);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        // This would test the health endpoint with a test server
        // Requires proper setup with mocked dependencies
    }

    #[tokio::test]
    async fn test_error_responses() {
        let error = FederationError::ClientNotFound { id: Uuid::new_v4() };
        let response = error.into_response();

        // In a real test, we would check the response status and body
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
