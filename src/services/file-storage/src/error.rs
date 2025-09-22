use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use thiserror::Error;

/// Result type alias for file storage operations
pub type FileStorageResult<T> = Result<T, FileStorageError>;

/// Main error type for file storage service
#[derive(Error, Debug)]
pub enum FileStorageError {
    // Storage errors
    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("File not found: {file_id}")]
    FileNotFound { file_id: String },

    #[error("File already exists: {file_name}")]
    FileAlreadyExists { file_name: String },

    #[error("Storage quota exceeded: {current}/{limit} bytes")]
    QuotaExceeded { current: usize, limit: usize },

    #[error("File too large: {size}/{max_size} bytes")]
    FileTooLarge { size: usize, max_size: usize },

    // Virus scanning errors
    #[error("File failed virus scan: {reason}")]
    VirusDetected { reason: String },

    #[error("Virus scanner unavailable: {message}")]
    VirusScannerUnavailable { message: String },

    #[error("Virus scan timeout after {timeout} seconds")]
    VirusScanTimeout { timeout: u64 },

    // Processing errors
    #[error("Image processing failed: {message}")]
    ImageProcessingError { message: String },

    #[error("Video processing failed: {message}")]
    VideoProcessingError { message: String },

    #[error("Thumbnail generation failed: {message}")]
    ThumbnailError { message: String },

    #[error("File processing timeout after {timeout} seconds")]
    ProcessingTimeout { timeout: u64 },

    // Validation errors
    #[error("Invalid file type: {mime_type} (allowed: {allowed:?})")]
    InvalidFileType {
        mime_type: String,
        allowed: Vec<String>,
    },

    #[error("Blocked file extension: {extension}")]
    BlockedExtension { extension: String },

    #[error("Invalid file name: {name}")]
    InvalidFileName { name: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid parameter: {parameter} = {value}")]
    InvalidParameter { parameter: String, value: String },

    // Authentication and authorization errors
    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid authentication token: {message}")]
    InvalidToken { message: String },

    #[error("Permission denied: {action} on {resource}")]
    PermissionDenied { action: String, resource: String },

    #[error("Access denied: insufficient privileges")]
    AccessDenied,

    // Database errors
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Connection failed: {service}")]
    ConnectionError { service: String },

    #[error("Transaction failed: {operation}")]
    TransactionError { operation: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    // Configuration errors
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Missing configuration: {key}")]
    MissingConfiguration { key: String },

    #[error("Invalid configuration: {key} = {value}")]
    InvalidConfiguration { key: String, value: String },

    // Rate limiting and resource errors
    #[error("Rate limit exceeded: {limit} requests per {window} seconds")]
    RateLimitExceeded { limit: u32, window: u64 },

    #[error("Too many concurrent uploads: {current}/{max}")]
    TooManyUploads { current: usize, max: usize },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    // IO and network errors
    #[error("IO error: {message}")]
    IoError { message: String },

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("Timeout error: operation timed out after {seconds} seconds")]
    TimeoutError { seconds: u64 },

    // Encryption errors
    #[error("Encryption error: {message}")]
    EncryptionError { message: String },

    #[error("Decryption error: {message}")]
    DecryptionError { message: String },

    #[error("Key management error: {message}")]
    KeyManagementError { message: String },

    // Internal errors
    #[error("Internal server error: {message}")]
    InternalError { message: String },

    #[error("Not implemented: {feature}")]
    NotImplemented { feature: String },

    #[error("Operation cancelled: {operation}")]
    OperationCancelled { operation: String },
}

