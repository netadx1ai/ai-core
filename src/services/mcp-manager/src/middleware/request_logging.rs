//! Request Logging Middleware
//!
//! This module provides comprehensive request logging middleware for the MCP Manager Service,
//! including request/response logging, performance metrics, and error tracking.

use crate::server::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{error, info, warn, Span};
use uuid::Uuid;

/// Request logging middleware
///
/// Logs incoming HTTP requests with detailed information including:
/// - Request method, URI, and headers
/// - Response status and processing time
/// - Error details for failed requests
/// - Performance metrics
pub async fn request_logging_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();

    // Extract request information
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();
    let headers = request.headers().clone();

    // Generate or extract request ID
    let request_id = extract_or_generate_request_id(&headers);

    // Add request ID to request extensions for downstream use
    request.extensions_mut().insert(request_id.clone());

    // Extract user agent and client info
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    let content_length = headers
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Create request span for tracing
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        uri = %uri,
        version = ?version,
        request_id = %request_id,
        user_agent = user_agent,
        content_length = content_length,
        status_code = tracing::field::Empty,
        response_time_ms = tracing::field::Empty,
    );

    // Enter the span
    let _span_guard = span.enter();

    // Log request start
    if state.config.logging.structured {
        info!(
            method = %method,
            uri = %uri,
            request_id = %request_id,
            user_agent = user_agent,
            content_length = content_length,
            "HTTP request started"
        );
    } else {
        info!("{} {} - Request started (ID: {})", method, uri, request_id);
    }

    // Process the request
    let response = next.run(request).await;

    // Calculate processing time
    let processing_time = start_time.elapsed();
    let processing_time_ms = processing_time.as_millis() as u64;

    // Extract response information
    let status_code = response.status();
    let response_headers = response.headers();

    let response_content_length = response_headers
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Update span with response information
    Span::current().record("status_code", status_code.as_u16());
    Span::current().record("response_time_ms", processing_time_ms);

    // Log response with appropriate level based on status
    let log_level = determine_log_level(&status_code);

    match log_level {
        LogLevel::Info => {
            if state.config.logging.structured {
                info!(
                    method = %method,
                    uri = %uri,
                    request_id = %request_id,
                    status_code = status_code.as_u16(),
                    response_time_ms = processing_time_ms,
                    response_content_length = response_content_length,
                    "HTTP request completed"
                );
            } else {
                info!(
                    "{} {} {} {}ms - Request completed (ID: {})",
                    method, uri, status_code, processing_time_ms, request_id
                );
            }
        }
        LogLevel::Warn => {
            if state.config.logging.structured {
                warn!(
                    method = %method,
                    uri = %uri,
                    request_id = %request_id,
                    status_code = status_code.as_u16(),
                    response_time_ms = processing_time_ms,
                    response_content_length = response_content_length,
                    "HTTP request completed with warning status"
                );
            } else {
                warn!(
                    "{} {} {} {}ms - Request completed with warning (ID: {})",
                    method, uri, status_code, processing_time_ms, request_id
                );
            }
        }
        LogLevel::Error => {
            if state.config.logging.structured {
                error!(
                    method = %method,
                    uri = %uri,
                    request_id = %request_id,
                    status_code = status_code.as_u16(),
                    response_time_ms = processing_time_ms,
                    response_content_length = response_content_length,
                    "HTTP request failed"
                );
            } else {
                error!(
                    "{} {} {} {}ms - Request failed (ID: {})",
                    method, uri, status_code, processing_time_ms, request_id
                );
            }
        }
    }

    // Log slow requests
    if processing_time_ms > 1000 {
        warn!(
            method = %method,
            uri = %uri,
            request_id = %request_id,
            response_time_ms = processing_time_ms,
            "Slow request detected"
        );
    }

    // TODO: Record metrics if enabled
    if state.metrics_enabled() {
        // This would integrate with the telemetry module's metrics
        // record_request_metrics!(metrics, method, status_code, processing_time_ms);
    }

    response
}

/// Extract or generate request ID
fn extract_or_generate_request_id(headers: &HeaderMap) -> String {
    // Try to extract from X-Request-ID header
    if let Some(request_id) = headers.get("x-request-id") {
        if let Ok(id_str) = request_id.to_str() {
            return id_str.to_string();
        }
    }

    // Try to extract from X-Correlation-ID header
    if let Some(correlation_id) = headers.get("x-correlation-id") {
        if let Ok(id_str) = correlation_id.to_str() {
            return id_str.to_string();
        }
    }

    // Generate new UUID
    Uuid::new_v4().to_string()
}

/// Log level for different HTTP status codes
#[derive(Debug, Clone, Copy)]
enum LogLevel {
    Info,
    Warn,
    Error,
}

/// Determine appropriate log level based on HTTP status code
fn determine_log_level(status: &StatusCode) -> LogLevel {
    match status.as_u16() {
        200..=299 => LogLevel::Info,  // 2xx Success
        300..=399 => LogLevel::Info,  // 3xx Redirection
        400..=499 => LogLevel::Warn,  // 4xx Client Error
        500..=599 => LogLevel::Error, // 5xx Server Error
        _ => LogLevel::Info,
    }
}

/// Request logging configuration
#[derive(Debug, Clone)]
pub struct RequestLoggingConfig {
    /// Enable request body logging
    pub log_request_body: bool,
    /// Enable response body logging
    pub log_response_body: bool,
    /// Enable header logging
    pub log_headers: bool,
    /// Maximum body size to log (in bytes)
    pub max_body_size: usize,
    /// Skip logging for certain paths
    pub skip_paths: Vec<String>,
}

impl Default for RequestLoggingConfig {
    fn default() -> Self {
        Self {
            log_request_body: false,
            log_response_body: false,
            log_headers: false,
            max_body_size: 1024, // 1KB
            skip_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/favicon.ico".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, StatusCode};

    #[test]
    fn test_extract_request_id_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("test-request-id"));

        let request_id = extract_or_generate_request_id(&headers);
        assert_eq!(request_id, "test-request-id");
    }

    #[test]
    fn test_extract_correlation_id_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-correlation-id",
            HeaderValue::from_static("test-correlation-id"),
        );

        let request_id = extract_or_generate_request_id(&headers);
        assert_eq!(request_id, "test-correlation-id");
    }

    #[test]
    fn test_generate_request_id_when_no_header() {
        let headers = HeaderMap::new();
        let request_id = extract_or_generate_request_id(&headers);

        // Should be a valid UUID
        assert!(Uuid::parse_str(&request_id).is_ok());
    }

    #[test]
    fn test_determine_log_level() {
        assert!(matches!(
            determine_log_level(&StatusCode::OK),
            LogLevel::Info
        ));
        assert!(matches!(
            determine_log_level(&StatusCode::FOUND),
            LogLevel::Info
        ));
        assert!(matches!(
            determine_log_level(&StatusCode::BAD_REQUEST),
            LogLevel::Warn
        ));
        assert!(matches!(
            determine_log_level(&StatusCode::UNAUTHORIZED),
            LogLevel::Warn
        ));
        assert!(matches!(
            determine_log_level(&StatusCode::INTERNAL_SERVER_ERROR),
            LogLevel::Error
        ));
        assert!(matches!(
            determine_log_level(&StatusCode::BAD_GATEWAY),
            LogLevel::Error
        ));
    }
}
