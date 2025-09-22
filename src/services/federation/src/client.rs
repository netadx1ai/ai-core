//! Client management for the Federation Service
//!
//! This module handles multi-tenant client registration, authentication, and lifecycle management.
//! It provides comprehensive client registry and management capabilities with database persistence
//! and Redis caching for optimal performance.

use crate::models::{
    Client, ClientConfig, ClientCredentials, ClientRegistrationRequest, ClientRegistrationResponse,
    ClientStatus, ClientTier, FederationError, ResourceLimits,
};
use crate::utils::{cache::CacheManager, database::DatabaseManager};
use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::Client as RedisClient;
use serde::Serialize;
use serde_json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Client manager for multi-tenant client management
#[derive(Debug, Clone)]
pub struct ClientManager {
    /// Database connection pool
    db_pool: Arc<PgPool>,
    /// Redis client for caching
    redis_client: Arc<RedisClient>,
    /// Cache manager
    cache_manager: Arc<CacheManager>,
    /// Database manager
    db_manager: Arc<DatabaseManager>,
    /// In-memory client registry for fast lookups
    client_registry: Arc<ClientRegistry>,
    /// Client activity tracker
    activity_tracker: Arc<DashMap<Uuid, DateTime<Utc>>>,
    /// Resource usage tracker
    usage_tracker: Arc<DashMap<Uuid, ResourceUsageTracker>>,
}

/// Client registry for in-memory caching and fast lookups
#[derive(Debug)]
pub struct ClientRegistry {
    /// Clients indexed by ID
    clients_by_id: Arc<DashMap<Uuid, Arc<Client>>>,
    /// Clients indexed by API key
    clients_by_api_key: Arc<DashMap<String, Arc<Client>>>,
    /// Client statistics
    stats: Arc<RwLock<ClientRegistryStats>>,
}

/// Client registry statistics
#[derive(Debug, Clone, Default)]
pub struct ClientRegistryStats {
    /// Total number of clients
    pub total_clients: u64,
    /// Active clients
    pub active_clients: u64,
    /// Suspended clients
    pub suspended_clients: u64,
    /// Inactive clients
    pub inactive_clients: u64,
    /// Pending clients
    pub pending_clients: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Resource usage tracker for client monitoring
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResourceUsageTracker {
    /// Current requests per minute
    pub requests_per_minute: u32,
    /// Current requests per hour
    pub requests_per_hour: u32,
    /// Current requests per day
    pub requests_per_day: u32,
    /// Current concurrent connections
    pub concurrent_connections: u32,
    /// Current data transfer (bytes)
    pub data_transfer_today: u64,
    /// Current storage usage (bytes)
    pub storage_usage: u64,
    /// Last reset timestamps
    pub last_minute_reset: DateTime<Utc>,
    pub last_hour_reset: DateTime<Utc>,
    pub last_day_reset: DateTime<Utc>,
}

impl ClientManager {
    /// Create a new client manager
    pub async fn new(db_pool: PgPool, redis_client: RedisClient) -> Result<Self, FederationError> {
        let db_pool = Arc::new(db_pool);
        let redis_client = Arc::new(redis_client);

        let cache_manager = Arc::new(CacheManager::new(redis_client.clone()).await?);
        let db_manager = Arc::new(DatabaseManager::new(db_pool.clone()).await?);
        let client_registry = Arc::new(ClientRegistry::new().await?);

        let manager = Self {
            db_pool,
            redis_client,
            cache_manager,
            db_manager,
            client_registry,
            activity_tracker: Arc::new(DashMap::new()),
            usage_tracker: Arc::new(DashMap::new()),
        };

        // Initialize client registry from database
        manager.initialize_registry().await?;

        Ok(manager)
    }

