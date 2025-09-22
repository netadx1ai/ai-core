//! Provider management for the Federation Service
//!
//! This module handles provider registration, discovery, selection, and health monitoring.
//! It provides comprehensive provider registry and management capabilities with intelligent
//! provider selection based on cost optimization, quality metrics, and availability.

use crate::models::{
    FederationError, Provider, ProviderConfig, ProviderSelectionRequest, ProviderSelectionResponse,
    ProviderStatus, ProviderType, QualityMetrics,
};
use crate::utils::{cache::CacheManager, database::DatabaseManager};
use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::Client as RedisClient;
use serde_json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Provider manager for comprehensive provider lifecycle management
#[derive(Debug, Clone)]
pub struct ProviderManager {
    /// Database connection pool
    db_pool: Arc<PgPool>,
    /// Redis client for caching
    redis_client: Arc<RedisClient>,
    /// Cache manager
    cache_manager: Arc<CacheManager>,
    /// Database manager
    db_manager: Arc<DatabaseManager>,
    /// In-memory provider registry for fast lookups
    provider_registry: Arc<ProviderRegistry>,
    /// Health monitor for provider availability
    health_monitor: Arc<ProviderHealthMonitor>,
    /// Selection engine for optimal provider selection
    selection_engine: Arc<ProviderSelectionEngine>,
}

/// Provider registry for in-memory caching and fast lookups
#[derive(Debug)]
pub struct ProviderRegistry {
    /// Providers indexed by ID
    providers_by_id: Arc<DashMap<Uuid, Arc<Provider>>>,
    /// Providers indexed by type
    providers_by_type: Arc<DashMap<ProviderType, Vec<Arc<Provider>>>>,
    /// Provider statistics
    stats: Arc<RwLock<ProviderRegistryStats>>,
}

/// Provider registry statistics
#[derive(Debug, Clone, Default)]
pub struct ProviderRegistryStats {
    /// Total number of providers
    pub total_providers: u64,
    /// Active providers
    pub active_providers: u64,
    /// Degraded providers
    pub degraded_providers: u64,
    /// Unavailable providers
    pub unavailable_providers: u64,
    /// Maintenance providers
    pub maintenance_providers: u64,
    /// Disabled providers
    pub disabled_providers: u64,
    /// Providers by type
    pub providers_by_type: HashMap<String, u64>,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Provider health monitor for continuous availability checking
#[derive(Debug)]
pub struct ProviderHealthMonitor {
    /// Health check interval in seconds
    check_interval: u64,
    /// Health check timeout in seconds
    check_timeout: u64,
    /// Provider health states
    health_states: Arc<DashMap<Uuid, ProviderHealthState>>,
    /// Health check statistics
    health_stats: Arc<RwLock<HealthMonitorStats>>,
}

/// Provider health state tracking
#[derive(Debug, Clone)]
pub struct ProviderHealthState {
    /// Provider ID
    pub provider_id: Uuid,
    /// Current status
    pub status: ProviderStatus,
    /// Last successful check
    pub last_success: Option<DateTime<Utc>>,
    /// Last failed check
    pub last_failure: Option<DateTime<Utc>>,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Consecutive successes
    pub consecutive_successes: u32,
    /// Response time history (last 10 checks)
    pub response_times: Vec<u64>,
    /// Error history
    pub error_history: Vec<HealthCheckError>,
}

/// Health check error information
#[derive(Debug, Clone)]
pub struct HealthCheckError {
    /// Error timestamp
    pub timestamp: DateTime<Utc>,
    /// Error message
    pub error: String,
    /// Error type
    pub error_type: HealthErrorType,
}

/// Health error types
#[derive(Debug, Clone)]
pub enum HealthErrorType {
    /// Connection timeout
    Timeout,
    /// Connection refused
    ConnectionRefused,
    /// DNS resolution failed
    DnsFailure,
    /// SSL/TLS error
    SslError,
    /// HTTP error
    HttpError(u16),
    /// Invalid response
    InvalidResponse,
    /// Unknown error
    Unknown,
}

/// Health monitor statistics
#[derive(Debug, Clone, Default)]
pub struct HealthMonitorStats {
    /// Total health checks performed
    pub total_checks: u64,
    /// Successful checks
    pub successful_checks: u64,
    /// Failed checks
    pub failed_checks: u64,
    /// Average response time
    pub avg_response_time: f64,
    /// Last check timestamp
    pub last_check: Option<DateTime<Utc>>,
}

/// Provider selection engine for optimal provider selection
#[derive(Debug)]
pub struct ProviderSelectionEngine {
    /// Selection strategies
    strategies: HashMap<String, Box<dyn SelectionStrategy + Send + Sync>>,
    /// Selection history for learning
    selection_history: Arc<DashMap<Uuid, Vec<SelectionRecord>>>,
    /// Performance metrics
    performance_metrics: Arc<DashMap<Uuid, PerformanceMetrics>>,
}

/// Selection strategy trait
pub trait SelectionStrategy: std::fmt::Debug {
    /// Select the best provider based on criteria
    fn select_provider(
        &self,
        providers: &[Arc<Provider>],
        request: &ProviderSelectionRequest,
    ) -> Result<Option<Arc<Provider>>, FederationError>;

