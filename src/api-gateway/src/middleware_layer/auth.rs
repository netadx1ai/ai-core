//! Authentication middleware for JWT token validation and user context extraction

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashSet;
use tracing::{debug, warn};

use crate::{
    error::{ApiError, Result},
    state::AppState,
};
use ai_core_shared::types::core::{Permission, SubscriptionTier, TokenClaims};

/// User context extracted from JWT token
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: HashSet<Permission>,
    pub subscription_tier: SubscriptionTier,
    pub token_claims: TokenClaims,
}

impl UserContext {
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| self.permissions.contains(p))
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        permissions.iter().all(|p| self.permissions.contains(p))
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Get user's subscription tier
    pub fn subscription_tier(&self) -> &SubscriptionTier {
        &self.subscription_tier
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.has_role("admin") || self.has_permission(&Permission::AdminSystem)
    }

    /// Check if user can create workflows
    pub fn can_create_workflows(&self) -> bool {
        self.has_permission(&Permission::WorkflowsCreate)
    }

    /// Check if user can manage federation
    pub fn can_manage_federation(&self) -> bool {
        self.has_permission(&Permission::FederationManage)
    }
}

/// Authentication middleware that validates JWT tokens and extracts user context
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| ApiError::authentication("Missing authorization header"))?;

    // Validate Bearer token format
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::authentication(
            "Invalid authorization header format",
        ));
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // Validate and decode JWT token
    let user_context = validate_jwt_token(&state, token).await?;

    debug!(
        user_id = %user_context.user_id,
        roles = ?user_context.roles,
        subscription_tier = ?user_context.subscription_tier,
        "User authenticated successfully"
    );

    // Add user context to request extensions
    request.extensions_mut().insert(user_context);

    // Continue with the request
    let response = next.run(request).await;

    Ok(response)
}

/// Optional authentication middleware that allows requests without authentication
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // Try to extract authorization header
    if let Some(auth_header) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
    {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];

            // Try to validate token, but continue even if it fails
            if let Ok(user_context) = validate_jwt_token(&state, token).await {
                request.extensions_mut().insert(user_context);
            }
        }
    }

    // Continue with the request regardless of authentication result
    let response = next.run(request).await;
    Ok(response)
}

/// Validate JWT token and return user context
async fn validate_jwt_token(state: &AppState, token: &str) -> Result<UserContext> {
    // Validate JWT token using auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("authentication"))?;

    let token_claims = auth_service.validate_token(token).await?;

    // Check if token is blacklisted (for logout functionality)
    if auth_service.is_token_blacklisted(token).await? {
        return Err(ApiError::authentication("Token has been revoked"));
    }

    // Convert string permissions to enum permissions
    let permissions = parse_permissions(&token_claims.permissions)?;

    // Create user context
    let user_context = UserContext {
        user_id: token_claims.sub.clone(),
        roles: token_claims.roles.clone(),
        permissions,
        subscription_tier: token_claims.subscription_tier.clone(),
        token_claims,
    };

    Ok(user_context)
}

/// Parse string permissions into enum permissions
fn parse_permissions(permission_strings: &[String]) -> Result<HashSet<Permission>> {
    let mut permissions = HashSet::new();

    for perm_str in permission_strings {
        let permission = match perm_str.as_str() {
            "workflows:read" => Permission::WorkflowsRead,
            "workflows:create" => Permission::WorkflowsCreate,
            "workflows:update" => Permission::WorkflowsUpdate,
            "workflows:delete" => Permission::WorkflowsDelete,
            "content:read" => Permission::ContentRead,
            "content:create" => Permission::ContentCreate,
            "content:update" => Permission::ContentUpdate,
            "content:delete" => Permission::ContentDelete,
            "campaigns:read" => Permission::CampaignsRead,
            "campaigns:create" => Permission::CampaignsCreate,
            "campaigns:update" => Permission::CampaignsUpdate,
            "campaigns:delete" => Permission::CampaignsDelete,
            "analytics:read" => Permission::AnalyticsRead,
            "analytics:export" => Permission::AnalyticsExport,
            "federation:proxy" => Permission::FederationProxy,
            "federation:manage" => Permission::FederationManage,
            "admin:users" => Permission::AdminUsers,
            "admin:system" => Permission::AdminSystem,
            "admin:billing" => Permission::AdminBilling,
            _ => {
                warn!("Unknown permission: {}", perm_str);
                continue;
            }
        };
        permissions.insert(permission);
    }

    Ok(permissions)
}

/// Middleware to require specific permissions
pub fn require_permission(
    required_permission: Permission,
) -> impl Fn(UserContext) -> Result<UserContext> {
    move |user_context: UserContext| {
        if !user_context.has_permission(&required_permission) {
            return Err(ApiError::authorization(format!(
                "Permission required: {:?}",
                required_permission
            )));
        }
        Ok(user_context)
    }
}

/// Middleware to require any of the specified permissions
pub fn require_any_permission(
    required_permissions: Vec<Permission>,
) -> impl Fn(UserContext) -> Result<UserContext> {
    move |user_context: UserContext| {
        if !user_context.has_any_permission(&required_permissions) {
            return Err(ApiError::authorization(format!(
                "One of the following permissions required: {:?}",
                required_permissions
            )));
        }
        Ok(user_context)
    }
}