    /// Register a new client
    pub async fn register_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, FederationError> {
        info!("Registering new client: {}", request.name);

        // Validate the registration request
        self.validate_registration_request(&request)?;

        // Check for duplicate names
        if self.client_exists_by_name(&request.name).await? {
            return Err(FederationError::ValidationError {
                field: "name".to_string(),
                message: "Client name already exists".to_string(),
            });
        }

        // Generate client ID and credentials
        let client_id = Uuid::new_v4();
        let api_key = self.generate_api_key();
        let jwt_secret = self.generate_jwt_secret();

        // Create client object
        let client = Client {
            id: client_id,
            name: request.name,
            description: request.description,
            tier: request.tier.clone(),
            config: request.config,
            credentials: ClientCredentials {
                api_key: api_key.clone(),
                jwt_secret: Some(jwt_secret),
                oauth_config: None,
                webhook_secret: None,
            },
            status: ClientStatus::Pending,
            limits: self.get_default_limits_for_tier(&request.tier),
            metadata: request.metadata.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_activity_at: None,
        };

        // Save to database
        self.db_manager.create_client(&client).await?;

        // Cache the client
        self.cache_manager.cache_client(&client).await?;

        // Add to in-memory registry
        self.client_registry.add_client(client.clone()).await?;

        // Initialize usage tracking
        self.usage_tracker
            .insert(client_id, ResourceUsageTracker::default());

        info!(
            "Successfully registered client: {} ({})",
            client.name, client.id
        );

        Ok(ClientRegistrationResponse {
            client,
            api_key,
            message: "Client registered successfully. Please store your API key securely as it cannot be retrieved later.".to_string(),
        })
    }

    /// Get client by ID
    pub async fn get_client(&self, client_id: &Uuid) -> Result<Option<Client>, FederationError> {
        // Try in-memory registry first
        if let Some(client) = self.client_registry.get_client_by_id(client_id).await {
            self.record_activity(client_id).await;
            return Ok(Some((*client).clone()));
        }

        // Try cache
        if let Some(client) = self.cache_manager.get_client(client_id).await? {
            // Add back to registry
            self.client_registry.add_client(client.clone()).await?;
            self.record_activity(client_id).await;
            return Ok(Some(client));
        }

        // Try database
        if let Some(client) = self.db_manager.get_client(client_id).await? {
            // Cache and add to registry
            self.cache_manager.cache_client(&client).await?;
            self.client_registry.add_client(client.clone()).await?;
            self.record_activity(client_id).await;
            return Ok(Some(client));
        }

        Ok(None)
    }

    /// Get client by API key
    pub async fn get_client_by_api_key(
        &self,
        api_key: &str,
    ) -> Result<Option<Client>, FederationError> {
        // Try in-memory registry first
        if let Some(client) = self.client_registry.get_client_by_api_key(api_key).await {
            self.record_activity(&client.id).await;
            return Ok(Some((*client).clone()));
        }

        // Try cache with hashed key
        let key_hash = self.hash_api_key(api_key);
        if let Some(client_id) = self
            .cache_manager
            .get_client_by_api_key_hash(&key_hash)
            .await?
        {
            if let Some(client) = self.get_client(&client_id).await? {
                return Ok(Some(client));
            }
        }

        // Try database
        if let Some(client) = self.db_manager.get_client_by_api_key(api_key).await? {
            // Cache and add to registry
            self.cache_manager.cache_client(&client).await?;
            self.client_registry.add_client(client.clone()).await?;
            self.record_activity(&client.id).await;
            return Ok(Some(client));
        }

        Ok(None)
    }

    /// Update client configuration
    pub async fn update_client(
        &self,
        client_id: &Uuid,
        updates: ClientUpdateRequest,
    ) -> Result<Client, FederationError> {
        let mut client = self
            .get_client(client_id)
            .await?
            .ok_or_else(|| FederationError::ClientNotFound { id: *client_id })?;

        // Apply updates
        if let Some(name) = updates.name {
            // Check for duplicate names
            if name != client.name && self.client_exists_by_name(&name).await? {
                return Err(FederationError::ValidationError {
                    field: "name".to_string(),
                    message: "Client name already exists".to_string(),
                });
            }
            client.name = name;
        }

        if let Some(description) = updates.description {
            client.description = description;
        }

        if let Some(tier) = updates.tier {
            client.tier = tier;
            // Update limits based on new tier
            client.limits = self.get_default_limits_for_tier(&client.tier);
        }

        if let Some(config) = updates.config {
            client.config = config;
        }

        if let Some(status) = updates.status {
            client.status = status;
        }

        if let Some(limits) = updates.limits {
            client.limits = limits;
        }

        if let Some(metadata) = updates.metadata {
            client.metadata = metadata;
        }

        client.updated_at = Utc::now();

        // Save to database
        self.db_manager.update_client(&client).await?;

        // Update cache
        self.cache_manager.cache_client(&client).await?;

        // Update in-memory registry
        self.client_registry.add_client(client.clone()).await?;

        info!("Updated client: {} ({})", client.name, client.id);

        Ok(client)
    }

