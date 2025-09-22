//! # Access Control Module
//!
//! This module provides database access control functionality that integrates
//! with the security-agent's authorization services. It implements role-based
//! and attribute-based access control for database operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use ai_core_security::RbacService;
use uuid::Uuid;

use crate::{
    error::SecureDatabaseError,
    security_context::{SecurityContext, SecurityLevel},
};

/// Database access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    /// Enable strict mode (deny by default)
    pub strict_mode: bool,
    /// Cache permissions for performance
    pub enable_permission_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Enable audit logging for access control decisions
    pub audit_access_decisions: bool,
    /// Resource-specific access rules
    pub resource_rules: HashMap<String, ResourceAccessRule>,
    /// Default permissions for new resources
    pub default_permissions: Vec<String>,
}

impl Default for AccessControlConfig {
    fn default() -> Self {
        let mut resource_rules = HashMap::new();

        // Define default resource rules
        resource_rules.insert(
            "users".to_string(),
            ResourceAccessRule {
                read_permissions: vec!["user:read".to_string()],
                write_permissions: vec!["user:write".to_string()],
                delete_permissions: vec!["user:delete".to_string()],
                admin_permissions: vec!["user:admin".to_string()],
                owner_access: true,
                require_mfa_for_admin: true,
                require_elevated_for_sensitive: true,
            },
        );

        resource_rules.insert(
            "workflows".to_string(),
            ResourceAccessRule {
                read_permissions: vec!["workflow:read".to_string()],
                write_permissions: vec!["workflow:write".to_string()],
                delete_permissions: vec!["workflow:delete".to_string()],
                admin_permissions: vec!["workflow:admin".to_string()],
                owner_access: true,
                require_mfa_for_admin: false,
                require_elevated_for_sensitive: false,
            },
        );

        resource_rules.insert(
            "analytics".to_string(),
            ResourceAccessRule {
                read_permissions: vec!["analytics:read".to_string()],
                write_permissions: vec!["analytics:write".to_string()],
                delete_permissions: vec!["analytics:delete".to_string()],
                admin_permissions: vec!["analytics:admin".to_string()],
                owner_access: false,
                require_mfa_for_admin: true,
                require_elevated_for_sensitive: true,
            },
        );

        Self {
            strict_mode: true,
            enable_permission_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
            max_cache_size: 10000,
            audit_access_decisions: true,
            resource_rules,
            default_permissions: vec![],
        }
    }
}

/// Resource-specific access rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAccessRule {
    /// Permissions required for read operations
    pub read_permissions: Vec<String>,
    /// Permissions required for write operations
    pub write_permissions: Vec<String>,
    /// Permissions required for delete operations
    pub delete_permissions: Vec<String>,
    /// Permissions required for admin operations
    pub admin_permissions: Vec<String>,
    /// Whether resource owners have automatic access
    pub owner_access: bool,
    /// Whether admin operations require MFA
    pub require_mfa_for_admin: bool,
    /// Whether sensitive operations require elevated context
    pub require_elevated_for_sensitive: bool,
}

/// Permission cache entry
#[derive(Debug, Clone)]
struct PermissionCacheEntry {
    pub user_id: Uuid,
    permission: String,
    allowed: bool,
    cached_at: chrono::DateTime<chrono::Utc>,
}

/// Database access control manager
pub struct DatabaseAccessControl {
    /// Authorization service from security-agent
    authz_service: Arc<RbacService>,
    /// Access control configuration
    config: AccessControlConfig,
    /// Permission cache
    permission_cache: Arc<RwLock<HashMap<String, PermissionCacheEntry>>>,
    /// Access control metrics
    metrics: Arc<RwLock<AccessControlMetrics>>,
}

/// Access control metrics
#[derive(Debug, Default, Clone)]
pub struct AccessControlMetrics {
    pub total_checks: u64,
    pub allowed_checks: u64,
    pub denied_checks: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub mfa_required_checks: u64,
    pub elevation_required_checks: u64,
}

