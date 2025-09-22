//! Server Registry Module
//!
//! This module provides the core registry functionality for managing MCP server instances.
//! It handles server registration, deregistration, lookup, and lifecycle management.

use crate::{
    models::{ServerInfo, ServerStatus},
    McpError, Result,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Server registry for managing MCP server instances
#[derive(Debug)]
pub struct ServerRegistry {
    /// Active servers indexed by ID
    servers: DashMap<Uuid, ServerInfo>,

    /// Server indices for efficient lookups
    indices: Arc<RwLock<RegistryIndices>>,

    /// Registry configuration
    config: RegistryConfig,
}

/// Registry indices for efficient server lookups
#[derive(Debug, Default)]
struct RegistryIndices {
    /// Servers by name
    by_name: HashMap<String, Uuid>,

    /// Servers by type
    by_type: HashMap<String, Vec<Uuid>>,

    /// Servers by status
    by_status: HashMap<ServerStatus, Vec<Uuid>>,

    /// Servers by tags
    by_tags: HashMap<String, Vec<Uuid>>,

    /// Servers by owner
    by_owner: HashMap<String, Vec<Uuid>>,
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum number of servers
    pub max_servers: u32,

    /// Enable automatic cleanup of failed servers
    pub auto_cleanup: bool,

    /// Cleanup interval in seconds
    pub cleanup_interval_seconds: u64,

    /// Maximum time before considering a server stale (in seconds)
    pub stale_timeout_seconds: u64,
}

/// Server filter criteria
#[derive(Debug, Clone, Default)]
pub struct ServerFilter {
    /// Filter by server status
    pub status: Option<ServerStatus>,

    /// Filter by server type
    pub server_type: Option<String>,

    /// Filter by tags (any of these tags)
    pub tags: Option<Vec<String>>,

    /// Filter by owner
    pub owner: Option<String>,

    /// Filter by name (partial match)
    pub name_contains: Option<String>,
}

/// Server sort criteria
#[derive(Debug, Clone)]
pub struct ServerSort {
    /// Sort field
    pub field: SortField,

    /// Sort order
    pub order: SortOrder,
}

/// Sort fields
#[derive(Debug, Clone)]
pub enum SortField {
    Name,
    Type,
    Status,
    CreatedAt,
    UpdatedAt,
    LastHealthCheck,
}

/// Sort orders
#[derive(Debug, Clone)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl ServerRegistry {
    /// Create a new server registry
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            servers: DashMap::new(),
            indices: Arc::new(RwLock::new(RegistryIndices::default())),
            config,
        }
    }

    /// Register a new server
    pub async fn register(&self, mut server: ServerInfo) -> Result<Uuid> {
        // Check if we've reached the maximum number of servers
        if self.servers.len() >= self.config.max_servers as usize {
            return Err(McpError::ServerManagement(
                "Maximum number of servers reached".to_string(),
            ));
        }

        // Check if a server with the same name already exists
        if self.get_by_name(&server.name).await.is_some() {
            return Err(McpError::ServerManagement(format!(
                "Server with name '{}' already exists",
                server.name
            )));
        }

        let server_id = server.id;
        server.status = ServerStatus::Starting;

        // Insert server into the main collection
        self.servers.insert(server_id, server.clone());

        // Update indices
        self.update_indices_on_insert(&server).await;

        info!(
            server_id = %server_id,
            server_name = %server.name,
            server_type = %server.server_type,
            "Server registered successfully"
        );

        Ok(server_id)
    }

    /// Deregister a server
    pub async fn deregister(&self, server_id: &Uuid) -> Result<()> {
        if let Some((_, server)) = self.servers.remove(server_id) {
            // Update indices
            self.update_indices_on_remove(&server).await;

            info!(
                server_id = %server_id,
                server_name = %server.name,
                "Server deregistered successfully"
            );

            Ok(())
        } else {
            Err(McpError::ServerManagement(format!(
                "Server with ID {} not found",
                server_id
            )))
        }
    }

    /// Get a server by ID
    pub async fn get(&self, server_id: &Uuid) -> Option<ServerInfo> {
        self.servers.get(server_id).map(|entry| entry.clone())
    }

    /// Get a server by name
    pub async fn get_by_name(&self, name: &str) -> Option<ServerInfo> {
        let indices = self.indices.read().await;
        if let Some(server_id) = indices.by_name.get(name) {
            self.servers.get(server_id).map(|entry| entry.clone())
        } else {
            None
        }
    }

    /// Update a server's information
    pub async fn update(&self, server_id: &Uuid, mut updates: ServerInfo) -> Result<()> {
        if let Some(mut entry) = self.servers.get_mut(server_id) {
            let old_server = entry.clone();

            // Update the server
            updates.id = *server_id;
            updates.updated_at = chrono::Utc::now();
            *entry = updates.clone();

            // Update indices if necessary
            if old_server.name != updates.name
                || old_server.server_type != updates.server_type
                || old_server.status != updates.status
                || old_server.tags != updates.tags
                || old_server.owner != updates.owner
            {
                self.update_indices_on_update(&old_server, &updates).await;
            }

            debug!(
                server_id = %server_id,
                server_name = %updates.name,
                "Server updated successfully"
            );

            Ok(())
        } else {
            Err(McpError::ServerManagement(format!(
                "Server with ID {} not found",
                server_id
            )))
        }
    }

    /// Update a server's status
    pub async fn update_status(&self, server_id: &Uuid, status: ServerStatus) -> Result<()> {
        if let Some(mut entry) = self.servers.get_mut(server_id) {
            let old_status = entry.status;
            entry.update_status(status);

            // Update status index if changed
            if old_status != status {
                self.update_status_index(server_id, old_status, status)
                    .await;
            }

            debug!(
                server_id = %server_id,
                old_status = ?old_status,
                new_status = ?status,
                "Server status updated"
            );

            Ok(())
        } else {
            Err(McpError::ServerManagement(format!(
                "Server with ID {} not found",
                server_id
            )))
        }
    }

    /// Update a server's health check timestamp
    pub async fn update_health_check(&self, server_id: &Uuid) -> Result<()> {
        if let Some(mut entry) = self.servers.get_mut(server_id) {
            entry.update_health_check();

            debug!(
                server_id = %server_id,
                "Server health check timestamp updated"
            );

            Ok(())
        } else {
            Err(McpError::ServerManagement(format!(
                "Server with ID {} not found",
                server_id
            )))
        }
    }

    /// List all servers
    pub async fn list(&self) -> Vec<ServerInfo> {
        self.servers.iter().map(|entry| entry.clone()).collect()
    }

    /// List servers with filtering and sorting
    pub async fn list_filtered(
        &self,
        filter: &ServerFilter,
        sort: Option<&ServerSort>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<ServerInfo> {
        let mut servers = self.apply_filter(filter).await;

        // Apply sorting
        if let Some(sort_criteria) = sort {
            self.sort_servers(&mut servers, sort_criteria);
        }

        // Apply pagination
        let start = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            (start + limit).min(servers.len())
        } else {
            servers.len()
        };

        servers.into_iter().skip(start).take(end - start).collect()
    }

    /// Get servers by status
    pub async fn get_by_status(&self, status: ServerStatus) -> Vec<ServerInfo> {
        let indices = self.indices.read().await;
        if let Some(server_ids) = indices.by_status.get(&status) {
            server_ids
                .iter()
                .filter_map(|id| self.servers.get(id).map(|entry| entry.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get servers by type
    pub async fn get_by_type(&self, server_type: &str) -> Vec<ServerInfo> {
        let indices = self.indices.read().await;
        if let Some(server_ids) = indices.by_type.get(server_type) {
            server_ids
                .iter()
                .filter_map(|id| self.servers.get(id).map(|entry| entry.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get servers by tag
    pub async fn get_by_tag(&self, tag: &str) -> Vec<ServerInfo> {
        let indices = self.indices.read().await;
        if let Some(server_ids) = indices.by_tags.get(tag) {
            server_ids
                .iter()
                .filter_map(|id| self.servers.get(id).map(|entry| entry.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get healthy servers
    pub async fn get_healthy_servers(&self) -> Vec<ServerInfo> {
        self.get_by_status(ServerStatus::Running).await
    }

    /// Get available servers (running or unhealthy but responsive)
    pub async fn get_available_servers(&self) -> Vec<ServerInfo> {
        let mut servers = self.get_by_status(ServerStatus::Running).await;
        servers.extend(self.get_by_status(ServerStatus::Unhealthy).await);
        servers
    }

    /// Get server count
    pub async fn count(&self) -> usize {
        self.servers.len()
    }

    /// Get server count by status
    pub async fn count_by_status(&self, status: ServerStatus) -> usize {
        let indices = self.indices.read().await;
        indices
            .by_status
            .get(&status)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// Get server statistics
    pub async fn get_statistics(&self) -> RegistryStatistics {
        let indices = self.indices.read().await;

        let mut status_counts = HashMap::new();
        for (status, ids) in &indices.by_status {
            status_counts.insert(*status, ids.len());
        }

        let mut type_counts = HashMap::new();
        for (server_type, ids) in &indices.by_type {
            type_counts.insert(server_type.clone(), ids.len());
        }

        RegistryStatistics {
            total_servers: self.servers.len(),
            status_counts,
            type_counts,
            tag_counts: indices.by_tags.len(),
            owner_counts: indices.by_owner.len(),
        }
    }

    /// Check if a server exists
    pub async fn exists(&self, server_id: &Uuid) -> bool {
        self.servers.contains_key(server_id)
    }

    /// Check if a server name is available
    pub async fn is_name_available(&self, name: &str) -> bool {
        let indices = self.indices.read().await;
        !indices.by_name.contains_key(name)
    }

    /// Cleanup stale servers
    pub async fn cleanup_stale_servers(&self) -> Result<Vec<Uuid>> {
        if !self.config.auto_cleanup {
            return Ok(Vec::new());
        }

        let stale_threshold = chrono::Utc::now()
            - chrono::Duration::seconds(self.config.stale_timeout_seconds as i64);

        let mut stale_servers = Vec::new();

        for entry in self.servers.iter() {
            let server = entry.value();

            // Consider servers stale if they haven't had a health check in a while
            // and are in a failed or unknown state
            if matches!(server.status, ServerStatus::Failed | ServerStatus::Unknown) {
                if let Some(last_check) = server.last_health_check {
                    if last_check < stale_threshold {
                        stale_servers.push(server.id);
                    }
                } else if server.updated_at < stale_threshold {
                    stale_servers.push(server.id);
                }
            }
        }

        // Remove stale servers
        for server_id in &stale_servers {
            if let Err(e) = self.deregister(server_id).await {
                warn!(
                    server_id = %server_id,
                    error = %e,
                    "Failed to cleanup stale server"
                );
            } else {
                info!(
                    server_id = %server_id,
                    "Cleaned up stale server"
                );
            }
        }

        Ok(stale_servers)
    }

    // Private helper methods

    async fn update_indices_on_insert(&self, server: &ServerInfo) {
        let mut indices = self.indices.write().await;

        // Update name index
        indices.by_name.insert(server.name.clone(), server.id);

        // Update type index
        indices
            .by_type
            .entry(server.server_type.clone())
            .or_insert_with(Vec::new)
            .push(server.id);

        // Update status index
        indices
            .by_status
            .entry(server.status)
            .or_insert_with(Vec::new)
            .push(server.id);

        // Update tags index
        for tag in &server.tags {
            indices
                .by_tags
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(server.id);
        }

        // Update owner index
        if let Some(owner) = &server.owner {
            indices
                .by_owner
                .entry(owner.clone())
                .or_insert_with(Vec::new)
                .push(server.id);
        }
    }

    async fn update_indices_on_remove(&self, server: &ServerInfo) {
        let mut indices = self.indices.write().await;

        // Remove from name index
        indices.by_name.remove(&server.name);

        // Remove from type index
        if let Some(type_servers) = indices.by_type.get_mut(&server.server_type) {
            type_servers.retain(|id| *id != server.id);
            if type_servers.is_empty() {
                indices.by_type.remove(&server.server_type);
            }
        }

        // Remove from status index
        if let Some(status_servers) = indices.by_status.get_mut(&server.status) {
            status_servers.retain(|id| *id != server.id);
            if status_servers.is_empty() {
                indices.by_status.remove(&server.status);
            }
        }

        // Remove from tags index
        for tag in &server.tags {
            if let Some(tag_servers) = indices.by_tags.get_mut(tag) {
                tag_servers.retain(|id| *id != server.id);
                if tag_servers.is_empty() {
                    indices.by_tags.remove(tag);
                }
            }
        }

        // Remove from owner index
        if let Some(owner) = &server.owner {
            if let Some(owner_servers) = indices.by_owner.get_mut(owner) {
                owner_servers.retain(|id| *id != server.id);
                if owner_servers.is_empty() {
                    indices.by_owner.remove(owner);
                }
            }
        }
    }

    async fn update_indices_on_update(&self, old_server: &ServerInfo, new_server: &ServerInfo) {
        let mut indices = self.indices.write().await;

        // Update name index if changed
        if old_server.name != new_server.name {
            indices.by_name.remove(&old_server.name);
            indices
                .by_name
                .insert(new_server.name.clone(), new_server.id);
        }

        // Update type index if changed
        if old_server.server_type != new_server.server_type {
            // Remove from old type
            if let Some(type_servers) = indices.by_type.get_mut(&old_server.server_type) {
                type_servers.retain(|id| *id != new_server.id);
                if type_servers.is_empty() {
                    indices.by_type.remove(&old_server.server_type);
                }
            }
            // Add to new type
            indices
                .by_type
                .entry(new_server.server_type.clone())
                .or_insert_with(Vec::new)
                .push(new_server.id);
        }

        // Update status index if changed
        if old_server.status != new_server.status {
            self.update_status_index_locked(
                &mut indices,
                &new_server.id,
                old_server.status,
                new_server.status,
            );
        }

        // Update tags index if changed
        if old_server.tags != new_server.tags {
            // Remove from old tags
            for tag in &old_server.tags {
                if let Some(tag_servers) = indices.by_tags.get_mut(tag) {
                    tag_servers.retain(|id| *id != new_server.id);
                    if tag_servers.is_empty() {
                        indices.by_tags.remove(tag);
                    }
                }
            }
            // Add to new tags
            for tag in &new_server.tags {
                indices
                    .by_tags
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(new_server.id);
            }
        }

        // Update owner index if changed
        if old_server.owner != new_server.owner {
            // Remove from old owner
            if let Some(old_owner) = &old_server.owner {
                if let Some(owner_servers) = indices.by_owner.get_mut(old_owner) {
                    owner_servers.retain(|id| *id != new_server.id);
                    if owner_servers.is_empty() {
                        indices.by_owner.remove(old_owner);
                    }
                }
            }
            // Add to new owner
            if let Some(new_owner) = &new_server.owner {
                indices
                    .by_owner
                    .entry(new_owner.clone())
                    .or_insert_with(Vec::new)
                    .push(new_server.id);
            }
        }
    }

    async fn update_status_index(
        &self,
        server_id: &Uuid,
        old_status: ServerStatus,
        new_status: ServerStatus,
    ) {
        let mut indices = self.indices.write().await;
        self.update_status_index_locked(&mut indices, server_id, old_status, new_status);
    }

    fn update_status_index_locked(
        &self,
        indices: &mut RegistryIndices,
        server_id: &Uuid,
        old_status: ServerStatus,
        new_status: ServerStatus,
    ) {
        // Remove from old status
        if let Some(status_servers) = indices.by_status.get_mut(&old_status) {
            status_servers.retain(|id| *id != *server_id);
            if status_servers.is_empty() {
                indices.by_status.remove(&old_status);
            }
        }

        // Add to new status
        indices
            .by_status
            .entry(new_status)
            .or_insert_with(Vec::new)
            .push(*server_id);
    }

    async fn apply_filter(&self, filter: &ServerFilter) -> Vec<ServerInfo> {
        let indices = self.indices.read().await;
        let mut candidate_ids: Option<Vec<Uuid>> = None;

        // Apply status filter
        if let Some(status) = filter.status {
            if let Some(status_ids) = indices.by_status.get(&status) {
                candidate_ids = Some(status_ids.clone());
            } else {
                return Vec::new();
            }
        }

        // Apply type filter
        if let Some(server_type) = &filter.server_type {
            if let Some(type_ids) = indices.by_type.get(server_type) {
                if let Some(ref mut ids) = candidate_ids {
                    ids.retain(|id| type_ids.contains(id));
                } else {
                    candidate_ids = Some(type_ids.clone());
                }
            } else {
                return Vec::new();
            }
        }

        // Apply tags filter (any of the specified tags)
        if let Some(tags) = &filter.tags {
            let mut tag_ids = Vec::new();
            for tag in tags {
                if let Some(ids) = indices.by_tags.get(tag) {
                    tag_ids.extend_from_slice(ids);
                }
            }
            tag_ids.sort_unstable();
            tag_ids.dedup();

            if let Some(ref mut ids) = candidate_ids {
                ids.retain(|id| tag_ids.contains(id));
            } else {
                candidate_ids = Some(tag_ids);
            }
        }

        // Apply owner filter
        if let Some(owner) = &filter.owner {
            if let Some(owner_ids) = indices.by_owner.get(owner) {
                if let Some(ref mut ids) = candidate_ids {
                    ids.retain(|id| owner_ids.contains(id));
                } else {
                    candidate_ids = Some(owner_ids.clone());
                }
            } else {
                return Vec::new();
            }
        }

        drop(indices);

        // Get servers for candidate IDs
        let server_ids = candidate_ids
            .unwrap_or_else(|| self.servers.iter().map(|entry| *entry.key()).collect());

        let mut servers: Vec<ServerInfo> = server_ids
            .into_iter()
            .filter_map(|id| self.servers.get(&id).map(|entry| entry.clone()))
            .collect();

        // Apply name filter (needs to be done after fetching servers)
        if let Some(name_contains) = &filter.name_contains {
            let name_lower = name_contains.to_lowercase();
            servers.retain(|server| server.name.to_lowercase().contains(&name_lower));
        }

        servers
    }

    fn sort_servers(&self, servers: &mut Vec<ServerInfo>, sort: &ServerSort) {
        servers.sort_by(|a, b| {
            let comparison = match sort.field {
                SortField::Name => a.name.cmp(&b.name),
                SortField::Type => a.server_type.cmp(&b.server_type),
                SortField::Status => a.status.to_string().cmp(&b.status.to_string()),
                SortField::CreatedAt => a.created_at.cmp(&b.created_at),
                SortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                SortField::LastHealthCheck => a.last_health_check.cmp(&b.last_health_check),
            };

            match sort.order {
                SortOrder::Ascending => comparison,
                SortOrder::Descending => comparison.reverse(),
            }
        });
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_servers: 100,
            auto_cleanup: true,
            cleanup_interval_seconds: 300, // 5 minutes
            stale_timeout_seconds: 3600,   // 1 hour
        }
    }
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStatus::Starting => write!(f, "starting"),
            ServerStatus::Running => write!(f, "running"),
            ServerStatus::Unhealthy => write!(f, "unhealthy"),
            ServerStatus::Stopping => write!(f, "stopping"),
            ServerStatus::Stopped => write!(f, "stopped"),
            ServerStatus::Failed => write!(f, "failed"),
            ServerStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatistics {
    /// Total number of servers
    pub total_servers: usize,

    /// Server count by status
    pub status_counts: HashMap<ServerStatus, usize>,

    /// Server count by type
    pub type_counts: HashMap<String, usize>,

    /// Number of unique tags
    pub tag_counts: usize,

    /// Number of unique owners
    pub owner_counts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServerCapabilities, ServerConfig};

    fn create_test_server(name: &str, server_type: &str) -> ServerInfo {
        ServerInfo::new(
            name.to_string(),
            "1.0.0".to_string(),
            server_type.to_string(),
            ServerConfig {
                endpoint: format!("http://localhost:8080/{}", name),
                port: 8080,
                host: "localhost".to_string(),
                timeout_seconds: 30,
                max_connections: 100,
                settings: HashMap::new(),
                environment: HashMap::new(),
                auth: None,
                ssl: None,
            },
            ServerCapabilities {
                protocol_version: "2024-11-05".to_string(),
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
                features: Vec::new(),
                max_request_size: None,
                max_response_size: None,
                content_types: Vec::new(),
            },
        )
    }

    #[tokio::test]
    async fn test_server_registration() {
        let registry = ServerRegistry::new(RegistryConfig::default());
        let server = create_test_server("test-server", "test");

        let server_id = registry.register(server.clone()).await.unwrap();
        assert_eq!(server_id, server.id);

        let retrieved = registry.get(&server_id).await.unwrap();
        assert_eq!(retrieved.name, server.name);
        assert_eq!(retrieved.server_type, server.server_type);
    }

    #[tokio::test]
    async fn test_duplicate_name_registration() {
        let registry = ServerRegistry::new(RegistryConfig::default());
        let server1 = create_test_server("duplicate", "test");
        let server2 = create_test_server("duplicate", "test");

        registry.register(server1).await.unwrap();
        let result = registry.register(server2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_server_deregistration() {
        let registry = ServerRegistry::new(RegistryConfig::default());
        let server = create_test_server("test-server", "test");

        let server_id = registry.register(server).await.unwrap();
        assert!(registry.exists(&server_id).await);

        registry.deregister(&server_id).await.unwrap();
        assert!(!registry.exists(&server_id).await);
    }

    #[tokio::test]
    async fn test_server_filtering() {
        let registry = ServerRegistry::new(RegistryConfig::default());

        let mut server1 = create_test_server("server1", "database");
        server1.tags = vec!["production".to_string()];

        let mut server2 = create_test_server("server2", "api");
        server2.tags = vec!["development".to_string()];

        registry.register(server1).await.unwrap();
        registry.register(server2).await.unwrap();

        // Filter by type
        let filter = ServerFilter {
            server_type: Some("database".to_string()),
            ..Default::default()
        };
        let results = registry.list_filtered(&filter, None, None, None).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "server1");

        // Filter by tags
        let filter = ServerFilter {
            tags: Some(vec!["production".to_string()]),
            ..Default::default()
        };
        let results = registry.list_filtered(&filter, None, None, None).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "server1");
    }

    #[tokio::test]
    async fn test_server_statistics() {
        let registry = ServerRegistry::new(RegistryConfig::default());

        let server1 = create_test_server("server1", "database");
        let server2 = create_test_server("server2", "api");

        registry.register(server1).await.unwrap();
        registry.register(server2).await.unwrap();

        let stats = registry.get_statistics().await;
        assert_eq!(stats.total_servers, 2);
        assert_eq!(stats.type_counts.get("database"), Some(&1));
        assert_eq!(stats.type_counts.get("api"), Some(&1));
    }
}
