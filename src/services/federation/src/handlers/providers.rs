//! Provider management handlers for the Federation Service
//!
//! This module provides HTTP handlers for provider registration, management,
//! selection, and lifecycle operations within the federation service.

use crate::handlers::{
    error_response, not_found_response, success_response, ApiResponse, IdPath, ListResponse,
    PaginationParams,
};
use crate::models::{Provider, ProviderSelectionRequest, ProviderSelectionResponse};
use crate::server::ServerState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    response::Result as AxumResult,
};
use serde::Deserialize;

/// Register a new provider
pub async fn register_provider(
    State(state): State<ServerState>,
    Json(provider): Json<Provider>,
) -> AxumResult<Json<ApiResponse<Provider>>> {
    match state.provider_manager.register_provider(provider).await {
        Ok(provider) => success_response(provider),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// List providers with filtering and pagination
pub async fn list_providers(
    State(state): State<ServerState>,
    Query(pagination): Query<PaginationParams>,
) -> AxumResult<Json<ApiResponse<ListResponse<Provider>>>> {
    // This would implement provider listing with filters
    let providers = vec![]; // Stub implementation
    let response = ListResponse::new(providers, 0, pagination.offset, pagination.limit);
    success_response(response)
}

/// Get provider by ID
pub async fn get_provider(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
) -> Result<Json<ApiResponse<Provider>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.provider_manager.get_provider(&id_path.id).await {
        Ok(Some(provider)) => Ok(Json(ApiResponse::success(provider))),
        Ok(None) => Err(not_found_response("Provider", id_path.id)),
        Err(e) => Err(error_response(e.to_string())),
    }
}

/// Update provider configuration
pub async fn update_provider(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
    Json(update_request): Json<ProviderUpdateRequestPayload>,
) -> Result<Json<ApiResponse<Provider>>, (StatusCode, Json<ApiResponse<()>>)> {
    let updates = crate::provider::ProviderUpdateRequest {
        name: update_request.name,
        config: update_request.config,
        cost_info: update_request.cost_info,
        status: update_request.status,
        capabilities: update_request.capabilities,
        health_endpoint: update_request.health_endpoint,
    };

    match state
        .provider_manager
        .update_provider(&id_path.id, updates)
        .await
    {
        Ok(provider) => Ok(Json(ApiResponse::success(provider))),
        Err(crate::models::FederationError::ProviderNotFound { .. }) => {
            Err(not_found_response("Provider", id_path.id))
        }
        Err(e) => Err(error_response(e.to_string())),
    }
}

/// Delete a provider
pub async fn delete_provider(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.provider_manager.delete_provider(&id_path.id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(crate::models::FederationError::ProviderNotFound { .. }) => {
            Err(not_found_response("Provider", id_path.id))
        }
        Err(e) => Err(error_response(e.to_string())),
    }
}

/// Select optimal provider based on criteria
pub async fn select_provider(
    State(state): State<ServerState>,
    Json(request): Json<ProviderSelectionRequest>,
) -> AxumResult<Json<ApiResponse<ProviderSelectionResponse>>> {
    match state.provider_manager.select_provider(request).await {
        Ok(response) => success_response(response),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// Provider update request payload
#[derive(Debug, Deserialize)]
pub struct ProviderUpdateRequestPayload {
    pub name: Option<String>,
    pub config: Option<crate::models::ProviderConfig>,
    pub cost_info: Option<crate::models::CostInfo>,
    pub status: Option<crate::models::ProviderStatus>,
    pub capabilities: Option<Vec<String>>,
    pub health_endpoint: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_handlers() {
        // This would test the provider handlers with proper mocking
    }
}
