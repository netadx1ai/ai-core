//! Error handling module for the AI-CORE Integration Service
//!
//! This module provides comprehensive error types and handling for all integration
//! operations including webhook processing, API calls, and authentication.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Result type alias for integration operations
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Comprehensive error types for the integration service
#[derive(Error, Debug)]
pub enum IntegrationError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Database operation errors
    #[error("Database error: {source}")]
    Database {
        #[from]
        source: sqlx::Error,
    },

    /// Redis operation errors
    #[error("Redis error: {source}")]
    Redis {
        #[from]
        source: redis::RedisError,
    },

    /// HTTP client errors
    #[error("HTTP client error: {source}")]
    HttpClient {
        #[from]
        source: reqwest::Error,
    },

    /// Serialization/deserialization errors
    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        source: serde_json::Error,
    },

    /// Authentication and authorization errors
    #[error("Authentication error: {message}")]
    Authentication { message: String },

    /// Authorization errors
    #[error("Authorization error: {message}")]
    Authorization { message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded for {resource}")]
    RateLimit { resource: String },

    /// Webhook signature verification errors
    #[error("Webhook signature verification failed for {integration}: {reason}")]
    SignatureVerification { integration: String, reason: String },

    /// Integration-specific errors
    #[error("Zapier integration error: {message}")]
    Zapier { message: String },

    #[error("Slack integration error: {message}")]
    Slack { message: String },

    #[error("GitHub integration error: {message}")]
    GitHub { message: String },

    /// OAuth flow errors
    #[error("OAuth error for {provider}: {message}")]
    OAuth { provider: String, message: String },

    /// Webhook processing errors
    #[error("Webhook processing error: {message}")]
    WebhookProcessing { message: String },

    /// Template rendering errors
    #[error("Template rendering error: {message}")]
    TemplateRendering { message: String },

    /// Circuit breaker errors
    #[error("Circuit breaker open for {service}")]
    CircuitBreaker { service: String },

    /// Timeout errors
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Invalid payload errors
    #[error("Invalid payload for {integration}: {reason}")]
    InvalidPayload { integration: String, reason: String },

    /// External API errors
    #[error("External API error for {service}: {status_code} - {message}")]
    ExternalApi {
        service: String,
        status_code: u16,
        message: String,
    },

    /// Validation errors
    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    /// Internal server errors
    #[error("Internal server error: {message}")]
    Internal { message: String },

    /// Not found errors
    #[error("{resource} not found")]
    NotFound { resource: String },

    /// Service unavailable errors
    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    /// Workflow execution errors
    #[error("Workflow execution error: {workflow_id} - {message}")]
    WorkflowExecution {
        workflow_id: String,
        message: String,
    },
}

impl IntegrationError {
    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a new authentication error
    pub fn authentication<S: Into<String>>(message: S) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create a new authorization error
    pub fn authorization<S: Into<String>>(message: S) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    /// Create a new rate limit error
    pub fn rate_limit<S: Into<String>>(resource: S) -> Self {
        Self::RateLimit {
            resource: resource.into(),
        }
    }

    /// Create a new signature verification error
    pub fn signature_verification<S1: Into<String>, S2: Into<String>>(
        integration: S1,
        reason: S2,
    ) -> Self {
        Self::SignatureVerification {
            integration: integration.into(),
            reason: reason.into(),
        }
    }

    /// Create a new Zapier error
    pub fn zapier<S: Into<String>>(message: S) -> Self {
        Self::Zapier {
            message: message.into(),
        }
    }

    /// Create a new Slack error
    pub fn slack<S: Into<String>>(message: S) -> Self {
        Self::Slack {
            message: message.into(),
        }
    }

    /// Create a new GitHub error
    pub fn github<S: Into<String>>(message: S) -> Self {
        Self::GitHub {
            message: message.into(),
        }
    }

