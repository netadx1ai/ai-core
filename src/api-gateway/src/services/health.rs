//! Health check service for monitoring system status

use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::{
    config::RoutingConfig,
    error::{ApiError, Result},
    services::router::ServiceRouter,
};
use ai_core_shared::types::core::{HealthStatus, ServiceHealth, SystemInfo};

/// Health check service
#[derive(Clone)]
pub struct HealthService {
    db_pool: Option<PgPool>,
    redis_manager: Option<ConnectionManager>,
    service_router: Arc<ServiceRouter>,
    routing_config: RoutingConfig,
}

impl HealthService {
    /// Create a new health service
    pub fn new(
        db_pool: Option<PgPool>,
        redis_manager: Option<ConnectionManager>,
        service_router: Arc<ServiceRouter>,
        routing_config: RoutingConfig,
    ) -> Self {
        Self {
            db_pool,
            redis_manager,
            service_router,
            routing_config,
        }
    }

    /// Check health of all components
    pub async fn check_all(&self) -> Result<Vec<ServiceHealth>> {
        let mut services = Vec::new();

        // Check database
        services.push(self.check_database().await);

        // Check Redis
        services.push(self.check_redis().await);

        // Note: Downstream service checks disabled - shared RoutingConfig doesn't have services field
        // This could be re-enabled by adding service discovery integration

        Ok(services)
    }

    /// Get overall health status
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        let services = self.check_all().await?;

        if services.iter().any(|s| s.status == HealthStatus::Unhealthy) {
            Ok(HealthStatus::Unhealthy)
        } else if services.iter().any(|s| s.status == HealthStatus::Degraded) {
            Ok(HealthStatus::Degraded)
        } else {
            Ok(HealthStatus::Healthy)
        }
    }

    /// Check database health
    async fn check_database(&self) -> ServiceHealth {
        match &self.db_pool {
            Some(pool) => {
                let start = std::time::Instant::now();

                match sqlx::query("SELECT 1").fetch_one(pool).await {
                    Ok(_) => {
                        let response_time = start.elapsed().as_millis() as f64;
                        debug!(
                            response_time_ms = response_time,
                            "Database health check passed"
                        );
                        ServiceHealth {
                            name: "database".to_string(),
                            status: HealthStatus::Healthy,
                            response_time_ms: Some(response_time),
                            last_check: chrono::Utc::now(),
                            error: None,
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Database health check failed");
                        ServiceHealth {
                            name: "database".to_string(),
                            status: HealthStatus::Unhealthy,
                            response_time_ms: None,
                            last_check: chrono::Utc::now(),
                            error: Some(e.to_string()),
                        }
                    }
                }
            }
            None => {
                debug!("Database not available (degraded mode)");
                ServiceHealth {
                    name: "database".to_string(),
                    status: HealthStatus::Degraded,
                    response_time_ms: None,
                    last_check: chrono::Utc::now(),
                    error: Some("Database not configured (degraded mode)".to_string()),
                }
            }
        }
    }

    /// Check Redis health
    async fn check_redis(&self) -> ServiceHealth {
        match &self.redis_manager {
            Some(manager) => {
                let start = std::time::Instant::now();
                let mut conn = manager.clone();

                match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                    Ok(_) => {
                        let response_time = start.elapsed().as_millis() as f64;
                        debug!(
                            response_time_ms = response_time,
                            "Redis health check passed"
                        );
                        ServiceHealth {
                            name: "redis".to_string(),
                            status: HealthStatus::Healthy,
                            response_time_ms: Some(response_time),
                            last_check: chrono::Utc::now(),
                            error: None,
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Redis health check failed");
                        ServiceHealth {
                            name: "redis".to_string(),
                            status: HealthStatus::Unhealthy,
                            response_time_ms: None,
                            last_check: chrono::Utc::now(),
                            error: Some(e.to_string()),
                        }
                    }
                }
            }
            None => {
                debug!("Redis not available (degraded mode)");
                ServiceHealth {
                    name: "redis".to_string(),
                    status: HealthStatus::Degraded,
                    response_time_ms: None,
                    last_check: chrono::Utc::now(),
                    error: Some("Redis not configured (degraded mode)".to_string()),
                }
            }
        }
    }

    /// Check downstream service health
    async fn check_service(&self, name: &str, _url: &str) -> ServiceHealth {
        // For now, return healthy status
        // In a full implementation, this would make HTTP requests to service health endpoints
        ServiceHealth {
            name: name.to_string(),
            status: HealthStatus::Healthy,
            response_time_ms: Some(10.0),
            last_check: chrono::Utc::now(),
            error: None,
        }
    }

    /// Get system information
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            platform_name: "AI-PLATFORM Intelligent Automation Platform".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_date: chrono::Utc::now(), // In real app, this would be build time
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            api_version: "v1".to_string(),
            documentation_url: Some("https://docs.AI-PLATFORM.dev".to_string()),
            support_email: Some("support@AI-PLATFORM.dev".to_string()),
            features: vec![
                "workflows".to_string(),
                "content-generation".to_string(),
                "analytics".to_string(),
                "federation".to_string(),
            ],
        }
    }
}
