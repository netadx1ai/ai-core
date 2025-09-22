//! Main Security Service
//!
//! Provides a unified interface to all security components including JWT authentication,
//! RBAC/ABAC authorization, encryption services, and security middleware.

use crate::config::SecurityConfig;
use crate::encryption::PasswordHashResult;
use crate::encryption::{EncryptionService, InMemoryKeyManager, PasswordService};
use crate::errors::{SecurityError, SecurityResult};
use crate::jwt::{JwtService, JwtServiceTrait, TokenPair, ValidationResult};
use crate::rbac::{AuthorizationContext, AuthorizationDecision, RbacService, RedisPermissionCache};
use ai_core_shared::types::User;
use chrono::Duration;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Main security service providing unified access to all security components
pub struct SecurityService {
    /// JWT authentication service
    jwt_service: Arc<JwtService>,
    /// RBAC/ABAC authorization service
    rbac_service: Arc<RbacService>,
    /// Encryption and key management service
    encryption_service: Arc<EncryptionService>,
    /// Password hashing and verification service
    password_service: Arc<PasswordService>,
    /// Security configuration
    config: SecurityConfig,
}

impl SecurityService {
    /// Create a new security service with the provided configuration
    pub async fn new(config: SecurityConfig) -> SecurityResult<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize Redis client
        let redis_client = Arc::new(
            redis::Client::open(config.database.redis_url.as_str())
                .map_err(|e| SecurityError::CacheConnection(e.to_string()))?,
        );

        // Initialize JWT service
        let jwt_service = Arc::new(JwtService::new(
            crate::jwt::JwtConfig {
                secret: config.jwt.secret.clone(),
                issuer: config.jwt.issuer.clone(),
                audience: config.jwt.audience.clone(),
                access_token_ttl: chrono::Duration::from_std(config.jwt.access_token_ttl)
                    .map_err(|e| SecurityError::Configuration(e.to_string()))?,
                refresh_token_ttl: chrono::Duration::from_std(config.jwt.refresh_token_ttl)
                    .map_err(|e| SecurityError::Configuration(e.to_string()))?,
                algorithm: jsonwebtoken::Algorithm::HS256,
                enable_blacklist: config.jwt.enable_blacklist,
                max_tokens_per_user: config.jwt.max_tokens_per_user,
            },
            redis_client.clone(),
        )?);

        // Initialize key manager and encryption service
        let key_manager = InMemoryKeyManager::new(Duration::seconds(
            config.encryption.key_rotation_interval.as_secs() as i64,
        ));
        let encryption_service = Arc::new(EncryptionService::new(key_manager).await?);

        // Initialize password service
        let password_service = Arc::new(PasswordService::new());

        // Initialize permission cache
        let permission_cache = Arc::new(RedisPermissionCache::new(redis_client.clone()));

        // Create a mock role repository (in production, this would be a real database implementation)
        let role_repository = Arc::new(MockRoleRepository::new());

        // Initialize RBAC service
        let rbac_service = Arc::new(RbacService::new(
            role_repository,
            permission_cache,
            crate::rbac::RbacConfig {
                enable_rbac: config.authorization.enable_rbac,
                enable_abac: config.authorization.enable_abac,
                cache_ttl: chrono::Duration::from_std(config.authorization.permission_cache_ttl)
                    .map_err(|e| SecurityError::Configuration(e.to_string()))?,
                admin_override: config.authorization.admin_override,
                evaluation_mode: match config.authorization.evaluation_mode {
                    crate::config::PermissionEvaluationMode::Strict => {
                        crate::rbac::PermissionEvaluationMode::Strict
                    }
                    crate::config::PermissionEvaluationMode::Permissive => {
                        crate::rbac::PermissionEvaluationMode::Permissive
                    }
                },
                max_policy_evaluation_time_ms: 100,
            },
        ));