    /// Get strategy name
    fn name(&self) -> &str;
}

/// Selection record for learning and optimization
#[derive(Debug, Clone)]
pub struct SelectionRecord {
    /// Selection timestamp
    pub timestamp: DateTime<Utc>,
    /// Selected provider
    pub provider_id: Uuid,
    /// Selection criteria
    pub criteria: ProviderSelectionRequest,
    /// Actual cost
    pub actual_cost: Option<f64>,
    /// Actual performance
    pub actual_performance: Option<f64>,
    /// Success/failure
    pub success: bool,
}

/// Performance metrics for providers
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Average response time
    pub avg_response_time: f64,
    /// Success rate
    pub success_rate: f64,
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

impl ProviderManager {
    /// Create a new provider manager
    pub async fn new(db_pool: PgPool, redis_client: RedisClient) -> Result<Self, FederationError> {
        let db_pool = Arc::new(db_pool);
        let redis_client = Arc::new(redis_client);

        let cache_manager = Arc::new(CacheManager::new(redis_client.clone()).await?);
        let db_manager = Arc::new(DatabaseManager::new(db_pool.clone()).await?);
        let provider_registry = Arc::new(ProviderRegistry::new().await?);
        let health_monitor = Arc::new(ProviderHealthMonitor::new(30, 10).await?);
        let selection_engine = Arc::new(ProviderSelectionEngine::new().await?);

        let manager = Self {
            db_pool,
            redis_client,
            cache_manager,
            db_manager,
            provider_registry,
            health_monitor,
            selection_engine,
        };

        // Initialize provider registry from database
        manager.initialize_registry().await?;

        Ok(manager)
    }

    /// Register a new provider
    pub async fn register_provider(
        &self,
        mut provider: Provider,
    ) -> Result<Provider, FederationError> {
        info!("Registering new provider: {}", provider.name);

        // Validate the provider
        self.validate_provider(&provider)?;

        // Check for duplicate names
        if self.provider_exists_by_name(&provider.name).await? {
            return Err(FederationError::ValidationError {
                field: "name".to_string(),
                message: "Provider name already exists".to_string(),
            });
        }

        // Set timestamps
        provider.created_at = Utc::now();
        provider.updated_at = Utc::now();

        // Save to database
        self.db_manager.create_provider(&provider).await?;

        // Cache the provider
        self.cache_manager.cache_provider(&provider).await?;

        // Add to in-memory registry
        self.provider_registry
            .add_provider(provider.clone())
            .await?;

        // Initialize health monitoring
        self.health_monitor.add_provider(provider.id).await?;

        info!(
            "Successfully registered provider: {} ({})",
            provider.name, provider.id
        );

        Ok(provider)
    }

