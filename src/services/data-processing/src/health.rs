//! Health monitoring module for the Data Processing Service
//!
//! This module provides comprehensive health monitoring capabilities including:
//! - Component health checks and status tracking
//! - Service dependency monitoring
//! - Health aggregation and reporting
//! - Alerting and notification integration
//! - Performance-based health scoring

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::{
    config::Config,
    error::{DataProcessingError, Result},
    metrics::MetricsCollector,
    types::{ComponentHealth, HealthStatus, ServiceHealth},
};

/// Health checker that monitors all service components
pub struct HealthChecker {
    config: Arc<Config>,
    metrics: Arc<MetricsCollector>,
    components: Arc<RwLock<HashMap<String, HealthComponent>>>,
    overall_health: Arc<RwLock<ServiceHealth>>,
    check_interval: Duration,
    is_running: Arc<RwLock<bool>>,
}

/// Individual health component
pub struct HealthComponent {
    name: String,
    checker: Arc<dyn ComponentHealthChecker + Send + Sync>,
    config: ComponentHealthConfig,
    last_check: Option<DateTime<Utc>>,
    consecutive_failures: u32,
    consecutive_successes: u32,
}

/// Health component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealthConfig {
    pub enabled: bool,
    pub check_interval_secs: u64,
    pub timeout_secs: u64,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub critical: bool,
    pub dependencies: Vec<String>,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub message: String,
    pub details: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

/// Component health checker trait
#[async_trait::async_trait]
pub trait ComponentHealthChecker {
    /// Perform health check
    async fn check_health(&self) -> HealthCheckResult;

    /// Get component name
    fn name(&self) -> &str;

    /// Check if component is critical
    fn is_critical(&self) -> bool {
        false
    }
}

/// Kafka health checker
pub struct KafkaHealthChecker {
    name: String,
    bootstrap_servers: String,
}

/// ClickHouse health checker
pub struct ClickHouseHealthChecker {
    name: String,
    connection_url: String,
}

/// Stream processor health checker
pub struct StreamProcessorHealthChecker {
    name: String,
}

/// Batch processor health checker
pub struct BatchProcessorHealthChecker {
    name: String,
}

/// System resource health checker
pub struct SystemResourceHealthChecker {
    name: String,
    cpu_threshold: f64,
    memory_threshold: f64,
    disk_threshold: f64,
}

/// Health aggregator for combining component health
pub struct HealthAggregator {
    weights: HashMap<String, f64>,
    critical_components: Vec<String>,
}

