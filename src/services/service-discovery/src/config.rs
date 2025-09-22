//! Service Discovery Configuration Module
//!
//! Handles loading, validation, and management of service discovery configuration
//! from various sources including files, environment variables, and command-line arguments.

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Main service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Service registry configuration
    pub registry: RegistryConfig,

    /// Load balancer configuration
    pub load_balancer: LoadBalancerConfig,

    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,

    /// Service mesh configuration
    pub service_mesh: ServiceMeshConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,

    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,

    /// Configuration management
    pub configuration: ConfigurationManagement,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Development configuration
    pub development: DevelopmentConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Shutdown timeout in seconds
    pub shutdown_timeout: u64,

    /// Keep-alive timeout in seconds
    pub keep_alive: u64,

    /// Maximum concurrent connections
    pub max_connections: u32,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Maximum request body size
    pub body_limit: String,
}

impl ServerConfig {
    /// Get the server socket address
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .context("Invalid server address")
    }

    /// Get shutdown timeout as Duration
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout)
    }
}

/// Service registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registration settings
    pub registration: RegistrationConfig,

    /// Discovery settings
    pub discovery: DiscoveryConfig,

    /// Health check settings
    pub health_checks: HealthCheckConfig,
}

/// Service registration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    /// Default TTL for service registrations in seconds
    pub default_ttl: u32,

    /// Heartbeat interval in seconds
    pub heartbeat_interval: u32,

    /// Grace period before marking unhealthy in seconds
    pub grace_period: u32,

    /// Maximum registration retries
    pub max_retries: u32,

    /// Retry interval in seconds
    pub retry_interval: u32,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Cache TTL in seconds
    pub cache_ttl: u32,

    /// Refresh interval in seconds
    pub refresh_interval: u32,

    /// Batch size for discovery operations
    pub batch_size: u32,

    /// Enable caching
    pub enable_caching: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,

    /// Default health check interval in seconds
    pub default_interval: u32,

    /// Health check timeout in seconds
    pub timeout: u32,

    /// Failure threshold before marking unhealthy
    pub failure_threshold: u32,

    /// Success threshold before marking healthy
    pub success_threshold: u32,

    /// Health check types configuration
    pub types: HealthCheckTypesConfig,
}

/// Health check types configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckTypesConfig {
    /// HTTP health check configuration
    pub http: HttpHealthCheckConfig,

    /// TCP health check configuration
    pub tcp: TcpHealthCheckConfig,

    /// gRPC health check configuration
    pub grpc: GrpcHealthCheckConfig,
}

/// HTTP health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHealthCheckConfig {
    /// Enable HTTP health checks
    pub enabled: bool,

    /// Default health check path
    pub default_path: String,

    /// Follow redirects
    pub follow_redirects: bool,
}

/// TCP health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpHealthCheckConfig {
    /// Enable TCP health checks
    pub enabled: bool,

    /// Connection timeout in seconds
    pub connect_timeout: u32,
}

/// gRPC health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcHealthCheckConfig {
    /// Enable gRPC health checks
    pub enabled: bool,

    /// Default service name for health checks
    pub service_name: String,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Default load balancing strategy
    pub default_strategy: String,

    /// Strategy-specific configurations
    pub strategies: LoadBalancingStrategiesConfig,
}

/// Load balancing strategies configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingStrategiesConfig {
    /// Round robin strategy
    pub round_robin: StrategyConfig,

    /// Least connections strategy
    pub least_connections: LeastConnectionsConfig,

    /// Weighted round robin strategy
    pub weighted_round_robin: WeightedRoundRobinConfig,

    /// Consistent hash strategy
    pub consistent_hash: ConsistentHashConfig,

    /// Random strategy
    pub random: StrategyConfig,
}

/// Basic strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Enable this strategy
    pub enabled: bool,
}

/// Least connections strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeastConnectionsConfig {
    /// Enable this strategy
    pub enabled: bool,

    /// Enable connection tracking
    pub connection_tracking: bool,
}

/// Weighted round robin strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedRoundRobinConfig {
    /// Enable this strategy
    pub enabled: bool,

    /// Default weight for services
    pub default_weight: u32,
}

/// Consistent hash strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistentHashConfig {
    /// Enable this strategy
    pub enabled: bool,

    /// Number of virtual nodes
    pub virtual_nodes: u32,

    /// Hash function to use
    pub hash_function: String,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,

    /// Default circuit breaker settings
    pub defaults: CircuitBreakerDefaults,

    /// Per-service overrides
    pub services: HashMap<String, CircuitBreakerDefaults>,
}

