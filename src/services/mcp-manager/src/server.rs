//! MCP Manager Server Module
//!
//! This module provides the main server implementation for the MCP Manager Service,
//! handling HTTP requests, server lifecycle, and coordinating all service components.

use crate::{
    config::Config,
    handlers,
    health::{HealthConfig, HealthMonitor},
    load_balancer::{LoadBalancer, LoadBalancerConfig},
    middleware,
    registry::{RegistryConfig, ServerRegistry},
    telemetry::setup_metrics,
    McpError, Result,
};
use axum::{
    extract::DefaultBodyLimit,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Method,
    },
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info};

/// MCP Manager server
#[derive(Debug)]
pub struct McpManagerServer {
    /// Server configuration
    config: Config,

    /// Server registry
    registry: Arc<ServerRegistry>,

    /// Health monitor
    health_monitor: Arc<HealthMonitor>,

    /// Application state
    app_state: AppState,
}

/// Application state shared across handlers
#[derive(Debug, Clone)]
pub struct AppState {
    /// Configuration
    pub config: Config,

    /// Server registry
    pub registry: Arc<ServerRegistry>,

    /// Health monitor
    pub health_monitor: Arc<HealthMonitor>,

    /// Load balancer
    pub load_balancer: Arc<LoadBalancer>,

    /// HTTP client
    pub http_client: reqwest::Client,
}

impl McpManagerServer {
    /// Create a new MCP Manager server
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing MCP Manager Server");

        // Create server registry
        let registry_config = RegistryConfig {
            max_servers: config.mcp.max_servers,
            auto_cleanup: true,
            cleanup_interval_seconds: 300,
            stale_timeout_seconds: 3600,
        };
        let registry = Arc::new(ServerRegistry::new(registry_config));

        // Create health monitor
        let health_config = HealthConfig {
            check_interval_seconds: config.health.check_interval_seconds,
            check_timeout_seconds: config.health.check_timeout_seconds,
            failure_threshold: config.health.failure_threshold,
            success_threshold: config.health.success_threshold,
            detailed_metrics: config.health.detailed_metrics,
            endpoints: config.health.endpoints.clone(),
            auto_recovery: true,
            max_recovery_attempts: 3,
            recovery_backoff_multiplier: 2.0,
        };
        let health_monitor = Arc::new(HealthMonitor::new(Arc::clone(&registry), health_config));

        // Create load balancer
        let load_balancer_config = LoadBalancerConfig {
            strategy: match config.load_balancer.strategy {
                crate::config::LoadBalancingStrategy::RoundRobin => {
                    crate::load_balancer::LoadBalancingStrategyType::RoundRobin
                }
                crate::config::LoadBalancingStrategy::LeastConnections => {
                    crate::load_balancer::LoadBalancingStrategyType::LeastConnections
                }
                crate::config::LoadBalancingStrategy::WeightedRoundRobin => {
                    crate::load_balancer::LoadBalancingStrategyType::WeightedRoundRobin
                }
                crate::config::LoadBalancingStrategy::Random => {
                    crate::load_balancer::LoadBalancingStrategyType::Random
                }
                crate::config::LoadBalancingStrategy::IpHash => {
                    crate::load_balancer::LoadBalancingStrategyType::IpHash
                }
                crate::config::LoadBalancingStrategy::ConsistentHash => {
                    crate::load_balancer::LoadBalancingStrategyType::ConsistentHash
                }
            },
            sticky_sessions: config.load_balancer.sticky_sessions,
            session_timeout_seconds: config.load_balancer.session_timeout_seconds,
            max_requests_per_server: config.load_balancer.max_requests_per_server,
            circuit_breaker_enabled: config.load_balancer.circuit_breaker,
            circuit_breaker_config: crate::load_balancer::CircuitBreakerConfig {
                failure_threshold: config
                    .load_balancer
                    .circuit_breaker_config
                    .failure_threshold,
                min_requests: config.load_balancer.circuit_breaker_config.min_requests,
                window_seconds: config.load_balancer.circuit_breaker_config.window_seconds,
                recovery_timeout_seconds: config
                    .load_balancer
                    .circuit_breaker_config
                    .recovery_timeout_seconds,
                half_open_max_requests: 5,
            },
            health_aware: true,
            server_weights: std::collections::HashMap::new(),
        };
        let load_balancer = Arc::new(LoadBalancer::new(
            Arc::clone(&registry),
            load_balancer_config,
        ));

        // Create HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.server.timeout_seconds))
            .user_agent("AI-CORE MCP Manager")
            .build()
            .map_err(|e| McpError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        // Create application state
        let app_state = AppState {
            config: config.clone(),
            registry: Arc::clone(&registry),
            health_monitor: Arc::clone(&health_monitor),
            load_balancer: Arc::clone(&load_balancer),
            http_client,
        };

        Ok(Self {
            config,
            registry,
            health_monitor,
            app_state,
        })
    }

