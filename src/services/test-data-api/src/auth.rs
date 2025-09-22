// AI-CORE Test Data API Authentication Service
// JWT-based authentication and authorization for test data management
// Backend Agent Implementation - T2.2

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::models::UserRole;

// ============================================================================
// Authentication Service - JWT Token Management
// ============================================================================

pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    token_expiry_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // Subject (user ID)
    pub username: String,      // Username
    pub email: String,         // Email address
    pub role: UserRole,        // User role
    pub permissions: Vec<String>, // User permissions
    pub environment: String,   // Test environment
    pub exp: i64,             // Expiration time
    pub iat: i64,             // Issued at
    pub jti: String,          // JWT ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub expires_at: DateTime<Utc>,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub environment: Option<String>,
}

impl AuthService {
    pub async fn new(jwt_secret: String) -> Result<Self> {
        info!("Initializing AuthService with JWT authentication");

        let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

        let mut validation = Validation::new(Algorithm::HS256);
        validation.required_spec_claims = HashSet::from(["sub".to_string(), "exp".to_string()]);
        validation.validate_exp = true;

        let service = Self {
            encoding_key,
            decoding_key,
            validation,
            token_expiry_hours: 24, // 24 hours default
        };

        info!("AuthService initialized successfully");
        Ok(service)
    }

    // ========================================================================
    // Public Authentication Methods
    // ========================================================================

    pub async fn authenticate(&self, request: AuthRequest) -> Result<TokenResponse> {
        debug!("Authenticating user: {}", request.username);

        // In a real implementation, this would validate against a user database
        // For testing purposes, we'll accept certain test credentials
        if !self.validate_credentials(&request).await? {
            return Err(anyhow!("Invalid credentials"));
        }

        // Get user details (mock implementation)
        let user_details = self.get_user_details(&request.username).await?;

        // Generate JWT token
        let token = self.generate_token(user_details, &request).await?;

        info!("User authenticated successfully: {}", request.username);
        Ok(token)
    }

    pub async fn validate_token(&self, token: &str) -> Result<Claims> {
        debug!("Validating JWT token");

        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &self.validation,
        ).map_err(|e| anyhow!("Token validation failed: {}", e))?;

        let claims = token_data.claims;

        // Additional validation checks
        self.validate_claims(&claims).await?;

