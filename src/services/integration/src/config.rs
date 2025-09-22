//! Configuration module for the AI-CORE Integration Service
//!
//! This module provides comprehensive configuration structures for all supported
//! third-party integrations including Zapier, Slack, and GitHub.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Main configuration structure for the Integration Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Redis configuration for caching and rate limiting
    pub redis: RedisConfig,
    /// Zapier integration configuration
    pub zapier: ZapierConfig,
    /// Slack integration configuration
    pub slack: SlackConfig,
    /// GitHub integration configuration
    pub github: GitHubConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Observability configuration
    pub observability: ObservabilityConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host (default: 0.0.0.0)
    pub host: String,
    /// Server port (default: 8004)
    pub port: u16,
    /// Request timeout in seconds (default: 30)
    pub request_timeout: u64,
    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,
    /// Enable CORS (default: true)
    pub cors_enabled: bool,
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub postgres_url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Key prefix for all Redis keys
    pub key_prefix: String,
}

/// Zapier integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierConfig {
    /// Enable Zapier integration
    pub enabled: bool,
    /// Webhook secret key for signature verification
    pub webhook_secret: Option<String>,
    /// Webhook endpoint path (default: /webhooks/zapier)
    pub webhook_path: String,
    /// Maximum payload size in bytes
    pub max_payload_size: usize,
    /// Timeout for processing webhooks in seconds
    pub processing_timeout: u64,
    /// Enable request logging
    pub log_requests: bool,
    /// Custom headers to include in responses
    pub response_headers: HashMap<String, String>,
}

/// Slack integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Enable Slack integration
    pub enabled: bool,
    /// Slack bot token (xoxb-)
    pub bot_token: Option<String>,
    /// Slack app token (xapp-)
    pub app_token: Option<String>,
    /// Slack signing secret for webhook verification
    pub signing_secret: Option<String>,
    /// OAuth client ID
    pub client_id: Option<String>,
    /// OAuth client secret
    pub client_secret: Option<String>,
    /// OAuth redirect URI
    pub redirect_uri: Option<String>,
    /// Slack API base URL
    pub api_base_url: String,
    /// Socket mode enabled for real-time events
    pub socket_mode: bool,
    /// Webhook endpoint path (default: /webhooks/slack)
    pub webhook_path: String,
    /// OAuth callback endpoint path (default: /oauth/slack/callback)
    pub oauth_callback_path: String,
    /// Bot scopes required for the application
    pub bot_scopes: Vec<String>,
    /// User scopes required for the application
    pub user_scopes: Vec<String>,
}

/// GitHub integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// Enable GitHub integration
    pub enabled: bool,
    /// GitHub App ID
    pub app_id: Option<u64>,
    /// GitHub App private key (PEM format)
    pub private_key: Option<String>,
    /// GitHub webhook secret for signature verification
    pub webhook_secret: Option<String>,
    /// GitHub API base URL (for GitHub Enterprise)
    pub api_base_url: String,
    /// OAuth client ID (for user authentication)
    pub client_id: Option<String>,
    /// OAuth client secret (for user authentication)
    pub client_secret: Option<String>,
    /// OAuth redirect URI
    pub redirect_uri: Option<String>,
    /// Webhook endpoint path (default: /webhooks/github)
    pub webhook_path: String,
    /// OAuth callback endpoint path (default: /oauth/github/callback)
    pub oauth_callback_path: String,
    /// Default repository permissions
    pub default_permissions: Vec<String>,
    /// Events to subscribe to
    pub webhook_events: Vec<String>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT signing key
    pub jwt_secret: String,
    /// JWT token expiration time in seconds
    pub jwt_expiration: u64,
    /// Enable API key authentication
    pub api_key_enabled: bool,
    /// Valid API keys
    pub api_keys: Vec<String>,
    /// Enable request signing
    pub request_signing_enabled: bool,
    /// HMAC signing key
    pub hmac_key: Option<String>,
    /// Enable HTTPS redirect
    pub force_https: bool,
    /// Trusted proxy IPs for rate limiting
    pub trusted_proxies: Vec<String>,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,
    /// Metrics endpoint path (default: /metrics)
    pub metrics_path: String,
    /// Enable health checks
    pub health_checks_enabled: bool,
    /// Health check endpoint path (default: /health)
    pub health_path: String,
    /// Tracing configuration
    pub tracing: TracingConfig,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Log format (json, text)
    pub log_format: String,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable distributed tracing
    pub enabled: bool,
    /// Jaeger endpoint URL
    pub jaeger_endpoint: Option<String>,
    /// Service name for tracing
    pub service_name: String,
    /// Sampling ratio (0.0 to 1.0)
    pub sampling_ratio: f64,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Default requests per second limit
    pub requests_per_second: u32,
    /// Burst size for rate limiting
    pub burst_size: u32,
    /// Per-integration rate limits
    pub per_integration_limits: HashMap<String, IntegrationRateLimit>,
}