    /// Create a new OAuth error
    pub fn oauth<S1: Into<String>, S2: Into<String>>(provider: S1, message: S2) -> Self {
        Self::OAuth {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Create a new webhook processing error
    pub fn webhook_processing<S: Into<String>>(message: S) -> Self {
        Self::WebhookProcessing {
            message: message.into(),
        }
    }

    /// Create a new circuit breaker error
    pub fn circuit_breaker<S: Into<String>>(service: S) -> Self {
        Self::CircuitBreaker {
            service: service.into(),
        }
    }

    /// Create a new timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }

    /// Create a new invalid payload error
    pub fn invalid_payload<S1: Into<String>, S2: Into<String>>(
        integration: S1,
        reason: S2,
    ) -> Self {
        Self::InvalidPayload {
            integration: integration.into(),
            reason: reason.into(),
        }
    }

    /// Create a new external API error
    pub fn external_api<S1: Into<String>, S2: Into<String>>(
        service: S1,
        status_code: u16,
        message: S2,
    ) -> Self {
        Self::ExternalApi {
            service: service.into(),
            status_code,
            message: message.into(),
        }
    }

    /// Create a new validation error
    pub fn validation<S1: Into<String>, S2: Into<String>>(field: S1, message: S2) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a new not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create a new service unavailable error
    pub fn service_unavailable<S: Into<String>>(service: S) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
        }
    }

    /// Create a new workflow execution error
    pub fn workflow_execution<S1: Into<String>, S2: Into<String>>(
        workflow_id: S1,
        message: S2,
    ) -> Self {
        Self::WorkflowExecution {
            workflow_id: workflow_id.into(),
            message: message.into(),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            IntegrationError::Configuration { .. } => StatusCode::BAD_REQUEST,
            IntegrationError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            IntegrationError::Authorization { .. } => StatusCode::FORBIDDEN,
            IntegrationError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            IntegrationError::SignatureVerification { .. } => StatusCode::UNAUTHORIZED,
            IntegrationError::OAuth { .. } => StatusCode::BAD_REQUEST,
            IntegrationError::InvalidPayload { .. } => StatusCode::BAD_REQUEST,
            IntegrationError::Validation { .. } => StatusCode::BAD_REQUEST,
            IntegrationError::NotFound { .. } => StatusCode::NOT_FOUND,
            IntegrationError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            IntegrationError::CircuitBreaker { .. } => StatusCode::SERVICE_UNAVAILABLE,
            IntegrationError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            IntegrationError::ExternalApi { status_code, .. } => {
                StatusCode::from_u16(*status_code).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            IntegrationError::Database { .. }
            | IntegrationError::Redis { .. }
            | IntegrationError::HttpClient { .. }
            | IntegrationError::Serialization { .. }
            | IntegrationError::Zapier { .. }
            | IntegrationError::Slack { .. }
            | IntegrationError::GitHub { .. }
            | IntegrationError::WebhookProcessing { .. }
            | IntegrationError::TemplateRendering { .. }
            | IntegrationError::Internal { .. }
            | IntegrationError::WorkflowExecution { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error code for this error (for API responses)
    pub fn error_code(&self) -> &'static str {
        match self {
            IntegrationError::Configuration { .. } => "CONFIGURATION_ERROR",
            IntegrationError::Database { .. } => "DATABASE_ERROR",
            IntegrationError::Redis { .. } => "CACHE_ERROR",
            IntegrationError::HttpClient { .. } => "HTTP_CLIENT_ERROR",
            IntegrationError::Serialization { .. } => "SERIALIZATION_ERROR",
            IntegrationError::Authentication { .. } => "AUTHENTICATION_ERROR",
            IntegrationError::Authorization { .. } => "AUTHORIZATION_ERROR",
            IntegrationError::RateLimit { .. } => "RATE_LIMIT_EXCEEDED",
            IntegrationError::SignatureVerification { .. } => "SIGNATURE_VERIFICATION_FAILED",
            IntegrationError::Zapier { .. } => "ZAPIER_ERROR",
            IntegrationError::Slack { .. } => "SLACK_ERROR",
            IntegrationError::GitHub { .. } => "GITHUB_ERROR",
            IntegrationError::OAuth { .. } => "OAUTH_ERROR",
            IntegrationError::WebhookProcessing { .. } => "WEBHOOK_PROCESSING_ERROR",
            IntegrationError::TemplateRendering { .. } => "TEMPLATE_RENDERING_ERROR",
            IntegrationError::CircuitBreaker { .. } => "CIRCUIT_BREAKER_OPEN",
            IntegrationError::Timeout { .. } => "TIMEOUT",
            IntegrationError::InvalidPayload { .. } => "INVALID_PAYLOAD",
            IntegrationError::ExternalApi { .. } => "EXTERNAL_API_ERROR",
            IntegrationError::Validation { .. } => "VALIDATION_ERROR",
            IntegrationError::Internal { .. } => "INTERNAL_ERROR",
            IntegrationError::NotFound { .. } => "NOT_FOUND",
            IntegrationError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            IntegrationError::WorkflowExecution { .. } => "WORKFLOW_EXECUTION_ERROR",
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            IntegrationError::Database { .. }
            | IntegrationError::Redis { .. }
            | IntegrationError::HttpClient { .. }
            | IntegrationError::ServiceUnavailable { .. }
            | IntegrationError::Timeout { .. } => true,
            IntegrationError::ExternalApi { status_code, .. } => *status_code >= 500,
            _ => false,
        }
    }
}

impl IntoResponse for IntegrationError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let error_code = self.error_code();
        let error_message = self.to_string();

        tracing::error!(
            error_code = error_code,
            error_message = %error_message,
            "Integration service error"
        );

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": error_message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "retryable": self.is_retryable()
            }
        }));

        (status_code, body).into_response()
    }
}