/// Error response structure for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code for programmatic handling
    pub error: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    pub details: Option<serde_json::Value>,
    /// Request ID for tracking
    pub request_id: Option<String>,
    /// Timestamp of the error
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl FileStorageError {
    /// Convert error to HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            FileStorageError::FileNotFound { .. } => StatusCode::NOT_FOUND,

            FileStorageError::FileAlreadyExists { .. } => StatusCode::CONFLICT,

            FileStorageError::QuotaExceeded { .. } | FileStorageError::FileTooLarge { .. } => {
                StatusCode::PAYLOAD_TOO_LARGE
            }

            FileStorageError::VirusDetected { .. }
            | FileStorageError::InvalidFileType { .. }
            | FileStorageError::BlockedExtension { .. }
            | FileStorageError::InvalidFileName { .. }
            | FileStorageError::MissingField { .. }
            | FileStorageError::InvalidParameter { .. } => StatusCode::BAD_REQUEST,

            FileStorageError::AuthenticationRequired | FileStorageError::InvalidToken { .. } => {
                StatusCode::UNAUTHORIZED
            }

            FileStorageError::PermissionDenied { .. } | FileStorageError::AccessDenied => {
                StatusCode::FORBIDDEN
            }

            FileStorageError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,

            FileStorageError::TooManyUploads { .. }
            | FileStorageError::ServiceUnavailable { .. }
            | FileStorageError::VirusScannerUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,

            FileStorageError::TimeoutError { .. }
            | FileStorageError::VirusScanTimeout { .. }
            | FileStorageError::ProcessingTimeout { .. } => StatusCode::REQUEST_TIMEOUT,

            FileStorageError::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,

            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            FileStorageError::StorageError { .. } => "STORAGE_ERROR",
            FileStorageError::FileNotFound { .. } => "FILE_NOT_FOUND",
            FileStorageError::FileAlreadyExists { .. } => "FILE_ALREADY_EXISTS",
            FileStorageError::QuotaExceeded { .. } => "QUOTA_EXCEEDED",
            FileStorageError::FileTooLarge { .. } => "FILE_TOO_LARGE",
            FileStorageError::VirusDetected { .. } => "VIRUS_DETECTED",
            FileStorageError::VirusScannerUnavailable { .. } => "VIRUS_SCANNER_UNAVAILABLE",
            FileStorageError::VirusScanTimeout { .. } => "VIRUS_SCAN_TIMEOUT",
            FileStorageError::ImageProcessingError { .. } => "IMAGE_PROCESSING_ERROR",
            FileStorageError::VideoProcessingError { .. } => "VIDEO_PROCESSING_ERROR",
            FileStorageError::ThumbnailError { .. } => "THUMBNAIL_ERROR",
            FileStorageError::ProcessingTimeout { .. } => "PROCESSING_TIMEOUT",
            FileStorageError::InvalidFileType { .. } => "INVALID_FILE_TYPE",
            FileStorageError::BlockedExtension { .. } => "BLOCKED_EXTENSION",
            FileStorageError::InvalidFileName { .. } => "INVALID_FILE_NAME",
            FileStorageError::MissingField { .. } => "MISSING_FIELD",
            FileStorageError::InvalidParameter { .. } => "INVALID_PARAMETER",
            FileStorageError::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            FileStorageError::InvalidToken { .. } => "INVALID_TOKEN",
            FileStorageError::PermissionDenied { .. } => "PERMISSION_DENIED",
            FileStorageError::AccessDenied => "ACCESS_DENIED",
            FileStorageError::DatabaseError { .. } => "DATABASE_ERROR",
            FileStorageError::ConnectionError { .. } => "CONNECTION_ERROR",
            FileStorageError::TransactionError { .. } => "TRANSACTION_ERROR",
            FileStorageError::SerializationError { .. } => "SERIALIZATION_ERROR",
            FileStorageError::ConfigurationError { .. } => "CONFIGURATION_ERROR",
            FileStorageError::MissingConfiguration { .. } => "MISSING_CONFIGURATION",
            FileStorageError::InvalidConfiguration { .. } => "INVALID_CONFIGURATION",
            FileStorageError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            FileStorageError::TooManyUploads { .. } => "TOO_MANY_UPLOADS",
            FileStorageError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            FileStorageError::ResourceExhausted { .. } => "RESOURCE_EXHAUSTED",
            FileStorageError::IoError { .. } => "IO_ERROR",
            FileStorageError::NetworkError { .. } => "NETWORK_ERROR",
            FileStorageError::TimeoutError { .. } => "TIMEOUT_ERROR",
            FileStorageError::EncryptionError { .. } => "ENCRYPTION_ERROR",
            FileStorageError::DecryptionError { .. } => "DECRYPTION_ERROR",
            FileStorageError::KeyManagementError { .. } => "KEY_MANAGEMENT_ERROR",
            FileStorageError::InternalError { .. } => "INTERNAL_ERROR",
            FileStorageError::NotImplemented { .. } => "NOT_IMPLEMENTED",
            FileStorageError::OperationCancelled { .. } => "OPERATION_CANCELLED",
        }
    }

    /// Create error response for API
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            details: self.get_details(),
            request_id: None, // Set by middleware
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get additional error details
    fn get_details(&self) -> Option<serde_json::Value> {
        match self {
            FileStorageError::InvalidFileType { mime_type, allowed } => Some(serde_json::json!({
                "mime_type": mime_type,
                "allowed_types": allowed
            })),
            FileStorageError::QuotaExceeded { current, limit } => Some(serde_json::json!({
                "current_usage": current,
                "quota_limit": limit
            })),
            FileStorageError::FileTooLarge { size, max_size } => Some(serde_json::json!({
                "file_size": size,
                "max_size": max_size
            })),
            FileStorageError::RateLimitExceeded { limit, window } => Some(serde_json::json!({
                "rate_limit": limit,
                "window_seconds": window
            })),
            _ => None,
        }
    }
}