        Ok(Self {
            jwt_service,
            rbac_service,
            encryption_service,
            password_service,
            config,
        })
    }

    /// Create a security service with default configuration for development
    pub async fn with_defaults() -> SecurityResult<Self> {
        let config = SecurityConfig::default();
        Self::new(config).await
    }

    /// Get the JWT service
    pub fn jwt(&self) -> &JwtService {
        &self.jwt_service
    }

    /// Get the RBAC service
    pub fn rbac(&self) -> &RbacService {
        &self.rbac_service
    }

    /// Get the encryption service
    pub fn encryption(&self) -> &EncryptionService {
        &self.encryption_service
    }

    /// Get the password service
    pub fn password(&self) -> &PasswordService {
        &self.password_service
    }

    /// Get the security configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Authenticate a user and generate JWT tokens
    pub async fn authenticate_user(
        &self,
        user: &User,
        client_ip: Option<String>,
        user_agent: Option<String>,
        device_fingerprint: Option<String>,
    ) -> SecurityResult<TokenPair> {
        self.jwt_service
            .generate_token_pair(user, client_ip, user_agent, device_fingerprint)
            .await
    }

    /// Validate an access token and return user information
    pub async fn validate_token(&self, token: &str) -> SecurityResult<ValidationResult> {
        self.jwt_service.validate_access_token(token).await
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> SecurityResult<TokenPair> {
        self.jwt_service.refresh_token(refresh_token).await
    }

    /// Revoke a specific token
    pub async fn revoke_token(&self, token_id: &str, reason: &str) -> SecurityResult<()> {
        self.jwt_service.revoke_token(token_id, reason).await
    }

    /// Revoke all tokens for a user
    pub async fn revoke_all_user_tokens(&self, user_id: Uuid, reason: &str) -> SecurityResult<()> {
        self.jwt_service
            .revoke_all_user_tokens(user_id, reason)
            .await
    }

    /// Check if a user has permission to perform an action on a resource
    pub async fn check_permission(
        &self,
        user_id: Uuid,
        resource: &str,
        action: &str,
    ) -> SecurityResult<bool> {
        self.rbac_service
            .check_permission(user_id, resource, action)
            .await
    }

    /// Perform comprehensive authorization with context
    pub async fn authorize(
        &self,
        context: &AuthorizationContext,
    ) -> SecurityResult<AuthorizationDecision> {
        self.rbac_service.authorize(context).await
    }

    /// Hash a password securely
    pub fn hash_password(&self, password: &str) -> SecurityResult<PasswordHashResult> {
        self.password_service.hash_password(password)
    }

    /// Verify a password against a hash
    pub fn verify_password(
        &self,
        password: &str,
        hash_result: &PasswordHashResult,
    ) -> SecurityResult<bool> {
        self.password_service.verify_password(password, hash_result)
    }

    /// Check password strength
    pub fn check_password_strength(
        &self,
        password: &str,
    ) -> crate::encryption::PasswordStrengthLevel {
        self.password_service.check_password_strength(password)
    }

    /// Encrypt data using the default encryption algorithm
    pub async fn encrypt(&self, data: &[u8]) -> SecurityResult<crate::encryption::EncryptedData> {
        self.encryption_service.encrypt(data).await
    }

    /// Encrypt data with additional authenticated data
    pub async fn encrypt_with_aad(
        &self,
        data: &[u8],
        aad: &[u8],
    ) -> SecurityResult<crate::encryption::EncryptedData> {
        self.encryption_service.encrypt_with_aad(data, aad).await
    }

    /// Decrypt data (AAD is handled from EncryptedData structure)
    pub async fn decrypt(
        &self,
        encrypted_data: &crate::encryption::EncryptedData,
    ) -> SecurityResult<Vec<u8>> {
        self.encryption_service.decrypt(encrypted_data).await
    }

    /// Perform cleanup operations (remove expired tokens, keys, etc.)
    pub async fn cleanup(&self) -> SecurityResult<()> {
        // Cleanup JWT tokens
        self.jwt_service.cleanup_expired().await?;

        // Cleanup encryption keys
        self.encryption_service
            .key_manager
            .cleanup_expired_keys()
            .await?;

        Ok(())
    }

    /// Get security health status
    pub async fn health_check(&self) -> SecurityResult<SecurityHealthStatus> {
        let mut status = SecurityHealthStatus {
            overall_status: HealthStatus::Healthy,
            jwt_service: HealthStatus::Healthy,
            rbac_service: HealthStatus::Healthy,
            encryption_service: HealthStatus::Healthy,
            cache_status: HealthStatus::Healthy,
            error_messages: Vec::new(),
        };

        // Test JWT service
        if let Err(e) = self.jwt_service.cleanup_expired().await {
            status.jwt_service = HealthStatus::Degraded;
            status.error_messages.push(format!("JWT service: {}", e));
        }

        // Test RBAC service
        if let Err(e) = self.rbac_service.get_authorization_stats().await {
            status.rbac_service = HealthStatus::Degraded;
            status.error_messages.push(format!("RBAC service: {}", e));
        }

        // Test encryption service
        if let Err(e) = self
            .encryption_service
            .key_manager
            .cleanup_expired_keys()
            .await
        {
            status.encryption_service = HealthStatus::Degraded;
            status
                .error_messages
                .push(format!("Encryption service: {}", e));
        }

        // Determine overall status
        if status.jwt_service == HealthStatus::Unhealthy
            || status.rbac_service == HealthStatus::Unhealthy
            || status.encryption_service == HealthStatus::Unhealthy
        {
            status.overall_status = HealthStatus::Unhealthy;
        } else if status.jwt_service == HealthStatus::Degraded
            || status.rbac_service == HealthStatus::Degraded
            || status.encryption_service == HealthStatus::Degraded
        {
            status.overall_status = HealthStatus::Degraded;
        }

        Ok(status)
    }
}