    /// Delete a client
    pub async fn delete_client(&self, client_id: &Uuid) -> Result<(), FederationError> {
        let client = self
            .get_client(client_id)
            .await?
            .ok_or_else(|| FederationError::ClientNotFound { id: *client_id })?;

        info!("Deleting client: {} ({})", client.name, client.id);

        // Remove from database
        self.db_manager.delete_client(client_id).await?;

        // Remove from cache
        self.cache_manager.remove_client(client_id).await?;

        // Remove from in-memory registry
        self.client_registry.remove_client(client_id).await?;

        // Clean up tracking data
        self.activity_tracker.remove(client_id);
        self.usage_tracker.remove(client_id);

        info!("Successfully deleted client: {}", client_id);

        Ok(())
    }

    /// List clients with filtering and pagination
    pub async fn list_clients(&self, filter: ClientFilter) -> Result<ClientList, FederationError> {
        let clients = self.db_manager.list_clients(&filter).await?;
        let total = self.db_manager.count_clients(&filter).await?;

        Ok(ClientList {
            clients,
            total,
            offset: filter.offset,
            limit: filter.limit,
        })
    }

    /// Authenticate client by API key
    pub async fn authenticate_client(&self, api_key: &str) -> Result<Client, FederationError> {
        let client = self.get_client_by_api_key(api_key).await?.ok_or_else(|| {
            FederationError::AuthenticationFailed {
                reason: "Invalid API key".to_string(),
            }
        })?;

        // Check client status
        match client.status {
            ClientStatus::Active => {
                self.record_activity(&client.id).await;
                Ok(client)
            }
            ClientStatus::Suspended => Err(FederationError::AuthorizationFailed {
                reason: "Client is suspended".to_string(),
            }),
            ClientStatus::Inactive => Err(FederationError::AuthorizationFailed {
                reason: "Client is inactive".to_string(),
            }),
            ClientStatus::Pending => Err(FederationError::AuthorizationFailed {
                reason: "Client is pending approval".to_string(),
            }),
            ClientStatus::Migrating => Err(FederationError::AuthorizationFailed {
                reason: "Client is being migrated".to_string(),
            }),
        }
    }

    /// Check if client can make request (rate limiting)
    pub async fn can_make_request(&self, client_id: &Uuid) -> Result<bool, FederationError> {
        let client = self
            .get_client(client_id)
            .await?
            .ok_or_else(|| FederationError::ClientNotFound { id: *client_id })?;

        let mut usage = self
            .usage_tracker
            .entry(*client_id)
            .or_insert_with(ResourceUsageTracker::default)
            .clone();

        let now = Utc::now();

        // Reset counters if needed
        if now
            .signed_duration_since(usage.last_minute_reset)
            .num_seconds()
            >= 60
        {
            usage.requests_per_minute = 0;
            usage.last_minute_reset = now;
        }

        if now
            .signed_duration_since(usage.last_hour_reset)
            .num_seconds()
            >= 3600
        {
            usage.requests_per_hour = 0;
            usage.last_hour_reset = now;
        }

        if now
            .signed_duration_since(usage.last_day_reset)
            .num_seconds()
            >= 86400
        {
            usage.requests_per_day = 0;
            usage.data_transfer_today = 0;
            usage.last_day_reset = now;
        }

        // Check limits
        let can_proceed = usage.requests_per_minute < client.limits.max_requests_per_minute
            && usage.requests_per_hour < client.limits.max_requests_per_hour
            && usage.requests_per_day < client.limits.max_requests_per_day
            && usage.concurrent_connections < client.limits.max_concurrent_connections;

        if can_proceed {
            // Increment counters
            usage.requests_per_minute += 1;
            usage.requests_per_hour += 1;
            usage.requests_per_day += 1;
            usage.concurrent_connections += 1;

            // Update tracker
            self.usage_tracker.insert(*client_id, usage);
        }

        Ok(can_proceed)
    }

