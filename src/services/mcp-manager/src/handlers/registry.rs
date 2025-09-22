//! Registry Handlers
//!
//! This module provides HTTP handlers for server registry management,
//! including registry statistics and cleanup operations.

use crate::server::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use std::collections::HashMap;
use tracing::{error, info};

/// Registry statistics response
#[derive(Debug, Serialize)]
pub struct RegistryStatsResponse {
    /// Total servers
    pub total_servers: usize,
    /// Server count by status
    pub status_counts: HashMap<String, usize>,
    /// Server count by type
    pub type_counts: HashMap<String, usize>,
    /// Number of unique tags
    pub tag_counts: usize,
    /// Number of unique owners
    pub owner_counts: usize,
}

/// Cleanup result response
#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    /// Number of servers cleaned up
    pub cleaned_up_count: usize,
    /// List of cleaned up server IDs
    pub cleaned_up_servers: Vec<String>,
    /// Cleanup timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Get registry statistics
pub async fn get_statistics(
    State(state): State<AppState>,
) -> Result<Json<RegistryStatsResponse>, StatusCode> {
    let stats = state.registry().get_statistics().await;

    let status_counts = stats
        .status_counts
        .iter()
        .map(|(status, count)| (status.to_string(), *count))
        .collect();

    let response = RegistryStatsResponse {
        total_servers: stats.total_servers,
        status_counts,
        type_counts: stats.type_counts,
        tag_counts: stats.tag_counts,
        owner_counts: stats.owner_counts,
    };

    info!(
        total_servers = stats.total_servers,
        "Registry statistics requested"
    );

    Ok(Json(response))
}

/// Clean up stale servers
pub async fn cleanup_stale(
    State(state): State<AppState>,
) -> Result<Json<CleanupResponse>, StatusCode> {
    match state.registry().cleanup_stale_servers().await {
        Ok(cleaned_up_servers) => {
            let response = CleanupResponse {
                cleaned_up_count: cleaned_up_servers.len(),
                cleaned_up_servers: cleaned_up_servers.iter().map(|id| id.to_string()).collect(),
                timestamp: chrono::Utc::now(),
            };

            info!(
                cleaned_up_count = cleaned_up_servers.len(),
                "Registry cleanup completed"
            );

            Ok(Json(response))
        }
        Err(e) => {
            error!("Registry cleanup failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
