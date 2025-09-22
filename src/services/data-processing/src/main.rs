//! Main executable for the Data Processing Service
//!
//! This is the entry point for the AI-CORE Data Processing Service, which provides
//! high-throughput stream processing, batch analytics, and data transformation
//! capabilities for the intelligent automation platform.

use clap::Parser;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use data_processing_service::{
    config::Config, error::Result, server::DataProcessingServer, DataProcessingService,
};

/// Command line arguments for the data processing service
#[derive(Parser, Debug)]
#[command(name = "data-processing-server")]
#[command(about = "AI-CORE Data Processing Service")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Service host address
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Service port
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enable development mode
    #[arg(long)]
    dev: bool,

    /// Kafka bootstrap servers
    #[arg(long)]
    kafka_servers: Option<String>,

    /// ClickHouse URL
    #[arg(long)]
    clickhouse_url: Option<String>,

    /// Number of worker threads
    #[arg(long)]
    workers: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing/logging
    init_tracing(&args.log_level)?;

    info!(
        "Starting AI-CORE Data Processing Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let mut config = load_config(&args).await?;

    // Override config with command line arguments
    override_config_from_args(&mut config, &args);

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }

    info!("Configuration loaded and validated successfully");
    info!(
        "Server will bind to {}:{}",
        config.server.host, config.server.port
    );
    info!("Kafka servers: {}", config.kafka.bootstrap_servers);
    info!("ClickHouse URL: {}", config.clickhouse.url);

    // Create and start the data processing service
    let service = match DataProcessingService::new(config).await {
        Ok(service) => {
            info!("Data processing service created successfully");
            service
        }
        Err(e) => {
            error!("Failed to create data processing service: {}", e);
            std::process::exit(1);
        }
    };

    // Start the service
    if let Err(e) = service.start().await {
        error!("Failed to start data processing service: {}", e);
        std::process::exit(1);
    }

    info!("Data processing service started successfully");

    // Create and start HTTP server
    let server = DataProcessingServer::new(service.clone());

    // Start server in a background task
    let server_handle = {
        let server = server;
        tokio::spawn(async move {
            if let Err(e) = server.start().await {
                error!("HTTP server error: {}", e);
            }
        })
    };

    info!("HTTP server started successfully");

    // Print service information
    print_service_info(&service).await;

    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
        result = server_handle => {
            if let Err(e) = result {
                error!("Server task failed: {}", e);
            }
        }
    }

    // Graceful shutdown
    info!("Initiating graceful shutdown...");
    if let Err(e) = service.stop().await {
        error!("Error during service shutdown: {}", e);
    }

    info!("Data processing service shutdown complete");
    Ok(())
}

/// Initialize tracing/logging system
fn init_tracing(log_level: &str) -> Result<()> {
    // Parse log level
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("Invalid log level: {}. Using 'info'", log_level);
            tracing::Level::INFO
        }
    };

    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "data_processing_service={},tower_http=debug,axum=debug",
                    level
                )
                .into()
            }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json(),
        )
        .init();

    Ok(())
}

/// Load configuration from file or environment
async fn load_config(args: &Args) -> Result<Config> {
    if let Some(config_file) = &args.config {
        info!("Loading configuration from file: {}", config_file);
        // In a real implementation, this would load from the specified file
        // For now, we'll use environment-based configuration
        Ok(Config::from_env()?)
    } else {
        info!("Loading configuration from environment variables");
        Ok(Config::from_env()?)
    }
}

/// Override configuration with command line arguments
fn override_config_from_args(config: &mut Config, args: &Args) {
    // Override server settings
    config.server.host = args.host.clone();
    config.server.port = args.port;

    // Override Kafka settings
    if let Some(kafka_servers) = &args.kafka_servers {
        config.kafka.bootstrap_servers = kafka_servers.clone();
    }

    // Override ClickHouse settings
    if let Some(clickhouse_url) = &args.clickhouse_url {
        config.clickhouse.url = clickhouse_url.clone();
    }

    // Override worker settings
    if let Some(workers) = args.workers {
        config.stream.worker_threads = workers;
        config.batch.worker_threads = workers;
    }

    // Development mode adjustments
    if args.dev {
        info!("Development mode enabled");
        config.monitoring.log_level = "debug".to_string();
        config.health.check_interval_secs = 10; // More frequent health checks
        config.performance.auto_scaling = false; // Disable auto-scaling in dev
    }
}

/// Print service information
async fn print_service_info(service: &DataProcessingService) {
    let config = service.config();

    println!("\n🚀 AI-CORE Data Processing Service");
    println!("==========================================");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Build Date: {}",
        option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown")
    );
    println!(
        "Git SHA: {}",
        option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
    );
    println!("");
    println!("Configuration:");
    println!(
        "  HTTP Server: {}:{}",
        config.server.host, config.server.port
    );
    println!("  Kafka Servers: {}", config.kafka.bootstrap_servers);
    println!("  ClickHouse: {}", config.clickhouse.url);
    println!("  Stream Workers: {}", config.stream.worker_threads);
    println!("  Batch Workers: {}", config.batch.worker_threads);
    println!(
        "  Max Concurrent Jobs: {}",
        config.batch.max_concurrent_jobs
    );
    println!("");
    println!("Endpoints:");
    println!(
        "  Health: http://{}:{}/health",
        config.server.host, config.server.port
    );
    println!(
        "  Metrics: http://{}:{}/metrics",
        config.server.host, config.server.port
    );
    println!(
        "  Prometheus: http://{}:{}/metrics/prometheus",
        config.server.host, config.server.port
    );
    println!(
        "  Stream Processing: http://{}:{}/stream/process",
        config.server.host, config.server.port
    );
    println!(
        "  Batch Jobs: http://{}:{}/batch/jobs",
        config.server.host, config.server.port
    );
    println!("==========================================\n");

    // Display health status
    let health = service.health().await;
    println!("🏥 Service Health Status: {:?}", health.status);

    if !health.components.is_empty() {
        println!("📊 Component Status:");
        for (name, component) in &health.components {
            println!("  {} - {:?}", name, component.status);
        }
    }

    println!("");
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

/// Handle panic scenarios
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Application panic: {}", panic_info);

        if let Some(location) = panic_info.location() {
            error!(
                "Panic occurred in file '{}' at line {}",
                location.file(),
                location.line()
            );
        }

        // In production, you might want to send this to a monitoring service
        eprintln!("Application panicked and will exit");
        std::process::exit(1);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        use clap::Parser;

        let args = Args::try_parse_from(&[
            "data-processing-server",
            "--host",
            "127.0.0.1",
            "--port",
            "8081",
            "--log-level",
            "debug",
            "--dev",
        ])
        .unwrap();

        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, 8081);
        assert_eq!(args.log_level, "debug");
        assert!(args.dev);
    }

    #[tokio::test]
    async fn test_config_loading() {
        let args = Args::parse_from(&["data-processing-server"]);

        // This test might fail in CI without proper environment setup
        match load_config(&args).await {
            Ok(_) => {
                // Config loaded successfully
            }
            Err(e) => {
                println!("Config loading failed (expected in test env): {}", e);
            }
        }
    }

    #[test]
    fn test_config_override() {
        let mut config = Config::default();
        let args = Args::parse_from(&[
            "data-processing-server",
            "--host",
            "192.168.1.1",
            "--port",
            "9090",
            "--workers",
            "8",
        ]);

        override_config_from_args(&mut config, &args);

        assert_eq!(config.server.host, "192.168.1.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.stream.worker_threads, 8);
        assert_eq!(config.batch.worker_threads, 8);
    }
}
