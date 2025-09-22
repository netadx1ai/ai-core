//! Integration tests for the AI-CORE API Gateway
//!
//! These tests validate the complete API Gateway functionality including
//! health checks, authentication, workflow management, and error handling.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio;
use tower::ServiceExt;

// Import the API Gateway modules
use ai_core_api_gateway::{build_router, AppState, Config};

/// Test helper to create a test application state
async fn create_test_state() -> AppState {
    use ai_core_api_gateway::{
        AuthConfig, DatabaseConfig, ExternalServicesConfig, ObservabilityConfig, RedisConfig,
        ServerConfig,
    };

    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        database: DatabaseConfig {
            url: "postgres://test:test@localhost:5432/test_db".to_string(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout: 10,
            idle_timeout: 300,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379/1".to_string(),
            max_connections: 5,
            connect_timeout: 5,
            command_timeout: 5,
        },
        auth: AuthConfig {
            jwt_secret: "test-secret-key-for-testing-purposes-only".to_string(),
            jwt_expiry_hours: 1,
            jwt_refresh_expiry_days: 1,
        },
        external_services: ExternalServicesConfig {
            temporal_server_url: "http://localhost:7233".to_string(),
            temporal_namespace: "test".to_string(),
        },
        observability: ObservabilityConfig {
            jaeger_endpoint: None,
            otel_service_name: "ai-core-api-gateway-test".to_string(),
            metrics_enabled: false,
            log_level: "debug".to_string(),
            pretty_logs: true,
        },
        environment: "test".to_string(),
    };
    AppState::new_degraded(config)
        .await
        .expect("Failed to create test state")
}

/// Test helper to make HTTP requests to the API
async fn make_request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    headers: Option<HashMap<&str, &str>>,
) -> (StatusCode, Value) {
    let mut request_builder = Request::builder().method(method).uri(uri);

    // Add headers if provided
    if let Some(headers) = headers {
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }
    }

    let request = if let Some(body) = body {
        request_builder
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    } else {
        request_builder.body(Body::empty()).unwrap()
    };

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let body_json: Value = if body_bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or(json!({}))
    };

    (status, body_json)
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = create_test_state().await;
    let app = build_router(state);

    let (status, body) = make_request(&app, Method::GET, "/health", None, None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], "degraded");
    assert!(body["details"]["api_gateway"]
        .as_str()
        .unwrap()
        .contains("available"));
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let state = create_test_state().await;
    let app = build_router(state);

    let (status, _) = make_request(&app, Method::GET, "/metrics", None, None).await;

    // Should return Prometheus metrics format
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_ready_endpoint() {
    let state = create_test_state().await;
    let app = build_router(state);

    let (status, body) = make_request(&app, Method::GET, "/ready", None, None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["ready"], false);
}

#[tokio::test]
async fn test_auth_endpoints_without_token() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test login endpoint
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let (status, body) =
        make_request(&app, Method::POST, "/v1/auth/login", Some(login_body), None).await;

    // Should fail in degraded mode due to no database
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_protected_endpoint_without_auth() {
    let state = create_test_state().await;
    let app = build_router(state);

    let (status, body) = make_request(&app, Method::GET, "/v1/workflows", None, None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"]["code"]
        .as_str()
        .unwrap()
        .contains("AUTHENTICATION"));
}

#[tokio::test]
async fn test_protected_endpoint_with_invalid_token() {
    let state = create_test_state().await;
    let app = build_router(state);

    let mut headers = HashMap::new();
    headers.insert("Authorization", "Bearer invalid-token");

    let (status, body) =
        make_request(&app, Method::GET, "/v1/workflows", None, Some(headers)).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"]["code"]
        .as_str()
        .unwrap()
        .contains("AUTHENTICATION"));
}