    /// Get provider by ID
    pub async fn get_provider(
        &self,
        provider_id: &Uuid,
    ) -> Result<Option<Provider>, FederationError> {
        // Try in-memory registry first
        if let Some(provider) = self.provider_registry.get_provider_by_id(provider_id).await {
            return Ok(Some((*provider).clone()));
        }

        // Try cache
        if let Some(provider) = self.cache_manager.get_provider(provider_id).await? {
            // Add back to registry
            self.provider_registry
                .add_provider(provider.clone())
                .await?;
            return Ok(Some(provider));
        }

        // Try database
        if let Some(provider) = self.db_manager.get_provider(provider_id).await? {
            // Cache and add to registry
            self.cache_manager.cache_provider(&provider).await?;
            self.provider_registry
                .add_provider(provider.clone())
                .await?;
            return Ok(Some(provider));
        }

        Ok(None)
    }

    /// Get providers by type
    pub async fn get_providers_by_type(
        &self,
        provider_type: &ProviderType,
    ) -> Result<Vec<Provider>, FederationError> {
        let providers = self
            .provider_registry
            .get_providers_by_type(provider_type)
            .await;
        Ok(providers.into_iter().map(|p| (*p).clone()).collect())
    }

    /// Update provider configuration
    pub async fn update_provider(
        &self,
        provider_id: &Uuid,
        updates: ProviderUpdateRequest,
    ) -> Result<Provider, FederationError> {
        let mut provider = self
            .get_provider(provider_id)
            .await?
            .ok_or_else(|| FederationError::ProviderNotFound { id: *provider_id })?;

        // Apply updates
        if let Some(name) = updates.name {
            if name != provider.name && self.provider_exists_by_name(&name).await? {
                return Err(FederationError::ValidationError {
                    field: "name".to_string(),
                    message: "Provider name already exists".to_string(),
                });
            }
            provider.name = name;
        }

        if let Some(config) = updates.config {
            provider.config = config;
        }

        if let Some(cost_info) = updates.cost_info {
            provider.cost_info = cost_info;
        }

        if let Some(status) = updates.status {
            provider.status = status;
        }

        if let Some(capabilities) = updates.capabilities {
            provider.capabilities = capabilities;
        }

        if let Some(health_endpoint) = updates.health_endpoint {
            provider.health_endpoint = health_endpoint;
        }

        provider.updated_at = Utc::now();

        // Save to database
        self.db_manager.update_provider(&provider).await?;

        // Update cache
        self.cache_manager.cache_provider(&provider).await?;

        // Update in-memory registry
        self.provider_registry
            .add_provider(provider.clone())
            .await?;

        info!("Updated provider: {} ({})", provider.name, provider.id);

        Ok(provider)
    }

    /// Delete a provider
    pub async fn delete_provider(&self, provider_id: &Uuid) -> Result<(), FederationError> {
        let provider = self
            .get_provider(provider_id)
            .await?
            .ok_or_else(|| FederationError::ProviderNotFound { id: *provider_id })?;

        info!("Deleting provider: {} ({})", provider.name, provider.id);

        // Remove from database
        self.db_manager.delete_provider(provider_id).await?;

        // Remove from cache
        self.cache_manager.remove_provider(provider_id).await?;

        // Remove from in-memory registry
        self.provider_registry.remove_provider(provider_id).await?;

        // Stop health monitoring
        self.health_monitor.remove_provider(provider_id).await?;

        info!("Successfully deleted provider: {}", provider_id);

        Ok(())
    }

    /// Select optimal provider based on criteria
    pub async fn select_provider(
        &self,
        request: ProviderSelectionRequest,
    ) -> Result<ProviderSelectionResponse, FederationError> {
        debug!("Selecting provider for client: {}", request.client_id);

        // Get available providers of the requested type
        let providers = self.get_providers_by_type(&request.service_type).await?;

        // Filter providers based on availability and capabilities
        let available_providers: Vec<Arc<Provider>> = providers
            .into_iter()
            .filter(|p| {
                // Check status
                matches!(p.status, ProviderStatus::Active) &&
                // Check capabilities
                request.required_capabilities.iter().all(|cap| p.capabilities.contains(cap))
            })
            .map(Arc::new)
            .collect();

        if available_providers.is_empty() {
            return Err(FederationError::ProviderSelectionFailed {
                reason: "No available providers match the criteria".to_string(),
            });
        }

        // Use selection engine to choose the best provider
        let selected_provider = self
            .selection_engine
            .select_best_provider(&available_providers, &request)
            .await?
            .ok_or_else(|| FederationError::ProviderSelectionFailed {
                reason: "Selection engine failed to choose a provider".to_string(),
            })?;

        // Calculate estimated cost
        let estimated_cost = self
            .calculate_estimated_cost(&selected_provider, &request)
            .await?;

        // Get expected quality metrics
        let expected_quality = self.get_expected_quality(&selected_provider.id).await?;

        // Record selection for learning
        self.selection_engine
            .record_selection(&selected_provider.id, &request, estimated_cost)
            .await?;

        let reasoning = format!(
            "Selected {} based on optimal cost-quality ratio: ${:.4} estimated cost, {:.2}% success rate",
            selected_provider.name,
            estimated_cost,
            expected_quality.success_rate * 100.0
        );

        info!(
            "Selected provider: {} for client: {}",
            selected_provider.name, request.client_id
        );

        Ok(ProviderSelectionResponse {
            provider: (*selected_provider).clone(),
            reasoning,
            estimated_cost,
            expected_quality,
        })
    }

