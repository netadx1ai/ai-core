//! Application state management for the API Gateway
//!
//! Manages shared state including database connections, HTTP clients, and service configurations.

use std::sync::Arc;
use std::time::Duration;

use redis::aio::ConnectionManager;
use reqwest::Client;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::Config;
use crate::error::{ApiError, Result};
use crate::services::{
    auth::AuthService, circuit_breaker::CircuitBreakerService, health::HealthService,
    intent_parser::IntentParserService, metrics::MetricsService,
    orchestrator::WorkflowOrchestratorService, rate_limiter::RateLimiterService,
    router::ServiceRouter, workflow::WorkflowService,
};
use ai_core_shared::config::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy, RateLimitStrategy,
    RetryStrategy, RoutingConfig,
};

/// Application mode indicating what services are available
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Full functionality with all services available
    Full,
    /// Degraded mode with limited functionality (no database/redis)
    Degraded,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub mode: AppMode,
    pub db_pool: Option<PgPool>,
    pub redis_manager: Option<ConnectionManager>,
    pub http_client: Client,
    pub auth_service: Option<Arc<AuthService>>,
    pub rate_limiter: Option<Arc<RateLimiterService>>,
    pub service_router: Arc<ServiceRouter>,
    pub circuit_breaker: Arc<CircuitBreakerService>,
    pub health_service: Arc<HealthService>,
    pub workflow_service: Option<Arc<WorkflowService>>,
    pub workflow_orchestrator: Option<Arc<WorkflowOrchestratorService>>,
    pub intent_parser: Arc<IntentParserService>,
    pub metrics: Arc<MetricsService>,
}

impl AppState {
    /// Initialize application state with all dependencies
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing application state...");

        // Initialize database connection pool
        let db_pool = create_db_pool(&config).await?;
        info!("Database connection pool initialized");

        // Initialize Redis connection manager
        let redis_manager = create_redis_manager(&config).await?;
        info!("Redis connection manager initialized");

        // Initialize HTTP client
        let http_client = create_http_client(&config)?;
        info!("HTTP client initialized");

        // Initialize metrics service
        let metrics = Arc::new(MetricsService::new()?);
        info!("Metrics service initialized");

        // Initialize services
        let shared_auth_config = ai_core_shared::config::AuthConfig {
            jwt_secret: config.auth.jwt_secret.clone(),
            jwt_expiration_seconds: config.auth.jwt_expiry_seconds as u64,
            jwt_refresh_expiration_seconds: config.auth.refresh_token_expiry_seconds as u64,
            enable_refresh_tokens: true,
            jwt_issuer: "AI-PLATFORM-platform".to_string(),
            jwt_audience: "AI-PLATFORM-users".to_string(),
            password_min_length: 8,
            password_require_special: true,
            password_require_numbers: true,
            password_require_uppercase: true,
            max_login_attempts: 5,
            lockout_duration_seconds: 300, // 5 minutes
        };

        let auth_service = Arc::new(AuthService::new(
            shared_auth_config,
            db_pool.clone(),
            redis_manager.clone(),
        ));

        let shared_rate_limit_config = ai_core_shared::config::RateLimitConfig {
            enabled: config.rate_limiting.enabled,
            requests_per_second: 100, // Default value
            burst_size: config.rate_limiting.default_burst_size,
            strategy: RateLimitStrategy::InMemory,
            custom_limits: std::collections::HashMap::new(),
            redis_url: None,
        };

        let rate_limiter = Arc::new(RateLimiterService::new(
            shared_rate_limit_config,
            redis_manager.clone(),
        ));

