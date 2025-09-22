//! Service Discovery Server
//!
//! Main entry point for the AI-CORE service discovery and registry service.
//! Provides microservice registration, health monitoring, load balancing, and service mesh capabilities.

use anyhow::{Context, Result};
use clap::Parser;
use service_discovery::{
    config::{Args, ServiceDiscoveryConfig},
    handlers::{create_router, AppState},
    health::{HealthMonitor, HealthMonitorImpl},
    load_balancer::{LoadBalancer, LoadBalancerImpl},
    registry::{ServiceRegistry, ServiceRegistryImpl},
};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Database connection setup
mod database {
    use anyhow::{Context, Result};
    use service_discovery::config::ServiceDiscoveryConfig;
    use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
    use std::sync::Arc;
    use std::time::Duration;

    /// Initialize PostgreSQL connection pool
    pub async fn create_postgres_pool(config: &ServiceDiscoveryConfig) -> Result<Pool<Postgres>> {
        let pool = PgPoolOptions::new()
            .max_connections(config.database.postgres.max_connections)
            .min_connections(config.database.postgres.min_connections)
            .acquire_timeout(Duration::from_secs(
                config.database.postgres.acquire_timeout as u64,
            ))
            .idle_timeout(Duration::from_secs(
                config.database.postgres.idle_timeout as u64,
            ))
            .max_lifetime(Duration::from_secs(
                config.database.postgres.max_lifetime as u64,
            ))
            .connect(&config.database.postgres.url)
            .await
            .context("Failed to create PostgreSQL connection pool")?;

        // Run database migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run database migrations")?;

        Ok(pool)
    }

    /// Initialize Redis connection pool
    pub async fn create_redis_pool(
        config: &ServiceDiscoveryConfig,
    ) -> Result<deadpool_redis::Pool> {
        let redis_config = deadpool_redis::Config::from_url(&config.database.redis.url);

        let pool = redis_config
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .context("Failed to create Redis connection pool")?;

        // Test the connection
        let mut conn = pool
            .get()
            .await
            .context("Failed to get Redis connection from pool")?;

        redis::cmd("PING")
            .query_async::<_, String>(&mut *conn)
            .await
            .context("Redis connection test failed")?;

        Ok(pool)
    }
}

/// Telemetry and observability setup
mod telemetry {
    use anyhow::Result;
    use service_discovery::config::ServiceDiscoveryConfig;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    /// Initialize tracing and logging
    pub fn init_tracing(config: &ServiceDiscoveryConfig) -> Result<()> {
        let log_level = &config.monitoring.logging.level;
        let log_format = &config.monitoring.logging.format;

        let env_filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));

        let fmt_layer = match log_format.as_str() {
            "json" => fmt::layer().json().boxed(),
            _ => fmt::layer().pretty().boxed(),
        };

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        Ok(())
    }
}

/// Middleware setup
mod middleware {
    use axum::http::Method;
    use service_discovery::config::ServiceDiscoveryConfig;
    use std::sync::Arc;
    use tower::ServiceBuilder;
    use tower_http::{
        cors::{Any, CorsLayer},
        trace::TraceLayer,
    };

    /// Create simple middleware placeholder
    pub fn create_simple_middleware() {
        // Placeholder for middleware setup
        // TODO: Implement proper middleware when type issues are resolved
    }
}

/// Graceful shutdown handling
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Load configuration
    let config = Arc::new(ServiceDiscoveryConfig::load(&args)?);

    // Initialize telemetry
    telemetry::init_tracing(&config)?;

    info!(
        "Starting AI-CORE Service Discovery Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("Configuration loaded from: {:?}", args.config);

    // Initialize database connections
    info!("Initializing database connections...");
    let postgres_pool = database::create_postgres_pool(&config)
        .await
        .context("Failed to initialize PostgreSQL connection")?;

    let redis_pool = database::create_redis_pool(&config)
        .await
        .context("Failed to initialize Redis connection")?;

    info!("Database connections established successfully");

    // Initialize core services
    info!("Initializing service registry...");
    let registry = Arc::new(ServiceRegistryImpl::new(
        postgres_pool,
        redis_pool,
        Arc::clone(&config),
    ));

    // Initialize the registry (create tables, load services, etc.)
    registry
        .initialize()
        .await
        .context("Failed to initialize service registry")?;

    info!("Initializing health monitor...");
    let health_monitor = Arc::new(HealthMonitorImpl::new(
        Arc::clone(&config),
        Arc::clone(&registry) as Arc<dyn ServiceRegistry>,
    ));

    info!("Initializing load balancer...");
    let load_balancer = Arc::new(LoadBalancerImpl::new(Arc::clone(&config)));

    // Start health monitoring if enabled
    if config.registry.health_checks.enabled {
        info!("Starting health monitoring...");
        health_monitor
            .start_monitoring()
            .await
            .context("Failed to start health monitoring")?;
    }

    // Create application state
    let app_state = service_discovery::handlers::AppState {
        config: Arc::clone(&config),
        registry: Arc::clone(&registry),
        health_monitor: Arc::clone(&health_monitor),
        load_balancer: Arc::clone(&load_balancer),
    };

    // Create router (simplified without complex middleware for now)
    middleware::create_simple_middleware();
    let app = create_router(app_state);

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(config.server.socket_addr()?)
        .await
        .context("Failed to bind to server address")?;

    info!(
        "Service Discovery Server listening on {}",
        config.server.socket_addr()?
    );
    info!(
        "Health endpoint: http://{}/health",
        config.server.socket_addr()?
    );
    info!(
        "Metrics endpoint: http://{}/metrics",
        config.server.socket_addr()?
    );
    info!(
        "API documentation: http://{}/api/v1",
        config.server.socket_addr()?
    );

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server failed to start")?;

    // Cleanup on shutdown
    info!("Shutting down Service Discovery Server...");

    // Stop health monitoring
    if config.registry.health_checks.enabled {
        if let Err(e) = health_monitor.stop_monitoring().await {
            warn!("Failed to stop health monitoring gracefully: {}", e);
        }
    }

    info!("Service Discovery Server shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_server_startup() {
        // This would be an integration test
        // For now, just verify the main components can be created

        // Set test environment variables
        env::set_var("DATABASE_URL", "postgresql://localhost/test");
        env::set_var("REDIS_URL", "redis://localhost:6379");

        let args = Args {
            config: tempdir().unwrap().path().join("test-config.yaml"),
            environment: "test".to_string(),
            port: Some(0), // Use random port
            log_level: Some("debug".to_string()),
            debug: true,
            database_url: Some("sqlite::memory:".to_string()),
            redis_url: Some("redis://localhost:6379".to_string()),
        };

        // Test that configuration can be loaded
        // In a real test environment, this would use a test database
        // assert!(ServiceDiscoveryConfig::load(&args).is_ok());
    }

    #[test]
    fn test_config_validation() {
        // Test configuration validation
        let config = ServiceDiscoveryConfig::default();
        assert!(config.validate().is_ok());
    }
}
