//! Simplified Security Middleware Implementation
//!
//! This module provides a cleaner, simpler implementation of security middleware
//! that avoids complex async trait issues while maintaining core security functionality.

use crate::config::SecurityConfig;
use crate::errors::{SecurityError, SecurityResult};
use crate::jwt::{JwtService, JwtServiceTrait, ValidationResult};
use crate::rate_limiting::RateLimiter;
use crate::rbac::{AuthorizationContext, RbacService, RequestMetadata};

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Security middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMiddlewareConfig {
    /// Enable authentication middleware
    pub enable_authentication: bool,
    /// Enable authorization middleware
    pub enable_authorization: bool,
    /// Enable rate limiting middleware
    pub enable_rate_limiting: bool,
    /// Enable input validation middleware
    pub enable_input_validation: bool,
    /// Enable security headers middleware
    pub enable_security_headers: bool,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// Security headers configuration
    pub security_headers: SecurityHeadersConfig,
    /// Input validation configuration
    pub input_validation: InputValidationConfig,
}

impl Default for SecurityMiddlewareConfig {
    fn default() -> Self {
        Self {
            enable_authentication: true,
            enable_authorization: true,
            enable_rate_limiting: true,
            enable_input_validation: true,
            enable_security_headers: true,
            rate_limit: RateLimitConfig::default(),
            security_headers: SecurityHeadersConfig::default(),
            input_validation: InputValidationConfig::default(),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Burst size
    pub burst_size: u32,
    /// Enable per-user rate limiting
    pub per_user: bool,
    /// Enable per-IP rate limiting
    pub per_ip: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            per_user: true,
            per_ip: true,
        }
    }
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub content_security_policy: Option<String>,
    /// Strict Transport Security
    pub strict_transport_security: Option<String>,
    /// X-Frame-Options
    pub x_frame_options: Option<String>,
    /// X-Content-Type-Options
    pub x_content_type_options: Option<String>,
    /// Referrer Policy
    pub referrer_policy: Option<String>,
    /// X-XSS-Protection
    pub x_xss_protection: Option<String>,
    /// Custom headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_security_policy: Some("default-src 'self'".to_string()),
            strict_transport_security: Some("max-age=31536000; includeSubDomains".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            custom_headers: HashMap::new(),
        }
    }
}

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidationConfig {
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum number of headers
    pub max_header_count: usize,
    /// Maximum header value length
    pub max_header_value_length: usize,
    /// Blocked user agents
    pub blocked_user_agents: Vec<String>,
    /// Allowed origins
    pub allowed_origins: Vec<String>,
}

impl Default for InputValidationConfig {
    fn default() -> Self {
        Self {
            max_request_size: 1024 * 1024, // 1MB
            max_header_count: 100,
            max_header_value_length: 8192,
            blocked_user_agents: vec!["bot".to_string(), "crawler".to_string()],
            allowed_origins: vec!["*".to_string()],
        }
    }
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthenticationResult {
    pub user_id: Uuid,
    pub validation_result: ValidationResult,
}

/// Security context for requests
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub request_id: String,
    pub client_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub authentication: Option<AuthenticationResult>,
    pub timestamp: DateTime<Utc>,
}

/// Security statistics
#[derive(Debug, Default, Serialize)]
pub struct SecurityStats {
    pub total_requests: u64,
    pub authenticated_requests: u64,
    pub failed_authentications: u64,
    pub authorization_denials: u64,
    pub rate_limit_blocks: u64,
    pub validation_failures: u64,
    pub blocked_requests: u64,
    pub last_reset: DateTime<Utc>,
}

/// Main security middleware
#[derive(Clone)]
pub struct SimpleSecurityMiddleware {
    config: SecurityMiddlewareConfig,
    jwt_service: Arc<JwtService>,
    rbac_service: Arc<RbacService>,
    rate_limiter: Arc<RateLimiter>,
    stats: Arc<RwLock<SecurityStats>>,
}

