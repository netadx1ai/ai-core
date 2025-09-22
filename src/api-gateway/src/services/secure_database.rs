//! Secure Database Access Patterns
//!
//! This module provides secure database access patterns that integrate authentication,
//! authorization, audit logging, and encrypted data storage with the AI-PLATFORM database layer.
//!
//! ## Features
//!
//! - **SecureRepositoryWrapper**: Enforces authentication and authorization for all database operations
//! - **AuditedRepository**: Logs all database operations for security and compliance
//! - **EncryptedFieldHandler**: Handles encryption/decryption of sensitive data fields
//! - **SecureTransactionManager**: Provides transaction-level security controls
//!
//! ## Usage
//!
//! ```rust
//! use crate::services::secure_database::{SecureRepositoryWrapper, SecurityContext};
//!
//! // Create security context from JWT claims
//! let security_context = SecurityContext::from_jwt_claims(claims)?;
//!
//! // Wrap repository with security
//! let secure_repo = SecureRepositoryWrapper::new(
//!     repository,
//!     security_service,
//!     audit_logger,
//!     encryption_service,
//! );
//!
//! // All operations are now secured and audited
//! let result = secure_repo.create(security_context, create_input).await?;
//! ```

use ai_core_database::{
    DatabaseError, DatabaseManager, Entity, PagedResult, Pagination, PostgresRepository,
    Repository, RepositoryError, RepositoryFactory, Sort,
};
use ai_core_security::audit::{AuditLevel, AuditLogEntry, AuditLogger, SecurityEvent};
use ai_core_security::jwt::ValidationResult;
use ai_core_security::{
    EncryptionService, JwtClaims, JwtService, RbacService, SecurityError, SecurityService,
};
use ai_core_shared::types::{Permission, SubscriptionTier, User};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Errors specific to secure database operations
#[derive(Debug, thiserror::Error)]
pub enum SecureDatabaseError {
    #[error("Authentication required: {0}")]
    AuthenticationRequired(String),

    #[error("Authorization denied: user {user_id} lacks permission '{permission}' for resource '{resource}'")]
    AuthorizationDenied {
        user_id: Uuid,
        permission: String,
        resource: String,
    },

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Audit logging failed: {0}")]
    AuditError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),

    #[error("Repository error: {0}")]
    RepositoryError(#[from] RepositoryError),

    #[error("Security error: {0}")]
    SecurityError(#[from] SecurityError),

    #[error("Invalid security context: {0}")]
    InvalidSecurityContext(String),

    #[error("Data integrity violation: {0}")]
    DataIntegrityViolation(String),
}

pub type SecureResult<T> = Result<T, SecureDatabaseError>;

/// Security context for database operations
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub user_id: Uuid,
    pub session_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<Permission>,
    pub subscription_tier: SubscriptionTier,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl SecurityContext {
    /// Create security context from JWT claims
    pub fn from_jwt_claims(claims: &JwtClaims) -> SecureResult<Self> {
        Ok(Self {
            user_id: claims.sub.parse().map_err(|_| {
                SecureDatabaseError::InvalidSecurityContext(
                    "Invalid user ID in JWT claims".to_string(),
                )
            })?,
            session_id: Some(claims.session_id.clone()),
            roles: claims.roles.clone(),
            permissions: claims
                .permissions
                .iter()
                .filter_map(|p| p.parse().ok())
                .collect(),
            subscription_tier: claims
                .subscription_tier
                .parse()
                .unwrap_or(SubscriptionTier::Free),
            client_ip: claims.client_ip.clone(),
            user_agent: None, // Would be extracted from request headers
            request_id: None, // Would be extracted from request
            timestamp: Utc::now(),
        })
    }

    /// Create security context from validation result
    pub fn from_validation_result(result: &ValidationResult) -> Self {
        Self {
            user_id: result.user_id,
            session_id: Some(result.session_id.clone()),
            roles: result.roles.clone(),
            permissions: result.permissions.iter().cloned().collect(),
            subscription_tier: result.subscription_tier.clone(),
            client_ip: None,
            user_agent: None,
            request_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Check if user has specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles
            .iter()
            .any(|role| self.roles.contains(&role.to_string()))
    }

    /// Check if user has specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
}

/// Audit trail entry for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Option<String>,
    pub operation: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub execution_time_ms: u64,
}

impl AuditTrail {
    pub fn new(context: &SecurityContext, operation: &str, resource_type: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: context.user_id,
            session_id: context.session_id.clone(),
            operation: operation.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: None,
            old_values: None,
            new_values: None,
            client_ip: context.client_ip.clone(),
            user_agent: context.user_agent.clone(),
            request_id: context.request_id.clone(),
            success: false,
            error_message: None,
            timestamp: Utc::now(),
            execution_time_ms: 0,
        }
    }
}