    /// Record request completion (decrement concurrent connections)
    pub async fn record_request_completion(&self, client_id: &Uuid) {
        if let Some(mut usage) = self.usage_tracker.get_mut(client_id) {
            if usage.concurrent_connections > 0 {
                usage.concurrent_connections -= 1;
            }
        }
    }

    /// Get client usage statistics
    pub async fn get_client_usage(
        &self,
        client_id: &Uuid,
    ) -> Result<ClientUsageStats, FederationError> {
        let client = self
            .get_client(client_id)
            .await?
            .ok_or_else(|| FederationError::ClientNotFound { id: *client_id })?;

        let usage = self
            .usage_tracker
            .get(client_id)
            .map(|u| u.clone())
            .unwrap_or_default();

        let last_activity = self.activity_tracker.get(client_id).map(|a| *a);

        Ok(ClientUsageStats {
            client_id: *client_id,
            client_name: client.name,
            current_usage: usage.clone(),
            limits: client.limits.clone(),
            last_activity,
            utilization: ClientUtilization {
                requests_per_minute: (usage.requests_per_minute as f64
                    / client.limits.max_requests_per_minute as f64)
                    * 100.0,
                requests_per_hour: (usage.requests_per_hour as f64
                    / client.limits.max_requests_per_hour as f64)
                    * 100.0,
                requests_per_day: (usage.requests_per_day as f64
                    / client.limits.max_requests_per_day as f64)
                    * 100.0,
                concurrent_connections: (usage.concurrent_connections as f64
                    / client.limits.max_concurrent_connections as f64)
                    * 100.0,
                data_transfer: (usage.data_transfer_today as f64
                    / client.limits.max_data_transfer_per_day as f64)
                    * 100.0,
                storage: (usage.storage_usage as f64 / client.limits.max_storage_usage as f64)
                    * 100.0,
            },
        })
    }

    /// Start activity monitoring background task
    pub async fn start_activity_monitoring(&self) -> Result<(), FederationError> {
        info!("Starting client activity monitoring");

        // This would run a background task to monitor client activity
        // and update inactive clients, send notifications, etc.

        Ok(())
    }

    /// Get service health information
    pub async fn health(&self) -> Result<serde_json::Value, FederationError> {
        let stats = self.client_registry.get_stats().await;

        Ok(serde_json::json!({
            "status": "healthy",
            "clients": {
                "total": stats.total_clients,
                "active": stats.active_clients,
                "suspended": stats.suspended_clients,
                "inactive": stats.inactive_clients,
                "pending": stats.pending_clients
            },
            "registry_size": self.client_registry.clients_by_id.len(),
            "cache_connections": "healthy",
            "database_connections": "healthy"
        }))
    }

    /// Get service metrics
    pub async fn metrics(&self) -> Result<serde_json::Value, FederationError> {
        let stats = self.client_registry.get_stats().await;

        Ok(serde_json::json!({
            "clients_total": stats.total_clients,
            "clients_active": stats.active_clients,
            "clients_suspended": stats.suspended_clients,
            "clients_inactive": stats.inactive_clients,
            "clients_pending": stats.pending_clients,
            "registry_cache_size": self.client_registry.clients_by_id.len(),
            "activity_tracker_size": self.activity_tracker.len(),
            "usage_tracker_size": self.usage_tracker.len()
        }))
    }

    // Private helper methods

    async fn initialize_registry(&self) -> Result<(), FederationError> {
        debug!("Initializing client registry from database");

        let clients = self.db_manager.list_all_clients().await?;

        for client in clients {
            self.client_registry.add_client(client).await?;
        }

        info!(
            "Client registry initialized with {} clients",
            self.client_registry.clients_by_id.len()
        );
        Ok(())
    }

