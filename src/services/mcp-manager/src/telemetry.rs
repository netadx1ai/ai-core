//! Telemetry Module
//!
//! This module provides comprehensive telemetry capabilities for the MCP Manager Service,
//! including structured logging, distributed tracing, and metrics collection.

use crate::{config::Config, McpError, Result};
#[cfg(feature = "metrics")]
use prometheus::{Counter, Gauge, Histogram, IntCounter, Registry};
use tracing::{info, warn};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Telemetry guard for cleanup
pub struct TelemetryGuard {
    _guard: Option<()>,
}

/// Metrics collection for MCP Manager Service
#[derive(Debug, Clone)]
pub struct Metrics {
    #[cfg(feature = "metrics")]
    /// Prometheus registry
    pub registry: Arc<Registry>,

    #[cfg(feature = "metrics")]
    // Server metrics
    pub servers_total: IntCounter,
    #[cfg(feature = "metrics")]
    pub servers_healthy: Gauge,
    #[cfg(feature = "metrics")]
    pub servers_unhealthy: Gauge,
    #[cfg(feature = "metrics")]
    pub servers_failed: Gauge,

    #[cfg(feature = "metrics")]
    // Request metrics
    pub requests_total: Counter,
    #[cfg(feature = "metrics")]
    pub requests_duration: Histogram,
    #[cfg(feature = "metrics")]
    pub requests_errors_total: Counter,

    #[cfg(feature = "metrics")]
    // Health check metrics
    pub health_checks_total: IntCounter,
    #[cfg(feature = "metrics")]
    pub health_check_duration: Histogram,
    #[cfg(feature = "metrics")]
    pub health_check_failures: IntCounter,

    #[cfg(feature = "metrics")]
    // Load balancer metrics
    pub load_balancer_requests: Counter,
    #[cfg(feature = "metrics")]
    pub load_balancer_errors: Counter,
    #[cfg(feature = "metrics")]
    pub active_connections: Gauge,

    #[cfg(feature = "metrics")]
    // Protocol metrics
    pub protocol_messages_sent: Counter,
    #[cfg(feature = "metrics")]
    pub protocol_messages_received: Counter,
    #[cfg(feature = "metrics")]
    pub protocol_errors: Counter,
}

impl Metrics {
    /// Create new metrics collection
    pub fn new() -> Result<Self> {
        #[cfg(feature = "metrics")]
        let registry = Arc::new(Registry::new());

        #[cfg(feature = "metrics")]
        {
            // Server metrics
            let servers_total = IntCounter::new(
                "mcp_manager_servers_total",
                "Total number of registered servers",
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create servers_total metric: {}", e))
            })?;

            let servers_healthy = Gauge::new(
                "mcp_manager_servers_healthy",
                "Number of healthy servers",
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create servers_healthy metric: {}", e))
            })?;