impl SimpleSecurityMiddleware {
    /// Create new security middleware
    pub async fn new(
        config: SecurityMiddlewareConfig,
        security_config: &SecurityConfig,
    ) -> SecurityResult<Self> {
        let redis_client = Arc::new(
            redis::Client::open("redis://127.0.0.1:6379/")
                .map_err(|e| SecurityError::CacheConnection(e.to_string()))?,
        );

        let jwt_config = crate::jwt::JwtConfig {
            secret: security_config.jwt.secret.clone(),
            issuer: security_config.jwt.issuer.clone(),
            audience: security_config.jwt.audience.clone(),
            access_token_ttl: chrono::TimeDelta::from_std(security_config.jwt.access_token_ttl)
                .unwrap(),
            refresh_token_ttl: chrono::TimeDelta::from_std(security_config.jwt.refresh_token_ttl)
                .unwrap(),
            algorithm: jsonwebtoken::Algorithm::HS256,
            enable_blacklist: security_config.jwt.enable_blacklist,
            max_tokens_per_user: security_config.jwt.max_tokens_per_user,
        };
        let jwt_service = Arc::new(JwtService::new(jwt_config, redis_client.clone())?);

        // Create Redis permission cache for RBAC
        let permission_cache =
            Arc::new(crate::rbac::RedisPermissionCache::new(redis_client.clone()));

        // Create mock role repository for demo (would use real DB in production)
        let role_repo: Arc<dyn crate::rbac::RoleRepository> =
            Arc::new(crate::service::MockRoleRepository::default());

        let rbac_config = crate::rbac::RbacConfig {
            enable_rbac: true,
            enable_abac: false,
            cache_ttl: chrono::TimeDelta::seconds(300),
            admin_override: true,
            evaluation_mode: crate::rbac::PermissionEvaluationMode::Strict,
            max_policy_evaluation_time_ms: 1000,
        };

        let rbac_service = Arc::new(RbacService::new(role_repo, permission_cache, rbac_config));

        let rate_limit_config = crate::rate_limiting::RateLimitConfig {
            requests_per_minute: security_config.rate_limiting.requests_per_minute,
            requests_per_hour: security_config.rate_limiting.requests_per_hour,
            burst_multiplier: security_config.rate_limiting.burst_multiplier,
            per_user_limiting: true,
            per_ip_limiting: true,
            cleanup_interval: std::time::Duration::from_secs(300),
        };
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_config));

        Ok(Self {
            config,
            jwt_service,
            rbac_service,
            rate_limiter,
            stats: Arc::new(RwLock::new(SecurityStats {
                last_reset: Utc::now(),
                ..Default::default()
            })),
        })
    }

    /// Authentication middleware function
    pub async fn authenticate(&self, mut req: Request, next: Next) -> Result<Response, StatusCode> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += 1;
        }

        if !self.config.enable_authentication {
            return Ok(next.run(req).await);
        }

        // Extract token from Authorization header
        let token = self.extract_token_from_request(&req)?;

        // Validate token
        match self.jwt_service.validate_access_token(&token).await {
            Ok(validation_result) => {
                // Add authentication result to request extensions
                let auth_result = AuthenticationResult {
                    user_id: validation_result.user_id,
                    validation_result,
                };
                req.extensions_mut().insert(auth_result);

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.authenticated_requests += 1;
                }

                Ok(next.run(req).await)
            }
            Err(_) => {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.failed_authentications += 1;
                }
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }

    /// Authorization middleware function
    pub async fn authorize(
        &self,
        req: Request,
        next: Next,
        resource: String,
        action: String,
    ) -> Result<Response, StatusCode> {
        if !self.config.enable_authorization {
            return Ok(next.run(req).await);
        }

        // Get authentication result from request extensions
        let auth_result = req
            .extensions()
            .get::<AuthenticationResult>()
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Create authorization context
        let context = AuthorizationContext {
            user_id: auth_result.user_id,
            resource,
            action,
            attributes: HashMap::new(),
            request_metadata: RequestMetadata {
                client_ip: self
                    .extract_client_ip(req.headers())
                    .map(|ip| ip.to_string()),
                user_agent: req
                    .headers()
                    .get("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string()),
                timestamp: chrono::Utc::now(),
                request_id: Some(uuid::Uuid::new_v4().to_string()),
                geolocation: None,
            },
        };

        // Check authorization
        match self.rbac_service.authorize(&context).await {
            Ok(decision) if decision.allowed => Ok(next.run(req).await),
            _ => {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.authorization_denials += 1;
                }
                Err(StatusCode::FORBIDDEN)
            }
        }
    }

    /// Rate limiting middleware function
    pub async fn rate_limit(&self, req: Request, next: Next) -> Result<Response, StatusCode> {
        if !self.config.enable_rate_limiting {
            return Ok(next.run(req).await);
        }

        let client_ip = self.extract_client_ip(req.headers());
        let path = req.uri().path();

        // Check rate limits
        if let Err(_) = self.rate_limiter.check_endpoint_limit(path).await {
            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.rate_limit_blocks += 1;
            }
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        if let Some(ip) = client_ip {
            if let Err(_) = self.rate_limiter.check_ip_limit(ip).await {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.rate_limit_blocks += 1;
                }
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }

        Ok(next.run(req).await)
    }

    /// Input validation middleware function
    pub async fn validate_input(&self, req: Request, next: Next) -> Result<Response, StatusCode> {
        if !self.config.enable_input_validation {
            return Ok(next.run(req).await);
        }

        // Validate headers
        if req.headers().len() > self.config.input_validation.max_header_count {
            {
                let mut stats = self.stats.write().await;
                stats.validation_failures += 1;
            }
            return Err(StatusCode::BAD_REQUEST);
        }

        // Check for blocked user agents
        if let Some(user_agent) = self.extract_user_agent(req.headers()) {
            for blocked_ua in &self.config.input_validation.blocked_user_agents {
                if user_agent
                    .to_lowercase()
                    .contains(&blocked_ua.to_lowercase())
                {
                    {
                        let mut stats = self.stats.write().await;
                        stats.blocked_requests += 1;
                    }
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }

        Ok(next.run(req).await)
    }

    /// Add security headers to response
    pub fn add_security_headers(&self, mut response: Response) -> Response {
        if !self.config.enable_security_headers {
            return response;
        }

        let headers = response.headers_mut();

        if let Some(csp) = &self.config.security_headers.content_security_policy {
            headers.insert("Content-Security-Policy", csp.parse().unwrap());
        }

        if let Some(hsts) = &self.config.security_headers.strict_transport_security {
            headers.insert("Strict-Transport-Security", hsts.parse().unwrap());
        }

        if let Some(frame_options) = &self.config.security_headers.x_frame_options {
            headers.insert("X-Frame-Options", frame_options.parse().unwrap());
        }

        if let Some(content_type_options) = &self.config.security_headers.x_content_type_options {
            headers.insert(
                "X-Content-Type-Options",
                content_type_options.parse().unwrap(),
            );
        }

        if let Some(referrer_policy) = &self.config.security_headers.referrer_policy {
            headers.insert("Referrer-Policy", referrer_policy.parse().unwrap());
        }

        if let Some(xss_protection) = &self.config.security_headers.x_xss_protection {
            headers.insert("X-XSS-Protection", xss_protection.parse().unwrap());
        }

        // Add custom headers
        for (key, value) in &self.config.security_headers.custom_headers {
            if let Ok(header_value) = value.parse() {
                let header_name = HeaderName::from_bytes(key.as_bytes()).unwrap();
                headers.insert(header_name, header_value);
            }
        }

        response
    }

    /// Get security statistics
    pub async fn get_stats(&self) -> SecurityStats {
        let stats = self.stats.read().await;
        SecurityStats {
            total_requests: stats.total_requests,
            authenticated_requests: stats.authenticated_requests,
            failed_authentications: stats.failed_authentications,
            authorization_denials: stats.authorization_denials,
            rate_limit_blocks: stats.rate_limit_blocks,
            validation_failures: stats.validation_failures,
            blocked_requests: stats.blocked_requests,
            last_reset: stats.last_reset,
        }
    }

    /// Reset security statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = SecurityStats {
            last_reset: Utc::now(),
            ..Default::default()
        };
    }

    // Helper methods
    fn extract_token_from_request(&self, req: &Request) -> Result<String, StatusCode> {
        let auth_header = req
            .headers()
            .get("Authorization")
            .ok_or(StatusCode::UNAUTHORIZED)?
            .to_str()
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        if auth_header.starts_with("Bearer ") {
            Ok(auth_header[7..].to_string())
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    fn extract_client_ip(&self, headers: &HeaderMap) -> Option<IpAddr> {
        // Try X-Forwarded-For first
        if let Some(xff) = headers.get("x-forwarded-for") {
            if let Ok(xff_str) = xff.to_str() {
                if let Some(first_ip) = xff_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse() {
                        return Some(ip);
                    }
                }
            }
        }

        // Try X-Real-IP
        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.parse() {
                    return Some(ip);
                }
            }
        }

        // Try CF-Connecting-IP (Cloudflare)
        if let Some(cf_ip) = headers.get("cf-connecting-ip") {
            if let Ok(ip_str) = cf_ip.to_str() {
                if let Ok(ip) = ip_str.parse() {
                    return Some(ip);
                }
            }
        }

        None
    }

    fn extract_user_agent(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_middleware_config() {
        let config = SecurityMiddlewareConfig::default();
        assert!(config.enable_authentication);
        assert!(config.enable_authorization);
        assert_eq!(config.rate_limit.requests_per_minute, 60);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn test_security_headers_config() {
        let config = SecurityHeadersConfig::default();
        assert!(config.content_security_policy.is_some());
        assert!(config.strict_transport_security.is_some());
    }

    #[test]
    fn test_input_validation_config() {
        let config = InputValidationConfig::default();
        assert_eq!(config.max_request_size, 1024 * 1024);
        assert_eq!(config.max_header_count, 100);
    }
}
