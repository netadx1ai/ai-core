//! HTTP handlers for the AI-CORE Integration Service
//!
//! This module provides HTTP endpoint handlers for webhook processing, health checks,
//! metrics, and OAuth flows for all supported integrations.

use crate::models::{
    HealthCheckResponse, HealthStatus, IntegrationHealth, SystemHealth, WebhookPayload,
    WebhookResponse,
};
use crate::service::AppState;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Create all routes for the integration service
pub fn create_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Health and monitoring endpoints
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/live", get(liveness_check))
        .route("/metrics", get(metrics_handler))
        // Webhook endpoints
        .route("/webhooks/zapier", post(zapier_webhook_handler))
        .route("/webhooks/slack", post(slack_webhook_handler))
        .route("/webhooks/github", post(github_webhook_handler))
        .route("/webhooks/:integration", post(generic_webhook_handler))
        // OAuth endpoints
        .route("/oauth/slack/callback", get(slack_oauth_callback))
        .route("/oauth/github/callback", get(github_oauth_callback))
        // API endpoints
        .route("/api/v1/integrations", get(list_integrations))
        .route(
            "/api/v1/integrations/:integration/status",
            get(integration_status),
        )
        .route("/api/v1/events", get(list_events))
        .route("/api/v1/events/:event_id", get(get_event))
        .with_state(state)
}

/// Health check endpoint
async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Health check requested");

    let mut integration_healths = HashMap::new();
    let mut overall_status = HealthStatus::Healthy;

    // Check each integration
    for (name, integration) in &state.integrations {
        match integration.health_check().await {
            Ok(is_healthy) => {
                let status = if is_healthy {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Unhealthy
                };
                if status == HealthStatus::Unhealthy && overall_status == HealthStatus::Healthy {
                    overall_status = HealthStatus::Degraded;
                }
                integration_healths.insert(
                    name.clone(),
                    IntegrationHealth {
                        status,
                        last_check: Utc::now(),
                        response_time_ms: None,
                        error: None,
                    },
                );
            }
            Err(e) => {
                overall_status = HealthStatus::Degraded;
                integration_healths.insert(
                    name.clone(),
                    IntegrationHealth {
                        status: HealthStatus::Unhealthy,
                        last_check: Utc::now(),
                        response_time_ms: None,
                        error: Some(e.to_string()),
                    },
                );
            }
        }
    }

    // Check system health
    let db_healthy = state.db_pool.is_some();
    let redis_healthy = state.redis_pool.is_some();

    if (!db_healthy || !redis_healthy) && overall_status == HealthStatus::Healthy {
        overall_status = HealthStatus::Degraded;
    }

    let system_health = SystemHealth {
        database: db_healthy,
        redis: redis_healthy,
        memory_usage_percent: 0.0, // TODO: Get actual system metrics
        cpu_usage_percent: 0.0,
        uptime_seconds: 0, // TODO: Track service uptime
    };

    let response = HealthCheckResponse {
        service: "integration-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: overall_status,
        integrations: integration_healths,
        system: system_health,
        timestamp: Utc::now(),
    };

    let status_code = match response.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still accepting traffic
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response))
}

/// Readiness check endpoint (for Kubernetes)
async fn readiness_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Check if critical dependencies are available
    let ready = state.integrations.values().any(|integration| {
        // At least one integration should be healthy
        matches!(
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(integration.health_check())
            }),
            Ok(true)
        )
    });

    if ready {
        (StatusCode::OK, Json(json!({"status": "ready"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "not ready"})),
        )
    }
}

/// Liveness check endpoint (for Kubernetes)
async fn liveness_check() -> impl IntoResponse {
    // Simple liveness check - service is running
    (StatusCode::OK, Json(json!({"status": "alive"})))
}

/// Metrics endpoint (Prometheus format)
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let metrics = state.metrics.lock().await;

    // TODO: Generate Prometheus metrics format
    let metrics_text = format!(
        "# HELP integration_requests_total Total number of integration requests\n# TYPE integration_requests_total counter\nintegration_requests_total {}\n\n# HELP integration_requests_successful_total Total number of successful integration requests\n# TYPE integration_requests_successful_total counter\nintegration_requests_successful_total {}\n\n# HELP integration_requests_failed_total Total number of failed integration requests\n# TYPE integration_requests_failed_total counter\nintegration_requests_failed_total {}\n\n# HELP integration_response_time_seconds Average response time in seconds\n# TYPE integration_response_time_seconds gauge\nintegration_response_time_seconds {}\n",
        metrics.total_requests,
        metrics.successful_requests,
        metrics.failed_requests,
        metrics.avg_response_time_ms / 1000.0
    );

    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        metrics_text,
    )
}

