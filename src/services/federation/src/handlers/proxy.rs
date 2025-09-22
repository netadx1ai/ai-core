//! MCP Proxy handlers for the Federation Service
//!
//! This module provides HTTP handlers for MCP proxy operations,
//! including request proxying, connection management, and protocol
//! translation within the federation service.

use crate::handlers::{error_response, ApiResponse};
use crate::server::ServerState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

/// Proxy MCP request to target server
pub async fn proxy_mcp_request(
    State(state): State<ServerState>,
    Path(params): Path<ProxyRequestParams>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract headers
    let mut header_map = HashMap::new();
    for (key, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(key.to_string(), value_str.to_string());
        }
    }

    // Parse body if present
    let body_json = if body.is_empty() {
        None
    } else {
        serde_json::from_str(&body).ok()
    };

    match state
        .mcp_proxy
        .proxy_request(
            &params.server_id,
            &params.path,
            "POST", // This would be extracted from the actual HTTP method
            header_map,
            body_json,
        )
        .await
    {
        Ok(response) => {
            let result = serde_json::json!({
                "status_code": response.status_code,
                "headers": response.headers,
                "body": response.body
            });
            Ok(Json(ApiResponse::success(result)))
        }
        Err(e) => Err(error_response(e.to_string())),
    }
}

/// Path parameters for proxy requests
#[derive(Debug, Deserialize)]
pub struct ProxyRequestParams {
    /// Target MCP server ID
    pub server_id: Uuid,
    /// Request path to proxy
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_handlers() {
        // This would test the proxy handlers with proper mocking
    }
}
