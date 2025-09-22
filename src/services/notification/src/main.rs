//! Main binary for the AI-CORE Notification Service
//!
//! This service provides multi-channel notification delivery including:
//! - Email notifications via SMTP
//! - SMS notifications via Twilio/AWS SNS
//! - Push notifications (Web Push/FCM)
//! - Webhook notifications via HTTP POST
//! - Real-time WebSocket notifications
//! - Template management and personalization
//! - Delivery tracking and retry mechanisms
//! - Subscription management

use notification_service::{
    config::NotificationConfig, manager::NotificationManager, routes::create_router,
    websocket::WebSocketManager, NotificationService,
};

use axum::serve;
use clap::{Arg, Command};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    init_tracing()?;

    // Parse command line arguments
    let matches = create_cli().get_matches();

    // Load configuration
    let config = load_config(&matches).await?;

    // Validate configuration
    config.validate().map_err(|e| {
        error!("Configuration validation failed: {}", e);
        e
    })?;

    info!("Starting AI-CORE Notification Service");
    info!(
        "Configuration: Server {}:{}",
        config.server.host, config.server.port
    );
    info!(
        "Enabled channels: Email={}, SMS={}, Push={}, Webhook={}, WebSocket={}",
        config.email.enabled,
        config.sms.enabled,
        config.push.enabled,
        config.webhook.enabled,
        config.websocket.enabled
    );

    // Create cancellation token for graceful shutdown
    let cancellation_token = CancellationToken::new();

    // Initialize notification manager
    let notification_manager = Arc::new(NotificationManager::new(config.clone()).await.map_err(
        |e| {
            error!("Failed to initialize notification manager: {}", e);
            e
        },
    )?);

    // Initialize WebSocket manager
    let websocket_manager = Arc::new(WebSocketManager::new().await.map_err(|e| {
        error!("Failed to initialize WebSocket manager: {}", e);
        e
    })?);

    // Start scheduler if enabled
    if config.scheduler.enabled {
        info!("Starting notification scheduler");
        if let Err(e) = notification_manager.start_scheduler().await {
            warn!(
                "Failed to start scheduler: {}, continuing without scheduler",
                e
            );
        } else {
            info!("Notification scheduler started successfully");
        }
    }

    // Create router
    let app = create_router(notification_manager.clone(), websocket_manager.clone());

    // Create socket address
    let addr = SocketAddr::new(
        config
            .server
            .host
            .parse()
            .map_err(|e| format!("Invalid host address: {}", e))?,
        config.server.port,
    );

    info!("Starting HTTP server on {}", addr);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        error!("Failed to bind to address {}: {}", addr, e);
        e
    })?;

    info!("Notification service started successfully on {}", addr);
    info!("Health check: http://{}/health", addr);
    info!("Metrics: http://{}/metrics", addr);
    info!("API Documentation: http://{}/api/v1", addr);
    info!("WebSocket endpoint: ws://{}/ws", addr);

    // Start background tasks
    let cleanup_task = start_cleanup_task(websocket_manager.clone(), cancellation_token.clone());

    // Start server with graceful shutdown
    let server_task = tokio::spawn({
        let cancellation_token = cancellation_token.clone();
        async move {
            let server = serve(listener, app);

            tokio::select! {
                result = server => {
                    if let Err(e) = result {
                        error!("Server error: {}", e);
                    }
                }
                _ = cancellation_token.cancelled() => {
                    info!("Server shutdown requested");
                }
            }
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown_signal().await;

    info!("Shutdown signal received, initiating graceful shutdown...");

    // Cancel all tasks
    cancellation_token.cancel();

    // Stop scheduler
    if config.scheduler.enabled {
        info!("Stopping notification scheduler");
        if let Err(e) = notification_manager.stop_scheduler().await {
            warn!("Failed to stop scheduler gracefully: {}", e);
        } else {
            info!("Notification scheduler stopped successfully");
        }
    }

    // Wait for server to shutdown
    if let Err(e) = server_task.await {
        error!("Server task error during shutdown: {}", e);
    }

    // Wait for cleanup task to finish
    cleanup_task.abort();

    info!("AI-CORE Notification Service stopped gracefully");
    Ok(())
}

/// Initialize tracing/logging
fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "notification_service=info,tower_http=info,axum=info,sqlx=warn,mongodb=warn,redis=warn"
            .into()
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_target(true))
        .init();

    Ok(())
}

