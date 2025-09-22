//! # Error Types Module
//!
//! This module defines error types used throughout the database-security
//! integration crate. It provides comprehensive error handling for all
//! security, database, and integration operations.

use std::fmt;
use thiserror::Error;

/// Main error type for database-security integration operations
#[derive(Error, Debug)]
pub enum SecureDatabaseError {
    /// Access denied error
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// Multi-factor authentication required
    #[error("MFA required: {0}")]
    MfaRequired(String),

    /// Security context elevation required
    #[error("Elevation required: {0}")]
    ElevationRequired(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// Database operation error
    #[error("Database operation failed: {0}")]
    DatabaseOperation(String),

    /// Encryption error
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    /// Decryption error
    #[error("Decryption error: {0}")]
    DecryptionError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Audit logging error
    #[error("Audit logging error: {0}")]
    AuditError(String),

    /// Cache operation error
    #[error("Cache operation error: {0}")]
    CacheError(String),

    /// Security context error
    #[error("Security context error: {0}")]
    SecurityContextError(String),

    /// Permission validation error
    #[error("Permission validation error: {0}")]
    PermissionError(String),

    /// Role validation error
    #[error("Role validation error: {0}")]
    RoleError(String),

    /// Session error
    #[error("Session error: {0}")]
    SessionError(String),

    /// Key management error
    #[error("Key management error: {0}")]
    KeyManagementError(String),

    /// Data validation error
    #[error("Data validation error: {0}")]
    ValidationError(String),

    /// Resource not found error
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Resource conflict error
    #[error("Resource conflict: {0}")]
    Conflict(String),

    /// Rate limiting error
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Timeout error
    #[error("Operation timeout: {0}")]
    Timeout(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Service unavailable error
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Schema mismatch error
    #[error("Schema mismatch: {0}")]
    SchemaMismatch(String),

    /// Version mismatch error
    #[error("Version mismatch: {0}")]
    VersionMismatch(String),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Backup/restore error
    #[error("Backup/restore error: {0}")]
    BackupRestore(String),

    /// Generic error with context
    #[error("Error in {context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Multiple errors
    #[error("Multiple errors occurred")]
    Multiple(Vec<SecureDatabaseError>),
}

impl SecureDatabaseError {
    /// Create a new access denied error
    pub fn access_denied<S: Into<String>>(message: S) -> Self {
        Self::AccessDenied(message.into())
    }

    /// Create a new MFA required error
    pub fn mfa_required<S: Into<String>>(message: S) -> Self {
        Self::MfaRequired(message.into())
    }

    /// Create a new elevation required error
    pub fn elevation_required<S: Into<String>>(message: S) -> Self {
        Self::ElevationRequired(message.into())
    }

    /// Create a new authentication error
    pub fn authentication_error<S: Into<String>>(message: S) -> Self {
        Self::AuthenticationError(message.into())
    }

    /// Create a new authorization error
    pub fn authorization_error<S: Into<String>>(message: S) -> Self {
        Self::AuthorizationError(message.into())
    }

    /// Create a new database operation error
    pub fn database_operation<S: Into<String>>(message: S) -> Self {
        Self::DatabaseOperation(message.into())
    }

    /// Create a new encryption error
    pub fn encryption_error<S: Into<String>>(message: S) -> Self {
        Self::EncryptionError(message.into())
    }

    /// Create a new decryption error
    pub fn decryption_error<S: Into<String>>(message: S) -> Self {
        Self::DecryptionError(message.into())
    }

    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration(message.into())
    }

    /// Create a new validation error
    pub fn validation_error<S: Into<String>>(message: S) -> Self {
        Self::ValidationError(message.into())
    }

    /// Create a new not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound(resource.into())
    }

    /// Create a new conflict error
    pub fn conflict<S: Into<String>>(message: S) -> Self {
        Self::Conflict(message.into())
    }

    /// Create a new rate limit exceeded error
    pub fn rate_limit_exceeded<S: Into<String>>(message: S) -> Self {
        Self::RateLimitExceeded(message.into())
    }

