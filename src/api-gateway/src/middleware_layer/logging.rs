//! Logging and observability middleware for request/response tracking

use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{error::Result, middleware_layer::auth::extract_user_context};

/// Request logging middleware that tracks all HTTP requests
pub async fn logging_middleware(request: Request<Body>, next: Next) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = generate_request_id();

    // Extract matched path for better grouping in metrics
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched_path| matched_path.as_str())
        .unwrap_or_else(|| uri.path())
        .to_string();

    // Extract user context if available
    let user_id = extract_user_context(&request)
        .map(|ctx| ctx.user_id.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    // Extract client IP
    let client_ip = extract_client_ip(&request);

    // Extract user agent
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|header| header.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Create tracing span for this request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = %request_id,
        user_id = %user_id,
        client_ip = %client_ip,
        user_agent = %user_agent,
        status = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
        response_size = tracing::field::Empty,
    );

    let _guard = span.enter();

    info!(
        method = %method,
        uri = %uri,
        user_id = %user_id,
        client_ip = %client_ip,
        user_agent = %user_agent,
        "Request started"
    );

    // Process the request
    let response = next.run(request).await;

    // Calculate request duration
    let duration = start_time.elapsed();
    let status = response.status();

    // Extract response size if available
    let response_size = response
        .headers()
        .get("content-length")
        .and_then(|header| header.to_str().ok())
        .and_then(|size_str| size_str.parse::<u64>().ok())
        .unwrap_or(0);

    // Update span with response information
    span.record("status", status.as_u16());
    span.record("duration_ms", duration.as_millis() as f64);
    span.record("response_size", response_size);

    // Log request completion with appropriate level
    match status {
        status if status.is_server_error() => {
            error!(
                method = %method,
                path = %path,
                status = %status,
                duration_ms = duration.as_millis(),
                user_id = %user_id,
                "Request completed with server error"
            );
        }
        status if status.is_client_error() => {
            warn!(
                method = %method,
                path = %path,
                status = %status,
                duration_ms = duration.as_millis(),
                user_id = %user_id,
                "Request completed with client error"
            );
        }
        _ => {
            info!(
                method = %method,
                path = %path,
                status = %status,
                duration_ms = duration.as_millis(),
                user_id = %user_id,
                response_size = response_size,
                "Request completed successfully"
            );
        }
    }

    response
}

/// Security logging middleware for sensitive operations
pub async fn security_logging_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_context = extract_user_context(&request);
    let user_id_str = user_context
        .as_ref()
        .map(|ctx| ctx.user_id.clone())
        .unwrap_or_else(|| "anonymous".to_string());
    let client_ip = extract_client_ip(&request);
    let path = uri.path().to_string();

    // Check if this is a sensitive operation
    if is_sensitive_operation(&method, &path) {
        info!(
            method = %method,
            path = path,
            user_id = %user_id_str,
            client_ip = %client_ip,
            event_type = "sensitive_operation_attempt",
            "Sensitive operation attempted"
        );
    }

    // Process the request
    let response = next.run(request).await;

    // Log security events based on response
    if is_sensitive_operation(&method, &path) {
        match response.status() {
            StatusCode::UNAUTHORIZED => {
                warn!(
                    method = %method,
                    path = path,
                    user_id = %user_id_str,
                    client_ip = %client_ip,
                    event_type = "authentication_failure",
                    "Authentication failed for sensitive operation"
                );
            }
            StatusCode::FORBIDDEN => {
                warn!(
                    method = %method,
                    path = path,
                    user_id = %user_id_str,
                    client_ip = %client_ip,
                    event_type = "authorization_failure",
                    "Authorization failed for sensitive operation"
                );
            }
            status if status.is_success() => {
                info!(
                    method = %method,
                    path = path,
                    user_id = %user_id_str,
                    client_ip = %client_ip,
                    event_type = "sensitive_operation_success",
                    "Sensitive operation completed successfully"
                );
            }
            _ => {}
        }
    }

    response
}

/// Performance monitoring middleware for slow requests
pub async fn performance_monitoring_middleware(request: Request<Body>, next: Next) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let user_context = extract_user_context(&request);
    let user_id = user_context
        .map(|ctx| ctx.user_id.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    // Process the request
    let response = next.run(request).await;

    let duration = start_time.elapsed();

    // Log slow requests (configurable threshold)
    let slow_request_threshold = Duration::from_millis(1000); // 1 second
    if duration > slow_request_threshold {
        warn!(
            method = %method,
            path = path,
            duration_ms = duration.as_millis(),
            user_id = user_id,
            event_type = "slow_request",
            "Slow request detected"
        );
    }

    // Log very slow requests as errors
    let very_slow_threshold = Duration::from_millis(5000); // 5 seconds
    if duration > very_slow_threshold {
        error!(
            method = %method,
            path = path,
            duration_ms = duration.as_millis(),
            user_id = %user_id,
            event_type = "very_slow_request",
            "Very slow request detected - possible performance issue"
        );
    }

    response
}

