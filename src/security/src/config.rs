//! Security Configuration Module
//!
//! Centralized configuration management for all security components including
//! JWT settings, encryption parameters, rate limiting, and security policies.

use crate::constants::*;
use crate::errors::{SecurityError, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Main security configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Encryption configuration
    pub encryption: EncryptionConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
    /// RBAC/ABAC configuration
    pub authorization: AuthorizationConfig,
    /// Audit logging configuration
    pub audit: AuditConfig,
    /// Security headers configuration
    pub headers: SecurityHeadersConfig,
    /// Input validation configuration
    pub input_validation: InputValidationConfig,
    /// Threat detection configuration
    pub threat_detection: ThreatDetectionConfig,
    /// Database configuration for security storage
    pub database: SecurityDatabaseConfig,
}

/// JWT authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Secret key for JWT signing (base64 encoded)
    pub secret: String,
    /// JWT issuer identifier
    pub issuer: String,
    /// JWT audience identifier
    pub audience: String,
    /// Access token time-to-live
    #[serde(with = "duration_serde")]
    pub access_token_ttl: Duration,
    /// Refresh token time-to-live
    #[serde(with = "duration_serde")]
    pub refresh_token_ttl: Duration,
    /// JWT signing algorithm
    pub algorithm: String,
    /// Enable token blacklisting
    pub enable_blacklist: bool,
    /// Maximum number of tokens per user
    pub max_tokens_per_user: u32,
    /// Token rotation interval
    #[serde(with = "duration_serde")]
    pub token_rotation_interval: Duration,
}

/// Encryption services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Master encryption key (base64 encoded)
    pub master_key: String,
    /// Key derivation iterations for PBKDF2
    pub key_derivation_iterations: u32,
    /// Key rotation interval
    #[serde(with = "duration_serde")]
    pub key_rotation_interval: Duration,
    /// Enable automatic key rotation
    pub auto_rotate_keys: bool,
    /// Encryption algorithm preference
    pub algorithm: EncryptionAlgorithm,
    /// Key storage configuration
    pub key_storage: KeyStorageConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Requests per hour limit
    pub requests_per_hour: u32,
    /// Requests per day limit
    pub requests_per_day: u32,
    /// Burst multiplier for short spikes
    pub burst_multiplier: f64,
    /// Enable distributed rate limiting
    pub distributed: bool,
    /// Rate limit storage backend
    pub storage_backend: RateLimitStorage,
    /// Custom rate limits per endpoint
    pub endpoint_limits: HashMap<String, EndpointRateLimit>,
    /// Rate limit by user tier
    pub tier_limits: HashMap<String, TierRateLimit>,
}

/// Authorization (RBAC/ABAC) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Enable role-based access control
    pub enable_rbac: bool,
    /// Enable attribute-based access control
    pub enable_abac: bool,
    /// Permission cache TTL
    #[serde(with = "duration_serde")]
    pub permission_cache_ttl: Duration,
    /// Role hierarchy configuration
    pub role_hierarchy: HashMap<String, Vec<String>>,
    /// Default permissions for new users
    pub default_permissions: Vec<String>,
    /// Admin override enabled
    pub admin_override: bool,
    /// Permission evaluation mode
    pub evaluation_mode: PermissionEvaluationMode,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log level
    pub level: AuditLevel,
    /// Log all authentication attempts
    pub log_auth_attempts: bool,
    /// Log all authorization checks
    pub log_authorization: bool,
    /// Log all data access
    pub log_data_access: bool,
    /// Log admin actions
    pub log_admin_actions: bool,
    /// Audit log retention period
    #[serde(with = "duration_serde")]
    pub retention_period: Duration,
    /// Audit log storage configuration
    pub storage: AuditStorageConfig,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub content_security_policy: String,
    /// Strict Transport Security
    pub strict_transport_security: String,
    /// X-Frame-Options
    pub x_frame_options: String,
    /// X-Content-Type-Options
    pub x_content_type_options: String,
    /// Referrer Policy
    pub referrer_policy: String,
    /// Additional custom headers
    pub custom_headers: HashMap<String, String>,
}

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidationConfig {
    /// Maximum input length
    pub max_input_length: usize,
    /// Enable HTML sanitization
    pub enable_html_sanitization: bool,
    /// Enable SQL injection protection
    pub enable_sql_injection_protection: bool,
    /// Enable XSS protection
    pub enable_xss_protection: bool,
    /// Custom validation rules
    pub custom_rules: HashMap<String, ValidationRule>,
    /// Allowed file extensions for uploads
    pub allowed_file_extensions: Vec<String>,
    /// Maximum file size for uploads (bytes)
    pub max_file_size: usize,
}