    /// Create a new timeout error
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::Timeout(message.into())
    }

    /// Create a new internal server error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal(message.into())
    }

    /// Create a new service unavailable error
    pub fn service_unavailable<S: Into<String>>(message: S) -> Self {
        Self::ServiceUnavailable(message.into())
    }

    /// Create a new invalid input error
    pub fn invalid_input<S: Into<String>>(message: S) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Add context to an error
    pub fn with_context<C: Into<String>>(self, context: C) -> Self {
        Self::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Check if error is security-related
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            Self::AccessDenied(_)
                | Self::MfaRequired(_)
                | Self::ElevationRequired(_)
                | Self::AuthenticationError(_)
                | Self::AuthorizationError(_)
                | Self::PermissionError(_)
                | Self::RoleError(_)
                | Self::SecurityContextError(_)
        )
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_)
                | Self::Network(_)
                | Self::ServiceUnavailable(_)
                | Self::RateLimitExceeded(_)
        )
    }

    /// Check if error should be retried
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::ServiceUnavailable(_))
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::AccessDenied(_)
            | Self::MfaRequired(_)
            | Self::ElevationRequired(_)
            | Self::AuthenticationError(_)
            | Self::AuthorizationError(_) => ErrorSeverity::High,

            Self::EncryptionError(_)
            | Self::DecryptionError(_)
            | Self::KeyManagementError(_)
            | Self::Internal(_) => ErrorSeverity::Critical,

            Self::DatabaseOperation(_)
            | Self::Configuration(_)
            | Self::ValidationError(_)
            | Self::SchemaMismatch(_)
            | Self::Migration(_) => ErrorSeverity::Medium,

            Self::NotFound(_)
            | Self::InvalidInput(_)
            | Self::SerializationError(_)
            | Self::DeserializationError(_) => ErrorSeverity::Low,

            Self::RateLimitExceeded(_)
            | Self::Timeout(_)
            | Self::Network(_)
            | Self::ServiceUnavailable(_) => ErrorSeverity::Medium,

            _ => ErrorSeverity::Low,
        }
    }

    /// Get error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::AccessDenied(_)
            | Self::MfaRequired(_)
            | Self::ElevationRequired(_)
            | Self::AuthenticationError(_)
            | Self::AuthorizationError(_)
            | Self::PermissionError(_)
            | Self::RoleError(_)
            | Self::SecurityContextError(_)
            | Self::SessionError(_) => ErrorCategory::Security,

            Self::DatabaseOperation(_)
            | Self::SchemaMismatch(_)
            | Self::Migration(_)
            | Self::BackupRestore(_) => ErrorCategory::Database,

            Self::EncryptionError(_) | Self::DecryptionError(_) | Self::KeyManagementError(_) => {
                ErrorCategory::Encryption
            }

            Self::SerializationError(_)
            | Self::DeserializationError(_)
            | Self::ValidationError(_)
            | Self::InvalidInput(_) => ErrorCategory::Validation,

            Self::Configuration(_) => ErrorCategory::Configuration,

            Self::AuditError(_) => ErrorCategory::Audit,

            Self::CacheError(_) => ErrorCategory::Cache,

            Self::NotFound(_) | Self::Conflict(_) => ErrorCategory::Resource,

            Self::RateLimitExceeded(_) => ErrorCategory::RateLimit,

            Self::Timeout(_) | Self::Network(_) | Self::ServiceUnavailable(_) => {
                ErrorCategory::Network
            }

            Self::Internal(_) => ErrorCategory::Internal,

            _ => ErrorCategory::General,
        }
    }

    /// Convert to HTTP status code
    pub fn to_http_status(&self) -> u16 {
        match self {
            Self::AccessDenied(_) | Self::AuthenticationError(_) | Self::PermissionError(_) => 401, // Unauthorized

            Self::MfaRequired(_) | Self::ElevationRequired(_) | Self::AuthorizationError(_) => 403, // Forbidden

            Self::NotFound(_) => 404, // Not Found

            Self::Conflict(_) => 409, // Conflict

            Self::InvalidInput(_)
            | Self::ValidationError(_)
            | Self::SerializationError(_)
            | Self::DeserializationError(_) => 400, // Bad Request

            Self::RateLimitExceeded(_) => 429, // Too Many Requests

            Self::Timeout(_) => 408, // Request Timeout

            Self::ServiceUnavailable(_) => 503, // Service Unavailable

            Self::Internal(_)
            | Self::EncryptionError(_)
            | Self::DecryptionError(_)
            | Self::DatabaseOperation(_)
            | Self::Configuration(_)
            | Self::AuditError(_)
            | Self::CacheError(_)
            | Self::KeyManagementError(_) => 500, // Internal Server Error

            _ => 500, // Default to Internal Server Error
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Security,
    Database,
    Encryption,
    Validation,
    Configuration,
    Audit,
    Cache,
    Resource,
    RateLimit,
    Network,
    Internal,
    General,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Security => write!(f, "security"),
            Self::Database => write!(f, "database"),
            Self::Encryption => write!(f, "encryption"),
            Self::Validation => write!(f, "validation"),
            Self::Configuration => write!(f, "configuration"),
            Self::Audit => write!(f, "audit"),
            Self::Cache => write!(f, "cache"),
            Self::Resource => write!(f, "resource"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::Network => write!(f, "network"),
            Self::Internal => write!(f, "internal"),
            Self::General => write!(f, "general"),
        }
    }
}