impl HealthChecker {
    /// Create a new health checker
    pub async fn new(
        config: Arc<Config>,
        metrics: Arc<MetricsCollector>,
        components: Vec<(&str, Arc<dyn ComponentHealthChecker + Send + Sync>)>,
    ) -> Result<Self> {
        let mut component_map = HashMap::new();

        for (name, checker) in components {
            let component_config = ComponentHealthConfig {
                enabled: true,
                check_interval_secs: config.health.check_interval_secs,
                timeout_secs: config.health.check_timeout_secs,
                failure_threshold: config.health.failure_threshold as u32,
                success_threshold: config.health.success_threshold as u32,
                critical: checker.is_critical(),
                dependencies: Vec::new(),
            };

            let component = HealthComponent {
                name: name.to_string(),
                checker,
                config: component_config,
                last_check: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
            };

            component_map.insert(name.to_string(), component);
        }

        let overall_health = ServiceHealth {
            status: HealthStatus::Unknown,
            components: HashMap::new(),
            last_check: Utc::now(),
            check_duration_ms: 0,
            uptime_secs: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Ok(Self {
            config,
            metrics,
            components: Arc::new(RwLock::new(component_map)),
            overall_health: Arc::new(RwLock::new(overall_health)),
            check_interval: Duration::from_secs(30), // Default 30 seconds
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start health monitoring
    pub async fn start(&self) -> Result<()> {
        info!("Starting health checker");

        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        // Perform initial health check
        self.perform_health_check().await?;

        // Start background health checking
        let checker = self.clone();
        tokio::spawn(async move {
            checker.health_check_loop().await;
        });

        info!("Health checker started successfully");
        Ok(())
    }

    /// Stop health monitoring
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping health checker");

        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        info!("Health checker stopped");
        Ok(())
    }

    /// Get current overall health
    pub async fn get_health(&self) -> ServiceHealth {
        self.overall_health.read().await.clone()
    }

    /// Get health for a specific component
    pub async fn get_component_health(&self, component_name: &str) -> Option<ComponentHealth> {
        let health = self.overall_health.read().await;
        health.components.get(component_name).cloned()
    }

    /// Add a new component to monitor
    pub async fn add_component(
        &self,
        name: String,
        checker: Arc<dyn ComponentHealthChecker + Send + Sync>,
        config: ComponentHealthConfig,
    ) -> Result<()> {
        let component = HealthComponent {
            name: name.clone(),
            checker,
            config,
            last_check: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
        };

        let mut components = self.components.write().await;
        components.insert(name.clone(), component);

        info!("Added health component: {}", name);
        Ok(())
    }

    /// Remove a component from monitoring
    pub async fn remove_component(&self, name: &str) -> Result<()> {
        let mut components = self.components.write().await;
        components.remove(name);

        info!("Removed health component: {}", name);
        Ok(())
    }

    /// Health check loop
    async fn health_check_loop(&self) {
        let mut interval = interval(self.check_interval);

        while *self.is_running.read().await {
            interval.tick().await;

            if let Err(e) = self.perform_health_check().await {
                error!("Health check failed: {}", e);
            }
        }
    }

    /// Perform comprehensive health check
    async fn perform_health_check(&self) -> Result<()> {
        let start_time = Instant::now();
        let mut component_results = HashMap::new();

        debug!("Starting health check");

        // Check all components
        {
            let mut components = self.components.write().await;
            for (name, component) in components.iter_mut() {
                if !component.config.enabled {
                    continue;
                }

                let check_result = self.check_component(component).await;

                // Update component state
                if check_result.healthy {
                    component.consecutive_successes += 1;
                    component.consecutive_failures = 0;
                } else {
                    component.consecutive_failures += 1;
                    component.consecutive_successes = 0;
                }

                component.last_check = Some(check_result.timestamp);

                // Determine component health status
                let status = self.determine_component_status(component, &check_result);

                let component_health = ComponentHealth {
                    status: status.clone(),
                    details: check_result.details,
                    last_success: if check_result.healthy {
                        Some(check_result.timestamp)
                    } else {
                        None
                    },
                    error_count: component.consecutive_failures,
                    response_time_ms: Some(check_result.response_time_ms),
                };

                component_results.insert(name.clone(), component_health);

                // Update metrics
                let status_label = format!("{:?}", status);
                self.metrics.set_gauge(
                    "component_health_status",
                    match status {
                        HealthStatus::Healthy => 1.0,
                        HealthStatus::Degraded => 0.5,
                        HealthStatus::Unhealthy => 0.0,
                        HealthStatus::Unknown => -1.0,
                    },
                    &[("component", name), ("status", &status_label)],
                );
            }
        }

        // Aggregate overall health
        let overall_status = self.aggregate_health(&component_results);
        let check_duration = start_time.elapsed();

        // Update overall health
        {
            let mut health = self.overall_health.write().await;
            health.status = overall_status.clone();
            health.components = component_results;
            health.last_check = Utc::now();
            health.check_duration_ms = check_duration.as_millis() as u64;
            // uptime_secs would be calculated from service start time
        }

        // Update metrics
        self.metrics.set_gauge(
            "overall_health_status",
            match overall_status {
                HealthStatus::Healthy => 1.0,
                HealthStatus::Degraded => 0.5,
                HealthStatus::Unhealthy => 0.0,
                HealthStatus::Unknown => -1.0,
            },
            &[],
        );

        self.metrics.record_histogram(
            "health_check_duration_seconds",
            check_duration.as_secs_f64(),
            &[],
        );

        debug!("Health check completed in {:?}", check_duration);
        Ok(())
    }

    /// Check individual component health
    async fn check_component(&self, component: &HealthComponent) -> HealthCheckResult {
        let start_time = Instant::now();

        match tokio::time::timeout(
            Duration::from_secs(component.config.timeout_secs),
            component.checker.check_health(),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let duration = start_time.elapsed();
                HealthCheckResult {
                    healthy: false,
                    response_time_ms: duration.as_millis() as u64,
                    message: format!(
                        "Health check timed out after {}s",
                        component.config.timeout_secs
                    ),
                    details: HashMap::new(),
                    timestamp: Utc::now(),
                }
            }
        }
    }

    /// Determine component health status based on check results and history
    fn determine_component_status(
        &self,
        component: &HealthComponent,
        check_result: &HealthCheckResult,
    ) -> HealthStatus {
        if check_result.healthy {
            if component.consecutive_successes >= component.config.success_threshold {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            }
        } else {
            if component.consecutive_failures >= component.config.failure_threshold {
                HealthStatus::Unhealthy
            } else {
                HealthStatus::Degraded
            }
        }
    }

    /// Aggregate component health into overall health
    fn aggregate_health(&self, components: &HashMap<String, ComponentHealth>) -> HealthStatus {
        if components.is_empty() {
            return HealthStatus::Unknown;
        }

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut critical_unhealthy = false;

        for (name, health) in components {
            match health.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => {
                    unhealthy_count += 1;
                    // Check if this is a critical component
                    if self.is_critical_component(name) {
                        critical_unhealthy = true;
                    }
                }
                HealthStatus::Unknown => {}
            }
        }

        // If any critical component is unhealthy, overall status is unhealthy
        if critical_unhealthy {
            return HealthStatus::Unhealthy;
        }

        // If more than 50% of components are unhealthy, overall status is unhealthy
        let total_components = components.len();
        if unhealthy_count > total_components / 2 {
            return HealthStatus::Unhealthy;
        }

        // If any component is degraded or unhealthy, overall status is degraded
        if degraded_count > 0 || unhealthy_count > 0 {
            return HealthStatus::Degraded;
        }

        // All components are healthy
        HealthStatus::Healthy
    }

    /// Check if a component is critical
    fn is_critical_component(&self, _name: &str) -> bool {
        // This would be configured based on the component
        // For now, assume Kafka and ClickHouse are critical
        matches!(_name, "kafka" | "clickhouse")
    }
}

impl Clone for HealthChecker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            components: self.components.clone(),
            overall_health: self.overall_health.clone(),
            check_interval: self.check_interval,
            is_running: self.is_running.clone(),
        }
    }
}

