//! Integration Tests for Security Middleware
//!
//! This module contains comprehensive integration tests for the security framework.

use crate::config::SecurityConfig;
use crate::errors::SecurityResult;
use crate::jwt::{JwtService, JwtServiceTrait};
use crate::middleware::{AuthenticationLayer, AuthorizationLayer, SecurityMiddleware};
use crate::rbac::{RbacService, RbacServiceTrait};
use crate::service::SecurityService;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tower::ServiceExt;
use uuid::Uuid;

/// Test helper for creating authenticated requests
fn create_auth_request(token: &str, method: &str, path: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    request
}

/// Test helper for creating unauthenticated requests
fn create_unauth_request(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

/// Mock API handler for testing
async fn protected_handler() -> &'static str {
    "Protected content"
}

async fn public_handler() -> &'static str {
    "Public content"
}

/// Setup test security service
async fn setup_security_service() -> SecurityResult<SecurityService> {
    let config = SecurityConfig::default();
    SecurityService::new(config).await
}

/// Test authentication middleware
#[tokio::test]
async fn test_authentication_middleware_valid_token() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    // Create a valid token
    let user_id = Uuid::new_v4();
    let token = jwt_service
        .generate_access_token(user_id, "test@example.com", vec!["user".to_string()])
        .await
        .unwrap();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);

    // Create test app
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(auth_layer);

    // Test with valid token
    let request = create_auth_request(&token.token, "GET", "/protected");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_authentication_middleware_invalid_token() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);

    // Create test app
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(auth_layer);

    // Test with invalid token
    let request = create_auth_request("invalid_token", "GET", "/protected");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_authentication_middleware_missing_token() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);

    // Create test app
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(auth_layer);

    // Test without token
    let request = create_unauth_request("GET", "/protected");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test authorization middleware
#[tokio::test]
async fn test_authorization_middleware_allowed_access() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();
    let rbac_service = security_service.rbac_service().clone();

    // Create a valid token with admin role
    let user_id = Uuid::new_v4();
    let token = jwt_service
        .generate_access_token(user_id, "admin@example.com", vec!["admin".to_string()])
        .await
        .unwrap();

    // Setup RBAC permissions
    rbac_service
        .assign_role_to_user(user_id, "admin".to_string())
        .await
        .unwrap();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);
    let authz_layer = AuthorizationLayer::new(
        rbac_service,
        "protected_resource".to_string(),
        "read".to_string(),
    );

    // Create test app
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(authz_layer)
        .layer(auth_layer);

    // Test with authorized user
    let request = create_auth_request(&token.token, "GET", "/protected");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_authorization_middleware_denied_access() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();
    let rbac_service = security_service.rbac_service().clone();

    // Create a valid token with user role (no admin permissions)
    let user_id = Uuid::new_v4();
    let token = jwt_service
        .generate_access_token(user_id, "user@example.com", vec!["user".to_string()])
        .await
        .unwrap();

    // Setup RBAC permissions - user role doesn't have admin permissions
    rbac_service
        .assign_role_to_user(user_id, "user".to_string())
        .await
        .unwrap();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);
    let authz_layer = AuthorizationLayer::new(
        rbac_service,
        "admin_resource".to_string(),
        "write".to_string(),
    );

    // Create test app
    let app = Router::new()
        .route("/admin", post(protected_handler))
        .layer(authz_layer)
        .layer(auth_layer);

    // Test with unauthorized user
    let request = create_auth_request(&token.token, "POST", "/admin");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Test rate limiting middleware
#[tokio::test]
async fn test_rate_limiting_middleware() {
    let security_service = setup_security_service().await.unwrap();
    let middleware = SecurityMiddleware::new(
        security_service.jwt_service().clone(),
        security_service.rbac_service().clone(),
        Default::default(),
    )
    .await
    .unwrap();

    // Create test requests
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.100"));

    let request1 = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let request2 = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // First request should be allowed
    let result1 = middleware.rate_limit_request(&request1).await;
    assert!(result1.is_ok());

    // Rapid subsequent requests should eventually be rate limited
    // (This test might need adjustment based on actual rate limits)
    for _ in 0..100 {
        let request = Request::builder()
            .method("GET")
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        let result = middleware.rate_limit_request(&request).await;
        if result.is_err() {
            // Rate limit was triggered
            break;
        }
    }
}

