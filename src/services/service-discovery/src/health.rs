//! Health Monitoring Module
//!
//! Provides active health checking capabilities for registered services.
//! Supports HTTP, TCP, gRPC, and script-based health checks with configurable
//! intervals, timeouts, and failure thresholds.

use crate::config::ServiceDiscoveryConfig;
use crate::models::{
    HealthCheckConfig, HealthCheckResult, HealthCheckTypeConfig, HealthStatus,
    ServiceRegistration, ServiceStatus,
};
use crate::registry::ServiceRegistry;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Health monitor trait for dependency injection
#[async_trait]
pub trait HealthMonitor: Send + Sync {
    /// Start health monitoring for all registered services
    async fn start_monitoring(&self) -> Result<()>;

    /// Stop health monitoring
    async fn stop_monitoring(&self) -> Result<()>;

    /// Add a service to health monitoring
    async fn monitor_service(&self, service: ServiceRegistration) -> Result<()>;

    /// Remove a service from health monitoring
    async fn remove_service(&self, service_id: Uuid) -> Result<()>;

    /// Perform immediate health check for a service
    async fn check_service_health(&self, service_id: Uuid) -> Result<HealthCheckResult>;

    /// Get health check statistics
    async fn get_health_stats(&self) -> Result<HealthMonitoringStats>;
}

/// Health monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitoringStats {
    /// Total services being monitored
    pub total_services: u64,

    /// Number of healthy services
    pub healthy_services: u64,

    /// Number of unhealthy services
    pub unhealthy_services: u64,

    /// Total health checks performed
    pub total_health_checks: u64,

    /// Average health check response time in milliseconds
    pub avg_response_time_ms: f64,

    /// Health check error rate (0.0 to 1.0)
    pub error_rate: f64,

    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Health check scheduler for managing check intervals
struct HealthCheckScheduler {
    /// Service ID
    service_id: Uuid,

    /// Health check configuration
    config: HealthCheckConfig,

    /// Current failure count
    failure_count: u32,

    /// Current success count
    success_count: u32,

    /// Last check result
    last_result: Option<HealthCheckResult>,

    /// Next scheduled check time
    next_check_at: Instant,
}

impl HealthCheckScheduler {
    /// Create a new scheduler for a service
    fn new(service_id: Uuid, config: HealthCheckConfig) -> Self {
        Self {
            service_id,
            config,
            failure_count: 0,
            success_count: 0,
            last_result: None,
            next_check_at: Instant::now(),
        }
    }

    /// Check if it's time for the next health check
    fn is_due(&self) -> bool {
        Instant::now() >= self.next_check_at
    }

    /// Update scheduler with health check result
    fn update_with_result(&mut self, result: &HealthCheckResult) {
        self.last_result = Some(result.clone());

        match result.status {
            HealthStatus::Healthy => {
                self.success_count += 1;
                self.failure_count = 0;
            }
            HealthStatus::Unhealthy | HealthStatus::Timeout => {
                self.failure_count += 1;
                self.success_count = 0;
            }
            HealthStatus::Unknown => {
                // Don't change counts for unknown status
            }
        }

        // Schedule next check
        self.next_check_at = Instant::now() + Duration::from_secs(self.config.interval as u64);
    }

    /// Determine if service should be marked as healthy
    fn should_mark_healthy(&self) -> bool {
        self.success_count >= self.config.success_threshold
    }

    /// Determine if service should be marked as unhealthy
    fn should_mark_unhealthy(&self) -> bool {
        self.failure_count >= self.config.failure_threshold
    }
}

/// Health monitor implementation
pub struct HealthMonitorImpl {
    /// Configuration
    config: Arc<ServiceDiscoveryConfig>,

    /// Service registry for updating service status
    registry: Arc<dyn ServiceRegistry>,

    /// HTTP client for health checks
    http_client: Client,

    /// Services being monitored
    monitored_services: Arc<DashMap<Uuid, ServiceRegistration>>,

