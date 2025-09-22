//! AI-PLATFORM API Gateway Library
//!
//! This library provides the core functionality of the AI-PLATFORM API Gateway,
//! including routing, middleware, authentication, and service management.

pub mod config;
pub mod error;
pub mod handlers;
pub mod middleware_layer;
pub mod routes;
pub mod services;
pub mod state;

// Re-export main types and functions for external use
pub use config::{
    AuthConfig, Config, DatabaseConfig, ObservabilityConfig, RateLimitConfig, RedisConfig,
    RoutingConfig, ServerConfig, ServiceConfig,
};
pub use error::{ApiError, Result};
pub use state::AppState;

use axum::Router;

/// Build the main application router with all middleware and routes
pub fn build_router(state: AppState) -> Router {
    use axum::middleware;
    use tower::ServiceBuilder;
    use tower_http::{
        compression::CompressionLayer, cors::CorsLayer, request_id::SetRequestIdLayer,
        trace::TraceLayer,
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_exports() {
        // Test that main types can be imported
        let _config_type: Option<Config> = None;
        let _state_type: Option<AppState> = None;
        let _error_type: Option<ApiError> = None;

        // If compilation passes, exports are working
        assert!(true);
    }
}
