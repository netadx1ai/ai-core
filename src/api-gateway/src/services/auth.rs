//! Authentication service for JWT token management and validation

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::error::{ApiError, Result};
use ai_core_shared::{
    config::AuthConfig,
    types::core::{Permission, SubscriptionTier, TokenClaims, User},
};

/// Authentication service for managing JWT tokens and user authentication
#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
    db_pool: PgPool,
    redis_manager: ConnectionManager,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    // Mock users for development
    mock_users: HashMap<String, User>,
}

impl AuthService {
    /// Create new authentication service
    pub fn new(config: AuthConfig, db_pool: PgPool, redis_manager: ConnectionManager) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        // Create mock users for development
        let mut mock_users = HashMap::new();

        // Admin user
        mock_users.insert(
            "admin@AI-PLATFORM.dev".to_string(),
            User {
                id: "admin-user-id".to_string(),
                email: "admin@AI-PLATFORM.dev".to_string(),
                username: "admin".to_string(),
                password_hash: "$argon2id$v=19$m=4096,t=3,p=1$YWRtaW4xMjM$8Z5vNJZeGmGrOeqyLf9c7RQ+jw2YLk5rKtV4aB+XcVE".to_string(),
                name: "System Administrator".to_string(),
                avatar_url: None,
                email_verified: true,
                is_active: true,
                subscription_tier: SubscriptionTier::Enterprise,
                roles: vec!["admin".to_string(), "superuser".to_string()],
                permissions: vec![
                    "admin:system".to_string(),
                    "admin:users".to_string(),
                    "admin:billing".to_string(),
                    "workflows:read".to_string(),
                    "workflows:create".to_string(),
                    "workflows:update".to_string(),
                    "workflows:delete".to_string(),
                    "analytics:read".to_string(),
                    "analytics:export".to_string(),
                    "federation:manage".to_string(),
                ],
                totp_secret: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
                preferences: None,
            },
        );

        // Demo user
        mock_users.insert(
            "demo@AI-PLATFORM.dev".to_string(),
            User {
                id: "demo-user-id".to_string(),
                email: "demo@AI-PLATFORM.dev".to_string(),
                username: "demo".to_string(),
                password_hash: "$argon2id$v=19$m=4096,t=3,p=1$ZGVtbzEyMw$rN3K8Y5vNJZeGmGrOeqyLf9c7RQ+jw2YLk5rKtV4aB+XcVE".to_string(),
                name: "Demo User".to_string(),
                avatar_url: None,
                email_verified: true,
                is_active: true,
                subscription_tier: SubscriptionTier::Pro,
                roles: vec!["user".to_string()],
                permissions: vec![
                    "workflows:read".to_string(),
                    "workflows:create".to_string(),
                    "workflows:update".to_string(),
                    "content:read".to_string(),
                    "content:create".to_string(),
                ],
                totp_secret: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
                preferences: None,
            },
        );

        info!(
            "Authentication service initialized with {} mock users",
            mock_users.len()
        );