/// Rate limiting configuration for specific integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationRateLimit {
    /// Requests per second for this integration
    pub requests_per_second: u32,
    /// Burst size for this integration
    pub burst_size: u32,
    /// Whether to apply per-IP limiting
    pub per_ip_enabled: bool,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            zapier: ZapierConfig::default(),
            slack: SlackConfig::default(),
            github: GitHubConfig::default(),
            security: SecurityConfig::default(),
            observability: ObservabilityConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8004,
            request_timeout: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: "postgresql://localhost:5432/ai_core_integration".to_string(),
            max_connections: 10,
            connection_timeout: 30,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            max_connections: 10,
            connection_timeout: 5,
            key_prefix: "integration:".to_string(),
        }
    }
}

impl Default for ZapierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            webhook_secret: None,
            webhook_path: "/webhooks/zapier".to_string(),
            max_payload_size: 1024 * 1024, // 1MB
            processing_timeout: 30,
            log_requests: true,
            response_headers: HashMap::new(),
        }
    }
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bot_token: None,
            app_token: None,
            signing_secret: None,
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            api_base_url: "https://slack.com/api".to_string(),
            socket_mode: false,
            webhook_path: "/webhooks/slack".to_string(),
            oauth_callback_path: "/oauth/slack/callback".to_string(),
            bot_scopes: vec![
                "app_mentions:read".to_string(),
                "channels:read".to_string(),
                "chat:write".to_string(),
                "commands".to_string(),
                "im:read".to_string(),
                "im:write".to_string(),
                "users:read".to_string(),
            ],
            user_scopes: vec!["identity.basic".to_string()],
        }
    }
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: None,
            private_key: None,
            webhook_secret: None,
            api_base_url: "https://api.github.com".to_string(),
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            webhook_path: "/webhooks/github".to_string(),
            oauth_callback_path: "/oauth/github/callback".to_string(),
            default_permissions: vec![
                "contents".to_string(),
                "issues".to_string(),
                "pull_requests".to_string(),
                "metadata".to_string(),
            ],
            webhook_events: vec![
                "push".to_string(),
                "pull_request".to_string(),
                "issues".to_string(),
                "release".to_string(),
                "workflow_run".to_string(),
            ],
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: uuid::Uuid::new_v4().to_string(),
            jwt_expiration: 3600, // 1 hour
            api_key_enabled: true,
            api_keys: Vec::new(),
            request_signing_enabled: false,
            hmac_key: None,
            force_https: false,
            trusted_proxies: Vec::new(),
        }
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            metrics_path: "/metrics".to_string(),
            health_checks_enabled: true,
            health_path: "/health".to_string(),
            tracing: TracingConfig::default(),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
        }
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jaeger_endpoint: None,
            service_name: "integration-service".to_string(),
            sampling_ratio: 0.1,
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
            per_integration_limits: HashMap::new(),
        }
    }
}

impl Default for IntegrationRateLimit {
    fn default() -> Self {
        Self {
            requests_per_second: 50,
            burst_size: 100,
            per_ip_enabled: true,
        }
    }
}