        debug!("Token validated successfully for user: {}", claims.username);
        Ok(claims)
    }

    pub async fn refresh_token(&self, token: &str) -> Result<TokenResponse> {
        debug!("Refreshing JWT token");

        // Validate existing token (even if expired, for refresh purposes)
        let mut validation = self.validation.clone();
        validation.validate_exp = false; // Allow expired tokens for refresh

        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &validation,
        ).map_err(|e| anyhow!("Token refresh validation failed: {}", e))?;

        let old_claims = token_data.claims;

        // Check if token is not too old (within refresh window)
        let now = Utc::now().timestamp();
        let token_age_hours = (now - old_claims.iat) / 3600;

        if token_age_hours > (self.token_expiry_hours * 2) {
            return Err(anyhow!("Token too old to refresh"));
        }

        // Generate new token with same user details
        let user_details = UserDetails {
            id: Uuid::parse_str(&old_claims.sub)?,
            username: old_claims.username,
            email: old_claims.email,
            role: old_claims.role,
            permissions: old_claims.permissions,
            environment: old_claims.environment,
        };

        let auth_request = AuthRequest {
            username: user_details.username.clone(),
            password: String::new(), // Not needed for refresh
            environment: Some(user_details.environment.clone()),
        };

        let new_token = self.generate_token(user_details, &auth_request).await?;

        info!("Token refreshed successfully for user: {}", auth_request.username);
        Ok(new_token)
    }

    pub async fn revoke_token(&self, token: &str) -> Result<()> {
        debug!("Revoking JWT token");

        // In a real implementation, this would add the token to a blacklist
        // or revocation list stored in Redis or database

        let claims = self.validate_token(token).await?;
        info!("Token revoked for user: {}", claims.username);
        Ok(())
    }

    // ========================================================================
    // Authorization Methods
    // ========================================================================

    pub async fn check_permission(&self, claims: &Claims, required_permission: &str) -> Result<bool> {
        debug!("Checking permission '{}' for user: {}", required_permission, claims.username);

        // Admin role has all permissions
        if matches!(claims.role, UserRole::Admin) {
            return Ok(true);
        }

        // Check if user has wildcard permission
        if claims.permissions.contains(&"*".to_string()) {
            return Ok(true);
        }

        // Check specific permission
        if claims.permissions.contains(&required_permission.to_string()) {
            return Ok(true);
        }

        // Check role-based permissions
        let role_permissions = self.get_role_permissions(&claims.role);
        if role_permissions.contains(&required_permission.to_string()) {
            return Ok(true);
        }

        debug!("Permission '{}' denied for user: {}", required_permission, claims.username);
        Ok(false)
    }

    pub async fn check_environment_access(&self, claims: &Claims, environment: &str) -> Result<bool> {
        debug!("Checking environment access '{}' for user: {}", environment, claims.username);

        // Admin can access all environments
        if matches!(claims.role, UserRole::Admin) {
            return Ok(true);
        }

        // Check if user's environment matches
        if claims.environment == environment {
            return Ok(true);
        }

        // Check if user has cross-environment permissions
        if claims.permissions.contains(&"cross_environment".to_string()) {
            return Ok(true);
        }

        debug!("Environment access '{}' denied for user: {}", environment, claims.username);
        Ok(false)
    }

    pub async fn check_resource_access(&self, claims: &Claims, resource_type: &str, resource_id: &str) -> Result<bool> {
        debug!("Checking resource access '{}:{}' for user: {}", resource_type, resource_id, claims.username);

        // Admin has access to all resources
        if matches!(claims.role, UserRole::Admin) {
            return Ok(true);
        }

        // Check resource-specific permissions
        let resource_permission = format!("{}:read", resource_type);
        if self.check_permission(claims, &resource_permission).await? {
            return Ok(true);
        }

        // Check ownership (simplified - in real implementation would check database)
        if self.is_resource_owner(claims, resource_type, resource_id).await? {
            return Ok(true);
        }

        debug!("Resource access '{}:{}' denied for user: {}", resource_type, resource_id, claims.username);
        Ok(false)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    async fn validate_credentials(&self, request: &AuthRequest) -> Result<bool> {
        // Mock credential validation for testing
        // In production, this would hash the password and check against database
        match request.username.as_str() {
            "test_admin" => Ok(request.password == "admin_password"),
            "test_user" => Ok(request.password == "user_password"),
            "test_developer" => Ok(request.password == "dev_password"),
            "test_tester" => Ok(request.password == "test_password"),
            username if username.starts_with("test_") => Ok(request.password == "test_password"),
            _ => Ok(false),
        }
    }

    async fn get_user_details(&self, username: &str) -> Result<UserDetails> {
        // Mock user details - in production would fetch from database
        let (role, permissions) = match username {
            "test_admin" => (UserRole::Admin, vec!["*".to_string()]),
            "test_manager" => (UserRole::Manager, vec!["read".to_string(), "write".to_string(), "manage".to_string()]),
            "test_developer" => (UserRole::Developer, vec!["read".to_string(), "write".to_string(), "deploy".to_string()]),
            "test_tester" => (UserRole::Tester, vec!["read".to_string(), "test".to_string()]),
            "test_user" => (UserRole::User, vec!["read".to_string()]),
            username if username.starts_with("test_") => (UserRole::User, vec!["read".to_string()]),
            _ => return Err(anyhow!("User not found")),
        };

        Ok(UserDetails {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: format!("{}@test.com", username),
            role,
            permissions,
            environment: "test".to_string(),
        })
    }

    async fn generate_token(&self, user: UserDetails, request: &AuthRequest) -> Result<TokenResponse> {
        let now = Utc::now();
        let expiry = now + Duration::hours(self.token_expiry_hours);

        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username,
            email: user.email,
            role: user.role,
            permissions: user.permissions.clone(),
            environment: request.environment.clone().unwrap_or(user.environment),
            exp: expiry.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| anyhow!("Token generation failed: {}", e))?;

        Ok(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.token_expiry_hours * 3600, // Convert to seconds
            expires_at: expiry,
            scope: user.permissions,
        })
    }

    async fn validate_claims(&self, claims: &Claims) -> Result<()> {
        let now = Utc::now().timestamp();

        // Check expiration
        if claims.exp < now {
            return Err(anyhow!("Token has expired"));
        }

        // Check issued time (not too far in the future)
        if claims.iat > now + 300 { // 5 minutes tolerance
            return Err(anyhow!("Token issued time is invalid"));
        }

        // Validate user ID format
        Uuid::parse_str(&claims.sub)
            .map_err(|_| anyhow!("Invalid user ID format in token"))?;

        // Validate JWT ID format
        Uuid::parse_str(&claims.jti)
            .map_err(|_| anyhow!("Invalid JWT ID format in token"))?;

        Ok(())
    }

    fn get_role_permissions(&self, role: &UserRole) -> Vec<String> {
        match role {
            UserRole::Admin => vec!["*".to_string()],
            UserRole::Manager => vec![
                "read".to_string(),
                "write".to_string(),
                "manage".to_string(),
                "user:create".to_string(),
                "user:delete".to_string(),
                "environment:create".to_string(),
                "environment:reset".to_string(),
            ],
            UserRole::Developer => vec![
                "read".to_string(),
                "write".to_string(),
                "deploy".to_string(),
                "test:create".to_string(),
                "test:execute".to_string(),
                "data:generate".to_string(),
            ],
            UserRole::Tester => vec![
                "read".to_string(),
                "test".to_string(),
                "test:create".to_string(),
                "test:execute".to_string(),
                "test:report".to_string(),
            ],
            UserRole::User => vec![
                "read".to_string(),
                "workflow:execute".to_string(),
            ],
            UserRole::Viewer => vec![
                "read".to_string(),
            ],
            UserRole::Guest => vec![
                "limited_read".to_string(),
            ],
        }
    }

    async fn is_resource_owner(&self, claims: &Claims, resource_type: &str, resource_id: &str) -> Result<bool> {
        // Simplified ownership check - in production would query database
        // For now, assume user owns resources they created
        debug!("Checking ownership of {}:{} for user {}", resource_type, resource_id, claims.username);

        // This would typically query the database to check if the user owns the resource
        // For mock implementation, return true for certain patterns
        Ok(resource_id.contains(&claims.sub) || claims.username == "test_admin")
    }

    pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
        if auth_header.starts_with("Bearer ") {
            Some(&auth_header[7..])
        } else {
            None
        }
    }

    pub async fn create_service_token(&self, service_name: &str, permissions: Vec<String>) -> Result<TokenResponse> {
        debug!("Creating service token for: {}", service_name);

        let user_details = UserDetails {
            id: Uuid::new_v4(),
            username: format!("service_{}", service_name),
            email: format!("{}@service.internal", service_name),
            role: UserRole::Developer, // Services typically have developer-level access
            permissions,
            environment: "all".to_string(),
        };

        let auth_request = AuthRequest {
            username: user_details.username.clone(),
            password: String::new(),
            environment: Some("all".to_string()),
        };

        let token = self.generate_token(user_details, &auth_request).await?;

        info!("Service token created for: {}", service_name);
        Ok(token)
    }

    pub async fn validate_api_key(&self, api_key: &str) -> Result<Claims> {
        debug!("Validating API key");

        // Simple API key validation - in production would be more sophisticated
        let service_name = match api_key {
            "test-api-key-admin" => "admin-service",
            "test-api-key-user" => "user-service",
            "test-api-key-system" => "system-service",
            _ => return Err(anyhow!("Invalid API key")),
        };

        // Create claims for API key
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: format!("apikey_{}", service_name),
            email: format!("{}@api.internal", service_name),
            role: if api_key.contains("admin") { UserRole::Admin } else { UserRole::User },
            permissions: if api_key.contains("admin") {
                vec!["*".to_string()]
            } else {
                vec!["read".to_string(), "write".to_string()]
            },
            environment: "all".to_string(),
            exp: (Utc::now() + Duration::hours(24)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        debug!("API key validated for service: {}", service_name);
        Ok(claims)
    }
}

