use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{error::FileStorageError, AppState};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration time
    pub exp: i64,
    /// Issued at time
    pub iat: i64,
    /// User roles
    pub roles: Vec<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Subscription tier
    pub subscription_tier: Option<String>,
}

/// User context extracted from authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID
    pub user_id: Uuid,
    /// User roles
    pub roles: HashSet<String>,
    /// User permissions
    pub permissions: HashSet<String>,
    /// Subscription tier
    pub subscription_tier: Option<String>,
    /// Is admin user
    pub is_admin: bool,
}

#[async_trait]
impl<S> FromRequestParts<S> for UserContext
where
    S: Send + Sync,
{
    type Rejection = FileStorageError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<UserContext>()
            .cloned()
            .ok_or(FileStorageError::AuthenticationRequired)
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, FileStorageError> {
    // Skip auth for health checks and public endpoints
    if is_public_endpoint(request.uri().path()) {
        debug!(
            "Skipping auth for public endpoint: {}",
            request.uri().path()
        );
        return Ok(next.run(request).await);
    }

    // Extract token from Authorization header
    let token = extract_token_from_request(&request)?;

    // Validate and decode the token
    let claims = validate_token(&token, &state.config.security.jwt_secret)?;

    // Parse user ID from claims
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| FileStorageError::InvalidToken {
        message: "Invalid user ID in token".to_string(),
    })?;

    // Check if user is admin before moving roles
    let is_admin = claims.roles.contains(&"admin".to_string());

    // Create user context
    let user_context = UserContext {
        user_id,
        roles: claims.roles.into_iter().collect(),
        permissions: claims.permissions.into_iter().collect(),
        subscription_tier: claims.subscription_tier,
        is_admin,
    };

    // Check service-specific permissions
    check_service_permissions(&user_context, request.uri().path(), request.method())?;

    // Add user context to request extensions
    request.extensions_mut().insert(user_context);

    debug!("Authentication successful for user: {}", user_id);
    Ok(next.run(request).await)
}

/// Extract token from request headers
fn extract_token_from_request(request: &Request) -> Result<String, FileStorageError> {
    // Try Authorization header first (Bearer token)
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        let auth_str = auth_header
            .to_str()
            .map_err(|_| FileStorageError::InvalidToken {
                message: "Invalid Authorization header encoding".to_string(),
            })?;

        if auth_str.starts_with("Bearer ") {
            return Ok(auth_str[7..].to_string());
        }
    }

    // Try X-API-Key header for service-to-service authentication
    if let Some(api_key) = request.headers().get("X-API-Key") {
        let key_str = api_key
            .to_str()
            .map_err(|_| FileStorageError::InvalidToken {
                message: "Invalid API key header encoding".to_string(),
            })?;

        // For now, treat API key as JWT token
        // In production, you'd validate API keys separately
        return Ok(key_str.to_string());
    }

    // Try query parameter for download links (less secure, limited use)
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "token" {
                    return Ok(urlencoding::decode(value)
                        .map_err(|_| FileStorageError::InvalidToken {
                            message: "Invalid token URL encoding".to_string(),
                        })?
                        .to_string());
                }
            }
        }
    }

    Err(FileStorageError::AuthenticationRequired)
}

/// Validate JWT token
fn validate_token(token: &str, secret: &str) -> Result<Claims, FileStorageError> {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let mut validation = Validation::new(Algorithm::HS256);

    // Set expected issuer and audience
    validation.set_issuer(&["ai-core-platform"]);
    validation.set_audience(&["file-storage-service", "api-gateway"]);

    // Validate expiration
    validation.validate_exp = true;

    // Decode and validate token
    let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
        warn!("Token validation failed: {}", e);
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => FileStorageError::InvalidToken {
                message: "Token has expired".to_string(),
            },
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => FileStorageError::InvalidToken {
                message: "Invalid token issuer".to_string(),
            },
            jsonwebtoken::errors::ErrorKind::InvalidAudience => FileStorageError::InvalidToken {
                message: "Invalid token audience".to_string(),
            },
            _ => FileStorageError::InvalidToken {
                message: "Invalid token".to_string(),
            },
        }
    })?;

    Ok(token_data.claims)
}

