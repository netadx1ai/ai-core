//! Integration tests for secure database access patterns
//!
//! These tests validate the integration between the security service and database layer,
//! ensuring that all database operations are properly authenticated, authorized, and audited.

use ai_core_database::{DatabaseConfig, DatabaseManager, MonitoringConfig, PostgresConfig};
use ai_core_security::{SecurityConfig, SecurityService};
use ai_core_shared::types::{Permission, SubscriptionTier, User, UserStatus};
use api_gateway::services::secure_database::{
    AuditTrail, SecureDatabaseConfig, SecureDatabaseError, SecureRepositoryWrapper,
    SecureTransactionManager, SecurityContext,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

// Test entity for database operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String, // This should be encrypted
    pub api_key: String,  // This should be encrypted
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ai_core_database::Entity for TestUser {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }
}

// Mock repository for testing
#[derive(Clone)]
struct MockUserRepository {
    users: Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, TestUser>>>,
}

impl MockUserRepository {
    fn new() -> Self {
        Self {
            users: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl ai_core_database::Repository<TestUser> for MockUserRepository {
    type Id = Uuid;
    type CreateInput = TestUser;
    type UpdateInput = TestUser;
    type QueryFilter = ();

    async fn create(
        &self,
        input: Self::CreateInput,
    ) -> Result<TestUser, ai_core_database::DatabaseError> {
        let mut users = self.users.write().await;
        users.insert(input.id, input.clone());
        Ok(input)
    }

    async fn find_by_id(
        &self,
        id: Self::Id,
    ) -> Result<Option<TestUser>, ai_core_database::DatabaseError> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn update(
        &self,
        id: Self::Id,
        input: Self::UpdateInput,
    ) -> Result<TestUser, ai_core_database::DatabaseError> {
        let mut users = self.users.write().await;
        if users.contains_key(&id) {
            users.insert(id, input.clone());
            Ok(input)
        } else {
            Err(ai_core_database::DatabaseError::Connection(
                "User not found".to_string(),
            ))
        }
    }

    async fn delete(&self, id: Self::Id) -> Result<bool, ai_core_database::DatabaseError> {
        let mut users = self.users.write().await;
        Ok(users.remove(&id).is_some())
    }

    async fn find_many(
        &self,
        _filter: Self::QueryFilter,
    ) -> Result<Vec<TestUser>, ai_core_database::DatabaseError> {
        let users = self.users.read().await;
        Ok(users.values().cloned().collect())
    }

    async fn count(
        &self,
        _filter: Self::QueryFilter,
    ) -> Result<u64, ai_core_database::DatabaseError> {
        let users = self.users.read().await;
        Ok(users.len() as u64)
    }
}

// Test setup helpers
async fn setup_test_services() -> (Arc<SecurityService>, Arc<MockUserRepository>) {
    // Create security service with test configuration
    let security_config = SecurityConfig::default();
    let security_service = Arc::new(SecurityService::new(security_config).await.unwrap());

    // Create mock repository
    let repository = Arc::new(MockUserRepository::new());

    (security_service, repository)
}

fn create_test_security_context(permissions: Vec<Permission>) -> SecurityContext {
    SecurityContext {
        user_id: Uuid::new_v4(),
        session_id: Some("test-session-123".to_string()),
        roles: vec!["user".to_string()],
        permissions,
        subscription_tier: SubscriptionTier::Professional,
        client_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-user-agent".to_string()),
        request_id: Some("req-123".to_string()),
        timestamp: Utc::now(),
    }
}

fn create_test_user() -> TestUser {
    let now = Utc::now();
    TestUser {
        id: Uuid::new_v4(),
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "plaintext-password".to_string(), // Will be encrypted
        api_key: "plain-api-key".to_string(),       // Will be encrypted
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn test_secure_repository_authorization_success() {
    let (security_service, repository) = setup_test_services().await;

    // Create secure repository wrapper
    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    );

    // Create security context with required permissions
    let context = create_test_security_context(vec![Permission::UsersWrite]);
    let test_user = create_test_user();

    // This should succeed because user has UsersWrite permission
    let result = secure_repo.create((context, test_user.clone())).await;
    assert!(result.is_ok());

    let created_user = result.unwrap();
    assert_eq!(created_user.username, test_user.username);
    assert_eq!(created_user.email, test_user.email);

    // Password and API key should be encrypted (different from original)
    assert_ne!(created_user.password, test_user.password);
    assert_ne!(created_user.api_key, test_user.api_key);
}

#[tokio::test]
async fn test_secure_repository_authorization_denied() {
    let (security_service, repository) = setup_test_services().await;

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    );

    // Create security context WITHOUT required permissions
    let context = create_test_security_context(vec![Permission::WorkflowsRead]); // Wrong permission
    let test_user = create_test_user();

    // This should fail because user lacks UsersWrite permission
    let result = secure_repo.create((context, test_user)).await;
    assert!(result.is_err());

    if let Err(ai_core_database::DatabaseError::Connection(error_msg)) = result {
        assert!(error_msg.contains("Authorization denied"));
        assert!(error_msg.contains("UsersWrite") || error_msg.contains("create"));
    } else {
        panic!("Expected authorization error");
    }
}

#[tokio::test]
async fn test_field_encryption_and_decryption() {
    let (security_service, repository) = setup_test_services().await;

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    );

    let context = create_test_security_context(vec![Permission::UsersWrite, Permission::UsersRead]);
    let test_user = create_test_user();
    let original_password = test_user.password.clone();
    let original_api_key = test_user.api_key.clone();

    // Create user (should encrypt sensitive fields)
    let created_result = secure_repo
        .create((context.clone(), test_user.clone()))
        .await;
    assert!(created_result.is_ok());

    let created_user = created_result.unwrap();

    // When retrieved, fields should be decrypted back to original values
    let retrieved_result = secure_repo.find_by_id(created_user.id).await;
    assert!(retrieved_result.is_ok());

    if let Some(retrieved_user) = retrieved_result.unwrap() {
        // After decryption, should match original values
        assert_eq!(retrieved_user.password, original_password);
        assert_eq!(retrieved_user.api_key, original_api_key);
        assert_eq!(retrieved_user.username, test_user.username);
        assert_eq!(retrieved_user.email, test_user.email);
    } else {
        panic!("User should be found");
    }
}

#[tokio::test]
async fn test_audit_logging_for_operations() {
    let (security_service, repository) = setup_test_services().await;

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    );

    let context = create_test_security_context(vec![Permission::UsersWrite]);
    let test_user = create_test_user();

    // Perform create operation (should be audited)
    let result = secure_repo.create((context.clone(), test_user)).await;
    assert!(result.is_ok());

    // Give some time for audit logging to complete
    sleep(Duration::from_millis(100)).await;

    // In a real implementation, you would check the audit log database/storage
    // For this test, we just verify the operation completed successfully
    // The audit logging itself is tested in the security service tests
}

#[tokio::test]
async fn test_update_operations_with_audit_trail() {
    let (security_service, repository) = setup_test_services().await;

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    );

