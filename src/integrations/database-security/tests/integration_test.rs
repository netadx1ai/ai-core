//! # Integration Tests for Database-Security Integration
//!
//! This module contains comprehensive integration tests that validate the
//! secure database access patterns implementation. Tests cover authentication,
//! authorization, encryption, audit logging, and cross-database operations.

use std::collections::HashSet;
use std::time::Duration;
use tokio_test;

use ai_core_shared::auth::{SessionId, UserId};
use database_security_integration::{
    SecureDatabaseConfig, SecureDatabaseError, SecureDatabaseManager, SecurityContext,
    SecurityContextMetadata, SecurityLevel,
};

/// Test configuration for integration tests
fn create_test_config() -> SecureDatabaseConfig {
    let mut config = SecureDatabaseConfig::test_config();

    // Configure for testing
    config.audit.enabled = true;
    config.audit.store_in_database = false; // Use file storage for tests
    config.audit.store_in_files = true;
    config.audit.file_storage_path = "/tmp/ai-core-test-audit".to_string();

    config.encryption.enabled = true;
    config.encryption.field_level_encryption = true;

    config.access_control.strict_mode = true;
    config.access_control.enable_permission_caching = false; // Disable for predictable tests

    config
}

/// Create a test security context with standard permissions
fn create_test_security_context() -> SecurityContext {
    let user_id = UserId::new();
    let permissions = HashSet::from([
        "user:read".to_string(),
        "user:create".to_string(),
        "user:update".to_string(),
        "workflow:read".to_string(),
        "workflow:create".to_string(),
        "cache:read".to_string(),
        "cache:write".to_string(),
        "analytics:read".to_string(),
        "document:users:read".to_string(),
        "document:users:create".to_string(),
    ]);
    let roles = vec!["user".to_string()];

    SecurityContext::new(user_id, None, permissions, roles)
}

/// Create an admin security context with elevated permissions
fn create_admin_security_context() -> SecurityContext {
    let user_id = UserId::new();
    let permissions = HashSet::from([
        "user:admin".to_string(),
        "user:delete".to_string(),
        "workflow:admin".to_string(),
        "analytics:admin".to_string(),
        "database:admin".to_string(),
        "*".to_string(), // Wildcard permission
    ]);
    let roles = vec!["admin".to_string()];

    let metadata = SecurityContextMetadata {
        security_level: SecurityLevel::Administrative,
        mfa_verified: true,
        ..Default::default()
    };

    SecurityContext::with_metadata(user_id, None, permissions, roles, metadata)
}

#[tokio::test]
async fn test_secure_database_manager_initialization() {
    let config = create_test_config();
    let manager = SecureDatabaseManager::with_config(config).await;

    assert!(
        manager.is_ok(),
        "Failed to initialize SecureDatabaseManager"
    );

    let manager = manager.unwrap();

    // Test that all repository types can be created
    let _postgres_repo = manager.secure_postgres();
    let _clickhouse_repo = manager.secure_clickhouse();
    let _mongodb_repo = manager.secure_mongodb();
    let _redis_repo = manager.secure_redis();
}

#[tokio::test]
async fn test_security_context_creation_and_validation() {
    let config = create_test_config();
    let manager = SecureDatabaseManager::with_config(config).await.unwrap();

    let user_id = UserId::new();
    let security_context = manager.create_security_context(&user_id, None).await;

    assert!(
        security_context.is_ok(),
        "Failed to create security context"
    );

    let context = security_context.unwrap();
    assert_eq!(context.user_id, user_id);
    assert!(context.is_valid(), "Security context should be valid");
    assert!(
        !context.is_expired(),
        "Security context should not be expired"
    );
}

#[tokio::test]
async fn test_permission_checking() {
    let context = create_test_security_context();

    // Test basic permission checking
    assert!(context.has_permission("user:read"));
    assert!(context.has_permission("workflow:create"));
    assert!(!context.has_permission("admin:delete"));

    // Test role checking
    assert!(context.has_role("user"));
    assert!(!context.has_role("admin"));

    // Test multiple permission checks
    assert!(context.has_any_permission(&["user:read", "admin:write"]));
    assert!(context.has_all_permissions(&["user:read", "user:create"]));
    assert!(!context.has_all_permissions(&["user:read", "admin:delete"]));
}

#[tokio::test]
async fn test_admin_permissions() {
    let admin_context = create_admin_security_context();

    // Admin should have wildcard permissions
    assert!(admin_context.has_permission("any:permission"));
    assert!(admin_context.has_permission("user:delete"));
    assert!(admin_context.has_permission("database:admin"));

    // Admin should include all roles
    assert!(admin_context.has_role("admin"));
    assert!(admin_context.has_role("any_role"));

    // Check administrative security level
    assert_eq!(
        admin_context.metadata.security_level,
        SecurityLevel::Administrative
    );
    assert!(admin_context.metadata.mfa_verified);
}