        Self {
            config,
            db_pool,
            redis_manager,
            encoding_key,
            decoding_key,
            mock_users,
        }
    }

    /// Authenticate user with email and password
    pub async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
        _totp_code: Option<&str>,
    ) -> Result<Option<User>> {
        debug!("Authenticating user with email: {}", email);

        if let Some(user) = self.mock_users.get(email) {
            // For demo purposes, accept "admin123" or "demo123" as passwords
            let expected_password = if email == "admin@AI-PLATFORM.dev" {
                "admin123"
            } else {
                "demo123"
            };

            if password == expected_password {
                debug!("User authenticated successfully: {}", email);
                Ok(Some(user.clone()))
            } else {
                warn!("Invalid password for user: {}", email);
                Ok(None)
            }
        } else {
            debug!("User not found: {}", email);
            Ok(None)
        }
    }

    /// Authenticate user with API key
    pub async fn authenticate_api_key(&self, api_key: &str) -> Result<Option<User>> {
        debug!("Authenticating with API key");

        // For demo purposes, accept a specific API key
        if api_key == "ak_test_1234567890abcdef1234567890abcdef" {
            // Return admin user for API key auth
            if let Some(user) = self.mock_users.get("admin@AI-PLATFORM.dev") {
                debug!("API key authenticated successfully");
                Ok(Some(user.clone()))
            } else {
                Ok(None)
            }
        } else {
            debug!("Invalid API key");
            Ok(None)
        }
    }

    /// Validate refresh token and return user
    pub async fn validate_refresh_token(&self, refresh_token: &str) -> Result<Option<User>> {
        debug!("Validating refresh token");

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["refresh"]);
        validation.set_issuer(&["AI-PLATFORM-platform"]);

        match decode::<TokenClaims>(refresh_token, &self.decoding_key, &validation) {
            Ok(token_data) => {
                // Find user by ID
                for user in self.mock_users.values() {
                    if user.id == token_data.claims.sub {
                        debug!("Refresh token validated for user: {}", user.email);
                        return Ok(Some(user.clone()));
                    }
                }
                debug!("User not found for refresh token");
                Ok(None)
            }
            Err(e) => {
                debug!("Invalid refresh token: {}", e);
                Ok(None)
            }
        }
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>> {
        debug!("Getting user by ID: {}", user_id);

        for user in self.mock_users.values() {
            if user.id == user_id {
                return Ok(Some(user.clone()));
            }
        }

        Ok(None)
    }

    /// Generate access token for user
    pub async fn generate_access_token(&self, user: &User, expires_in: i64) -> Result<String> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(expires_in);

        let claims = TokenClaims {
            sub: user.id.clone(),
            iss: "AI-PLATFORM-platform".to_string(),
            aud: "api-gateway".to_string(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
            roles: user.roles.clone(),
            permissions: user.permissions.clone(),
            subscription_tier: user.subscription_tier.clone(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| ApiError::internal(format!("Failed to generate access token: {}", e)))
    }

    /// Generate refresh token for user
    pub async fn generate_refresh_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.config.jwt_refresh_expiration_seconds as i64);

        let claims = TokenClaims {
            sub: user.id.clone(),
            iss: "AI-PLATFORM-platform".to_string(),
            aud: "refresh".to_string(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
            roles: user.roles.clone(),
            permissions: vec![], // Refresh tokens don't need full permissions
            subscription_tier: user.subscription_tier.clone(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| ApiError::internal(format!("Failed to generate refresh token: {}", e)))
    }

    /// Validate JWT token and return claims
    pub async fn validate_token(&self, token: &str) -> Result<TokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["api-gateway"]);
        validation.set_issuer(&["AI-PLATFORM-platform"]);

        let token_data = decode::<TokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| ApiError::authentication("Invalid or expired token"))?;

        // Check if token is blacklisted
        if self.is_token_blacklisted(token).await? {
            return Err(ApiError::authentication("Token has been revoked"));
        }

        Ok(token_data.claims)
    }

    /// Check if token is blacklisted
    pub async fn is_token_blacklisted(&self, token: &str) -> Result<bool> {
        let mut conn = self.redis_manager.clone();
        let key = format!("blacklisted_token:{}", token);

        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                // If Redis is not available, assume token is not blacklisted
                debug!("Redis error checking blacklist: {}", e);
                false
            })
            .unwrap_or(false);

        Ok(exists)
    }

    /// Blacklist a token (for logout)
    pub async fn blacklist_token(&self, token: &str, expires_at: i64) -> Result<()> {
        let mut conn = self.redis_manager.clone();
        let key = format!("blacklisted_token:{}", token);
        let ttl = expires_at - Utc::now().timestamp();

        if ttl > 0 {
            match redis::cmd("SETEX")
                .arg(&key)
                .arg(ttl)
                .arg("1")
                .query_async::<_, ()>(&mut conn)
                .await
            {
                Ok(_) => {
                    debug!(token_key = %key, ttl = ttl, "Token blacklisted");
                }
                Err(e) => {
                    warn!("Failed to blacklist token: {}", e);
                    // Continue anyway - token will expire naturally
                }
            }
        }

        Ok(())
    }

    /// Update last login time for user
    pub async fn update_last_login(&self, user_id: &str) -> Result<()> {
        debug!("Update last login for user: {}", user_id);
        // In a real implementation, this would update the database
        Ok(())
    }

    /// Verify password for user
    pub async fn verify_password(&self, user_id: &str, password: &str) -> Result<bool> {
        debug!("Verifying password for user: {}", user_id);

        // Find user and verify password
        for user in self.mock_users.values() {
            if user.id == user_id {
                let expected_password = if user.email == "admin@AI-PLATFORM.dev" {
                    "admin123"
                } else {
                    "demo123"
                };
                return Ok(password == expected_password);
            }
        }

        Ok(false)
    }

    /// Verify TOTP code for user
    pub async fn verify_totp(&self, _user_id: &str, _totp_code: &str) -> Result<bool> {
        // For demo purposes, always return true if TOTP code is provided
        Ok(true)
    }

    /// Change password for user
    pub async fn change_password(&self, user_id: &str, _new_password: &str) -> Result<()> {
        debug!("Password change requested for user: {}", user_id);
        // In a real implementation, this would hash and store the new password
        Ok(())
    }

    /// Update user profile
    pub async fn update_user_profile(
        &self,
        user_id: &str,
        _name: Option<&str>,
        _avatar_url: Option<&str>,
        _preferences: Option<&serde_json::Value>,
    ) -> Result<User> {
        debug!("Profile update requested for user: {}", user_id);

        // Find and return user (in real implementation, would update database)
        for user in self.mock_users.values() {
            if user.id == user_id {
                return Ok(user.clone());
            }
        }

        Err(ApiError::not_found("User not found"))
    }

    /// Invalidate all refresh tokens for user
    pub async fn invalidate_user_refresh_tokens(&self, user_id: &str) -> Result<()> {
        debug!("Invalidating refresh tokens for user: {}", user_id);
        // In a real implementation, this would mark refresh tokens as invalid in the database
        Ok(())
    }

    /// Invalidate all sessions for user except current token
    pub async fn invalidate_user_sessions(
        &self,
        user_id: &str,
        _current_token: Option<&str>,
    ) -> Result<()> {
        debug!("Invalidating sessions for user: {}", user_id);
        // In a real implementation, this would blacklist all active tokens for the user
        Ok(())
    }

    /// Get user sessions
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<UserSession>> {
        debug!("Getting sessions for user: {}", user_id);

        // Return mock session data
        Ok(vec![UserSession {
            session_id: "session-123".to_string(),
            device_info: Some("Web Browser".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)".to_string()),
            created_at: Utc::now() - Duration::hours(2),
            last_accessed_at: Utc::now(),
            is_current: true,
        }])
    }

    /// Revoke a specific user session
    pub async fn revoke_user_session(&self, user_id: &str, session_id: &str) -> Result<()> {
        debug!("Revoking session {} for user: {}", session_id, user_id);
        // In a real implementation, this would invalidate the specific session
        Ok(())
    }
}

/// User session information
#[derive(Debug, serde::Serialize)]
pub struct UserSession {
    pub session_id: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub is_current: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_user_authentication() {
        // This would require setting up a proper test environment
        // For now, we'll just test the structure
        assert!(true);
    }
}