/// Helper trait for converting errors to IntegrationError
pub trait IntoIntegrationError<T> {
    fn into_integration_error(self) -> IntegrationResult<T>;
}

impl<T, E> IntoIntegrationError<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_integration_error(self) -> IntegrationResult<T> {
        self.map_err(|e| IntegrationError::internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = IntegrationError::zapier("Test error");
        assert_eq!(error.to_string(), "Zapier integration error: Test error");
        assert_eq!(error.error_code(), "ZAPIER_ERROR");
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_authentication_error() {
        let error = IntegrationError::authentication("Invalid token");
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(error.error_code(), "AUTHENTICATION_ERROR");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_rate_limit_error() {
        let error = IntegrationError::rate_limit("API calls");
        assert_eq!(error.status_code(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(error.error_code(), "RATE_LIMIT_EXCEEDED");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_external_api_error() {
        let error = IntegrationError::external_api("GitHub", 503, "Service unavailable");
        assert_eq!(error.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.error_code(), "EXTERNAL_API_ERROR");
        assert!(error.is_retryable());
    }

    #[test]
    fn test_retryable_errors() {
        let timeout_error = IntegrationError::timeout(30);
        assert!(timeout_error.is_retryable());

        let validation_error = IntegrationError::validation("field", "Invalid value");
        assert!(!validation_error.is_retryable());

        let circuit_breaker_error = IntegrationError::circuit_breaker("slack");
        assert!(!circuit_breaker_error.is_retryable());
    }

    #[test]
    fn test_signature_verification_error() {
        let error = IntegrationError::signature_verification("zapier", "Invalid signature");
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
        assert!(error
            .to_string()
            .contains("Webhook signature verification failed"));
    }

    #[test]
    fn test_workflow_execution_error() {
        let error = IntegrationError::workflow_execution("wf-123", "Step failed");
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error.to_string().contains("wf-123"));
        assert!(error.to_string().contains("Step failed"));
    }
}
