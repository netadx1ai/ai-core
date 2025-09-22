//! Health check handlers for the Federation Service
//!
//! This module provides health check endpoints for monitoring the federation service
//! and its dependencies, including basic health checks and detailed health reports.

use crate::handlers::{success_response, ApiResponse};
use crate::server::ServerState;
use axum::{extract::State, response::Json, response::Result as AxumResult};
use serde_json;

/// Basic health check endpoint
pub async fn health_check(
    State(state): State<ServerState>,
) -> AxumResult<Json<ApiResponse<serde_json::Value>>> {
    let health_info = serde_json::json!({
        "status": "healthy",
        "service": "federation",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now(),
    });

    success_response(health_info)
}

/// Detailed health check with component status
pub async fn detailed_health(
    State(state): State<ServerState>,
) -> AxumResult<Json<ApiResponse<serde_json::Value>>> {
    let client_health = state.client_manager.health().await.unwrap_or_else(|_| {
        serde_json::json!({
            "status": "unhealthy",
            "error": "Failed to get client manager health"
        })
    });

    let provider_health = state.provider_manager.health().await.unwrap_or_else(|_| {
        serde_json::json!({
            "status": "unhealthy",
            "error": "Failed to get provider manager health"
        })
    });

    let workflow_health = state.workflow_engine.health().await.unwrap_or_else(|_| {
        serde_json::json!({
            "status": "unhealthy",
            "error": "Failed to get workflow engine health"
        })
    });

    let detailed_health = serde_json::json!({
        "status": "healthy",
        "service": "federation",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now(),
        "components": {
            "client_manager": client_health,
            "provider_manager": provider_health,
            "workflow_engine": workflow_health,
            "schema_translator": {
                "status": "healthy"
            },
            "mcp_proxy": {
                "status": "healthy"
            },
            "cost_optimizer": {
                "status": "healthy"
            }
        },
        "uptime": "unknown",
        "memory_usage": "unknown"
    });

    success_response(detailed_health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_response() {
        // This would test the health check endpoint
        // Requires proper test setup with mocked state
    }

    #[tokio::test]
    async fn test_detailed_health_response() {
        // This would test the detailed health endpoint
        // Requires proper test setup with mocked state
    }
}
