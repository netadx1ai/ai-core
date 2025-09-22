//! Authentication Middleware
//!
//! This module provides authentication middleware for the MCP Manager Service,
//! supporting JWT tokens and API key authentication methods.

use crate::server::AppState;
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Issued at
    pub iat: usize,
    /// Expiration time
    pub exp: usize,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// User roles
    pub roles: Vec<String>,
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID
    pub user_id: String,
    /// User roles
    pub roles: Vec<String>,
    /// Authentication method used
    pub auth_method: AuthMethod,
}

/// Authentication methods
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// JWT token authentication
    Jwt,
    /// API key authentication
    ApiKey,
    /// No authentication (development mode)
    None,
}

/// Authentication middleware
///
/// Validates JWT tokens or API keys based on the service configuration.
/// In development mode, authentication can be bypassed.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication in development mode if configured
    if state.is_development()
        && !state.config.security.jwt_enabled
        && !state.config.security.api_key_enabled
    {
        debug!("Skipping authentication in development mode");
        return Ok(next.run(request).await);
    }

    // Extract authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_context = match auth_header {
        Some(header) => {
            if header.starts_with("Bearer ") {
                // JWT authentication
                if state.config.security.jwt_enabled {
                    let token = &header[7..]; // Remove "Bearer " prefix
                    match validate_jwt_token(token, &state.config.security.jwt_secret).await {
                        Ok(claims) => Some(AuthContext {
                            user_id: claims.sub,
                            roles: claims.roles,
                            auth_method: AuthMethod::Jwt,
                        }),
                        Err(e) => {
                            warn!("JWT validation failed: {}", e);
                            return Err(StatusCode::UNAUTHORIZED);
                        }
                    }
                } else {
                    warn!("JWT token provided but JWT authentication is disabled");
                    return Err(StatusCode::UNAUTHORIZED);
                }
            } else if header.starts_with("ApiKey ") {
                // API key authentication
                if state.config.security.api_key_enabled {
                    let api_key = &header[7..]; // Remove "ApiKey " prefix
                    if validate_api_key(api_key, &state.config.security.api_keys).await {
                        Some(AuthContext {
                            user_id: "api_key_user".to_string(),
                            roles: vec!["api_user".to_string()],
                            auth_method: AuthMethod::ApiKey,
                        })
                    } else {
                        warn!("Invalid API key provided");
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                } else {
                    warn!("API key provided but API key authentication is disabled");
                    return Err(StatusCode::UNAUTHORIZED);
                }
            } else {
                warn!("Invalid authorization header format");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => {
            // No authorization header
            if state.config.security.jwt_enabled || state.config.security.api_key_enabled {
                warn!("No authorization header provided");
                return Err(StatusCode::UNAUTHORIZED);
            } else {
                // Authentication not required
                Some(AuthContext {
                    user_id: "anonymous".to_string(),
                    roles: vec!["anonymous".to_string()],
                    auth_method: AuthMethod::None,
                })
            }
        }
    };

    // Add auth context to request extensions
    if let Some(context) = auth_context {
        request.extensions_mut().insert(context);
    }

    Ok(next.run(request).await)
}

/// Validate JWT token
async fn validate_jwt_token(token: &str, secret: &str) -> Result<Claims, String> {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let mut validation = Validation::new(Algorithm::HS256);

    // Configure validation
    validation.set_audience(&["mcp-manager"]);
    validation.set_issuer(&["ai-core-platform"]);

    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            debug!(
                user_id = %token_data.claims.sub,
                roles = ?token_data.claims.roles,
                "JWT token validated successfully"
            );
            Ok(token_data.claims)
        }
        Err(e) => Err(format!("JWT validation error: {}", e)),
    }
}

/// Validate API key
async fn validate_api_key(api_key: &str, valid_keys: &[String]) -> bool {
    if valid_keys.is_empty() {
        warn!("No valid API keys configured");
        return false;
    }

    let is_valid = valid_keys.iter().any(|key| key == api_key);

    if is_valid {
        debug!("API key validated successfully");
    } else {
        warn!("Invalid API key provided");
    }

    is_valid
}

/// Check if user has required role
pub fn has_role(auth_context: &AuthContext, required_role: &str) -> bool {
    auth_context.roles.iter().any(|role| role == required_role)
}

/// Check if user has any of the required roles
pub fn has_any_role(auth_context: &AuthContext, required_roles: &[String]) -> bool {
    auth_context
        .roles
        .iter()
        .any(|role| required_roles.contains(role))
}

/// Role-based authorization middleware
pub async fn require_role(
    required_role: String,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>,
> + Clone {
    move |request: Request, next: Next| {
        let required_role = required_role.clone();
        Box::pin(async move {
            let auth_context = request
                .extensions()
                .get::<AuthContext>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            if !has_role(auth_context, &required_role) {
                warn!(
                    user_id = %auth_context.user_id,
                    required_role = %required_role,
                    user_roles = ?auth_context.roles,
                    "Access denied: insufficient privileges"
                );
                return Err(StatusCode::FORBIDDEN);
            }

            Ok(next.run(request).await)
        })
    }
}

/// Admin role authorization middleware
pub async fn require_admin(request: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !has_role(auth_context, "admin") {
        warn!(
            user_id = %auth_context.user_id,
            user_roles = ?auth_context.roles,
            "Access denied: admin role required"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Extract auth context from request
pub fn extract_auth_context(request: &Request) -> Option<&AuthContext> {
    request.extensions().get::<AuthContext>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_claims() -> Claims {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        Claims {
            sub: "test-user".to_string(),
            iat: now,
            exp: now + 3600, // 1 hour from now
            iss: "ai-core-platform".to_string(),
            aud: "mcp-manager".to_string(),
            roles: vec!["user".to_string(), "admin".to_string()],
        }
    }

    #[tokio::test]
    async fn test_jwt_validation() {
        let secret = "test-secret-key-that-is-long-enough";
        let claims = create_test_claims();

        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        let token = encode(&Header::default(), &claims, &encoding_key).unwrap();

        let result = validate_jwt_token(&token, secret).await;
        assert!(result.is_ok());

        let decoded_claims = result.unwrap();
        assert_eq!(decoded_claims.sub, "test-user");
        assert_eq!(decoded_claims.roles, vec!["user", "admin"]);
    }

    #[tokio::test]
    async fn test_jwt_validation_invalid_token() {
        let secret = "test-secret-key-that-is-long-enough";
        let invalid_token = "invalid.token.here";

        let result = validate_jwt_token(invalid_token, secret).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_api_key_validation() {
        let valid_keys = vec!["key1".to_string(), "key2".to_string()];

        assert!(validate_api_key("key1", &valid_keys).await);
        assert!(validate_api_key("key2", &valid_keys).await);
        assert!(!validate_api_key("invalid", &valid_keys).await);
        assert!(!validate_api_key("key1", &[]).await);
    }

    #[test]
    fn test_role_checking() {
        let auth_context = AuthContext {
            user_id: "test-user".to_string(),
            roles: vec!["user".to_string(), "admin".to_string()],
            auth_method: AuthMethod::Jwt,
        };

        assert!(has_role(&auth_context, "user"));
        assert!(has_role(&auth_context, "admin"));
        assert!(!has_role(&auth_context, "superuser"));

        let required_roles = vec!["admin".to_string(), "superuser".to_string()];
        assert!(has_any_role(&auth_context, &required_roles));

        let required_roles = vec!["superuser".to_string(), "root".to_string()];
        assert!(!has_any_role(&auth_context, &required_roles));
    }
}