/// Security service health status
#[derive(Debug, Clone)]
pub struct SecurityHealthStatus {
    pub overall_status: HealthStatus,
    pub jwt_service: HealthStatus,
    pub rbac_service: HealthStatus,
    pub encryption_service: HealthStatus,
    pub cache_status: HealthStatus,
    pub error_messages: Vec<String>,
}

/// Health status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

// Mock role repository for demonstration
pub struct MockRoleRepository {
    roles: Arc<RwLock<std::collections::HashMap<Uuid, Vec<crate::rbac::Role>>>>,
}

impl MockRoleRepository {
    pub fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for MockRoleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::rbac::RoleRepository for MockRoleRepository {
    async fn get_user_roles(&self, user_id: Uuid) -> SecurityResult<Vec<crate::rbac::Role>> {
        let roles = self.roles.read().unwrap();
        Ok(roles.get(&user_id).cloned().unwrap_or_default())
    }

    async fn get_role_by_name(&self, _name: &str) -> SecurityResult<Option<crate::rbac::Role>> {
        Ok(None)
    }

    async fn create_role(&self, _role: &crate::rbac::Role) -> SecurityResult<()> {
        Ok(())
    }

    async fn update_role(&self, _role: &crate::rbac::Role) -> SecurityResult<()> {
        Ok(())
    }

    async fn delete_role(&self, _role_id: Uuid) -> SecurityResult<()> {
        Ok(())
    }

    async fn get_role_hierarchy(&self, _role_name: &str) -> SecurityResult<Vec<crate::rbac::Role>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_core_shared::types::SubscriptionTier;
    use chrono::Utc;

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

    #[tokio::test]
    async fn test_security_service_creation() {
        let service = SecurityService::with_defaults().await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_user_authentication() {
        let service = SecurityService::with_defaults().await.unwrap();
        let user = create_test_user();

        let token_pair = service
            .authenticate_user(&user, None, None, None)
            .await
            .unwrap();

        assert!(!token_pair.access_token.token.is_empty());
        assert!(!token_pair.refresh_token.token.is_empty());
    }

    #[tokio::test]
    async fn test_token_validation() {
        let service = SecurityService::with_defaults().await.unwrap();
        let user = create_test_user();

        let token_pair = service
            .authenticate_user(&user, None, None, None)
            .await
            .unwrap();

        let validation_result = service
            .validate_token(&token_pair.access_token.token)
            .await
            .unwrap();

        assert_eq!(validation_result.user_id.to_string(), user.id);
    }

    #[tokio::test]
    async fn test_password_operations() {
        let service = SecurityService::with_defaults().await.unwrap();
        let password = "TestPassword123!";

        let hash = service.hash_password(password).unwrap();
        assert!(!hash.hash.is_empty());

        assert!(service.verify_password(password, &hash).unwrap());
        assert!(!service.verify_password("WrongPassword", &hash).unwrap());

        let strength = service.check_password_strength(password);
        assert!(matches!(
            strength,
            crate::encryption::PasswordStrengthLevel::Strong
                | crate::encryption::PasswordStrengthLevel::VeryStrong
        ));
    }

    #[tokio::test]
    async fn test_encryption_operations() {
        let service = SecurityService::with_defaults().await.unwrap();
        let data = b"Secret message";

        let encrypted = service.encrypt(data).await.unwrap();
        assert!(!encrypted.ciphertext.is_empty());

        let decrypted = service.decrypt(&encrypted).await.unwrap();
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_health_check() {
        let service = SecurityService::with_defaults().await.unwrap();
        let health = service.health_check().await.unwrap();

        assert_eq!(health.overall_status, HealthStatus::Healthy);
    }
}
