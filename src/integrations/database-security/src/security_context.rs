//! # Security Context Module
//!
//! This module provides the security context management for database operations.
//! It handles user authentication state, permissions, roles, and session management
//! for secure database access patterns.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Security context for database operations
///
/// Contains all necessary information to perform secure database operations
/// including user identity, session information, permissions, and roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// User ID performing the operation
    pub user_id: Uuid,
    /// Session ID (if applicable)
    pub session_id: Option<Uuid>,
    /// User permissions
    pub permissions: HashSet<String>,
    /// User roles
    pub roles: Vec<String>,
    /// Context creation timestamp
    pub created_at: DateTime<Utc>,
    /// Context expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Additional context metadata
    pub metadata: SecurityContextMetadata,
}

/// Additional metadata for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContextMetadata {
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Organization/tenant ID
    pub organization_id: Option<Uuid>,
    /// Department or team ID
    pub department_id: Option<String>,
    /// Security level (e.g., "standard", "elevated", "administrative")
    pub security_level: SecurityLevel,
    /// Whether this is an API key authentication
    pub is_api_key: bool,
    /// Multi-factor authentication status
    pub mfa_verified: bool,
}

/// Security level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Standard user access
    Standard,
    /// Elevated privileges (requires recent authentication)
    Elevated,
    /// Administrative access (requires MFA)
    Administrative,
    /// System-level access (service accounts)
    System,
}

impl Default for SecurityContextMetadata {
    fn default() -> Self {
        Self {
            client_ip: None,
            user_agent: None,
            request_id: None,
            organization_id: None,
            department_id: None,
            security_level: SecurityLevel::Standard,
            is_api_key: false,
            mfa_verified: false,
        }
    }
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(
        user_id: Uuid,
        session_id: Option<Uuid>,
        permissions: HashSet<String>,
        roles: Vec<String>,
    ) -> Self {
        Self {
            user_id,
            session_id,
            permissions,
            roles,
            created_at: Utc::now(),
            expires_at: None,
            metadata: SecurityContextMetadata::default(),
        }
    }

    /// Create a security context with expiration
    pub fn with_expiration(
        user_id: Uuid,
        session_id: Option<Uuid>,
        permissions: HashSet<String>,
        roles: Vec<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            session_id,
            permissions,
            roles,
            created_at: Utc::now(),
            expires_at: Some(expires_at),
            metadata: SecurityContextMetadata::default(),
        }
    }

    /// Create a security context with metadata
    pub fn with_metadata(
        user_id: Uuid,
        session_id: Option<Uuid>,
        permissions: HashSet<String>,
        roles: Vec<String>,
        metadata: SecurityContextMetadata,
    ) -> Self {
        Self {
            user_id,
            session_id,
            permissions,
            roles,
            created_at: Utc::now(),
            expires_at: None,
            metadata,
        }
    }

    /// Create a system-level security context (for service accounts)
    pub fn system_context(service_name: &str, permissions: HashSet<String>) -> Self {
        let user_id = Uuid::new_v4(); // Generate service account ID
        let metadata = SecurityContextMetadata {
            security_level: SecurityLevel::System,
            request_id: Some(format!("system-{}", Uuid::new_v4())),
            ..Default::default()
        };

        Self {
            user_id,
            session_id: None,
            permissions,
            roles: vec!["system".to_string(), service_name.to_string()],
            created_at: Utc::now(),
            expires_at: None,
            metadata,
        }
    }

    /// Check if the security context has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission) ||
        self.permissions.contains("*") || // Wildcard permission
        self.has_role("admin") || // Admin role has all permissions
        self.metadata.security_level == SecurityLevel::System
    }

    /// Check if the security context has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string()) || self.roles.contains(&"admin".to_string())
        // Admin role includes all roles
    }

    /// Check if the security context has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|perm| self.has_permission(perm))
    }

    /// Check if the security context has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|perm| self.has_permission(perm))
    }

    /// Check if the security context has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if the security context is valid (not expired)
    pub fn is_valid(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => Utc::now() < expires_at,
            None => true, // No expiration means always valid
        }
    }

    /// Check if the security context is expired
    pub fn is_expired(&self) -> bool {
        !self.is_valid()
    }

    /// Get the remaining time until expiration
    pub fn time_until_expiration(&self) -> Option<chrono::Duration> {
        self.expires_at.map(|expires_at| expires_at - Utc::now())
    }

    /// Check if the context requires MFA for the given operation
    pub fn requires_mfa(&self, operation: &str) -> bool {
        // Administrative operations always require MFA
        if self.metadata.security_level == SecurityLevel::Administrative {
            return true;
        }

        // Specific operations that require MFA
        matches!(
            operation,
            "user:delete" | "user:admin" | "database:admin" | "security:admin" | "billing:admin"
        )
    }

    /// Validate that MFA is satisfied for the given operation
    pub fn validate_mfa_for_operation(&self, operation: &str) -> Result<()> {
        if self.requires_mfa(operation) && !self.metadata.mfa_verified {
            return Err(anyhow::anyhow!(
                "Operation '{}' requires MFA verification",
                operation
            ));
        }
        Ok(())
    }

    /// Create an audit context for logging
    pub fn audit_context(&self) -> AuditContext {
        AuditContext {
            user_id: self.user_id,
            session_id: self.session_id,
            organization_id: self.metadata.organization_id,
            client_ip: self.metadata.client_ip.clone(),
            user_agent: self.metadata.user_agent.clone(),
            request_id: self.metadata.request_id.clone(),
            security_level: self.metadata.security_level.clone(),
            is_api_key: self.metadata.is_api_key,
            mfa_verified: self.metadata.mfa_verified,
        }
    }

    /// Elevate the security context (requires recent authentication)
    pub fn elevate(&mut self) -> Result<()> {
        // Check if elevation is possible
        if self.metadata.security_level == SecurityLevel::System {
            return Err(anyhow::anyhow!("System contexts cannot be elevated"));
        }

        // For now, simple elevation (in production this would require
        // recent authentication verification)
        self.metadata.security_level = match self.metadata.security_level {
            SecurityLevel::Standard => SecurityLevel::Elevated,
            SecurityLevel::Elevated => SecurityLevel::Administrative,
            SecurityLevel::Administrative => SecurityLevel::Administrative,
            SecurityLevel::System => SecurityLevel::System,
        };

        Ok(())
    }

    /// Add a permission to the security context
    pub fn add_permission(&mut self, permission: String) {
        self.permissions.insert(permission);
    }

    /// Remove a permission from the security context
    pub fn remove_permission(&mut self, permission: &str) {
        self.permissions.remove(permission);
    }

    /// Add a role to the security context
    pub fn add_role(&mut self, role: String) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
    }

    /// Remove a role from the security context
    pub fn remove_role(&mut self, role: &str) {
        self.roles.retain(|r| r != role);
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: SecurityContextMetadata) {
        self.metadata = metadata;
    }

    /// Set request ID for tracing
    pub fn set_request_id(&mut self, request_id: String) {
        self.metadata.request_id = Some(request_id);
    }

    /// Set organization ID
    pub fn set_organization_id(&mut self, org_id: Uuid) {
        self.metadata.organization_id = Some(org_id);
    }

    /// Clone with new expiration
    pub fn with_new_expiration(&self, expires_at: DateTime<Utc>) -> Self {
        let mut cloned = self.clone();
        cloned.expires_at = Some(expires_at);
        cloned
    }
}