impl IntoResponse for FileStorageError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = self.to_error_response();
        (status, Json(error_response)).into_response()
    }
}

// Conversion implementations for common error types

impl From<std::io::Error> for FileStorageError {
    fn from(err: std::io::Error) -> Self {
        FileStorageError::IoError {
            message: err.to_string(),
        }
    }
}

impl From<sqlx::Error> for FileStorageError {
    fn from(err: sqlx::Error) -> Self {
        FileStorageError::DatabaseError {
            message: err.to_string(),
        }
    }
}

impl From<mongodb::error::Error> for FileStorageError {
    fn from(err: mongodb::error::Error) -> Self {
        FileStorageError::DatabaseError {
            message: err.to_string(),
        }
    }
}

impl From<redis::RedisError> for FileStorageError {
    fn from(err: redis::RedisError) -> Self {
        FileStorageError::DatabaseError {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for FileStorageError {
    fn from(err: serde_json::Error) -> Self {
        FileStorageError::SerializationError {
            message: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for FileStorageError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            FileStorageError::TimeoutError {
                seconds: 60, // Default timeout
            }
        } else if err.is_connect() {
            FileStorageError::NetworkError {
                message: format!("Connection error: {}", err),
            }
        } else {
            FileStorageError::NetworkError {
                message: err.to_string(),
            }
        }
    }
}

impl From<image::ImageError> for FileStorageError {
    fn from(err: image::ImageError) -> Self {
        FileStorageError::ImageProcessingError {
            message: err.to_string(),
        }
    }
}

impl From<jsonwebtoken::errors::Error> for FileStorageError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        FileStorageError::InvalidToken {
            message: err.to_string(),
        }
    }
}

impl From<config::ConfigError> for FileStorageError {
    fn from(err: config::ConfigError) -> Self {
        FileStorageError::ConfigurationError {
            message: err.to_string(),
        }
    }
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>>
    for FileStorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>,
    ) -> Self {
        FileStorageError::StorageError {
            message: format!("S3 GetObject error: {}", err),
        }
    }
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>>
    for FileStorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>,
    ) -> Self {
        FileStorageError::StorageError {
            message: format!("S3 PutObject error: {}", err),
        }
    }
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::delete_object::DeleteObjectError>>
    for FileStorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::delete_object::DeleteObjectError>,
    ) -> Self {
        FileStorageError::StorageError {
            message: format!("S3 DeleteObject error: {}", err),
        }
    }
}

// Utility functions for creating common errors

impl FileStorageError {
    pub fn file_not_found<S: Into<String>>(file_id: S) -> Self {
        Self::FileNotFound {
            file_id: file_id.into(),
        }
    }

    pub fn permission_denied<S: Into<String>>(action: S, resource: S) -> Self {
        Self::PermissionDenied {
            action: action.into(),
            resource: resource.into(),
        }
    }

    pub fn invalid_file_type<S: Into<String>>(mime_type: S, allowed: Vec<String>) -> Self {
        Self::InvalidFileType {
            mime_type: mime_type.into(),
            allowed,
        }
    }

    pub fn quota_exceeded(current: usize, limit: usize) -> Self {
        Self::QuotaExceeded { current, limit }
    }

    pub fn file_too_large(size: usize, max_size: usize) -> Self {
        Self::FileTooLarge { size, max_size }
    }

    pub fn virus_detected<S: Into<String>>(reason: S) -> Self {
        Self::VirusDetected {
            reason: reason.into(),
        }
    }

    pub fn internal_error<S: Into<String>>(message: S) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            FileStorageError::file_not_found("test").status_code(),
            StatusCode::NOT_FOUND
        );

        assert_eq!(
            FileStorageError::AuthenticationRequired.status_code(),
            StatusCode::UNAUTHORIZED
        );

        assert_eq!(
            FileStorageError::permission_denied("read", "file123").status_code(),
            StatusCode::FORBIDDEN
        );

        assert_eq!(
            FileStorageError::quota_exceeded(1000, 500).status_code(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            FileStorageError::file_not_found("test").error_code(),
            "FILE_NOT_FOUND"
        );

        assert_eq!(
            FileStorageError::virus_detected("malware").error_code(),
            "VIRUS_DETECTED"
        );

        assert_eq!(
            FileStorageError::invalid_file_type("text/plain", vec!["image/jpeg".to_string()])
                .error_code(),
            "INVALID_FILE_TYPE"
        );
    }

    #[test]
    fn test_error_response() {
        let error = FileStorageError::quota_exceeded(1000, 500);
        let response = error.to_error_response();

        assert_eq!(response.error, "QUOTA_EXCEEDED");
        assert!(response.message.contains("quota exceeded"));
        assert!(response.details.is_some());
    }
}