/// Circuit breaker default settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerDefaults {
    /// Failure threshold
    pub failure_threshold: u32,

    /// Recovery timeout in seconds
    pub recovery_timeout: u32,

    /// Success threshold
    pub success_threshold: u32,

    /// Request timeout in seconds
    pub timeout: u32,

    /// Maximum retries
    pub max_retries: u32,
}

/// Service mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshConfig {
    /// Enable service mesh integration
    pub enabled: bool,

    /// Service mesh backend
    pub backend: String,

    /// Consul configuration
    pub consul: ConsulConfig,

    /// etcd configuration
    pub etcd: EtcdConfig,

    /// Kubernetes configuration
    pub kubernetes: KubernetesConfig,
}

/// Consul configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulConfig {
    /// Consul address
    pub address: String,

    /// Datacenter
    pub datacenter: String,

    /// Authentication token
    pub token: String,
}

/// etcd configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdConfig {
    /// etcd endpoints
    pub endpoints: Vec<String>,

    /// Username for authentication
    pub username: String,

    /// Password for authentication
    pub password: String,
}

/// Kubernetes configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    /// Kubernetes namespace
    pub namespace: String,

    /// Service account
    pub service_account: String,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL configuration
    pub postgres: PostgresConfig,

    /// Redis configuration
    pub redis: RedisConfig,
}

/// PostgreSQL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database URL
    pub url: String,

    /// Maximum connections in pool
    pub max_connections: u32,

    /// Minimum connections in pool
    pub min_connections: u32,

    /// Connection acquire timeout in seconds
    pub acquire_timeout: u32,

    /// Idle timeout in seconds
    pub idle_timeout: u32,

    /// Maximum connection lifetime in seconds
    pub max_lifetime: u32,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,

    /// Maximum connections in pool
    pub max_connections: u32,

    /// Connection acquire timeout in seconds
    pub acquire_timeout: u32,

    /// Idle timeout in seconds
    pub idle_timeout: u32,

    /// Maximum connection lifetime in seconds
    pub max_lifetime: u32,

    /// Redis database number
    pub database: u32,

    /// Key prefix for all operations
    pub prefix: String,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: JwtConfig,

    /// API keys configuration
    pub api_keys: ApiKeysConfig,

    /// RBAC configuration
    pub rbac: RbacConfig,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret
    pub secret: String,

    /// Token expiration in seconds
    pub expiration: u32,
}

/// API keys configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeysConfig {
    /// Enable API key authentication
    pub enabled: bool,

    /// Header name for API key
    pub header_name: String,
}

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Enable RBAC
    pub enabled: bool,

    /// Default role for new users
    pub default_role: String,

    /// Role definitions
    pub roles: HashMap<String, RoleConfig>,
}

/// Role configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    /// Permissions for this role
    pub permissions: Vec<String>,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Tracing configuration
    pub tracing: TracingConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Health endpoint configuration
    pub health: HealthEndpointConfig,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,

    /// Metrics endpoint path
    pub path: String,

    /// Include detailed labels
    pub include_labels: bool,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,

    /// Jaeger endpoint
    pub jaeger_endpoint: String,

    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,

    /// Log format (json or pretty)
    pub format: String,
}

/// Health endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEndpointConfig {
    /// Enable health endpoint
    pub enabled: bool,

    /// Health endpoint path
    pub path: String,

    /// Enable deep health checks
    pub deep_checks: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Global rate limits
    pub global: GlobalRateLimits,

    /// Per-endpoint rate limits
    pub endpoints: HashMap<String, EndpointRateLimits>,
}

/// Global rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRateLimits {
    /// Requests per minute
    pub requests_per_minute: u32,

    /// Burst capacity
    pub burst: u32,
}

/// Per-endpoint rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRateLimits {
    /// Requests per minute
    pub requests_per_minute: u32,

    /// Burst capacity
    pub burst: u32,
}

/// Configuration management settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationManagement {
    /// Enable configuration management
    pub enabled: bool,

    /// Storage configuration
    pub storage: ConfigStorageConfig,

    /// Versioning configuration
    pub versioning: ConfigVersioningConfig,

    /// Validation configuration
    pub validation: ConfigValidationConfig,

    /// Distribution configuration
    pub distribution: ConfigDistributionConfig,
}

/// Configuration storage settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigStorageConfig {
    /// Storage backend
    pub backend: String,
}