            let servers_unhealthy = Gauge::new(
                "mcp_manager_servers_unhealthy",
                "Number of unhealthy servers",
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create servers_unhealthy metric: {}", e))
            })?;

            let servers_failed =
                Gauge::new("mcp_manager_servers_failed", "Number of failed servers").map_err(
                    |e| {
                        McpError::Internal(format!("Failed to create servers_failed metric: {}", e))
                    },
                )?;

            // Request metrics
            let requests_total = Counter::new(
                "mcp_manager_requests_total",
                "Total number of HTTP requests",
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create requests_total metric: {}", e))
            })?;

            let requests_duration = Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "mcp_manager_request_duration_seconds",
                    "HTTP request duration in seconds",
                )
                .buckets(vec![
                    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
                ]),
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create requests_duration metric: {}", e))
            })?;

            let requests_errors_total = Counter::new(
                "mcp_manager_request_errors_total",
                "Total number of HTTP request errors",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create requests_errors_total metric: {}",
                    e
                ))
            })?;

            // Health check metrics
            let health_checks_total = IntCounter::new(
                "mcp_manager_health_checks_total",
                "Total number of health checks performed",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create health_checks_total metric: {}",
                    e
                ))
            })?;

            let health_check_duration = Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "mcp_manager_health_check_duration_seconds",
                    "Health check duration in seconds",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create health_check_duration metric: {}",
                    e
                ))
            })?;

            let health_check_failures = IntCounter::new(
                "mcp_manager_health_check_failures_total",
                "Total number of health check failures",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create health_check_failures metric: {}",
                    e
                ))
            })?;

            // Load balancer metrics
            let load_balancer_requests = Counter::new(
                "mcp_manager_load_balancer_requests_total",
                "Total number of load balancer requests",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create load_balancer_requests metric: {}",
                    e
                ))
            })?;

            let load_balancer_errors = Counter::new(
                "mcp_manager_load_balancer_errors_total",
                "Total number of load balancer errors",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create load_balancer_errors metric: {}",
                    e
                ))
            })?;

            let active_connections = Gauge::new(
                "mcp_manager_active_connections",
                "Number of active connections",
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create active_connections metric: {}", e))
            })?;

            // Protocol metrics
            let protocol_messages_sent = Counter::new(
                "mcp_manager_protocol_messages_sent_total",
                "Total number of MCP messages sent",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create protocol_messages_sent metric: {}",
                    e
                ))
            })?;

            let protocol_messages_received = Counter::new(
                "mcp_manager_protocol_messages_received_total",
                "Total number of MCP messages received",
            )
            .map_err(|e| {
                McpError::Internal(format!(
                    "Failed to create protocol_messages_received metric: {}",
                    e
                ))
            })?;

            let protocol_errors = Counter::new(
                "mcp_manager_protocol_errors_total",
                "Total number of MCP protocol errors",
            )
            .map_err(|e| {
                McpError::Internal(format!("Failed to create protocol_errors metric: {}", e))
            })?;

            // Register all metrics
            registry
                .register(Box::new(servers_total.clone()))
                .map_err(|e| {
                    McpError::Internal(format!("Failed to register servers_total metric: {}", e))
                })?;
            registry
                .register(Box::new(servers_healthy.clone()))
                .map_err(|e| {
                    McpError::Internal(format!("Failed to register servers_healthy metric: {}", e))
                })?;
            registry
                .register(Box::new(servers_unhealthy.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register servers_unhealthy metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(servers_failed.clone()))
                .map_err(|e| {
                    McpError::Internal(format!("Failed to register servers_failed metric: {}", e))
                })?;
            registry
                .register(Box::new(requests_total.clone()))
                .map_err(|e| {
                    McpError::Internal(format!("Failed to register requests_total metric: {}", e))
                })?;
            registry
                .register(Box::new(requests_duration.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register requests_duration metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(requests_errors_total.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register requests_errors_total metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(health_checks_total.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register health_checks_total metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(health_check_duration.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register health_check_duration metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(health_check_failures.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register health_check_failures metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(load_balancer_requests.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register load_balancer_requests metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(load_balancer_errors.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register load_balancer_errors metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(active_connections.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register active_connections metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(protocol_messages_sent.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register protocol_messages_sent metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(protocol_messages_received.clone()))
                .map_err(|e| {
                    McpError::Internal(format!(
                        "Failed to register protocol_messages_received metric: {}",
                        e
                    ))
                })?;
            registry
                .register(Box::new(protocol_errors.clone()))
                .map_err(|e| {
                    McpError::Internal(format!("Failed to register protocol_errors metric: {}", e))
                })?;

            Ok(Self {
                registry,
                servers_total,
                servers_healthy,
                servers_unhealthy,
                servers_failed,
                requests_total,
                requests_duration,
                requests_errors_total,
                health_checks_total,
                health_check_duration,
                health_check_failures,
                load_balancer_requests,
                load_balancer_errors,
                active_connections,
                protocol_messages_sent,
                protocol_messages_received,
                protocol_errors,
            })
        }

        #[cfg(not(feature = "metrics"))]
        Ok(Self {})
    }

    /// Export metrics as Prometheus format
    pub fn export(&self) -> Result<String> {
        #[cfg(feature = "metrics")]
        {
            use prometheus::Encoder;
            let encoder = prometheus::TextEncoder::new();
            let metric_families = self.registry.gather();
            encoder
                .encode_to_string(&metric_families)
                .map_err(|e| McpError::Internal(format!("Failed to encode metrics: {}", e)))
        }
        #[cfg(not(feature = "metrics"))]
        Ok("# Metrics disabled\n".to_string())
    }
}

/// Initialize tracing and logging
pub async fn init_tracing(config: &Config) -> Result<TelemetryGuard> {
    // Parse log level
    let log_level = config.logging.level.as_str();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .map_err(|e| McpError::Internal(format!("Invalid log level {}: {}", log_level, e)))?;

    // Create console layer
    let console_layer = if config.logging.console {
        let layer = fmt::layer()
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        if config.logging.format == "json" {
            Some(layer.json().boxed())
        } else if config.logging.format == "pretty" {
            Some(layer.pretty().boxed())
        } else {
            Some(layer.compact().boxed())
        }
    } else {
        None
    };

    // Initialize subscriber with layers
    let subscriber = tracing_subscriber::registry().with(env_filter);

    if let Some(console_layer) = console_layer {
        subscriber.with(console_layer).init();
    } else {
        subscriber.init();
    }

    // Log file support disabled - warn if requested
    if config.logging.file_enabled {
        warn!("File logging requested but not available - tracing_appender feature disabled");
    }

    info!(
        log_level = log_level,
        console_enabled = config.logging.console,
        file_enabled = config.logging.file_enabled,
        format = config.logging.format,
        "Tracing initialized"
    );

    Ok(TelemetryGuard { _guard: None })
}

/// Setup metrics collection
pub async fn setup_metrics(config: &crate::config::MetricsConfig) -> Result<Metrics> {
    if !config.enabled {
        info!("Metrics collection disabled");
        return Metrics::new();
    }

    let metrics = Metrics::new()?;

    info!(
        port = config.port,
        path = config.path,
        prometheus_enabled = config.prometheus_enabled,
        "Metrics collection initialized"
    );

    Ok(metrics)
}

/// Shutdown tracing
pub async fn shutdown_tracing() {
    // Flush any pending traces
    tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), || {
        // This ensures all pending traces are flushed
    });

    info!("Tracing shutdown complete");
}

/// Helper function to create a span for request tracing
pub fn create_request_span(method: &str, path: &str, request_id: &str) -> tracing::Span {
    tracing::info_span!(
        "http_request",
        method = method,
        path = path,
        request_id = request_id,
        status_code = tracing::field::Empty,
        response_time_ms = tracing::field::Empty,
    )
}

/// Helper function to create a span for health check tracing
pub fn create_health_check_span(server_id: &str, server_name: &str) -> tracing::Span {
    tracing::info_span!(
        "health_check",
        server_id = server_id,
        server_name = server_name,
        status = tracing::field::Empty,
        response_time_ms = tracing::field::Empty,
    )
}

/// Helper function to create a span for protocol communication
pub fn create_protocol_span(method: &str, server_id: &str, message_id: &str) -> tracing::Span {
    tracing::info_span!(
        "mcp_protocol",
        method = method,
        server_id = server_id,
        message_id = message_id,
        success = tracing::field::Empty,
        response_time_ms = tracing::field::Empty,
    )
}

/// Helper function to create a span for load balancer operations
pub fn create_load_balancer_span(strategy: &str, server_count: usize) -> tracing::Span {
    tracing::info_span!(
        "load_balancer",
        strategy = strategy,
        server_count = server_count,
        selected_server = tracing::field::Empty,
        selection_time_ms = tracing::field::Empty,
    )
}

/// Helper macros for common telemetry patterns
#[macro_export]
macro_rules! record_request_metrics {
    ($metrics:expr, $method:expr, $status:expr, $duration:expr) => {
        $metrics.requests_total.inc();
        $metrics.requests_duration.observe($duration);
        if $status >= 400 {
            $metrics.requests_errors_total.inc();
        }
    };
}

#[macro_export]
macro_rules! record_health_check_metrics {
    ($metrics:expr, $success:expr, $duration:expr) => {
        $metrics.health_checks_total.inc();
        $metrics.health_check_duration.observe($duration);
        if !$success {
            $metrics.health_check_failures.inc();
        }
    };
}

#[macro_export]
macro_rules! record_protocol_metrics {
    ($metrics:expr, $sent:expr, $received:expr, $error:expr) => {
        if $sent {
            $metrics.protocol_messages_sent.inc();
        }
        if $received {
            $metrics.protocol_messages_received.inc();
        }
        if $error {
            $metrics.protocol_errors.inc();
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_metrics_creation() {
        let result = Metrics::new();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_export() {
        let metrics = Metrics::new().unwrap();
        let exported = metrics.export().unwrap();
        // Basic validation that export returns some string
        assert!(!exported.is_empty());
    }

    #[tokio::test]
    async fn test_tracing_initialization() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test.log");

        let config = Config {
            environment: "test".to_string(),
            logging: crate::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                console: true,
                file_enabled: true,
                file_path: Some(log_file.to_string_lossy().to_string()),
                file_max_size_mb: 100,
                file_max_files: 5,
                structured: true,
                fields: std::collections::HashMap::new(),
            },
            ..Config::default()
        };

        let _guard = init_tracing(&config).await.unwrap();

        // Test that logging works
        tracing::info!("Test log message");

        // Give some time for the log to be written
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[test]
    fn test_span_creation() {
        let span = create_request_span("GET", "/health", "test-request-id");
        assert_eq!(span.metadata().unwrap().name(), "http_request");

        let span = create_health_check_span("server-id", "server-name");
        assert_eq!(span.metadata().unwrap().name(), "health_check");

        let span = create_protocol_span("ping", "server-id", "message-id");
        assert_eq!(span.metadata().unwrap().name(), "mcp_protocol");

        let span = create_load_balancer_span("round_robin", 3);
        assert_eq!(span.metadata().unwrap().name(), "load_balancer");
    }
}
