//! Health Monitoring Module
//!
//! This module provides comprehensive health monitoring capabilities for MCP servers,
//! including health checks, status tracking, and automated recovery mechanisms.

use crate::{
    models::{HealthCheck, HealthDetails, HealthStatus, ServerInfo, ServerStatus},
    registry::ServerRegistry,
    McpError, Result,
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Health monitor for managing server health checks
#[derive(Debug)]
pub struct HealthMonitor {
    /// HTTP client for health checks
    client: Client,

    /// Server registry reference
    registry: Arc<ServerRegistry>,

    /// Health monitor configuration
    config: HealthConfig,

    /// Health check results cache
    health_cache: Arc<RwLock<HashMap<Uuid, HealthCheck>>>,

    /// Health check statistics
    stats: Arc<RwLock<HealthStats>>,
}

/// Health monitoring configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Health check interval in seconds
    pub check_interval_seconds: u64,

    /// Health check timeout in seconds
    pub check_timeout_seconds: u64,

    /// Number of failed checks before marking unhealthy
    pub failure_threshold: u32,

    /// Number of successful checks before marking healthy
    pub success_threshold: u32,

    /// Enable detailed health metrics
    pub detailed_metrics: bool,

    /// Health check endpoints
    pub endpoints: Vec<String>,

    /// Enable automatic recovery
    pub auto_recovery: bool,

    /// Maximum recovery attempts
    pub max_recovery_attempts: u32,

    /// Recovery backoff multiplier
    pub recovery_backoff_multiplier: f64,
}

/// Health monitoring statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HealthStats {
    /// Total health checks performed
    pub total_checks: u64,

    /// Total successful checks
    pub successful_checks: u64,

    /// Total failed checks
    pub failed_checks: u64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// Minimum response time in milliseconds
    pub min_response_time_ms: u64,

    /// Maximum response time in milliseconds
    pub max_response_time_ms: u64,

    /// Checks performed in the last hour
    pub checks_last_hour: u64,

    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Server health state tracking
#[derive(Debug, Clone)]
struct ServerHealthState {
    /// Current health status
    status: HealthStatus,

    /// Consecutive successful checks
    consecutive_successes: u32,

    /// Consecutive failed checks
    consecutive_failures: u32,

    /// Last health check timestamp
    last_check: Option<DateTime<Utc>>,

    /// Last successful check timestamp
    last_success: Option<DateTime<Utc>>,

    /// Last failure timestamp
    last_failure: Option<DateTime<Utc>>,

    /// Recovery attempts
    recovery_attempts: u32,