/// Zapier webhook handler
async fn zapier_webhook_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    process_webhook(state, "zapier", addr, headers, body).await
}

/// Slack webhook handler
async fn slack_webhook_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    process_webhook(state, "slack", addr, headers, body).await
}

/// GitHub webhook handler
async fn github_webhook_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    process_webhook(state, "github", addr, headers, body).await
}

/// Generic webhook handler
async fn generic_webhook_handler(
    Path(integration): Path<String>,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    process_webhook(state, &integration, addr, headers, body).await
}

/// Core webhook processing logic
async fn process_webhook(
    state: Arc<AppState>,
    integration_name: &str,
    addr: SocketAddr,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();

    debug!(
        request_id = %request_id,
        integration = integration_name,
        source_ip = %addr.ip(),
        "Processing webhook request"
    );

    // Get the integration
    let integration = match state.integrations.get(integration_name) {
        Some(integration) => integration,
        None => {
            warn!(
                request_id = %request_id,
                integration = integration_name,
                "Unknown integration"
            );
            return (
                StatusCode::NOT_FOUND,
                Json(WebhookResponse::error(
                    request_id,
                    format!("Integration '{}' not found", integration_name),
                )),
            )
                .into_response();
        }
    };

    // Convert headers to HashMap
    let header_map: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Validate webhook signature
    if let Err(e) = integration.validate_webhook(&body, &header_map).await {
        error!(
            request_id = %request_id,
            integration = integration_name,
            error = %e,
            "Webhook validation failed"
        );

        // Update metrics
        let mut metrics = state.metrics.lock().await;
        metrics.total_requests += 1;
        metrics.failed_requests += 1;

        return (
            StatusCode::UNAUTHORIZED,
            Json(WebhookResponse::error(request_id, e.to_string())),
        )
            .into_response();
    }

    // Parse JSON payload
    let json_data: Value = match serde_json::from_slice(&body) {
        Ok(data) => data,
        Err(e) => {
            error!(
                request_id = %request_id,
                integration = integration_name,
                error = %e,
                "Failed to parse JSON payload"
            );

            // Update metrics
            let mut metrics = state.metrics.lock().await;
            metrics.total_requests += 1;
            metrics.failed_requests += 1;

            return (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse::error(
                    request_id,
                    format!("Invalid JSON payload: {}", e),
                )),
            )
                .into_response();
        }
    };

    // Create webhook payload
    let webhook_payload = WebhookPayload {
        id: Uuid::parse_str(&request_id).unwrap_or_else(|_| Uuid::new_v4()),
        integration: integration_name.to_string(),
        event_type: extract_event_type(&json_data, integration_name),
        timestamp: Utc::now(),
        data: json_data,
        headers: header_map,
        source_ip: Some(addr.ip().to_string()),
        user_agent: headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    };

    // Process the webhook
    match integration.process_webhook(webhook_payload).await {
        Ok(event) => {
            let processing_time = start_time.elapsed();

            info!(
                request_id = %request_id,
                integration = integration_name,
                event_id = %event.id,
                processing_time_ms = processing_time.as_millis(),
                "Webhook processed successfully"
            );

            // Update metrics
            let mut metrics = state.metrics.lock().await;
            metrics.total_requests += 1;
            metrics.successful_requests += 1;
            metrics.avg_response_time_ms = (metrics.avg_response_time_ms
                * (metrics.total_requests - 1) as f64
                + processing_time.as_millis() as f64)
                / metrics.total_requests as f64;

            (
                StatusCode::OK,
                Json(WebhookResponse::success_with_data(
                    request_id,
                    "Webhook processed successfully".to_string(),
                    json!({
                        "event_id": event.id,
                        "status": event.status.to_string(),
                        "processing_time_ms": processing_time.as_millis()
                    }),
                )),
            )
                .into_response()
        }
        Err(e) => {
            let processing_time = start_time.elapsed();

            error!(
                request_id = %request_id,
                integration = integration_name,
                error = %e,
                processing_time_ms = processing_time.as_millis(),
                "Webhook processing failed"
            );

            // Update metrics
            let mut metrics = state.metrics.lock().await;
            metrics.total_requests += 1;
            metrics.failed_requests += 1;

            (
                e.status_code(),
                Json(WebhookResponse::error(request_id, e.to_string())),
            )
                .into_response()
        }
    }
}

