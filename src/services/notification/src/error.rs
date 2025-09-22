//! Error handling for the notification service
//!
//! This module defines all error types that can occur in the notification service
//! and provides utilities for error handling and conversion.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

/// Result type alias for notification service operations
pub type Result<T> = std::result::Result<T, NotificationError>;

/// Main error type for the notification service
#[derive(Error, Debug)]
pub enum NotificationError {
    /// Database-related errors
    #[error("Database error: {message}")]
    Database { message: String },

    /// Redis/caching errors
    #[error("Cache error: {message}")]
    Cache { message: String },

    /// Email delivery errors
    #[error("Email error: {message}")]
    Email { message: String },

    /// SMS delivery errors
    #[error("SMS error: {message}")]
    Sms { message: String },

    /// Push notification errors
    #[error("Push notification error: {message}")]
    Push { message: String },

    /// Webhook delivery errors
    #[error("Webhook error: {message}")]
    Webhook { message: String },

    /// WebSocket connection errors
    #[error("WebSocket error: {message}")]
    WebSocket { message: String },

    /// Template processing errors
    #[error("Template error: {message}")]
    Template { message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Validation errors
    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    /// Authentication/authorization errors
    #[error("Authentication error: {message}")]
    Auth { message: String },

    /// Not found errors
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Conflict errors (e.g., duplicate entries)
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// Timeout errors
    #[error("Operation timed out: {operation}")]
    Timeout { operation: String },

    /// Network/connection errors
    #[error("Network error: {message}")]
    Network { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// External service errors
    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    /// Internal service errors
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Service unavailable errors
    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    /// Business logic errors
    #[error("Business logic error: {message}")]
    BusinessLogic { message: String },

    /// Resource exhaustion errors
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
}

impl NotificationError {
    /// Get the HTTP status code that should be returned for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            NotificationError::Database { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NotificationError::Cache { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NotificationError::Email { .. } => StatusCode::BAD_GATEWAY,
            NotificationError::Sms { .. } => StatusCode::BAD_GATEWAY,
            NotificationError::Push { .. } => StatusCode::BAD_GATEWAY,
            NotificationError::Webhook { .. } => StatusCode::BAD_GATEWAY,
            NotificationError::WebSocket { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NotificationError::Template { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NotificationError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            NotificationError::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NotificationError::Validation { .. } => StatusCode::BAD_REQUEST,
            NotificationError::Auth { .. } => StatusCode::UNAUTHORIZED,
            NotificationError::NotFound { .. } => StatusCode::NOT_FOUND,
            NotificationError::Conflict { .. } => StatusCode::CONFLICT,
            NotificationError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            NotificationError::Network { .. } => StatusCode::BAD_GATEWAY,
            NotificationError::Serialization { .. } => StatusCode::BAD_REQUEST,
            NotificationError::ExternalService { .. } => StatusCode::BAD_GATEWAY,
            NotificationError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            NotificationError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            NotificationError::BusinessLogic { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            NotificationError::ResourceExhausted { .. } => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    /// Get the error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            NotificationError::Database { .. } => "DATABASE_ERROR",
            NotificationError::Cache { .. } => "CACHE_ERROR",
            NotificationError::Email { .. } => "EMAIL_ERROR",
            NotificationError::Sms { .. } => "SMS_ERROR",
            NotificationError::Push { .. } => "PUSH_ERROR",
            NotificationError::Webhook { .. } => "WEBHOOK_ERROR",
            NotificationError::WebSocket { .. } => "WEBSOCKET_ERROR",
            NotificationError::Template { .. } => "TEMPLATE_ERROR",
            NotificationError::RateLimit { .. } => "RATE_LIMIT_EXCEEDED",
            NotificationError::Config { .. } => "CONFIG_ERROR",
            NotificationError::Validation { .. } => "VALIDATION_ERROR",
            NotificationError::Auth { .. } => "AUTH_ERROR",
            NotificationError::NotFound { .. } => "NOT_FOUND",
            NotificationError::Conflict { .. } => "CONFLICT",
            NotificationError::Timeout { .. } => "TIMEOUT",
            NotificationError::Network { .. } => "NETWORK_ERROR",
            NotificationError::Serialization { .. } => "SERIALIZATION_ERROR",
            NotificationError::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            NotificationError::Internal { .. } => "INTERNAL_ERROR",
            NotificationError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            NotificationError::BusinessLogic { .. } => "BUSINESS_LOGIC_ERROR",
            NotificationError::ResourceExhausted { .. } => "RESOURCE_EXHAUSTED",
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            NotificationError::Database { .. } => true,
            NotificationError::Cache { .. } => true,
            NotificationError::Email { .. } => true,
            NotificationError::Sms { .. } => true,
            NotificationError::Push { .. } => true,
            NotificationError::Webhook { .. } => true,
            NotificationError::WebSocket { .. } => false,
            NotificationError::Template { .. } => false,
            NotificationError::RateLimit { .. } => true,
            NotificationError::Config { .. } => false,
            NotificationError::Validation { .. } => false,
            NotificationError::Auth { .. } => false,
            NotificationError::NotFound { .. } => false,
            NotificationError::Conflict { .. } => false,
            NotificationError::Timeout { .. } => true,
            NotificationError::Network { .. } => true,
            NotificationError::Serialization { .. } => false,
            NotificationError::ExternalService { .. } => true,
            NotificationError::Internal { .. } => true,
            NotificationError::ServiceUnavailable { .. } => true,
            NotificationError::BusinessLogic { .. } => false,
            NotificationError::ResourceExhausted { .. } => true,
        }
    }
}

impl IntoResponse for NotificationError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
                "status": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}

// Conversion implementations for external error types

impl From<sqlx::Error> for NotificationError {
    fn from(err: sqlx::Error) -> Self {
        NotificationError::Database {
            message: err.to_string(),
        }
    }
}

impl From<mongodb::error::Error> for NotificationError {
    fn from(err: mongodb::error::Error) -> Self {
        NotificationError::Database {
            message: err.to_string(),
        }
    }
}

impl From<redis::RedisError> for NotificationError {
    fn from(err: redis::RedisError) -> Self {
        NotificationError::Cache {
            message: err.to_string(),
        }
    }
}

impl From<lettre::error::Error> for NotificationError {
    fn from(err: lettre::error::Error) -> Self {
        NotificationError::Email {
            message: err.to_string(),
        }
    }
}

impl From<lettre::transport::smtp::Error> for NotificationError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        NotificationError::Email {
            message: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for NotificationError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            NotificationError::Timeout {
                operation: "HTTP request".to_string(),
            }
        } else if err.is_connect() {
            NotificationError::Network {
                message: err.to_string(),
            }
        } else {
            NotificationError::ExternalService {
                service: "HTTP".to_string(),
                message: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for NotificationError {
    fn from(err: serde_json::Error) -> Self {
        NotificationError::Serialization {
            message: err.to_string(),
        }
    }
}

impl From<handlebars::RenderError> for NotificationError {
    fn from(err: handlebars::RenderError) -> Self {
        NotificationError::Template {
            message: err.to_string(),
        }
    }
}

impl From<handlebars::TemplateError> for NotificationError {
    fn from(err: handlebars::TemplateError) -> Self {
        NotificationError::Template {
            message: err.to_string(),
        }
    }
}

impl From<config::ConfigError> for NotificationError {
    fn from(err: config::ConfigError) -> Self {
        NotificationError::Config {
            message: err.to_string(),
        }
    }
}

impl From<tokio::time::error::Elapsed> for NotificationError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        NotificationError::Timeout {
            operation: err.to_string(),
        }
    }
}

impl From<validator::ValidationErrors> for NotificationError {
    fn from(err: validator::ValidationErrors) -> Self {
        let message = err
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                let field_errors: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        e.message
                            .as_ref()
                            .unwrap_or(&"Invalid value".into())
                            .to_string()
                    })
                    .collect();
                format!("{}: {}", field, field_errors.join(", "))
            })
            .collect::<Vec<String>>()
            .join("; ");

        NotificationError::Validation {
            field: "multiple".to_string(),
            message,
        }
    }
}

