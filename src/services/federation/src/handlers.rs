//! HTTP request handlers for the Federation Service
//!
//! This module provides all HTTP request handlers for the federation service,
//! organized by functional area. Each sub-module handles a specific aspect
//! of the federation service API.

use crate::server::ServerState;

// Re-export all handler modules
pub mod auth;
pub mod blog_api;
pub mod clients;
pub mod cost;
pub mod health;
pub mod metrics;
pub mod providers;
pub mod proxy;
pub mod schema;
pub mod status;
pub mod workflows;

// Common handler utilities and types
use axum::{http::StatusCode, response::Json, response::Result as AxumResult};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error_generic<U>(message: String) -> ApiResponse<U> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Common query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    50
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

/// Common query parameters for filtering
#[derive(Debug, Deserialize)]
pub struct FilterParams {
    pub status: Option<String>,
    pub name: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

/// Path parameter for entity ID
#[derive(Debug, Deserialize)]
pub struct IdPath {
    pub id: Uuid,
}

/// Response for list endpoints
#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

impl<T> ListResponse<T> {
    pub fn new(items: Vec<T>, total: u64, offset: u64, limit: u64) -> Self {
        Self {
            items,
            total,
            offset,
            limit,
        }
    }
}

/// Helper function to create a success response
pub fn success_response<T: Serialize>(data: T) -> AxumResult<Json<ApiResponse<T>>> {
    Ok(Json(ApiResponse::success(data)))
}

/// Helper function to create an error response
pub fn error_response(message: String) -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }),
    )
}

/// Helper function to create a not found response
pub fn not_found_response(resource: &str, id: Uuid) -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("{} not found: {}", resource, id)),
            timestamp: chrono::Utc::now(),
        }),
    )
}

/// Helper function to create a validation error response
pub fn validation_error_response(
    field: &str,
    message: &str,
) -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Validation error for {}: {}", field, message)),
            timestamp: chrono::Utc::now(),
        }),
    )
}

/// Extract client ID from authentication context
pub fn extract_client_id(
    state: &ServerState,
    auth_header: Option<&str>,
) -> Result<Uuid, crate::models::FederationError> {
    // This would implement actual client ID extraction from JWT or API key
    // For now, return a dummy UUID
    Ok(Uuid::new_v4())
}

/// Common handler for checking permissions
pub async fn check_permissions(
    state: &ServerState,
    client_id: Uuid,
    resource: &str,
    action: &str,
) -> Result<bool, crate::models::FederationError> {
    // This would implement actual permission checking
    // For now, always return true
    Ok(true)
}