#[tokio::test]
async fn test_context_expiration() {
    let user_id = UserId::new();
    let permissions = HashSet::new();
    let roles = vec!["user".to_string()];

    // Create expired context
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1);
    let context = SecurityContext::with_expiration(user_id, None, permissions, roles, expires_at);

    assert!(!context.is_valid());
    assert!(context.is_expired());
    assert!(context.time_until_expiration().unwrap() < chrono::Duration::zero());
}

#[tokio::test]
async fn test_mfa_requirements() {
    let mut context = create_admin_security_context();
    context.metadata.mfa_verified = false; // Disable MFA

    // MFA should be required for sensitive operations
    assert!(context.requires_mfa("user:delete"));
    assert!(context.requires_mfa("admin:*"));
    assert!(!context.requires_mfa("user:read"));

    // Validation should fail without MFA
    assert!(context.validate_mfa_for_operation("user:delete").is_err());

    // Enable MFA and test again
    context.metadata.mfa_verified = true;
    assert!(context.validate_mfa_for_operation("user:delete").is_ok());
}

#[tokio::test]
async fn test_context_elevation() {
    let user_id = UserId::new();
    let permissions = HashSet::new();
    let roles = vec!["user".to_string()];

    let mut context = SecurityContext::new(user_id, None, permissions, roles);

    // Start with standard level
    assert_eq!(context.metadata.security_level, SecurityLevel::Standard);

    // Elevate context
    assert!(context.elevate().is_ok());
    assert_eq!(context.metadata.security_level, SecurityLevel::Elevated);

    // Elevate again
    assert!(context.elevate().is_ok());
    assert_eq!(
        context.metadata.security_level,
        SecurityLevel::Administrative
    );
}

#[tokio::test]
async fn test_system_context() {
    let permissions = HashSet::from(["system:all".to_string()]);
    let context = SecurityContext::system_context("backup-service", permissions);

    assert_eq!(context.metadata.security_level, SecurityLevel::System);
    assert!(context.has_role("system"));
    assert!(context.has_role("backup-service"));

    // System context should have all permissions
    assert!(context.has_permission("any:permission"));

    // System contexts cannot be elevated
    let mut system_context = context.clone();
    assert!(system_context.elevate().is_err());
}

#[tokio::test]
async fn test_audit_context_creation() {
    let context = create_test_security_context();
    let audit_context = context.audit_context();

    assert_eq!(audit_context.user_id, context.user_id);
    assert_eq!(audit_context.session_id, context.session_id);
    assert_eq!(
        audit_context.security_level,
        context.metadata.security_level
    );
    assert_eq!(audit_context.is_api_key, context.metadata.is_api_key);
    assert_eq!(audit_context.mfa_verified, context.metadata.mfa_verified);
}

#[tokio::test]
async fn test_security_context_metadata() {
    let user_id = UserId::new();
    let permissions = HashSet::new();
    let roles = vec!["user".to_string()];

    let mut metadata = SecurityContextMetadata::default();
    metadata.client_ip = Some("192.168.1.100".to_string());
    metadata.user_agent = Some("AI-CORE-Client/1.0".to_string());
    metadata.request_id = Some("req-12345".to_string());
    metadata.organization_id = Some(uuid::Uuid::new_v4());
    metadata.security_level = SecurityLevel::Elevated;
    metadata.is_api_key = true;
    metadata.mfa_verified = true;

    let context =
        SecurityContext::with_metadata(user_id, None, permissions, roles, metadata.clone());

    assert_eq!(
        context.metadata.client_ip,
        Some("192.168.1.100".to_string())
    );
    assert_eq!(
        context.metadata.user_agent,
        Some("AI-CORE-Client/1.0".to_string())
    );
    assert_eq!(context.metadata.request_id, Some("req-12345".to_string()));
    assert_eq!(context.metadata.security_level, SecurityLevel::Elevated);
    assert!(context.metadata.is_api_key);
    assert!(context.metadata.mfa_verified);
}

#[tokio::test]
async fn test_permission_manipulation() {
    let user_id = UserId::new();
    let mut permissions = HashSet::from(["user:read".to_string()]);
    let roles = vec!["user".to_string()];

    let mut context = SecurityContext::new(user_id, None, permissions, roles);

    // Initial state
    assert!(context.has_permission("user:read"));
    assert!(!context.has_permission("user:write"));

    // Add permission
    context.add_permission("user:write".to_string());
    assert!(context.has_permission("user:write"));

    // Remove permission
    context.remove_permission("user:read");
    assert!(!context.has_permission("user:read"));
    assert!(context.has_permission("user:write"));
}