// ============================================================================
// Supporting Data Structures
// ============================================================================

#[derive(Debug, Clone)]
struct UserDetails {
    id: Uuid,
    username: String,
    email: String,
    role: UserRole,
    permissions: Vec<String>,
    environment: String,
}

// ============================================================================
// Middleware Helper Functions
// ============================================================================

pub fn extract_user_claims(extensions: &axum::http::Extensions) -> Option<Claims> {
    extensions.get::<Claims>().cloned()
}

pub fn require_permission(required_permission: &str) -> impl Fn(&Claims) -> bool + '_ {
    move |claims: &Claims| {
        // Simple permission check for middleware
        matches!(claims.role, UserRole::Admin) ||
        claims.permissions.contains(&"*".to_string()) ||
        claims.permissions.contains(&required_permission.to_string())
    }
}

pub fn require_role(required_role: UserRole) -> impl Fn(&Claims) -> bool {
    move |claims: &Claims| {
        matches!(claims.role, UserRole::Admin) || claims.role == required_role
    }
}

// ============================================================================
// Error Types for Authentication
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid: {0}")]
    TokenInvalid(String),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Environment access denied")]
    EnvironmentAccessDenied,

    #[error("Resource access denied")]
    ResourceAccessDenied,

    #[error("User not found")]
    UserNotFound,

    #[error("Service unavailable")]
    ServiceUnavailable,
}