/// Check if endpoint is public (doesn't require authentication)
fn is_public_endpoint(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/metrics" | "/api/v1/files/public/*" | "/"
    ) || path.starts_with("/static/")
}

/// Check service-specific permissions
fn check_service_permissions(
    user_context: &UserContext,
    path: &str,
    method: &axum::http::Method,
) -> Result<(), FileStorageError> {
    // Admin users have access to everything
    if user_context.is_admin {
        return Ok(());
    }

    // Check based on HTTP method and path
    match (method.as_str(), path) {
        // Read operations - require read permission
        ("GET", path) if path.starts_with("/api/v1/files/") => {
            if !user_context.permissions.contains("files:read") {
                return Err(FileStorageError::PermissionDenied {
                    action: "read".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        // Upload operations - require write permission
        ("POST", "/api/v1/files/upload") | ("POST", "/api/v1/files/upload/multipart") => {
            if !user_context.permissions.contains("files:write") {
                return Err(FileStorageError::PermissionDenied {
                    action: "write".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        // Update operations - require write permission
        ("PUT", path) if path.starts_with("/api/v1/files/") => {
            if !user_context.permissions.contains("files:write") {
                return Err(FileStorageError::PermissionDenied {
                    action: "update".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        // Delete operations - require delete permission
        ("DELETE", path) if path.starts_with("/api/v1/files/") => {
            if !user_context.permissions.contains("files:delete") {
                return Err(FileStorageError::PermissionDenied {
                    action: "delete".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        // Batch operations - require appropriate permissions
        ("POST", "/api/v1/files/batch/delete") => {
            if !user_context.permissions.contains("files:delete") {
                return Err(FileStorageError::PermissionDenied {
                    action: "batch_delete".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        ("POST", "/api/v1/files/batch/move") => {
            if !user_context.permissions.contains("files:write") {
                return Err(FileStorageError::PermissionDenied {
                    action: "batch_move".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        // Admin operations - require admin permission
        ("GET", path) if path.starts_with("/api/v1/admin/") => {
            if !user_context.permissions.contains("files:admin") {
                return Err(FileStorageError::PermissionDenied {
                    action: "admin".to_string(),
                    resource: "files".to_string(),
                });
            }
        }

        // Folder operations - require appropriate permissions
        ("POST", "/api/v1/folders") => {
            if !user_context.permissions.contains("folders:create") {
                return Err(FileStorageError::PermissionDenied {
                    action: "create".to_string(),
                    resource: "folders".to_string(),
                });
            }
        }

        ("GET", path) if path.starts_with("/api/v1/folders/") => {
            if !user_context.permissions.contains("folders:read") {
                return Err(FileStorageError::PermissionDenied {
                    action: "read".to_string(),
                    resource: "folders".to_string(),
                });
            }
        }

        // Default allow for other endpoints (they may have their own checks)
        _ => {}
    }

    Ok(())
}

/// Check subscription tier limits
pub fn check_subscription_limits(
    user_context: &UserContext,
    operation: &str,
) -> Result<(), FileStorageError> {
    match user_context.subscription_tier.as_deref() {
        Some("free") => {
            match operation {
                "upload" => {
                    // Free tier: max 10MB file size, 1GB total storage
                    // These checks would be implemented in the upload handler
                }
                "batch_operation" => {
                    return Err(FileStorageError::PermissionDenied {
                        action: "batch_operations".to_string(),
                        resource: "free_tier_limitation".to_string(),
                    });
                }
                "video_processing" => {
                    return Err(FileStorageError::PermissionDenied {
                        action: "video_processing".to_string(),
                        resource: "free_tier_limitation".to_string(),
                    });
                }
                _ => {}
            }
        }
        Some("pro") => {
            // Pro tier: max 1GB file size, 100GB total storage
            // More permissive limits
        }
        Some("enterprise") => {
            // Enterprise tier: no limits
        }
        None | Some(_) => {
            // Default to free tier limits
            if matches!(operation, "batch_operation" | "video_processing") {
                return Err(FileStorageError::PermissionDenied {
                    action: operation.to_string(),
                    resource: "subscription_required".to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Extract user context from request extensions
pub fn get_user_context(request: &Request) -> Option<&UserContext> {
    request.extensions().get::<UserContext>()
}

/// Require user context (for handlers)
pub fn require_user_context(request: &Request) -> Result<&UserContext, FileStorageError> {
    get_user_context(request).ok_or(FileStorageError::AuthenticationRequired)
}

/// Check if user has specific permission
pub fn has_permission(user_context: &UserContext, permission: &str) -> bool {
    user_context.is_admin || user_context.permissions.contains(permission)
}

/// Check if user has any of the specified roles
pub fn has_role(user_context: &UserContext, roles: &[&str]) -> bool {
    user_context.is_admin || roles.iter().any(|role| user_context.roles.contains(*role))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_token(user_id: &str, roles: Vec<String>, permissions: Vec<String>) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let claims = Claims {
            sub: user_id.to_string(),
            iss: "ai-core-platform".to_string(),
            aud: "file-storage-service".to_string(),
            exp: now + 3600, // 1 hour from now
            iat: now,
            roles,
            permissions,
            subscription_tier: Some("pro".to_string()),
        };

        let secret = "test-secret";
        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(secret.as_ref());

        encode(&header, &claims, &encoding_key).unwrap()
    }

    #[test]
    fn test_public_endpoints() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/metrics"));
        assert!(is_public_endpoint("/static/favicon.ico"));
        assert!(!is_public_endpoint("/api/v1/files/upload"));
    }

    #[test]
    fn test_token_validation() {
        let token = create_test_token(
            &Uuid::new_v4().to_string(),
            vec!["user".to_string()],
            vec!["files:read".to_string()],
        );

        let claims = validate_token(&token, "test-secret").unwrap();
        assert_eq!(claims.iss, "ai-core-platform");
        assert_eq!(claims.aud, "file-storage-service");
        assert!(claims.roles.contains(&"user".to_string()));
        assert!(claims.permissions.contains(&"files:read".to_string()));
    }

    #[test]
    fn test_permission_checks() {
        let user_context = UserContext {
            user_id: Uuid::new_v4(),
            roles: ["user"].iter().map(|s| s.to_string()).collect(),
            permissions: ["files:read", "files:write"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            subscription_tier: Some("pro".to_string()),
            is_admin: false,
        };

        // Should allow read operations
        assert!(
            check_service_permissions(&user_context, "/api/v1/files/test-file", &Method::GET)
                .is_ok()
        );

        // Should allow write operations
        assert!(
            check_service_permissions(&user_context, "/api/v1/files/upload", &Method::POST).is_ok()
        );

        // Should deny delete operations (no permission)
        assert!(check_service_permissions(
            &user_context,
            "/api/v1/files/test-file",
            &Method::DELETE
        )
        .is_err());

        // Should deny admin operations
        assert!(
            check_service_permissions(&user_context, "/api/v1/admin/stats", &Method::GET).is_err()
        );
    }

    #[test]
    fn test_admin_permissions() {
        let admin_context = UserContext {
            user_id: Uuid::new_v4(),
            roles: ["admin"].iter().map(|s| s.to_string()).collect(),
            permissions: HashSet::new(),
            subscription_tier: Some("enterprise".to_string()),
            is_admin: true,
        };

        // Admin should have access to everything
        assert!(
            check_service_permissions(&admin_context, "/api/v1/admin/stats", &Method::GET).is_ok()
        );

        assert!(check_service_permissions(
            &admin_context,
            "/api/v1/files/test-file",
            &Method::DELETE
        )
        .is_ok());
    }

    #[test]
    fn test_subscription_limits() {
        let free_user = UserContext {
            user_id: Uuid::new_v4(),
            roles: ["user"].iter().map(|s| s.to_string()).collect(),
            permissions: ["files:read"].iter().map(|s| s.to_string()).collect(),
            subscription_tier: Some("free".to_string()),
            is_admin: false,
        };

        // Free tier should not allow batch operations
        assert!(check_subscription_limits(&free_user, "batch_operation").is_err());

        // Free tier should not allow video processing
        assert!(check_subscription_limits(&free_user, "video_processing").is_err());

        // Free tier should allow basic operations
        assert!(check_subscription_limits(&free_user, "upload").is_ok());

        let pro_user = UserContext {
            subscription_tier: Some("pro".to_string()),
            ..free_user
        };

        // Pro tier should allow batch operations
        assert!(check_subscription_limits(&pro_user, "batch_operation").is_ok());
    }

    #[test]
    fn test_helper_functions() {
        let user_context = UserContext {
            user_id: Uuid::new_v4(),
            roles: ["user", "editor"].iter().map(|s| s.to_string()).collect(),
            permissions: ["files:read", "files:write"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            subscription_tier: Some("pro".to_string()),
            is_admin: false,
        };

        assert!(has_permission(&user_context, "files:read"));
        assert!(!has_permission(&user_context, "files:admin"));

        assert!(has_role(&user_context, &["user"]));
        assert!(has_role(&user_context, &["editor", "admin"])); // Has editor
        assert!(!has_role(&user_context, &["admin"]));

        // Admin should have all permissions and roles
        let admin_context = UserContext {
            is_admin: true,
            ..user_context
        };

        assert!(has_permission(&admin_context, "files:admin"));
        assert!(has_role(&admin_context, &["admin"]));
    }
}
