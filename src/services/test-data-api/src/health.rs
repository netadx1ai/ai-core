// AI-CORE Test Data API Health Service
// Comprehensive health monitoring and status reporting
// Backend Agent Implementation - T2.2

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::database::DatabaseManager;
use crate::models::*;

// ============================================================================
// Health Service - Service health monitoring and reporting
// ============================================================================

pub struct HealthService {
    database: Arc<DatabaseManager>,
    service_start_time: DateTime<Utc>,
    health_checks: HashMap<String, HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthCheck {
    name: String,
    description: String,
    check_type: HealthCheckType,
    timeout_seconds: u64,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum HealthCheckType {
    Database,
    ExternalService,
    FileSystem,
    Memory,
    Disk,
    Network,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedHealthStatus {
    pub service_name: String,
    pub version: String,
    pub status: ServiceHealthStatus,
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: i64,
    pub database_connections: DatabaseHealthStatus,
    pub external_services: Vec<ExternalServiceHealth>,
    pub metrics: ServiceMetrics,
    pub system_info: SystemInfo,
    pub health_checks: Vec<HealthCheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub platform: String,
    pub architecture: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub disk_usage_percent: f64,
    pub load_average: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: ServiceHealthStatus,
    pub message: String,
    pub duration_ms: u64,
    pub details: Option<serde_json::Value>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_failure: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
}

impl HealthService {
    pub async fn new(database: Arc<DatabaseManager>) -> Result<Self> {
        info!("Initializing HealthService");

        let mut health_checks = HashMap::new();

        // Initialize default health checks
        health_checks.insert("postgresql".to_string(), HealthCheck {
            name: "PostgreSQL Database".to_string(),
            description: "Primary database connection health".to_string(),
            check_type: HealthCheckType::Database,
            timeout_seconds: 5,
            enabled: true,
        });

        health_checks.insert("mongodb".to_string(), HealthCheck {
            name: "MongoDB Connection".to_string(),
            description: "Document database connection health".to_string(),
            check_type: HealthCheckType::Database,
            timeout_seconds: 5,
            enabled: true,
        });

        health_checks.insert("redis".to_string(), HealthCheck {
            name: "Redis Cache".to_string(),
            description: "Cache and session storage health".to_string(),
            check_type: HealthCheckType::Database,
            timeout_seconds: 3,
            enabled: true,
        });

        health_checks.insert("clickhouse".to_string(), HealthCheck {
            name: "ClickHouse Analytics".to_string(),
            description: "Analytics database health".to_string(),
            check_type: HealthCheckType::Database,
            timeout_seconds: 5,
            enabled: true,
        });

        health_checks.insert("memory".to_string(), HealthCheck {
            name: "Memory Usage".to_string(),
            description: "System memory utilization check".to_string(),
            check_type: HealthCheckType::Memory,
            timeout_seconds: 1,
            enabled: true,
        });

        health_checks.insert("disk".to_string(), HealthCheck {
            name: "Disk Usage".to_string(),
            description: "Disk space utilization check".to_string(),
            check_type: HealthCheckType::Disk,
            timeout_seconds: 2,
            enabled: true,
        });

        let service = Self {
            database,
            service_start_time: Utc::now(),
            health_checks,
        };

        info!("HealthService initialized successfully");
        Ok(service)
    }

    // ========================================================================
    // Public Health Check Methods
    // ========================================================================

    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        debug!("Performing basic health check");

        let timestamp = Utc::now();
        let uptime_seconds = (timestamp - self.service_start_time).num_seconds();

        // Get database health
        let database_health = self.database.health_check().await?;

        // Get external services health
        let external_services = self.check_external_services().await;

        // Get service metrics
        let metrics = self.get_service_metrics().await;

        // Determine overall status
        let overall_status = self.determine_overall_status(&database_health, &external_services);

        Ok(HealthStatus {
            service_name: "test-data-api".to_string(),
            version: "1.0.0".to_string(),
            status: overall_status,
            timestamp,
            uptime_seconds,
            database_connections: database_health,
            external_services,
            metrics,
        })
    }

    pub async fn get_detailed_health_status(&self) -> Result<DetailedHealthStatus> {
        debug!("Performing detailed health check");

        let timestamp = Utc::now();
        let uptime_seconds = (timestamp - self.service_start_time).num_seconds();

        // Get all health check results
        let health_check_results = self.run_all_health_checks().await;

        // Get database health
        let database_health = self.database.health_check().await?;

        // Get external services health
        let external_services = self.check_external_services().await;

        // Get service metrics
        let metrics = self.get_service_metrics().await;

        // Get system information
        let system_info = self.get_system_info().await;

        // Determine overall status
        let overall_status = self.determine_detailed_status(
            &database_health,
            &external_services,
            &health_check_results,
        );

        Ok(DetailedHealthStatus {
            service_name: "test-data-api".to_string(),
            version: "1.0.0".to_string(),
            status: overall_status,
            timestamp,
            uptime_seconds,
            database_connections: database_health,
            external_services,
            metrics,
            system_info,
            health_checks: health_check_results,
        })
    }

    pub async fn check_readiness(&self) -> Result<bool> {
        debug!("Checking service readiness");

        // Check critical dependencies
        let database_health = self.database.health_check().await?;

        // Service is ready if PostgreSQL (primary database) is healthy
        let is_ready = matches!(database_health.postgresql.status, ServiceHealthStatus::Healthy);

        if is_ready {
            debug!("Service is ready");
        } else {
            warn!("Service is not ready - database issues detected");
        }

        Ok(is_ready)
    }

    pub async fn check_liveness(&self) -> Result<bool> {
        debug!("Checking service liveness");

        // Basic liveness check - can we respond to requests?
        let start_time = std::time::Instant::now();

        // Perform minimal health check
        let _ = self.get_service_metrics().await;

        let duration = start_time.elapsed();

        // Service is alive if it can respond within reasonable time
        let is_alive = duration < Duration::from_secs(5);

        if is_alive {
            debug!("Service is alive (responded in {:?})", duration);
        } else {
            error!("Service liveness check failed (took {:?})", duration);
        }

        Ok(is_alive)
    }

    // ========================================================================
    // Health Check Implementation
    // ========================================================================

    async fn run_all_health_checks(&self) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();

        for (check_name, health_check) in &self.health_checks {
            if !health_check.enabled {
                continue;
            }

            let result = self.run_health_check(check_name, health_check).await;
            results.push(result);
        }

        results
    }

    async fn run_health_check(&self, check_name: &str, health_check: &HealthCheck) -> HealthCheckResult {
        let start_time = std::time::Instant::now();

        let check_timeout = Duration::from_secs(health_check.timeout_seconds);

        let check_result = timeout(check_timeout, async {
            match health_check.check_type {
                HealthCheckType::Database => self.check_database_health(check_name).await,
                HealthCheckType::ExternalService => self.check_external_service_health(check_name).await,
                HealthCheckType::FileSystem => self.check_filesystem_health().await,
                HealthCheckType::Memory => self.check_memory_health().await,
                HealthCheckType::Disk => self.check_disk_health().await,
                HealthCheckType::Network => self.check_network_health().await,
                HealthCheckType::Custom => self.check_custom_health(check_name).await,
            }
        }).await;

        let duration = start_time.elapsed();

        match check_result {
            Ok(Ok((status, message, details))) => HealthCheckResult {
                name: health_check.name.clone(),
                status,
                message,
                duration_ms: duration.as_millis() as u64,
                details,
                last_success: if matches!(status, ServiceHealthStatus::Healthy) {
                    Some(Utc::now())
                } else {
                    None
                },
                last_failure: if !matches!(status, ServiceHealthStatus::Healthy) {
                    Some(Utc::now())
                } else {
                    None
                },
                consecutive_failures: 0, // Would track this in production
            },
            Ok(Err(e)) => HealthCheckResult {
                name: health_check.name.clone(),
                status: ServiceHealthStatus::Unhealthy,
                message: format!("Health check failed: {}", e),
                duration_ms: duration.as_millis() as u64,
                details: None,
                last_success: None,
                last_failure: Some(Utc::now()),
                consecutive_failures: 1,
            },
            Err(_) => HealthCheckResult {
                name: health_check.name.clone(),
                status: ServiceHealthStatus::Unhealthy,
                message: format!("Health check timed out after {}s", health_check.timeout_seconds),
                duration_ms: duration.as_millis() as u64,
                details: None,
                last_success: None,
                last_failure: Some(Utc::now()),
                consecutive_failures: 1,
            },
        }
    }

    async fn check_database_health(&self, db_name: &str) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        match db_name {
            "postgresql" => {
                let health = self.database.health_check().await?;
                Ok((
                    health.postgresql.status.clone(),
                    format!("PostgreSQL: {} connections active", health.postgresql.connection_count),
                    Some(serde_json::json!({
                        "connection_count": health.postgresql.connection_count,
                        "max_connections": health.postgresql.max_connections,
                        "response_time_ms": health.postgresql.response_time_ms
                    }))
                ))
            },
            "mongodb" => {
                let health = self.database.health_check().await?;
                Ok((
                    health.mongodb.status.clone(),
                    "MongoDB connection active".to_string(),
                    Some(serde_json::json!({
                        "response_time_ms": health.mongodb.response_time_ms
                    }))
                ))
            },
            "redis" => {
                let health = self.database.health_check().await?;
                Ok((
                    health.redis.status.clone(),
                    "Redis cache accessible".to_string(),
                    Some(serde_json::json!({
                        "response_time_ms": health.redis.response_time_ms
                    }))
                ))
            },
            "clickhouse" => {
                let health = self.database.health_check().await?;
                Ok((
                    health.clickhouse.status.clone(),
                    "ClickHouse analytics available".to_string(),
                    Some(serde_json::json!({
                        "response_time_ms": health.clickhouse.response_time_ms
                    }))
                ))
            },
            _ => Err(anyhow!("Unknown database: {}", db_name))
        }
    }

