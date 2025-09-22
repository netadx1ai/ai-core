//! Quality Dashboard Binary
//!
//! Standalone web server providing a real-time quality dashboard for monitoring
//! test results, performance metrics, security status, and quality trends.

use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use clap::{Arg, ArgMatches, Command};
use qa_agent::config::{DashboardConfig, QAConfig};
use qa_agent::dashboard::{DashboardStatus, QualityDashboard};
use qa_agent::metrics::{MetricsCollector, QualityScore, QualityTrends};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Deserialize)]
struct DashboardQuery {
    timeframe: Option<String>,
    component: Option<String>,
    refresh: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("quality_dashboard=info".parse()?),
        )
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::rfc_3339())
        .init();

    info!("📊 AI-CORE Quality Dashboard v0.1.0");

    let matches = build_cli().get_matches();
    let config = load_configuration(&matches).await?;

    let host = matches.get_one::<String>("host").unwrap();
    let port: u16 = matches.get_one::<String>("port").unwrap().parse()?;
    let static_dir = matches.get_one::<String>("static-dir").unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(
        "🚀 Starting quality dashboard server on http://{}:{}",
        host, port
    );

    // Initialize dashboard
    let dashboard = QualityDashboard::new(config.dashboard.clone()).await?;
    let metrics_collector = QualityMetricsCollector::new(config.metrics.clone()).await?;

    // Create shared state
    let app_state = Arc::new(DashboardAppState {
        dashboard,
        metrics_collector,
        config: config.clone(),
    });

    // Build the router
    let app = build_router(app_state, static_dir).await?;

    // Start the server
    let listener = TcpListener::bind(&addr).await?;
    info!("✅ Quality dashboard listening on {}", addr);
    info!("🌐 Dashboard available at: http://{}:{}", host, port);
    info!(
        "📊 API endpoints available at: http://{}:{}/api",
        host, port
    );

    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Debug, Clone)]
struct DashboardAppState {
    dashboard: QualityDashboard,
    metrics_collector: MetricsCollector,
    config: QAConfig,
}

fn build_cli() -> Command {
    Command::new("quality-dashboard")
        .version("0.1.0")
        .author("AI-CORE Team")
        .about("Real-time quality dashboard for AI-CORE platform")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/qa.toml"),
        )
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .value_name("HOST")
                .help("Host to bind to")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Port to listen on")
                .default_value("8080"),
        )
        .arg(
            Arg::new("static-dir")
                .short('s')
                .long("static-dir")
                .value_name("DIR")
                .help("Static files directory")
                .default_value("src/qa-agent/dashboard/static"),
        )
        .arg(
            Arg::new("auto-refresh")
                .long("auto-refresh")
                .value_name("SECONDS")
                .help("Auto-refresh interval in seconds")
                .default_value("30"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity level"),
        )
}

async fn load_configuration(matches: &ArgMatches) -> Result<QAConfig> {
    let config_path = matches.get_one::<String>("config").unwrap();
    info!("Loading configuration from: {}", config_path);

    let config = QAConfig::from_file(config_path)?;
    info!("Configuration loaded successfully");

    Ok(config)
}

async fn build_router(state: Arc<DashboardAppState>, static_dir: &str) -> Result<Router> {
    let app = Router::new()
        // Dashboard routes
        .route("/", get(dashboard_home))
        .route("/dashboard", get(dashboard_home))
        .route("/health", get(health_check))
        // API routes
        .route("/api/quality/score", get(get_quality_score))
        .route("/api/quality/trends", get(get_quality_trends))
        .route("/api/quality/components", get(get_component_scores))
        .route("/api/tests/results", get(get_test_results))
        .route("/api/tests/coverage", get(get_test_coverage))
        .route("/api/performance/metrics", get(get_performance_metrics))
        .route(
            "/api/performance/benchmarks",
            get(get_performance_benchmarks),
        )
        .route("/api/security/status", get(get_security_status))
        .route(
            "/api/security/vulnerabilities",
            get(get_security_vulnerabilities),
        )
        .route("/api/reports/generate", post(generate_report))
        .route("/api/alerts", get(get_alerts))
        .route("/api/system/status", get(get_system_status))
        // WebSocket for real-time updates
        .route("/ws", get(websocket_handler))
        // Static file serving
        .nest_service("/static", ServeDir::new(static_dir))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        )
        .with_state(state);

    Ok(app)
}

// Dashboard HTML page
async fn dashboard_home() -> impl IntoResponse {
    let html = include_str!("../../dashboard/templates/index.html");
    Html(html)
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "service": "quality-dashboard",
        "version": "0.1.0",
        "uptime": "system_uptime_placeholder"
    })))
}

// Quality score endpoint
async fn get_quality_score(
    State(state): State<Arc<DashboardAppState>>,
    Query(params): Query<DashboardQuery>,
) -> impl IntoResponse {
    match state.dashboard.get_current_quality_score().await {
        Ok(score) => Json(ApiResponse::success(score)),
        Err(e) => {
            error!("Failed to get quality score: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get quality score: {}",
                e
            )))
        }
    }
}