impl DatabaseAccessControl {
    /// Create a new database access control manager
    pub fn new(
        authz_service: Arc<RbacService>,
        config: AccessControlConfig,
    ) -> Result<Self, SecureDatabaseError> {
        info!("Initializing database access control");

        Ok(Self {
            authz_service,
            config,
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(AccessControlMetrics::default())),
        })
    }

    /// Check if the security context has permission for the given operation
    pub async fn check_permission(
        &self,
        context: &SecurityContext,
        permission: &str,
    ) -> Result<(), SecureDatabaseError> {
        let start_time = std::time::Instant::now();

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_checks += 1;
        }

        debug!(
            user_id = %context.user_id,
            permission = %permission,
            "Checking database permission"
        );

        // Check if context is valid
        if !context.is_valid() {
            warn!(
                user_id = %context.user_id,
                "Access denied: expired security context"
            );
            self.update_denied_metrics().await;
            return Err(SecureDatabaseError::AccessDenied(
                "Security context has expired".to_string(),
            ));
        }

        // Check cache first if enabled
        if self.config.enable_permission_caching {
            if let Some(cached_result) = self.get_cached_permission(context, permission).await? {
                if cached_result {
                    self.update_allowed_metrics().await;
                    return Ok(());
                } else {
                    self.update_denied_metrics().await;
                    return Err(SecureDatabaseError::AccessDenied(format!(
                        "Permission denied: {}",
                        permission
                    )));
                }
            }
        }

        // Perform permission check
        let allowed = self.perform_permission_check(context, permission).await?;

        // Cache the result if enabled
        if self.config.enable_permission_caching {
            self.cache_permission_result(context, permission, allowed)
                .await;
        }

        if allowed {
            debug!(
                user_id = %context.user_id,
                permission = %permission,
                duration_ms = start_time.elapsed().as_millis(),
                "Permission granted"
            );
            self.update_allowed_metrics().await;
            Ok(())
        } else {
            warn!(
                user_id = %context.user_id,
                permission = %permission,
                duration_ms = start_time.elapsed().as_millis(),
                "Permission denied"
            );
            self.update_denied_metrics().await;
            Err(SecureDatabaseError::AccessDenied(format!(
                "Permission denied: {}",
                permission
            )))
        }
    }

    /// Check resource-specific access
    pub async fn check_resource_access(
        &self,
        context: &SecurityContext,
        resource_type: &str,
        resource_id: &str,
        operation: &str,
    ) -> Result<(), SecureDatabaseError> {
        debug!(
            user_id = %context.user_id,
            resource_type = %resource_type,
            resource_id = %resource_id,
            operation = %operation,
            "Checking resource access"
        );

        // Get resource access rules
        let resource_rule = self.config.resource_rules.get(resource_type);

        // Determine required permissions based on operation
        let required_permissions = match operation {
            "read" | "get" | "list" => resource_rule
                .map(|r| r.read_permissions.clone())
                .unwrap_or_else(|| vec![format!("{}:read", resource_type)]),
            "create" | "update" | "write" => resource_rule
                .map(|r| r.write_permissions.clone())
                .unwrap_or_else(|| vec![format!("{}:write", resource_type)]),
            "delete" => resource_rule
                .map(|r| r.delete_permissions.clone())
                .unwrap_or_else(|| vec![format!("{}:delete", resource_type)]),
            "admin" => resource_rule
                .map(|r| r.admin_permissions.clone())
                .unwrap_or_else(|| vec![format!("{}:admin", resource_type)]),
            _ => {
                return Err(SecureDatabaseError::AccessDenied(format!(
                    "Unknown operation: {}",
                    operation
                )));
            }
        };

        // Check if any required permission is satisfied
        let mut access_granted = false;
        for permission in &required_permissions {
            if context.has_permission(permission) {
                access_granted = true;
                break;
            }
        }

        // Check owner access if enabled
        if !access_granted {
            if let Some(rule) = resource_rule {
                if rule.owner_access
                    && self
                        .is_resource_owner(context, resource_type, resource_id)
                        .await?
                {
                    debug!(
                        user_id = %context.user_id,
                        resource_type = %resource_type,
                        resource_id = %resource_id,
                        "Access granted based on ownership"
                    );
                    access_granted = true;
                }
            }
        }

        if !access_granted {
            return Err(SecureDatabaseError::AccessDenied(format!(
                "Access denied to {} {} for operation {}",
                resource_type, resource_id, operation
            )));
        }

        // Check additional requirements
        if let Some(rule) = resource_rule {
            // Check MFA requirement for admin operations
            if operation == "admin" && rule.require_mfa_for_admin {
                if !context.metadata.mfa_verified {
                    {
                        let mut metrics = self.metrics.write().await;
                        metrics.mfa_required_checks += 1;
                    }
                    return Err(SecureDatabaseError::MfaRequired(
                        "Admin operation requires MFA verification".to_string(),
                    ));
                }
            }

            // Check elevation requirement for sensitive operations
            if (operation == "delete" || operation == "admin")
                && rule.require_elevated_for_sensitive
            {
                if context.metadata.security_level == SecurityLevel::Standard {
                    {
                        let mut metrics = self.metrics.write().await;
                        metrics.elevation_required_checks += 1;
                    }
                    return Err(SecureDatabaseError::ElevationRequired(
                        "Sensitive operation requires elevated security context".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if user is owner of the resource
    async fn is_resource_owner(
        &self,
        context: &SecurityContext,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<bool, SecureDatabaseError> {
        // This would typically query the database to check ownership
        // For now, we implement basic ownership rules
        match resource_type {
            "users" => {
                // User owns their own user record
                Ok(context.user_id.to_string() == resource_id)
            }
            "workflows" => {
                // This would require querying the workflow to check creator_id
                // For now, return false (implement when database queries are available)
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Perform the actual permission check
    async fn perform_permission_check(
        &self,
        context: &SecurityContext,
        permission: &str,
    ) -> Result<bool, SecureDatabaseError> {
        // First check context permissions
        if context.has_permission(permission) {
            return Ok(true);
        }

        // Check with authorization service
        // Split permission into resource and action (format: "resource:action")
        let parts: Vec<&str> = permission.split(':').collect();
        let (resource, action) = if parts.len() >= 2 {
            (parts[0], parts[1])
        } else {
            ("unknown", permission)
        };

        match self
            .authz_service
            .check_permission(context.user_id, resource, action)
            .await
        {
            Ok(allowed) => Ok(allowed),
            Err(e) => {
                error!(
                    error = %e,
                    user_id = %context.user_id,
                    permission = %permission,
                    "Authorization service error"
                );

                if self.config.strict_mode {
                    Ok(false)
                } else {
                    // In non-strict mode, fall back to context permissions
                    Ok(context.has_permission(permission))
                }
            }
        }
    }

    /// Get cached permission result
    async fn get_cached_permission(
        &self,
        context: &SecurityContext,
        permission: &str,
    ) -> Result<Option<bool>, SecureDatabaseError> {
        let cache_key = format!("{}:{}", context.user_id, permission);
        let cache = self.permission_cache.read().await;

        if let Some(entry) = cache.get(&cache_key) {
            // Check if cache entry is still valid
            let now = chrono::Utc::now();
            let cache_age = now - entry.cached_at;

            if cache_age.num_seconds() < self.config.cache_ttl_seconds as i64 {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.cache_hits += 1;
                }
                debug!(
                    user_id = %context.user_id,
                    permission = %permission,
                    cached_result = entry.allowed,
                    "Permission cache hit"
                );
                return Ok(Some(entry.allowed));
            }
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.cache_misses += 1;
        }
        Ok(None)
    }

    /// Cache permission result
    async fn cache_permission_result(
        &self,
        context: &SecurityContext,
        permission: &str,
        allowed: bool,
    ) {
        let cache_key = format!("{}:{}", context.user_id, permission);
        let entry = PermissionCacheEntry {
            user_id: context.user_id,
            permission: permission.to_string(),
            allowed,
            cached_at: chrono::Utc::now(),
        };

        let mut cache = self.permission_cache.write().await;

        // Check cache size limit
        if cache.len() >= self.config.max_cache_size {
            // Remove oldest entries (simple LRU)
            let mut entries: Vec<(String, PermissionCacheEntry)> = cache.drain().collect();
            entries.sort_by(|a, b| a.1.cached_at.cmp(&b.1.cached_at));

            // Keep newest 80% of entries
            let keep_count = (self.config.max_cache_size as f64 * 0.8) as usize;
            let total_entries = entries.len();
            for (key, entry) in entries.into_iter().skip(total_entries - keep_count) {
                cache.insert(key, entry);
            }
        }

        cache.insert(cache_key, entry);
    }

    /// Clear permission cache for a specific user
    pub async fn clear_user_cache(&self, user_id: &Uuid) {
        let mut cache = self.permission_cache.write().await;
        let user_id_str = user_id.to_string();

        cache.retain(|key, _| !key.starts_with(&format!("{}:", user_id_str)));

        info!(user_id = %user_id, "Cleared permission cache for user");
    }

    /// Clear entire permission cache
    pub async fn clear_cache(&self) {
        let mut cache = self.permission_cache.write().await;
        cache.clear();

        info!("Cleared entire permission cache");
    }

    /// Get access control metrics
    pub async fn get_metrics(&self) -> AccessControlMetrics {
        self.metrics.read().await.clone()
    }

    /// Update allowed metrics
    async fn update_allowed_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.allowed_checks += 1;
    }

    /// Update denied metrics
    async fn update_denied_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.denied_checks += 1;
    }
}

impl Clone for DatabaseAccessControl {
    fn clone(&self) -> Self {
        Self {
            authz_service: self.authz_service.clone(),
            config: self.config.clone(),
            permission_cache: self.permission_cache.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

// Default implementation for testing
impl Default for DatabaseAccessControl {
    fn default() -> Self {
        // This is only for testing - in production, use the `new` method
        panic!("DatabaseAccessControl::default() should not be used in production - use DatabaseAccessControl::new() instead")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn create_test_context() -> SecurityContext {
        let user_id = Uuid::new_v4();
        let permissions = std::collections::HashSet::from([
            "user:read".to_string(),
            "workflow:write".to_string(),
        ]);
        let roles = vec!["user".to_string()];

        SecurityContext::new(user_id, None, permissions, roles)
    }

    #[test]
    fn test_access_control_config_default() {
        let config = AccessControlConfig::default();
        assert!(config.strict_mode);
        assert!(config.enable_permission_caching);
        assert!(config.resource_rules.contains_key("users"));
        assert!(config.resource_rules.contains_key("workflows"));
    }

    #[test]
    fn test_resource_access_rule() {
        let rule = ResourceAccessRule {
            read_permissions: vec!["test:read".to_string()],
            write_permissions: vec!["test:write".to_string()],
            delete_permissions: vec!["test:delete".to_string()],
            admin_permissions: vec!["test:admin".to_string()],
            owner_access: true,
            require_mfa_for_admin: true,
            require_elevated_for_sensitive: false,
        };

        assert_eq!(rule.read_permissions, vec!["test:read"]);
        assert!(rule.owner_access);
        assert!(rule.require_mfa_for_admin);
    }

    #[tokio::test]
    async fn test_permission_caching() {
        let config = AccessControlConfig::default();
        let authz_service = Arc::new(AuthorizationService::default());
        let access_control = DatabaseAccessControl::new(authz_service, config).unwrap();

        let context = create_test_context();
        let permission = "user:read";

        // First check - should be a cache miss
        let cache_key = format!("{}:{}", context.user_id, permission);
        assert!(access_control
            .get_cached_permission(&context, permission)
            .await
            .unwrap()
            .is_none());

        // Cache a result
        access_control
            .cache_permission_result(&context, permission, true)
            .await;

        // Second check - should be a cache hit
        let cached_result = access_control
            .get_cached_permission(&context, permission)
            .await
            .unwrap();
        assert!(cached_result.is_some());
        assert!(cached_result.unwrap());
    }

    #[tokio::test]
    async fn test_cache_clearing() {
        let config = AccessControlConfig::default();
        let authz_service = Arc::new(AuthorizationService::default());
        let access_control = DatabaseAccessControl::new(authz_service, config).unwrap();

        let context = create_test_context();
        let permission = "user:read";

        // Cache a result
        access_control
            .cache_permission_result(&context, permission, true)
            .await;

        // Verify it's cached
        assert!(access_control
            .get_cached_permission(&context, permission)
            .await
            .unwrap()
            .is_some());

        // Clear user cache
        access_control.clear_user_cache(&context.user_id).await;

        // Verify it's cleared
        assert!(access_control
            .get_cached_permission(&context, permission)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let config = AccessControlConfig::default();
        let authz_service = Arc::new(AuthorizationService::default());
        let access_control = DatabaseAccessControl::new(authz_service, config).unwrap();

        // Update metrics
        access_control.update_allowed_metrics().await;
        access_control.update_denied_metrics().await;

        let metrics = access_control.get_metrics().await;
        assert_eq!(metrics.allowed_checks, 1);
        assert_eq!(metrics.denied_checks, 1);
    }
}
