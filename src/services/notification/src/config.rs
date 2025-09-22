//! Configuration module for the notification service
//!
//! This module provides configuration structures and defaults for all notification
//! channels and service settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Main configuration structure for the notification service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Redis configuration for caching and pub/sub
    pub redis: RedisConfig,

    /// Email configuration
    pub email: EmailConfig,

    /// SMS configuration
    pub sms: SmsConfig,

    /// Push notification configuration
    pub push: PushConfig,

    /// Webhook configuration
    pub webhook: WebhookConfig,

    /// WebSocket configuration
    pub websocket: WebSocketConfig,

    /// Template configuration
    pub template: TemplateConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,

    /// Scheduling configuration
    pub scheduler: SchedulerConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub max_connections: usize,
    pub timeout_seconds: u64,
    pub keep_alive_seconds: u64,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub postgres_url: String,
    pub mongo_url: String,
    pub max_pool_size: u32,
    pub min_pool_size: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_pool_size: u32,
    pub connection_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
    pub key_prefix: String,
    pub cache_ttl_seconds: u64,
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_use_tls: bool,
    pub smtp_use_starttls: bool,
    pub from_email: String,
    pub from_name: String,
    pub reply_to: Option<String>,
    pub max_recipients_per_message: usize,
    pub timeout_seconds: u64,
    pub rate_limit_per_minute: u32,
}

/// SMS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub enabled: bool,
    pub provider: SmsProvider,
    pub twilio: Option<TwilioConfig>,
    pub aws_sns: Option<AwsSnsConfig>,
    pub timeout_seconds: u64,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmsProvider {
    Twilio,
    AwsSns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub from_phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsSnsConfig {
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

/// Push notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushConfig {
    pub enabled: bool,
    pub web_push: Option<WebPushConfig>,
    pub fcm: Option<FcmConfig>,
    pub timeout_seconds: u64,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPushConfig {
    pub vapid_subject: String,
    pub vapid_public_key: String,
    pub vapid_private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmConfig {
    pub server_key: String,
    pub project_id: String,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub verify_ssl: bool,
    pub user_agent: String,
    pub max_payload_size: usize,
    pub rate_limit_per_minute: u32,
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub max_connections: usize,
    pub ping_interval_seconds: u64,
    pub pong_timeout_seconds: u64,
    pub message_buffer_size: usize,
    pub max_message_size: usize,
}

/// Template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub cache_enabled: bool,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub template_directory: Option<String>,
    pub default_locale: String,
    pub supported_locales: Vec<String>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_seconds: u64,
    pub max_delay_seconds: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub retry_expired_notifications: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub default_per_minute: u32,
    pub default_burst: u32,
    pub channel_limits: HashMap<String, ChannelRateLimit>,
    pub user_tier_limits: HashMap<String, UserTierRateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRateLimit {
    pub per_minute: u32,
    pub burst: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTierRateLimit {
    pub email_per_minute: u32,
    pub sms_per_minute: u32,
    pub push_per_minute: u32,
    pub webhook_per_minute: u32,
    pub daily_quota: u32,
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub worker_threads: usize,
    pub check_interval_seconds: u64,
    pub batch_size: usize,
    pub cleanup_interval_hours: u64,
    pub retention_days: u32,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub namespace: String,
    pub collect_detailed_metrics: bool,
    pub histogram_buckets: Vec<f64>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            email: EmailConfig::default(),
            sms: SmsConfig::default(),
            push: PushConfig::default(),
            webhook: WebhookConfig::default(),
            websocket: WebSocketConfig::default(),
            template: TemplateConfig::default(),
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::default(),
            scheduler: SchedulerConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8086,
            workers: None,
            max_connections: 1000,
            timeout_seconds: 30,
            keep_alive_seconds: 75,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://postgres:password@localhost:5432/aicore".to_string()
            }),
            mongo_url: std::env::var("MONGO_URL")
                .unwrap_or_else(|_| "mongodb://localhost:27017/aicore".to_string()),
            max_pool_size: 20,
            min_pool_size: 5,
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 600,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            max_pool_size: 20,
            connection_timeout_seconds: 5,
            command_timeout_seconds: 30,
            key_prefix: "notification:".to_string(),
            cache_ttl_seconds: 3600,
        }
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            smtp_host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            smtp_username: std::env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: std::env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_use_tls: std::env::var("SMTP_USE_TLS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            smtp_use_starttls: std::env::var("SMTP_USE_STARTTLS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            from_email: std::env::var("FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@aicore.local".to_string()),
            from_name: std::env::var("FROM_NAME")
                .unwrap_or_else(|_| "AI-CORE Platform".to_string()),
            reply_to: std::env::var("REPLY_TO_EMAIL").ok(),
            max_recipients_per_message: 50,
            timeout_seconds: 30,
            rate_limit_per_minute: 100,
        }
    }
}