/// Create CLI argument parser
fn create_cli() -> Command {
    Command::new("notification-server")
        .version("1.0.0")
        .about("AI-CORE Notification Service - Multi-channel notification delivery")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path"),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Server host address")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Server port")
                .default_value("8086"),
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .value_name("NUM")
                .help("Number of worker threads"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info"),
        )
}

/// Load configuration from file and environment
async fn load_config(
    matches: &clap::ArgMatches,
) -> Result<NotificationConfig, Box<dyn std::error::Error + Send + Sync>> {
    // Start with default configuration
    let mut config = if let Some(config_file) = matches.get_one::<String>("config") {
        info!("Loading configuration from file: {}", config_file);
        std::env::set_var("NOTIFICATION_CONFIG_FILE", config_file);
        NotificationConfig::from_env()
            .map_err(|e| format!("Failed to load configuration from file: {}", e))?
    } else {
        info!("Using default configuration with environment overrides");
        NotificationConfig::from_env().unwrap_or_else(|e| {
            warn!(
                "Failed to load configuration from environment: {}, using defaults",
                e
            );
            NotificationConfig::default()
        })
    };

    // Override with CLI arguments
    if let Some(host) = matches.get_one::<String>("host") {
        config.server.host = host.clone();
    }

    if let Some(port_str) = matches.get_one::<String>("port") {
        config.server.port = port_str
            .parse()
            .map_err(|e| format!("Invalid port number '{}': {}", port_str, e))?;
    }

    if let Some(workers_str) = matches.get_one::<String>("workers") {
        let workers: usize = workers_str
            .parse()
            .map_err(|e| format!("Invalid worker count '{}': {}", workers_str, e))?;
        config.server.workers = Some(workers);
    }

    Ok(config)
}

/// Start background cleanup tasks
fn start_cleanup_task(
    websocket_manager: Arc<WebSocketManager>,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Clean up stale WebSocket connections
                    let cleaned = websocket_manager.cleanup_stale_connections().await;
                    if cleaned > 0 {
                        info!("Cleaned up {} stale WebSocket connections", cleaned);
                    }

                    // Send periodic ping to all connections
                    let pinged = websocket_manager.ping_all_connections().await;
                    if pinged > 0 {
                        info!("Sent ping to {} WebSocket connections", pinged);
                    }
                }
                _ = cancellation_token.cancelled() => {
                    info!("Cleanup task shutting down");
                    break;
                }
            }
        }
    })
}

/// Wait for shutdown signals
async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cli() {
        let cli = create_cli();
        let matches = cli.try_get_matches_from(vec!["notification-server", "--port", "9090"]);
        assert!(matches.is_ok());

        let matches = matches.unwrap();
        assert_eq!(matches.get_one::<String>("port"), Some(&"9090".to_string()));
    }

    #[tokio::test]
    async fn test_load_default_config() {
        let cli = create_cli();
        let matches = cli.get_matches_from(vec!["notification-server"]);

        let config = load_config(&matches).await;
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.server.port, 8086);
        assert_eq!(config.server.host, "0.0.0.0");
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = NotificationConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid configuration
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_init_tracing() {
        // This test just ensures tracing initialization doesn't panic
        // In a real test environment, you might want to test specific log outputs
        let result = init_tracing();
        // Note: This might fail in test environments where tracing is already initialized
        // In practice, you'd handle this more gracefully
        match result {
            Ok(_) => println!("Tracing initialized successfully"),
            Err(e) => println!(
                "Tracing initialization failed (may be already initialized): {}",
                e
            ),
        }
    }

    #[tokio::test]
    async fn test_load_config_with_overrides() {
        let cli = create_cli();
        let matches = cli.get_matches_from(vec![
            "notification-server",
            "--host",
            "127.0.0.1",
            "--port",
            "9999",
            "--workers",
            "4",
        ]);

        let config = load_config(&matches).await.unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9999);
        assert_eq!(config.server.workers, Some(4));
    }

    #[tokio::test]
    async fn test_invalid_port_handling() {
        let cli = create_cli();
        let matches = cli.get_matches_from(vec!["notification-server", "--port", "invalid"]);

        let config = load_config(&matches).await;
        assert!(config.is_err());
    }

    #[tokio::test]
    async fn test_invalid_workers_handling() {
        let cli = create_cli();
        let matches = cli.get_matches_from(vec!["notification-server", "--workers", "invalid"]);

        let config = load_config(&matches).await;
        assert!(config.is_err());
    }
}