    /// Start health monitoring background task
    pub async fn start_health_monitoring(&self) -> Result<(), FederationError> {
        info!("Starting provider health monitoring");

        let health_monitor = self.health_monitor.clone();
        let provider_registry = self.provider_registry.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(health_monitor.check_interval));

            loop {
                interval.tick().await;

                if let Err(e) = Self::run_health_checks(&health_monitor, &provider_registry).await {
                    error!("Health check cycle failed: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Get service health information
    pub async fn health(&self) -> Result<serde_json::Value, FederationError> {
        let stats = self.provider_registry.get_stats().await;
        let health_stats = self.health_monitor.get_stats().await;

        Ok(serde_json::json!({
            "status": "healthy",
            "providers": {
                "total": stats.total_providers,
                "active": stats.active_providers,
                "degraded": stats.degraded_providers,
                "unavailable": stats.unavailable_providers,
                "maintenance": stats.maintenance_providers,
                "disabled": stats.disabled_providers
            },
            "health_monitoring": {
                "total_checks": health_stats.total_checks,
                "successful_checks": health_stats.successful_checks,
                "failed_checks": health_stats.failed_checks,
                "avg_response_time": health_stats.avg_response_time
            },
            "registry_size": self.provider_registry.providers_by_id.len()
        }))
    }

    /// Get service metrics
    pub async fn metrics(&self) -> Result<serde_json::Value, FederationError> {
        let stats = self.provider_registry.get_stats().await;
        let health_stats = self.health_monitor.get_stats().await;

        Ok(serde_json::json!({
            "providers_total": stats.total_providers,
            "providers_active": stats.active_providers,
            "providers_degraded": stats.degraded_providers,
            "providers_unavailable": stats.unavailable_providers,
            "health_checks_total": health_stats.total_checks,
            "health_checks_successful": health_stats.successful_checks,
            "health_checks_failed": health_stats.failed_checks,
            "avg_response_time": health_stats.avg_response_time,
            "registry_cache_size": self.provider_registry.providers_by_id.len()
        }))
    }

    // Private helper methods

    async fn initialize_registry(&self) -> Result<(), FederationError> {
        debug!("Initializing provider registry from database");

        let providers = self.db_manager.list_all_providers().await?;

        for provider in providers {
            self.provider_registry.add_provider(provider).await?;
        }

        info!(
            "Provider registry initialized with {} providers",
            self.provider_registry.providers_by_id.len()
        );
        Ok(())
    }

    fn validate_provider(&self, provider: &Provider) -> Result<(), FederationError> {
        if provider.name.is_empty() {
            return Err(FederationError::ValidationError {
                field: "name".to_string(),
                message: "Provider name is required".to_string(),
            });
        }

        if provider.config.endpoint.is_empty() {
            return Err(FederationError::ValidationError {
                field: "config.endpoint".to_string(),
                message: "Provider endpoint is required".to_string(),
            });
        }

        // Validate endpoint URL
        if !provider.config.endpoint.starts_with("http://")
            && !provider.config.endpoint.starts_with("https://")
        {
            return Err(FederationError::ValidationError {
                field: "config.endpoint".to_string(),
                message: "Provider endpoint must be a valid HTTP(S) URL".to_string(),
            });
        }

        Ok(())
    }

    async fn provider_exists_by_name(&self, name: &str) -> Result<bool, FederationError> {
        self.db_manager.provider_exists_by_name(name).await
    }

    async fn calculate_estimated_cost(
        &self,
        provider: &Provider,
        _request: &ProviderSelectionRequest,
    ) -> Result<f64, FederationError> {
        // Simple cost estimation based on provider's cost info
        // In a real implementation, this would be more sophisticated
        Ok(provider.cost_info.cost_per_request)
    }

    async fn get_expected_quality(
        &self,
        provider_id: &Uuid,
    ) -> Result<QualityMetrics, FederationError> {
        if let Some(metrics) = self.selection_engine.performance_metrics.get(provider_id) {
            Ok(QualityMetrics {
                avg_response_time: metrics.avg_response_time,
                success_rate: metrics.success_rate,
                availability: if metrics.success_rate > 0.95 {
                    0.99
                } else {
                    0.95
                },
                quality_score: metrics.success_rate * 0.7
                    + (1.0 / (1.0 + metrics.avg_response_time / 1000.0)) * 0.3,
                last_updated: metrics.last_updated,
            })
        } else {
            // Default metrics for new providers
            Ok(QualityMetrics {
                avg_response_time: 100.0,
                success_rate: 0.99,
                availability: 0.99,
                quality_score: 0.95,
                last_updated: Utc::now(),
            })
        }
    }

    async fn run_health_checks(
        health_monitor: &ProviderHealthMonitor,
        provider_registry: &ProviderRegistry,
    ) -> Result<(), FederationError> {
        let providers: Vec<_> = provider_registry
            .providers_by_id
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for provider_id in providers {
            if let Err(e) = health_monitor.check_provider_health(&provider_id).await {
                warn!("Health check failed for provider {}: {}", provider_id, e);
            }
        }

        Ok(())
    }
}

impl ProviderRegistry {
    async fn new() -> Result<Self, FederationError> {
        Ok(Self {
            providers_by_id: Arc::new(DashMap::new()),
            providers_by_type: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(ProviderRegistryStats::default())),
        })
    }

    async fn add_provider(&self, provider: Provider) -> Result<(), FederationError> {
        let provider_arc = Arc::new(provider.clone());

        // Add to ID index
        self.providers_by_id
            .insert(provider.id, provider_arc.clone());

        // Add to type index
        self.providers_by_type
            .entry(provider.provider_type.clone())
            .or_insert_with(Vec::new)
            .push(provider_arc);

        self.update_stats().await;

        Ok(())
    }

    async fn remove_provider(&self, provider_id: &Uuid) -> Result<(), FederationError> {
        if let Some((_, provider)) = self.providers_by_id.remove(provider_id) {
            // Remove from type index
            if let Some(mut providers) = self.providers_by_type.get_mut(&provider.provider_type) {
                providers.retain(|p| p.id != *provider_id);
            }
        }

        self.update_stats().await;

        Ok(())
    }

    async fn get_provider_by_id(&self, provider_id: &Uuid) -> Option<Arc<Provider>> {
        self.providers_by_id.get(provider_id).map(|p| p.clone())
    }

    async fn get_providers_by_type(&self, provider_type: &ProviderType) -> Vec<Arc<Provider>> {
        self.providers_by_type
            .get(provider_type)
            .map(|providers| providers.clone())
            .unwrap_or_default()
    }

    async fn get_stats(&self) -> ProviderRegistryStats {
        self.stats.read().await.clone()
    }

    async fn update_stats(&self) {
        let mut stats = self.stats.write().await;

        stats.total_providers = self.providers_by_id.len() as u64;
        stats.active_providers = 0;
        stats.degraded_providers = 0;
        stats.unavailable_providers = 0;
        stats.maintenance_providers = 0;
        stats.disabled_providers = 0;
        stats.providers_by_type.clear();

        for provider in self.providers_by_id.iter() {
            match provider.status {
                ProviderStatus::Active => stats.active_providers += 1,
                ProviderStatus::Degraded => stats.degraded_providers += 1,
                ProviderStatus::Unavailable => stats.unavailable_providers += 1,
                ProviderStatus::Maintenance => stats.maintenance_providers += 1,
                ProviderStatus::Disabled => stats.disabled_providers += 1,
            }

            let type_key = format!("{:?}", provider.provider_type);
            *stats.providers_by_type.entry(type_key).or_insert(0) += 1;
        }

        stats.last_updated = Utc::now();
    }
}

impl ProviderHealthMonitor {
    async fn new(check_interval: u64, check_timeout: u64) -> Result<Self, FederationError> {
        Ok(Self {
            check_interval,
            check_timeout,
            health_states: Arc::new(DashMap::new()),
            health_stats: Arc::new(RwLock::new(HealthMonitorStats::default())),
        })
    }

    async fn add_provider(&self, provider_id: Uuid) -> Result<(), FederationError> {
        self.health_states.insert(
            provider_id,
            ProviderHealthState {
                provider_id,
                status: ProviderStatus::Active,
                last_success: None,
                last_failure: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                response_times: Vec::new(),
                error_history: Vec::new(),
            },
        );

        Ok(())
    }

    async fn remove_provider(&self, provider_id: &Uuid) -> Result<(), FederationError> {
        self.health_states.remove(provider_id);
        Ok(())
    }

    async fn check_provider_health(&self, provider_id: &Uuid) -> Result<(), FederationError> {
        // This would implement actual health checking logic
        // For now, we'll simulate it

        let now = Utc::now();
        let success = rand::random::<f64>() > 0.1; // 90% success rate simulation

        if let Some(mut state) = self.health_states.get_mut(provider_id) {
            if success {
                state.last_success = Some(now);
                state.consecutive_successes += 1;
                state.consecutive_failures = 0;
                state.response_times.push(100); // Simulated response time

                // Keep only last 10 response times
                if state.response_times.len() > 10 {
                    state.response_times.remove(0);
                }

                // Update status based on consecutive successes
                if state.consecutive_successes >= 3 {
                    state.status = ProviderStatus::Active;
                }
            } else {
                state.last_failure = Some(now);
                state.consecutive_failures += 1;
                state.consecutive_successes = 0;

                let error = HealthCheckError {
                    timestamp: now,
                    error: "Simulated health check failure".to_string(),
                    error_type: HealthErrorType::Timeout,
                };
                state.error_history.push(error);

                // Keep only last 10 errors
                if state.error_history.len() > 10 {
                    state.error_history.remove(0);
                }

                // Update status based on consecutive failures
                if state.consecutive_failures >= 3 {
                    state.status = ProviderStatus::Unavailable;
                } else if state.consecutive_failures >= 2 {
                    state.status = ProviderStatus::Degraded;
                }
            }
        }

        // Update global stats
        let mut stats = self.health_stats.write().await;
        stats.total_checks += 1;
        if success {
            stats.successful_checks += 1;
        } else {
            stats.failed_checks += 1;
        }
        stats.last_check = Some(now);

        // Calculate average response time
        if stats.successful_checks > 0 {
            stats.avg_response_time = 100.0; // Simplified calculation
        }

        Ok(())
    }

    async fn get_stats(&self) -> HealthMonitorStats {
        self.health_stats.read().await.clone()
    }
}

impl ProviderSelectionEngine {
    async fn new() -> Result<Self, FederationError> {
        let mut strategies: HashMap<String, Box<dyn SelectionStrategy + Send + Sync>> =
            HashMap::new();

        strategies.insert(
            "cost_optimized".to_string(),
            Box::new(CostOptimizedStrategy),
        );
        strategies.insert(
            "quality_optimized".to_string(),
            Box::new(QualityOptimizedStrategy),
        );
        strategies.insert("balanced".to_string(), Box::new(BalancedStrategy));
        strategies.insert(
            "round_robin".to_string(),
            Box::new(RoundRobinStrategy::new()),
        );

        Ok(Self {
            strategies,
            selection_history: Arc::new(DashMap::new()),
            performance_metrics: Arc::new(DashMap::new()),
        })
    }

    async fn select_best_provider(
        &self,
        providers: &[Arc<Provider>],
        request: &ProviderSelectionRequest,
    ) -> Result<Option<Arc<Provider>>, FederationError> {
        // Use balanced strategy by default
        let strategy = self.strategies.get("balanced").unwrap();
        strategy.select_provider(providers, request)
    }

    async fn record_selection(
        &self,
        provider_id: &Uuid,
        request: &ProviderSelectionRequest,
        estimated_cost: f64,
    ) -> Result<(), FederationError> {
        let record = SelectionRecord {
            timestamp: Utc::now(),
            provider_id: *provider_id,
            criteria: request.clone(),
            actual_cost: Some(estimated_cost),
            actual_performance: None,
            success: true, // This would be updated after actual execution
        };

        self.selection_history
            .entry(*provider_id)
            .or_insert_with(Vec::new)
            .push(record);

        Ok(())
    }
}

// Selection strategy implementations

#[derive(Debug)]
struct CostOptimizedStrategy;

impl SelectionStrategy for CostOptimizedStrategy {
    fn select_provider(
        &self,
        providers: &[Arc<Provider>],
        _request: &ProviderSelectionRequest,
    ) -> Result<Option<Arc<Provider>>, FederationError> {
        let cheapest = providers.iter().min_by(|a, b| {
            a.cost_info
                .cost_per_request
                .partial_cmp(&b.cost_info.cost_per_request)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(cheapest.cloned())
    }

    fn name(&self) -> &str {
        "cost_optimized"
    }
}

#[derive(Debug)]
struct QualityOptimizedStrategy;

impl SelectionStrategy for QualityOptimizedStrategy {
    fn select_provider(
        &self,
        providers: &[Arc<Provider>],
        _request: &ProviderSelectionRequest,
    ) -> Result<Option<Arc<Provider>>, FederationError> {
        let highest_quality = providers.iter().max_by(|a, b| {
            a.quality_metrics
                .quality_score
                .partial_cmp(&b.quality_metrics.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(highest_quality.cloned())
    }

    fn name(&self) -> &str {
        "quality_optimized"
    }
}

#[derive(Debug)]
struct BalancedStrategy;

impl SelectionStrategy for BalancedStrategy {
    fn select_provider(
        &self,
        providers: &[Arc<Provider>],
        _request: &ProviderSelectionRequest,
    ) -> Result<Option<Arc<Provider>>, FederationError> {
        let mut scored_providers: Vec<(Arc<Provider>, f64)> = Vec::new();

        for provider in providers {
            // Calculate a balanced score (higher is better)
            let cost_score = 1.0 / (provider.cost_info.cost_per_request + 0.001);
            let quality_score = provider.quality_metrics.quality_score;
            let balanced_score = cost_score * 0.4 + quality_score * 0.6;

            scored_providers.push((provider.clone(), balanced_score));
        }

        // Sort by score (highest first)
        scored_providers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_providers
            .into_iter()
            .next()
            .map(|(provider, _)| provider))
    }

    fn name(&self) -> &str {
        "balanced"
    }
}

#[derive(Debug)]
struct RoundRobinStrategy {
    counter: std::sync::atomic::AtomicUsize,
}

impl RoundRobinStrategy {
    fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl SelectionStrategy for RoundRobinStrategy {
    fn select_provider(
        &self,
        providers: &[Arc<Provider>],
        _request: &ProviderSelectionRequest,
    ) -> Result<Option<Arc<Provider>>, FederationError> {
        if providers.is_empty() {
            return Ok(None);
        }

        let index = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % providers.len();
        Ok(Some(providers[index].clone()))
    }

    fn name(&self) -> &str {
        "round_robin"
    }
}

// Supporting types

#[derive(Debug, Clone)]
pub struct ProviderUpdateRequest {
    pub name: Option<String>,
    pub config: Option<ProviderConfig>,
    pub cost_info: Option<crate::models::CostInfo>,
    pub status: Option<ProviderStatus>,
    pub capabilities: Option<Vec<String>>,
    pub health_endpoint: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AuthMethod, CostInfo, RateLimits};

    #[test]
    fn test_provider_registry_creation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let registry = ProviderRegistry::new().await.unwrap();
            assert_eq!(registry.providers_by_id.len(), 0);
            assert_eq!(registry.providers_by_type.len(), 0);
        });
    }

    #[test]
    fn test_cost_optimized_strategy() {
        let strategy = CostOptimizedStrategy;
        let providers = vec![
            Arc::new(create_test_provider("Provider A", 0.50)),
            Arc::new(create_test_provider("Provider B", 0.25)),
            Arc::new(create_test_provider("Provider C", 0.75)),
        ];

        let request = create_test_selection_request();
        let result = strategy.select_provider(&providers, &request).unwrap();

        assert!(result.is_some());
        let selected = result.unwrap();
        assert_eq!(selected.name, "Provider B"); // Cheapest
        assert_eq!(selected.cost_info.cost_per_request, 0.25);
    }

    #[test]
    fn test_quality_optimized_strategy() {
        let strategy = QualityOptimizedStrategy;
        let providers = vec![
            Arc::new(create_test_provider_with_quality("Provider A", 0.50, 0.95)),
            Arc::new(create_test_provider_with_quality("Provider B", 0.25, 0.85)),
            Arc::new(create_test_provider_with_quality("Provider C", 0.75, 0.99)),
        ];

        let request = create_test_selection_request();
        let result = strategy.select_provider(&providers, &request).unwrap();

        assert!(result.is_some());
        let selected = result.unwrap();
        assert_eq!(selected.name, "Provider C"); // Highest quality
    }

    #[test]
    fn test_round_robin_strategy() {
        let strategy = RoundRobinStrategy::new();
        let providers = vec![
            Arc::new(create_test_provider("Provider A", 0.50)),
            Arc::new(create_test_provider("Provider B", 0.25)),
            Arc::new(create_test_provider("Provider C", 0.75)),
        ];

        let request = create_test_selection_request();

        // Test multiple selections for round-robin behavior
        let result1 = strategy
            .select_provider(&providers, &request)
            .unwrap()
            .unwrap();
        let result2 = strategy
            .select_provider(&providers, &request)
            .unwrap()
            .unwrap();
        let result3 = strategy
            .select_provider(&providers, &request)
            .unwrap()
            .unwrap();
        let result4 = strategy
            .select_provider(&providers, &request)
            .unwrap()
            .unwrap();

        // Should cycle through providers
        assert_eq!(result1.name, "Provider A");
        assert_eq!(result2.name, "Provider B");
        assert_eq!(result3.name, "Provider C");
        assert_eq!(result4.name, "Provider A"); // Back to first
    }

    fn create_test_provider(name: &str, cost_per_request: f64) -> Provider {
        Provider {
            id: Uuid::new_v4(),
            name: name.to_string(),
            provider_type: ProviderType::Llm,
            config: ProviderConfig {
                endpoint: "http://example.com".to_string(),
                auth_method: AuthMethod::None,
                timeout: 30000,
                rate_limits: RateLimits {
                    requests_per_second: None,
                    requests_per_minute: None,
                    requests_per_hour: None,
                    concurrent_requests: None,
                },
                headers: HashMap::new(),
            },
            cost_info: CostInfo {
                cost_per_request,
                cost_per_token: None,
                cost_per_gb: None,
                cost_per_compute_hour: None,
                minimum_cost: 0.0,
                currency: "USD".to_string(),
            },
            quality_metrics: QualityMetrics {
                avg_response_time: 100.0,
                success_rate: 0.99,
                availability: 0.99,
                quality_score: 0.95,
                last_updated: Utc::now(),
            },
            status: ProviderStatus::Active,
            capabilities: vec!["test".to_string()],
            health_endpoint: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_provider_with_quality(
        name: &str,
        cost_per_request: f64,
        quality_score: f64,
    ) -> Provider {
        let mut provider = create_test_provider(name, cost_per_request);
        provider.quality_metrics.quality_score = quality_score;
        provider
    }

    fn create_test_selection_request() -> ProviderSelectionRequest {
        ProviderSelectionRequest {
            client_id: Uuid::new_v4(),
            service_type: ProviderType::Llm,
            required_capabilities: vec!["test".to_string()],
            cost_constraints: None,
            quality_requirements: None,
        }
    }
}