    /// Start the server
    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting MCP Manager Server on {}:{}",
            self.config.server.host, self.config.server.port
        );

        // Setup metrics if enabled
        if self.config.metrics.enabled {
            setup_metrics(&self.config.metrics).await?;
        }

        // Start background services
        self.start_background_services().await?;

        // Create the application router
        let app = self.create_router().await?;

        // Create the listener
        let listener = TcpListener::bind(&self.config.server_address())
            .await
            .map_err(|e| McpError::Internal(format!("Failed to bind to address: {}", e)))?;

        info!(
            "MCP Manager Server listening on {}",
            self.config.server_address()
        );

        // Start the server
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| McpError::Internal(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down MCP Manager Server");
        // Perform cleanup operations here
        info!("MCP Manager Server shutdown complete");
        Ok(())
    }

    /// Create the application router with all routes and middleware
    async fn create_router(&self) -> Result<Router> {
        // Create CORS layer
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
            .allow_origin(Any);

        // Create the main application router
        let app = Router::new()
            // Health and status endpoints
            .route("/health", get(handlers::health::health_check))
            .route("/health/detailed", get(handlers::health::detailed_health))
            .route("/status", get(handlers::status::server_status))
            // Server management endpoints
            .route("/servers", get(handlers::servers::list_servers))
            .route("/servers", post(handlers::servers::register_server))
            .route("/servers/:id", get(handlers::servers::get_server))
            .route("/servers/:id", put(handlers::servers::update_server))
            .route("/servers/:id", delete(handlers::servers::deregister_server))
            .route(
                "/servers/:id/status",
                put(handlers::servers::update_server_status),
            )
            .route(
                "/servers/:id/health",
                get(handlers::health::get_server_health),
            )
            .route(
                "/servers/:id/health",
                post(handlers::health::check_server_health),
            )
            // Protocol endpoints
            .route("/protocol/request", post(handlers::protocol::send_request))
            .route(
                "/protocol/notification",
                post(handlers::protocol::send_notification),
            )
            .route("/protocol/batch", post(handlers::protocol::batch_request))
            // Load balancer endpoints
            .route(
                "/load-balancer/select",
                post(handlers::load_balancer::select_server),
            )
            .route(
                "/load-balancer/stats",
                get(handlers::load_balancer::get_statistics),
            )
            .route(
                "/load-balancer/weights",
                put(handlers::load_balancer::update_weights),
            )
            // Registry endpoints
            .route("/registry/stats", get(handlers::registry::get_statistics))
            .route("/registry/cleanup", post(handlers::registry::cleanup_stale))
            // Metrics endpoint (if enabled)
            .route("/metrics", get(handlers::metrics::prometheus_metrics))
            // Add middleware
            .layer(
                ServiceBuilder::new()
                    // Request ID
                    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                    .layer(PropagateRequestIdLayer::x_request_id())
                    // Tracing
                    .layer(TraceLayer::new_for_http())
                    // Compression
                    .layer(CompressionLayer::new())
                    // CORS
                    .layer(cors)
                    // Request size limit
                    .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB
                    // Custom middleware
                    .layer(axum_middleware::from_fn_with_state(
                        self.app_state.clone(),
                        middleware::auth::auth_middleware,
                    ))
                    .layer(axum_middleware::from_fn_with_state(
                        self.app_state.clone(),
                        middleware::rate_limit::rate_limit_middleware,
                    ))
                    .layer(axum_middleware::from_fn_with_state(
                        self.app_state.clone(),
                        middleware::request_logging::request_logging_middleware,
                    )),
            )
            .with_state(self.app_state.clone());

        Ok(app)
    }

    /// Start background services
    async fn start_background_services(&self) -> Result<()> {
        // Start health monitoring
        if self.config.health.enabled {
            let health_monitor = Arc::clone(&self.health_monitor);
            tokio::spawn(async move {
                if let Err(e) = health_monitor.start().await {
                    error!("Health monitor failed: {}", e);
                }
            });
        }

        // Start registry cleanup task
        let registry = Arc::clone(&self.registry);
        let cleanup_interval = Duration::from_secs(300); // 5 minutes
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                if let Err(e) = registry.cleanup_stale_servers().await {
                    error!("Registry cleanup failed: {}", e);
                }
            }
        });

        info!("Background services started");
        Ok(())
    }
}

/// Wait for shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        }
    }
}

impl AppState {
    /// Get server registry
    pub fn registry(&self) -> &Arc<ServerRegistry> {
        &self.registry
    }

    /// Get health monitor
    pub fn health_monitor(&self) -> &Arc<HealthMonitor> {
        &self.health_monitor
    }

    /// Get load balancer
    pub fn load_balancer(&self) -> &Arc<LoadBalancer> {
        &self.load_balancer
    }

    /// Get HTTP client
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Check if metrics are enabled
    pub fn metrics_enabled(&self) -> bool {
        self.config.metrics.enabled
    }

    /// Check if development mode is enabled
    pub fn is_development(&self) -> bool {
        self.config.is_development()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    async fn create_test_config() -> Config {
        let yaml_content = r#"
environment: "development"
server:
  host: "127.0.0.1"
  port: 8083
mcp:
  max_servers: 50
health:
  enabled: true
  check_interval_seconds: 30
  check_timeout_seconds: 5
  failure_threshold: 3
  success_threshold: 2
  detailed_metrics: true
  endpoints: ["/health"]
load_balancer:
  strategy: "round_robin"
  sticky_sessions: false
  session_timeout_seconds: 1800
  max_requests_per_server: 100
  circuit_breaker: true
  circuit_breaker_config:
    failure_threshold: 50
    min_requests: 10
    window_seconds: 60
    recovery_timeout_seconds: 30
security:
  jwt_enabled: false
  api_key_enabled: false
logging:
  level: "info"
  format: "json"
metrics:
  enabled: false
rate_limiting:
  enabled: false
integrations: {}
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).await.unwrap();
        Config::from_file(temp_file.path()).await.unwrap()
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = create_test_config().await;
        let server = McpManagerServer::new(config).await.unwrap();

        assert!(server.registry.count().await == 0);
    }

    #[tokio::test]
    async fn test_app_state() {
        let config = create_test_config().await;
        let server = McpManagerServer::new(config).await.unwrap();

        assert!(!server.app_state.metrics_enabled());
        assert!(server.app_state.is_development());
    }
}