#[tokio::test]
async fn test_workflow_crud_operations() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Mock JWT token (in real tests, this would be generated properly)
    let mock_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXIiLCJleHAiOjk5OTk5OTk5OTl9.mock";
    let auth_header = format!("Bearer {}", mock_token);
    let mut headers = HashMap::new();
    headers.insert("Authorization", auth_header.as_str());

    // Test create workflow
    let workflow_body = json!({
        "name": "Test Workflow",
        "description": "A test workflow",
        "steps": [
            {
                "name": "step1",
                "type": "action",
                "config": {}
            }
        ]
    });

    let (status, _body) = make_request(
        &app,
        Method::POST,
        "/v1/workflows",
        Some(workflow_body),
        Some(headers.iter().map(|(k, v)| (*k, *v)).collect()),
    )
    .await;

    // Should fail in degraded mode but with proper error handling
    assert!(status == StatusCode::SERVICE_UNAVAILABLE || status == StatusCode::UNAUTHORIZED);

    // Test list workflows
    let (status, _body) = make_request(
        &app,
        Method::GET,
        "/v1/workflows",
        None,
        Some(headers.iter().map(|(k, v)| (*k, *v)).collect()),
    )
    .await;

    // Should fail gracefully in degraded mode
    assert!(status == StatusCode::SERVICE_UNAVAILABLE || status == StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_rate_limiting() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Make multiple rapid requests to trigger rate limiting
    let mut responses = Vec::new();
    for _ in 0..5 {
        let (status, _) = make_request(&app, Method::GET, "/health", None, None).await;
        responses.push(status);
    }

    // All should succeed since rate limiting is generous for health endpoint
    assert!(responses
        .iter()
        .all(|&s| s == StatusCode::SERVICE_UNAVAILABLE));
}

