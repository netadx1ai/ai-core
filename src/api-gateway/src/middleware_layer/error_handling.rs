//! Error handling middleware for graceful API error responses
//!
//! Provides centralized error handling, logging, and response formatting
//! for all API Gateway endpoints.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    state::AppState,
};

/// Error handling middleware that catches and formats all API errors
pub async fn error_handling_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = Uuid::new_v4().to_string();

    // Extract user info if available (for audit logging)
    let user_id = request
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    debug!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        user_id = %user_id,
        "Processing request"
    );

    // Process the request
    let response = next.run(request).await;
    let duration = start_time.elapsed();

    // Check if the response indicates an error
    let status = response.status();

    if status.is_client_error() || status.is_server_error() {
        // Log the error with appropriate level
        if status.is_server_error() {
            error!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                user_id = %user_id,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request failed with server error"
            );

            // Record error metrics
            state.metrics.record_http_request(
                method.as_str(),
                uri.path(),
                status.as_u16(),
                duration,
                "error",
            );
        } else {
            warn!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                user_id = %user_id,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request failed with client error"
            );

            // Record client error metrics
            state.metrics.record_http_request(
                method.as_str(),
                uri.path(),
                status.as_u16(),
                duration,
                "client_error",
            );
        }
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            user_id = %user_id,
            status = %status,
            duration_ms = %duration.as_millis(),
            "Request completed successfully"
        );

        // Record success metrics
        state.metrics.record_http_request(
            method.as_str(),
            uri.path(),
            status.as_u16(),
            duration,
            "success",
        );
    }

    response
}

/// Convert API errors into properly formatted HTTP responses
pub fn handle_api_error(error: ApiError) -> Response {
    let (status, error_code, message, details) = match &error {
        ApiError::Authentication { message } => (
            StatusCode::UNAUTHORIZED,
            "AUTHENTICATION_FAILED",
            message.clone(),
            None,
        ),
        ApiError::Authorization { message } => (
            StatusCode::FORBIDDEN,
            "AUTHORIZATION_FAILED",
            message.clone(),
            None,
        ),
        ApiError::Validation { field, message } => (
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            format!("Validation failed for field '{}': {}", field, message),
            Some(json!({
                "field": field,
                "validation_message": message
            })),
        ),
        ApiError::NotFound { resource } => (
            StatusCode::NOT_FOUND,
            "RESOURCE_NOT_FOUND",
            format!("{} not found", resource),
            Some(json!({
                "resource": resource
            })),
        ),
        ApiError::Conflict { message } => (
            StatusCode::CONFLICT,
            "RESOURCE_CONFLICT",
            message.clone(),
            None,
        ),
        ApiError::Database(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "A database error occurred".to_string(),
            if cfg!(debug_assertions) {
                Some(json!({ "debug_message": err.to_string() }))
            } else {
                None
            },
        ),
        ApiError::ExternalService { service, message } => (
            StatusCode::BAD_GATEWAY,
            "EXTERNAL_SERVICE_ERROR",
            format!("External service '{}' error", service),
            if cfg!(debug_assertions) {
                Some(json!({
                    "service": service,
                    "debug_message": message
                }))
            } else {
                None
            },
        ),
        ApiError::ServiceUnavailable { service } => (
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            format!("Service '{}' is currently unavailable", service),
            Some(json!({ "service": service })),
        ),
        ApiError::Timeout { message } => (
            StatusCode::REQUEST_TIMEOUT,
            "REQUEST_TIMEOUT",
            message.clone(),
            None,
        ),
        ApiError::RequestTooLarge { max_size } => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "REQUEST_TOO_LARGE",
            format!(
                "Request payload too large. Maximum size: {} bytes",
                max_size
            ),
            Some(json!({ "max_size_bytes": max_size })),
        ),
        ApiError::UnsupportedMediaType { media_type } => (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "UNSUPPORTED_MEDIA_TYPE",
            format!("Unsupported media type: {}", media_type),
            Some(json!({ "media_type": media_type })),
        ),
        ApiError::CircuitBreakerOpen { service } => (
            StatusCode::SERVICE_UNAVAILABLE,
            "CIRCUIT_BREAKER_OPEN",
            format!(
                "Service '{}' is currently unavailable (circuit breaker open)",
                service
            ),
            Some(json!({ "service": service })),
        ),
        ApiError::Internal { message } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_SERVER_ERROR",
            "An internal server error occurred".to_string(),
            if cfg!(debug_assertions) {
                Some(json!({ "debug_message": message }))
            } else {
                None
            },
        ),
        ApiError::Redis(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CACHE_ERROR",
            "A cache error occurred".to_string(),
            if cfg!(debug_assertions) {
                Some(json!({ "debug_message": err.to_string() }))
            } else {
                None
            },
        ),
        ApiError::HttpClient(err) => (
            StatusCode::BAD_GATEWAY,
            "HTTP_CLIENT_ERROR",
            "External HTTP request failed".to_string(),
            if cfg!(debug_assertions) {
                Some(json!({ "debug_message": err.to_string() }))
            } else {
                None
            },
        ),
        ApiError::RateLimit { message } => (
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMIT_EXCEEDED",
            message.clone(),
            None,
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_SERVER_ERROR",
            "An internal server error occurred".to_string(),
            None,
        ),
    };

    let request_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Log the error with context
    if status.is_server_error() {
        error!(
            request_id = %request_id,
            error_code = %error_code,
            status = %status.as_u16(),
            message = %message,
            "API error occurred"
        );
    } else {
        warn!(
            request_id = %request_id,
            error_code = %error_code,
            status = %status.as_u16(),
            message = %message,
            "Client error occurred"
        );
    }

    // Create error response body
    let mut error_body = json!({
        "error": {
            "code": error_code,
            "message": message,
            "request_id": request_id,
            "timestamp": timestamp
        }
    });

    // Add details if available
    if let Some(details) = details {
        error_body["error"]["details"] = details;
    }

    // Add debug information in development
    if cfg!(debug_assertions) {
        error_body["error"]["debug"] = json!({
            "rust_error": format!("{:?}", error)
        });
    }

    (status, Json(error_body)).into_response()
}

