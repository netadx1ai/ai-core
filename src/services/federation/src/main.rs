//! Federation Service Main Binary
//!
//! This is the main entry point for the Federation Service, providing comprehensive
//! multi-tenant client management, provider selection, schema translation, workflow
//! execution, and MCP server integration capabilities.

use anyhow::{Context, Result};
use clap::{Arg, Command};
use federation::{Config, FederationService};
use std::env;
use std::path::PathBuf;
use tokio::signal;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("federation")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Federation service for multi-tenant client management and provider orchestration")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/federation.yaml"),
        )
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .value_name("HOST")
                .help("Server host (overrides config)"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Server port (overrides config)"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (overrides config)")
                .value_parser(["trace", "debug", "info", "warn", "error"]),
        )
        .arg(
            Arg::new("validate-config")
                .long("validate-config")
                .help("Validate configuration and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dev")
                .long("dev")
                .help("Enable development mode")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize basic logging for startup
    init_basic_logging();

    // Load configuration
    let config_path = PathBuf::from(matches.get_one::<String>("config").unwrap());
    info!("Loading configuration from: {}", config_path.display());

    let mut config = if config_path.exists() {
        Config::from_file(&config_path)
            .await
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        warn!("Configuration file not found, using default configuration");
        Config::default()
    };

    // Apply command line overrides
    let mut overrides = federation::config::ConfigOverrides::default();

    if let Some(host) = matches.get_one::<String>("host") {
        overrides.host = Some(host.clone());
    }

    if let Some(port_str) = matches.get_one::<String>("port") {
        overrides.port = Some(
            port_str
                .parse()
                .with_context(|| format!("Invalid port number: {}", port_str))?,
        );
    }

    if let Some(log_level) = matches.get_one::<String>("log-level") {
        overrides.log_level = Some(log_level.clone());
    }

    // Apply environment variable overrides
    if let Ok(db_url) = env::var("DATABASE_URL") {
        overrides.database_url = Some(db_url);
    }

    if let Ok(redis_url) = env::var("REDIS_URL") {
        overrides.redis_url = Some(redis_url);
    }

    // Merge overrides
    config = config.merge_with_overrides(overrides)?;

    // Set development mode if requested
    if matches.get_flag("dev") {
        config = config.for_environment(federation::config::Environment::Development);
        info!("Running in development mode");
    }

    // Validate configuration if requested
    if matches.get_flag("validate-config") {
        info!("Validating configuration...");
        config.validate()?;
        println!("✅ Configuration is valid");
        return Ok(());
    }

    // Display startup information
    print_startup_banner();
    info!("Starting Federation Service v{}", env!("CARGO_PKG_VERSION"));
    info!(
        "Server will bind to {}:{}",
        config.server.host, config.server.port
    );
    info!("Database URL: {}", mask_sensitive_url(&config.database.url));
    info!("Redis URL: {}", mask_sensitive_url(&config.redis.url));
    info!("Temporal URL: {}", config.temporal.server_url);
    info!("Environment: {:?}", config.environment);

    // Create and start the federation service
    let service = FederationService::new(config)
        .await
        .context("Failed to initialize Federation Service")?;

    info!("Federation Service initialized successfully");

    // Setup graceful shutdown
    let service_clone = service.clone();
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        info!("Shutdown signal received, stopping Federation Service...");

        if let Err(e) = service_clone.stop().await {
            error!("Error during service shutdown: {}", e);
        } else {
            info!("Federation Service stopped gracefully");
        }

        std::process::exit(0);
    });

    // Start the service (this will block until shutdown)
    info!("🚀 Federation Service starting up...");

    match service.start().await {
        Ok(()) => {
            info!("Federation Service has been stopped");
            Ok(())
        }
        Err(e) => {
            error!("Federation Service failed: {}", e);
            Err(e.into())
        }
    }
}

/// Initialize basic logging for startup messages
fn init_basic_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).compact())
        .with(env_filter)
        .init();
}

/// Print startup banner
fn print_startup_banner() {
    println!(
        r#"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║    ███████╗███████╗██████╗ ███████╗██████╗  █████╗ ████████╗  ║
║    ██╔════╝██╔════╝██╔══██╗██╔════╝██╔══██╗██╔══██╗╚══██╔══╝  ║
║    █████╗  █████╗  ██║  ██║█████╗  ██████╔╝███████║   ██║     ║
║    ██╔══╝  ██╔══╝  ██║  ██║██╔══╝  ██╔══██╗██╔══██║   ██║     ║
║    ██║     ███████╗██████╔╝███████╗██║  ██║██║  ██║   ██║     ║
║    ╚═╝     ╚══════╝╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝     ║
║                                                               ║
║              AI-CORE Federation Service v{}                ║
║                                                               ║
║    Multi-tenant client management and provider orchestration ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
"#,
        env!("CARGO_PKG_VERSION")
    );
}

/// Wait for shutdown signal (SIGTERM, SIGINT, or Ctrl+C)
async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C");
        },
        _ = terminate => {
            info!("Received SIGTERM");
        },
    }
}

/// Mask sensitive parts of URLs for logging
fn mask_sensitive_url(url: &str) -> String {
    if let Ok(parsed_url) = url::Url::parse(url) {
        let mut masked = parsed_url.clone();

        // Mask password if present
        if parsed_url.password().is_some() {
            let _ = masked.set_password(Some("***"));
        }

        // Mask username if it looks like a token or key
        let username = parsed_url.username();
        if !username.is_empty()
            && (username.len() > 10 || username.contains("key") || username.contains("token"))
        {
            let _ = masked.set_username("***");
        }

        masked.to_string()
    } else {
        // If URL parsing fails, just mask anything that looks like credentials
        let mut masked = url.to_string();

        // Simple pattern to mask potential credentials
        if let Some(at_pos) = masked.find('@') {
            if let Some(scheme_end) = masked.find("://") {
                let credentials_start = scheme_end + 3;
                if credentials_start < at_pos {
                    masked.replace_range(credentials_start..at_pos, "***");
                }
            }
        }

        masked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_sensitive_url() {
        // Test PostgreSQL URL with password
        let postgres_url = "postgresql://user:password123@localhost:5432/database";
        let masked = mask_sensitive_url(postgres_url);
        assert!(!masked.contains("password123"));
        assert!(masked.contains("***"));

        // Test Redis URL with password
        let redis_url = "redis://:secret_key@localhost:6379";
        let masked = mask_sensitive_url(redis_url);
        assert!(!masked.contains("secret_key"));

        // Test URL without credentials
        let clean_url = "http://localhost:8080/health";
        let masked = mask_sensitive_url(clean_url);
        assert_eq!(masked, clean_url);

        // Test malformed URL
        let malformed = "not-a-url://user:pass@host";
        let masked = mask_sensitive_url(malformed);
        assert!(!masked.contains("user:pass"));
    }

    #[test]
    fn test_startup_banner() {
        // Just ensure it doesn't panic
        print_startup_banner();
    }
}
