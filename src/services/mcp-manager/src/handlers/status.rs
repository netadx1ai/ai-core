//! Status Handlers
//!
//! This module provides HTTP handlers for service status endpoints,
//! including overall service status and operational information.

use crate::server::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::info;

/// Service status response
#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Service status
    pub status: String,
    /// Environment
    pub environment: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Build information
    pub build_info: BuildInfo,
}

/// Build information
#[derive(Debug, Serialize)]
pub struct BuildInfo {
    /// Git commit hash
    pub commit: String,
    /// Build timestamp
    pub build_time: String,
    /// Rust version
    pub rust_version: String,
}

/// Get service status
///
/// Returns basic service information and operational status.
pub async fn server_status(
    State(state): State<AppState>,
) -> Result<Json<ServiceStatus>, StatusCode> {
    let status = ServiceStatus {
        service: "MCP Manager".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: "running".to_string(),
        environment: state.config.environment.clone(),
        timestamp: Utc::now(),
        uptime_seconds: 0, // TODO: Track actual uptime
        build_info: BuildInfo {
            commit: std::env::var("VERGEN_GIT_SHA").unwrap_or_else(|_| "unknown".to_string()),
            build_time: std::env::var("VERGEN_BUILD_TIMESTAMP")
                .unwrap_or_else(|_| "unknown".to_string()),
            rust_version: std::env::var("VERGEN_RUSTC_SEMVER")
                .unwrap_or_else(|_| "unknown".to_string()),
        },
    };

    info!("Service status requested");
    Ok(Json(status))
}