/// Handle panics and convert them to proper error responses
pub fn handle_panic_error(panic_info: String) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();

    error!(
        request_id = %request_id,
        panic_info = %panic_info,
        "Panic occurred in request handler"
    );

    let error_body = json!({
        "error": {
            "code": "INTERNAL_PANIC",
            "message": "An internal server error occurred",
            "request_id": request_id,
            "timestamp": timestamp
        }
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_body)).into_response()
}

/// Middleware to handle database connection errors gracefully
pub async fn database_error_middleware(request: Request<Body>, next: Next) -> Result<Response> {
    let response = next.run(request).await;
    match response.status() {
        status if status.is_server_error() => {
            // Check if this might be a database connection error
            warn!("Database connection error detected, switching to degraded mode");

            // Return a service unavailable response with retry information
            Ok((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": {
                        "code": "SERVICE_TEMPORARILY_UNAVAILABLE",
                        "message": "Service is temporarily unavailable. Please try again later.",
                        "retry_after_seconds": 30
                    }
                })),
            )
                .into_response())
        }
        _ => {
            // Request succeeded, return response
            Ok(response)
        }
    }
}

/// Health check error handler for degraded mode
pub fn health_check_error_response() -> Response {
    let error_body = json!({
        "status": "degraded",
        "message": "Service is running in degraded mode",
        "details": {
            "database": "unavailable",
            "redis": "unavailable",
            "api_gateway": "available"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    (StatusCode::SERVICE_UNAVAILABLE, Json(error_body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_authentication_error() {
        let error = ApiError::authentication("Invalid token");
        let response = handle_api_error(error);
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_handle_validation_error() {
        let error = ApiError::validation("email", "Invalid email format");
        let response = handle_api_error(error);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_handle_not_found_error() {
        let error = ApiError::not_found("workflow");
        let response = handle_api_error(error);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_handle_internal_error() {
        let error = ApiError::internal("Something went wrong");
        let response = handle_api_error(error);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_health_check_error_response() {
        let response = health_check_error_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
