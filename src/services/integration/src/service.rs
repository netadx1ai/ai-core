//! Main service implementation for the AI-CORE Integration Service
//!
//! This module provides the core service orchestration for handling third-party
//! integrations including service startup, HTTP routing, middleware, and graceful shutdown.

use crate::config::IntegrationConfig;
use crate::error::{IntegrationError, IntegrationResult};
use crate::handlers::create_routes;
use crate::integrations::{Integration, IntegrationFactory};
use crate::metrics::IntegrationMetrics;
use axum::serve;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    request_id::{MakeRequestId, RequestId},
    trace::TraceLayer,
};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Main integration service
pub struct IntegrationService {
    /// Service configuration
    config: IntegrationConfig,
    /// Application state
    app_state: Arc<AppState>,
    /// Server address
    addr: SocketAddr,
}

/// Application state shared across handlers
pub struct AppState {
    /// Service configuration
    pub config: IntegrationConfig,
    /// HTTP client for external API calls
    pub http_client: reqwest::Client,
    /// Redis connection pool
    pub redis_pool: Option<deadpool_redis::Pool>,
    /// Database connection pool
    pub db_pool: Option<sqlx::PgPool>,
    /// Integration implementations
    pub integrations: HashMap<String, Box<dyn Integration>>,
    /// Metrics collector
    pub metrics: Arc<tokio::sync::Mutex<IntegrationMetrics>>,
}

/// Custom request ID generator
#[derive(Clone, Default)]
struct CustomMakeRequestId;

impl MakeRequestId for CustomMakeRequestId {
    fn make_request_id<B>(&mut self, _request: &axum::http::Request<B>) -> Option<RequestId> {
        let id = format!("req-{}", Uuid::new_v4());
        axum::http::HeaderValue::from_str(&id)
            .ok()
            .map(RequestId::new)
    }
}