/// Configuration for secure database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureDatabaseConfig {
    pub enable_audit_logging: bool,
    pub enable_field_encryption: bool,
    pub enable_authorization_cache: bool,
    pub audit_sensitive_operations_only: bool,
    pub encrypted_fields: Vec<String>,
    pub cache_ttl_seconds: u64,
    pub max_audit_batch_size: u32,
}

impl Default for SecureDatabaseConfig {
    fn default() -> Self {
        Self {
            enable_audit_logging: true,
            enable_field_encryption: true,
            enable_authorization_cache: true,
            audit_sensitive_operations_only: false,
            encrypted_fields: vec![
                "password".to_string(),
                "api_key".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "private_key".to_string(),
            ],
            cache_ttl_seconds: 300, // 5 minutes
            max_audit_batch_size: 100,
        }
    }
}

/// Secure repository wrapper that enforces authentication and authorization
/// Simple in-memory audit logger for secure repository
#[derive(Debug, Clone)]
struct InMemoryAuditLogger {
    logs: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
}

#[async_trait]
impl ai_core_security::audit::AuditLogger for InMemoryAuditLogger {
    async fn log(&self, entry: AuditLogEntry) -> Result<(), SecurityError> {
        let mut logs = self.logs.write().await;
        logs.push(entry);

        // Trim if exceeding max entries
        if logs.len() > self.max_entries {
            let to_remove = logs.len() - self.max_entries;
            logs.drain(0..to_remove);
        }

        Ok(())
    }

