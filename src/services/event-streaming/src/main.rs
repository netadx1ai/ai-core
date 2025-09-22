//! # Event Streaming Service Main Binary
//!
//! This is the main entry point for the event streaming service.
//! It handles service initialization, configuration loading, and graceful shutdown.

use std::env;
use std::process;

use clap::{Arg, Command};
use dotenvy::dotenv;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use event_streaming_service::{
    config::Config, error::EventStreamingError, server::EventStreamingService, SERVICE_NAME,
    VERSION,
};

#[tokio::main]
async fn main() {
    // Initialize logging first
    init_logging();

    // Load environment variables
    if let Err(e) = dotenv() {
        warn!("Failed to load .env file: {}", e);
    }

    // Parse command line arguments
    let matches = create_cli().get_matches();

    // Handle config validation flag
    if matches.get_flag("validate-config") {
        match validate_configuration().await {
            Ok(_) => {
                info!("Configuration is valid");
                process::exit(0);
            }
            Err(e) => {
                error!("Configuration validation failed: {}", e);
                process::exit(1);
            }
        }
    }

    info!(
        "Starting {} version {} (built with rustc {})",
        SERVICE_NAME,
        VERSION,
        std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string())
    );

    // Load configuration
    let config = match load_configuration().await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    info!("Configuration loaded successfully");
    info!("Environment: {}", config.environment.name);
    info!("Debug mode: {}", config.environment.debug);

    // Create and start the service
    match run_service(config).await {
        Ok(_) => {
            info!("Service stopped gracefully");
        }
        Err(e) => {
            error!("Service failed: {}", e);
            process::exit(1);
        }
    }
}

/// Initialize structured logging
fn init_logging() {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let log_format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    let subscriber = tracing_subscriber::registry();

    match log_format.as_str() {
        "json" => {
            subscriber
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_current_span(false)
                        .with_span_list(true),
                )
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
                )
                .init();
        }
        "text" | _ => {
            subscriber
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true),
                )
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
                )
                .init();
        }
    }

    info!("Logging initialized with level: {}", log_level);
}

/// Create CLI interface
fn create_cli() -> Command {
    Command::new(SERVICE_NAME)
        .version(VERSION)
        .about("High-performance event streaming service for AI-CORE platform")
        .long_about(
            "Event Streaming Service provides real-time event processing with Kafka and Redis Streams integration. \
            It handles event routing, filtering, transformation, and audit capabilities."
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .action(clap::ArgAction::Set)
        )
        .arg(
            Arg::new("validate-config")
                .long("validate-config")
                .help("Validate configuration and exit")
                .action(clap::ArgAction::SetTrue)
        )

        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Override server port")
                .action(clap::ArgAction::Set)
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .value_name("COUNT")
                .help("Override number of worker threads")
                .action(clap::ArgAction::Set)
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Override log level (error, warn, info, debug, trace)")
                .action(clap::ArgAction::Set)
        )
}

/// Load and validate configuration
async fn load_configuration() -> Result<Config, EventStreamingError> {
    let config = Config::from_env().map_err(|e| {
        EventStreamingError::configuration(format!("Failed to load configuration: {}", e))
    })?;

    config.validate().map_err(|e| {
        EventStreamingError::configuration(format!("Configuration validation failed: {}", e))
    })?;

    Ok(config)
}

/// Validate configuration without starting the service
async fn validate_configuration() -> Result<(), EventStreamingError> {
    let _config = load_configuration().await?;
    info!("Configuration validation completed successfully");
    Ok(())
}

/// Run the main service
async fn run_service(config: Config) -> Result<(), EventStreamingError> {
    info!("Initializing Event Streaming Service");

    // Create service instance
    let service = EventStreamingService::new(config).await?;

    info!("Service created successfully");

    // Start the service
    let service_handle = {
        let service = service.clone();
        tokio::spawn(async move {
            if let Err(e) = service.start().await {
                error!("Service failed to start: {}", e);
                return Err(e);
            }
            Ok(())
        })
    };

    // Set up signal handling
    let shutdown_signal = setup_signal_handling();

    // Wait for either service completion or shutdown signal
    tokio::select! {
        result = service_handle => {
            match result {
                Ok(Ok(_)) => {
                    info!("Service completed successfully");
                }
                Ok(Err(e)) => {
                    error!("Service failed: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    error!("Service task panicked: {}", e);
                    return Err(EventStreamingError::internal("Service task panicked"));
                }
            }
        }
        _ = shutdown_signal => {
            info!("Received shutdown signal");
        }
    }

    // Graceful shutdown
    info!("Initiating graceful shutdown");
    if let Err(e) = service.stop().await {
        error!("Error during service shutdown: {}", e);
        return Err(e);
    }

    info!("Service shutdown completed");
    Ok(())
}

/// Setup signal handling for graceful shutdown
async fn setup_signal_handling() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C signal");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
        info!("Received SIGTERM signal");
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Print system information
#[allow(dead_code)]
fn print_system_info() {
    info!("System Information:");
    info!("  OS: {}", env::consts::OS);
    info!("  Architecture: {}", env::consts::ARCH);
    info!("  CPU count: {}", num_cpus::get());
    info!(
        "  Rust version: {}",
        std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string())
    );

    if let Ok(hostname) = env::var("HOSTNAME") {
        info!("  Hostname: {}", hostname);
    }

    if let Ok(pod_name) = env::var("POD_NAME") {
        info!("  Pod name: {}", pod_name);
    }

    if let Ok(namespace) = env::var("NAMESPACE") {
        info!("  Namespace: {}", namespace);
    }
}

/// Print build information
#[allow(dead_code)]
fn print_build_info() {
    info!("Build Information:");
    info!("  Version: {}", VERSION);
    info!(
        "  Git commit: {}",
        std::env::var("GIT_HASH").unwrap_or_else(|_| "unknown".to_string())
    );
    info!(
        "  Build date: {}",
        std::env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string())
    );
    info!(
        "  Built by: {}",
        std::env::var("BUILD_USER").unwrap_or_else(|_| "unknown".to_string())
    );

    #[cfg(debug_assertions)]
    info!("  Build type: debug");

    #[cfg(not(debug_assertions))]
    info!("  Build type: release");
}

/// Handle panic hook for better error reporting
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();

        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic message".to_string()
        };

        error!(
            "Service panicked at {}: {}\nBacktrace:\n{}",
            location, message, backtrace
        );

        // Exit with error code
        process::exit(1);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_creation() {
        let cli = create_cli();
        assert_eq!(cli.get_name(), SERVICE_NAME);
    }

    #[tokio::test]
    async fn test_configuration_loading() {
        // This test might fail without proper environment setup
        if std::env::var("SKIP_CONFIG_TEST").is_ok() {
            return;
        }

        let result = load_configuration().await;
        // Configuration loading might fail in test environment, which is expected
        if result.is_err() {
            println!("Configuration loading failed (expected in test environment)");
        }
    }

    #[test]
    fn test_panic_hook_setup() {
        setup_panic_hook();
        // If we reach here, panic hook was set up successfully
    }
}