    /// Next recovery attempt timestamp
    next_recovery_attempt: Option<DateTime<Utc>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(registry: Arc<ServerRegistry>, config: HealthConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.check_timeout_seconds))
            .user_agent("AI-CORE MCP Manager Health Monitor")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            registry,
            config,
            health_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HealthStats::default())),
        }
    }

    /// Start the health monitoring service
    pub async fn start(&self) -> Result<()> {
        info!("Starting health monitoring service");

        let mut interval = interval(Duration::from_secs(self.config.check_interval_seconds));

        loop {
            interval.tick().await;

            if let Err(e) = self.perform_health_checks().await {
                error!("Health check cycle failed: {}", e);
            }
        }
    }

    /// Perform health checks for all servers
    async fn perform_health_checks(&self) -> Result<()> {
        let servers = self.registry.list().await;

        debug!("Performing health checks for {} servers", servers.len());

        let mut check_tasks = Vec::new();

        for server in servers {
            // Skip servers that are not in a state where health checks make sense
            if !matches!(
                server.status,
                ServerStatus::Running | ServerStatus::Unhealthy | ServerStatus::Unknown
            ) {
                continue;
            }

            let client = self.client.clone();
            let config = self.config.clone();
            let server_clone = server.clone();

            let task = tokio::spawn(async move {
                Self::check_server_health(client, server_clone, config).await
            });

            check_tasks.push(task);
        }

        // Wait for all health checks to complete
        let results = futures::future::join_all(check_tasks).await;

        let mut successful_checks = 0;
        let mut failed_checks = 0;

        for result in results {
            match result {
                Ok(Ok(health_check)) => {
                    successful_checks += 1;
                    if let Err(e) = self.process_health_check_result(health_check).await {
                        error!("Failed to process health check result: {}", e);
                    }
                }
                Ok(Err(e)) => {
                    failed_checks += 1;
                    error!("Health check failed: {}", e);
                }
                Err(e) => {
                    failed_checks += 1;
                    error!("Health check task failed: {}", e);
                }
            }
        }

        // Update statistics
        let _ = self.update_stats(successful_checks, failed_checks).await;

        debug!(
            "Health check cycle completed: {} successful, {} failed",
            successful_checks, failed_checks
        );

        Ok(())
    }

    /// Check health of a single server
    async fn check_server_health(
        client: Client,
        server: ServerInfo,
        config: HealthConfig,
    ) -> Result<HealthCheck> {
        let start_time = Instant::now();

        debug!(
            server_id = %server.id,
            server_name = %server.name,
            "Performing health check"
        );

        // Try each configured health endpoint
        let mut last_error = None;

        for endpoint in &config.endpoints {
            let url = format!("{}{}", server.config.endpoint, endpoint);

            match timeout(
                Duration::from_secs(config.check_timeout_seconds),
                client.get(&url).send(),
            )
            .await
            {
                Ok(Ok(response)) => {
                    let response_time = start_time.elapsed().as_millis() as u64;

                    if response.status().is_success() {
                        let mut health_check =
                            HealthCheck::new(server.id, HealthStatus::Healthy, response_time);

                        // Try to parse detailed health information from response
                        if config.detailed_metrics {
                            if let Ok(body) = response.text().await {
                                if let Ok(details) = serde_json::from_str::<HealthDetails>(&body) {
                                    health_check.details = details;
                                }
                            }
                        }

                        return Ok(health_check);
                    } else {
                        last_error = Some(format!(
                            "Health check returned status: {}",
                            response.status()
                        ));
                    }
                }
                Ok(Err(e)) => {
                    last_error = Some(format!("HTTP request failed: {}", e));
                }
                Err(_) => {
                    last_error = Some("Health check timed out".to_string());
                }
            }
        }

        // All health checks failed
        let error_message =
            last_error.unwrap_or_else(|| "Unknown health check failure".to_string());
        Ok(HealthCheck::failed(server.id, error_message))
    }

    /// Process a health check result and update server status
    async fn process_health_check_result(&self, health_check: HealthCheck) -> Result<()> {
        // Update health cache
        {
            let mut cache = self.health_cache.write().await;
            cache.insert(health_check.server_id, health_check.clone());
        }

        // Update server's last health check timestamp
        if let Err(e) = self
            .registry
            .update_health_check(&health_check.server_id)
            .await
        {
            warn!(
                server_id = %health_check.server_id,
                error = %e,
                "Failed to update server health check timestamp"
            );
        }

        // Determine if server status should be updated
        let current_server = match self.registry.get(&health_check.server_id).await {
            Some(server) => server,
            None => {
                warn!(
                    server_id = %health_check.server_id,
                    "Server not found in registry during health check processing"
                );
                return Ok(());
            }
        };

        let new_status = self
            .determine_server_status(&current_server, &health_check)
            .await?;

        if current_server.status != new_status {
            info!(
                server_id = %health_check.server_id,
                old_status = ?current_server.status,
                new_status = ?new_status,
                health_status = ?health_check.status,
                "Server status changed based on health check"
            );

            if let Err(e) = self
                .registry
                .update_status(&health_check.server_id, new_status)
                .await
            {
                error!(
                    server_id = %health_check.server_id,
                    error = %e,
                    "Failed to update server status"
                );
            }

            // Attempt recovery if enabled and server is unhealthy
            if self.config.auto_recovery && new_status == ServerStatus::Failed {
                if let Err(e) = self.attempt_server_recovery(&health_check.server_id).await {
                    error!(
                        server_id = %health_check.server_id,
                        error = %e,
                        "Server recovery attempt failed"
                    );
                }
            }
        }

        Ok(())
    }

    /// Determine new server status based on health check result
    async fn determine_server_status(
        &self,
        server: &ServerInfo,
        health_check: &HealthCheck,
    ) -> Result<ServerStatus> {
        // Get current health state or create new one
        let mut state = self.get_or_create_health_state(server.id).await;

        match health_check.status {
            HealthStatus::Healthy => {
                state.consecutive_successes += 1;
                state.consecutive_failures = 0;
                state.last_success = Some(health_check.timestamp);

                // If we have enough consecutive successes, mark as healthy
                if state.consecutive_successes >= self.config.success_threshold {
                    state.status = HealthStatus::Healthy;
                    return Ok(ServerStatus::Running);
                }
            }
            HealthStatus::Unhealthy | HealthStatus::Degraded => {
                state.consecutive_failures += 1;
                state.consecutive_successes = 0;
                state.last_failure = Some(health_check.timestamp);

                // If we have enough consecutive failures, mark as unhealthy
                if state.consecutive_failures >= self.config.failure_threshold {
                    state.status = HealthStatus::Unhealthy;
                    return Ok(ServerStatus::Unhealthy);
                }
            }
            HealthStatus::Unknown => {
                // Unknown status doesn't change consecutive counters
                state.last_failure = Some(health_check.timestamp);
            }
        }

        state.last_check = Some(health_check.timestamp);

        // Return current status if thresholds haven't been met
        Ok(server.status)
    }

    /// Get or create health state for a server
    async fn get_or_create_health_state(&self, _server_id: Uuid) -> ServerHealthState {
        // This would typically be stored in a persistent store or cache
        // For now, we'll create a new state each time
        ServerHealthState {
            status: HealthStatus::Unknown,
            consecutive_successes: 0,
            consecutive_failures: 0,
            last_check: None,
            last_success: None,
            last_failure: None,
            recovery_attempts: 0,
            next_recovery_attempt: None,
        }
    }

    /// Attempt to recover a failed server
    async fn attempt_server_recovery(&self, server_id: &Uuid) -> Result<()> {
        info!(server_id = %server_id, "Attempting server recovery");

        // Get server information
        let _server = match self.registry.get(server_id).await {
            Some(server) => server,
            None => {
                return Err(McpError::ServerManagement(
                    "Server not found for recovery".to_string(),
                ));
            }
        };

        // Check if we've exceeded maximum recovery attempts
        let mut state = self.get_or_create_health_state(*server_id).await;
        if state.recovery_attempts >= self.config.max_recovery_attempts {
            warn!(
                server_id = %server_id,
                attempts = state.recovery_attempts,
                "Maximum recovery attempts reached"
            );
            return Ok(());
        }

        // Check if we should wait before next recovery attempt
        if let Some(next_attempt) = state.next_recovery_attempt {
            if Utc::now() < next_attempt {
                debug!(
                    server_id = %server_id,
                    next_attempt = %next_attempt,
                    "Waiting for next recovery attempt"
                );
                return Ok(());
            }
        }

        // Attempt recovery by updating server status to starting
        // This would trigger the server management system to restart the server
        if let Err(e) = self
            .registry
            .update_status(server_id, ServerStatus::Starting)
            .await
        {
            error!(
                server_id = %server_id,
                error = %e,
                "Failed to update server status for recovery"
            );
            return Err(e);
        }

        // Update recovery state
        state.recovery_attempts += 1;
        let backoff_seconds = (self
            .config
            .recovery_backoff_multiplier
            .powi(state.recovery_attempts as i32)) as i64;
        state.next_recovery_attempt = Some(Utc::now() + chrono::Duration::seconds(backoff_seconds));

        info!(
            server_id = %server_id,
            attempt = state.recovery_attempts,
            next_attempt = ?state.next_recovery_attempt,
            "Server recovery initiated"
        );

        Ok(())
    }

    /// Update health monitoring statistics
    async fn update_stats(&self, successful_checks: u64, failed_checks: u64) -> Result<()> {
        let mut stats = self.stats.write().await;

        stats.total_checks += successful_checks + failed_checks;
        stats.successful_checks += successful_checks;
        stats.failed_checks += failed_checks;
        stats.checks_last_hour += successful_checks + failed_checks;
        stats.last_updated = Utc::now();

        // Calculate average response time from recent health checks
        let cache = self.health_cache.read().await;
        if !cache.is_empty() {
            let total_response_time: u64 = cache.values().map(|hc| hc.response_time_ms).sum();
            let count = cache.len() as u64;
            stats.avg_response_time_ms = total_response_time as f64 / count as f64;

            stats.min_response_time_ms = cache
                .values()
                .map(|hc| hc.response_time_ms)
                .min()
                .unwrap_or(0);

            stats.max_response_time_ms = cache
                .values()
                .map(|hc| hc.response_time_ms)
                .max()
                .unwrap_or(0);
        }

        Ok(())
    }

    /// Get health check result for a server
    pub async fn get_server_health(&self, server_id: &Uuid) -> Option<HealthCheck> {
        let cache = self.health_cache.read().await;
        cache.get(server_id).cloned()
    }

    /// Get health status for all servers
    pub async fn get_all_health_status(&self) -> HashMap<Uuid, HealthCheck> {
        let cache = self.health_cache.read().await;
        cache.clone()
    }

    /// Get health monitoring statistics
    pub async fn get_statistics(&self) -> HealthStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Manually trigger health check for a specific server
    pub async fn check_server(&self, server_id: &Uuid) -> Result<HealthCheck> {
        let server = self
            .registry
            .get(server_id)
            .await
            .ok_or_else(|| McpError::ServerManagement("Server not found".to_string()))?;

        let health_check =
            Self::check_server_health(self.client.clone(), server, self.config.clone()).await?;

        self.process_health_check_result(health_check.clone())
            .await?;

        Ok(health_check)
    }

    /// Get healthy servers
    pub async fn get_healthy_servers(&self) -> Vec<Uuid> {
        let cache = self.health_cache.read().await;
        cache
            .iter()
            .filter_map(|(server_id, health_check)| {
                if health_check.status == HealthStatus::Healthy {
                    Some(*server_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get unhealthy servers
    pub async fn get_unhealthy_servers(&self) -> Vec<Uuid> {
        let cache = self.health_cache.read().await;
        cache
            .iter()
            .filter_map(|(server_id, health_check)| {
                if health_check.status == HealthStatus::Unhealthy {
                    Some(*server_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Clean up old health check results
    pub async fn cleanup_old_results(&self, max_age_seconds: u64) -> Result<()> {
        let cutoff_time = Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        let mut cache = self.health_cache.write().await;

        cache.retain(|_, health_check| health_check.timestamp > cutoff_time);

        debug!(
            "Cleaned up old health check results, {} remaining",
            cache.len()
        );

        Ok(())
    }
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 30,
            check_timeout_seconds: 5,
            failure_threshold: 3,
            success_threshold: 2,
            detailed_metrics: true,
            endpoints: vec!["/health".to_string(), "/status".to_string()],
            auto_recovery: true,
            max_recovery_attempts: 3,
            recovery_backoff_multiplier: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{ServerCapabilities, ServerConfig},
        registry::{RegistryConfig, ServerRegistry},
    };
    use std::collections::HashMap;

    fn create_test_server() -> ServerInfo {
        ServerInfo::new(
            "test-server".to_string(),
            "1.0.0".to_string(),
            "test".to_string(),
            ServerConfig {
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
    async fn test_health_monitor_creation() {
        let registry = Arc::new(ServerRegistry::new(RegistryConfig::default()));
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(registry, config);

        assert!(monitor.health_cache.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_creation() {
        let server_id = Uuid::new_v4();
        let health_check = HealthCheck::new(server_id, HealthStatus::Healthy, 100);

        assert_eq!(health_check.server_id, server_id);
        assert_eq!(health_check.status, HealthStatus::Healthy);
        assert_eq!(health_check.response_time_ms, 100);
    }

    #[tokio::test]
    async fn test_failed_health_check_creation() {
        let server_id = Uuid::new_v4();
        let error_message = "Connection refused".to_string();
        let health_check = HealthCheck::failed(server_id, error_message.clone());

        assert_eq!(health_check.server_id, server_id);
        assert_eq!(health_check.status, HealthStatus::Unhealthy);
        assert_eq!(health_check.error, Some(error_message));
    }

    #[tokio::test]
    async fn test_health_stats_update() {
        let registry = Arc::new(ServerRegistry::new(RegistryConfig::default()));
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(registry, config);

        monitor.update_stats(5, 2).await.unwrap();

        let stats = monitor.get_statistics().await;
        assert_eq!(stats.total_checks, 7);
        assert_eq!(stats.successful_checks, 5);
        assert_eq!(stats.failed_checks, 2);
    }
}