/// Extract client IP from request headers
fn extract_client_ip(request: &Request<Body>) -> String {
    // Check X-Forwarded-For header first (for proxy/load balancer)
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // Take the first IP in the chain
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Check CF-Connecting-IP header (Cloudflare)
    if let Some(cf_ip) = request.headers().get("cf-connecting-ip") {
        if let Ok(ip_str) = cf_ip.to_str() {
            return ip_str.to_string();
        }
    }

    "unknown".to_string()
}

/// Generate a unique request ID for tracing
fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// Check if the operation is considered sensitive for security logging
fn is_sensitive_operation(method: &Method, path: &str) -> bool {
    match method {
        &Method::DELETE => true, // All DELETE operations are sensitive
        &Method::POST | &Method::PUT | &Method::PATCH => {
            // Specific sensitive POST/PUT/PATCH operations
            path.contains("/auth/")
                || path.contains("/admin/")
                || path.contains("/users/")
                || path.contains("/federation/")
                || path.contains("/workflows/")
                    && (path.contains("/cancel") || path.contains("/delete"))
        }
        &Method::GET => {
            // Sensitive GET operations
            path.contains("/admin/")
                || path.contains("/analytics/export")
                || path.contains("/users/") && path != "/users/me"
        }
        _ => false,
    }
}

/// Structured log entry for JSON logging
#[derive(serde::Serialize)]
pub struct StructuredLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
    pub request_id: Option<String>,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub user_id: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub response_size: Option<u64>,
    pub event_type: Option<String>,
}

impl StructuredLogEntry {
    pub fn new(
        level: String,
        message: String,
        method: String,
        path: String,
        status: u16,
        duration_ms: u64,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            level,
            message,
            request_id: None,
            method,
            path,
            status,
            duration_ms,
            user_id: None,
            client_ip: None,
            user_agent: None,
            response_size: None,
            event_type: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_client_ip(mut self, client_ip: String) -> Self {
        self.client_ip = Some(client_ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_response_size(mut self, response_size: u64) -> Self {
        self.response_size = Some(response_size);
        self
    }

    pub fn with_event_type(mut self, event_type: String) -> Self {
        self.event_type = Some(event_type);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request};

    #[test]
    fn test_extract_client_ip() {
        // Test with X-Forwarded-For header
        let mut request: Request<Body> =
            Request::builder().uri("/test").body(Body::empty()).unwrap();

        request.headers_mut().insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.168.1.1, 10.0.0.1"),
        );

        let ip = extract_client_ip(&request);
        assert_eq!(ip, "192.168.1.1");

        // Test with X-Real-IP header
        let mut request: Request<Body> =
            Request::builder().uri("/test").body(Body::empty()).unwrap();

        request
            .headers_mut()
            .insert("x-real-ip", HeaderValue::from_static("10.0.0.1"));

        let ip = extract_client_ip(&request);
        assert_eq!(ip, "10.0.0.1");

        // Test with CF-Connecting-IP header
        let mut request: Request<Body> =
            Request::builder().uri("/test").body(Body::empty()).unwrap();

        request
            .headers_mut()
            .insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.1"));

        let ip = extract_client_ip(&request);
        assert_eq!(ip, "203.0.113.1");

        // Test fallback
        let request: Request<Body> = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let ip = extract_client_ip(&request);
        assert_eq!(ip, "unknown");
    }

    #[test]
    fn test_is_sensitive_operation() {
        assert!(is_sensitive_operation(
            &Method::DELETE,
            "/api/v1/workflows/123"
        ));
        assert!(is_sensitive_operation(&Method::POST, "/api/v1/auth/login"));
        assert!(is_sensitive_operation(&Method::GET, "/api/v1/admin/users"));
        assert!(is_sensitive_operation(
            &Method::POST,
            "/api/v1/workflows/123/cancel"
        ));

        assert!(!is_sensitive_operation(&Method::GET, "/api/v1/workflows"));
        assert!(!is_sensitive_operation(&Method::GET, "/api/v1/health"));
        assert!(!is_sensitive_operation(&Method::POST, "/api/v1/workflows"));
    }

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert_ne!(id1, id2);
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
    }

    #[test]
    fn test_structured_log_entry() {
        let entry = StructuredLogEntry::new(
            "INFO".to_string(),
            "Request completed".to_string(),
            "GET".to_string(),
            "/api/v1/workflows".to_string(),
            200,
            150,
        )
        .with_request_id("req_123".to_string())
        .with_user_id("user_456".to_string())
        .with_client_ip("192.168.1.1".to_string())
        .with_event_type("request_completed".to_string());

        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.method, "GET");
        assert_eq!(entry.status, 200);
        assert_eq!(entry.duration_ms, 150);
        assert_eq!(entry.request_id, Some("req_123".to_string()));
        assert_eq!(entry.user_id, Some("user_456".to_string()));
        assert_eq!(entry.client_ip, Some("192.168.1.1".to_string()));
        assert_eq!(entry.event_type, Some("request_completed".to_string()));
    }
}