/// Extract event type from payload based on integration
fn extract_event_type(payload: &Value, integration: &str) -> String {
    match integration {
        "zapier" => payload
            .get("event_name")
            .and_then(|v| v.as_str())
            .unwrap_or("webhook")
            .to_string(),
        "slack" => payload
            .get("type")
            .and_then(|v| v.as_str())
            .or_else(|| {
                payload
                    .get("event")
                    .and_then(|e| e.get("type"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("message")
            .to_string(),
        "github" => payload
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("push")
            .to_string(),
        _ => "webhook".to_string(),
    }
}

/// Slack OAuth callback handler
async fn slack_oauth_callback(
    Query(params): Query<HashMap<String, String>>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement OAuth callback handling
    let code = params.get("code");
    let state_param = params.get("state");

    debug!(
        code = ?code,
        state = ?state_param,
        "Slack OAuth callback received"
    );

    // For now, return a simple response
    (
        StatusCode::OK,
        Json(json!({
            "message": "OAuth callback received",
            "integration": "slack"
        })),
    )
}

/// GitHub OAuth callback handler
async fn github_oauth_callback(
    Query(params): Query<HashMap<String, String>>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement OAuth callback handling
    let code = params.get("code");
    let state_param = params.get("state");

    debug!(
        code = ?code,
        state = ?state_param,
        "GitHub OAuth callback received"
    );

    // For now, return a simple response
    (
        StatusCode::OK,
        Json(json!({
            "message": "OAuth callback received",
            "integration": "github"
        })),
    )
}

/// List all available integrations
async fn list_integrations(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let integrations: Vec<_> = state
        .integrations
        .keys()
        .map(|name| {
            json!({
                "name": name,
                "enabled": true,
                "supported_events": state.integrations[name].supported_events()
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({ "integrations": integrations })),
    )
}

/// Get integration status
async fn integration_status(
    Path(integration_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.integrations.get(&integration_name) {
        Some(integration) => {
            let is_healthy = integration.health_check().await.unwrap_or(false);
            (
                StatusCode::OK,
                Json(json!({
                    "integration": integration_name,
                    "healthy": is_healthy,
                    "supported_events": integration.supported_events()
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Integration '{}' not found", integration_name)
            })),
        ),
    }
}

/// Query parameters for event listing
#[derive(Debug, Deserialize)]
struct EventQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    integration: Option<String>,
    status: Option<String>,
}

/// List events (placeholder implementation)
async fn list_events(
    Query(_query): Query<EventQuery>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement actual event querying from database
    (
        StatusCode::OK,
        Json(json!({
            "events": [],
            "total": 0,
            "message": "Event storage not yet implemented"
        })),
    )
}

/// Get specific event (placeholder implementation)
async fn get_event(
    Path(_event_id): Path<String>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement actual event retrieval from database
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Event storage not yet implemented"
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IntegrationConfig;
    use crate::integrations::IntegrationFactory;
    use axum_test::TestServer;
    use serde_json::json;

    async fn create_test_state() -> Arc<AppState> {
        let config = IntegrationConfig::default();
        let mut integrations = HashMap::new();
        integrations.insert(
            "zapier".to_string(),
            IntegrationFactory::create_zapier(&config.zapier),
        );

        Arc::new(AppState {
            config,
            http_client: reqwest::Client::new(),
            redis_pool: None,
            db_pool: None,
            integrations,
            metrics: Arc::new(tokio::sync::Mutex::new(
                crate::metrics::IntegrationMetrics::new(),
            )),
        })
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = create_test_state().await;
        let app = create_routes(state);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let state = create_test_state().await;
        let app = create_routes(state);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health/live").await;
        assert_eq!(response.status_code(), 200);

        let body: Value = response.json();
        assert_eq!(body["status"], "alive");
    }

    #[tokio::test]
    async fn test_list_integrations() {
        let state = create_test_state().await;
        let app = create_routes(state);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/v1/integrations").await;
        assert_eq!(response.status_code(), 200);

        let body: Value = response.json();
        assert!(body["integrations"].is_array());
    }

    #[tokio::test]
    async fn test_integration_status() {
        let state = create_test_state().await;
        let app = create_routes(state);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/v1/integrations/zapier/status").await;
        assert_eq!(response.status_code(), 200);

        let body: Value = response.json();
        assert_eq!(body["integration"], "zapier");
    }

    #[test]
    fn test_extract_event_type() {
        // Test Zapier event type extraction
        let zapier_payload = json!({"event_name": "new_customer"});
        assert_eq!(
            extract_event_type(&zapier_payload, "zapier"),
            "new_customer"
        );

        // Test Slack event type extraction
        let slack_payload = json!({"type": "message"});
        assert_eq!(extract_event_type(&slack_payload, "slack"), "message");

        // Test GitHub event type extraction
        let github_payload = json!({"action": "opened"});
        assert_eq!(extract_event_type(&github_payload, "github"), "opened");

        // Test fallback
        let empty_payload = json!({});
        assert_eq!(extract_event_type(&empty_payload, "unknown"), "webhook");
    }
}
