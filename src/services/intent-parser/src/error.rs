use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("LLM API error: {0}")]
    LlmError(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::ConfigurationError(_) => "CONFIGURATION_ERROR",
            AppError::DatabaseError(_) => "DATABASE_ERROR",
            AppError::RedisError(_) => "REDIS_ERROR",
            AppError::LlmError(_) => "LLM_ERROR",
            AppError::ExternalServiceError(_) => "EXTERNAL_SERVICE_ERROR",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::Conflict(_) => "CONFLICT",
            AppError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            AppError::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            AppError::TimeoutError(_) => "TIMEOUT_ERROR",
            AppError::ValidationError(_) => "VALIDATION_ERROR",
            AppError::ParseError(_) => "PARSE_ERROR",
            AppError::InternalServerError(_) => "INTERNAL_SERVER_ERROR",
            AppError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::ParseError(_) => StatusCode::BAD_REQUEST,
            AppError::TimeoutError(_) => StatusCode::REQUEST_TIMEOUT,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::ConfigurationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::LlmError(_) => StatusCode::BAD_GATEWAY,
            AppError::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            AppError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::TimeoutError(_) => true,
            AppError::ServiceUnavailable(_) => true,
            AppError::LlmError(_) => true,
            AppError::ExternalServiceError(_) => true,
            AppError::DatabaseError(_) => true,
            AppError::RedisError(_) => true,
            _ => false,
        }
    }

    pub fn is_client_error(&self) -> bool {
        self.status_code().is_client_error()
    }

    pub fn is_server_error(&self) -> bool {
        self.status_code().is_server_error()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();

        let error_response = ErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            code: format!("{}", status_code.as_u16()),
            details: None,
            timestamp: chrono::Utc::now(),
            request_id: None, // Could be populated from request context
        };

        // Log errors based on severity
        match &self {
            AppError::InternalServerError(_)
            | AppError::DatabaseError(_)
            | AppError::ConfigurationError(_) => {
                tracing::error!("Server error: {:?}", self);
            }
            AppError::ExternalServiceError(_)
            | AppError::LlmError(_)
            | AppError::ServiceUnavailable(_)
            | AppError::TimeoutError(_) => {
                tracing::warn!("External service error: {:?}", self);
            }
            AppError::BadRequest(_) | AppError::ValidationError(_) | AppError::ParseError(_) => {
                tracing::info!("Client error: {:?}", self);
            }
            _ => {
                tracing::debug!("Error: {:?}", self);
            }
        }

        (status_code, Json(error_response)).into_response()
    }
}

// From implementations for common error types

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                AppError::DatabaseError(format!("Database error: {}", db_err))
            }
            sqlx::Error::PoolTimedOut => {
                AppError::TimeoutError("Database connection pool timeout".to_string())
            }
            sqlx::Error::RowNotFound => AppError::NotFound("Database record not found".to_string()),
            _ => AppError::DatabaseError(format!("Database operation failed: {}", err)),
        }
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        match err.kind() {
            redis::ErrorKind::AuthenticationFailed => {
                AppError::Unauthorized("Redis authentication failed".to_string())
            }
            redis::ErrorKind::IoError => {
                AppError::ServiceUnavailable("Redis connection failed".to_string())
            }
            redis::ErrorKind::ResponseError => {
                AppError::ExternalServiceError(format!("Redis response error: {}", err))
            }
            _ => AppError::RedisError(format!("Redis operation failed: {}", err)),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            AppError::TimeoutError(format!("HTTP request timeout: {}", err))
        } else if err.is_connect() {
            AppError::ServiceUnavailable(format!("Connection failed: {}", err))
        } else if err.is_status() {
            if let Some(status) = err.status() {
                match status.as_u16() {
                    400..=499 => AppError::ExternalServiceError(format!("Client error: {}", err)),
                    500..=599 => AppError::ServiceUnavailable(format!("Server error: {}", err)),
                    _ => AppError::ExternalServiceError(format!("HTTP error: {}", err)),
                }
            } else {
                AppError::ExternalServiceError(format!("HTTP error: {}", err))
            }
        } else {
            AppError::ExternalServiceError(format!("Request failed: {}", err))
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::ParseError(format!("JSON parsing failed: {}", err))
    }
}

impl From<uuid::Error> for AppError {
    fn from(err: uuid::Error) -> Self {
        AppError::ValidationError(format!("Invalid UUID: {}", err))
    }
}

impl From<chrono::ParseError> for AppError {
    fn from(err: chrono::ParseError) -> Self {
        AppError::ValidationError(format!("Invalid datetime format: {}", err))
    }
}

impl From<tokio::time::error::Elapsed> for AppError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        AppError::TimeoutError(format!("Operation timeout: {}", err))
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound(format!("File not found: {}", err)),
            std::io::ErrorKind::PermissionDenied => {
                AppError::Forbidden(format!("Permission denied: {}", err))
            }
            std::io::ErrorKind::TimedOut => AppError::TimeoutError(format!("I/O timeout: {}", err)),
            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionAborted => {
                AppError::ServiceUnavailable(format!("Connection failed: {}", err))
            }
            _ => AppError::InternalServerError(format!("I/O error: {}", err)),
        }
    }
}