// Utility functions for creating specific error types

impl NotificationError {
    /// Create a database error
    pub fn database<S: Into<String>>(message: S) -> Self {
        Self::Database {
            message: message.into(),
        }
    }

    /// Create a cache error
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Create an email error
    pub fn email<S: Into<String>>(message: S) -> Self {
        Self::Email {
            message: message.into(),
        }
    }

    /// Create an SMS error
    pub fn sms<S: Into<String>>(message: S) -> Self {
        Self::Sms {
            message: message.into(),
        }
    }

    /// Create a push notification error
    pub fn push<S: Into<String>>(message: S) -> Self {
        Self::Push {
            message: message.into(),
        }
    }

    /// Create a webhook error
    pub fn webhook<S: Into<String>>(message: S) -> Self {
        Self::Webhook {
            message: message.into(),
        }
    }

    /// Create a WebSocket error
    pub fn websocket<S: Into<String>>(message: S) -> Self {
        Self::WebSocket {
            message: message.into(),
        }
    }

    /// Create a template error
    pub fn template<S: Into<String>>(message: S) -> Self {
        Self::Template {
            message: message.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>>(message: S) -> Self {
        Self::RateLimit {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation<S1: Into<String>, S2: Into<String>>(field: S1, message: S2) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn auth<S: Into<String>>(message: S) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create a conflict error
    pub fn conflict<S: Into<String>>(message: S) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(operation: S) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// Create a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization<S: Into<String>>(message: S) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Create an external service error
    pub fn external_service<S1: Into<String>, S2: Into<String>>(service: S1, message: S2) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a service unavailable error
    pub fn service_unavailable<S: Into<String>>(service: S) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
        }
    }

    /// Create a business logic error
    pub fn business_logic<S: Into<String>>(message: S) -> Self {
        Self::BusinessLogic {
            message: message.into(),
        }
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted<S: Into<String>>(resource: S) -> Self {
        Self::ResourceExhausted {
            resource: resource.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            NotificationError::database("test").status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            NotificationError::validation("field", "message").status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            NotificationError::not_found("resource").status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            NotificationError::rate_limit("too many requests").status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            NotificationError::database("test").error_code(),
            "DATABASE_ERROR"
        );
        assert_eq!(
            NotificationError::validation("field", "message").error_code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(
            NotificationError::not_found("resource").error_code(),
            "NOT_FOUND"
        );
    }

    #[test]
    fn test_retryable_errors() {
        assert!(NotificationError::database("test").is_retryable());
        assert!(NotificationError::timeout("operation").is_retryable());
        assert!(!NotificationError::validation("field", "message").is_retryable());
        assert!(!NotificationError::not_found("resource").is_retryable());
    }

    #[test]
    fn test_error_display() {
        let error = NotificationError::database("Connection failed");
        assert_eq!(error.to_string(), "Database error: Connection failed");
    }

    #[test]
    fn test_from_conversions() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_error.is_err());
        let notification_error: NotificationError = json_error.unwrap_err().into();
        matches!(notification_error, NotificationError::Serialization { .. });
    }
}