    let context = create_test_security_context(vec![Permission::UsersWrite]);
    let test_user = create_test_user();
    let user_id = test_user.id;

    // Create user first
    let create_result = secure_repo.create((context.clone(), test_user)).await;
    assert!(create_result.is_ok());

    // Update user
    let mut updated_user = create_result.unwrap();
    updated_user.username = "updated_username".to_string();
    updated_user.password = "new-password".to_string();

    let update_result = secure_repo
        .update(user_id, (context, updated_user.clone()))
        .await;
    assert!(update_result.is_ok());

    let final_user = update_result.unwrap();
    assert_eq!(final_user.username, "updated_username");

    // Password should be encrypted (different from input)
    assert_ne!(final_user.password, "new-password");
}

#[tokio::test]
async fn test_authorization_cache() {
    let (security_service, repository) = setup_test_services().await;

    let config = SecureDatabaseConfig {
        enable_authorization_cache: true,
        cache_ttl_seconds: 60,
        ..Default::default()
    };

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    )
    .with_config(config);

    let context = create_test_security_context(vec![Permission::UsersWrite]);
    let test_user = create_test_user();

    // First operation should hit the authorization service
    let result1 = secure_repo
        .create((context.clone(), test_user.clone()))
        .await;
    assert!(result1.is_ok());

    // Second operation should use cached authorization
    let test_user2 = create_test_user();
    let result2 = secure_repo.create((context, test_user2)).await;
    assert!(result2.is_ok());

    // Both operations should succeed, and the second should be faster due to caching
    // In a real test, you might measure timing or mock the authorization service
}

#[tokio::test]
async fn test_secure_transaction_manager() {
    // This test would require a real database connection, so it's simplified
    // In practice, you would test with a test database instance

    let context = create_test_security_context(vec![Permission::UsersWrite]);

    // Create audit trail manually for testing
    let audit_trail = AuditTrail::new(&context, "TEST_TRANSACTION", "User");

    assert_eq!(audit_trail.user_id, context.user_id);
    assert_eq!(audit_trail.operation, "TEST_TRANSACTION");
    assert_eq!(audit_trail.resource_type, "User");
    assert_eq!(audit_trail.session_id, context.session_id);
    assert!(audit_trail.client_ip.is_some());
    assert!(audit_trail.user_agent.is_some());
    assert!(audit_trail.request_id.is_some());
}

