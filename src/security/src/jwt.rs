//! JWT Authentication Service
//!
//! Provides secure JWT token generation, validation, and management with support for
//! token blacklisting, rotation, and comprehensive security features.

use crate::errors::{SecurityError, SecurityResult};
use ai_core_shared::types::{Permission, SubscriptionTier, User};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// JWT service configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
    pub algorithm: Algorithm,
    pub enable_blacklist: bool,
    pub max_tokens_per_user: u32,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-jwt-secret-key-change-in-production".to_string(),
            issuer: "ai-core-platform".to_string(),
            audience: "api".to_string(),
            access_token_ttl: Duration::hours(1),
            refresh_token_ttl: Duration::days(30),
            algorithm: Algorithm::HS256,
            enable_blacklist: true,
            max_tokens_per_user: 10,
        }
    }
}

/// Enhanced JWT claims with additional security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration time (UTC timestamp)
    pub exp: i64,
    /// Issued at (UTC timestamp)
    pub iat: i64,
    /// Not before (UTC timestamp)
    pub nbf: i64,
    /// JWT ID for tracking and revocation
    pub jti: String,
    /// User roles
    pub roles: Vec<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Subscription tier
    pub subscription_tier: String,
    /// Token type (access or refresh)
    pub token_type: TokenType,
    /// Client IP address for additional security
    pub client_ip: Option<String>,
    /// User agent hash for device tracking
    pub user_agent_hash: Option<String>,
    /// Session ID for multi-session management
    pub session_id: String,
    /// Device fingerprint
    pub device_fingerprint: Option<String>,
}

/// Token type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenType {
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "refresh")]
    Refresh,
}

/// Access token wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub token_type: String,
    pub scope: String,
}

/// Refresh token wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub session_id: String,
}

/// Token pair response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: AccessToken,
    pub refresh_token: RefreshToken,
}

/// Token validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub claims: JwtClaims,
    pub user_id: Uuid,
    pub roles: Vec<String>,
    pub permissions: HashSet<Permission>,
    pub subscription_tier: SubscriptionTier,
    pub session_id: String,
}

/// Token blacklist entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlacklistEntry {
    pub token_id: String,
    pub user_id: Uuid,
    pub reason: String,
    pub blacklisted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Session information for tracking active tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: Uuid,
    pub tokens: Vec<String>, // JTI list
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
}

/// JWT service trait for dependency injection
#[async_trait]
pub trait JwtServiceTrait: Send + Sync {
    async fn generate_token_pair(
        &self,
        user: &User,
        client_ip: Option<String>,
        user_agent: Option<String>,
        device_fingerprint: Option<String>,
    ) -> SecurityResult<TokenPair>;

    async fn validate_access_token(&self, token: &str) -> SecurityResult<ValidationResult>;

    async fn refresh_token(&self, refresh_token: &str) -> SecurityResult<TokenPair>;

    async fn revoke_token(&self, token_id: &str, reason: &str) -> SecurityResult<()>;

    async fn revoke_all_user_tokens(&self, user_id: Uuid, reason: &str) -> SecurityResult<()>;

    async fn get_active_sessions(&self, user_id: Uuid) -> SecurityResult<Vec<SessionInfo>>;

    async fn revoke_session(&self, session_id: &str) -> SecurityResult<()>;
}

/// Main JWT service implementation
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    redis_client: Arc<redis::Client>,
    token_blacklist: Arc<DashMap<String, BlacklistEntry>>,
    user_sessions: Arc<DashMap<Uuid, Vec<SessionInfo>>>,
    validation_cache: Arc<RwLock<DashMap<String, (ValidationResult, DateTime<Utc>)>>>,
}