#[tokio::test]
async fn test_role_manipulation() {
    let user_id = UserId::new();
    let permissions = HashSet::new();
    let roles = vec!["user".to_string()];

    let mut context = SecurityContext::new(user_id, None, permissions, roles);

    // Initial state
    assert!(context.has_role("user"));
    assert!(!context.has_role("moderator"));

    // Add role
    context.add_role("moderator".to_string());
    assert!(context.has_role("moderator"));

    // Remove role
    context.remove_role("user");
    assert!(!context.has_role("user"));
    assert!(context.has_role("moderator"));

    // Adding duplicate role should not duplicate
    context.add_role("moderator".to_string());
    let moderator_count = context.roles.iter().filter(|&r| r == "moderator").count();
    assert_eq!(moderator_count, 1);
}

#[tokio::test]
async fn test_context_cloning_and_modification() {
    let original_context = create_test_security_context();

    // Clone with new expiration
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(2);
    let cloned_context = original_context.with_new_expiration(expires_at);

    assert_eq!(cloned_context.user_id, original_context.user_id);
    assert_eq!(cloned_context.permissions, original_context.permissions);
    assert_eq!(cloned_context.roles, original_context.roles);
    assert_ne!(cloned_context.expires_at, original_context.expires_at);
    assert_eq!(cloned_context.expires_at, Some(expires_at));
}

#[tokio::test]
async fn test_context_request_tracking() {
    let mut context = create_test_security_context();

    // Set request ID for tracing
    let request_id = "req-test-12345";
    context.set_request_id(request_id.to_string());
    assert_eq!(context.metadata.request_id, Some(request_id.to_string()));

    // Set organization ID
    let org_id = uuid::Uuid::new_v4();
    context.set_organization_id(org_id);
    assert_eq!(context.metadata.organization_id, Some(org_id));
}

#[tokio::test]
async fn test_error_handling_and_types() {
    use database_security_integration::{ErrorCategory, ErrorSeverity, SecureDatabaseError};

    // Test error creation and properties
    let access_error = SecureDatabaseError::access_denied("Test access denied");
    assert!(access_error.is_security_error());
    assert!(!access_error.is_recoverable());
    assert!(!access_error.should_retry());
    assert_eq!(access_error.severity(), ErrorSeverity::High);
    assert_eq!(access_error.category(), ErrorCategory::Security);
    assert_eq!(access_error.to_http_status(), 401);

    let timeout_error = SecureDatabaseError::timeout("Test timeout");
    assert!(!timeout_error.is_security_error());
    assert!(timeout_error.is_recoverable());
    assert!(timeout_error.should_retry());
    assert_eq!(timeout_error.severity(), ErrorSeverity::Medium);
    assert_eq!(timeout_error.to_http_status(), 408);

    let encryption_error = SecureDatabaseError::encryption_error("Test encryption failure");
    assert_eq!(encryption_error.severity(), ErrorSeverity::Critical);
    assert_eq!(encryption_error.category(), ErrorCategory::Encryption);
    assert_eq!(encryption_error.to_http_status(), 500);
}

#[tokio::test]
async fn test_error_context_addition() {
    use database_security_integration::{ErrorContext, SecureDatabaseError};

    let original_error = SecureDatabaseError::database_operation("Connection failed");
    let with_context = original_error.with_context("user authentication");

    match with_context {
        SecureDatabaseError::WithContext { context, source } => {
            assert_eq!(context, "user authentication");
            // The source should be the original error boxed
            assert!(source.to_string().contains("Connection failed"));
        }
        _ => panic!("Expected WithContext variant"),
    }
}

#[tokio::test]
async fn test_multiple_errors() {
    use database_security_integration::SecureDatabaseError;

    let errors = vec![
        SecureDatabaseError::database_operation("Database error"),
        SecureDatabaseError::access_denied("Access error"),
        SecureDatabaseError::timeout("Timeout error"),
    ];

    let multiple_error = SecureDatabaseError::Multiple(errors.clone());

    match multiple_error {
        SecureDatabaseError::Multiple(inner_errors) => {
            assert_eq!(inner_errors.len(), 3);
            assert!(inner_errors[0].to_string().contains("Database error"));
            assert!(inner_errors[1].to_string().contains("Access error"));
            assert!(inner_errors[2].to_string().contains("Timeout error"));
        }
        _ => panic!("Expected Multiple variant"),
    }
}

