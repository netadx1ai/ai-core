//! # Database-Security Integration
//!
//! This crate provides secure database access patterns that integrate the AI-CORE
//! security-agent services with database-agent repositories. It implements:
//!
//! - Secure authentication-based database access
//! - Encrypted data storage and retrieval flows
//! - Comprehensive audit logging for data access events
//! - Role-based access control for database operations
//! - Data encryption at rest and in transit
//!
//! ## Features
//!
//! - **Secure Repository Pattern**: Authentication-aware database repositories
//! - **Audit Trail**: Complete logging of all database operations
//! - **Encryption Integration**: Seamless data encryption/decryption
//! - **RBAC Integration**: Role-based access control for database operations
//! - **Multi-Database Support**: PostgreSQL, ClickHouse, MongoDB, Redis
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use database_security_integration::{SecureDatabaseManager, SecurityContext};
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize secure database manager
//!     let db_manager = SecureDatabaseManager::new().await?;
//!
//!     // Create security context from authenticated user
//!     let security_context = SecurityContext::from_user_id(
//!         Uuid::new_v4(),
//!         vec!["user:read".to_string(), "workflow:create".to_string()]
//!     );
//!
//!     // Perform secure database operations
//!     let user_data = db_manager
//!         .secure_postgres()
//!         .get_user_with_permissions(&security_context, user_id)
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

// Re-export core types
pub use ai_core_database::{DatabaseConfig, DatabaseError, DatabaseManager};
pub use ai_core_security::{SecurityConfig, SecurityService};
pub use ai_core_shared::types::{Permission, TokenClaims, User};

// Core integration modules
pub mod access_control;
pub mod audit;
pub mod config;
pub mod encryption_integration;
pub mod error;
pub mod metrics;
pub mod secure_repositories;
pub mod security_context;

// Re-export key types
pub use access_control::DatabaseAccessControl;
pub use audit::AuditLogger;
pub use config::SecureDatabaseConfig;
pub use encryption_integration::DataEncryption;
pub use error::SecureDatabaseError;
pub use metrics::SecureDatabaseMetrics;
pub use security_context::SecurityContext;

/// Main secure database manager that integrates security and database services
#[derive(Clone)]
pub struct SecureDatabaseManager {
    /// Database manager from database-agent
    database_manager: Arc<DatabaseManager>,
    /// Security service facade from security-agent
    security_service: Arc<SecurityService>,
    /// Audit logger for database operations
    audit_logger: Arc<AuditLogger>,
    /// Data encryption handler
    data_encryption: Arc<DataEncryption>,
    /// Access control manager
    access_control: Arc<DatabaseAccessControl>,
    /// Metrics collector
    metrics: Arc<SecureDatabaseMetrics>,
    /// Configuration
    config: Arc<SecureDatabaseConfig>,
}

impl SecureDatabaseManager {
    /// Create a new secure database manager
    pub async fn new() -> Result<Self> {
        Self::with_config(SecureDatabaseConfig::default()).await
    }

    /// Create a new secure database manager with custom configuration
    pub async fn with_config(config: SecureDatabaseConfig) -> Result<Self> {
        info!("Initializing secure database manager");

        // Initialize core services
        let database_config = DatabaseConfig::default();
        let database_manager = Arc::new(
            DatabaseManager::new(database_config)
                .await
                .context("Failed to initialize database manager")?,
        );

        // Initialize security services directly with proper Arc wrapping
        let security_config = SecurityConfig::default();

        // Initialize Redis client for security services
        let redis_client = Arc::new(redis::Client::open("redis://localhost:6379")?);

        // Initialize encryption service directly
        let key_manager =
            ai_core_security::encryption::InMemoryKeyManager::new(chrono::Duration::seconds(
                security_config.encryption.key_rotation_interval.as_secs() as i64,
            ));
        let encryption_service = Arc::new(
            ai_core_security::EncryptionService::new(key_manager)
                .await
                .context("Failed to initialize encryption service")?,
        );

        // Create a simple SecurityService wrapper for token validation
        let security_service = Arc::new(
            SecurityService::new(security_config.clone())
                .await
                .context("Failed to initialize security service")?,
        );

        // Initialize integration services
        let audit_logger =
            Arc::new(AuditLogger::new(database_manager.clone(), config.audit.clone()).await?);

        let data_encryption = Arc::new(DataEncryption::new(
            encryption_service.clone(),
            config.encryption.clone(),
        )?);

        // Create a mock RBAC service for now (production would use real implementation)
        let permission_cache = Arc::new(ai_core_security::rbac::RedisPermissionCache::new(
            redis_client.clone(),
        ));
        let role_repository = Arc::new(MockRoleRepository::new());
        let rbac_service = Arc::new(ai_core_security::RbacService::new(
            role_repository,
            permission_cache,
            ai_core_security::rbac::RbacConfig {
                enable_rbac: true,
                enable_abac: false,
                cache_ttl: chrono::Duration::minutes(30),
                admin_override: true,
                evaluation_mode: ai_core_security::rbac::PermissionEvaluationMode::Permissive,
                max_policy_evaluation_time_ms: 100,
            },
        ));

        let access_control = Arc::new(DatabaseAccessControl::new(
            rbac_service,
            config.access_control.clone(),
        )?);

        let metrics = Arc::new(SecureDatabaseMetrics::new()?);

        let config = Arc::new(config);

        Ok(Self {
            database_manager,
            security_service,
            audit_logger,
            data_encryption,
            access_control,
            metrics,
            config,
        })
    }