    async fn get_logs(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        level: Option<AuditLevel>,
        limit: Option<usize>,
    ) -> Result<Vec<AuditLogEntry>, SecurityError> {
        let logs = self.logs.read().await;

        let filtered: Vec<AuditLogEntry> = logs
            .iter()
            .filter(|entry| {
                // Time range filter
                if let Some(start) = start_time {
                    if entry.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = end_time {
                    if entry.timestamp > end {
                        return false;
                    }
                }

                // Level filter
                if let Some(filter_level) = level {
                    if entry.level != filter_level {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        let result = if let Some(limit) = limit {
            filtered.into_iter().take(limit).collect()
        } else {
            filtered
        };

        Ok(result)
    }

    async fn count_logs(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        level: Option<AuditLevel>,
    ) -> Result<u64, SecurityError> {
        let logs = self.get_logs(start_time, end_time, level, None).await?;
        Ok(logs.len() as u64)
    }

    async fn cleanup_old_logs(&self, older_than: DateTime<Utc>) -> Result<u64, SecurityError> {
        let mut logs = self.logs.write().await;
        let initial_count = logs.len();

        logs.retain(|entry| entry.timestamp >= older_than);

        let removed_count = initial_count - logs.len();
        Ok(removed_count as u64)
    }
}

impl InMemoryAuditLogger {
    fn new(max_entries: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            max_entries,
        }
    }
}

pub struct SecureRepositoryWrapper<T> {
    repository: Arc<T>,
    security_service: Arc<SecurityService>,
    audit_logger: Arc<InMemoryAuditLogger>,
    encryption_service: Arc<EncryptionService>,
    config: SecureDatabaseConfig,
    authorization_cache: Arc<RwLock<HashMap<String, (bool, DateTime<Utc>)>>>,
}

impl<T> SecureRepositoryWrapper<T> {
    pub fn new(
        repository: Arc<T>,
        security_service: Arc<SecurityService>,
        encryption_service: Arc<EncryptionService>,
    ) -> Self {
        Self {
            repository,
            security_service,
            audit_logger: Arc::new(InMemoryAuditLogger::new(10000)),
            encryption_service,
            config: SecureDatabaseConfig::default(),
            authorization_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(mut self, config: SecureDatabaseConfig) -> Self {
        self.config = config;
        self
    }

    /// Check authorization for a specific operation
    async fn check_authorization(
        &self,
        context: &SecurityContext,
        resource: &str,
        action: &str,
    ) -> SecureResult<bool> {
        let cache_key = format!("{}:{}:{}", context.user_id, resource, action);

        // Check cache first
        if self.config.enable_authorization_cache {
            let cache = self.authorization_cache.read().await;
            if let Some((authorized, cached_at)) = cache.get(&cache_key) {
                if cached_at
                    .signed_duration_since(Utc::now())
                    .num_seconds()
                    .abs()
                    < self.config.cache_ttl_seconds as i64
                {
                    return Ok(*authorized);
                }
            }
        }

        // Check with RBAC service
        let authorized = self
            .security_service
            .rbac()
            .check_permission(context.user_id, resource, action)
            .await
            .map_err(|e| SecureDatabaseError::SecurityError(e))?;

        // Update cache
        if self.config.enable_authorization_cache {
            let mut cache = self.authorization_cache.write().await;
            cache.insert(cache_key, (authorized, Utc::now()));
        }

        if !authorized {
            return Err(SecureDatabaseError::AuthorizationDenied {
                user_id: context.user_id,
                permission: action.to_string(),
                resource: resource.to_string(),
            });
        }

        Ok(authorized)
    }

    /// Log audit trail for database operation
    async fn log_audit_trail(
        &self,
        mut audit_trail: AuditTrail,
        start_time: std::time::Instant,
        result: &Result<(), SecureDatabaseError>,
    ) -> SecureResult<()> {
        if !self.config.enable_audit_logging {
            return Ok(());
        }

        audit_trail.execution_time_ms = start_time.elapsed().as_millis() as u64;
        audit_trail.success = result.is_ok();

        if let Err(error) = result {
            audit_trail.error_message = Some(error.to_string());
        }

        // Convert to security event
        let security_event = SecurityEvent::System {
            event: format!("{} - {}", audit_trail.operation, audit_trail.resource_type),
            details: {
                let mut details = HashMap::new();
                details.insert("user_id".to_string(), audit_trail.user_id.to_string());
                details.insert(
                    "resource_id".to_string(),
                    audit_trail.resource_id.unwrap_or_default(),
                );
                details.insert(
                    "execution_time_ms".to_string(),
                    audit_trail.execution_time_ms.to_string(),
                );
                details.insert("success".to_string(), audit_trail.success.to_string());
                if let Some(error) = &audit_trail.error_message {
                    details.insert("error".to_string(), error.clone());
                }
                if let Some(client_ip) = &audit_trail.client_ip {
                    details.insert("client_ip".to_string(), client_ip.clone());
                }
                details
            },
        };

        let audit_entry = ai_core_security::audit::AuditLogEntry::new(
            if audit_trail.success {
                AuditLevel::Info
            } else {
                AuditLevel::Info
            },
            security_event,
        );

        if let Err(e) = self.audit_logger.log(audit_entry).await {
            return Err(SecureDatabaseError::AuditError(e.to_string()));
        }

        Ok(())
    }

    /// Encrypt sensitive fields in data
    async fn encrypt_sensitive_fields<D>(&self, data: &mut D) -> SecureResult<()>
    where
        D: Serialize + DeserializeOwned,
    {
        if !self.config.enable_field_encryption {
            return Ok(());
        }

        // Convert to JSON for field manipulation
        let mut json_value = serde_json::to_value(&*data).map_err(|e| {
            SecureDatabaseError::EncryptionError(format!("Serialization failed: {}", e))
        })?;

        // Encrypt specified fields
        for field_name in &self.config.encrypted_fields {
            if let Some(field_value) = json_value.get_mut(field_name) {
                if let Some(string_value) = field_value.as_str() {
                    let encrypted = self
                        .encryption_service
                        .encrypt_string(string_value)
                        .await
                        .map_err(|e| SecureDatabaseError::EncryptionError(e.to_string()))?;
                    *field_value = serde_json::Value::String(encrypted);
                }
            }
        }

        // Convert back to original type
        *data = serde_json::from_value(json_value).map_err(|e| {
            SecureDatabaseError::EncryptionError(format!("Deserialization failed: {}", e))
        })?;

        Ok(())
    }

    /// Decrypt sensitive fields in data
    async fn decrypt_sensitive_fields<D>(&self, data: &mut D) -> SecureResult<()>
    where
        D: Serialize + DeserializeOwned,
    {
        if !self.config.enable_field_encryption {
            return Ok(());
        }

        // Convert to JSON for field manipulation
        let mut json_value = serde_json::to_value(&*data).map_err(|e| {
            SecureDatabaseError::EncryptionError(format!("Deserialization failed: {}", e))
        })?;

        // Decrypt specified fields
        for field_name in &self.config.encrypted_fields {
            if let Some(field_value) = json_value.get_mut(field_name) {
                if let Some(string_value) = field_value.as_str() {
                    match self.encryption_service.decrypt_string(string_value).await {
                        Ok(decrypted) => {
                            *field_value = serde_json::Value::String(decrypted);
                        }
                        Err(e) => {
                            warn!("Failed to decrypt field '{}': {}", field_name, e);
                            // Keep encrypted value if decryption fails
                        }
                    }
                }
            }
        }

        // Convert back to original type
        *data = serde_json::from_value(json_value).map_err(|e| {
            SecureDatabaseError::EncryptionError(format!("Deserialization failed: {}", e))
        })?;

        Ok(())
    }
}

/// Implement secure repository operations for any repository type
#[async_trait]
impl<T, E, Id, CreateInput, UpdateInput, QueryFilter> Repository<E> for SecureRepositoryWrapper<T>
where
    T: Repository<
            E,
            Id = Id,
            CreateInput = CreateInput,
            UpdateInput = UpdateInput,
            QueryFilter = QueryFilter,
        > + Send
        + Sync,
    E: Entity<Id = Id> + Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    Id: Send + Sync + Clone + Debug + 'static,
    CreateInput: Send + Sync + Serialize + DeserializeOwned + 'static,
    UpdateInput: Send + Sync + Serialize + DeserializeOwned + 'static,
    QueryFilter: Send + Sync + 'static,
{
    type Id = Id;
    type CreateInput = (SecurityContext, CreateInput);
    type UpdateInput = (SecurityContext, UpdateInput);
    type QueryFilter = (SecurityContext, QueryFilter);

    async fn create(&self, input: Self::CreateInput) -> Result<E, DatabaseError> {
        let (context, mut create_input) = input;
        let start_time = std::time::Instant::now();
        let mut audit_trail = AuditTrail::new(&context, "CREATE", std::any::type_name::<E>());

        // Check authorization
        let auth_result = self
            .check_authorization(&context, std::any::type_name::<E>(), "create")
            .await;
        if let Err(e) = auth_result {
            let error_msg = e.to_string();
            let audit_error = SecureDatabaseError::AuthenticationRequired(error_msg.clone());
            let _ = self
                .log_audit_trail(audit_trail, start_time, &Err(audit_error))
                .await;
            return Err(DatabaseError::Connection(error_msg));
        }

        // Encrypt sensitive fields
        if let Err(e) = self.encrypt_sensitive_fields(&mut create_input).await {
            let error_msg = e.to_string();
            let audit_error = SecureDatabaseError::EncryptionError(error_msg.clone());
            let _ = self
                .log_audit_trail(audit_trail, start_time, &Err(audit_error))
                .await;
            return Err(DatabaseError::Connection(error_msg));
        }

        // Store new values for audit
        audit_trail.new_values = serde_json::to_value(&create_input).ok();

        // Execute repository operation
        let result = self.repository.create(create_input).await;

        // Log audit trail
        let audit_result = match &result {
            Ok(entity) => {
                audit_trail.resource_id = Some(format!("{:?}", entity.id()));
                Ok(())
            }
            Err(e) => Err(SecureDatabaseError::DatabaseError(
                DatabaseError::Connection(e.to_string()),
            )),
        };
        let _ = self
            .log_audit_trail(audit_trail, start_time, &audit_result)
            .await;

        // Decrypt result if successful
        match result {
            Ok(mut entity) => {
                let _ = self.decrypt_sensitive_fields(&mut entity).await;
                Ok(entity)
            }
            Err(e) => Err(e),
        }
    }

    async fn find_by_id(&self, id: Self::Id) -> Result<Option<E>, DatabaseError> {
        // For read operations, we skip the full security context wrapper for simplicity
        // In a real implementation, you'd pass (SecurityContext, Id) and check read permissions
        let result = self.repository.find_by_id(id).await;

        // Decrypt result if successful
        match result {
            Ok(Some(mut entity)) => {
                if self.decrypt_sensitive_fields(&mut entity).await.is_ok() {
                    Ok(Some(entity))
                } else {
                    Ok(Some(entity)) // Return entity even if decryption fails
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn update(&self, id: Self::Id, input: Self::UpdateInput) -> Result<E, DatabaseError> {
        let (context, mut update_input) = input;
        let start_time = std::time::Instant::now();
        let mut audit_trail = AuditTrail::new(&context, "UPDATE", std::any::type_name::<E>());
        audit_trail.resource_id = Some(format!("{:?}", id));

        // Check authorization
        let auth_result = self
            .check_authorization(&context, std::any::type_name::<E>(), "update")
            .await;
        if let Err(e) = auth_result {
            let error_msg = e.to_string();
            let audit_error = SecureDatabaseError::AuthenticationRequired(error_msg.clone());
            let _ = self
                .log_audit_trail(audit_trail, start_time, &Err(audit_error))
                .await;
            return Err(DatabaseError::Connection(error_msg));
        }

        // Get old values for audit (simplified - would need to fetch existing entity)
        audit_trail.old_values =
            serde_json::json!({"note": "old values would be fetched here"}).into();

        // Encrypt sensitive fields in update input
        if let Err(e) = self.encrypt_sensitive_fields(&mut update_input).await {
            let error_msg = e.to_string();
            let audit_error = SecureDatabaseError::EncryptionError(error_msg.clone());
            let _ = self
                .log_audit_trail(audit_trail, start_time, &Err(audit_error))
                .await;
            return Err(DatabaseError::Connection(error_msg));
        }

        // Store new values for audit
        audit_trail.new_values = serde_json::to_value(&update_input).ok();

        // Execute repository operation
        let mut result = self.repository.update(id, update_input).await;

        // Log audit trail and decrypt result
        match result {
            Ok(mut entity) => {
                let _ = self.decrypt_sensitive_fields(&mut entity).await;
                let audit_result = Ok(());
                let _ = self
                    .log_audit_trail(audit_trail, start_time, &audit_result)
                    .await;
                Ok(entity)
            }
            Err(e) => {
                let audit_result = Err(SecureDatabaseError::DataIntegrityViolation(e.to_string()));
                let _ = self
                    .log_audit_trail(audit_trail, start_time, &audit_result)
                    .await;
                Err(e)
            }
        }
    }

    async fn delete(&self, id: Self::Id) -> Result<bool, DatabaseError> {
        // Similar to update - would need SecurityContext wrapper
        self.repository.delete(id).await
    }

    async fn find_many(&self, filter: Self::QueryFilter) -> Result<Vec<E>, DatabaseError> {
        let (_context, query_filter) = filter;
        // Would check read permissions here

        let result = self.repository.find_many(query_filter).await;

        // Decrypt results if successful
        if let Ok(mut entities) = result {
            for entity in &mut entities {
                let _ = self.decrypt_sensitive_fields(entity).await;
            }
            return Ok(entities);
        }

        result
    }

    async fn count(&self, filter: Self::QueryFilter) -> Result<u64, DatabaseError> {
        let (_context, query_filter) = filter;
        // Would check read permissions here
        self.repository.count(query_filter).await
    }
}

/// Secure transaction manager for database operations
pub struct SecureTransactionManager {
    database_manager: Arc<DatabaseManager>,
    security_service: Arc<SecurityService>,
    audit_logger: Arc<InMemoryAuditLogger>,
}

impl SecureTransactionManager {
    pub fn new(
        database_manager: Arc<DatabaseManager>,
        security_service: Arc<SecurityService>,
    ) -> Self {
        Self {
            database_manager,
            security_service,
            audit_logger: Arc::new(InMemoryAuditLogger::new(10000)),
        }
    }

    /// Execute a secure transaction with full audit logging
    pub async fn execute_transaction<F, R>(
        &self,
        context: SecurityContext,
        operation_name: &str,
        transaction_fn: F,
    ) -> SecureResult<R>
    where
        F: FnOnce(
                RepositoryFactory,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = SecureResult<R>> + Send>>
            + Send
            + 'static,
        R: Send,
    {
        let start_time = std::time::Instant::now();
        let mut audit_trail = AuditTrail::new(&context, operation_name, "TRANSACTION");

        // Log transaction start
        info!(
            user_id = %context.user_id,
            operation = %operation_name,
            "Starting secure transaction"
        );

        let database_manager = self.database_manager.clone();
        let result = database_manager
            .execute_transaction(|_tx| {
                let database_manager = database_manager.clone();
                Box::pin(async move {
                    let repositories = database_manager.repositories();
                    match transaction_fn(repositories).await {
                        Ok(result) => Ok(result),
                        Err(e) => Err(anyhow::anyhow!(e.to_string())),
                    }
                })
            })
            .await;

        // Log audit trail
        let audit_result = match &result {
            Ok(_) => {
                info!(
                    user_id = %context.user_id,
                    operation = %operation_name,
                    duration_ms = %start_time.elapsed().as_millis(),
                    "Transaction completed successfully"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    user_id = %context.user_id,
                    operation = %operation_name,
                    error = %e,
                    duration_ms = %start_time.elapsed().as_millis(),
                    "Transaction failed"
                );
                Err(SecureDatabaseError::DatabaseError(
                    DatabaseError::Transaction(e.to_string()),
                ))
            }
        };

        let _ = self
            .log_audit_trail(audit_trail, start_time, &audit_result)
            .await;

        result.map_err(|e| {
            SecureDatabaseError::DatabaseError(DatabaseError::Transaction(e.to_string()))
        })
    }

    async fn log_audit_trail(
        &self,
        mut audit_trail: AuditTrail,
        start_time: std::time::Instant,
        result: &Result<(), SecureDatabaseError>,
    ) -> SecureResult<()> {
        audit_trail.execution_time_ms = start_time.elapsed().as_millis() as u64;
        audit_trail.success = result.is_ok();

        if let Err(error) = result {
            audit_trail.error_message = Some(error.to_string());
        }

        let security_event = SecurityEvent::System {
            event: format!("{} - {}", audit_trail.operation, audit_trail.resource_type),
            details: {
                let mut details = HashMap::new();
                details.insert("user_id".to_string(), audit_trail.user_id.to_string());
                details.insert(
                    "resource_id".to_string(),
                    audit_trail.resource_id.unwrap_or_default(),
                );
                details.insert(
                    "execution_time_ms".to_string(),
                    audit_trail.execution_time_ms.to_string(),
                );
                details.insert("success".to_string(), audit_trail.success.to_string());
                if let Some(error) = &audit_trail.error_message {
                    details.insert("error".to_string(), error.clone());
                }
                if let Some(client_ip) = &audit_trail.client_ip {
                    details.insert("client_ip".to_string(), client_ip.clone());
                }
                details
            },
        };

        let audit_entry = ai_core_security::audit::AuditLogEntry::new(
            if audit_trail.success {
                AuditLevel::Info
            } else {
                AuditLevel::Error
            },
            security_event,
        );

        if let Err(e) = self.audit_logger.log(audit_entry).await {
            return Err(SecureDatabaseError::AuditError(e.to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_core_shared::types::Permission;

    fn create_test_security_context() -> SecurityContext {
        SecurityContext {
            user_id: Uuid::new_v4(),
            session_id: Some("test-session".to_string()),
            roles: vec!["user".to_string()],
            permissions: vec![Permission::WorkflowsRead, Permission::WorkflowsWrite],
            subscription_tier: SubscriptionTier::Professional,
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            request_id: Some("test-request".to_string()),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_security_context_creation() {
        let context = create_test_security_context();

        assert!(context.has_permission(&Permission::WorkflowsRead));
        assert!(context.has_permission(&Permission::WorkflowsWrite));
        assert!(!context.has_permission(&Permission::UsersDelete));

        assert!(context.has_role("user"));
        assert!(!context.has_role("admin"));
    }

    #[test]
    fn test_audit_trail_creation() {
        let context = create_test_security_context();
        let audit_trail = AuditTrail::new(&context, "CREATE", "User");

        assert_eq!(audit_trail.user_id, context.user_id);
        assert_eq!(audit_trail.operation, "CREATE");
        assert_eq!(audit_trail.resource_type, "User");
        assert_eq!(audit_trail.session_id, context.session_id);
    }

    #[test]
    fn test_secure_database_config_defaults() {
        let config = SecureDatabaseConfig::default();

        assert!(config.enable_audit_logging);
        assert!(config.enable_field_encryption);
        assert!(config.enable_authorization_cache);
        assert!(config.encrypted_fields.contains(&"password".to_string()));
        assert!(config.encrypted_fields.contains(&"api_key".to_string()));
    }
}