/// Threat detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    /// Enable threat detection
    pub enabled: bool,
    /// Maximum login attempts before lockout
    pub max_login_attempts: u32,
    /// Login attempt window
    #[serde(with = "duration_serde")]
    pub login_attempt_window: Duration,
    /// Account lockout duration
    #[serde(with = "duration_serde")]
    pub lockout_duration: Duration,
    /// Enable IP-based blocking
    pub enable_ip_blocking: bool,
    /// Suspicious activity threshold
    pub suspicious_activity_threshold: u32,
    /// GeoIP database configuration
    pub geoip: Option<GeoIpConfig>,
}

/// Security database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityDatabaseConfig {
    /// PostgreSQL URL for ACID transactions
    pub postgres_url: String,
    /// Redis URL for caching and sessions
    pub redis_url: String,
    /// Maximum database connections
    pub max_connections: u32,
    /// Connection timeout
    #[serde(with = "duration_serde")]
    pub connection_timeout: Duration,
    /// Query timeout
    #[serde(with = "duration_serde")]
    pub query_timeout: Duration,
}

// Supporting configuration types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    #[serde(rename = "aes-256-gcm")]
    Aes256Gcm,
    #[serde(rename = "chacha20-poly1305")]
    ChaCha20Poly1305,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStorageConfig {
    pub storage_type: KeyStorageType,
    pub file_path: Option<PathBuf>,
    pub vault_url: Option<String>,
    pub vault_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyStorageType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "vault")]
    Vault,
    #[serde(rename = "memory")]
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitStorage {
    #[serde(rename = "memory")]
    Memory,
    #[serde(rename = "redis")]
    Redis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRateLimit {
    pub requests_per_minute: u32,
    pub burst_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionEvaluationMode {
    #[serde(rename = "strict")]
    Strict,
    #[serde(rename = "permissive")]
    Permissive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditLevel {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStorageConfig {
    pub storage_type: AuditStorageType,
    pub file_path: Option<PathBuf>,
    pub database_table: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditStorageType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "both")]
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub pattern: String,
    pub message: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpConfig {
    pub database_path: PathBuf,
    pub update_interval: Duration,
    pub blocked_countries: Vec<String>,
    pub allowed_countries: Option<Vec<String>>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt: JwtConfig::default(),
            encryption: EncryptionConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            authorization: AuthorizationConfig::default(),
            audit: AuditConfig::default(),
            headers: SecurityHeadersConfig::default(),
            input_validation: InputValidationConfig::default(),
            threat_detection: ThreatDetectionConfig::default(),
            database: SecurityDatabaseConfig::default(),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-secret-key-change-in-production".to_string(),
            issuer: "ai-core-platform".to_string(),
            audience: "api".to_string(),
            access_token_ttl: DEFAULT_ACCESS_TOKEN_TTL,
            refresh_token_ttl: DEFAULT_REFRESH_TOKEN_TTL,
            algorithm: DEFAULT_JWT_ALGORITHM.to_string(),
            enable_blacklist: true,
            max_tokens_per_user: 10,
            token_rotation_interval: Duration::from_secs(86400), // 24 hours
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            master_key: "change-this-master-key-in-production".to_string(),
            key_derivation_iterations: 100_000,
            key_rotation_interval: Duration::from_secs(7_776_000), // 90 days
            auto_rotate_keys: false,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_storage: KeyStorageConfig {
                storage_type: KeyStorageType::Memory,
                file_path: None,
                vault_url: None,
                vault_token: None,
            },
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: DEFAULT_RATE_LIMIT_PER_MINUTE,
            requests_per_hour: DEFAULT_RATE_LIMIT_PER_HOUR,
            requests_per_day: 10_000,
            burst_multiplier: DEFAULT_BURST_MULTIPLIER,
            distributed: true,
            storage_backend: RateLimitStorage::Redis,
            endpoint_limits: HashMap::new(),
            tier_limits: HashMap::new(),
        }
    }
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            enable_rbac: true,
            enable_abac: false,
            permission_cache_ttl: Duration::from_secs(900), // 15 minutes
            role_hierarchy: HashMap::new(),
            default_permissions: vec!["workflows:read".to_string()],
            admin_override: true,
            evaluation_mode: PermissionEvaluationMode::Strict,
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: AuditLevel::Info,
            log_auth_attempts: true,
            log_authorization: true,
            log_data_access: false,
            log_admin_actions: true,
            retention_period: Duration::from_secs(31_536_000), // 1 year
            storage: AuditStorageConfig {
                storage_type: AuditStorageType::Database,
                file_path: None,
                database_table: Some("security_audit_log".to_string()),
            },
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_security_policy: CONTENT_SECURITY_POLICY.to_string(),
            strict_transport_security: STRICT_TRANSPORT_SECURITY.to_string(),
            x_frame_options: X_FRAME_OPTIONS.to_string(),
            x_content_type_options: X_CONTENT_TYPE_OPTIONS.to_string(),
            referrer_policy: REFERRER_POLICY.to_string(),
            custom_headers: HashMap::new(),
        }
    }
}

