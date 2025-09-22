//! Error handling for the API Gateway
//!
//! Provides comprehensive error types and HTTP response mappings.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::error;

pub type Result<T> = std::result::Result<T, ApiError>;

/// Main error type for the API Gateway
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("JWT token error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),

    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    #[error("Authorization failed: {message}")]
    Authorization { message: String },

    #[error("Validation failed: {field}: {message}")]
    Validation { field: String, message: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Resource conflict: {message}")]
    Conflict { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Request timeout: {message}")]
    Timeout { message: String },

    #[error("Request too large: max_size={max_size}")]
    RequestTooLarge { max_size: usize },

    #[error("Unsupported media type: {media_type}")]
    UnsupportedMediaType { media_type: String },

    #[error("Circuit breaker open for service: {service}")]
    CircuitBreakerOpen { service: String },

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Standardized error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<Vec<ErrorDetail>>,
    pub request_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Error detail for validation errors
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub field: String,
    pub error: String,
    pub value: Option<String>,
}

impl ApiError {
    /// Create a new authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create a new authorization error
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    /// Create a new validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a new not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create a new conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Create a new rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        ApiError::RateLimit {
            message: message.into(),
        }
    }

    /// Create a new bad gateway error
    pub fn bad_gateway(message: impl Into<String>) -> Self {
        ApiError::Internal {
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        ApiError::Validation {
            field: "request".to_string(),
            message: message.into(),
        }
    }

    pub fn database(message: impl Into<String>) -> Self {
        ApiError::Internal {
            message: message.into(),
        }
    }

    /// Create configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Create a new external service error
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create a new service unavailable error
    pub fn service_unavailable(service: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
        }
    }

    /// Create a new timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    /// Create a new request too large error
    pub fn request_too_large(max_size: usize) -> Self {
        Self::RequestTooLarge { max_size }
    }

    /// Create a new unsupported media type error
    pub fn unsupported_media_type(media_type: impl Into<String>) -> Self {
        Self::UnsupportedMediaType {
            media_type: media_type.into(),
        }
    }

    /// Create a new circuit breaker open error
    pub fn circuit_breaker_open(service: impl Into<String>) -> Self {
        Self::CircuitBreakerOpen {
            service: service.into(),
        }
    }

    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            ApiError::Authorization { .. } => StatusCode::FORBIDDEN,
            ApiError::Validation { .. } => StatusCode::BAD_REQUEST,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::RequestTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            ApiError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::CircuitBreakerOpen { .. } => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::ExternalService { .. } => StatusCode::BAD_GATEWAY,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::HttpClient(_) => StatusCode::BAD_GATEWAY,
            ApiError::Json(_) => StatusCode::BAD_REQUEST,
            ApiError::Jwt(_) => StatusCode::UNAUTHORIZED,
            ApiError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Generic(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Prometheus(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error type string for API responses
    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::Authentication { .. } => "authentication_error",
            ApiError::Authorization { .. } => "authorization_error",
            ApiError::Validation { .. } => "validation_error",
            ApiError::NotFound { .. } => "not_found_error",
            ApiError::Conflict { .. } => "conflict_error",
            ApiError::RateLimit { .. } => "rate_limit_error",
            ApiError::RequestTooLarge { .. } => "request_too_large_error",
            ApiError::UnsupportedMediaType { .. } => "unsupported_media_type_error",
            ApiError::Timeout { .. } => "timeout_error",
            ApiError::ServiceUnavailable { .. } => "service_unavailable_error",
            ApiError::CircuitBreakerOpen { .. } => "circuit_breaker_error",
            ApiError::ExternalService { .. } => "external_service_error",
            ApiError::Database(_) => "database_error",
            ApiError::Redis(_) => "cache_error",
            ApiError::Config(_) => "configuration_error",
            ApiError::HttpClient(_) => "http_client_error",
            ApiError::Json(_) => "json_error",
            ApiError::Jwt(_) => "jwt_error",
            ApiError::Io(_) => "io_error",
            ApiError::Generic(_) => "generic_error",
            ApiError::Internal { .. } => "internal_error",
            ApiError::Configuration(_) => "configuration_error",
            ApiError::Prometheus(_) => "prometheus_error",
        }
    }

    /// Check if this error should be logged
    pub fn should_log(&self) -> bool {
        match self {
            // Client errors - log as warnings or not at all
            ApiError::Authentication { .. }
            | ApiError::Authorization { .. }
            | ApiError::Validation { .. }
            | ApiError::NotFound { .. }
            | ApiError::Conflict { .. }
            | ApiError::RateLimit { .. }
            | ApiError::RequestTooLarge { .. }
            | ApiError::UnsupportedMediaType { .. } => false,

            // Server errors - log as errors
            _ => true,
        }
    }

    /// Create error details for validation errors
    pub fn with_validation_details(mut self, details: Vec<ErrorDetail>) -> Self {
        match &mut self {
            ApiError::Validation { .. } => {
                // For validation errors, we'll include details in the response
                self
            }
            _ => self,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let error_type = self.error_type();
        let message = self.to_string();

        // Log server errors
        if self.should_log() {
            error!(
                error = %self,
                status_code = %status_code,
                error_type = error_type,
                "API error occurred"
            );
        }

        // Create error response
        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
            details: match &self {
                ApiError::Validation { field, message } => Some(vec![ErrorDetail {
                    field: field.clone(),
                    error: message.clone(),
                    value: None,
                }]),
                _ => None,
            },
            request_id: None, // This will be set by middleware
            timestamp: chrono::Utc::now(),
        };

        (status_code, Json(error_response)).into_response()
    }
}