    async fn check_external_service_health(&self, service_name: &str) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        // Mock external service health check
        match service_name {
            "auth_service" => Ok((
                ServiceHealthStatus::Healthy,
                "Authentication service accessible".to_string(),
                Some(serde_json::json!({"endpoint": "/auth/health"}))
            )),
            _ => Err(anyhow!("Unknown external service: {}", service_name))
        }
    }

    async fn check_filesystem_health(&self) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        // Check if we can write to temp directory
        let temp_file = std::env::temp_dir().join("health_check.tmp");

        match tokio::fs::write(&temp_file, "health_check").await {
            Ok(_) => {
                let _ = tokio::fs::remove_file(&temp_file).await;
                Ok((
                    ServiceHealthStatus::Healthy,
                    "Filesystem read/write operations working".to_string(),
                    None
                ))
            }
            Err(e) => Ok((
                ServiceHealthStatus::Unhealthy,
                format!("Filesystem check failed: {}", e),
                None
            ))
        }
    }

    async fn check_memory_health(&self) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        let system_info = self.get_system_info().await;
        let memory_usage_percent = ((system_info.total_memory_mb - system_info.available_memory_mb) as f64
            / system_info.total_memory_mb as f64) * 100.0;

        let status = if memory_usage_percent < 80.0 {
            ServiceHealthStatus::Healthy
        } else if memory_usage_percent < 90.0 {
            ServiceHealthStatus::Degraded
        } else {
            ServiceHealthStatus::Unhealthy
        };

        Ok((
            status,
            format!("Memory usage: {:.1}%", memory_usage_percent),
            Some(serde_json::json!({
                "usage_percent": memory_usage_percent,
                "total_mb": system_info.total_memory_mb,
                "available_mb": system_info.available_memory_mb
            }))
        ))
    }

    async fn check_disk_health(&self) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        let system_info = self.get_system_info().await;
        let disk_usage_percent = system_info.disk_usage_percent;

        let status = if disk_usage_percent < 80.0 {
            ServiceHealthStatus::Healthy
        } else if disk_usage_percent < 90.0 {
            ServiceHealthStatus::Degraded
        } else {
            ServiceHealthStatus::Unhealthy
        };

        Ok((
            status,
            format!("Disk usage: {:.1}%", disk_usage_percent),
            Some(serde_json::json!({
                "usage_percent": disk_usage_percent
            }))
        ))
    }

    async fn check_network_health(&self) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        // Simple network connectivity check
        let start_time = std::time::Instant::now();

        // Try to resolve a hostname (basic network check)
        let result = tokio::net::lookup_host("google.com:80").await;
        let duration = start_time.elapsed();

        match result {
            Ok(_) => Ok((
                ServiceHealthStatus::Healthy,
                format!("Network connectivity OK ({:?})", duration),
                Some(serde_json::json!({
                    "response_time_ms": duration.as_millis()
                }))
            )),
            Err(e) => Ok((
                ServiceHealthStatus::Degraded,
                format!("Network connectivity issues: {}", e),
                None
            ))
        }
    }

    async fn check_custom_health(&self, check_name: &str) -> Result<(ServiceHealthStatus, String, Option<serde_json::Value>)> {
        // Placeholder for custom health checks
        Ok((
            ServiceHealthStatus::Healthy,
            format!("Custom check '{}' passed", check_name),
            None
        ))
    }

    // ========================================================================
    // Service Information Methods
    // ========================================================================

    async fn check_external_services(&self) -> Vec<ExternalServiceHealth> {
        let mut services = Vec::new();

        // Mock external service checks
        services.push(ExternalServiceHealth {
            service_name: "api-gateway".to_string(),
            url: "http://localhost:8000/health".to_string(),
            status: ServiceHealthStatus::Healthy,
            response_time_ms: 25,
            last_check: Utc::now(),
            error_count: 0,
        });

        services.push(ExternalServiceHealth {
            service_name: "intent-parser".to_string(),
            url: "http://localhost:8001/health".to_string(),
            status: ServiceHealthStatus::Healthy,
            response_time_ms: 18,
            last_check: Utc::now(),
            error_count: 0,
        });

        services
    }

    async fn get_service_metrics(&self) -> ServiceMetrics {
        // Mock service metrics - in production would collect real metrics
        ServiceMetrics {
            requests_per_second: 12.5,
            average_response_time_ms: 95.2,
            error_rate_percent: 0.1,
            active_connections: 8,
            memory_usage_mb: 128.0,
            cpu_usage_percent: 15.3,
        }
    }

    async fn get_system_info(&self) -> SystemInfo {
        // Get system information
        SystemInfo {
            hostname: hostname::get().unwrap_or_default().to_string_lossy().to_string(),
            platform: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_cores: num_cpus::get(),
            total_memory_mb: self.get_total_memory_mb(),
            available_memory_mb: self.get_available_memory_mb(),
            disk_usage_percent: self.get_disk_usage_percent(),
            load_average: self.get_load_average(),
        }
    }

    // ========================================================================
    // System Resource Helpers
    // ========================================================================

    fn get_total_memory_mb(&self) -> u64 {
        // Mock implementation - in production would use system APIs
        8192 // 8GB
    }

    fn get_available_memory_mb(&self) -> u64 {
        // Mock implementation - in production would use system APIs
        6144 // 6GB available
    }

    fn get_disk_usage_percent(&self) -> f64 {
        // Mock implementation - in production would check actual disk usage
        45.2
    }

    fn get_load_average(&self) -> Vec<f64> {
        // Mock implementation - in production would get real load average
        vec![0.8, 1.2, 1.1]
    }

    // ========================================================================
    // Status Determination Logic
    // ========================================================================

    fn determine_overall_status(
        &self,
        database_health: &DatabaseHealthStatus,
        external_services: &[ExternalServiceHealth],
    ) -> ServiceHealthStatus {
        // Critical dependency: PostgreSQL must be healthy
        if !matches!(database_health.postgresql.status, ServiceHealthStatus::Healthy) {
            return ServiceHealthStatus::Unhealthy;
        }

        // Check if any critical external services are down
        let critical_service_down = external_services.iter()
            .any(|service| matches!(service.status, ServiceHealthStatus::Unhealthy));

        if critical_service_down {
            return ServiceHealthStatus::Degraded;
        }

        // Check if Redis or MongoDB are degraded (non-critical but important)
        if matches!(database_health.redis.status, ServiceHealthStatus::Unhealthy) ||
           matches!(database_health.mongodb.status, ServiceHealthStatus::Unhealthy) {
            return ServiceHealthStatus::Degraded;
        }

        ServiceHealthStatus::Healthy
    }

    fn determine_detailed_status(
        &self,
        database_health: &DatabaseHealthStatus,
        external_services: &[ExternalServiceHealth],
        health_checks: &[HealthCheckResult],
    ) -> ServiceHealthStatus {
        // Start with basic status
        let mut status = self.determine_overall_status(database_health, external_services);

        // Factor in additional health checks
        let failed_checks = health_checks.iter()
            .filter(|check| matches!(check.status, ServiceHealthStatus::Unhealthy))
            .count();

        let degraded_checks = health_checks.iter()
            .filter(|check| matches!(check.status, ServiceHealthStatus::Degraded))
            .count();

        // Adjust status based on health check results
        if failed_checks > 0 {
            status = ServiceHealthStatus::Unhealthy;
        } else if degraded_checks > 1 {
            status = ServiceHealthStatus::Degraded;
        }

        status
    }
}

// ============================================================================
// Additional Dependencies (would be in Cargo.toml)
// ============================================================================

/*
[dependencies]
hostname = "0.3"
num_cpus = "1.16"
*/