// Custom error types for specific parsing failures

#[derive(Debug, thiserror::Error)]
pub enum ParseIntentError {
    #[error("Invalid request format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported function: {0}")]
    UnsupportedFunction(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter value: {0}")]
    InvalidParameter(String),

    #[error("Context processing failed: {0}")]
    ContextError(String),

    #[error("LLM response parsing failed: {0}")]
    ResponseParsingError(String),

    #[error("Confidence score too low: {0}")]
    LowConfidence(f32),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),
}

impl From<ParseIntentError> for AppError {
    fn from(err: ParseIntentError) -> Self {
        match err {
            ParseIntentError::InvalidFormat(msg) => AppError::BadRequest(msg),
            ParseIntentError::UnsupportedFunction(msg) => AppError::UnprocessableEntity(msg),
            ParseIntentError::MissingParameter(msg) => AppError::BadRequest(msg),
            ParseIntentError::InvalidParameter(msg) => AppError::BadRequest(msg),
            ParseIntentError::ContextError(msg) => AppError::UnprocessableEntity(msg),
            ParseIntentError::ResponseParsingError(msg) => AppError::ExternalServiceError(msg),
            ParseIntentError::LowConfidence(score) => AppError::UnprocessableEntity(format!(
                "Intent confidence score too low: {:.2}",
                score
            )),
            ParseIntentError::ResourceLimit(msg) => AppError::UnprocessableEntity(msg),
        }
    }
}

// Error context helpers

pub trait ErrorContext<T> {
    fn with_context(self, context: &str) -> Result<T>;
    fn with_context_lazy<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<AppError>,
{
    fn with_context(self, context: &str) -> Result<T> {
        self.map_err(|e| {
            let app_error = e.into();
            match app_error {
                AppError::InternalServerError(msg) => {
                    AppError::InternalServerError(format!("{}: {}", context, msg))
                }
                AppError::DatabaseError(msg) => {
                    AppError::DatabaseError(format!("{}: {}", context, msg))
                }
                AppError::ExternalServiceError(msg) => {
                    AppError::ExternalServiceError(format!("{}: {}", context, msg))
                }
                _ => app_error,
            }
        })
    }

    fn with_context_lazy<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let app_error = e.into();
            let context = f();
            match app_error {
                AppError::InternalServerError(msg) => {
                    AppError::InternalServerError(format!("{}: {}", context, msg))
                }
                AppError::DatabaseError(msg) => {
                    AppError::DatabaseError(format!("{}: {}", context, msg))
                }
                AppError::ExternalServiceError(msg) => {
                    AppError::ExternalServiceError(format!("{}: {}", context, msg))
                }
                _ => app_error,
            }
        })
    }
}

// Utility functions for error handling

pub fn internal_error<T>(msg: impl Into<String>) -> Result<T> {
    Err(AppError::InternalServerError(msg.into()))
}

pub fn bad_request<T>(msg: impl Into<String>) -> Result<T> {
    Err(AppError::BadRequest(msg.into()))
}

pub fn not_found<T>(msg: impl Into<String>) -> Result<T> {
    Err(AppError::NotFound(msg.into()))
}

pub fn validation_error<T>(msg: impl Into<String>) -> Result<T> {
    Err(AppError::ValidationError(msg.into()))
}

pub fn unauthorized<T>(msg: impl Into<String>) -> Result<T> {
    Err(AppError::Unauthorized(msg.into()))
}

pub fn service_unavailable<T>(msg: impl Into<String>) -> Result<T> {
    Err(AppError::ServiceUnavailable(msg.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            AppError::BadRequest("test".to_string()).error_code(),
            "BAD_REQUEST"
        );
        assert_eq!(
            AppError::NotFound("test".to_string()).error_code(),
            "NOT_FOUND"
        );
        assert_eq!(
            AppError::InternalServerError("test".to_string()).error_code(),
            "INTERNAL_SERVER_ERROR"
        );
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            AppError::BadRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::InternalServerError("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_retryable_errors() {
        assert!(AppError::TimeoutError("test".to_string()).is_retryable());
        assert!(AppError::ServiceUnavailable("test".to_string()).is_retryable());
        assert!(!AppError::BadRequest("test".to_string()).is_retryable());
        assert!(!AppError::NotFound("test".to_string()).is_retryable());
    }

    #[test]
    fn test_error_classification() {
        let client_error = AppError::BadRequest("test".to_string());
        let server_error = AppError::InternalServerError("test".to_string());

        assert!(client_error.is_client_error());
        assert!(!client_error.is_server_error());
        assert!(!server_error.is_client_error());
        assert!(server_error.is_server_error());
    }
}
