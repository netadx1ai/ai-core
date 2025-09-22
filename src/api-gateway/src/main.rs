//! AI-PLATFORM API Gateway
//!
//! High-performance API Gateway service for the AI-PLATFORM Intelligent Automation Platform.
//! Provides centralized authentication, rate limiting, routing, and observability.

use std::net::SocketAddr;

use axum::{middleware, Router};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, request_id::SetRequestIdLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod handlers;
mod middleware_layer;
mod routes;
mod services;
mod state;

use config::Config;
use error::Result;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing()?;

    info!(
        "Starting AI-PLATFORM API Gateway v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let config = Config::from_env()?;
    info!(
        "Configuration loaded for environment: {}",
        config.environment
    );

    // Initialize application state with graceful degradation
    let state = match AppState::new(config.clone()).await {
        Ok(state) => {
            info!("Application state initialized successfully");
            state
        }
        Err(e) => {
            warn!("Failed to initialize full application state: {}", e);
            info!("Starting in degraded mode without database/redis connections");
            AppState::new_degraded(config.clone()).await?
        }
    };

    // Build the application router
    let app = build_router(state);

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("API Gateway listening on {}", addr);
    info!("Health check endpoint: http://{}/health", addr);
    info!("Metrics endpoint: http://{}/metrics", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("API Gateway shutdown complete");
    Ok(())
}

/// Build the main application router with all middleware and routes
fn build_router(state: AppState) -> Router {
    let api_routes = routes::api::router()
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_layer::auth::auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_layer::rate_limit::rate_limit_middleware,
        ));

    let public_routes = routes::public::router();

    Router::new()
        .nest("/v1", api_routes)
        .merge(public_routes)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(
                    tower_http::request_id::MakeRequestUuid,
                ))
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive()) // Configure CORS appropriately for production
                .layer(middleware::from_fn(
                    middleware_layer::logging::logging_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middleware_layer::error_handling::error_handling_middleware,
                )),
        )
        .with_state(state)
}

/// Initialize distributed tracing and logging
fn init_tracing() -> Result<()> {
    // Try to initialize Jaeger tracing, but don't fail if Jaeger is unavailable
    let opentelemetry_layer = match opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("AI-PLATFORM-api-gateway")
        .with_endpoint("http://jaeger:14268/api/traces")
        .install_simple()
    {
        Ok(tracer) => {
            info!("Jaeger tracing initialized successfully");
            Some(tracing_opentelemetry::layer().with_tracer(tracer))
        }
        Err(e) => {
            warn!(
                "Failed to initialize Jaeger tracing: {}. Continuing without distributed tracing.",
                e
            );
            None
        }
    };

    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "ai_core_api_gateway=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().json());

    if let Some(otel_layer) = opentelemetry_layer {
        subscriber.with(otel_layer).init();
    } else {
        subscriber.init();
    }

    Ok(())
}

/// Graceful shutdown signal handler
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
            warn!("Received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            warn!("Received SIGTERM, shutting down gracefully");
        },
    }
}
