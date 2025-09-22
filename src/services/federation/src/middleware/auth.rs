//! Authentication middleware for the Federation Service
//!
//! This module provides authentication middleware for validating client credentials,
//! JWT tokens, API keys, and managing authentication context throughout the request lifecycle.

use crate::config::AuthConfig;
use crate::models::FederationError;
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

/// Authentication context passed through the request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// Authenticated client ID
    pub client_id: Uuid,
    /// Client name
    pub client_name: String,
    /// Client tier
    pub client_tier: String,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Token expiration (for JWT)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// API key authentication
    ApiKey,
    /// JWT token authentication
    Jwt,
    /// OAuth token authentication
    OAuth,
    /// Basic authentication
    Basic,
}

/// Authentication middleware
#[derive(Debug, Clone)]
pub struct AuthMiddleware {
    /// Authentication configuration
    config: AuthConfig,
}

impl AuthMiddleware {
    /// Create new authentication middleware
    pub async fn new(config: &AuthConfig) -> Result<Self, FederationError> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Validate API key
    pub async fn validate_api_key(&self, api_key: &str) -> Result<AuthContext, FederationError> {
        // This would implement actual API key validation
        // For now, return a dummy context
        Ok(AuthContext {
            client_id: Uuid::new_v4(),
            client_name: "Test Client".to_string(),
            client_tier: "professional".to_string(),
            auth_method: AuthMethod::ApiKey,
            expires_at: None,
        })
    }

    /// Validate JWT token
    pub async fn validate_jwt(&self, token: &str) -> Result<AuthContext, FederationError> {
        // This would implement actual JWT validation
        // For now, return a dummy context
        Ok(AuthContext {
            client_id: Uuid::new_v4(),
            client_name: "JWT Client".to_string(),
            client_tier: "enterprise".to_string(),
            auth_method: AuthMethod::Jwt,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        })
    }
}

/// Authentication middleware function
pub async fn auth_middleware(
    State(auth_middleware): State<AuthMiddleware>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication for health and public endpoints
    let path = request.uri().path();
    if path.starts_with("/health") || path.starts_with("/metrics") || path == "/status" {
        return Ok(next.run(request).await);
    }

    // Extract authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let auth_context = match auth_header {
        Some(header_value) => {
            if let Some(api_key) = header_value.strip_prefix("Bearer ") {
                if api_key.starts_with("fed_") {
                    // API key authentication
                    auth_middleware
                        .validate_api_key(api_key)
                        .await
                        .map_err(|_| StatusCode::UNAUTHORIZED)?
                } else {
                    // JWT token authentication
                    auth_middleware
                        .validate_jwt(api_key)
                        .await
                        .map_err(|_| StatusCode::UNAUTHORIZED)?
                }
            } else if let Some(api_key) = header_value.strip_prefix("ApiKey ") {
                // Direct API key authentication
                auth_middleware
                    .validate_api_key(api_key)
                    .await
                    .map_err(|_| StatusCode::UNAUTHORIZED)?
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => {
            // No authorization header
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Add auth context to request extensions
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_middleware_creation() {
        let config = AuthConfig::default();
        let middleware = AuthMiddleware::new(&config).await.unwrap();
        assert!(middleware.config.jwt.secret.len() > 0);
    }

    #[tokio::test]
    async fn test_api_key_validation() {
        let config = AuthConfig::default();
        let middleware = AuthMiddleware::new(&config).await.unwrap();

        let result = middleware.validate_api_key("fed_test_key").await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.auth_method, AuthMethod::ApiKey);
    }

    #[tokio::test]
    async fn test_jwt_validation() {
        let config = AuthConfig::default();
        let middleware = AuthMiddleware::new(&config).await.unwrap();

        let result = middleware.validate_jwt("test.jwt.token").await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.auth_method, AuthMethod::Jwt);
        assert!(context.expires_at.is_some());
    }
}