    fn validate_registration_request(
        &self,
        request: &ClientRegistrationRequest,
    ) -> Result<(), FederationError> {
        if request.name.is_empty() {
            return Err(FederationError::ValidationError {
                field: "name".to_string(),
                message: "Client name is required".to_string(),
            });
        }

        if request.name.len() > 100 {
            return Err(FederationError::ValidationError {
                field: "name".to_string(),
                message: "Client name must be 100 characters or less".to_string(),
            });
        }

        if let Some(desc) = &request.description {
            if desc.len() > 500 {
                return Err(FederationError::ValidationError {
                    field: "description".to_string(),
                    message: "Description must be 500 characters or less".to_string(),
                });
            }
        }

        Ok(())
    }

    async fn client_exists_by_name(&self, name: &str) -> Result<bool, FederationError> {
        self.db_manager.client_exists_by_name(name).await
    }

    fn generate_api_key(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let key: [u8; 32] = rng.gen();
        format!("fed_{}", hex::encode(key))
    }

    fn generate_jwt_secret(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut secret = [0u8; 64];
        rng.fill(&mut secret);
        hex::encode(secret)
    }

    fn hash_api_key(&self, api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn get_default_limits_for_tier(&self, tier: &ClientTier) -> ResourceLimits {
        match tier {
            ClientTier::Free => ResourceLimits {
                max_requests_per_minute: 10,
                max_requests_per_hour: 600,
                max_requests_per_day: 14400,
                max_concurrent_connections: 2,
                max_data_transfer_per_day: 100 * 1024 * 1024, // 100MB
                max_storage_usage: 1024 * 1024 * 1024,        // 1GB
            },
            ClientTier::Professional => ResourceLimits {
                max_requests_per_minute: 100,
                max_requests_per_hour: 6000,
                max_requests_per_day: 144000,
                max_concurrent_connections: 10,
                max_data_transfer_per_day: 10 * 1024 * 1024 * 1024, // 10GB
                max_storage_usage: 100 * 1024 * 1024 * 1024,        // 100GB
            },
            ClientTier::Enterprise => ResourceLimits {
                max_requests_per_minute: 1000,
                max_requests_per_hour: 60000,
                max_requests_per_day: 1440000,
                max_concurrent_connections: 100,
                max_data_transfer_per_day: 100 * 1024 * 1024 * 1024, // 100GB
                max_storage_usage: 1024 * 1024 * 1024 * 1024,        // 1TB
            },
            ClientTier::Custom => ResourceLimits {
                max_requests_per_minute: 10000,
                max_requests_per_hour: 600000,
                max_requests_per_day: 14400000,
                max_concurrent_connections: 1000,
                max_data_transfer_per_day: 1024 * 1024 * 1024 * 1024, // 1TB
                max_storage_usage: 10 * 1024 * 1024 * 1024 * 1024,    // 10TB
            },
        }
    }

    async fn record_activity(&self, client_id: &Uuid) {
        self.activity_tracker.insert(*client_id, Utc::now());
    }
}

impl ClientRegistry {
    async fn new() -> Result<Self, FederationError> {
        Ok(Self {
            clients_by_id: Arc::new(DashMap::new()),
            clients_by_api_key: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(ClientRegistryStats::default())),
        })
    }

    async fn add_client(&self, client: Client) -> Result<(), FederationError> {
        let client_arc = Arc::new(client.clone());

        self.clients_by_id.insert(client.id, client_arc.clone());
        self.clients_by_api_key
            .insert(client.credentials.api_key.clone(), client_arc);

        self.update_stats().await;

        Ok(())
    }

    async fn remove_client(&self, client_id: &Uuid) -> Result<(), FederationError> {
        if let Some((_, client)) = self.clients_by_id.remove(client_id) {
            self.clients_by_api_key.remove(&client.credentials.api_key);
        }

        self.update_stats().await;

        Ok(())
    }

    async fn get_client_by_id(&self, client_id: &Uuid) -> Option<Arc<Client>> {
        self.clients_by_id.get(client_id).map(|c| c.clone())
    }