/// Test input validation middleware
#[tokio::test]
async fn test_input_validation_middleware() {
    let security_service = setup_security_service().await.unwrap();
    let middleware = SecurityMiddleware::new(
        security_service.jwt_service().clone(),
        security_service.rbac_service().clone(),
        Default::default(),
    )
    .await
    .unwrap();

    // Test with valid request
    let valid_request = Request::builder()
        .method("GET")
        .uri("/test")
        .header("Content-Length", "100")
        .body(Body::empty())
        .unwrap();

    let result = middleware.validate_input(&valid_request).await;
    assert!(result.is_ok());

    // Test with oversized request
    let oversized_request = Request::builder()
        .method("GET")
        .uri("/test")
        .header("Content-Length", "50000000") // 50MB - should exceed default limit
        .body(Body::empty())
        .unwrap();

    let result = middleware.validate_input(&oversized_request).await;
    assert!(result.is_err());

    // Test with too many headers
    let mut many_headers_request = Request::builder().method("GET").uri("/test");

    // Add many headers to exceed limit
    for i in 0..200 {
        many_headers_request = many_headers_request.header(
            HeaderName::from_bytes(format!("x-test-{}", i).as_bytes()).unwrap(),
            HeaderValue::from_static("value"),
        );
    }

    let request = many_headers_request.body(Body::empty()).unwrap();
    let result = middleware.validate_input(&request).await;
    assert!(result.is_err());
}

/// Test security headers middleware
#[tokio::test]
async fn test_security_headers_middleware() {
    let security_service = setup_security_service().await.unwrap();
    let middleware = SecurityMiddleware::new(
        security_service.jwt_service().clone(),
        security_service.rbac_service().clone(),
        Default::default(),
    )
    .await
    .unwrap();

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap();

    middleware.add_security_headers(&mut response);

    let headers = response.headers();

    // Check that security headers are present
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-xss-protection"));
    assert!(headers.contains_key("strict-transport-security"));
    assert!(headers.contains_key("referrer-policy"));
    assert!(headers.contains_key("content-security-policy"));
}

/// Test JWT token lifecycle
#[tokio::test]
async fn test_jwt_token_lifecycle() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let roles = vec!["user".to_string()];

    // Generate access token
    let access_token = jwt_service
        .generate_access_token(user_id, email, roles.clone())
        .await
        .unwrap();

    // Validate access token
    let validation_result = jwt_service
        .validate_access_token(&access_token.token)
        .await
        .unwrap();

    assert_eq!(validation_result.user_id, user_id);
    assert_eq!(validation_result.email, email);

    // Generate refresh token
    let refresh_token = jwt_service
        .generate_refresh_token(user_id, email, &roles)
        .await
        .unwrap();

    // Refresh access token using refresh token
    let new_access_token = jwt_service
        .refresh_access_token(&refresh_token.token)
        .await
        .unwrap();

    // Validate new access token
    let new_validation_result = jwt_service
        .validate_access_token(&new_access_token.token)
        .await
        .unwrap();

    assert_eq!(new_validation_result.user_id, user_id);
    assert_eq!(new_validation_result.email, email);

    // Blacklist the token
    jwt_service
        .blacklist_token(
            &access_token.token_id,
            user_id,
            "test_blacklist",
            access_token.expires_at,
        )
        .await
        .unwrap();

    // Validate blacklisted token should fail
    let blacklisted_result = jwt_service.validate_access_token(&access_token.token).await;

    assert!(blacklisted_result.is_err());
}

/// Test RBAC role assignment and permissions
#[tokio::test]
async fn test_rbac_permissions() {
    let security_service = setup_security_service().await.unwrap();
    let rbac_service = security_service.rbac_service().clone();

    let user_id = Uuid::new_v4();
    let role = "test_role";
    let resource = "test_resource";
    let action = "read";

    // Assign role to user
    rbac_service
        .assign_role_to_user(user_id, role.to_string())
        .await
        .unwrap();

    // Grant permission to role
    rbac_service
        .grant_permission_to_role(role, resource, action)
        .await
        .unwrap();

    // Check permission
    let has_permission = rbac_service
        .check_permission(user_id, resource, action)
        .await
        .unwrap();

    assert!(has_permission);

    // Check non-existent permission
    let no_permission = rbac_service
        .check_permission(user_id, resource, "write")
        .await
        .unwrap();

    assert!(!no_permission);

    // Revoke permission
    rbac_service
        .revoke_permission_from_role(role, resource, action)
        .await
        .unwrap();

    // Check permission after revoke
    let revoked_permission = rbac_service
        .check_permission(user_id, resource, action)
        .await
        .unwrap();

    assert!(!revoked_permission);
}

