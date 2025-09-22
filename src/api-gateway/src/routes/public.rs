//! Public routes that don't require authentication

use axum::{routing::get, Router};

use crate::{handlers, state::AppState};

/// Create public routes router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/info", get(handlers::health::system_info))
        .route("/liveness", get(handlers::health::liveness))
        .route("/readiness", get(handlers::health::readiness))
        .route("/metrics", get(metrics_handler))
}

/// Prometheus metrics handler
async fn metrics_handler() -> String {
    // Return Prometheus metrics
    let metric_families = prometheus::gather();
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    match encoder.encode_to_string(&metric_families) {
        Ok(output) => output,
        Err(_) => "# Failed to encode metrics\n".to_string(),
    }
}
