//! Client management handlers for the Federation Service
//!
//! This module provides HTTP handlers for client registration, management,
//! authentication, and lifecycle operations within the federation service.

use crate::client::ClientUsageStats;
use crate::handlers::{success_response, ApiResponse, IdPath, ListResponse, PaginationParams};
use crate::models::{Client, ClientRegistrationRequest, ClientRegistrationResponse};
use crate::server::ServerState;
use axum::{
    extract::{Path, Query, State},
    response::Json,
    response::Result as AxumResult,
};
use serde::Deserialize;

/// Register a new client
pub async fn register_client(
    State(state): State<ServerState>,
    Json(request): Json<ClientRegistrationRequest>,
) -> AxumResult<Json<ApiResponse<ClientRegistrationResponse>>> {
    match state.client_manager.register_client(request).await {
        Ok(response) => success_response(response),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// List clients with filtering and pagination
pub async fn list_clients(
    State(state): State<ServerState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<ClientFilterQuery>,
) -> AxumResult<Json<ApiResponse<ListResponse<Client>>>> {
    let client_filter = crate::client::ClientFilter {
        status: filter.status.and_then(|s| s.parse().ok()),
        tier: filter.tier.and_then(|t| t.parse().ok()),
        name_contains: filter.name,
        created_after: filter.created_after,
        created_before: filter.created_before,
        offset: pagination.offset,
        limit: pagination.limit,
    };

    match state.client_manager.list_clients(client_filter).await {
        Ok(client_list) => {
            let response = ListResponse::new(
                client_list.clients,
                client_list.total,
                client_list.offset,
                client_list.limit,
            );
            success_response(response)
        }
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// Get client by ID
pub async fn get_client(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
) -> AxumResult<Json<ApiResponse<Client>>> {
    match state.client_manager.get_client(&id_path.id).await {
        Ok(Some(client)) => success_response(client),
        Ok(None) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Client not found: {}", id_path.id)),
            timestamp: chrono::Utc::now(),
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// Update client configuration
pub async fn update_client(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
    Json(update_request): Json<ClientUpdateRequestPayload>,
) -> AxumResult<Json<ApiResponse<Client>>> {
    let updates = crate::client::ClientUpdateRequest {
        name: update_request.name,
        description: update_request.description,
        tier: update_request.tier,
        config: update_request.config,
        status: update_request.status,
        limits: update_request.limits,
        metadata: update_request.metadata,
    };

    match state
        .client_manager
        .update_client(&id_path.id, updates)
        .await
    {
        Ok(client) => success_response(client),
        Err(crate::models::FederationError::ClientNotFound { .. }) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Client not found: {}", id_path.id)),
            timestamp: chrono::Utc::now(),
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// Delete a client
pub async fn delete_client(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
) -> AxumResult<Json<ApiResponse<()>>> {
    match state.client_manager.delete_client(&id_path.id).await {
        Ok(()) => success_response(()),
        Err(crate::models::FederationError::ClientNotFound { .. }) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Client not found: {}", id_path.id)),
            timestamp: chrono::Utc::now(),
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// Get client usage statistics
pub async fn get_client_usage(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
) -> AxumResult<Json<ApiResponse<ClientUsageStats>>> {
    match state.client_manager.get_client_usage(&id_path.id).await {
        Ok(usage_stats) => success_response(usage_stats),
        Err(crate::models::FederationError::ClientNotFound { .. }) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Client not found: {}", id_path.id)),
            timestamp: chrono::Utc::now(),
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// Client filter query parameters
#[derive(Debug, Deserialize)]
pub struct ClientFilterQuery {
    pub status: Option<String>,
    pub tier: Option<String>,
    pub name: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

/// Client update request payload
#[derive(Debug, Deserialize)]
pub struct ClientUpdateRequestPayload {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub tier: Option<crate::models::ClientTier>,
    pub config: Option<crate::models::ClientConfig>,
    pub status: Option<crate::models::ClientStatus>,
    pub limits: Option<crate::models::ResourceLimits>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_handlers() {
        // This would test the client handlers with proper mocking
    }
}