impl JwtService {
    /// Create a new JWT service instance
    pub fn new(config: JwtConfig, redis_client: Arc<redis::Client>) -> SecurityResult<Self> {
        if config.secret.len() < 32 {
            return Err(SecurityError::Configuration(
                "JWT secret must be at least 32 characters".to_string(),
            ));
        }

        let secret_bytes = config.secret.as_bytes();
        let encoding_key = EncodingKey::from_secret(secret_bytes);
        let decoding_key = DecodingKey::from_secret(secret_bytes);

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            redis_client,
            token_blacklist: Arc::new(DashMap::new()),
            user_sessions: Arc::new(DashMap::new()),
            validation_cache: Arc::new(RwLock::new(DashMap::new())),
        })
    }

    /// Generate a unique token ID
    fn generate_token_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Generate a unique session ID
    fn generate_session_id() -> String {
        format!("sess_{}", Uuid::new_v4())
    }

    /// Hash user agent for privacy
    fn hash_user_agent(user_agent: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(user_agent.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Create JWT claims for a user
    fn create_claims(
        &self,
        user: &User,
        token_type: TokenType,
        session_id: String,
        client_ip: Option<String>,
        user_agent: Option<String>,
        device_fingerprint: Option<String>,
    ) -> SecurityResult<JwtClaims> {
        let now = Utc::now();
        let ttl = match token_type {
            TokenType::Access => self.config.access_token_ttl,
            TokenType::Refresh => self.config.refresh_token_ttl,
        };

        let permissions: Vec<String> = user.permissions.iter().map(|p| p.to_string()).collect();

        Ok(JwtClaims {
            sub: user.id.to_string(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            exp: (now + ttl).timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: Self::generate_token_id(),
            roles: user.roles.clone(),
            permissions,
            subscription_tier: user.subscription_tier.to_string(),
            token_type,
            client_ip,
            user_agent_hash: user_agent.map(|ua| Self::hash_user_agent(&ua)),
            session_id,
            device_fingerprint,
        })
    }

    /// Encode JWT token
    fn encode_token(&self, claims: &JwtClaims) -> SecurityResult<String> {
        let header = Header::new(self.config.algorithm);
        encode(&header, claims, &self.encoding_key)
            .map_err(|e| SecurityError::TokenGeneration(e.to_string()))
    }

    /// Decode and validate JWT token
    fn decode_token(&self, token: &str) -> SecurityResult<JwtClaims> {
        let mut validation = Validation::new(self.config.algorithm);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Check if token is blacklisted
    async fn is_token_blacklisted(&self, token_id: &str) -> SecurityResult<bool> {
        // Check local cache first
        if self.token_blacklist.contains_key(token_id) {
            return Ok(true);
        }

        // Check Redis for distributed blacklist
        if self.config.enable_blacklist {
            let mut conn = self
                .redis_client
                .get_async_connection()
                .await
                .map_err(|e| SecurityError::CacheConnection(e.to_string()))?;

            let blacklist_key = format!("jwt:blacklist:{}", token_id);
            let exists: bool = conn
                .exists(&blacklist_key)
                .await
                .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;

            if exists {
                // Cache locally for performance
                let entry_data: String = conn
                    .get(&blacklist_key)
                    .await
                    .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;

                if let Ok(entry) = serde_json::from_str::<BlacklistEntry>(&entry_data) {
                    self.token_blacklist.insert(token_id.to_string(), entry);
                }
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Add token to blacklist
    async fn blacklist_token(
        &self,
        token_id: &str,
        user_id: Uuid,
        reason: &str,
        expires_at: DateTime<Utc>,
    ) -> SecurityResult<()> {
        if !self.config.enable_blacklist {
            return Ok(());
        }

        let entry = BlacklistEntry {
            token_id: token_id.to_string(),
            user_id,
            reason: reason.to_string(),
            blacklisted_at: Utc::now(),
            expires_at,
        };

        // Add to local cache
        self.token_blacklist
            .insert(token_id.to_string(), entry.clone());

        // Add to Redis for distributed blacklist
        let mut conn = self
            .redis_client
            .get_async_connection()
            .await
            .map_err(|e| SecurityError::CacheConnection(e.to_string()))?;

        let blacklist_key = format!("jwt:blacklist:{}", token_id);
        let entry_json = serde_json::to_string(&entry)
            .map_err(|e| SecurityError::Serialization(e.to_string()))?;

        let ttl_seconds = (expires_at - Utc::now()).num_seconds().max(0) as u64;

        conn.set_ex::<_, _, ()>(&blacklist_key, entry_json, ttl_seconds)
            .await
            .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;

        info!(
            "Token blacklisted: token_id={}, user_id={}, reason={}",
            token_id, user_id, reason
        );

        Ok(())
    }

    /// Manage user sessions
    async fn add_session(&self, user_id: Uuid, session_info: SessionInfo) -> SecurityResult<()> {
        let mut user_sessions = self.user_sessions.entry(user_id).or_insert_with(Vec::new);

        // Enforce maximum sessions per user
        if user_sessions.len() >= self.config.max_tokens_per_user as usize {
            // Remove oldest session
            if let Some(oldest_session) = user_sessions.first() {
                warn!(
                    "Maximum sessions exceeded for user {}, revoking oldest session {}",
                    user_id, oldest_session.session_id
                );
                for token_id in &oldest_session.tokens {
                    self.blacklist_token(
                        token_id,
                        user_id,
                        "session_limit_exceeded",
                        Utc::now() + Duration::days(1),
                    )
                    .await?;
                }
            }
            user_sessions.remove(0);
        }

        user_sessions.push(session_info);
        Ok(())
    }

    /// Clean up expired tokens and sessions
    pub async fn cleanup_expired(&self) -> SecurityResult<()> {
        let now = Utc::now();
        let mut cleanup_count = 0;

        // Clean up blacklist
        self.token_blacklist.retain(|_, entry| {
            if entry.expires_at <= now {
                cleanup_count += 1;
                false
            } else {
                true
            }
        });

        // Clean up validation cache
        let cache = self.validation_cache.write().await;
        cache.retain(|_, (_, cached_at)| {
            now.signed_duration_since(*cached_at) < Duration::minutes(5)
        });

        debug!("Cleaned up {} expired security entries", cleanup_count);
        Ok(())
    }

    /// Parse permissions from string list
    fn parse_permissions(permissions: &[String]) -> HashSet<Permission> {
        permissions.iter().filter_map(|p| p.parse().ok()).collect()
    }

    /// Parse subscription tier from string
    fn parse_subscription_tier(tier: &str) -> SubscriptionTier {
        match tier.to_lowercase().as_str() {
            "pro" => SubscriptionTier::Pro,
            "enterprise" => SubscriptionTier::Enterprise,
            _ => SubscriptionTier::Free,
        }
    }
}

#[async_trait]
impl JwtServiceTrait for JwtService {
    async fn generate_token_pair(
        &self,
        user: &User,
        client_ip: Option<String>,
        user_agent: Option<String>,
        device_fingerprint: Option<String>,
    ) -> SecurityResult<TokenPair> {
        let session_id = Self::generate_session_id();

        // Generate access token
        let access_claims = self.create_claims(
            user,
            TokenType::Access,
            session_id.clone(),
            client_ip.clone(),
            user_agent.clone(),
            device_fingerprint.clone(),
        )?;

        let access_token = self.encode_token(&access_claims)?;

        // Generate refresh token
        let refresh_claims = self.create_claims(
            user,
            TokenType::Refresh,
            session_id.clone(),
            client_ip.clone(),
            user_agent.clone(),
            device_fingerprint,
        )?;

        let refresh_token = self.encode_token(&refresh_claims)?;

        // Create session info
        let session_info = SessionInfo {
            session_id: session_id.clone(),
            user_id: user.id.parse().unwrap(),
            tokens: vec![access_claims.jti.clone(), refresh_claims.jti.clone()],
            created_at: Utc::now(),
            last_activity: Utc::now(),
            device_info: user_agent,
            ip_address: client_ip,
        };

        // Add session to tracking
        self.add_session(user.id.parse().unwrap(), session_info)
            .await?;

        info!(
            "Generated token pair for user {} (session: {})",
            user.id, session_id
        );

        Ok(TokenPair {
            access_token: AccessToken {
                token: access_token,
                expires_at: DateTime::from_timestamp(access_claims.exp, 0)
                    .unwrap_or_else(|| Utc::now() + self.config.access_token_ttl),
                token_type: "Bearer".to_string(),
                scope: "api".to_string(),
            },
            refresh_token: RefreshToken {
                token: refresh_token,
                expires_at: DateTime::from_timestamp(refresh_claims.exp, 0)
                    .unwrap_or_else(|| Utc::now() + self.config.refresh_token_ttl),
                user_id: user.id.parse().unwrap(),
                session_id,
            },
        })
    }

    async fn validate_access_token(&self, token: &str) -> SecurityResult<ValidationResult> {
        // Check validation cache first
        let cached_result = {
            let cache = self.validation_cache.read().await;
            cache.get(token).map(|entry| {
                let (result, cached_at) = entry.value();
                (result.clone(), *cached_at)
            })
        };

        if let Some((result, cached_at)) = cached_result {
            if Utc::now().signed_duration_since(cached_at) < Duration::minutes(1) {
                debug!("Token validation cache hit");
                return Ok(result);
            }
        }

        // Decode and validate token
        let claims = self.decode_token(token)?;

        // Verify token type
        if claims.token_type != TokenType::Access {
            return Err(SecurityError::InvalidToken(
                "Expected access token".to_string(),
            ));
        }

        // Check if token is blacklisted
        if self.is_token_blacklisted(&claims.jti).await? {
            return Err(SecurityError::TokenBlacklisted(claims.jti));
        }

        // Parse user ID
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| SecurityError::InvalidToken("Invalid user ID in token".to_string()))?;

        // Parse permissions and subscription tier
        let permissions = Self::parse_permissions(&claims.permissions);
        let subscription_tier = Self::parse_subscription_tier(&claims.subscription_tier);

        let result = ValidationResult {
            claims: claims.clone(),
            user_id,
            roles: claims.roles.clone(),
            permissions,
            subscription_tier,
            session_id: claims.session_id.clone(),
        };

        // Cache validation result
        {
            let cache = self.validation_cache.write().await;
            cache.insert(token.to_string(), (result.clone(), Utc::now()));
        }

        debug!("Access token validated for user {}", user_id);
        Ok(result)
    }

    async fn refresh_token(&self, refresh_token: &str) -> SecurityResult<TokenPair> {
        // Decode and validate refresh token
        let claims = self.decode_token(refresh_token)?;

        // Verify token type
        if claims.token_type != TokenType::Refresh {
            return Err(SecurityError::InvalidToken(
                "Expected refresh token".to_string(),
            ));
        }

        // Check if token is blacklisted
        if self.is_token_blacklisted(&claims.jti).await? {
            return Err(SecurityError::TokenBlacklisted(claims.jti));
        }

        // Parse user ID
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| SecurityError::InvalidToken("Invalid user ID in token".to_string()))?;

        // Create a temporary user object for token generation
        // In practice, you would fetch the current user from database
        let user = User {
            id: user_id.to_string(),
            email: String::new(), // Would be fetched from database
            username: String::new(),
            password_hash: String::new(),
            name: String::new(),
            avatar_url: None,
            email_verified: true,
            is_active: true,
            subscription_tier: Self::parse_subscription_tier(&claims.subscription_tier),
            roles: claims.roles.clone(),
            permissions: claims.permissions.iter().map(|p| p.to_string()).collect(),
            totp_secret: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: Some(Utc::now()),
            preferences: Some(serde_json::Value::Null),
        };

        // Blacklist the old refresh token
        self.blacklist_token(
            &claims.jti,
            user_id,
            "token_refreshed",
            DateTime::from_timestamp(claims.exp, 0)
                .unwrap_or_else(|| Utc::now() + Duration::hours(1)),
        )
        .await?;

        // Generate new token pair
        let new_token_pair = self
            .generate_token_pair(
                &user,
                claims.client_ip,
                None, // User agent not available from token
                claims.device_fingerprint,
            )
            .await?;

        info!("Refreshed token pair for user {}", user_id);
        Ok(new_token_pair)
    }

    async fn revoke_token(&self, token_id: &str, reason: &str) -> SecurityResult<()> {
        // Find the token in active sessions to get user_id and expiration
        let mut user_id_opt = None;
        let expires_at = Utc::now() + Duration::days(1); // Default expiration

        for user_sessions in self.user_sessions.iter() {
            for session in user_sessions.value() {
                if session.tokens.contains(&token_id.to_string()) {
                    user_id_opt = Some(*user_sessions.key());
                    break;
                }
            }
        }

        let user_id = user_id_opt.ok_or_else(|| {
            SecurityError::InvalidToken("Token not found in active sessions".to_string())
        })?;

        self.blacklist_token(token_id, user_id, reason, expires_at)
            .await?;

        info!("Revoked token: token_id={}, reason={}", token_id, reason);
        Ok(())
    }

    async fn revoke_all_user_tokens(&self, user_id: Uuid, reason: &str) -> SecurityResult<()> {
        if let Some(sessions) = self.user_sessions.get(&user_id) {
            for session in sessions.value() {
                for token_id in &session.tokens {
                    self.blacklist_token(token_id, user_id, reason, Utc::now() + Duration::days(1))
                        .await?;
                }
            }
            // Clear user sessions
            drop(sessions);
            self.user_sessions.remove(&user_id);
        }

        info!(
            "Revoked all tokens for user {}, reason: {}",
            user_id, reason
        );
        Ok(())
    }

    async fn get_active_sessions(&self, user_id: Uuid) -> SecurityResult<Vec<SessionInfo>> {
        if let Some(sessions) = self.user_sessions.get(&user_id) {
            Ok(sessions.value().clone())
        } else {
            Ok(Vec::new())
        }
    }

    async fn revoke_session(&self, session_id: &str) -> SecurityResult<()> {
        // Find and remove the session
        let mut found_session = None;
        let mut found_user_id = None;

        for entry in self.user_sessions.iter() {
            let user_id = *entry.key();
            let sessions = entry.value();

            if let Some(pos) = sessions.iter().position(|s| s.session_id == session_id) {
                found_session = Some(sessions[pos].clone());
                found_user_id = Some(user_id);
                break;
            }
        }

        if let (Some(session), Some(user_id)) = (found_session, found_user_id) {
            // Blacklist all tokens in the session
            for token_id in &session.tokens {
                self.blacklist_token(
                    token_id,
                    user_id,
                    "session_revoked",
                    Utc::now() + chrono::TimeDelta::days(1),
                )
                .await?;
            }

            // Remove the session from the user's sessions
            self.user_sessions.alter(&user_id, |_, mut sessions| {
                sessions.retain(|s| s.session_id != session_id);
                sessions
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_core_shared::types::UserStatus;

    fn create_test_user() -> User {
        User {
            id: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password_hash: "hashed_password".to_string(),
            name: "Test User".to_string(),
            avatar_url: None,
            email_verified: true,
            is_active: true,
            subscription_tier: SubscriptionTier::Pro,
            roles: vec!["user".to_string()],
            permissions: vec![
                format!("{:?}", Permission::WorkflowsRead),
                format!("{:?}", Permission::ContentRead),
            ],
            totp_secret: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: Some(Utc::now()),
            preferences: Some(serde_json::Value::Null),
        }
    }

    fn create_test_jwt_service() -> JwtService {
        let config = JwtConfig {
            secret: "test-secret-key-32-characters-long".to_string(),
            ..Default::default()
        };
        let redis_client = Arc::new(redis::Client::open("redis://localhost").unwrap());
        JwtService::new(config, redis_client).unwrap()
    }

    #[tokio::test]
    async fn test_token_generation() {
        let service = create_test_jwt_service();
        let user = create_test_user();

        let token_pair = service
            .generate_token_pair(&user, None, None, None)
            .await
            .unwrap();

        assert!(!token_pair.access_token.token.is_empty());
        assert!(!token_pair.refresh_token.token.is_empty());
        assert_eq!(token_pair.access_token.token_type, "Bearer");
    }

    #[tokio::test]
    async fn test_token_validation() {
        let service = create_test_jwt_service();
        let user = create_test_user();

        let token_pair = service
            .generate_token_pair(&user, None, None, None)
            .await
            .unwrap();

        let validation_result = service
            .validate_access_token(&token_pair.access_token.token)
            .await
            .unwrap();

        assert_eq!(validation_result.user_id.to_string(), user.id);
        assert_eq!(validation_result.roles, user.roles);
        assert!(validation_result
            .permissions
            .contains(&Permission::WorkflowsRead));
    }

    #[test]
    fn test_jwt_claims_creation() {
        let service = create_test_jwt_service();
        let user = create_test_user();
        let session_id = "test_session".to_string();

        let claims = service
            .create_claims(
                &user,
                TokenType::Access,
                session_id.clone(),
                Some("127.0.0.1".to_string()),
                Some("Test User Agent".to_string()),
                None,
            )
            .unwrap();

        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.token_type, TokenType::Access);
        assert_eq!(claims.session_id, session_id);
        assert_eq!(claims.client_ip, Some("127.0.0.1".to_string()));
        assert!(claims.user_agent_hash.is_some());
    }

    #[test]
    fn test_permission_parsing() {
        let permissions = vec![
            "WorkflowsRead".to_string(),
            "ContentCreate".to_string(),
            "InvalidPermission".to_string(),
        ];

        let parsed = JwtService::parse_permissions(&permissions);

        assert_eq!(parsed.len(), 2);
        assert!(parsed.contains(&Permission::WorkflowsRead));
        assert!(parsed.contains(&Permission::ContentCreate));
    }

    #[test]
    fn test_subscription_tier_parsing() {
        assert_eq!(
            JwtService::parse_subscription_tier("pro"),
            SubscriptionTier::Pro
        );
        assert_eq!(
            JwtService::parse_subscription_tier("enterprise"),
            SubscriptionTier::Enterprise
        );
        assert_eq!(
            JwtService::parse_subscription_tier("free"),
            SubscriptionTier::Free
        );
        assert_eq!(
            JwtService::parse_subscription_tier("invalid"),
            SubscriptionTier::Free
        );
    }

    #[test]
    fn test_user_agent_hashing() {
        let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";
        let hash1 = JwtService::hash_user_agent(user_agent);
        let hash2 = JwtService::hash_user_agent(user_agent);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex string length
    }
}