impl Default for InputValidationConfig {
    fn default() -> Self {
        Self {
            max_input_length: MAX_INPUT_LENGTH,
            enable_html_sanitization: true,
            enable_sql_injection_protection: true,
            enable_xss_protection: true,
            custom_rules: HashMap::new(),
            allowed_file_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "pdf".to_string(),
                "txt".to_string(),
                "doc".to_string(),
                "docx".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_login_attempts: MAX_LOGIN_ATTEMPTS,
            login_attempt_window: LOGIN_ATTEMPT_WINDOW,
            lockout_duration: Duration::from_secs(3600), // 1 hour
            enable_ip_blocking: true,
            suspicious_activity_threshold: SUSPICIOUS_ACTIVITY_THRESHOLD,
            geoip: None,
        }
    }
}

impl Default for SecurityDatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: "postgresql://localhost/ai_core_security".to_string(),
            redis_url: "redis://localhost:6379/0".to_string(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
        }
    }
}

impl SecurityConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> SecurityResult<Self> {
        let mut config = Self::default();

        // JWT configuration from environment
        if let Ok(secret) = std::env::var("JWT_SECRET") {
            config.jwt.secret = secret;
        }
        if let Ok(issuer) = std::env::var("JWT_ISSUER") {
            config.jwt.issuer = issuer;
        }
        if let Ok(audience) = std::env::var("JWT_AUDIENCE") {
            config.jwt.audience = audience;
        }

        // Database configuration from environment
        if let Ok(postgres_url) = std::env::var("POSTGRES_URL") {
            config.database.postgres_url = postgres_url;
        }
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.database.redis_url = redis_url;
        }

        // Encryption configuration from environment
        if let Ok(master_key) = std::env::var("ENCRYPTION_MASTER_KEY") {
            config.encryption.master_key = master_key;
        }

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> SecurityResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SecurityError::Configuration(format!("Failed to read config file: {}", e))
        })?;

        let config: Self = serde_yaml::from_str(&content)
            .map_err(|e| SecurityError::Configuration(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> SecurityResult<()> {
        // Validate JWT configuration
        if self.jwt.secret.len() < 32 {
            return Err(SecurityError::Configuration(
                "JWT secret must be at least 32 characters".to_string(),
            ));
        }

        if self.jwt.access_token_ttl.as_secs() == 0 {
            return Err(SecurityError::Configuration(
                "Access token TTL must be greater than 0".to_string(),
            ));
        }

        // Validate encryption configuration
        if self.encryption.master_key.len() < 32 {
            return Err(SecurityError::Configuration(
                "Master encryption key must be at least 32 characters".to_string(),
            ));
        }

        // Validate rate limiting configuration
        if self.rate_limiting.requests_per_minute == 0 {
            return Err(SecurityError::Configuration(
                "Rate limit must be greater than 0".to_string(),
            ));
        }

        // Validate database URLs
        if self.database.postgres_url.is_empty() {
            return Err(SecurityError::Configuration(
                "PostgreSQL URL cannot be empty".to_string(),
            ));
        }

        if self.database.redis_url.is_empty() {
            return Err(SecurityError::Configuration(
                "Redis URL cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> SecurityResult<()> {
        let content = serde_yaml::to_string(self).map_err(|e| {
            SecurityError::Configuration(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, content).map_err(|e| {
            SecurityError::Configuration(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }
}

// Custom serialization for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.jwt.algorithm, "HS256");
        assert!(config.jwt.enable_blacklist);
        assert!(config.audit.enabled);
        assert!(config.threat_detection.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = SecurityConfig::default();

        // Valid configuration should pass
        assert!(config.validate().is_ok());

        // Invalid JWT secret should fail
        config.jwt.secret = "short".to_string();
        assert!(config.validate().is_err());

        // Reset and test invalid access token TTL
        config.jwt.secret = "valid-secret-key-with-32-characters".to_string();
        config.jwt.access_token_ttl = Duration::from_secs(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = SecurityConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: SecurityConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.jwt.algorithm, deserialized.jwt.algorithm);
        assert_eq!(
            config.jwt.enable_blacklist,
            deserialized.jwt.enable_blacklist
        );
    }
}