/// Configuration versioning settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersioningConfig {
    /// Enable versioning
    pub enabled: bool,

    /// Maximum versions to keep
    pub max_versions: u32,
}

/// Configuration validation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationConfig {
    /// Enable validation
    pub enabled: bool,

    /// Enable schema validation
    pub schema_validation: bool,
}

/// Configuration distribution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDistributionConfig {
    /// Enable distribution
    pub enabled: bool,

    /// Push updates to services
    pub push_updates: bool,

    /// Notification channels
    pub notification_channels: Vec<String>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Initial retry interval in seconds
    pub initial_interval: u32,

    /// Maximum retry interval in seconds
    pub max_interval: u32,

    /// Backoff multiplier
    pub multiplier: f64,

    /// Add jitter to retry intervals
    pub jitter: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// TLS configuration
    pub tls: TlsConfig,

    /// CORS configuration
    pub cors: CorsConfig,

    /// Security headers
    pub headers: SecurityHeadersConfig,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,

    /// Certificate file path
    pub cert_file: String,

    /// Private key file path
    pub key_file: String,

    /// CA certificate file path
    pub ca_file: String,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,

    /// Allowed origins
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Max age in seconds
    pub max_age: u32,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// X-Frame-Options header value
    pub x_frame_options: String,

    /// X-Content-Type-Options header value
    pub x_content_type_options: String,

    /// X-XSS-Protection header value
    pub x_xss_protection: String,
}

/// Development configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentConfig {
    /// Enable debug mode
    pub debug: bool,

    /// Pretty print logs
    pub pretty_logs: bool,

    /// Mock external services
    pub mock_external_services: bool,

    /// Test endpoints configuration
    pub test_endpoints: TestEndpointsConfig,
}

/// Test endpoints configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEndpointsConfig {
    /// Enable test endpoints
    pub enabled: bool,

    /// URL prefix for test endpoints
    pub prefix: String,
}

/// Environment-specific configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfigs {
    /// Development overrides
    pub development: Option<EnvironmentOverride>,

    /// Production overrides
    pub production: Option<EnvironmentOverride>,
}

/// Environment-specific configuration override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentOverride {
    /// Server overrides
    pub server: Option<ServerConfig>,

    /// Database overrides
    pub database: Option<DatabaseConfig>,

    /// Security overrides
    pub security: Option<SecurityConfig>,

    /// Monitoring overrides
    pub monitoring: Option<MonitoringConfig>,

    /// Rate limiting overrides
    pub rate_limiting: Option<RateLimitingConfig>,
}

/// Command-line arguments
#[derive(Debug, Parser)]
#[command(
    name = "service-discovery-server",
    about = "AI-CORE Service Discovery and Registry Service",
    version
)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config/service-discovery.yaml")]
    pub config: PathBuf,

    /// Environment (development, production)
    #[arg(short, long, default_value = "development")]
    pub environment: String,

    /// Server port (overrides config)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Log level (overrides config)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Enable debug mode
    #[arg(long)]
    pub debug: bool,

    /// Database URL (overrides config)
    #[arg(long)]
    pub database_url: Option<String>,

    /// Redis URL (overrides config)
    #[arg(long)]
    pub redis_url: Option<String>,
}

impl ServiceDiscoveryConfig {
    /// Load configuration from file and environment variables
    pub fn load(args: &Args) -> Result<Self> {
        let mut settings = config::Config::builder();

        // Load base configuration file
        if args.config.exists() {
            settings = settings.add_source(config::File::from(args.config.clone()).required(false));
        }

        // Add environment-specific overrides
        let env_file = format!("config/service-discovery-{}.yaml", args.environment);
        settings = settings.add_source(config::File::with_name(&env_file).required(false));

        // Add environment variables with prefix
        settings = settings
            .add_source(config::Environment::with_prefix("SERVICE_DISCOVERY").separator("__"));

        let mut config: ServiceDiscoveryConfig = settings
            .build()
            .context("Failed to build configuration")?
            .try_deserialize()
            .context("Failed to deserialize configuration")?;

        // Apply command-line overrides
        if let Some(port) = args.port {
            config.server.port = port;
        }

        if let Some(ref log_level) = args.log_level {
            config.monitoring.logging.level = log_level.clone();
        }

        if args.debug {
            config.development.debug = true;
            config.monitoring.logging.level = "debug".to_string();
        }

        if let Some(ref database_url) = args.database_url {
            config.database.postgres.url = database_url.clone();
        }

        if let Some(ref redis_url) = args.redis_url {
            config.database.redis.url = redis_url.clone();
        }

        // Validate configuration
        config
            .validate()
            .context("Configuration validation failed")?;

        Ok(config)
    }