impl Default for SmsConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default due to cost
            provider: SmsProvider::Twilio,
            twilio: Some(TwilioConfig::default()),
            aws_sns: Some(AwsSnsConfig::default()),
            timeout_seconds: 30,
            rate_limit_per_minute: 60,
        }
    }
}

impl Default for TwilioConfig {
    fn default() -> Self {
        Self {
            account_sid: std::env::var("TWILIO_ACCOUNT_SID").unwrap_or_default(),
            auth_token: std::env::var("TWILIO_AUTH_TOKEN").unwrap_or_default(),
            from_phone: std::env::var("TWILIO_FROM_PHONE").unwrap_or_default(),
        }
    }
}

impl Default for AwsSnsConfig {
    fn default() -> Self {
        Self {
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").unwrap_or_default(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_default(),
        }
    }
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default
            web_push: Some(WebPushConfig::default()),
            fcm: Some(FcmConfig::default()),
            timeout_seconds: 30,
            rate_limit_per_minute: 1000,
        }
    }
}

impl Default for WebPushConfig {
    fn default() -> Self {
        Self {
            vapid_subject: std::env::var("VAPID_SUBJECT")
                .unwrap_or_else(|_| "mailto:admin@aicore.local".to_string()),
            vapid_public_key: std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default(),
            vapid_private_key: std::env::var("VAPID_PRIVATE_KEY").unwrap_or_default(),
        }
    }
}

impl Default for FcmConfig {
    fn default() -> Self {
        Self {
            server_key: std::env::var("FCM_SERVER_KEY").unwrap_or_default(),
            project_id: std::env::var("FCM_PROJECT_ID").unwrap_or_default(),
        }
    }
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            verify_ssl: true,
            user_agent: "AI-CORE-Notification-Service/1.0".to_string(),
            max_payload_size: 1024 * 1024, // 1MB
            rate_limit_per_minute: 300,
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_connections: 10000,
            ping_interval_seconds: 30,
            pong_timeout_seconds: 10,
            message_buffer_size: 1024,
            max_message_size: 64 * 1024, // 64KB
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            template_directory: None,
            default_locale: "en".to_string(),
            supported_locales: vec!["en".to_string()],
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_seconds: 1,
            max_delay_seconds: 300, // 5 minutes
            backoff_multiplier: 2.0,
            jitter: true,
            retry_expired_notifications: false,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut channel_limits = HashMap::new();
        channel_limits.insert(
            "email".to_string(),
            ChannelRateLimit {
                per_minute: 100,
                burst: 10,
            },
        );
        channel_limits.insert(
            "sms".to_string(),
            ChannelRateLimit {
                per_minute: 60,
                burst: 5,
            },
        );
        channel_limits.insert(
            "push".to_string(),
            ChannelRateLimit {
                per_minute: 1000,
                burst: 50,
            },
        );
        channel_limits.insert(
            "webhook".to_string(),
            ChannelRateLimit {
                per_minute: 300,
                burst: 20,
            },
        );

        let mut user_tier_limits = HashMap::new();
        user_tier_limits.insert(
            "free".to_string(),
            UserTierRateLimit {
                email_per_minute: 10,
                sms_per_minute: 5,
                push_per_minute: 50,
                webhook_per_minute: 20,
                daily_quota: 100,
            },
        );
        user_tier_limits.insert(
            "pro".to_string(),
            UserTierRateLimit {
                email_per_minute: 100,
                sms_per_minute: 60,
                push_per_minute: 500,
                webhook_per_minute: 200,
                daily_quota: 10000,
            },
        );
        user_tier_limits.insert(
            "enterprise".to_string(),
            UserTierRateLimit {
                email_per_minute: 1000,
                sms_per_minute: 300,
                push_per_minute: 5000,
                webhook_per_minute: 1000,
                daily_quota: 100000,
            },
        );