/// Audit context for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub security_level: SecurityLevel,
    pub is_api_key: bool,
    pub mfa_verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_security_context_creation() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::from(["read".to_string(), "write".to_string()]);
        let roles = vec!["user".to_string()];

        let context = SecurityContext::new(user_id, None, permissions.clone(), roles.clone());

        assert_eq!(context.user_id, user_id);
        assert_eq!(context.permissions, permissions);
        assert_eq!(context.roles, roles);
        assert!(context.is_valid());
    }

    #[test]
    fn test_permission_checking() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::from(["user:read".to_string(), "workflow:create".to_string()]);
        let roles = vec!["user".to_string()];

        let context = SecurityContext::new(user_id, None, permissions, roles);

        assert!(context.has_permission("user:read"));
        assert!(context.has_permission("workflow:create"));
        assert!(!context.has_permission("admin:delete"));
    }

    #[test]
    fn test_role_checking() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::new();
        let roles = vec!["user".to_string(), "moderator".to_string()];

        let context = SecurityContext::new(user_id, None, permissions, roles);

        assert!(context.has_role("user"));
        assert!(context.has_role("moderator"));
        assert!(!context.has_role("admin"));
    }

    #[test]
    fn test_admin_role_permissions() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::new();
        let roles = vec!["admin".to_string()];

        let context = SecurityContext::new(user_id, None, permissions, roles);

        // Admin role should have all permissions
        assert!(context.has_permission("any:permission"));
        assert!(context.has_role("any_role")); // Admin includes all roles
    }

    #[test]
    fn test_system_context() {
        let permissions = HashSet::from(["system:all".to_string()]);
        let context = SecurityContext::system_context("backup-service", permissions);

        assert_eq!(context.metadata.security_level, SecurityLevel::System);
        assert!(context.has_role("system"));
        assert!(context.has_role("backup-service"));
        assert!(context.has_permission("any:permission")); // System has all permissions
    }

    #[test]
    fn test_expiration() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::new();
        let roles = vec!["user".to_string()];

        // Create expired context
        let expires_at = Utc::now() - chrono::Duration::hours(1);
        let context =
            SecurityContext::with_expiration(user_id, None, permissions, roles, expires_at);

        assert!(context.is_expired());
        assert!(!context.is_valid());
    }

    #[test]
    fn test_mfa_requirements() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::new();
        let roles = vec!["admin".to_string()];
        let metadata = SecurityContextMetadata {
            security_level: SecurityLevel::Administrative,
            mfa_verified: false,
            ..Default::default()
        };

        let context = SecurityContext::with_metadata(user_id, None, permissions, roles, metadata);

        assert!(context.requires_mfa("user:delete"));
        assert!(context.validate_mfa_for_operation("user:delete").is_err());
    }

    #[test]
    fn test_context_elevation() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::new();
        let roles = vec!["user".to_string()];

        let mut context = SecurityContext::new(user_id, None, permissions, roles);
        assert_eq!(context.metadata.security_level, SecurityLevel::Standard);

        context.elevate().unwrap();
        assert_eq!(context.metadata.security_level, SecurityLevel::Elevated);

        context.elevate().unwrap();
        assert_eq!(
            context.metadata.security_level,
            SecurityLevel::Administrative
        );
    }

    #[test]
    fn test_audit_context_creation() {
        let user_id = Uuid::new_v4();
        let permissions = HashSet::new();
        let roles = vec!["user".to_string()];

        let context = SecurityContext::new(user_id, None, permissions, roles);
        let audit_context = context.audit_context();

        assert_eq!(audit_context.user_id, user_id);
        assert_eq!(audit_context.security_level, SecurityLevel::Standard);
    }
}