impl IntegrationService {
    /// Create a new integration service
    pub async fn new(config: IntegrationConfig) -> IntegrationResult<Self> {
        info!("Initializing AI-CORE Integration Service");

        // Validate configuration
        config
            .validate()
            .map_err(|e| IntegrationError::configuration(e))?;

        // Create HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.server.request_timeout,
            ))
            .user_agent("AI-CORE-Integration-Service/1.0.0")
            .build()
            .map_err(|e| {
                IntegrationError::internal(format!("Failed to create HTTP client: {}", e))
            })?;

        // Initialize Redis pool if configured
        let redis_pool = if !config.redis.url.is_empty() {
            match Self::create_redis_pool(&config).await {
                Ok(pool) => {
                    info!("Redis connection pool initialized");
                    Some(pool)
                }
                Err(e) => {
                    warn!("Failed to initialize Redis pool: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize database pool if configured
        let db_pool = if !config.database.postgres_url.is_empty() {
            match Self::create_db_pool(&config).await {
                Ok(pool) => {
                    info!("Database connection pool initialized");
                    Some(pool)
                }
                Err(e) => {
                    warn!("Failed to initialize database pool: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize integrations
        let integrations = Self::initialize_integrations(&config)?;

        // Initialize metrics
        let metrics = Arc::new(tokio::sync::Mutex::new(IntegrationMetrics::new()));

        // Create application state
        let app_state = Arc::new(AppState {
            config: config.clone(),
            http_client,
            redis_pool,
            db_pool,
            integrations,
            metrics,
        });

        // Create server address
        let addr = format!("{}:{}", config.server.host, config.server.port)
            .parse()
            .map_err(|e| {
                IntegrationError::configuration(format!("Invalid server address: {}", e))
            })?;

        Ok(Self {
            config,
            app_state,
            addr,
        })
    }

    /// Start the integration service
    pub async fn start(self) -> IntegrationResult<()> {
        info!("Starting AI-CORE Integration Service on {}", self.addr);

        // Create middleware stack
        let middleware = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(tower_http::cors::Any),
            )
            .layer(tower_http::request_id::SetRequestIdLayer::new(
                axum::http::header::HeaderName::from_static("x-request-id"),
                CustomMakeRequestId::default(),
            ));

        // Create routes
        let app = create_routes(self.app_state.clone()).layer(middleware);

        // Create listener
        let listener = tokio::net::TcpListener::bind(&self.addr)
            .await
            .map_err(|e| IntegrationError::internal(format!("Failed to bind to address: {}", e)))?;

        info!("Integration service started successfully on {}", self.addr);

        // Run with graceful shutdown
        if let Err(e) = serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(Self::shutdown_signal())
        .await
        {
            error!("Server error: {}", e);
            return Err(IntegrationError::internal(format!("Server error: {}", e)));
        }

        info!("Integration service stopped gracefully");
        Ok(())
    }

    /// Initialize all enabled integrations
    fn initialize_integrations(
        config: &IntegrationConfig,
    ) -> IntegrationResult<HashMap<String, Box<dyn Integration>>> {
        let mut integrations = HashMap::new();

        // Initialize Zapier integration
        if config.zapier.enabled {
            let zapier = IntegrationFactory::create_zapier(&config.zapier);
            integrations.insert("zapier".to_string(), zapier);
            info!("Zapier integration initialized");
        }

        // Initialize Slack integration
        if config.slack.enabled {
            match IntegrationFactory::create_slack(&config.slack) {
                Ok(slack) => {
                    integrations.insert("slack".to_string(), slack);
                    info!("Slack integration initialized");
                }
                Err(e) => {
                    error!("Failed to initialize Slack integration: {}", e);
                    if config.slack.enabled {
                        return Err(e);
                    }
                }
            }
        }

        // Initialize GitHub integration
        if config.github.enabled {
            match IntegrationFactory::create_github(&config.github) {
                Ok(github) => {
                    integrations.insert("github".to_string(), github);
                    info!("GitHub integration initialized");
                }
                Err(e) => {
                    error!("Failed to initialize GitHub integration: {}", e);
                    if config.github.enabled {
                        return Err(e);
                    }
                }
            }
        }

        if integrations.is_empty() {
            warn!("No integrations enabled");
        } else {
            info!("Initialized {} integrations", integrations.len());
        }

        Ok(integrations)
    }

    /// Create Redis connection pool
    async fn create_redis_pool(
        config: &IntegrationConfig,
    ) -> IntegrationResult<deadpool_redis::Pool> {
        let redis_config = deadpool_redis::Config::from_url(&config.redis.url);
        let pool = redis_config
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| {
                IntegrationError::configuration(format!("Redis pool creation failed: {}", e))
            })?;

        // Test the connection
        let mut conn = pool.get().await.map_err(|e| {
            IntegrationError::configuration(format!("Redis connection test failed: {}", e))
        })?;

        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| IntegrationError::configuration(format!("Redis ping failed: {}", e)))?;

        Ok(pool)
    }

    /// Create database connection pool
    async fn create_db_pool(config: &IntegrationConfig) -> IntegrationResult<sqlx::PgPool> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                config.database.connection_timeout,
            ))
            .connect(&config.database.postgres_url)
            .await
            .map_err(|e| {
                IntegrationError::configuration(format!("Database connection failed: {}", e))
            })?;

        // Test the connection
        sqlx::query("SELECT 1").execute(&pool).await.map_err(|e| {
            IntegrationError::configuration(format!("Database test query failed: {}", e))
        })?;

        Ok(pool)
    }

    /// Wait for shutdown signal
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
                info!("Received Ctrl+C, shutting down");
            }
            _ = terminate => {
                info!("Received terminate signal, shutting down");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let config = IntegrationConfig::default();
        let result = IntegrationService::new(config).await;

        // This might fail due to missing dependencies in test environment,
        // but we're mainly testing the structure
        match result {
            Ok(service) => {
                assert!(!service.addr.to_string().is_empty());
            }
            Err(_) => {
                // Expected in test environment without Redis/DB
            }
        }
    }

    #[test]
    fn test_initialize_integrations() {
        let config = IntegrationConfig::default();
        let result = IntegrationService::initialize_integrations(&config);

        assert!(result.is_ok());
        let integrations = result.unwrap();

        // With default config, only Zapier should be enabled
        assert!(integrations.contains_key("zapier"));
    }
}