        Self {
            enabled: true,
            default_per_minute: 100,
            default_burst: 10,
            channel_limits,
            user_tier_limits,
        }
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            worker_threads: num_cpus::get(),
            check_interval_seconds: 60,
            batch_size: 100,
            cleanup_interval_hours: 24,
            retention_days: 30,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
            namespace: "notification_service".to_string(),
            collect_detailed_metrics: true,
            histogram_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        }
    }
}

impl NotificationConfig {
    /// Load configuration from environment variables and config file
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::builder();

        // Start with default configuration
        cfg = cfg.add_source(config::Config::try_from(&NotificationConfig::default())?);

        // Add environment variables with prefix
        cfg = cfg.add_source(
            config::Environment::with_prefix("NOTIFICATION")
                .separator("__")
                .try_parsing(true),
        );

        // Add config file if it exists
        if let Ok(config_file) = std::env::var("NOTIFICATION_CONFIG_FILE") {
            cfg = cfg.add_source(config::File::with_name(&config_file).required(false));
        }

        cfg.build()?.try_deserialize()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.server.port == 0 {
            return Err("Server port must be greater than 0".to_string());
        }

        if self.email.enabled {
            if self.email.smtp_host.is_empty() {
                return Err("SMTP host is required when email is enabled".to_string());
            }
            if self.email.from_email.is_empty() {
                return Err("From email is required when email is enabled".to_string());
            }
        }

        if self.sms.enabled {
            match self.sms.provider {
                SmsProvider::Twilio => {
                    if let Some(ref twilio) = self.sms.twilio {
                        if twilio.account_sid.is_empty() || twilio.auth_token.is_empty() {
                            return Err(
                                "Twilio credentials are required when SMS is enabled with Twilio"
                                    .to_string(),
                            );
                        }
                    } else {
                        return Err(
                            "Twilio configuration is required when SMS provider is Twilio"
                                .to_string(),
                        );
                    }
                }
                SmsProvider::AwsSns => {
                    if let Some(ref aws) = self.sms.aws_sns {
                        if aws.access_key_id.is_empty() || aws.secret_access_key.is_empty() {
                            return Err(
                                "AWS credentials are required when SMS is enabled with AWS SNS"
                                    .to_string(),
                            );
                        }
                    } else {
                        return Err(
                            "AWS SNS configuration is required when SMS provider is AWS SNS"
                                .to_string(),
                        );
                    }
                }
            }
        }

        if self.push.enabled {
            if let Some(ref web_push) = self.push.web_push {
                if web_push.vapid_public_key.is_empty() || web_push.vapid_private_key.is_empty() {
                    return Err("VAPID keys are required for web push notifications".to_string());
                }
            }
        }

        if self.retry.max_attempts == 0 {
            return Err("Max retry attempts must be greater than 0".to_string());
        }

        if self.retry.backoff_multiplier <= 1.0 {
            return Err("Backoff multiplier must be greater than 1.0".to_string());
        }

        Ok(())
    }

    /// Get timeout duration for the specified operation
    pub fn get_timeout(&self, operation: &str) -> Duration {
        let seconds = match operation {
            "email" => self.email.timeout_seconds,
            "sms" => self.sms.timeout_seconds,
            "push" => self.push.timeout_seconds,
            "webhook" => self.webhook.timeout_seconds,
            "server" => self.server.timeout_seconds,
            _ => 30,
        };
        Duration::from_secs(seconds)
    }

    /// Check if a channel is enabled
    pub fn is_channel_enabled(&self, channel: &str) -> bool {
        match channel {
            "email" => self.email.enabled,
            "sms" => self.sms.enabled,
            "push" => self.push.enabled,
            "webhook" => self.webhook.enabled,
            "websocket" => self.websocket.enabled,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NotificationConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8086);
        assert!(config.email.enabled);
        assert!(!config.sms.enabled);
        assert!(!config.push.enabled);
        assert!(config.webhook.enabled);
        assert!(config.websocket.enabled);
    }

    #[test]
    fn test_config_validation() {
        let config = NotificationConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = config;
        invalid_config.server.port = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_timeout_getter() {
        let config = NotificationConfig::default();
        assert_eq!(config.get_timeout("email"), Duration::from_secs(30));
        assert_eq!(config.get_timeout("unknown"), Duration::from_secs(30));
    }

    #[test]
    fn test_channel_enabled_check() {
        let config = NotificationConfig::default();
        assert!(config.is_channel_enabled("email"));
        assert!(!config.is_channel_enabled("sms"));
        assert!(config.is_channel_enabled("webhook"));
        assert!(!config.is_channel_enabled("unknown"));
    }
}
