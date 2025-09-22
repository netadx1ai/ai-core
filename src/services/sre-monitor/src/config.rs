use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub monitoring: MonitoringConfig,
    pub slo: SloConfig,
    pub error_budget: ErrorBudgetConfig,
    pub alerting: AlertingConfig,
    pub external_services: ExternalServicesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_request_size: usize,
    pub request_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub collection_interval: u64,
    pub metrics_retention_days: u32,
    pub batch_size: usize,
    pub enable_prometheus_export: bool,
    pub prometheus_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloConfig {
    pub validation_interval: u64,
    pub default_time_window: String,
    pub violation_cooldown: u64,
    pub enable_auto_remediation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudgetConfig {
    pub calculation_interval: u64,
    pub default_budget_percentage: f64,
    pub burn_rate_thresholds: BurnRateThresholds,
    pub alert_on_budget_exhaustion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRateThresholds {
    pub warning: f64,
    pub critical: f64,
    pub emergency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    pub enable_email_alerts: bool,
    pub enable_slack_alerts: bool,
    pub enable_webhook_alerts: bool,
    pub email_config: Option<EmailConfig>,
    pub slack_config: Option<SlackConfig>,
    pub webhook_config: Option<WebhookConfig>,
    pub alert_cooldown: u64,
    pub max_alerts_per_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub to_emails: Vec<String>,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub channel: String,
    pub username: String,
    pub icon_emoji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub timeout: u64,
    pub retry_count: u32,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServicesConfig {
    pub api_gateway_url: String,
    pub prometheus_url: String,
    pub grafana_url: String,
    pub jaeger_url: String,
    pub health_check_timeout: u64,
    pub health_check_interval: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://postgres:password@localhost:5432/ai_core_testing".to_string()
        });

        let server_port: u16 = env::var("SRE_MONITOR_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080);

        let bind_address = format!(
            "{}:{}",
            env::var("SRE_MONITOR_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port
        );

        Ok(Self {
            database_url,
            server: ServerConfig {
                bind_address,
                port: server_port,
                max_request_size: env::var("MAX_REQUEST_SIZE")
                    .unwrap_or_else(|_| "16777216".to_string()) // 16MB
                    .parse()
                    .unwrap_or(16777216),
                request_timeout: env::var("REQUEST_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
            database: DatabaseConfig {
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "20".to_string())
                    .parse()
                    .unwrap_or(20),
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
                connection_timeout: env::var("DB_CONNECTION_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                idle_timeout: env::var("DB_IDLE_TIMEOUT")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
                max_lifetime: env::var("DB_MAX_LIFETIME")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
            },
            monitoring: MonitoringConfig {
                collection_interval: env::var("MONITORING_COLLECTION_INTERVAL")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
                metrics_retention_days: env::var("METRICS_RETENTION_DAYS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                batch_size: env::var("METRICS_BATCH_SIZE")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
                enable_prometheus_export: env::var("ENABLE_PROMETHEUS_EXPORT")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                prometheus_endpoint: env::var("PROMETHEUS_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:9090".to_string()),
            },
            slo: SloConfig {
                validation_interval: env::var("SLO_VALIDATION_INTERVAL")
                    .unwrap_or_else(|_| "300".to_string()) // 5 minutes
                    .parse()
                    .unwrap_or(300),
                default_time_window: env::var("SLO_DEFAULT_TIME_WINDOW")
                    .unwrap_or_else(|_| "30d".to_string()),
                violation_cooldown: env::var("SLO_VIOLATION_COOLDOWN")
                    .unwrap_or_else(|_| "600".to_string()) // 10 minutes
                    .parse()
                    .unwrap_or(600),
                enable_auto_remediation: env::var("SLO_ENABLE_AUTO_REMEDIATION")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
            error_budget: ErrorBudgetConfig {
                calculation_interval: env::var("ERROR_BUDGET_CALCULATION_INTERVAL")
                    .unwrap_or_else(|_| "3600".to_string()) // 1 hour
                    .parse()
                    .unwrap_or(3600),
                default_budget_percentage: env::var("ERROR_BUDGET_DEFAULT_PERCENTAGE")
                    .unwrap_or_else(|_| "1.0".to_string())
                    .parse()
                    .unwrap_or(1.0),
                burn_rate_thresholds: BurnRateThresholds {
                    warning: env::var("BURN_RATE_WARNING_THRESHOLD")
                        .unwrap_or_else(|_| "2.0".to_string())
                        .parse()
                        .unwrap_or(2.0),
                    critical: env::var("BURN_RATE_CRITICAL_THRESHOLD")
                        .unwrap_or_else(|_| "5.0".to_string())
                        .parse()
                        .unwrap_or(5.0),
                    emergency: env::var("BURN_RATE_EMERGENCY_THRESHOLD")
                        .unwrap_or_else(|_| "10.0".to_string())
                        .parse()
                        .unwrap_or(10.0),
                },
                alert_on_budget_exhaustion: env::var("ALERT_ON_BUDGET_EXHAUSTION")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            alerting: AlertingConfig {
                enable_email_alerts: env::var("ENABLE_EMAIL_ALERTS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                enable_slack_alerts: env::var("ENABLE_SLACK_ALERTS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                enable_webhook_alerts: env::var("ENABLE_WEBHOOK_ALERTS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                email_config: Self::load_email_config(),
                slack_config: Self::load_slack_config(),
                webhook_config: Self::load_webhook_config(),
                alert_cooldown: env::var("ALERT_COOLDOWN")
                    .unwrap_or_else(|_| "1800".to_string()) // 30 minutes
                    .parse()
                    .unwrap_or(1800),
                max_alerts_per_hour: env::var("MAX_ALERTS_PER_HOUR")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
            },
            external_services: ExternalServicesConfig {
                api_gateway_url: env::var("API_GATEWAY_URL")
                    .unwrap_or_else(|_| "http://localhost:8000".to_string()),
                prometheus_url: env::var("PROMETHEUS_URL")
                    .unwrap_or_else(|_| "http://localhost:9090".to_string()),
                grafana_url: env::var("GRAFANA_URL")
                    .unwrap_or_else(|_| "http://localhost:3001".to_string()),
                jaeger_url: env::var("JAEGER_URL")
                    .unwrap_or_else(|_| "http://localhost:14268".to_string()),
                health_check_timeout: env::var("HEALTH_CHECK_TIMEOUT")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                health_check_interval: env::var("HEALTH_CHECK_INTERVAL")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
        })
    }

    fn load_email_config() -> Option<EmailConfig> {
        if env::var("ENABLE_EMAIL_ALERTS").unwrap_or_else(|_| "false".to_string()) == "true" {
            Some(EmailConfig {
                smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string()),
                smtp_port: env::var("SMTP_PORT")
                    .unwrap_or_else(|_| "587".to_string())
                    .parse()
                    .unwrap_or(587),
                smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
                smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
                from_email: env::var("FROM_EMAIL").unwrap_or_default(),
                to_emails: env::var("TO_EMAILS")
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                use_tls: env::var("SMTP_USE_TLS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            })
        } else {
            None
        }
    }

    fn load_slack_config() -> Option<SlackConfig> {
        if env::var("ENABLE_SLACK_ALERTS").unwrap_or_else(|_| "false".to_string()) == "true" {
            Some(SlackConfig {
                webhook_url: env::var("SLACK_WEBHOOK_URL").unwrap_or_default(),
                channel: env::var("SLACK_CHANNEL").unwrap_or_else(|_| "#alerts".to_string()),
                username: env::var("SLACK_USERNAME").unwrap_or_else(|_| "SRE Monitor".to_string()),
                icon_emoji: env::var("SLACK_ICON_EMOJI")
                    .unwrap_or_else(|_| ":warning:".to_string()),
            })
        } else {
            None
        }
    }

    fn load_webhook_config() -> Option<WebhookConfig> {
        if env::var("ENABLE_WEBHOOK_ALERTS").unwrap_or_else(|_| "false".to_string()) == "true" {
            let mut headers = std::collections::HashMap::new();

            // Load custom headers from environment
            if let Ok(headers_str) = env::var("WEBHOOK_HEADERS") {
                for header in headers_str.split(',') {
                    if let Some((key, value)) = header.split_once(':') {
                        headers.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }

            Some(WebhookConfig {
                url: env::var("WEBHOOK_URL").unwrap_or_default(),
                timeout: env::var("WEBHOOK_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                retry_count: env::var("WEBHOOK_RETRY_COUNT")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                headers,
            })
        } else {
            None
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.database_url.is_empty() {
            return Err(anyhow::anyhow!("DATABASE_URL is required"));
        }

        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port must be greater than 0"));
        }

        if self.database.max_connections == 0 {
            return Err(anyhow::anyhow!(
                "Database max_connections must be greater than 0"
            ));
        }

        if self.monitoring.collection_interval == 0 {
            return Err(anyhow::anyhow!(
                "Monitoring collection_interval must be greater than 0"
            ));
        }

        if self.error_budget.default_budget_percentage < 0.0
            || self.error_budget.default_budget_percentage > 100.0
        {
            return Err(anyhow::anyhow!(
                "Error budget percentage must be between 0 and 100"
            ));
        }

        // Validate burn rate thresholds
        if self.error_budget.burn_rate_thresholds.warning
            >= self.error_budget.burn_rate_thresholds.critical
        {
            return Err(anyhow::anyhow!(
                "Warning burn rate threshold must be less than critical threshold"
            ));
        }

        if self.error_budget.burn_rate_thresholds.critical
            >= self.error_budget.burn_rate_thresholds.emergency
        {
            return Err(anyhow::anyhow!(
                "Critical burn rate threshold must be less than emergency threshold"
            ));
        }

        // Validate alerting configuration
        if self.alerting.enable_email_alerts && self.alerting.email_config.is_none() {
            return Err(anyhow::anyhow!(
                "Email alerts enabled but email config is missing"
            ));
        }

        if self.alerting.enable_slack_alerts && self.alerting.slack_config.is_none() {
            return Err(anyhow::anyhow!(
                "Slack alerts enabled but slack config is missing"
            ));
        }

        if self.alerting.enable_webhook_alerts && self.alerting.webhook_config.is_none() {
            return Err(anyhow::anyhow!(
                "Webhook alerts enabled but webhook config is missing"
            ));
        }

        Ok(())
    }

    pub fn get_database_connection_string(&self) -> &str {
        &self.database_url
    }

    pub fn is_development(&self) -> bool {
        env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) == "development"
    }

    pub fn is_production(&self) -> bool {
        env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) == "production"
    }

    pub fn get_log_level(&self) -> String {
        env::var("RUST_LOG").unwrap_or_else(|_| {
            if self.is_development() {
                "debug".to_string()
            } else {
                "info".to_string()
            }
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "postgresql://postgres:password@localhost:5432/ai_core_testing"
                .to_string(),
            server: ServerConfig {
                bind_address: "0.0.0.0:8080".to_string(),
                port: 8080,
                max_request_size: 16777216, // 16MB
                request_timeout: 30,
            },
            database: DatabaseConfig {
                max_connections: 20,
                min_connections: 5,
                connection_timeout: 30,
                idle_timeout: 300,
                max_lifetime: 3600,
            },
            monitoring: MonitoringConfig {
                collection_interval: 60,
                metrics_retention_days: 30,
                batch_size: 1000,
                enable_prometheus_export: true,
                prometheus_endpoint: "http://localhost:9090".to_string(),
            },
            slo: SloConfig {
                validation_interval: 300,
                default_time_window: "30d".to_string(),
                violation_cooldown: 600,
                enable_auto_remediation: false,
            },
            error_budget: ErrorBudgetConfig {
                calculation_interval: 3600,
                default_budget_percentage: 1.0,
                burn_rate_thresholds: BurnRateThresholds {
                    warning: 2.0,
                    critical: 5.0,
                    emergency: 10.0,
                },
                alert_on_budget_exhaustion: true,
            },
            alerting: AlertingConfig {
                enable_email_alerts: false,
                enable_slack_alerts: false,
                enable_webhook_alerts: false,
                email_config: None,
                slack_config: None,
                webhook_config: None,
                alert_cooldown: 1800,
                max_alerts_per_hour: 10,
            },
            external_services: ExternalServicesConfig {
                api_gateway_url: "http://localhost:8000".to_string(),
                prometheus_url: "http://localhost:9090".to_string(),
                grafana_url: "http://localhost:3001".to_string(),
                jaeger_url: "http://localhost:14268".to_string(),
                health_check_timeout: 10,
                health_check_interval: 30,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_from_env() {
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/test");
        env::set_var("SRE_MONITOR_PORT", "9080");

        let config = Config::from_env().unwrap();
        assert_eq!(
            config.database_url,
            "postgresql://test:test@localhost:5432/test"
        );
        assert_eq!(config.server.port, 9080);
    }

    #[test]
    fn test_invalid_burn_rate_thresholds() {
        let mut config = Config::default();
        config.error_budget.burn_rate_thresholds.warning = 10.0;
        config.error_budget.burn_rate_thresholds.critical = 5.0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_budget_percentage() {
        let mut config = Config::default();
        config.error_budget.default_budget_percentage = 150.0;

        assert!(config.validate().is_err());
    }
}