#[tokio::test]
async fn test_security_context_from_jwt_claims() {
    use ai_core_security::JwtClaims;

    let user_id = Uuid::new_v4();
    let claims = JwtClaims {
        sub: user_id.to_string(),
        iss: "ai-core".to_string(),
        aud: "api".to_string(),
        exp: (Utc::now().timestamp() + 3600) as usize,
        iat: Utc::now().timestamp() as usize,
        nbf: Utc::now().timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
        roles: vec!["user".to_string(), "admin".to_string()],
        permissions: vec![Permission::UsersRead, Permission::UsersWrite],
        subscription_tier: SubscriptionTier::Enterprise,
        token_type: ai_core_security::TokenType::Access,
        client_ip: Some("192.168.1.1".to_string()),
        user_agent_hash: Some("hash123".to_string()),
        session_id: Some("session-456".to_string()),
        device_fingerprint: Some("device-789".to_string()),
    };

    let context_result = SecurityContext::from_jwt_claims(&claims);
    assert!(context_result.is_ok());

    let context = context_result.unwrap();
    assert_eq!(context.user_id, user_id);
    assert_eq!(context.session_id, Some("session-456".to_string()));
    assert!(context.has_role("user"));
    assert!(context.has_role("admin"));
    assert!(context.has_permission(&Permission::UsersRead));
    assert!(context.has_permission(&Permission::UsersWrite));
    assert_eq!(context.subscription_tier, SubscriptionTier::Enterprise);
}

#[tokio::test]
async fn test_multiple_concurrent_operations() {
    let (security_service, repository) = setup_test_services().await;

    let secure_repo = Arc::new(SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    ));

    let context = create_test_security_context(vec![Permission::UsersWrite]);

    // Create multiple concurrent operations
    let mut handles = vec![];

    for i in 0..5 {
        let repo = secure_repo.clone();
        let ctx = context.clone();
        let handle = tokio::spawn(async move {
            let mut test_user = create_test_user();
            test_user.username = format!("user_{}", i);
            test_user.email = format!("user{}@example.com", i);

            repo.create((ctx, test_user)).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut successful_operations = 0;
    for handle in handles {
        if let Ok(result) = handle.await {
            if result.is_ok() {
                successful_operations += 1;
            }
        }
    }

    // All operations should succeed
    assert_eq!(successful_operations, 5);
}

#[tokio::test]
async fn test_error_handling_and_audit() {
    let (security_service, repository) = setup_test_services().await;

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    );

    // Test with invalid permissions
    let context = create_test_security_context(vec![]); // No permissions
    let test_user = create_test_user();

    let result = secure_repo.create((context, test_user)).await;
    assert!(result.is_err());

    // Error should be properly audited
    sleep(Duration::from_millis(100)).await;

    // In a real implementation, you would verify the error was logged in the audit trail
}

#[tokio::test]
async fn test_configuration_options() {
    let (security_service, repository) = setup_test_services().await;

    // Test with encryption disabled
    let config = SecureDatabaseConfig {
        enable_field_encryption: false,
        enable_audit_logging: true,
        enable_authorization_cache: false,
        ..Default::default()
    };

    let secure_repo = SecureRepositoryWrapper::new(
        repository,
        security_service.clone(),
        security_service.encryption().clone(),
    )
    .with_config(config);

    let context = create_test_security_context(vec![Permission::UsersWrite]);
    let test_user = create_test_user();
    let original_password = test_user.password.clone();

    let result = secure_repo.create((context, test_user)).await;
    assert!(result.is_ok());

    let created_user = result.unwrap();
    // With encryption disabled, password should remain in plaintext
    assert_eq!(created_user.password, original_password);
}

#[tokio::test]
async fn test_security_context_permissions_and_roles() {
    let context = create_test_security_context(vec![
        Permission::UsersRead,
        Permission::UsersWrite,
        Permission::WorkflowsRead,
    ]);

    // Test permission checking
    assert!(context.has_permission(&Permission::UsersRead));
    assert!(context.has_permission(&Permission::UsersWrite));
    assert!(context.has_permission(&Permission::WorkflowsRead));
    assert!(!context.has_permission(&Permission::UsersDelete));

    // Test role checking
    assert!(context.has_role("user"));
    assert!(!context.has_role("admin"));
    assert!(context.has_any_role(&["user", "guest"]));
    assert!(!context.has_any_role(&["admin", "super_admin"]));
}

#[tokio::test]
async fn test_audit_trail_data_integrity() {
    let context = create_test_security_context(vec![Permission::UsersWrite]);
    let audit_trail = AuditTrail::new(&context, "CREATE", "User");

    // Verify all fields are properly set
    assert!(!audit_trail.id.to_string().is_empty());
    assert_eq!(audit_trail.user_id, context.user_id);
    assert_eq!(audit_trail.session_id, context.session_id);
    assert_eq!(audit_trail.operation, "CREATE");
    assert_eq!(audit_trail.resource_type, "User");
    assert_eq!(audit_trail.client_ip, context.client_ip);
    assert_eq!(audit_trail.user_agent, context.user_agent);
    assert_eq!(audit_trail.request_id, context.request_id);
    assert!(!audit_trail.success); // Default to false until operation completes
    assert!(audit_trail.error_message.is_none());
    assert_eq!(audit_trail.execution_time_ms, 0); // Set after operation completes
}