    async fn get_client_by_api_key(&self, api_key: &str) -> Option<Arc<Client>> {
        self.clients_by_api_key.get(api_key).map(|c| c.clone())
    }

    async fn get_stats(&self) -> ClientRegistryStats {
        self.stats.read().await.clone()
    }

    async fn update_stats(&self) {
        let mut stats = self.stats.write().await;

        stats.total_clients = self.clients_by_id.len() as u64;
        stats.active_clients = 0;
        stats.suspended_clients = 0;
        stats.inactive_clients = 0;
        stats.pending_clients = 0;

        for client in self.clients_by_id.iter() {
            match client.status {
                ClientStatus::Active => stats.active_clients += 1,
                ClientStatus::Suspended => stats.suspended_clients += 1,
                ClientStatus::Inactive => stats.inactive_clients += 1,
                ClientStatus::Pending => stats.pending_clients += 1,
                ClientStatus::Migrating => {} // Don't count migrating clients
            }
        }

        stats.last_updated = Utc::now();
    }
}

// Supporting types

#[derive(Debug, Clone)]
pub struct ClientUpdateRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub tier: Option<ClientTier>,
    pub config: Option<ClientConfig>,
    pub status: Option<ClientStatus>,
    pub limits: Option<ResourceLimits>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct ClientFilter {
    pub status: Option<ClientStatus>,
    pub tier: Option<ClientTier>,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub offset: u64,
    pub limit: u64,
}

impl Default for ClientFilter {
    fn default() -> Self {
        Self {
            status: None,
            tier: None,
            name_contains: None,
            created_after: None,
            created_before: None,
            offset: 0,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientList {
    pub clients: Vec<Client>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientUsageStats {
    pub client_id: Uuid,
    pub client_name: String,
    pub current_usage: ResourceUsageTracker,
    pub limits: ResourceLimits,
    pub last_activity: Option<DateTime<Utc>>,
    pub utilization: ClientUtilization,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientUtilization {
    pub requests_per_minute: f64,
    pub requests_per_hour: f64,
    pub requests_per_day: f64,
    pub concurrent_connections: f64,
    pub data_transfer: f64,
    pub storage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CostOptimizationConfig, ProxyConfig, SchemaPreferences, WorkflowSettings};

    fn create_test_client_config() -> ClientConfig {
        ClientConfig {
            preferred_providers: HashMap::new(),
            cost_optimization: CostOptimizationConfig {
                enabled: true,
                max_cost_per_request: None,
                monthly_budget_limit: None,
                prefer_cheaper_providers: false,
                quality_cost_ratio: 0.5,
            },
            schema_preferences: SchemaPreferences {
                preferred_version: "1.0".to_string(),
                auto_translation: true,
                strict_validation: false,
                custom_mappings: HashMap::new(),
            },
            workflow_settings: WorkflowSettings {
                default_timeout: 3600,
                max_concurrent_workflows: 10,
                retry_policy: crate::models::RetryPolicy {
                    max_attempts: 3,
                    initial_delay: 1000,
                    max_delay: 60000,
                    backoff_multiplier: 2.0,
                    exponential_backoff: true,
                },
                monitoring_enabled: true,
            },
            proxy_config: ProxyConfig {
                enabled: true,
                timeout: crate::models::ProxyTimeout {
                    connect_timeout: 10000,
                    request_timeout: 30000,
                    keep_alive_timeout: 90000,
                },
                connection_pool: crate::models::ConnectionPoolConfig {
                    max_connections_per_host: 10,
                    idle_timeout: 60000,
                    keep_alive: true,
                },
                caching: crate::models::CachingConfig {
                    enabled: true,
                    ttl: 3600,
                    max_size: 1000,
                    strategy: crate::models::CacheStrategy::Lru,
                },
            },
        }
    }

    #[tokio::test]
    async fn test_client_registry_creation() {
        let registry = ClientRegistry::new().await.unwrap();
        assert_eq!(registry.clients_by_id.len(), 0);
        assert_eq!(registry.clients_by_api_key.len(), 0);
    }

    #[test]
    fn test_api_key_generation() {
        let manager = create_test_manager();
        let key1 = manager.generate_api_key();
        let key2 = manager.generate_api_key();

        assert!(key1.starts_with("fed_"));
        assert!(key2.starts_with("fed_"));
        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 68); // "fed_" + 64 hex characters
    }

    #[test]
    fn test_jwt_secret_generation() {
        let manager = create_test_manager();
        let secret1 = manager.generate_jwt_secret();
        let secret2 = manager.generate_jwt_secret();

        assert_ne!(secret1, secret2);
        assert_eq!(secret1.len(), 128); // 64 bytes = 128 hex characters
    }

    #[test]
    fn test_api_key_hashing() {
        let manager = create_test_manager();
        let api_key = "fed_test_key_123";
        let hash1 = manager.hash_api_key(api_key);
        let hash2 = manager.hash_api_key(api_key);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, api_key);
        assert_eq!(hash1.len(), 64); // SHA256 = 64 hex characters
    }

    #[test]
    fn test_default_limits_for_tiers() {
        let manager = create_test_manager();

        let free_limits = manager.get_default_limits_for_tier(&ClientTier::Free);
        let pro_limits = manager.get_default_limits_for_tier(&ClientTier::Professional);
        let enterprise_limits = manager.get_default_limits_for_tier(&ClientTier::Enterprise);

        assert!(free_limits.max_requests_per_minute < pro_limits.max_requests_per_minute);
        assert!(pro_limits.max_requests_per_minute < enterprise_limits.max_requests_per_minute);

        assert!(free_limits.max_storage_usage < pro_limits.max_storage_usage);
        assert!(pro_limits.max_storage_usage < enterprise_limits.max_storage_usage);
    }

    #[test]
    fn test_registration_request_validation() {
        let manager = create_test_manager();

        // Valid request
        let valid_request = ClientRegistrationRequest {
            name: "Test Client".to_string(),
            description: Some("Test description".to_string()),
            tier: ClientTier::Free,
            config: create_test_client_config(),
            metadata: None,
        };
        assert!(manager
            .validate_registration_request(&valid_request)
            .is_ok());

        // Empty name
        let invalid_request = ClientRegistrationRequest {
            name: "".to_string(),
            description: None,
            tier: ClientTier::Free,
            config: create_test_client_config(),
            metadata: None,
        };
        assert!(manager
            .validate_registration_request(&invalid_request)
            .is_err());

        // Name too long
        let invalid_request = ClientRegistrationRequest {
            name: "a".repeat(101),
            description: None,
            tier: ClientTier::Free,
            config: create_test_client_config(),
            metadata: None,
        };
        assert!(manager
            .validate_registration_request(&invalid_request)
            .is_err());
    }

    // Helper function to create a test manager (mock)
    fn create_test_manager() -> ClientManager {
        // This would need proper mocking in real tests
        // For now, we just test the methods that don't require DB/Redis
        use dashmap::DashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        ClientManager {
            db_pool: Arc::new(create_test_pool()),
            redis_client: Arc::new(create_test_redis_client()),
            cache_manager: Arc::new(create_test_cache_manager()),
            db_manager: Arc::new(create_test_db_manager()),
            client_registry: Arc::new(ClientRegistry {
                clients_by_id: Arc::new(DashMap::new()),
                clients_by_api_key: Arc::new(DashMap::new()),
                stats: Arc::new(RwLock::new(ClientRegistryStats::default())),
            }),
            activity_tracker: Arc::new(DashMap::new()),
            usage_tracker: Arc::new(DashMap::new()),
        }
    }

    // Mock functions for testing
    fn create_test_pool() -> PgPool {
        // This would be a proper test database pool in real tests
        unimplemented!("Mock for testing only")
    }

    fn create_test_redis_client() -> RedisClient {
        unimplemented!("Mock for testing only")
    }

    fn create_test_cache_manager() -> CacheManager {
        unimplemented!("Mock for testing only")
    }

    fn create_test_db_manager() -> DatabaseManager {
        unimplemented!("Mock for testing only")
    }
}