    /// Validate configuration settings
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        // Validate database URLs
        if self.database.postgres.url.is_empty() {
            return Err(anyhow::anyhow!("PostgreSQL URL is required"));
        }

        if self.database.redis.url.is_empty() {
            return Err(anyhow::anyhow!("Redis URL is required"));
        }

        // Validate health check configuration
        if self.registry.health_checks.enabled {
            if self.registry.health_checks.default_interval < 5 {
                return Err(anyhow::anyhow!(
                    "Health check interval must be at least 5 seconds"
                ));
            }

            if self.registry.health_checks.timeout >= self.registry.health_checks.default_interval {
                return Err(anyhow::anyhow!(
                    "Health check timeout must be less than interval"
                ));
            }
        }

        // Validate circuit breaker configuration
        if self.circuit_breaker.enabled {
            if self.circuit_breaker.defaults.failure_threshold == 0 {
                return Err(anyhow::anyhow!(
                    "Circuit breaker failure threshold must be greater than 0"
                ));
            }
        }

        // Validate load balancer configuration
        let valid_strategies = vec![
            "round_robin",
            "least_connections",
            "weighted_round_robin",
            "consistent_hash",
            "random",
            "ip_hash",
        ];

        if !valid_strategies.contains(&self.load_balancer.default_strategy.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid load balancing strategy: {}. Valid strategies: {:?}",
                self.load_balancer.default_strategy,
                valid_strategies
            ));
        }

        Ok(())
    }
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                shutdown_timeout: 30,
                keep_alive: 75,
                max_connections: 10000,
                request_timeout: 60,
                body_limit: "10MB".to_string(),
            },
            registry: RegistryConfig {
                registration: RegistrationConfig {
                    default_ttl: 30,
                    heartbeat_interval: 10,
                    grace_period: 15,
                    max_retries: 3,
                    retry_interval: 5,
                },
                discovery: DiscoveryConfig {
                    cache_ttl: 60,
                    refresh_interval: 30,
                    batch_size: 100,
                    enable_caching: true,
                },
                health_checks: HealthCheckConfig {
                    enabled: true,
                    default_interval: 30,
                    timeout: 5,
                    failure_threshold: 3,
                    success_threshold: 2,
                    types: HealthCheckTypesConfig {
                        http: HttpHealthCheckConfig {
                            enabled: true,
                            default_path: "/health".to_string(),
                            follow_redirects: false,
                        },
                        tcp: TcpHealthCheckConfig {
                            enabled: true,
                            connect_timeout: 5,
                        },
                        grpc: GrpcHealthCheckConfig {
                            enabled: true,
                            service_name: "health".to_string(),
                        },
                    },
                },
            },
            load_balancer: LoadBalancerConfig {
                default_strategy: "round_robin".to_string(),
                strategies: LoadBalancingStrategiesConfig {
                    round_robin: StrategyConfig { enabled: true },
                    least_connections: LeastConnectionsConfig {
                        enabled: true,
                        connection_tracking: true,
                    },
                    weighted_round_robin: WeightedRoundRobinConfig {
                        enabled: true,
                        default_weight: 100,
                    },
                    consistent_hash: ConsistentHashConfig {
                        enabled: true,
                        virtual_nodes: 150,
                        hash_function: "sha256".to_string(),
                    },
                    random: StrategyConfig { enabled: true },
                },
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                defaults: CircuitBreakerDefaults {
                    failure_threshold: 5,
                    recovery_timeout: 60,
                    success_threshold: 3,
                    timeout: 30,
                    max_retries: 3,
                },
                services: HashMap::new(),
            },
            service_mesh: ServiceMeshConfig {
                enabled: true,
                backend: "native".to_string(),
                consul: ConsulConfig {
                    address: "http://consul:8500".to_string(),
                    datacenter: "dc1".to_string(),
                    token: String::new(),
                },
                etcd: EtcdConfig {
                    endpoints: vec!["http://etcd:2379".to_string()],
                    username: String::new(),
                    password: String::new(),
                },
                kubernetes: KubernetesConfig {
                    namespace: "default".to_string(),
                    service_account: String::new(),
                },
            },
            database: DatabaseConfig {
                postgres: PostgresConfig {
                    url: "postgresql://postgres:password@localhost:5432/ai_core".to_string(),
                    max_connections: 20,
                    min_connections: 5,
                    acquire_timeout: 30,
                    idle_timeout: 600,
                    max_lifetime: 1800,
                },
                redis: RedisConfig {
                    url: "redis://localhost:6379".to_string(),
                    max_connections: 20,
                    acquire_timeout: 30,
                    idle_timeout: 300,
                    max_lifetime: 3600,
                    database: 0,
                    prefix: "service_discovery:".to_string(),
                },
            },
            auth: AuthConfig {
                jwt: JwtConfig {
                    secret: "your-secret-key".to_string(),
                    expiration: 3600,
                },
                api_keys: ApiKeysConfig {
                    enabled: true,
                    header_name: "X-API-Key".to_string(),
                },
                rbac: RbacConfig {
                    enabled: true,
                    default_role: "viewer".to_string(),
                    roles: HashMap::from([
                        (
                            "admin".to_string(),
                            RoleConfig {
                                permissions: vec!["*".to_string()],
                            },
                        ),
                        (
                            "operator".to_string(),
                            RoleConfig {
                                permissions: vec![
                                    "service:register".to_string(),
                                    "service:deregister".to_string(),
                                    "service:update".to_string(),
                                    "service:read".to_string(),
                                ],
                            },
                        ),
                        (
                            "viewer".to_string(),
                            RoleConfig {
                                permissions: vec!["service:read".to_string()],
                            },
                        ),
                    ]),
                },
            },
            monitoring: MonitoringConfig {
                metrics: MetricsConfig {
                    enabled: true,
                    path: "/metrics".to_string(),
                    include_labels: true,
                },
                tracing: TracingConfig {
                    enabled: true,
                    jaeger_endpoint: "http://jaeger:14268/api/traces".to_string(),
                    sample_rate: 0.1,
                },
                logging: LoggingConfig {
                    level: "info".to_string(),
                    format: "json".to_string(),
                },
                health: HealthEndpointConfig {
                    enabled: true,
                    path: "/health".to_string(),
                    deep_checks: true,
                },
            },
            rate_limiting: RateLimitingConfig {
                enabled: true,
                global: GlobalRateLimits {
                    requests_per_minute: 10000,
                    burst: 1000,
                },
                endpoints: HashMap::from([
                    (
                        "register".to_string(),
                        EndpointRateLimits {
                            requests_per_minute: 100,
                            burst: 20,
                        },
                    ),
                    (
                        "deregister".to_string(),
                        EndpointRateLimits {
                            requests_per_minute: 100,
                            burst: 20,
                        },
                    ),
                    (
                        "discover".to_string(),
                        EndpointRateLimits {
                            requests_per_minute: 1000,
                            burst: 200,
                        },
                    ),
                ]),
            },
            configuration: ConfigurationManagement {
                enabled: true,
                storage: ConfigStorageConfig {
                    backend: "database".to_string(),
                },
                versioning: ConfigVersioningConfig {
                    enabled: true,
                    max_versions: 10,
                },
                validation: ConfigValidationConfig {
                    enabled: true,
                    schema_validation: true,
                },
                distribution: ConfigDistributionConfig {
                    enabled: true,
                    push_updates: true,
                    notification_channels: vec!["webhook".to_string(), "redis_pubsub".to_string()],
                },
            },
            retry: RetryConfig {
                max_attempts: 3,
                initial_interval: 1,
                max_interval: 60,
                multiplier: 2.0,
                jitter: true,
            },
            security: SecurityConfig {
                tls: TlsConfig {
                    enabled: false,
                    cert_file: String::new(),
                    key_file: String::new(),
                    ca_file: String::new(),
                },
                cors: CorsConfig {
                    enabled: true,
                    allowed_origins: vec!["*".to_string()],
                    allowed_methods: vec![
                        "GET".to_string(),
                        "POST".to_string(),
                        "PUT".to_string(),
                        "DELETE".to_string(),
                        "OPTIONS".to_string(),
                    ],
                    allowed_headers: vec!["*".to_string()],
                    max_age: 3600,
                },
                headers: SecurityHeadersConfig {
                    x_frame_options: "DENY".to_string(),
                    x_content_type_options: "nosniff".to_string(),
                    x_xss_protection: "1; mode=block".to_string(),
                },
            },
            development: DevelopmentConfig {
                debug: false,
                pretty_logs: true,
                mock_external_services: false,
                test_endpoints: TestEndpointsConfig {
                    enabled: false,
                    prefix: "/test".to_string(),
                },
            },
        }
    }
}