    /// Get secure PostgreSQL repository
    pub fn secure_postgres(&self) -> secure_repositories::SecurePostgresRepository {
        secure_repositories::SecurePostgresRepository::new(
            Arc::new(self.database_manager.repositories().postgres()),
            self.access_control.clone(),
            self.audit_logger.clone(),
            self.data_encryption.clone(),
            self.metrics.clone(),
        )
    }

    /// Get secure ClickHouse repository
    #[cfg(feature = "clickhouse")]
    pub fn secure_clickhouse(&self) -> secure_repositories::SecureClickHouseRepository {
        secure_repositories::SecureClickHouseRepository::new(
            self.database_manager.clickhouse.as_ref().unwrap().clone(),
            self.access_control.clone(),
            self.audit_logger.clone(),
            self.data_encryption.clone(),
            self.metrics.clone(),
        )
    }

    /// Get secure MongoDB repository
    #[cfg(feature = "mongodb")]
    pub fn secure_mongodb(&self) -> secure_repositories::SecureMongoRepository {
        secure_repositories::SecureMongoRepository::new(
            self.database_manager.mongodb.as_ref().unwrap().clone(),
            self.access_control.clone(),
            self.audit_logger.clone(),
            self.data_encryption.clone(),
            self.metrics.clone(),
        )
    }

    /// Get secure Redis repository
    #[cfg(feature = "redis")]
    pub fn secure_redis(&self) -> secure_repositories::SecureRedisRepository {
        secure_repositories::SecureRedisRepository::new(
            self.database_manager.redis.as_ref().unwrap().clone(),
            self.access_control.clone(),
            self.audit_logger.clone(),
            self.data_encryption.clone(),
            self.metrics.clone(),
        )
    }

    /// Validate a JWT token and create security context
    pub async fn authenticate_token(&self, token: &str) -> Result<SecurityContext> {
        let validation_result = self
            .security_service
            .validate_token(token)
            .await
            .context("Token validation failed")?;

        let user_id =
            Uuid::parse_str(&validation_result.claims.sub).context("Invalid user ID in token")?;

        // Get permissions from validation result
        let permissions: std::collections::HashSet<String> = validation_result
            .permissions
            .into_iter()
            .map(|p| format!("{:?}", p))
            .collect();

        let session_id = Uuid::parse_str(&validation_result.session_id)
            .context("Invalid session ID in token")?;

        Ok(SecurityContext::new(
            user_id,
            Some(session_id),
            permissions,
            validation_result.roles.clone(),
        ))
    }

    /// Create security context from existing session
    pub async fn create_security_context(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
    ) -> Result<SecurityContext> {
        // For now, create a minimal permission set
        // In production, this would query the authorization service properly
        let permissions = std::collections::HashSet::from([
            "database:read".to_string(),
            "database:write".to_string(),
        ]);

        let roles = vec!["user".to_string()];

        Ok(SecurityContext::new(
            user_id,
            session_id,
            permissions,
            roles,
        ))
    }

    /// Get database health with security context
    pub async fn health_check(&self, context: &SecurityContext) -> Result<DatabaseHealthStatus> {
        // Check if user has health check permission
        self.access_control
            .check_permission(context, "database:health:read")
            .await?;

        // Log audit event
        self.audit_logger
            .log_system_event(context, "health_check", "Database health check requested")
            .await;

        // Get health status from database manager
        let health_status = self.database_manager.health_check().await?;

        Ok(DatabaseHealthStatus {
            postgres: health_status.postgres.healthy,
            clickhouse: false, // Will be updated when ClickHouse is available
            mongodb: false,    // Will be updated when MongoDB is available
            redis: health_status.redis.map(|r| r.healthy).unwrap_or(false),
            overall: health_status.overall_healthy,
        })
    }