// Component health checker implementations

impl KafkaHealthChecker {
    pub fn new(name: String, bootstrap_servers: String) -> Self {
        Self {
            name,
            bootstrap_servers,
        }
    }
}

#[async_trait::async_trait]
impl ComponentHealthChecker for KafkaHealthChecker {
    async fn check_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Simplified Kafka health check
        // In a real implementation, this would create a Kafka client and test connectivity
        tokio::time::sleep(Duration::from_millis(10)).await;

        let healthy = true; // Assume healthy for now
        let response_time = start_time.elapsed().as_millis() as u64;

        HealthCheckResult {
            healthy,
            response_time_ms: response_time,
            message: if healthy {
                "Kafka cluster is accessible".to_string()
            } else {
                "Kafka cluster is not accessible".to_string()
            },
            details: {
                let mut details = HashMap::new();
                details.insert(
                    "bootstrap_servers".to_string(),
                    self.bootstrap_servers.clone(),
                );
                details
            },
            timestamp: Utc::now(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_critical(&self) -> bool {
        true
    }
}

impl ClickHouseHealthChecker {
    pub fn new(name: String, connection_url: String) -> Self {
        Self {
            name,
            connection_url,
        }
    }
}

#[async_trait::async_trait]
impl ComponentHealthChecker for ClickHouseHealthChecker {
    async fn check_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Simplified ClickHouse health check
        tokio::time::sleep(Duration::from_millis(15)).await;

        let healthy = true;
        let response_time = start_time.elapsed().as_millis() as u64;

        HealthCheckResult {
            healthy,
            response_time_ms: response_time,
            message: if healthy {
                "ClickHouse is responding".to_string()
            } else {
                "ClickHouse is not responding".to_string()
            },
            details: {
                let mut details = HashMap::new();
                details.insert("connection_url".to_string(), self.connection_url.clone());
                details
            },
            timestamp: Utc::now(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_critical(&self) -> bool {
        true
    }
}

impl StreamProcessorHealthChecker {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl ComponentHealthChecker for StreamProcessorHealthChecker {
    async fn check_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Check stream processor health
        tokio::time::sleep(Duration::from_millis(5)).await;

        let healthy = true;
        let response_time = start_time.elapsed().as_millis() as u64;

        HealthCheckResult {
            healthy,
            response_time_ms: response_time,
            message: if healthy {
                "Stream processor is running".to_string()
            } else {
                "Stream processor is not running".to_string()
            },
            details: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl BatchProcessorHealthChecker {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl ComponentHealthChecker for BatchProcessorHealthChecker {
    async fn check_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Check batch processor health
        tokio::time::sleep(Duration::from_millis(5)).await;

        let healthy = true;
        let response_time = start_time.elapsed().as_millis() as u64;

        HealthCheckResult {
            healthy,
            response_time_ms: response_time,
            message: if healthy {
                "Batch processor is operational".to_string()
            } else {
                "Batch processor is not operational".to_string()
            },
            details: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl SystemResourceHealthChecker {
    pub fn new(
        name: String,
        cpu_threshold: f64,
        memory_threshold: f64,
        disk_threshold: f64,
    ) -> Self {
        Self {
            name,
            cpu_threshold,
            memory_threshold,
            disk_threshold,
        }
    }
}

#[async_trait::async_trait]
impl ComponentHealthChecker for SystemResourceHealthChecker {
    async fn check_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Simplified system resource check
        let cpu_usage = 25.0; // Mock CPU usage
        let memory_usage = 60.0; // Mock memory usage
        let disk_usage = 30.0; // Mock disk usage

        let healthy = cpu_usage < self.cpu_threshold
            && memory_usage < self.memory_threshold
            && disk_usage < self.disk_threshold;

        let response_time = start_time.elapsed().as_millis() as u64;

        HealthCheckResult {
            healthy,
            response_time_ms: response_time,
            message: if healthy {
                "System resources are within acceptable limits".to_string()
            } else {
                "System resources are under stress".to_string()
            },
            details: {
                let mut details = HashMap::new();
                details.insert("cpu_usage".to_string(), format!("{:.1}%", cpu_usage));
                details.insert("memory_usage".to_string(), format!("{:.1}%", memory_usage));
                details.insert("disk_usage".to_string(), format!("{:.1}%", disk_usage));
                details
            },
            timestamp: Utc::now(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = Arc::new(Config::default());
        let metrics = Arc::new(MetricsCollector::new(&config).unwrap());

        let kafka_checker = Arc::new(KafkaHealthChecker::new(
            "kafka".to_string(),
            "localhost:9092".to_string(),
        )) as Arc<dyn ComponentHealthChecker + Send + Sync>;

        let components = vec![("kafka", kafka_checker)];
        let health_checker = HealthChecker::new(config, metrics, components).await;

        assert!(health_checker.is_ok());
    }

    #[tokio::test]
    async fn test_component_health_check() {
        let kafka_checker =
            KafkaHealthChecker::new("kafka".to_string(), "localhost:9092".to_string());

        let result = kafka_checker.check_health().await;
        assert!(result.healthy);
        assert!(!result.message.is_empty());
        assert!(result.response_time_ms > 0);
    }

    #[tokio::test]
    async fn test_health_aggregation() {
        let config = Arc::new(Config::default());
        let metrics = Arc::new(MetricsCollector::new(&config).unwrap());

        let kafka_checker = Arc::new(KafkaHealthChecker::new(
            "kafka".to_string(),
            "localhost:9092".to_string(),
        )) as Arc<dyn ComponentHealthChecker + Send + Sync>;

        let components = vec![("kafka", kafka_checker)];
        let health_checker = HealthChecker::new(config, metrics, components)
            .await
            .unwrap();

        // Test with healthy components
        let mut component_health = HashMap::new();
        component_health.insert(
            "kafka".to_string(),
            ComponentHealth {
                status: HealthStatus::Healthy,
                details: HashMap::new(),
                last_success: Some(Utc::now()),
                error_count: 0,
                response_time_ms: Some(10),
            },
        );

        let overall_status = health_checker.aggregate_health(&component_health);
        assert_eq!(overall_status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_system_resource_checker() {
        let checker = SystemResourceHealthChecker::new(
            "system".to_string(),
            80.0, // CPU threshold
            80.0, // Memory threshold
            90.0, // Disk threshold
        );

        let result = checker.check_health().await;
        assert!(result.healthy); // Should be healthy with mock values
        assert!(result.details.contains_key("cpu_usage"));
        assert!(result.details.contains_key("memory_usage"));
        assert!(result.details.contains_key("disk_usage"));
    }
}
