//! Security Error Types and Handling
//!
//! Comprehensive error types for all security operations including authentication,
//! authorization, encryption, and validation errors with proper error chaining.

use std::fmt;
use thiserror::Error;

/// Main security error type
#[derive(Error, Debug)]
pub enum SecurityError {
    // Authentication errors
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired: {0}")]
    TokenExpired(String),

    #[error("Token generation failed: {0}")]
    TokenGeneration(String),

    #[error("Token validation failed: {0}")]
    TokenValidation(String),

    #[error("Token blacklisted: {0}")]
    TokenBlacklisted(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Account locked: {0}")]
    AccountLocked(String),

    #[error("Multi-factor authentication required")]
    MfaRequired,

    #[error("Invalid MFA code")]
    InvalidMfaCode,

    // Authorization errors
    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Insufficient permissions: required {required}, found {found}")]
    InsufficientPermissions { required: String, found: String },

    #[error("Permission evaluation failed: {0}")]
    PermissionEvaluation(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Invalid role hierarchy: {0}")]
    InvalidRoleHierarchy(String),

    // Encryption errors
    #[error("Encryption failed: {0}")]
    Encryption(String),

    #[error("Decryption failed: {0}")]
    Decryption(String),

    #[error("Key generation failed: {0}")]
    KeyGeneration(String),

    #[error("Key rotation failed: {0}")]
    KeyRotation(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    // Password errors
    #[error("Password hashing failed: {0}")]
    PasswordHashing(String),

    #[error("Password hashing failed: {0}")]
    PasswordHashingFailed(String),

    #[error("Password verification failed: {0}")]
    PasswordVerification(String),

    #[error("Password verification failed: {0}")]
    PasswordVerificationFailed(String),

    #[error("Password policy violation: {0}")]
    PasswordPolicy(String),

    #[error("Weak password: {0}")]
    WeakPassword(String),

    // Rate limiting errors
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Rate limit configuration error: {0}")]
    RateLimitConfig(String),

    #[error("Rate limit storage error: {0}")]
    RateLimitStorage(String),

    // Input validation errors
    #[error("Input validation failed: {field}: {message}")]
    InputValidation { field: String, message: String },

    #[error("Invalid input format: {0}")]
    InvalidInputFormat(String),

    #[error("Input too long: max {max}, got {actual}")]
    InputTooLong { max: usize, actual: usize },

    #[error("Malicious input detected: {0}")]
    MaliciousInput(String),

    #[error("File validation failed: {0}")]
    FileValidation(String),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    // Threat detection errors
    #[error("Suspicious activity detected: {0}")]
    SuspiciousActivity(String),

    #[error("IP address blocked: {0}")]
    IpBlocked(String),

    #[error("Geographic restriction: {0}")]
    GeographicRestriction(String),

    #[error("Threat analysis failed: {0}")]
    ThreatAnalysis(String),

    // Audit logging errors
    #[error("Audit logging failed: {0}")]
    AuditLogging(String),

    #[error("Audit log retrieval failed: {0}")]
    AuditLogRetrieval(String),

    #[error("Audit log corruption detected: {0}")]
    AuditLogCorruption(String),

    // Database errors
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),

    #[error("Database query failed: {0}")]
    DatabaseQuery(String),

    #[error("Database transaction failed: {0}")]
    DatabaseTransaction(String),

    #[error("Database migration failed: {0}")]
    DatabaseMigration(String),

    // Cache errors
    #[error("Cache operation failed: {0}")]
    CacheOperation(String),

    #[error("Cache connection failed: {0}")]
    CacheConnection(String),

    #[error("Cache serialization failed: {0}")]
    CacheSerialization(String),

    // Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Environment variable not found: {0}")]
    EnvironmentVariable(String),

    #[error("Invalid configuration value: {0}")]
    InvalidConfiguration(String),

    // External service errors
    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("OAuth provider error: {0}")]
    OAuthProvider(String),

    #[error("LDAP authentication failed: {0}")]
    LdapAuthentication(String),

    // Session management errors
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session expired: {0}")]
    SessionExpired(String),

    #[error("Session creation failed: {0}")]
    SessionCreation(String),

    #[error("Maximum sessions exceeded")]
    MaxSessionsExceeded,

    // CSRF errors
    #[error("CSRF token validation failed")]
    CsrfValidation,

    #[error("CSRF token missing")]
    CsrfTokenMissing,

    #[error("CSRF token expired")]
    CsrfTokenExpired,

    // Internal errors
    #[error("Internal security error: {0}")]
    Internal(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    // Custom errors
    #[error("Custom security error: {0}")]
    Custom(String),
}

/// Result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Error context for additional debugging information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub request_id: Option<String>,
    pub user_id: Option<uuid::Uuid>,
    pub client_ip: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub operation: String,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            request_id: None,
            user_id: None,
            client_ip: None,
            user_agent: None,
            timestamp: chrono::Utc::now(),
            operation: operation.into(),
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_user_id(mut self, user_id: uuid::Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_client_ip(mut self, client_ip: std::net::IpAddr) -> Self {
        self.client_ip = Some(client_ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// Enhanced security error with context
#[derive(Debug)]
pub struct SecurityErrorWithContext {
    pub error: SecurityError,
    pub context: ErrorContext,
}

impl fmt::Display for SecurityErrorWithContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)?;

        if let Some(request_id) = &self.context.request_id {
            write!(f, " [request_id: {}]", request_id)?;
        }

        if let Some(user_id) = &self.context.user_id {
            write!(f, " [user_id: {}]", user_id)?;
        }

        if let Some(client_ip) = &self.context.client_ip {
            write!(f, " [client_ip: {}]", client_ip)?;
        }

        write!(f, " [operation: {}]", self.context.operation)?;
        write!(
            f,
            " [timestamp: {}]",
            self.context.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )?;

        Ok(())
    }
}

impl std::error::Error for SecurityErrorWithContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<SecurityError> for SecurityErrorWithContext {
    fn from(error: SecurityError) -> Self {
        Self {
            context: ErrorContext::new("unknown"),
            error,
        }
    }
}

/// Security error classification for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl SecurityError {
    /// Get the severity level of the security error
    pub fn severity(&self) -> SecurityErrorSeverity {
        match self {
            // Critical security errors
            SecurityError::TokenBlacklisted(_)
            | SecurityError::SuspiciousActivity(_)
            | SecurityError::MaliciousInput(_)
            | SecurityError::AuditLogCorruption(_)
            | SecurityError::KeyRotation(_)
            | SecurityError::KeyGeneration(_) => SecurityErrorSeverity::Critical,

            // High severity errors
            SecurityError::Unauthorized
            | SecurityError::InsufficientPermissions { .. }
            | SecurityError::AccountLocked(_)
            | SecurityError::IpBlocked(_)
            | SecurityError::GeographicRestriction(_)
            | SecurityError::InvalidCredentials
            | SecurityError::RateLimitExceeded => SecurityErrorSeverity::High,

            // Medium severity errors
            SecurityError::InvalidToken(_)
            | SecurityError::TokenExpired(_)
            | SecurityError::InvalidMfaCode
            | SecurityError::InputValidation { .. }
            | SecurityError::FileValidation(_)
            | SecurityError::SessionExpired(_)
            | SecurityError::CsrfValidation => SecurityErrorSeverity::Medium,

            // Low severity errors
            SecurityError::TokenGeneration(_)
            | SecurityError::TokenValidation(_)
            | SecurityError::Configuration(_)
            | SecurityError::DatabaseConnection(_)
            | SecurityError::CacheOperation(_)
            | SecurityError::Serialization(_)
            | SecurityError::Deserialization(_) => SecurityErrorSeverity::Low,

            // Default to medium for other errors
            _ => SecurityErrorSeverity::Medium,
        }
    }

    /// Check if the error should trigger an alert
    pub fn should_alert(&self) -> bool {
        matches!(
            self.severity(),
            SecurityErrorSeverity::High | SecurityErrorSeverity::Critical
        )
    }

    /// Get error code for external systems
    pub fn error_code(&self) -> &'static str {
        match self {
            SecurityError::InvalidToken(_) => "SEC001",
            SecurityError::TokenExpired(_) => "SEC002",
            SecurityError::TokenGeneration(_) => "SEC003",
            SecurityError::TokenValidation(_) => "SEC004",
            SecurityError::TokenBlacklisted(_) => "SEC005",
            SecurityError::InvalidCredentials => "SEC006",
            SecurityError::AccountLocked(_) => "SEC007",
            SecurityError::MfaRequired => "SEC008",
            SecurityError::InvalidMfaCode => "SEC009",
            SecurityError::Unauthorized => "SEC010",
            SecurityError::InsufficientPermissions { .. } => "SEC011",
            SecurityError::PermissionEvaluation(_) => "SEC012",
            SecurityError::RoleNotFound(_) => "SEC013",
            SecurityError::InvalidRoleHierarchy(_) => "SEC014",
            SecurityError::Encryption(_) => "SEC015",
            SecurityError::Decryption(_) => "SEC016",
            SecurityError::KeyGeneration(_) => "SEC017",
            SecurityError::KeyRotation(_) => "SEC018",
            SecurityError::InvalidKeyFormat(_) => "SEC019",
            SecurityError::KeyNotFound(_) => "SEC020",
            SecurityError::PasswordHashing(_) => "SEC021",
            SecurityError::PasswordHashingFailed(_) => "SEC021a",
            SecurityError::PasswordVerification(_) => "SEC022",
            SecurityError::PasswordVerificationFailed(_) => "SEC022a",
            SecurityError::PasswordPolicy(_) => "SEC023",
            SecurityError::WeakPassword(_) => "SEC024",
            SecurityError::RateLimitExceeded => "SEC025",
            SecurityError::RateLimitConfig(_) => "SEC026",
            SecurityError::RateLimitStorage(_) => "SEC027",
            SecurityError::InputValidation { .. } => "SEC028",
            SecurityError::InvalidInputFormat(_) => "SEC029",
            SecurityError::InputTooLong { .. } => "SEC030",
            SecurityError::MaliciousInput(_) => "SEC031",
            SecurityError::FileValidation(_) => "SEC032",
            SecurityError::UnsupportedFileType(_) => "SEC033",
            SecurityError::SuspiciousActivity(_) => "SEC034",
            SecurityError::IpBlocked(_) => "SEC035",
            SecurityError::GeographicRestriction(_) => "SEC036",
            SecurityError::ThreatAnalysis(_) => "SEC037",
            SecurityError::AuditLogging(_) => "SEC038",
            SecurityError::AuditLogRetrieval(_) => "SEC039",
            SecurityError::AuditLogCorruption(_) => "SEC040",
            SecurityError::DatabaseConnection(_) => "SEC041",
            SecurityError::DatabaseQuery(_) => "SEC042",
            SecurityError::DatabaseTransaction(_) => "SEC043",
            SecurityError::DatabaseMigration(_) => "SEC044",
            SecurityError::CacheOperation(_) => "SEC045",
            SecurityError::CacheConnection(_) => "SEC046",
            SecurityError::CacheSerialization(_) => "SEC047",
            SecurityError::Configuration(_) => "SEC048",
            SecurityError::EnvironmentVariable(_) => "SEC049",
            SecurityError::InvalidConfiguration(_) => "SEC050",
            SecurityError::ExternalService(_) => "SEC051",
            SecurityError::OAuthProvider(_) => "SEC052",
            SecurityError::LdapAuthentication(_) => "SEC053",
            SecurityError::SessionNotFound(_) => "SEC054",
            SecurityError::SessionExpired(_) => "SEC055",
            SecurityError::SessionCreation(_) => "SEC056",
            SecurityError::MaxSessionsExceeded => "SEC057",
            SecurityError::CsrfValidation => "SEC058",
            SecurityError::CsrfTokenMissing => "SEC059",
            SecurityError::CsrfTokenExpired => "SEC060",
            SecurityError::Internal(_) => "SEC061",
            SecurityError::ServiceUnavailable(_) => "SEC062",
            SecurityError::Timeout(_) => "SEC063",
            SecurityError::Serialization(_) => "SEC064",
            SecurityError::SerializationFailed(_) => "SEC064a",
            SecurityError::Deserialization(_) => "SEC065",
            SecurityError::DeserializationFailed(_) => "SEC065a",
            SecurityError::EncryptionFailed(_) => "SEC066",
            SecurityError::DecryptionFailed(_) => "SEC067",
            SecurityError::InvalidKey(_) => "SEC068",
            SecurityError::InvalidSignature(_) => "SEC069",
            SecurityError::KeyDerivation(_) => "SEC070",
            SecurityError::UnsupportedOperation(_) => "SEC071",
            SecurityError::Custom(_) => "SEC999",
        }
    }