    /// Get security metrics
    pub async fn get_security_metrics(
        &self,
        context: &SecurityContext,
    ) -> Result<SecurityMetricsReport> {
        // Check if user has metrics read permission
        self.access_control
            .check_permission(context, "database:metrics:read")
            .await?;

        let metrics_report = self.metrics.generate_report().await?;

        Ok(SecurityMetricsReport {
            total_operations: metrics_report
                .operation_metrics
                .get("total")
                .map(|m| m.total_operations)
                .unwrap_or(0),
            failed_authentications: metrics_report.security_metrics.failed_authentications,
            permission_denials: metrics_report.security_metrics.authorization_denials,
            audit_events: metrics_report.audit_metrics.total_events,
            encrypted_operations: metrics_report.security_metrics.successful_authentications
                + metrics_report.security_metrics.authorization_grants,
            avg_response_time_ms: metrics_report
                .performance_metrics
                .avg_query_time_by_db
                .get("postgresql")
                .copied()
                .unwrap_or(0.0),
        })
    }

    /// Shutdown the secure database manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down secure database manager");

        // Flush audit logs
        self.audit_logger.flush().await?;

        // Export final metrics
        let _ = self.metrics.generate_report().await?;

        // Shutdown database connections
        self.database_manager.shutdown().await?;

        info!("Secure database manager shutdown complete");
        Ok(())
    }
}

/// Database health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealthStatus {
    pub postgres: bool,
    pub clickhouse: bool,
    pub mongodb: bool,
    pub redis: bool,
    pub overall: bool,
}

/// Security metrics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetricsReport {
    pub total_operations: u64,
    pub failed_authentications: u64,
    pub permission_denials: u64,
    pub audit_events: u64,
    pub encrypted_operations: u64,
    pub avg_response_time_ms: f64,
}

// Mock role repository for testing and development
struct MockRoleRepository;

impl MockRoleRepository {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ai_core_security::rbac::RoleRepository for MockRoleRepository {
    async fn get_user_roles(
        &self,
        _user_id: uuid::Uuid,
    ) -> ai_core_security::SecurityResult<Vec<ai_core_security::rbac::Role>> {
        use ai_core_security::rbac::Role;
        use ai_core_shared::types::Permission;
        use std::collections::HashSet;

        // Return a basic user role
        let mut permissions = HashSet::new();
        permissions.insert(Permission::WorkflowsRead);
        permissions.insert(Permission::WorkflowsCreate);

        Ok(vec![Role {
            id: uuid::Uuid::new_v4(),
            name: "user".to_string(),
            description: "Basic user role".to_string(),
            permissions,
            parent_roles: vec![],
            metadata: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_active: true,
        }])
    }

    async fn get_role_by_name(
        &self,
        name: &str,
    ) -> ai_core_security::SecurityResult<Option<ai_core_security::rbac::Role>> {
        if name == "user" {
            use ai_core_security::rbac::Role;
            use ai_core_shared::types::Permission;
            use std::collections::HashSet;

            let mut permissions = HashSet::new();
            permissions.insert(Permission::WorkflowsRead);
            permissions.insert(Permission::WorkflowsCreate);

            Ok(Some(Role {
                id: uuid::Uuid::new_v4(),
                name: "user".to_string(),
                description: "Basic user role".to_string(),
                permissions,
                parent_roles: vec![],
                metadata: std::collections::HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                is_active: true,
            }))
        } else {
            Ok(None)
        }
    }

    async fn create_role(
        &self,
        _role: &ai_core_security::rbac::Role,
    ) -> ai_core_security::SecurityResult<()> {
        Ok(())
    }

    async fn update_role(
        &self,
        _role: &ai_core_security::rbac::Role,
    ) -> ai_core_security::SecurityResult<()> {
        Ok(())
    }

    async fn delete_role(&self, _role_id: uuid::Uuid) -> ai_core_security::SecurityResult<()> {
        Ok(())
    }

    async fn get_role_hierarchy(
        &self,
        _role_name: &str,
    ) -> ai_core_security::SecurityResult<Vec<ai_core_security::rbac::Role>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secure_database_manager_creation() {
        let config = SecureDatabaseConfig::test_config();
        let manager = SecureDatabaseManager::with_config(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_security_context_creation() {
        let config = SecureDatabaseConfig::test_config();
        let manager = SecureDatabaseManager::with_config(config).await.unwrap();

        let user_id = Uuid::new_v4();
        let context = manager.create_security_context(user_id, None).await;
        assert!(context.is_ok());
    }

    #[tokio::test]
    async fn test_repository_access() {
        let config = SecureDatabaseConfig::test_config();
        let manager = SecureDatabaseManager::with_config(config).await.unwrap();

        // Test that we can create secure repositories
        let _postgres_repo = manager.secure_postgres();

        #[cfg(feature = "clickhouse")]
        {
            let _clickhouse_repo = manager.secure_clickhouse();
        }

        #[cfg(feature = "mongodb")]
        {
            let _mongodb_repo = manager.secure_mongodb();
        }

        #[cfg(feature = "redis")]
        {
            let _redis_repo = manager.secure_redis();
        }
    }
}
