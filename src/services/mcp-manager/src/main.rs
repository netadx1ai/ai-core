//! MCP Manager Service - Main Entry Point
//!
//! This service manages Model Context Protocol (MCP) servers, providing:
//! - Server registry and lifecycle management
//! - Health monitoring and load balancing
//! - Integration with Intent Parser Service
//! - MCP protocol communication and routing

use clap::Parser;
use dotenvy::dotenv;
use mcp_manager::{
    config::Config,
    server::McpManagerServer,
    telemetry::{init_tracing, shutdown_tracing},
};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use validator::Validate;

#[derive(Parser, Debug)]
#[command(
    name = "mcp-manager",
    about = "Model Context Protocol (MCP) server registry and management service",
    version = env!("CARGO_PKG_VERSION")
)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config/mcp-manager.yaml")]
    config: String,

    /// Service port (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Log level (overrides config)
    #[arg(short, long)]
    log_level: Option<String>,

    /// Enable development mode
    #[arg(long)]
    dev: bool,

    /// Validate configuration and exit
    #[arg(long)]
    validate_config: bool,
}

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    // Load configuration
    let mut config = match Config::from_file(&args.config).await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Apply command line overrides
    if let Some(port) = args.port {
        config.server.port = port;
    }

    if let Some(log_level) = args.log_level {
        config.logging.level = log_level;
    }

    if args.dev {
        config.environment = "development".to_string();
        config.logging.level = "debug".to_string();
    }

    // Validate configuration if requested
    if args.validate_config {
        if let Err(e) = config.validate() {
            eprintln!("Configuration validation failed: {}", e);
            std::process::exit(1);
        }
        println!("✅ Configuration is valid");
        return;
    }

    // Initialize tracing
    let _guard = match init_tracing(&config).await {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Failed to initialize tracing: {}", e);
            std::process::exit(1);
        }
    };

    info!(
        "Starting MCP Manager Service v{} in {} mode",
        env!("CARGO_PKG_VERSION"),
        config.environment
    );

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }

    // Create and start the server
    let server = Arc::new(match McpManagerServer::new(config).await {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to create server: {}", e);
            std::process::exit(1);
        }
    });

    // Setup graceful shutdown
    let server_clone = Arc::clone(&server);
    let shutdown_handle = tokio::spawn(async move {
        shutdown_signal().await;
        warn!("Shutdown signal received, initiating graceful shutdown...");

        if let Err(e) = server_clone.shutdown().await {
            error!("Error during shutdown: {}", e);
        }
    });

    // Start the server
    let result = tokio::select! {
        result = server.start() => {
            match result {
                Ok(_) => {
                    info!("MCP Manager Service stopped normally");
                    Ok(())
                }
                Err(e) => {
                    error!("MCP Manager Service error: {}", e);
                    Err(e)
                }
            }
        }
        _ = shutdown_handle => {
            info!("Shutdown completed");
            Ok(())
        }
    };

    // Cleanup tracing
    shutdown_tracing().await;

    if let Err(e) = result {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// Wait for shutdown signal (SIGINT, SIGTERM, or Ctrl+C)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }
}