/// Result type alias for database-security operations
pub type SecureDatabaseResult<T> = Result<T, SecureDatabaseError>;

/// Error context trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to the result
    fn with_context<C: Into<String>>(self, context: C) -> SecureDatabaseResult<T>;
}

impl<T> ErrorContext<T> for SecureDatabaseResult<T> {
    fn with_context<C: Into<String>>(self, context: C) -> SecureDatabaseResult<T> {
        self.map_err(|e| e.with_context(context))
    }
}

// Removed conflicting ErrorContext implementation - use anyhow::Context instead

// Conversion implementations for common error types

impl From<anyhow::Error> for SecureDatabaseError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for SecureDatabaseError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for SecureDatabaseError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for SecureDatabaseError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(err.to_string())
    }
}

impl From<uuid::Error> for SecureDatabaseError {
    fn from(err: uuid::Error) -> Self {
        Self::ValidationError(err.to_string())
    }
}

impl From<chrono::ParseError> for SecureDatabaseError {
    fn from(err: chrono::ParseError) -> Self {
        Self::ValidationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = SecureDatabaseError::access_denied("User not authorized");
        assert!(matches!(error, SecureDatabaseError::AccessDenied(_)));
        assert_eq!(error.to_string(), "Access denied: User not authorized");
    }

    #[test]
    fn test_error_severity() {
        let access_denied = SecureDatabaseError::access_denied("test");
        let encryption_error = SecureDatabaseError::encryption_error("test");
        let not_found = SecureDatabaseError::not_found("test");

        assert_eq!(access_denied.severity(), ErrorSeverity::High);
        assert_eq!(encryption_error.severity(), ErrorSeverity::Critical);
        assert_eq!(not_found.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_category() {
        let access_denied = SecureDatabaseError::access_denied("test");
        let database_error = SecureDatabaseError::database_operation("test");
        let encryption_error = SecureDatabaseError::encryption_error("test");

        assert_eq!(access_denied.category(), ErrorCategory::Security);
        assert_eq!(database_error.category(), ErrorCategory::Database);
        assert_eq!(encryption_error.category(), ErrorCategory::Encryption);
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(
            SecureDatabaseError::access_denied("test").to_http_status(),
            401
        );
        assert_eq!(SecureDatabaseError::not_found("test").to_http_status(), 404);
        assert_eq!(SecureDatabaseError::conflict("test").to_http_status(), 409);
        assert_eq!(
            SecureDatabaseError::rate_limit_exceeded("test").to_http_status(),
            429
        );
        assert_eq!(SecureDatabaseError::internal("test").to_http_status(), 500);
    }

    #[test]
    fn test_security_error_detection() {
        let access_denied = SecureDatabaseError::access_denied("test");
        let database_error = SecureDatabaseError::database_operation("test");

        assert!(access_denied.is_security_error());
        assert!(!database_error.is_security_error());
    }

    #[test]
    fn test_recoverable_error_detection() {
        let timeout = SecureDatabaseError::timeout("test");
        let access_denied = SecureDatabaseError::access_denied("test");

        assert!(timeout.is_recoverable());
        assert!(!access_denied.is_recoverable());
    }

    #[test]
    fn test_retry_recommendation() {
        let timeout = SecureDatabaseError::timeout("test");
        let service_unavailable = SecureDatabaseError::service_unavailable("test");
        let access_denied = SecureDatabaseError::access_denied("test");

        assert!(timeout.should_retry());
        assert!(service_unavailable.should_retry());
        assert!(!access_denied.should_retry());
    }

    #[test]
    fn test_error_with_context() {
        let original = SecureDatabaseError::database_operation("Connection failed");
        let with_context = original.with_context("user authentication");

        match with_context {
            SecureDatabaseError::WithContext { context, .. } => {
                assert_eq!(context, "user authentication");
            }
            _ => panic!("Expected WithContext variant"),
        }
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let secure_db_error: SecureDatabaseError = io_error.into();

        assert!(matches!(secure_db_error, SecureDatabaseError::Internal(_)));
    }

    #[test]
    fn test_result_context_extension() {
        let result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Access denied",
        ));

        let with_context = result.with_context("reading config file");
        assert!(with_context.is_err());

        match with_context.unwrap_err() {
            SecureDatabaseError::WithContext { context, .. } => {
                assert_eq!(context, "reading config file");
            }
            _ => panic!("Expected WithContext variant"),
        }
    }
}
