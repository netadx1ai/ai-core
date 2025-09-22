//! Server Management Handlers
//!
//! This module provides HTTP handlers for managing MCP servers, including
//! registration, deregistration, updates, and status management.

use crate::{
    models::{
        ListServersRequest, ListServersResponse, RegisterServerRequest, RegisterServerResponse,
        ServerInfo, ServerStatus, UpdateServerRequest,
    },
    registry::{ServerFilter, ServerSort, SortField, SortOrder},
    server::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;
use validator::Validate;

/// Server status update request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateServerStatusRequest {
    /// New server status
    pub status: ServerStatus,
}

/// Server status update response
#[derive(Debug, Serialize)]
pub struct UpdateServerStatusResponse {
    /// Server ID
    pub server_id: Uuid,
    /// Old status
    pub old_status: String,
    /// New status
    pub new_status: String,
    /// Update timestamp
    pub updated_at: chrono::DateTime<Utc>,
    /// Success message
    pub message: String,
}

/// Register a new MCP server
///
/// Creates a new server registration in the registry with the provided
/// configuration and capabilities.
pub async fn register_server(
    State(state): State<AppState>,
    Json(request): Json<RegisterServerRequest>,
) -> Result<Json<RegisterServerResponse>, StatusCode> {
    // Validate request
    if let Err(e) = request.validate() {
        warn!("Invalid server registration request: {:?}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create server info
    let mut server = ServerInfo::new(
        request.name.clone(),
        request.version,
        request.server_type,
        request.config,
        request.capabilities,
    );

    // Set optional fields
    server.description = request.description;
    server.metadata = request.metadata.unwrap_or_default();
    server.tags = request.tags.unwrap_or_default();
    server.owner = request.owner;

    // Register server
    match state.registry().register(server.clone()).await {
        Ok(server_id) => {
            info!(
                server_id = %server_id,
                server_name = %request.name,
                "Server registered successfully"
            );

            let response = RegisterServerResponse {
                server_id,
                status: "registered".to_string(),
                message: "Server registered successfully".to_string(),
                endpoint: server.config.endpoint,
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!(
                server_name = %request.name,
                error = %e,
                "Failed to register server"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific server by ID
///
/// Retrieves detailed information about a registered server.
pub async fn get_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ServerInfo>, StatusCode> {
    match state.registry().get(&server_id).await {
        Some(server) => Ok(Json(server)),
        None => {
            warn!(server_id = %server_id, "Server not found");
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// List all registered servers with filtering and pagination
///
/// Returns a paginated list of servers matching the specified filters.
pub async fn list_servers(
    State(state): State<AppState>,
    Query(request): Query<ListServersRequest>,
) -> Result<Json<ListServersResponse>, StatusCode> {
    // Validate request
    if let Err(e) = request.validate() {
        warn!("Invalid list servers request: {:?}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Build filter
    let filter = ServerFilter {
        status: request.status,
        server_type: request.server_type,
        tags: request.tags,
        owner: request.owner,
        name_contains: None,
    };

    // Build sort criteria
    let sort = request.sort_by.as_ref().map(|sort_field| {
        let field = match sort_field.as_str() {
            "name" => SortField::Name,
            "type" => SortField::Type,
            "status" => SortField::Status,
            "created_at" => SortField::CreatedAt,
            "updated_at" => SortField::UpdatedAt,
            "last_health_check" => SortField::LastHealthCheck,
            _ => SortField::Name, // Default to name
        };

        let order = match request.sort_order.as_deref() {
            Some("desc") => SortOrder::Descending,
            _ => SortOrder::Ascending,
        };

        ServerSort { field, order }
    });

    // Calculate pagination
    let page = request.page.unwrap_or(1);
    let page_size = request.page_size.unwrap_or(20).min(100); // Max 100 items per page
    let offset = ((page - 1) * page_size) as usize;
    let limit = page_size as usize;

    // Get filtered servers
    let servers = state
        .registry()
        .list_filtered(&filter, sort.as_ref(), Some(limit), Some(offset))
        .await;

    // Get total count for pagination
    let total_servers = state.registry().list().await;
    let total = total_servers.len() as u64;
    let total_pages = (total + page_size as u64 - 1) / page_size as u64;

    let response = ListServersResponse {
        servers,
        total,
        page,
        page_size,
        total_pages: total_pages as u32,
    };

    info!(
        total_servers = total,
        page = page,
        page_size = page_size,
        "Listed servers"
    );

    Ok(Json(response))
}

/// Update an existing server
///
/// Updates the configuration, capabilities, or metadata of an existing server.
pub async fn update_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(request): Json<UpdateServerRequest>,
) -> Result<Json<ServerInfo>, StatusCode> {
    // Validate request
    if let Err(e) = request.validate() {
        warn!("Invalid server update request: {:?}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get existing server
    let mut server = match state.registry().get(&server_id).await {
        Some(server) => server,
        None => {
            warn!(server_id = %server_id, "Server not found for update");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Apply updates
    if let Some(name) = request.name {
        // Check if new name conflicts with existing server
        if let Some(existing) = state.registry().get_by_name(&name).await {
            if existing.id != server_id {
                warn!(
                    server_id = %server_id,
                    new_name = %name,
                    "Server name already exists"
                );
                return Err(StatusCode::CONFLICT);
            }
        }
        server.name = name;
    }

    if let Some(description) = request.description {
        server.description = Some(description);
    }

    if let Some(version) = request.version {
        server.version = version;
    }

    if let Some(config) = request.config {
        server.config = config;
    }

    if let Some(capabilities) = request.capabilities {
        server.capabilities = capabilities;
    }

    if let Some(metadata) = request.metadata {
        server.metadata = metadata;
    }

    if let Some(tags) = request.tags {
        server.tags = tags;
    }

    server.updated_at = Utc::now();

    // Update server in registry
    match state.registry().update(&server_id, server.clone()).await {
        Ok(_) => {
            info!(
                server_id = %server_id,
                server_name = %server.name,
                "Server updated successfully"
            );
            Ok(Json(server))
        }
        Err(e) => {
            error!(
                server_id = %server_id,
                error = %e,
                "Failed to update server"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update server status
///
/// Changes the operational status of a server (e.g., running, stopped, failed).
pub async fn update_server_status(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(request): Json<UpdateServerStatusRequest>,
) -> Result<Json<UpdateServerStatusResponse>, StatusCode> {
    // Validate request
    if let Err(e) = request.validate() {
        warn!("Invalid server status update request: {:?}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get current server
    let server = match state.registry().get(&server_id).await {
        Some(server) => server,
        None => {
            warn!(server_id = %server_id, "Server not found for status update");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let old_status = server.status;

    // Update server status
    match state
        .registry()
        .update_status(&server_id, request.status)
        .await
    {
        Ok(_) => {
            info!(
                server_id = %server_id,
                old_status = ?old_status,
                new_status = ?request.status,
                "Server status updated successfully"
            );

            let response = UpdateServerStatusResponse {
                server_id,
                old_status: old_status.to_string(),
                new_status: request.status.to_string(),
                updated_at: Utc::now(),
                message: "Server status updated successfully".to_string(),
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!(
                server_id = %server_id,
                error = %e,
                "Failed to update server status"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Deregister a server
///
/// Removes a server from the registry and stops all associated services.
pub async fn deregister_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Check if server exists
    let server = match state.registry().get(&server_id).await {
        Some(server) => server,
        None => {
            warn!(server_id = %server_id, "Server not found for deregistration");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Update status to stopping before deregistration
    if let Err(e) = state
        .registry()
        .update_status(&server_id, ServerStatus::Stopping)
        .await
    {
        warn!(
            server_id = %server_id,
            error = %e,
            "Failed to update server status to stopping"
        );
    }

    // Deregister server
    match state.registry().deregister(&server_id).await {
        Ok(_) => {
            info!(
                server_id = %server_id,
                server_name = %server.name,
                "Server deregistered successfully"
            );
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!(
                server_id = %server_id,
                error = %e,
                "Failed to deregister server"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get servers by status
///
/// Returns all servers with the specified status.
pub async fn get_servers_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
) -> Result<Json<Vec<ServerInfo>>, StatusCode> {
    let server_status = match status.as_str() {
        "starting" => ServerStatus::Starting,
        "running" => ServerStatus::Running,
        "unhealthy" => ServerStatus::Unhealthy,
        "stopping" => ServerStatus::Stopping,
        "stopped" => ServerStatus::Stopped,
        "failed" => ServerStatus::Failed,
        "unknown" => ServerStatus::Unknown,
        _ => {
            warn!(status = %status, "Invalid server status requested");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let servers = state.registry().get_by_status(server_status).await;

    info!(
        status = %status,
        count = servers.len(),
        "Retrieved servers by status"
    );

    Ok(Json(servers))
}

/// Get servers by type
///
/// Returns all servers of the specified type.
pub async fn get_servers_by_type(
    State(state): State<AppState>,
    Path(server_type): Path<String>,
) -> Result<Json<Vec<ServerInfo>>, StatusCode> {
    let servers = state.registry().get_by_type(&server_type).await;

    info!(
        server_type = %server_type,
        count = servers.len(),
        "Retrieved servers by type"
    );

    Ok(Json(servers))
}

/// Get healthy servers
///
/// Returns all servers that are currently healthy and available.
pub async fn get_healthy_servers(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServerInfo>>, StatusCode> {
    let servers = state.registry().get_healthy_servers().await;

    info!(count = servers.len(), "Retrieved healthy servers");

    Ok(Json(servers))
}

/// Server restart request
#[derive(Debug, Deserialize)]
pub struct RestartServerRequest {
    /// Force restart even if server is healthy
    pub force: Option<bool>,
    /// Restart timeout in seconds
    pub timeout_seconds: Option<u64>,
}

/// Restart a server
///
/// Gracefully restarts a server by stopping and then starting it again.
pub async fn restart_server(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(request): Json<RestartServerRequest>,
) -> Result<Json<UpdateServerStatusResponse>, StatusCode> {
    // Check if server exists
    let server = match state.registry().get(&server_id).await {
        Some(server) => server,
        None => {
            warn!(server_id = %server_id, "Server not found for restart");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let force = request.force.unwrap_or(false);

    // Check if restart is necessary
    if !force && server.status == ServerStatus::Running {
        // Check if server is healthy
        if let Some(health_check) = state.health_monitor().get_server_health(&server_id).await {
            if health_check.status == crate::models::HealthStatus::Healthy {
                warn!(
                    server_id = %server_id,
                    "Server is healthy, restart not necessary (use force=true to override)"
                );
                return Err(StatusCode::CONFLICT);
            }
        }
    }

    let old_status = server.status;

    // Stop the server first
    if let Err(e) = state
        .registry()
        .update_status(&server_id, ServerStatus::Stopping)
        .await
    {
        error!(
            server_id = %server_id,
            error = %e,
            "Failed to set server status to stopping"
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Wait a moment for graceful shutdown
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Start the server again
    match state
        .registry()
        .update_status(&server_id, ServerStatus::Starting)
        .await
    {
        Ok(_) => {
            info!(
                server_id = %server_id,
                old_status = ?old_status,
                "Server restart initiated"
            );

            let response = UpdateServerStatusResponse {
                server_id,
                old_status: old_status.to_string(),
                new_status: ServerStatus::Starting.to_string(),
                updated_at: Utc::now(),
                message: "Server restart initiated".to_string(),
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!(
                server_id = %server_id,
                error = %e,
                "Failed to restart server"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServerCapabilities, ServerConfig};
    use std::collections::HashMap;

    fn create_test_register_request() -> RegisterServerRequest {
        RegisterServerRequest {
            name: "test-server".to_string(),
            description: Some("Test server".to_string()),
            version: "1.0.0".to_string(),
            server_type: "test".to_string(),
            config: ServerConfig {
                endpoint: "http://localhost:8080".to_string(),
                port: 8080,
                host: "localhost".to_string(),
                timeout_seconds: 30,
                max_connections: 100,
                settings: HashMap::new(),
                environment: HashMap::new(),
                auth: None,
                ssl: None,
            },
            capabilities: ServerCapabilities {
                protocol_version: "2024-11-05".to_string(),
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
                features: Vec::new(),
                max_request_size: None,
                max_response_size: None,
                content_types: Vec::new(),
            },
            metadata: None,
            tags: Some(vec!["test".to_string()]),
            owner: Some("test-user".to_string()),
        }
    }

    #[test]
    fn test_register_request_validation() {
        let request = create_test_register_request();
        assert!(request.validate().is_ok());

        // Test with invalid name (empty)
        let mut invalid_request = request.clone();
        invalid_request.name = "".to_string();
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_status_update_request() {
        let request = UpdateServerStatusRequest {
            status: ServerStatus::Running,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_list_servers_request_validation() {
        let request = ListServersRequest {
            status: None,
            server_type: None,
            tags: None,
            owner: None,
            page: Some(1),
            page_size: Some(20),
            sort_by: None,
            sort_order: None,
        };
        assert!(request.validate().is_ok());

        // Test with invalid page
        let mut invalid_request = request.clone();
        invalid_request.page = Some(0);
        assert!(invalid_request.validate().is_err());
    }
}
