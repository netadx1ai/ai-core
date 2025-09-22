//! # AI-CORE Security Framework
//!
//! Comprehensive security framework for the AI-CORE Intelligent Automation Platform.
//! Provides authentication, authorization, encryption, and security middleware services.
//!
//! ## Features
//!
//! - **JWT Authentication**: Secure token generation, validation, and refresh
//! - **RBAC/ABAC Authorization**: Role-based and attribute-based access control
//! - **Encryption Services**: AES-256-GCM encryption with key management
//! - **Security Middleware**: Rate limiting, input validation, security headers
//! - **Audit Logging**: Comprehensive security event logging
//! - **Threat Detection**: DDoS protection and anomaly detection
//!
//! ## Quick Start
//!
//! ```rust
//! use ai_core_security::{SecurityService, SecurityConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SecurityConfig::from_env()?;
//!     let security = SecurityService::new(config).await?;
//!
//!     // Generate JWT token
//!     let token = security.jwt().generate_access_token(user_id, roles).await?;
//!
//!     // Validate token
//!     let claims = security.jwt().validate_token(&token).await?;
//!
//!     // Check permissions
//!     let authorized = security.rbac()
//!         .check_permission(user_id, "workflows", "create")
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

// Re-export shared types for convenience
pub use ai_core_shared::types::{
    AuthCredentials, ErrorResponse, ErrorType, Permission, SubscriptionTier, TokenClaims, User,
    UserStatus,
};

// Core security modules
pub mod audit;
pub mod encryption;
pub mod input_validation;
pub mod jwt;
// Temporarily disabled due to Send trait issues
// pub mod middleware;
pub mod middleware_simple;
pub mod rate_limiting;
pub mod rbac;
pub mod threat_detection;

// Configuration and service management
pub mod config;
pub mod errors;
pub mod service;

// Security utilities
pub mod utils;

// Re-export main service and configuration
pub use config::SecurityConfig;
pub use errors::{SecurityError, SecurityResult};
pub use service::SecurityService;

// Re-export commonly used types and traits
pub use audit::{AuditLevel, AuditLogger, SecurityEvent};
pub use encryption::{EncryptionService, KeyManager, PasswordService};
pub use input_validation::{InputValidator, SanitizationConfig};
pub use jwt::{AccessToken, JwtClaims, JwtService, RefreshToken};
// Temporarily disabled due to Send trait issues
// pub use middleware::{AuthenticationLayer, AuthorizationLayer, SecurityMiddleware};
pub use middleware_simple::SimpleSecurityMiddleware;
pub use rate_limiting::{RateLimitConfig, RateLimitResult, RateLimiter};
pub use rbac::{PermissionCache, RbacService, RoleRepository};
pub use threat_detection::{SecurityAlert, ThreatDetector, ThreatLevel};

// Security constants
pub mod constants {
    use std::time::Duration;

    // JWT Configuration
    pub const DEFAULT_ACCESS_TOKEN_TTL: Duration = Duration::from_secs(3600); // 1 hour
    pub const DEFAULT_REFRESH_TOKEN_TTL: Duration = Duration::from_secs(2_592_000); // 30 days
    pub const DEFAULT_JWT_ALGORITHM: &str = "HS256";

    // Password Security
    pub const MIN_PASSWORD_LENGTH: usize = 12;
    pub const MAX_PASSWORD_LENGTH: usize = 128;
    pub const ARGON2_MEMORY_SIZE: u32 = 65536; // 64 MB
    pub const ARGON2_ITERATIONS: u32 = 3;
    pub const ARGON2_PARALLELISM: u32 = 1;

    // Rate Limiting
    pub const DEFAULT_RATE_LIMIT_PER_MINUTE: u32 = 60;
    pub const DEFAULT_RATE_LIMIT_PER_HOUR: u32 = 1000;
    pub const DEFAULT_BURST_MULTIPLIER: f64 = 1.5;

    // Encryption
    pub const AES_KEY_SIZE: usize = 32; // 256 bits
    pub const AES_NONCE_SIZE: usize = 12; // 96 bits for GCM
    pub const CHACHA20_KEY_SIZE: usize = 32; // 256 bits
    pub const CHACHA20_NONCE_SIZE: usize = 12; // 96 bits

    // Session Management
    pub const MAX_CONCURRENT_SESSIONS: usize = 5;
    pub const SESSION_CLEANUP_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes
    pub const TOKEN_BLACKLIST_TTL: Duration = Duration::from_secs(86400); // 24 hours

    // Security Headers
    pub const CONTENT_SECURITY_POLICY: &str =
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:";
    pub const STRICT_TRANSPORT_SECURITY: &str = "max-age=31536000; includeSubDomains; preload";
    pub const X_FRAME_OPTIONS: &str = "DENY";
    pub const X_CONTENT_TYPE_OPTIONS: &str = "nosniff";
    pub const REFERRER_POLICY: &str = "strict-origin-when-cross-origin";

    // Input Validation
    pub const MAX_INPUT_LENGTH: usize = 10_000;
    pub const MAX_EMAIL_LENGTH: usize = 254;
    pub const MAX_USERNAME_LENGTH: usize = 64;
    pub const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10 MB

    // Threat Detection
    pub const MAX_LOGIN_ATTEMPTS: u32 = 5;
    pub const LOGIN_ATTEMPT_WINDOW: Duration = Duration::from_secs(900); // 15 minutes
    pub const IP_BLACKLIST_DURATION: Duration = Duration::from_secs(3600); // 1 hour
    pub const SUSPICIOUS_ACTIVITY_THRESHOLD: u32 = 10;
}

// Security utilities and helper functions
pub mod prelude {
    //! Common imports for security operations

    pub use crate::{
        constants::*, AuditLogger, EncryptionService, InputValidator, JwtService, RateLimiter,
        RbacService, SecurityConfig, SecurityError, SecurityResult, SecurityService,
        ThreatDetector,
    };

    pub use ai_core_shared::types::{AuthCredentials, Permission, TokenClaims, User};

    pub use async_trait::async_trait;
    pub use chrono::{DateTime, Utc};
    pub use serde::{Deserialize, Serialize};
    pub use uuid::Uuid;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        // Verify security constants are reasonable
        assert!(constants::MIN_PASSWORD_LENGTH >= 8);
        assert!(constants::MAX_PASSWORD_LENGTH <= 256);
        assert!(constants::DEFAULT_RATE_LIMIT_PER_MINUTE > 0);
        assert!(constants::AES_KEY_SIZE == 32);
        assert!(constants::MAX_CONCURRENT_SESSIONS > 0);
    }

    #[test]
    fn test_module_exports() {
        // Verify all modules are properly exported
        let _config = SecurityConfig::default();
        let _error = SecurityError::InvalidToken("test".to_string());
    }
}
