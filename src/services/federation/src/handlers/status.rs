//! Status handlers for the Federation Service
//!
//! This module provides status endpoints for monitoring the federation service
//! operational status, service information, and runtime statistics.

use crate::handlers::{success_response, ApiResponse};
use crate::server::ServerState;
use axum::{extract::State, response::Json, response::Result as AxumResult};
use serde_json;

/// Service status endpoint
pub async fn service_status(
    State(state): State<ServerState>,
) -> AxumResult<Json<ApiResponse<serde_json::Value>>> {
    let status_info = serde_json::json!({
        "service": "federation",
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "build_time": std::env::var("VERGEN_BUILD_TIMESTAMP").unwrap_or_else(|_| "unknown".to_string()),
        "git_hash": std::env::var("VERGEN_GIT_SHA_SHORT").unwrap_or_else(|_| "unknown".to_string()),
        "rust_version": std::env::var("VERGEN_RUSTC_SEMVER").unwrap_or_else(|_| "unknown".to_string()),
        "timestamp": chrono::Utc::now(),
        "environment": format!("{:?}", state.config.environment),
        "server": {
            "host": state.config.server.host,
            "port": state.config.server.port,
            "cors_enabled": state.config.server.enable_cors
        },
        "features": {
            "schema_translation": state.config.features.schema_translation,
            "cost_optimization": state.config.features.cost_optimization,
            "advanced_analytics": state.config.features.advanced_analytics
        }
    });

    success_response(status_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_status_response() {
        // This would test the service status endpoint
        // Requires proper test setup with mocked state
    }
}