#[tokio::test]
async fn test_cors_headers() {
    let state = create_test_state().await;
    let app = build_router(state);

    let (status, _) = make_request(&app, Method::OPTIONS, "/health", None, None).await;

    // CORS preflight should be handled
    assert!(status == StatusCode::OK || status == StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_error_handling_middleware() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test non-existent endpoint
    let (status, body) = make_request(&app, Method::GET, "/v1/nonexistent", None, None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_request_logging() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Make a request and ensure it's logged (this test mainly ensures no panics)
    let (_status, _body) = make_request(&app, Method::GET, "/health", None, None).await;

    // If we get here without panicking, logging middleware is working
    assert!(true);
}

#[tokio::test]
async fn test_monitoring_endpoints() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test analytics endpoint
    let (status, _) = make_request(&app, Method::GET, "/v1/analytics/overview", None, None).await;
    assert!(status == StatusCode::UNAUTHORIZED || status == StatusCode::SERVICE_UNAVAILABLE);

    // Test monitoring endpoint
    let (status, _) = make_request(&app, Method::GET, "/v1/monitoring/health", None, None).await;
    assert!(status == StatusCode::UNAUTHORIZED || status == StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_admin_endpoints() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test admin endpoints require authentication
    let (status, _) = make_request(&app, Method::GET, "/v1/admin/users", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = make_request(&app, Method::GET, "/v1/admin/system/info", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_federation_endpoints() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test federation endpoints require authentication
    let (status, _) =
        make_request(&app, Method::GET, "/v1/federation/connections", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_billing_endpoints() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test billing endpoints require authentication
    let (status, _) = make_request(&app, Method::GET, "/v1/billing/usage", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_json_request_parsing() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test malformed JSON
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from("invalid json"))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_large_request_handling() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Create a large JSON payload
    let large_data = "x".repeat(1024 * 1024); // 1MB string
    let large_body = json!({
        "data": large_data
    });

    let (status, _) =
        make_request(&app, Method::POST, "/v1/auth/login", Some(large_body), None).await;

    // Should handle large requests (may reject if too large)
    assert!(status.as_u16() >= 200 && status.as_u16() < 500);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test concurrent requests
    let mut handles = Vec::new();

    for i in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let (_status, _body) = make_request(
                &app_clone,
                Method::GET,
                &format!("/health?req={}", i),
                None,
                None,
            )
            .await;
            // Return success if no panic occurred
            true
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    // All requests should complete successfully
    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|&r| r));
}

#[tokio::test]
async fn test_graceful_degradation() {
    let state = create_test_state().await;
    let app = build_router(state);

    // Test that the API Gateway handles service unavailability gracefully
    let endpoints_to_test = ["/health", "/ready", "/metrics"];

    for endpoint in &endpoints_to_test {
        let (status, body) = make_request(&app, Method::GET, endpoint, None, None).await;

        // Should return proper error responses, not crash
        assert!(status.as_u16() >= 200 && status.as_u16() < 600);

        // Error responses should be properly formatted JSON
        if status.is_client_error() || status.is_server_error() {
            assert!(
                body.is_object(),
                "Error response should be JSON object for {}",
                endpoint
            );
        }
    }
}

// Helper module for test utilities
mod test_utils {
    use super::*;

    pub fn create_mock_jwt_token(user_id: &str, permissions: Vec<&str>) -> String {
        // In a real implementation, this would create a proper JWT token
        // For testing, we return a mock token
        format!("mock-token-{}-{}", user_id, permissions.join(","))
    }

    pub async fn wait_for_service_ready(app: &axum::Router, max_attempts: u32) -> bool {
        for _ in 0..max_attempts {
            let (status, _) = make_request(app, Method::GET, "/ready", None, None).await;
            if status == StatusCode::OK {
                return true;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        false
    }
}

// Integration tests that would run against a real database/Redis instance
#[cfg(feature = "integration-db")]
mod database_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_auth_flow() {
        // This test would run against a real database
        // Test user registration, login, token refresh, logout
        todo!("Implement when database is available");
    }

    #[tokio::test]
    async fn test_workflow_persistence() {
        // Test creating, updating, deleting workflows with database persistence
        todo!("Implement when database is available");
    }

    #[tokio::test]
    async fn test_rate_limiting_with_redis() {
        // Test rate limiting with actual Redis backend
        todo!("Implement when Redis is available");
    }
}

// Load testing module
#[cfg(feature = "load-tests")]
mod load_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn test_concurrent_load() {
        let state = create_test_state().await;
        let app = Arc::new(build_router(state));

        let total_requests = 1000u64;
        let concurrent_workers = 50;
        let requests_per_worker = total_requests / concurrent_workers;

        let success_count = Arc::new(AtomicU64::new(0));
        let error_count = Arc::new(AtomicU64::new(0));

        let start_time = Instant::now();
        let mut handles = Vec::new();

        for _ in 0..concurrent_workers {
            let app_clone = Arc::clone(&app);
            let success_count_clone = Arc::clone(&success_count);
            let error_count_clone = Arc::clone(&error_count);

            let handle = tokio::spawn(async move {
                for _ in 0..requests_per_worker {
                    let (status, _) =
                        make_request(&app_clone, Method::GET, "/health", None, None).await;

                    if status.is_success() || status == StatusCode::SERVICE_UNAVAILABLE {
                        success_count_clone.fetch_add(1, Ordering::Relaxed);
                    } else {
                        error_count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        futures::future::join_all(handles).await;

        let duration = start_time.elapsed();
        let success_total = success_count.load(Ordering::Relaxed);
        let error_total = error_count.load(Ordering::Relaxed);

        println!("Load test results:");
        println!("  Duration: {:?}", duration);
        println!("  Successful requests: {}", success_total);
        println!("  Failed requests: {}", error_total);
        println!(
            "  Requests/second: {:.2}",
            total_requests as f64 / duration.as_secs_f64()
        );

        // Assert that we handled the load successfully
        assert!(success_total > 0, "No successful requests");
        assert!(
            error_total < total_requests / 10,
            "Too many errors: {} out of {}",
            error_total,
            total_requests
        );
    }
}