    /// Health check schedulers
    schedulers: Arc<DashMap<Uuid, RwLock<HealthCheckScheduler>>>,

    /// Semaphore to limit concurrent health checks
    check_semaphore: Arc<Semaphore>,

    /// Monitoring task handles
    task_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,

    /// Health monitoring statistics
    stats: Arc<RwLock<HealthMonitoringStats>>,

    /// Running state
    is_running: Arc<RwLock<bool>>,
}

impl HealthMonitorImpl {
    /// Create a new health monitor instance
    pub fn new(config: Arc<ServiceDiscoveryConfig>, registry: Arc<dyn ServiceRegistry>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(
                config.registry.health_checks.timeout as u64,
            ))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            registry,
            http_client,
            monitored_services: Arc::new(DashMap::new()),
            schedulers: Arc::new(DashMap::new()),
            check_semaphore: Arc::new(Semaphore::new(100)), // Limit concurrent checks
            task_handles: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HealthMonitoringStats {
                total_services: 0,
                healthy_services: 0,
                unhealthy_services: 0,
                total_health_checks: 0,
                avg_response_time_ms: 0.0,
                error_rate: 0.0,
                last_updated: Utc::now(),
            })),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Main monitoring loop
    async fn monitoring_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(1)); // Check every second

        loop {
            interval.tick().await;

            if !*self.is_running.read().await {
                break;
            }

            // Check which services need health checks
            let services_to_check: Vec<Uuid> = self
                .schedulers
                .iter()
                .filter_map(|entry| {
                    let service_id = *entry.key();
                    let scheduler = entry.value();

                    // Use try_read to avoid blocking
                    if let Ok(scheduler_guard) = scheduler.try_read() {
                        if scheduler_guard.is_due() {
                            Some(service_id)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            // Perform health checks concurrently
            if !services_to_check.is_empty() {
                let check_tasks: Vec<_> = services_to_check
                    .into_iter()
                    .map(|service_id| {
                        let monitor = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = monitor.perform_health_check(service_id).await {
                                error!("Health check failed for service {}: {}", service_id, e);
                            }
                        })
                    })
                    .collect();

                // Wait for all checks to complete (with timeout)
                let _ = timeout(
                    Duration::from_secs(30),
                    futures::future::join_all(check_tasks),
                )
                .await;
            }
        }

        info!("Health monitoring loop stopped");
        Ok(())
    }

    /// Perform health check for a specific service
    async fn perform_health_check(&self, service_id: Uuid) -> Result<()> {
        // Acquire semaphore permit
        let _permit = self.check_semaphore.acquire().await?;

        // Get service and scheduler
        let service = self
            .monitored_services
            .get(&service_id)
            .ok_or_else(|| anyhow::anyhow!("Service not found: {}", service_id))?
            .clone();

        let scheduler_entry = self
            .schedulers
            .get(&service_id)
            .ok_or_else(|| anyhow::anyhow!("Scheduler not found: {}", service_id))?;

        let health_check_config = service
            .health_check
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No health check configuration for service"))?;

        // Perform the actual health check
        let result = self
            .execute_health_check(&service, &health_check_config)
            .await;

        // Update scheduler
        let mut scheduler = scheduler_entry.write().await;
        scheduler.update_with_result(&result);

        // Record result in registry
        if let Err(e) = self.registry.record_health_check(result.clone()).await {
            error!("Failed to record health check result: {}", e);
        }

        // Update service status if thresholds are met
        if scheduler.should_mark_unhealthy() && service.status != ServiceStatus::Unhealthy {
            if let Err(e) = self
                .registry
                .update_service(
                    service_id,
                    crate::models::UpdateServiceRequest {
                        status: Some(ServiceStatus::Unhealthy),
                        weight: None,
                        metadata: None,
                        health_check: None,
                        circuit_breaker: None,
                    },
                )
                .await
            {
                error!("Failed to update service status to unhealthy: {}", e);
            } else {
                warn!("Marked service {} as unhealthy", service_id);
            }
        } else if scheduler.should_mark_healthy() && service.status != ServiceStatus::Healthy {
            if let Err(e) = self
                .registry
                .update_service(
                    service_id,
                    crate::models::UpdateServiceRequest {
                        status: Some(ServiceStatus::Healthy),
                        weight: None,
                        metadata: None,
                        health_check: None,
                        circuit_breaker: None,
                    },
                )
                .await
            {
                error!("Failed to update service status to healthy: {}", e);
            } else {
                info!("Marked service {} as healthy", service_id);
            }
        }

        // Update statistics
        self.update_stats(&result).await;

        debug!(
            "Health check completed for service {}: {:?}",
            service_id, result.status
        );

        Ok(())
    }

    /// Execute the actual health check based on type
    async fn execute_health_check(
        &self,
        service: &ServiceRegistration,
        config: &HealthCheckConfig,
    ) -> HealthCheckResult {
        let start_time = Instant::now();
        let check_timeout = Duration::from_secs(config.timeout as u64);

        let (status, error_message, response_time_ms) = match timeout(
            check_timeout,
            self.perform_check_by_type(service, &config.config),
        )
        .await
        {
            Ok(Ok((status, error_msg))) => {
                let elapsed = start_time.elapsed();
                (status, error_msg, Some(elapsed.as_millis() as u64))
            }
            Ok(Err(e)) => (HealthStatus::Unhealthy, Some(e.to_string()), None),
            Err(_) => (
                HealthStatus::Timeout,
                Some("Health check timed out".to_string()),
                None,
            ),
        };

        HealthCheckResult {
            service_id: service.id,
            status,
            response_time_ms,
            error_message,
            timestamp: Utc::now(),
            details: HashMap::new(),
        }
    }

    /// Perform health check based on check type
    async fn perform_check_by_type(
        &self,
        service: &ServiceRegistration,
        config: &HealthCheckTypeConfig,
    ) -> Result<(HealthStatus, Option<String>)> {
        match config {
            HealthCheckTypeConfig::Http {
                path,
                method,
                headers,
                expected_status,
                expected_body,
            } => {
                self.perform_http_check(
                    service,
                    path,
                    method,
                    headers,
                    *expected_status,
                    expected_body,
                )
                .await
            }
            HealthCheckTypeConfig::Tcp {} => self.perform_tcp_check(service).await,
            HealthCheckTypeConfig::Grpc { service_name } => {
                self.perform_grpc_check(service, service_name).await
            }
            HealthCheckTypeConfig::Script {
                command,
                args,
                working_dir,
            } => {
                self.perform_script_check(command, args, working_dir.as_deref())
                    .await
            }
        }
    }

    /// Perform HTTP health check
    async fn perform_http_check(
        &self,
        service: &ServiceRegistration,
        path: &str,
        method: &str,
        headers: &HashMap<String, String>,
        expected_status: u16,
        expected_body: &Option<String>,
    ) -> Result<(HealthStatus, Option<String>)> {
        let url = format!(
            "{}://{}:{}{}",
            match service.protocol {
                crate::models::ServiceProtocol::Https => "https",
                _ => "http",
            },
            service.address,
            service.port,
            path
        );

        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "HEAD" => self.http_client.head(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request.send().await?;
        let status_code = response.status().as_u16();

        if status_code != expected_status {
            return Ok((
                HealthStatus::Unhealthy,
                Some(format!(
                    "Expected status {}, got {}",
                    expected_status, status_code
                )),
            ));
        }

        // Check response body if specified
        if let Some(expected) = expected_body {
            let body = response.text().await?;
            if !body.contains(expected) {
                return Ok((
                    HealthStatus::Unhealthy,
                    Some(format!(
                        "Response body does not contain expected text: {}",
                        expected
                    )),
                ));
            }
        }

        Ok((HealthStatus::Healthy, None))
    }

    /// Perform TCP health check
    async fn perform_tcp_check(
        &self,
        service: &ServiceRegistration,
    ) -> Result<(HealthStatus, Option<String>)> {
        let address = format!("{}:{}", service.address, service.port);

        match TcpStream::connect(&address).await {
            Ok(_) => Ok((HealthStatus::Healthy, None)),
            Err(e) => Ok((
                HealthStatus::Unhealthy,
                Some(format!("TCP connection failed: {}", e)),
            )),
        }
    }

    /// Perform gRPC health check
    async fn perform_grpc_check(
        &self,
        service: &ServiceRegistration,
        _service_name: &str,
    ) -> Result<(HealthStatus, Option<String>)> {
        // This is a simplified implementation
        // In a real implementation, you would use a gRPC client to call the health service

        // For now, just try to establish a TCP connection to the gRPC port
        let address = format!("{}:{}", service.address, service.port);

        match TcpStream::connect(&address).await {
            Ok(_) => {
                // In a real implementation, you would send a gRPC health check request here
                // For example: grpc_health_v1::health_client::HealthClient::check()
                Ok((HealthStatus::Healthy, None))
            }
            Err(e) => Ok((
                HealthStatus::Unhealthy,
                Some(format!("gRPC connection failed: {}", e)),
            )),
        }
    }

    /// Perform script-based health check
    async fn perform_script_check(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
    ) -> Result<(HealthStatus, Option<String>)> {
        let mut cmd = Command::new(command);
        cmd.args(args);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().await?;

        if output.status.success() {
            Ok((HealthStatus::Healthy, None))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok((
                HealthStatus::Unhealthy,
                Some(format!("Script failed: {}", stderr)),
            ))
        }
    }

    /// Update health monitoring statistics
    async fn update_stats(&self, result: &HealthCheckResult) {
        let mut stats = self.stats.write().await;

        stats.total_health_checks += 1;

        // Update response time average
        if let Some(response_time) = result.response_time_ms {
            let total_time = stats.avg_response_time_ms * (stats.total_health_checks - 1) as f64;
            stats.avg_response_time_ms =
                (total_time + response_time as f64) / stats.total_health_checks as f64;
        }

        // Update error rate
        let error_count = match result.status {
            HealthStatus::Healthy => 0,
            _ => 1,
        };
        let total_errors = stats.error_rate * (stats.total_health_checks - 1) as f64;
        stats.error_rate = (total_errors + error_count as f64) / stats.total_health_checks as f64;

        stats.last_updated = Utc::now();

        // Update healthy/unhealthy counts
        stats.healthy_services = self
            .monitored_services
            .iter()
            .filter(|entry| entry.value().status == ServiceStatus::Healthy)
            .count() as u64;

        stats.unhealthy_services = self
            .monitored_services
            .iter()
            .filter(|entry| entry.value().status != ServiceStatus::Healthy)
            .count() as u64;

        stats.total_services = self.monitored_services.len() as u64;
    }
}

#[async_trait]
impl HealthMonitor for HealthMonitorImpl {
    async fn start_monitoring(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(anyhow::anyhow!("Health monitoring is already running"));
        }

        *is_running = true;
        drop(is_running);

        // Start the main monitoring loop
        let monitor = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = monitor.monitoring_loop().await {
                error!("Health monitoring loop failed: {}", e);
            }
        });

        self.task_handles.write().await.push(handle);

        info!("Health monitoring started");
        Ok(())
    }

    async fn stop_monitoring(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        *is_running = false;
        drop(is_running);

        // Cancel all running tasks
        let mut handles = self.task_handles.write().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        info!("Health monitoring stopped");
        Ok(())
    }

    async fn monitor_service(&self, service: ServiceRegistration) -> Result<()> {
        if let Some(health_config) = &service.health_check {
            let scheduler = HealthCheckScheduler::new(service.id, health_config.clone());

            self.schedulers.insert(service.id, RwLock::new(scheduler));

            self.monitored_services.insert(service.id, service.clone());

            debug!("Added service {} to health monitoring", service.id);
        } else {
            debug!(
                "Service {} has no health check configuration, skipping monitoring",
                service.id
            );
        }

        Ok(())
    }

    async fn remove_service(&self, service_id: Uuid) -> Result<()> {
        self.schedulers.remove(&service_id);
        self.monitored_services.remove(&service_id);

        debug!("Removed service {} from health monitoring", service_id);
        Ok(())
    }

    async fn check_service_health(&self, service_id: Uuid) -> Result<HealthCheckResult> {
        let service = self
            .monitored_services
            .get(&service_id)
            .ok_or_else(|| anyhow::anyhow!("Service not monitored: {}", service_id))?
            .clone();

        let health_config = service
            .health_check
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No health check configuration for service"))?;

        let result = self.execute_health_check(&service, &health_config).await;

        // Record the result
        if let Err(e) = self.registry.record_health_check(result.clone()).await {
            error!("Failed to record health check result: {}", e);
        }

        Ok(result)
    }

    async fn get_health_stats(&self) -> Result<HealthMonitoringStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