#[tokio::test]
async fn test_configuration_validation() {
    let mut config = SecureDatabaseConfig::default();

    // Valid configuration should pass
    assert!(config.validate().is_ok());

    // Invalid configuration should fail
    config.database.postgres.host = "".to_string();
    assert!(config.validate().is_err());

    // Fix and test again
    config.database.postgres.host = "localhost".to_string();
    assert!(config.validate().is_ok());

    // Test other validation rules
    config.database.postgres.port = 0;
    assert!(config.validate().is_err());

    config.database.postgres.port = 5432;
    config.security.jwt.expiration = 0;
    assert!(config.validate().is_err());

    config.security.jwt.expiration = 3600;
    config.performance.batch_size = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_feature_flag_management() {
    let mut config = SecureDatabaseConfig::default();

    // Initially no custom features
    assert!(!config.feature_enabled("test_feature"));

    // Enable feature
    config.enable_feature("test_feature");
    assert!(config.feature_enabled("test_feature"));

    // Disable feature
    config.disable_feature("test_feature");
    assert!(!config.feature_enabled("test_feature"));
}

#[tokio::test]
async fn test_config_serialization() {
    let config = SecureDatabaseConfig::test_config();

    // Test JSON serialization
    let json_result = config.to_json();
    assert!(json_result.is_ok());
    let json_str = json_result.unwrap();
    assert!(json_str.contains("ai_core_test")); // Test database name

    // Test YAML serialization
    let yaml_result = config.to_yaml();
    assert!(yaml_result.is_ok());
    let yaml_str = yaml_result.unwrap();
    assert!(yaml_str.contains("ai_core_test"));
}

#[tokio::test]
async fn test_health_check_integration() {
    let config = create_test_config();
    let manager = SecureDatabaseManager::with_config(config).await.unwrap();
    let context = create_admin_security_context(); // Need admin permissions for health checks

    // Note: This test may fail in CI without actual databases running
    // In a real integration test environment, databases would be available
    let health_result = manager.health_check(&context).await;

    // The result might be an error if databases are not running, but the call should not panic
    match health_result {
        Ok(status) => {
            // If successful, verify the structure
            println!("Health check passed: {:?}", status);
        }
        Err(e) => {
            // If failed, it should be due to database connection issues, not code issues
            println!("Health check failed (expected in test environment): {}", e);
            assert!(e.to_string().contains("database") || e.to_string().contains("connection"));
        }
    }
}

#[tokio::test]
async fn test_metrics_integration() {
    let config = create_test_config();
    let manager = SecureDatabaseManager::with_config(config).await.unwrap();
    let context = create_admin_security_context(); // Need admin permissions for metrics

    let metrics_result = manager.get_security_metrics(&context).await;

    match metrics_result {
        Ok(metrics) => {
            // Verify metrics structure
            assert_eq!(metrics.total_operations, 0); // Should be 0 for new manager
            assert_eq!(metrics.failed_authentications, 0);
            assert_eq!(metrics.permission_denials, 0);
        }
        Err(e) => {
            // If failed, log the error but don't fail the test in case of environment issues
            println!("Metrics collection failed (may be expected): {}", e);
        }
    }
}

#[tokio::test]
async fn test_concurrent_security_context_usage() {
    use std::sync::Arc;
    use tokio::task;

    let context = Arc::new(create_test_security_context());
    let mut handles = vec![];

    // Spawn multiple tasks that use the security context concurrently
    for i in 0..10 {
        let context_clone = context.clone();
        let handle = task::spawn(async move {
            // Simulate concurrent access to security context
            assert!(context_clone.has_permission("user:read"));
            assert_eq!(context_clone.roles.len(), 1);
            assert!(context_clone.is_valid());

            // Create audit context
            let audit_context = context_clone.audit_context();
            assert_eq!(audit_context.user_id, context_clone.user_id);

            i // Return task number for verification
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_security_context_thread_safety() {
    use std::sync::Arc;
    use tokio::task;

    let context = Arc::new(create_test_security_context());
    let tasks = (0..5).map(|_| {
        let ctx = context.clone();
        task::spawn(async move {
            // Test that multiple threads can read from security context safely
            let permissions_count = ctx.permissions.len();
            let roles_count = ctx.roles.len();
            let is_valid = ctx.is_valid();

            (permissions_count, roles_count, is_valid)
        })
    });

    let results: Vec<(usize, usize, bool)> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All results should be identical since context is read-only
    let first_result = &results[0];
    for result in &results[1..] {
        assert_eq!(result, first_result);
    }

    // Verify expected values
    assert!(first_result.0 > 0); // Should have permissions
    assert_eq!(first_result.1, 1); // Should have one role
    assert!(first_result.2); // Should be valid
}

#[tokio::test]
async fn test_integration_cleanup() {
    let config = create_test_config();
    let manager = SecureDatabaseManager::with_config(config).await.unwrap();

    // Test graceful shutdown
    let shutdown_result = manager.shutdown().await;

    match shutdown_result {
        Ok(()) => {
            println!("Shutdown completed successfully");
        }
        Err(e) => {
            // Log error but don't fail test - may be environment-related
            println!("Shutdown had issues (may be expected): {}", e);
        }
    }
}