impl IntegrationConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8004)?
            .set_default("server.request_timeout", 30)?
            .set_default("server.max_body_size", 10485760)?
            .set_default("server.cors_enabled", true)?
            .set_default("zapier.enabled", true)?
            .set_default("zapier.webhook_path", "/webhooks/zapier")?
            .set_default("zapier.max_payload_size", 1048576)?
            .set_default("zapier.processing_timeout", 30)?
            .set_default("zapier.log_requests", true)?
            .set_default("slack.enabled", false)?
            .set_default("slack.api_base_url", "https://slack.com/api")?
            .set_default("slack.socket_mode", false)?
            .set_default("slack.webhook_path", "/webhooks/slack")?
            .set_default("slack.oauth_callback_path", "/oauth/slack/callback")?
            .set_default("github.enabled", false)?
            .set_default("github.api_base_url", "https://api.github.com")?
            .set_default("github.webhook_path", "/webhooks/github")?
            .set_default("github.oauth_callback_path", "/oauth/github/callback")?
            .set_default("security.jwt_expiration", 3600)?
            .set_default("security.api_key_enabled", true)?
            .set_default("security.request_signing_enabled", false)?
            .set_default("security.force_https", false)?
            .set_default("observability.metrics_enabled", true)?
            .set_default("observability.metrics_path", "/metrics")?
            .set_default("observability.health_checks_enabled", true)?
            .set_default("observability.health_path", "/health")?
            .set_default("observability.tracing.enabled", true)?
            .set_default("observability.tracing.service_name", "integration-service")?
            .set_default("observability.tracing.sampling_ratio", 0.1)?
            .set_default("observability.log_level", "info")?
            .set_default("observability.log_format", "json")?
            .set_default("rate_limiting.enabled", true)?
            .set_default("rate_limiting.requests_per_second", 100)?
            .set_default("rate_limiting.burst_size", 200)?
            .add_source(config::Environment::with_prefix("INTEGRATION").separator("_"));

        // Load from optional config file
        if let Ok(config_path) = std::env::var("INTEGRATION_CONFIG_FILE") {
            cfg = cfg.add_source(config::File::with_name(&config_path).required(false));
        }

        cfg.build()?.try_deserialize()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err("Server port cannot be 0".to_string());
        }

        // Validate enabled integrations have required fields
        if self.zapier.enabled && self.zapier.webhook_secret.is_none() {
            return Err("Zapier webhook secret is required when Zapier is enabled".to_string());
        }

        if self.slack.enabled {
            if self.slack.bot_token.is_none() {
                return Err("Slack bot token is required when Slack is enabled".to_string());
            }
            if self.slack.signing_secret.is_none() {
                return Err("Slack signing secret is required when Slack is enabled".to_string());
            }
        }

        if self.github.enabled {
            if self.github.app_id.is_none() {
                return Err("GitHub App ID is required when GitHub is enabled".to_string());
            }
            if self.github.private_key.is_none() {
                return Err("GitHub private key is required when GitHub is enabled".to_string());
            }
            if self.github.webhook_secret.is_none() {
                return Err("GitHub webhook secret is required when GitHub is enabled".to_string());
            }
        }

        // Validate URLs
        if let Some(ref jaeger_endpoint) = self.observability.tracing.jaeger_endpoint {
            Url::parse(jaeger_endpoint)
                .map_err(|e| format!("Invalid Jaeger endpoint URL: {}", e))?;
        }

        Url::parse(&self.database.postgres_url)
            .map_err(|e| format!("Invalid PostgreSQL URL: {}", e))?;

        Url::parse(&self.redis.url).map_err(|e| format!("Invalid Redis URL: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IntegrationConfig::default();
        assert_eq!(config.server.port, 8004);
        assert!(config.zapier.enabled);
        assert!(!config.slack.enabled);
        assert!(!config.github.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = IntegrationConfig::default();

        // Should pass validation by default
        assert!(config.validate().is_ok());

        // Should fail with invalid port
        config.server.port = 0;
        assert!(config.validate().is_err());

        config.server.port = 8004; // Reset

        // Should fail with enabled Zapier but no secret
        config.zapier.enabled = true;
        config.zapier.webhook_secret = None;
        assert!(config.validate().is_err());

        // Should pass with secret
        config.zapier.webhook_secret = Some("test-secret".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_integration_rate_limit() {
        let limit = IntegrationRateLimit::default();
        assert_eq!(limit.requests_per_second, 50);
        assert_eq!(limit.burst_size, 100);
        assert!(limit.per_ip_enabled);
    }

    #[test]
    fn test_slack_config_defaults() {
        let config = SlackConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.api_base_url, "https://slack.com/api");
        assert!(!config.bot_scopes.is_empty());
        assert!(config.bot_scopes.contains(&"chat:write".to_string()));
    }

    #[test]
    fn test_github_config_defaults() {
        let config = GitHubConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.api_base_url, "https://api.github.com");
        assert!(!config.webhook_events.is_empty());
        assert!(config.webhook_events.contains(&"push".to_string()));
    }
}