impl Clone for HealthMonitorImpl {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            registry: Arc::clone(&self.registry),
            http_client: self.http_client.clone(),
            monitored_services: Arc::clone(&self.monitored_services),
            schedulers: Arc::clone(&self.schedulers),
            check_semaphore: Arc::clone(&self.check_semaphore),
            task_handles: Arc::clone(&self.task_handles),
            stats: Arc::clone(&self.stats),
            is_running: Arc::clone(&self.is_running),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HealthCheckType, ServiceProtocol, ServiceStatus};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_health_check_scheduler() {
        let config = HealthCheckConfig {
            check_type: HealthCheckType::Http,
            interval: 30,
            timeout: 5,
            failure_threshold: 3,
            success_threshold: 2,
            config: HealthCheckTypeConfig::Http {
                path: "/health".to_string(),
                method: "GET".to_string(),
                headers: HashMap::new(),
                expected_status: 200,
                expected_body: None,
            },
        };

        let service_id = Uuid::new_v4();
        let mut scheduler = HealthCheckScheduler::new(service_id, config);

        // Test initial state
        assert_eq!(scheduler.failure_count, 0);
        assert_eq!(scheduler.success_count, 0);
        assert!(!scheduler.should_mark_unhealthy());
        assert!(!scheduler.should_mark_healthy());

        // Test failure threshold
        for _ in 0..3 {
            let result = HealthCheckResult {
                service_id,
                status: HealthStatus::Unhealthy,
                response_time_ms: None,
                error_message: Some("Service unavailable".to_string()),
                timestamp: Utc::now(),
                details: HashMap::new(),
            };
            scheduler.update_with_result(&result);
        }

        assert!(scheduler.should_mark_unhealthy());
        assert!(!scheduler.should_mark_healthy());

        // Test recovery
        for _ in 0..2 {
            let result = HealthCheckResult {
                service_id,
                status: HealthStatus::Healthy,
                response_time_ms: Some(100),
                error_message: None,
                timestamp: Utc::now(),
                details: HashMap::new(),
            };
            scheduler.update_with_result(&result);
        }

        assert!(!scheduler.should_mark_unhealthy());
        assert!(scheduler.should_mark_healthy());
    }

    #[test]
    fn test_health_monitoring_stats() {
        let stats = HealthMonitoringStats {
            total_services: 10,
            healthy_services: 8,
            unhealthy_services: 2,
            total_health_checks: 1000,
            avg_response_time_ms: 150.5,
            error_rate: 0.1,
            last_updated: Utc::now(),
        };

        assert_eq!(stats.total_services, 10);
        assert_eq!(stats.healthy_services, 8);
        assert_eq!(stats.error_rate, 0.1);
    }
}