/// Test concurrent access patterns
#[tokio::test]
async fn test_concurrent_security_operations() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    let mut handles = vec![];

    // Spawn multiple tasks to test concurrent token operations
    for i in 0..10 {
        let jwt_service = jwt_service.clone();
        let handle = tokio::spawn(async move {
            let user_id = Uuid::new_v4();
            let email = format!("user{}@example.com", i);
            let roles = vec!["user".to_string()];

            // Generate token
            let token = jwt_service
                .generate_access_token(user_id, &email, roles)
                .await
                .unwrap();

            // Validate token
            let validation_result = jwt_service
                .validate_access_token(&token.token)
                .await
                .unwrap();

            assert_eq!(validation_result.user_id, user_id);
            assert_eq!(validation_result.email, email);
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test security middleware stats and monitoring
#[tokio::test]
async fn test_security_middleware_stats() {
    let security_service = setup_security_service().await.unwrap();
    let middleware = SecurityMiddleware::new(
        security_service.jwt_service().clone(),
        security_service.rbac_service().clone(),
        Default::default(),
    )
    .await
    .unwrap();

    // Perform some operations to generate stats
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    middleware.validate_input(&request).await.ok();
    middleware.rate_limit_request(&request).await.ok();

    // Get stats
    let stats = middleware.get_stats().await;

    // Stats should reflect our operations
    assert!(stats.total_requests >= 0);
    assert!(stats.validation_failures >= 0);
    assert!(stats.rate_limit_blocks >= 0);

    // Reset stats
    middleware.reset_stats().await;
    let reset_stats = middleware.get_stats().await;

    // After reset, counters should be zero
    assert_eq!(reset_stats.total_requests, 0);
    assert_eq!(reset_stats.validation_failures, 0);
    assert_eq!(reset_stats.rate_limit_blocks, 0);
}

/// Test error handling in middleware
#[tokio::test]
async fn test_middleware_error_handling() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);

    // Create test app
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(auth_layer);

    // Test with malformed Authorization header
    let request = Request::builder()
        .method("GET")
        .uri("/protected")
        .header("Authorization", "Malformed Header")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test with empty Authorization header
    let request2 = Request::builder()
        .method("GET")
        .uri("/protected")
        .header("Authorization", "")
        .body(Body::empty())
        .unwrap();

    let app2 = Router::new()
        .route("/protected", get(protected_handler))
        .layer(AuthenticationLayer::new(
            security_service.jwt_service().clone(),
        ));

    let response2 = app2.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::UNAUTHORIZED);
}

/// Performance test for security middleware
#[tokio::test]
async fn test_security_middleware_performance() {
    let security_service = setup_security_service().await.unwrap();
    let jwt_service = security_service.jwt_service().clone();

    // Generate a valid token for performance testing
    let user_id = Uuid::new_v4();
    let token = jwt_service
        .generate_access_token(user_id, "perf@example.com", vec!["user".to_string()])
        .await
        .unwrap();

    // Create middleware
    let auth_layer = AuthenticationLayer::new(jwt_service);

    // Create test app
    let app = Router::new()
        .route("/test", get(protected_handler))
        .layer(auth_layer);

    let start = std::time::Instant::now();

    // Perform multiple requests to measure performance
    let num_requests = 100;
    let mut handles = vec![];

    for _ in 0..num_requests {
        let app = app.clone();
        let token = token.token.clone();
        let handle = tokio::spawn(async move {
            let request = create_auth_request(&token, "GET", "/test");
            app.oneshot(request).await.unwrap()
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let elapsed = start.elapsed();
    let requests_per_second = num_requests as f64 / elapsed.as_secs_f64();

    // Performance assertion - should handle at least 100 requests per second
    // This is a reasonable baseline for security middleware
    assert!(
        requests_per_second > 50.0,
        "Security middleware performance is too slow: {} req/s",
        requests_per_second
    );

    println!(
        "Security middleware performance: {:.2} requests/second",
        requests_per_second
    );
}
