//! Health check handlers

use axum::{extract::State, Json};

use crate::{error::Result, state::AppState};
use ai_core_shared::types::core::{ServiceHealth, SystemInfo};

/// Get system health status
pub async fn health_check(State(state): State<AppState>) -> Result<Json<Vec<ServiceHealth>>> {
    let health_status = state.health_service.check_all().await?;
    Ok(Json(health_status))
}

/// Get system information
pub async fn system_info(State(state): State<AppState>) -> Result<Json<SystemInfo>> {
    let info = state.health_service.get_system_info();
    Ok(Json(info))
}

/// Simple liveness probe
pub async fn liveness() -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now()
    })))
}

/// Readiness probe
pub async fn readiness(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    let is_ready = state.is_healthy().await;

    Ok(Json(serde_json::json!({
        "status": if is_ready { "ready" } else { "not_ready" },
        "timestamp": chrono::Utc::now()
    })))
}
