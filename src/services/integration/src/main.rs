//! Main binary entry point for the AI-CORE Integration Service
//!
//! This service provides comprehensive third-party API integrations including:
//! - Zapier webhook handling and workflow triggers
//! - Slack bot integration and workspace management
//! - GitHub repository integration and automated actions
//! - OAuth2 authentication flows
//! - Comprehensive security and monitoring

use integration_service::{IntegrationConfig, IntegrationService};
use std::process;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize tracing
    let result = init_tracing();
    if let Err(e) = result {
        eprintln!("Failed to initialize tracing: {}", e);
        process::exit(1);
    }

    info!(
        "Starting AI-CORE Integration Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let config = match IntegrationConfig::from_env() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        process::exit(1);
    }

    // Log enabled integrations
    log_enabled_integrations(&config);

    // Create and start the service
    let service = match IntegrationService::new(config).await {
        Ok(service) => {
            info!("Integration service initialized successfully");
            service
        }
        Err(e) => {
            error!("Failed to initialize service: {}", e);
            process::exit(1);
        }
    };

    // Start the service (this blocks until shutdown)
    if let Err(e) = service.start().await {
        error!("Service error: {}", e);
        process::exit(1);
    }

    info!("AI-CORE Integration Service shutdown complete");
}

/// Initialize tracing/logging
fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // Get log level from environment or default to info
    let log_level = std::env::var("INTEGRATION_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    // Get log format from environment or default to json
    let log_format = std::env::var("INTEGRATION_LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    // Create filter
    let filter = EnvFilter::try_new(&log_level).or_else(|_| EnvFilter::try_new("info"))?;

    match log_format.as_str() {
        "json" => {
            // JSON formatted logs for production
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        "pretty" | "text" => {
            // Pretty formatted logs for development
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .pretty()
                        .with_file(true)
                        .with_line_number(true)
                        .with_thread_ids(true)
                        .with_target(false),
                )
                .init();
        }
        _ => {
            // Default to compact format
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
    }

    Ok(())
}

/// Log information about enabled integrations
fn log_enabled_integrations(config: &IntegrationConfig) {
    let mut enabled_integrations = Vec::new();

    if config.zapier.enabled {
        enabled_integrations.push("Zapier");
        if config.zapier.webhook_secret.is_some() {
            info!("Zapier integration: ✓ Configured with webhook secret");
        } else {
            warn!("Zapier integration: ⚠️  No webhook secret configured");
        }
    }

    if config.slack.enabled {
        enabled_integrations.push("Slack");
        let has_bot_token = config.slack.bot_token.is_some();
        let has_signing_secret = config.slack.signing_secret.is_some();
        let has_oauth = config.slack.client_id.is_some() && config.slack.client_secret.is_some();

        if has_bot_token && has_signing_secret {
            info!("Slack integration: ✓ Fully configured");
        } else {
            warn!("Slack integration: ⚠️  Incomplete configuration");
            if !has_bot_token {
                warn!("  Missing: Bot token");
            }
            if !has_signing_secret {
                warn!("  Missing: Signing secret");
            }
        }

        if has_oauth {
            info!("Slack OAuth: ✓ Configured");
        } else {
            info!("Slack OAuth: ✗ Not configured");
        }

        if config.slack.socket_mode {
            info!("Slack Socket Mode: ✓ Enabled");
        }
    }

    if config.github.enabled {
        enabled_integrations.push("GitHub");
        let has_app_config = config.github.app_id.is_some() && config.github.private_key.is_some();
        let has_webhook_secret = config.github.webhook_secret.is_some();
        let has_oauth = config.github.client_id.is_some() && config.github.client_secret.is_some();

        if has_app_config && has_webhook_secret {
            info!("GitHub integration: ✓ Fully configured");
        } else {
            warn!("GitHub integration: ⚠️  Incomplete configuration");
            if !has_app_config {
                warn!("  Missing: App ID and private key");
            }
            if !has_webhook_secret {
                warn!("  Missing: Webhook secret");
            }
        }

        if has_oauth {
            info!("GitHub OAuth: ✓ Configured");
        } else {
            info!("GitHub OAuth: ✗ Not configured");
        }
    }

    if enabled_integrations.is_empty() {
        warn!("No integrations are enabled!");
    } else {
        info!("Enabled integrations: {}", enabled_integrations.join(", "));
    }

    // Log security settings
    if config.security.api_key_enabled {
        let key_count = config.security.api_keys.len();
        if key_count > 0 {
            info!("API Key authentication: ✓ Enabled ({} keys)", key_count);
        } else {
            warn!("API Key authentication: ⚠️  Enabled but no keys configured");
        }
    } else {
        info!("API Key authentication: ✗ Disabled");
    }

    if config.security.force_https {
        info!("HTTPS enforcement: ✓ Enabled");
    }

    if config.rate_limiting.enabled {
        info!(
            "Rate limiting: ✓ Enabled ({} req/s, burst: {})",
            config.rate_limiting.requests_per_second, config.rate_limiting.burst_size
        );
    } else {
        info!("Rate limiting: ✗ Disabled");
    }

    // Log observability settings
    if config.observability.metrics_enabled {
        info!(
            "Metrics collection: ✓ Enabled at {}",
            config.observability.metrics_path
        );
    }

    if config.observability.health_checks_enabled {
        info!(
            "Health checks: ✓ Enabled at {}",
            config.observability.health_path
        );
    }

    if config.observability.tracing.enabled {
        info!("Distributed tracing: ✓ Enabled");
        if let Some(ref endpoint) = config.observability.tracing.jaeger_endpoint {
            info!("Jaeger endpoint: {}", endpoint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing() {
        // Test that tracing initialization doesn't panic
        // Note: We can't actually test the full initialization in a unit test
        // because it sets global state, but we can ensure the function exists
        // and basic error handling works

        // This would normally fail in a test environment, but that's expected
        let _ = init_tracing();
    }

    #[test]
    fn test_log_enabled_integrations() {
        let config = IntegrationConfig::default();

        // This function should not panic with default config
        log_enabled_integrations(&config);
    }
}