        let shared_routing_config = RoutingConfig {
            discovery_enabled: false,
            upstream_timeout_seconds: 30,
            max_retries: 3,
            retry_strategy: RetryStrategy::Exponential {
                initial_delay_ms: 100,
                max_delay_ms: 5000,
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: config.routing.circuit_breaker_enabled,
                failure_threshold: config.routing.circuit_breaker_failure_threshold as u32,
                recovery_timeout_seconds: config.routing.circuit_breaker_timeout_seconds,
                min_request_threshold: 10,
            },
            load_balancing: LoadBalancingStrategy::RoundRobin,
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: config.routing.health_check_interval_seconds,
                timeout_seconds: 5,
                failure_threshold: 3,
                success_threshold: 2,
            },
        };

        let circuit_breaker = Arc::new(CircuitBreakerService::new(shared_routing_config.clone()));

        let service_router = Arc::new(ServiceRouter::new(
            shared_routing_config.clone(),
            http_client.clone(),
            circuit_breaker.clone(),
        ));

        let health_service = Arc::new(HealthService::new(
            Some(db_pool.clone()),
            Some(redis_manager.clone()),
            service_router.clone(),
            config.routing.clone(),
        ));

        let workflow_service = Arc::new(WorkflowService::new(db_pool.clone()));

        let workflow_orchestrator = Arc::new(WorkflowOrchestratorService::new(db_pool.clone()));

        let intent_parser = Arc::new(IntentParserService::new());

        info!("All services initialized successfully");

        Ok(Self {
            config,
            mode: AppMode::Full,
            db_pool: Some(db_pool),
            redis_manager: Some(redis_manager),
            http_client,
            auth_service: Some(auth_service),
            rate_limiter: Some(rate_limiter),
            service_router,
            circuit_breaker,
            health_service,
            workflow_service: Some(workflow_service),
            workflow_orchestrator: Some(workflow_orchestrator),
            intent_parser,
            metrics,
        })
    }

    /// Initialize application state in degraded mode (without database/redis)
    pub async fn new_degraded(config: Config) -> Result<Self> {
        warn!("Initializing application state in degraded mode...");

        // Initialize HTTP client
        let http_client = create_http_client(&config)?;
        info!("HTTP client initialized");

        // Initialize metrics service (in-memory only)
        let metrics = Arc::new(MetricsService::new()?);
        info!("Metrics service initialized (in-memory mode)");

        // Initialize minimal services that don't require database/redis
        let shared_routing_config = RoutingConfig {
            discovery_enabled: false,
            upstream_timeout_seconds: 30,
            max_retries: 3,
            retry_strategy: RetryStrategy::Exponential {
                initial_delay_ms: 100,
                max_delay_ms: 5000,
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: config.routing.circuit_breaker_enabled,
                failure_threshold: config.routing.circuit_breaker_failure_threshold as u32,
                recovery_timeout_seconds: config.routing.circuit_breaker_timeout_seconds,
                min_request_threshold: 10,
            },
            load_balancing: LoadBalancingStrategy::RoundRobin,
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: config.routing.health_check_interval_seconds,
                timeout_seconds: 5,
                failure_threshold: 3,
                success_threshold: 2,
            },
        };

        let circuit_breaker = Arc::new(CircuitBreakerService::new(shared_routing_config.clone()));

        let service_router = Arc::new(ServiceRouter::new(
            shared_routing_config.clone(),
            http_client.clone(),
            circuit_breaker.clone(),
        ));

        let health_service = Arc::new(HealthService::new(
            None, // No database
            None, // No redis
            service_router.clone(),
            config.routing.clone(),
        ));

        let intent_parser = Arc::new(IntentParserService::new());

        warn!("Application initialized in degraded mode - database-dependent features disabled");

        Ok(Self {
            config,
            mode: AppMode::Degraded,
            db_pool: None,
            redis_manager: None,
            http_client,
            auth_service: None,
            rate_limiter: None,
            service_router,
            circuit_breaker,
            health_service,
            workflow_service: None,
            workflow_orchestrator: None,
            intent_parser,
            metrics,
        })
    }

    /// Get a database connection from the pool
    pub async fn get_db_connection(&self) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>> {
        match &self.db_pool {
            Some(pool) => pool.acquire().await.map_err(ApiError::Database),
            None => Err(ApiError::service_unavailable("database")),
        }
    }

    /// Get a Redis connection
    pub async fn get_redis_connection(&self) -> Result<ConnectionManager> {
        match &self.redis_manager {
            Some(manager) => Ok(manager.clone()),
            None => Err(ApiError::service_unavailable("redis")),
        }
    }

    /// Check if the application is healthy
    pub async fn is_healthy(&self) -> bool {
        self.health_service.check_all().await.is_ok()
    }

    /// Check if running in degraded mode
    pub fn is_degraded(&self) -> bool {
        self.mode == AppMode::Degraded
    }

    /// Check if full functionality is available
    pub fn is_full_mode(&self) -> bool {
        self.mode == AppMode::Full
    }
}

/// Create database connection pool with proper configuration
async fn create_db_pool(config: &Config) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .acquire_timeout(Duration::from_secs(config.database.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(config.database.idle_timeout_seconds))
        .max_lifetime(Duration::from_secs(config.database.max_lifetime_seconds))
        .connect(&config.database.url)
        .await
        .map_err(ApiError::Database)?;

    // Run database migrations if in development
    if config.is_development() {
        info!("Running database migrations...");
        if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
            warn!(
                "Database migration failed: {}. This is expected if database is not available.",
                e
            );
            return Err(ApiError::internal(format!("Migration failed: {}", e)));
        }
        info!("Database migrations completed");
    }

    Ok(pool)
}

/// Create Redis connection manager
async fn create_redis_manager(config: &Config) -> Result<ConnectionManager> {
    let client = redis::Client::open(config.redis.url.clone()).map_err(ApiError::Redis)?;

    let manager = ConnectionManager::new(client)
        .await
        .map_err(ApiError::Redis)?;

    // Test the connection
    let mut conn = manager.clone();
    redis::cmd("PING")
        .query_async::<_, String>(&mut conn)
        .await
        .map_err(ApiError::Redis)?;

    Ok(manager)
}

/// Create HTTP client with proper timeout and configuration
fn create_http_client(config: &Config) -> Result<Client> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .user_agent(format!("AI-PLATFORM-gateway/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(ApiError::HttpClient)?;

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_creation() {
        let config = Config {
            server: crate::config::ServerConfig::default(),
            database: crate::config::DatabaseConfig::default(),
            redis: crate::config::RedisConfig::default(),
            auth: crate::config::AuthConfig::default(),
            rate_limiting: crate::config::RateLimitConfig::default(),
            routing: crate::config::RoutingConfig::default(),
            observability: crate::config::ObservabilityConfig::default(),
            environment: "test".to_string(),
        };

        let client = create_http_client(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_metrics_service_creation() {
        use crate::services::metrics::MetricsService;
        let metrics = MetricsService::new();
        assert!(metrics.is_ok());

        let metrics = metrics.unwrap();

        // Test recording a request
        metrics.record_http_request("GET", "/health", 200, Duration::from_millis(50), "pro");

        // Test setting active connections
        metrics.set_active_connections(100.0);

        // Test setting circuit breaker state
        use crate::services::metrics::CircuitBreakerState;
        metrics.set_circuit_breaker_state("intent-parser", CircuitBreakerState::Closed);

        // Test recording rate limit hit
        metrics.record_rate_limit_hit("user123", "pro");
    }
}