/// Middleware to require specific role
pub fn require_role(required_role: String) -> impl Fn(UserContext) -> Result<UserContext> {
    move |user_context: UserContext| {
        if !user_context.has_role(&required_role) {
            return Err(ApiError::authorization(format!(
                "Role required: {}",
                required_role
            )));
        }
        Ok(user_context)
    }
}

/// Middleware to require admin role
pub fn require_admin() -> impl Fn(UserContext) -> Result<UserContext> {
    |user_context: UserContext| {
        if !user_context.is_admin() {
            return Err(ApiError::authorization("Admin role required"));
        }
        Ok(user_context)
    }
}

/// Middleware to require minimum subscription tier
pub fn require_subscription_tier(
    min_tier: SubscriptionTier,
) -> impl Fn(UserContext) -> Result<UserContext> {
    move |user_context: UserContext| {
        let user_tier_level = match user_context.subscription_tier {
            SubscriptionTier::Free => 0,
            SubscriptionTier::Pro => 1,
            SubscriptionTier::Enterprise => 2,
        };

        let required_tier_level = match min_tier {
            SubscriptionTier::Free => 0,
            SubscriptionTier::Pro => 1,
            SubscriptionTier::Enterprise => 2,
        };

        if user_tier_level < required_tier_level {
            return Err(ApiError::authorization(format!(
                "Subscription tier required: {:?}",
                min_tier
            )));
        }

        Ok(user_context)
    }
}

/// Extract user context from request extensions
pub fn extract_user_context(request: &Request) -> Option<&UserContext> {
    request.extensions().get::<UserContext>()
}

/// Extract user context from request extensions, returning error if not found
pub fn require_user_context(request: &Request) -> Result<&UserContext> {
    extract_user_context(request).ok_or_else(|| ApiError::authentication("User context not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_user_context_permissions() {
        let mut permissions = HashSet::new();
        permissions.insert(Permission::WorkflowsRead);
        permissions.insert(Permission::WorkflowsCreate);
        permissions.insert(Permission::ContentRead);

        let user_context = UserContext {
            user_id: "test-user".to_string(),
            roles: vec!["user".to_string()],
            permissions: permissions.clone(),
            subscription_tier: SubscriptionTier::Pro,
            token_claims: TokenClaims {
                sub: "test-user".to_string(),
                iss: "AI-PLATFORM".to_string(),
                aud: "api-gateway".to_string(),
                exp: 1234567890,
                iat: 1234567890,
                roles: vec!["user".to_string()],
                permissions: vec!["workflows:read".to_string()],
                subscription_tier: SubscriptionTier::Pro,
            },
        };

        // Test single permission
        assert!(user_context.has_permission(&Permission::WorkflowsRead));
        assert!(!user_context.has_permission(&Permission::WorkflowsDelete));

        // Test multiple permissions
        assert!(user_context
            .has_any_permission(&[Permission::WorkflowsRead, Permission::WorkflowsDelete]));
        assert!(!user_context
            .has_all_permissions(&[Permission::WorkflowsRead, Permission::WorkflowsDelete]));
        assert!(
            user_context.has_all_permissions(&[Permission::WorkflowsRead, Permission::ContentRead])
        );

        // Test roles
        assert!(user_context.has_role("user"));
        assert!(!user_context.has_role("admin"));
        assert!(user_context.has_any_role(&["user", "admin"]));

        // Test admin check
        assert!(!user_context.is_admin());

        // Test workflow creation
        assert!(user_context.can_create_workflows());

        // Test federation management
        assert!(!user_context.can_manage_federation());
    }

    #[test]
    fn test_parse_permissions() {
        let permission_strings = vec![
            "workflows:read".to_string(),
            "workflows:create".to_string(),
            "content:read".to_string(),
            "unknown:permission".to_string(), // Should be ignored
        ];

        let permissions = parse_permissions(&permission_strings).unwrap();

        assert_eq!(permissions.len(), 3);
        assert!(permissions.contains(&Permission::WorkflowsRead));
        assert!(permissions.contains(&Permission::WorkflowsCreate));
        assert!(permissions.contains(&Permission::ContentRead));
    }

    #[test]
    fn test_subscription_tier_ordering() {
        let free_user = UserContext {
            user_id: "free-user".to_string(),
            roles: vec![],
            permissions: HashSet::new(),
            subscription_tier: SubscriptionTier::Free,
            token_claims: TokenClaims {
                sub: "free-user".to_string(),
                iss: "AI-PLATFORM".to_string(),
                aud: "api-gateway".to_string(),
                exp: 1234567890,
                iat: 1234567890,
                roles: vec![],
                permissions: vec![],
                subscription_tier: SubscriptionTier::Free,
            },
        };

        let pro_user = UserContext {
            subscription_tier: SubscriptionTier::Pro,
            ..free_user.clone()
        };

        // Test tier requirements
        let require_pro = require_subscription_tier(SubscriptionTier::Pro);

        assert!(require_pro(pro_user).is_ok());
        assert!(require_pro(free_user).is_err());
    }
}