/// Helper trait for creating validation error collections
pub trait ValidationErrors {
    fn add_error(&mut self, field: &str, message: &str);
    fn has_errors(&self) -> bool;
    fn into_api_error(self) -> ApiError;
}

/// Collection of validation errors
#[derive(Debug, Default)]
pub struct ValidationErrorCollection {
    pub errors: Vec<ErrorDetail>,
}

impl ValidationErrorCollection {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }
}

impl ValidationErrors for ValidationErrorCollection {
    fn add_error(&mut self, field: &str, message: &str) {
        self.errors.push(ErrorDetail {
            field: field.to_string(),
            error: message.to_string(),
            value: None,
        });
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn into_api_error(self) -> ApiError {
        if self.errors.is_empty() {
            ApiError::validation("unknown", "Validation failed")
        } else {
            // Use the first error as the primary error
            let first_error = &self.errors[0];
            ApiError::validation(&first_error.field, &first_error.error)
                .with_validation_details(self.errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            ApiError::authentication("test").status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ApiError::authorization("test").status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ApiError::validation("field", "message").status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::not_found("resource").status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::rate_limit("limit exceeded").status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_types() {
        assert_eq!(
            ApiError::authentication("test").error_type(),
            "authentication_error"
        );
        assert_eq!(
            ApiError::validation("field", "message").error_type(),
            "validation_error"
        );
        assert_eq!(
            ApiError::not_found("resource").error_type(),
            "not_found_error"
        );
    }

    #[test]
    fn test_should_log() {
        assert!(!ApiError::authentication("test").should_log());
        assert!(!ApiError::validation("field", "message").should_log());
        assert!(ApiError::internal("server error").should_log());
        assert!(ApiError::Database(sqlx::Error::RowNotFound).should_log());
    }

    #[test]
    fn test_validation_error_collection() {
        let mut collection = ValidationErrorCollection::new();
        assert!(!collection.has_errors());

        collection.add_error("email", "Invalid email format");
        collection.add_error("password", "Password too short");
        assert!(collection.has_errors());

        let api_error = collection.into_api_error();
        match api_error {
            ApiError::Validation { field, message } => {
                assert_eq!(field, "email");
                assert_eq!(message, "Invalid email format");
            }
            _ => panic!("Expected validation error"),
        }
    }
}