    /// Create error with context
    pub fn with_context(self, context: ErrorContext) -> SecurityErrorWithContext {
        SecurityErrorWithContext {
            error: self,
            context,
        }
    }
}

// Implement conversions from common error types
impl From<sqlx::Error> for SecurityError {
    fn from(err: sqlx::Error) -> Self {
        SecurityError::DatabaseQuery(err.to_string())
    }
}

impl From<redis::RedisError> for SecurityError {
    fn from(err: redis::RedisError) -> Self {
        SecurityError::CacheOperation(err.to_string())
    }
}

impl From<serde_json::Error> for SecurityError {
    fn from(err: serde_json::Error) -> Self {
        SecurityError::Serialization(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for SecurityError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                SecurityError::TokenExpired("JWT token expired".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidToken => {
                SecurityError::InvalidToken("Invalid JWT token".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                SecurityError::TokenValidation("Invalid JWT signature".to_string())
            }
            _ => SecurityError::TokenValidation(err.to_string()),
        }
    }
}

impl From<argon2::Error> for SecurityError {
    fn from(err: argon2::Error) -> Self {
        SecurityError::PasswordHashing(err.to_string())
    }
}

impl From<std::io::Error> for SecurityError {
    fn from(err: std::io::Error) -> Self {
        SecurityError::Internal(format!("IO error: {}", err))
    }
}

impl From<config::ConfigError> for SecurityError {
    fn from(err: config::ConfigError) -> Self {
        SecurityError::Configuration(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        assert_eq!(
            SecurityError::TokenBlacklisted("test".to_string()).severity(),
            SecurityErrorSeverity::Critical
        );
        assert_eq!(
            SecurityError::Unauthorized.severity(),
            SecurityErrorSeverity::High
        );
        assert_eq!(
            SecurityError::InvalidToken("test".to_string()).severity(),
            SecurityErrorSeverity::Medium
        );
        assert_eq!(
            SecurityError::Configuration("test".to_string()).severity(),
            SecurityErrorSeverity::Low
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(SecurityError::InvalidCredentials.error_code(), "SEC006");
        assert_eq!(SecurityError::Unauthorized.error_code(), "SEC010");
        assert_eq!(SecurityError::RateLimitExceeded.error_code(), "SEC025");
    }

    #[test]
    fn test_should_alert() {
        assert!(SecurityError::SuspiciousActivity("test".to_string()).should_alert());
        assert!(SecurityError::Unauthorized.should_alert());
        assert!(!SecurityError::Configuration("test".to_string()).should_alert());
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("test_operation")
            .with_request_id("req-123")
            .with_user_id(uuid::Uuid::new_v4())
            .with_client_ip("127.0.0.1".parse().unwrap())
            .with_info("key", "value");

        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.request_id, Some("req-123".to_string()));
        assert!(context.user_id.is_some());
        assert!(context.client_ip.is_some());
        assert_eq!(
            context.additional_info.get("key"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_error_with_context() {
        let error = SecurityError::Unauthorized;
        let context = ErrorContext::new("test_operation");
        let error_with_context = error.with_context(context);

        assert!(matches!(
            error_with_context.error,
            SecurityError::Unauthorized
        ));
        assert_eq!(error_with_context.context.operation, "test_operation");
    }
}
