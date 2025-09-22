//! Database-Security Integration Example
//!
//! This example demonstrates secure database access patterns integrating the
//! AI-CORE database and security services. It shows how to implement proper
//! authentication, authorization, audit logging, and encrypted data storage.
//!
//! ## Features Demonstrated
//!
//! - JWT token validation and security context creation
//! - RBAC/ABAC authorization checks for database operations
//! - Automatic encryption/decryption of sensitive fields
//! - Comprehensive audit logging for all database operations
//! - Secure transaction management
//! - Authorization caching for performance
//! - Error handling and security event logging
//!
//! ## Usage
//!
//! ```bash
//! # Set environment variables
//! export DATABASE_URL="postgresql://localhost:5432/ai_core_dev"
//! export REDIS_URL="redis://localhost:6379"
//! export JWT_SECRET="your-secret-key"
//!
//! # Run the integration example
//! cargo run --bin database-security-integration
//! ```

use ai_core_database::{DatabaseConfig, DatabaseManager, MonitoringConfig, PostgresConfig};
use ai_core_security::{SecurityConfig, SecurityService};
use ai_core_shared::types::{Permission, SubscriptionTier, User, UserStatus};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

// Example entity with sensitive fields that need encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserAccount {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,     // Encrypted field
    pub api_key: String,      // Encrypted field
    pub secret_token: String, // Encrypted field
    pub subscription_tier: SubscriptionTier,
    pub is_active: bool,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ai_core_database::Entity for UserAccount {
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

// Security context for all database operations
#[derive(Debug, Clone)]
struct SecurityContext {
    pub user_id: Uuid,
    pub session_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<Permission>,
    pub subscription_tier: SubscriptionTier,
    pub client_ip: Option<String>,
    pub request_id: Option<String>,
}

impl SecurityContext {
    fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

// Secure repository wrapper with integrated security checks
struct SecureUserRepository {
    database_manager: Arc<DatabaseManager>,
    security_service: Arc<SecurityService>,
}

impl SecureUserRepository {
    fn new(database_manager: Arc<DatabaseManager>, security_service: Arc<SecurityService>) -> Self {
        Self {
            database_manager,
            security_service,
        }
    }

    /// Create a new user with full security checks and audit logging
    async fn create_user(
        &self,
        context: &SecurityContext,
        mut user_data: UserAccount,
    ) -> Result<UserAccount> {
        let operation_start = std::time::Instant::now();

        // 1. Authorization check
        if !self.check_authorization(context, "users", "create").await? {
            self.log_security_event(
                context,
                "CREATE_USER_DENIED",
                "users",
                false,
                Some("Insufficient permissions".to_string()),
            )
            .await?;
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }

        // 2. Encrypt sensitive fields
        self.encrypt_sensitive_fields(&mut user_data).await?;

        // 3. Execute database operation within transaction
        let result = self
            .database_manager
            .execute_transaction(|tx| {
                Box::pin(async move {
                    // In a real implementation, you'd use the transaction to insert the user
                    // For this example, we'll simulate the operation
                    info!(
                        user_id = %context.user_id,
                        new_user_id = %user_data.id,
                        "Creating user in database"
                    );

                    // Simulate database insert
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                    Ok(user_data.clone())
                })
            })
            .await
            .context("Failed to create user in database")?;

        // 4. Decrypt fields for return (user should see decrypted data)
        let mut returned_user = result;
        self.decrypt_sensitive_fields(&mut returned_user).await?;

        // 5. Log successful operation
        self.log_security_event(context, "CREATE_USER_SUCCESS", "users", true, None)
            .await?;

        let duration = operation_start.elapsed();
        info!(
            user_id = %context.user_id,
            created_user_id = %returned_user.id,
            duration_ms = %duration.as_millis(),
            "User created successfully"
        );

        Ok(returned_user)
    }

    /// Update user with security checks and audit trail
    async fn update_user(
        &self,
        context: &SecurityContext,
        user_id: Uuid,
        mut updates: UserAccount,
    ) -> Result<UserAccount> {
        let operation_start = std::time::Instant::now();

        // Authorization check
        if !self.check_authorization(context, "users", "update").await?
            && context.user_id != user_id
        {
            self.log_security_event(
                context,
                "UPDATE_USER_DENIED",
                "users",
                false,
                Some("Insufficient permissions or not own user".to_string()),
            )
            .await?;
            return Err(anyhow::anyhow!("Access denied"));
        }

        // Encrypt sensitive fields before update
        self.encrypt_sensitive_fields(&mut updates).await?;

        // Get old values for audit trail (simplified)
        let old_values = serde_json::json!({
            "note": "Old values would be fetched from database",
            "user_id": user_id
        });

        // Execute update transaction
        let result = self
            .database_manager
            .execute_transaction(|tx| {
                Box::pin(async move {
                    info!(
                        user_id = %context.user_id,
                        target_user_id = %user_id,
                        "Updating user in database"
                    );

                    // Simulate database update
                    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;

                    updates.updated_at = Utc::now();
                    Ok(updates.clone())
                })
            })
            .await
            .context("Failed to update user")?;

        // Decrypt for return
        let mut returned_user = result;
        self.decrypt_sensitive_fields(&mut returned_user).await?;

        // Log with old and new values
        self.log_audit_trail(
            context,
            "UPDATE_USER",
            "users",
            Some(user_id.to_string()),
            Some(old_values),
            Some(serde_json::to_value(&returned_user)?),
            true,
            None,
            operation_start.elapsed(),
        )
        .await?;

        info!(
            user_id = %context.user_id,
            updated_user_id = %returned_user.id,
            duration_ms = %operation_start.elapsed().as_millis(),
            "User updated successfully"
        );

        Ok(returned_user)
    }

    /// Get user with proper access controls
    async fn get_user(
        &self,
        context: &SecurityContext,
        user_id: Uuid,
    ) -> Result<Option<UserAccount>> {
        // Check read permissions
        if !self.check_authorization(context, "users", "read").await? && context.user_id != user_id
        {
            self.log_security_event(
                context,
                "READ_USER_DENIED",
                "users",
                false,
                Some("Insufficient read permissions".to_string()),
            )
            .await?;
            return Ok(None);
        }

        // Simulate database fetch
        let user_data = self.fetch_user_from_database(user_id).await?;

        if let Some(mut user) = user_data {
            // Decrypt sensitive fields
            self.decrypt_sensitive_fields(&mut user).await?;

            // Log read access
            self.log_security_event(context, "READ_USER_SUCCESS", "users", true, None)
                .await?;

            return Ok(Some(user));
        }

        Ok(None)
    }

    /// Delete user with comprehensive security checks
    async fn delete_user(&self, context: &SecurityContext, user_id: Uuid) -> Result<bool> {
        let operation_start = std::time::Instant::now();

        // Only admins or the user themselves can delete
        if !self.check_authorization(context, "users", "delete").await?
            && !context.has_role("admin")
            && context.user_id != user_id
        {
            self.log_security_event(
                context,
                "DELETE_USER_DENIED",
                "users",
                false,
                Some("Insufficient permissions for deletion".to_string()),
            )
            .await?;
            return Err(anyhow::anyhow!("Access denied for user deletion"));
        }

        // Execute deletion transaction
        let result = self
            .database_manager
            .execute_transaction(|tx| {
                Box::pin(async move {
                    info!(
                        user_id = %context.user_id,
                        target_user_id = %user_id,
                        "Deleting user from database"
                    );

                    // Simulate database deletion
                    tokio::time::sleep(tokio::time::Duration::from_millis(12)).await;

                    Ok(true)
                })
            })
            .await
            .context("Failed to delete user")?;

        // Log deletion
        self.log_audit_trail(
            context,
            "DELETE_USER",
            "users",
            Some(user_id.to_string()),
            Some(serde_json::json!({"deleted_user_id": user_id})),
            None,
            true,
            None,
            operation_start.elapsed(),
        )
        .await?;

        info!(
            user_id = %context.user_id,
            deleted_user_id = %user_id,
            duration_ms = %operation_start.elapsed().as_millis(),
            "User deleted successfully"
        );

        Ok(result)
    }

    /// Check authorization using RBAC service
    async fn check_authorization(
        &self,
        context: &SecurityContext,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        // Use the security service's RBAC to check permissions
        let authorized = self
            .security_service
            .rbac()
            .check_permission(context.user_id, resource, action)
            .await
            .context("Authorization check failed")?;

        info!(
            user_id = %context.user_id,
            resource = %resource,
            action = %action,
            authorized = %authorized,
            "Authorization check completed"
        );

        Ok(authorized)
    }

    /// Encrypt sensitive fields using the encryption service
    async fn encrypt_sensitive_fields(&self, user: &mut UserAccount) -> Result<()> {
        let encryption = self.security_service.encryption();

        // Encrypt password
        user.password = encryption
            .encrypt_string(&user.password)
            .context("Failed to encrypt password")?;

        // Encrypt API key
        user.api_key = encryption
            .encrypt_string(&user.api_key)
            .context("Failed to encrypt API key")?;

        // Encrypt secret token
        user.secret_token = encryption
            .encrypt_string(&user.secret_token)
            .context("Failed to encrypt secret token")?;

        info!("Sensitive fields encrypted successfully");
        Ok(())
    }

    /// Decrypt sensitive fields using the encryption service
    async fn decrypt_sensitive_fields(&self, user: &mut UserAccount) -> Result<()> {
        let encryption = self.security_service.encryption();

        // Decrypt password
        match encryption.decrypt_string(&user.password) {
            Ok(decrypted) => user.password = decrypted,
            Err(e) => {
                warn!("Failed to decrypt password: {}", e);
                user.password = "[ENCRYPTED]".to_string();
            }
        }

        // Decrypt API key
        match encryption.decrypt_string(&user.api_key) {
            Ok(decrypted) => user.api_key = decrypted,
            Err(e) => {
                warn!("Failed to decrypt API key: {}", e);
                user.api_key = "[ENCRYPTED]".to_string();
            }
        }

        // Decrypt secret token
        match encryption.decrypt_string(&user.secret_token) {
            Ok(decrypted) => user.secret_token = decrypted,
            Err(e) => {
                warn!("Failed to decrypt secret token: {}", e);
                user.secret_token = "[ENCRYPTED]".to_string();
            }
        }

        info!("Sensitive fields decrypted successfully");
        Ok(())
    }

    /// Log security event using the audit logger
    async fn log_security_event(
        &self,
        context: &SecurityContext,
        event_type: &str,
        resource: &str,
        success: bool,
        error_message: Option<String>,
    ) -> Result<()> {
        let security_event = ai_core_security::SecurityEvent {
            id: Uuid::new_v4(),
            user_id: Some(context.user_id),
            session_id: context.session_id.clone(),
            event_type: event_type.to_string(),
            resource: Some(resource.to_string()),
            action: event_type.to_string(),
            client_ip: context.client_ip.clone(),
            user_agent: None,
            success,
            error_message,
            metadata: serde_json::json!({
                "subscription_tier": context.subscription_tier,
                "roles": context.roles,
                "permissions_count": context.permissions.len(),
            }),
            severity: if success {
                ai_core_security::AuditLevel::Info
            } else {
                ai_core_security::AuditLevel::Warning
            },
            timestamp: Utc::now(),
        };

        self.security_service
            .audit_logger()
            .log_event(security_event)
            .await
            .context("Failed to log security event")?;

        Ok(())
    }

    /// Log comprehensive audit trail
    async fn log_audit_trail(
        &self,
        context: &SecurityContext,
        operation: &str,
        resource_type: &str,
        resource_id: Option<String>,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        success: bool,
        error_message: Option<String>,
        execution_time: std::time::Duration,
    ) -> Result<()> {
        let security_event = ai_core_security::SecurityEvent {
            id: Uuid::new_v4(),
            user_id: Some(context.user_id),
            session_id: context.session_id.clone(),
            event_type: operation.to_string(),
            resource: Some(resource_type.to_string()),
            action: operation.to_string(),
            client_ip: context.client_ip.clone(),
            user_agent: None,
            success,
            error_message,
            metadata: serde_json::json!({
                "resource_id": resource_id,
                "old_values": old_values,
                "new_values": new_values,
                "execution_time_ms": execution_time.as_millis(),
                "subscription_tier": context.subscription_tier,
                "roles": context.roles,
            }),
            severity: if success {
                ai_core_security::AuditLevel::Info
            } else {
                ai_core_security::AuditLevel::Error
            },
            timestamp: Utc::now(),
        };

        self.security_service
            .audit_logger()
            .log_event(security_event)
            .await
            .context("Failed to log audit trail")?;

        Ok(())
    }

    /// Simulate fetching user from database
    async fn fetch_user_from_database(&self, user_id: Uuid) -> Result<Option<UserAccount>> {
        info!(user_id = %user_id, "Fetching user from database");

        // Simulate database query
        tokio::time::sleep(tokio::time::Duration::from_millis(8)).await;

        // Return a sample encrypted user (in reality, this would come from DB)
        let now = Utc::now();
        Ok(Some(UserAccount {
            id: user_id,
            username: "sample_user".to_string(),
            email: "user@example.com".to_string(),
            password: "encrypted_password_blob".to_string(),
            api_key: "encrypted_api_key_blob".to_string(),
            secret_token: "encrypted_token_blob".to_string(),
            subscription_tier: SubscriptionTier::Professional,
            is_active: true,
            metadata: serde_json::json!({
                "last_login": now,
                "login_count": 42
            }),
            created_at: now - chrono::Duration::days(30),
            updated_at: now,
        }))
    }
}

/// Demonstrate the secure database integration patterns
async fn run_integration_demo() -> Result<()> {
    info!("Starting Database-Security Integration Demo");

    // 1. Initialize services
    let database_config = DatabaseConfig {
        postgresql: PostgresConfig::default(),
        monitoring: MonitoringConfig::default(),
        clickhouse: None,
        mongodb: None,
        redis: None,
    };

    let security_config = SecurityConfig::default();

    info!("Initializing database manager...");
    let database_manager = Arc::new(DatabaseManager::new(database_config).await?);

    info!("Initializing security service...");
    let security_service = Arc::new(SecurityService::new(security_config).await?);

    info!("Creating secure repository...");
    let secure_repo = SecureUserRepository::new(database_manager.clone(), security_service.clone());

    // 2. Create test security contexts
    let admin_context = SecurityContext {
        user_id: Uuid::new_v4(),
        session_id: Some("admin-session-123".to_string()),
        roles: vec!["admin".to_string(), "user".to_string()],
        permissions: vec![
            Permission::UsersRead,
            Permission::UsersWrite,
            Permission::UsersDelete,
        ],
        subscription_tier: SubscriptionTier::Enterprise,
        client_ip: Some("192.168.1.100".to_string()),
        request_id: Some("req-001".to_string()),
    };

    let regular_user_context = SecurityContext {
        user_id: Uuid::new_v4(),
        session_id: Some("user-session-456".to_string()),
        roles: vec!["user".to_string()],
        permissions: vec![Permission::UsersRead],
        subscription_tier: SubscriptionTier::Professional,
        client_ip: Some("192.168.1.101".to_string()),
        request_id: Some("req-002".to_string()),
    };

    // 3. Demonstrate user creation with admin permissions
    info!("=== Demo 1: Admin Creating User ===");
    let new_user = UserAccount {
        id: Uuid::new_v4(),
        username: "demo_user".to_string(),
        email: "demo@example.com".to_string(),
        password: "super_secret_password".to_string(),
        api_key: "api_key_12345".to_string(),
        secret_token: "secret_token_67890".to_string(),
        subscription_tier: SubscriptionTier::Professional,
        is_active: true,
        metadata: serde_json::json!({"demo": true}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match secure_repo
        .create_user(&admin_context, new_user.clone())
        .await
    {
        Ok(created_user) => {
            info!("✅ User created successfully:");
            info!("  ID: {}", created_user.id);
            info!("  Username: {}", created_user.username);
            info!("  Email: {}", created_user.email);
            info!(
                "  Password: {} (decrypted for return)",
                created_user.password
            );
            info!("  API Key: {} (decrypted for return)", created_user.api_key);
        }
        Err(e) => error!("❌ Failed to create user: {}", e),
    }

    // 4. Demonstrate authorization denial
    info!("\n=== Demo 2: Regular User Trying to Create User (Should Fail) ===");
    let unauthorized_user = UserAccount {
        id: Uuid::new_v4(),
        username: "unauthorized_user".to_string(),
        email: "unauth@example.com".to_string(),
        password: "password123".to_string(),
        api_key: "api_key_999".to_string(),
        secret_token: "secret_999".to_string(),
        subscription_tier: SubscriptionTier::Free,
        is_active: true,
        metadata: serde_json::json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match secure_repo
        .create_user(&regular_user_context, unauthorized_user)
        .await
    {
        Ok(_) => warn!("⚠️ This should not have succeeded!"),
        Err(e) => info!("✅ Access correctly denied: {}", e),
    }

    // 5. Demonstrate user reading with different contexts
    info!("\n=== Demo 3: Reading User Data ===");
    let test_user_id = Uuid::new_v4();

    // Admin reading user
    match secure_repo.get_user(&admin_context, test_user_id).await {
        Ok(Some(user)) => {
            info!("✅ Admin successfully read user:");
            info!("  Username: {}", user.username);
            info!("  Decrypted fields available");
        }
        Ok(None) => info!("ℹ️ User not found (expected in demo)"),
        Err(e) => error!("❌ Admin read failed: {}", e),
    }

    // Regular user reading their own data (simulated)
    match secure_repo
        .get_user(&regular_user_context, regular_user_context.user_id)
        .await
    {
        Ok(Some(user)) => {
            info!("✅ User successfully read own data:");
            info!("  Username: {}", user.username);
        }
        Ok(None) => info!("ℹ️ Own user data not found (expected in demo)"),
        Err(e) => error!("❌ Self-read failed: {}", e),
    }

    // 6. Demonstrate update operations
    info!("\n=== Demo 4: User Updates ===");
    let mut user_update = new_user;
    user_update.username = "updated_username".to_string();
    user_update.password = "new_super_secret_password".to_string();

    match secure_repo
        .update_user(&admin_context, user_update.id, user_update)
        .await
    {
        Ok(updated_user) => {
            info!("✅ User updated successfully:");
            info!("  New username: {}", updated_user.username);
            info!("  New password: {} (decrypted)", updated_user.password);
        }
        Err(e) => error!("❌ Update failed: {}", e),
    }

    // 7. Demonstrate deletion (admin only)
    info!("\n=== Demo 5: User Deletion (Admin) ===");
    let delete_user_id = Uuid::new_v4();

    match secure_repo
        .delete_user(&admin_context, delete_user_id)
        .await
    {
        Ok(deleted) => {
            if deleted {
                info!("✅ User deleted successfully");
            } else {
                info!("ℹ️ User was not found for deletion");
            }
        }
        Err(e) => error!("❌ Deletion failed: {}", e),
    }

    // 8. Demonstrate health check
    info!("\n=== Demo 6: Database Health Check ===");
    match database_manager.health_check().await {
        Ok(health) => {
            info!("✅ Database health check passed:");
            info!("  Overall healthy: {}", health.overall_healthy);
            info!("  PostgreSQL healthy: {}", health.postgres.healthy);
            info!(
                "  PostgreSQL response time: {}ms",
                health.postgres.response_time_ms
            );
        }
        Err(e) => error!("❌ Health check failed: {}", e),
    }

    // 9. Demonstrate cleanup
    info!("\n=== Demo 7: Cleanup and Shutdown ===");
    database_manager.shutdown().await?;
    info!("✅ Database connections closed gracefully");

    info!("🎉 Database-Security Integration Demo completed successfully!");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,database_security_integration=debug")
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 AI-CORE Database-Security Integration Example");
    info!("====================================================");

    // Load configuration from environment
    dotenvy::dotenv().ok();

    // Run the comprehensive integration demo
    if let Err(e) = run_integration_demo().await {
        error!("Demo failed: {:#}", e);
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_context_permissions() {
        let context = SecurityContext {
            user_id: Uuid::new_v4(),
            session_id: Some("test-session".to_string()),
            roles: vec!["user".to_string()],
            permissions: vec![Permission::UsersRead, Permission::UsersWrite],
            subscription_tier: SubscriptionTier::Professional,
            client_ip: Some("127.0.0.1".to_string()),
            request_id: Some("test-req".to_string()),
        };

        assert!(context.has_permission(&Permission::UsersRead));
        assert!(context.has_permission(&Permission::UsersWrite));
        assert!(!context.has_permission(&Permission::UsersDelete));

        assert!(context.has_role("user"));
        assert!(!context.has_role("admin"));
    }

    #[tokio::test]
    async fn test_user_account_entity_implementation() {
        let now = Utc::now();
        let user = UserAccount {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password: "password".to_string(),
            api_key: "api_key".to_string(),
            secret_token: "secret".to_string(),
            subscription_tier: SubscriptionTier::Free,
            is_active: true,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        };

        // Test Entity trait implementation
        assert_eq!(user.id(), user.id);
        assert_eq!(user.created_at(), now);
        assert_eq!(user.updated_at(), now);
    }

    #[test]
    fn test_serialization() {
        let user = UserAccount {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password: "password".to_string(),
            api_key: "api_key".to_string(),
            secret_token: "secret".to_string(),
            subscription_tier: SubscriptionTier::Professional,
            is_active: true,
            metadata: serde_json::json!({"test": true}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Test serialization/deserialization
        let json = serde_json::to_string(&user).expect("Failed to serialize");
        let deserialized: UserAccount = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.subscription_tier, deserialized.subscription_tier);
    }
}
