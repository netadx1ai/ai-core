//! Schema translation handlers for the Federation Service
//!
//! This module provides HTTP handlers for schema translation operations,
//! including translation requests, translation management, and compatibility
//! layer operations within the federation service.

use crate::handlers::{
    error_response, not_found_response, success_response, ApiResponse, IdPath, ListResponse,
    PaginationParams,
};
use crate::models::{SchemaTranslation, SchemaTranslationRequest, SchemaTranslationResponse};
use crate::server::ServerState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    response::Result as AxumResult,
};
// Removed unused imports Deserialize, Serialize, and uuid::Uuid

/// Translate schema data
pub async fn translate_schema(
    State(state): State<ServerState>,
    Json(request): Json<SchemaTranslationRequest>,
) -> AxumResult<Json<ApiResponse<SchemaTranslationResponse>>> {
    match state.schema_translator.translate_schema(request).await {
        Ok(response) => success_response(response),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            timestamp: chrono::Utc::now(),
        })),
    }
}

/// List available schema translations
pub async fn list_translations(
    State(state): State<ServerState>,
    Query(pagination): Query<PaginationParams>,
) -> AxumResult<Json<ApiResponse<ListResponse<SchemaTranslation>>>> {
    match state.schema_translator.list_translations().await {
        Ok(translations) => {
            let total = translations.len() as u64;
            let start = pagination.offset as usize;
            let end = std::cmp::min(start + pagination.limit as usize, translations.len());
            let items = translations[start..end].to_vec();

            let response = ListResponse::new(items, total, pagination.offset, pagination.limit);
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

/// Get schema translation by ID
pub async fn get_translation(
    State(state): State<ServerState>,
    Path(id_path): Path<IdPath>,
) -> Result<Json<ApiResponse<SchemaTranslation>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.schema_translator.get_translation(&id_path.id).await {
        Ok(Some(translation)) => Ok(Json(ApiResponse::success(translation))),
        Ok(None) => Err(not_found_response("Schema Translation", id_path.id)),
        Err(e) => Err(error_response(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_handlers() {
        // This would test the schema handlers with proper mocking
    }
}