// Quality trends endpoint
async fn get_quality_trends(
    State(state): State<Arc<DashboardAppState>>,
    Query(params): Query<DashboardQuery>,
) -> impl IntoResponse {
    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());

    match state.dashboard.get_quality_trends(&timeframe).await {
        Ok(trends) => Json(ApiResponse::success(trends)),
        Err(e) => {
            error!("Failed to get quality trends: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get quality trends: {}",
                e
            )))
        }
    }
}

// Component scores endpoint
async fn get_component_scores(State(state): State<Arc<DashboardAppState>>) -> impl IntoResponse {
    match state.dashboard.get_component_scores().await {
        Ok(scores) => Json(ApiResponse::success(scores)),
        Err(e) => {
            error!("Failed to get component scores: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get component scores: {}",
                e
            )))
        }
    }
}

// Test results endpoint
async fn get_test_results(
    State(state): State<Arc<DashboardAppState>>,
    Query(params): Query<DashboardQuery>,
) -> impl IntoResponse {
    match state.metrics_collector.get_test_results().await {
        Ok(results) => Json(ApiResponse::success(results)),
        Err(e) => {
            error!("Failed to get test results: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get test results: {}",
                e
            )))
        }
    }
}

// Test coverage endpoint
async fn get_test_coverage(State(state): State<Arc<DashboardAppState>>) -> impl IntoResponse {
    match state.metrics_collector.get_test_coverage().await {
        Ok(coverage) => Json(ApiResponse::success(coverage)),
        Err(e) => {
            error!("Failed to get test coverage: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get test coverage: {}",
                e
            )))
        }
    }
}

// Performance metrics endpoint
async fn get_performance_metrics(
    State(state): State<Arc<DashboardAppState>>,
    Query(params): Query<DashboardQuery>,
) -> impl IntoResponse {
    match state.metrics_collector.get_performance_metrics().await {
        Ok(metrics) => Json(ApiResponse::success(metrics)),
        Err(e) => {
            error!("Failed to get performance metrics: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get performance metrics: {}",
                e
            )))
        }
    }
}

// Performance benchmarks endpoint
async fn get_performance_benchmarks(
    State(state): State<Arc<DashboardAppState>>,
) -> impl IntoResponse {
    match state.metrics_collector.get_performance_benchmarks().await {
        Ok(benchmarks) => Json(ApiResponse::success(benchmarks)),
        Err(e) => {
            error!("Failed to get performance benchmarks: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get performance benchmarks: {}",
                e
            )))
        }
    }
}

// Security status endpoint
async fn get_security_status(State(state): State<Arc<DashboardAppState>>) -> impl IntoResponse {
    match state.metrics_collector.get_security_status().await {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => {
            error!("Failed to get security status: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get security status: {}",
                e
            )))
        }
    }
}

// Security vulnerabilities endpoint
async fn get_security_vulnerabilities(
    State(state): State<Arc<DashboardAppState>>,
) -> impl IntoResponse {
    match state.metrics_collector.get_security_vulnerabilities().await {
        Ok(vulnerabilities) => Json(ApiResponse::success(vulnerabilities)),
        Err(e) => {
            error!("Failed to get security vulnerabilities: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get security vulnerabilities: {}",
                e
            )))
        }
    }
}

// Generate report endpoint
#[derive(Deserialize)]
struct ReportRequest {
    format: String,
    timeframe: Option<String>,
    components: Option<Vec<String>>,
}

async fn generate_report(
    State(state): State<Arc<DashboardAppState>>,
    Json(request): Json<ReportRequest>,
) -> impl IntoResponse {
    info!("Generating {} report", request.format);

    match state
        .dashboard
        .generate_quality_report(&request.format, request.timeframe.as_deref())
        .await
    {
        Ok(report) => {
            let content_type = match request.format.as_str() {
                "pdf" => "application/pdf",
                "html" => "text/html",
                "json" => "application/json",
                "csv" => "text/csv",
                _ => "application/octet-stream",
            };

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, content_type)],
                report,
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to generate report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!(
                    "Failed to generate report: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

// Alerts endpoint
async fn get_alerts(State(state): State<Arc<DashboardAppState>>) -> impl IntoResponse {
    match state.dashboard.get_current_alerts().await {
        Ok(alerts) => Json(ApiResponse::success(alerts)),
        Err(e) => {
            error!("Failed to get alerts: {}", e);
            Json(ApiResponse::error(format!("Failed to get alerts: {}", e)))
        }
    }
}

// System status endpoint
async fn get_system_status(State(state): State<Arc<DashboardAppState>>) -> impl IntoResponse {
    match state.dashboard.get_system_status().await {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => {
            error!("Failed to get system status: {}", e);
            Json(ApiResponse::error(format!(
                "Failed to get system status: {}",
                e
            )))
        }
    }
}

// WebSocket handler for real-time updates
use axum::{extract::WebSocketUpgrade, response::Response};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DashboardAppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: axum::extract::ws::WebSocket, state: Arc<DashboardAppState>) {
    use axum::extract::ws::Message;
    use tokio::time::{interval, Duration};

    info!("WebSocket connection established");

    let mut interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Send periodic updates
                match state.dashboard.get_dashboard_update().await {
                    Ok(update) => {
                        let message = serde_json::to_string(&update).unwrap_or_default();
                        if socket.send(Message::Text(message)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get dashboard update: {}", e);
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received WebSocket message: {}", text);
                        // Handle client requests for specific data
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