// ============================================================================
// Testing Utilities
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_generation_and_validation() {
        let auth_service = AuthService::new("test_secret_key".to_string()).await.unwrap();

        let auth_request = AuthRequest {
            username: "test_user".to_string(),
            password: "user_password".to_string(),
            environment: Some("test".to_string()),
        };

        // Test authentication
        let token_response = auth_service.authenticate(auth_request).await.unwrap();
        assert_eq!(token_response.token_type, "Bearer");
        assert!(!token_response.access_token.is_empty());

        // Test token validation
        let claims = auth_service.validate_token(&token_response.access_token).await.unwrap();
        assert_eq!(claims.username, "test_user");
        assert_eq!(claims.environment, "test");
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let auth_service = AuthService::new("test_secret_key".to_string()).await.unwrap();

        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            role: UserRole::User,
            permissions: vec!["read".to_string()],
            environment: "test".to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        // Test permission checking
        assert!(auth_service.check_permission(&claims, "read").await.unwrap());
        assert!(!auth_service.check_permission(&claims, "write").await.unwrap());
    }

    #[tokio::test]
    async fn test_admin_permissions() {
        let auth_service = AuthService::new("test_secret_key".to_string()).await.unwrap();

        let admin_claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            permissions: vec!["*".to_string()],
            environment: "all".to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        // Admin should have all permissions
        assert!(auth_service.check_permission(&admin_claims, "read").await.unwrap());
        assert!(auth_service.check_permission(&admin_claims, "write").await.unwrap());
        assert!(auth_service.check_permission(&admin_claims, "delete").await.unwrap());
        assert!(auth_service.check_environment_access(&admin_claims, "any_env").await.unwrap());
    }
}
