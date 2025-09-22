//! Security Middleware Module
//!
//! Provides comprehensive security middleware for API protection including authentication,
//! authorization, rate limiting, input validation, and security headers.

use crate::errors::SecurityResult;
use crate::jwt::{JwtService, JwtServiceTrait, ValidationResult};
use crate::rate_limiting::{RateLimitConfig, RateLimiter};
use crate::rbac::{AuthorizationContext, RbacService, RequestMetadata};
use ai_core_shared::types::User;
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
};
use std::{collections::HashMap, net::IpAddr, str::FromStr, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use tower::{Layer, Service};
use uuid::Uuid;

/// Security middleware configuration
#[derive(Debug, Clone)]
pub struct SecurityMiddlewareConfig {
    /// Enable authentication middleware
    pub enable_authentication: bool,
    /// Enable authorization middleware
    pub enable_authorization: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Enable security headers
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
            rate_limit: RateLimitConfig {
                requests_per_minute: 60,
                requests_per_hour: 3600,
                burst_multiplier: 1.5,
                per_user_limiting: true,
                per_ip_limiting: true,
                cleanup_interval: std::time::Duration::from_secs(300),
            },
            security_headers: SecurityHeadersConfig::default(),
            input_validation: InputValidationConfig::default(),
        }
    }
}

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub content_security_policy: String,
    /// Strict Transport Security
    pub strict_transport_security: String,
    /// X-Frame-Options
    pub x_frame_options: String,
    /// X-Content-Type-Options
    pub x_content_type_options: String,
    /// Referrer Policy
    pub referrer_policy: String,
    /// X-XSS-Protection
    pub x_xss_protection: String,
    /// Custom headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_security_policy: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".to_string(),
            strict_transport_security: "max-age=31536000; includeSubDomains; preload".to_string(),
            x_frame_options: "DENY".to_string(),
            x_content_type_options: "nosniff".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            x_xss_protection: "1; mode=block".to_string(),
            custom_headers: HashMap::new(),
        }
    }
}

/// Input validation configuration
#[derive(Debug, Clone)]
pub struct InputValidationConfig {
    /// Maximum request size (bytes)
    pub max_request_size: usize,
    /// Maximum header count
    pub max_header_count: usize,
    /// Maximum header value length
    pub max_header_value_length: usize,
    /// Blocked user agents
    pub blocked_user_agents: Vec<String>,
    /// Allowed origins for CORS
    pub allowed_origins: Vec<String>,
}

impl Default for InputValidationConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10 MB
            max_header_count: 100,
            max_header_value_length: 8192,
            blocked_user_agents: vec![
                "curl".to_string(),
                "wget".to_string(),
                "python-requests".to_string(),
            ],
            allowed_origins: vec!["*".to_string()],
        }
    }
}

/// Main security middleware
pub struct SecurityMiddleware {
    jwt_service: Arc<JwtService>,
    rbac_service: Arc<RbacService>,
    config: SecurityMiddlewareConfig,
    rate_limiter: Arc<RateLimiter>,
    request_stats: Arc<RwLock<SecurityStats>>,
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    pub total_requests: u64,
    pub authenticated_requests: u64,
    pub failed_authentications: u64,
    pub authorization_denials: u64,
    pub rate_limit_blocks: u64,
    pub validation_failures: u64,
    pub blocked_requests: u64,
    pub last_reset: Instant,
}

impl Default for SecurityStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            authenticated_requests: 0,
            failed_authentications: 0,
            authorization_denials: 0,
            rate_limit_blocks: 0,
            validation_failures: 0,
            blocked_requests: 0,
            last_reset: Instant::now(),
        }
    }
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthenticationResult {
    pub user_id: Uuid,
    pub user: Option<User>,
    pub validation_result: ValidationResult,
}

/// Request context for security operations
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub request_id: String,
    pub client_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub authentication: Option<AuthenticationResult>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SecurityMiddleware {
    /// Create a new security middleware
    pub async fn new(
        jwt_service: Arc<JwtService>,
        rbac_service: Arc<RbacService>,
        config: SecurityMiddlewareConfig,
    ) -> SecurityResult<Self> {
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));

        Ok(Self {
            jwt_service,
            rbac_service,
            config,
            rate_limiter,
            request_stats: Arc::new(RwLock::new(SecurityStats {
                last_reset: Instant::now(),
                ..Default::default()
            })),
        })
    }

    /// Authentication middleware
    pub async fn authenticate_request(
        &self,
        mut request: Request,
    ) -> Result<(Request, Option<AuthenticationResult>), StatusCode> {
        if !self.config.enable_authentication {
            return Ok((request, None));
        }

        // Extract authorization header
        let auth_header = request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok());

        if let Some(auth_header) = auth_header {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                match JwtServiceTrait::validate_access_token(&*self.jwt_service, token).await {
                    Ok(validation_result) => {
                        let auth_result = AuthenticationResult {
                            user_id: validation_result.user_id,
                            user: None, // Would be fetched from database in production
                            validation_result,
                        };

                        // Add authentication context to request extensions
                        request.extensions_mut().insert(auth_result.clone());

                        // Update stats
                        let mut stats = self.request_stats.write().await;
                        stats.authenticated_requests += 1;

                        return Ok((request, Some(auth_result)));
                    }
                    Err(e) => {
                        tracing::warn!("Authentication failed: {}", e);

                        // Update stats
                        let mut stats = self.request_stats.write().await;
                        stats.failed_authentications += 1;

                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
            }
        }

        // No authentication provided for protected endpoint
        Err(StatusCode::UNAUTHORIZED)
    }

    /// Authorization middleware
    pub async fn authorize_request(
        &self,
        request: &Request,
        auth_result: &AuthenticationResult,
        resource: &str,
        action: &str,
    ) -> Result<(), StatusCode> {
        if !self.config.enable_authorization {
            return Ok(());
        }

        let context = AuthorizationContext {
            user_id: auth_result.user_id,
            resource: resource.to_string(),
            action: action.to_string(),
            attributes: HashMap::new(),
            request_metadata: RequestMetadata {
                client_ip: self
                    .extract_client_ip(request.headers())
                    .map(|ip| ip.to_string()),
                user_agent: self.extract_user_agent(request.headers()),
                timestamp: chrono::Utc::now(),
                request_id: Some(Uuid::new_v4().to_string()),
                geolocation: None,
            },
        };

        match self.rbac_service.authorize(&context).await {
            Ok(decision) => {
                if decision.allowed {
                    Ok(())
                } else {
                    tracing::warn!(
                        "Authorization denied for user {} on {}:{}",
                        auth_result.user_id,
                        resource,
                        action
                    );

                    // Update stats
                    let mut stats = self.request_stats.write().await;
                    stats.authorization_denials += 1;

                    Err(StatusCode::FORBIDDEN)
                }
            }
            Err(e) => {
                tracing::error!("Authorization error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Rate limiting middleware
    pub async fn rate_limit_request(&self, request: &Request) -> Result<(), StatusCode> {
        if !self.config.enable_rate_limiting {
            return Ok(());
        }

        let client_ip = self.extract_client_ip(request.headers());
        let path = request.uri().path();

        // Check endpoint-specific rate limits
        if let Err(_) = self.rate_limiter.check_endpoint_limit(path).await {
            let mut stats = self.request_stats.write().await;
            stats.rate_limit_blocks += 1;
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        // Per-IP rate limiting
        if self.config.rate_limit.per_ip_limiting {
            if let Some(ip) = client_ip {
                if let Err(_) = self.rate_limiter.check_ip_limit(ip).await {
                    let mut stats = self.request_stats.write().await;
                    stats.rate_limit_blocks += 1;
                    return Err(StatusCode::TOO_MANY_REQUESTS);
                }
            }
        }

        Ok(())
    }

    /// Input validation middleware
    pub async fn validate_input(&self, request: &Request) -> Result<(), StatusCode> {
        if !self.config.enable_input_validation {
            return Ok(());
        }

        // Check request size
        if let Some(content_length) = request.headers().get("content-length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    if length > self.config.input_validation.max_request_size {
                        tracing::warn!(
                            "Request size {} exceeds limit {}",
                            length,
                            self.config.input_validation.max_request_size
                        );

                        let mut stats = self.request_stats.write().await;
                        stats.validation_failures += 1;

                        return Err(StatusCode::PAYLOAD_TOO_LARGE);
                    }
                }
            }
        }

        // Check header count
        if request.headers().len() > self.config.input_validation.max_header_count {
            tracing::warn!("Too many headers: {}", request.headers().len());

            let mut stats = self.request_stats.write().await;
            stats.validation_failures += 1;

            return Err(StatusCode::BAD_REQUEST);
        }

        // Check header value lengths
        for (name, value) in request.headers() {
            if value.len() > self.config.input_validation.max_header_value_length {
                tracing::warn!("Header {} value too long: {}", name, value.len());

                let mut stats = self.request_stats.write().await;
                stats.validation_failures += 1;

                return Err(StatusCode::BAD_REQUEST);
            }
        }

        // Check user agent blacklist
        if let Some(user_agent) = self.extract_user_agent(request.headers()) {
            for blocked_agent in &self.config.input_validation.blocked_user_agents {
                if user_agent
                    .to_lowercase()
                    .contains(&blocked_agent.to_lowercase())
                {
                    tracing::warn!("Blocked user agent: {}", user_agent);

                    let mut stats = self.request_stats.write().await;
                    stats.blocked_requests += 1;

                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }

        Ok(())
    }

    /// Add security headers to response
    pub fn add_security_headers(&self, response: &mut Response) {
        if !self.config.enable_security_headers {
            return;
        }

        let headers = response.headers_mut();

        // Add standard security headers
        self.add_header(
            headers,
            "Content-Security-Policy",
            &self.config.security_headers.content_security_policy,
        );
        self.add_header(
            headers,
            "Strict-Transport-Security",
            &self.config.security_headers.strict_transport_security,
        );
        self.add_header(
            headers,
            "X-Frame-Options",
            &self.config.security_headers.x_frame_options,
        );
        self.add_header(
            headers,
            "X-Content-Type-Options",
            &self.config.security_headers.x_content_type_options,
        );
        self.add_header(
            headers,
            "Referrer-Policy",
            &self.config.security_headers.referrer_policy,
        );
        self.add_header(
            headers,
            "X-XSS-Protection",
            &self.config.security_headers.x_xss_protection,
        );

        // Add custom headers
        for (name, value) in &self.config.security_headers.custom_headers {
            self.add_header(headers, name, value);
        }

        // Add request ID for tracing
        let request_id = Uuid::new_v4().to_string();
        self.add_header(headers, "X-Request-ID", &request_id);
    }

    /// Extract client IP from headers
    fn extract_client_ip(&self, headers: &HeaderMap) -> Option<IpAddr> {
        // Try X-Forwarded-For first
        if let Some(xff) = headers.get("x-forwarded-for") {
            if let Ok(xff_str) = xff.to_str() {
                // Take the first IP (original client)
                if let Some(first_ip) = xff_str.split(',').next() {
                    if let Ok(ip) = IpAddr::from_str(first_ip.trim()) {
                        return Some(ip);
                    }
                }
            }
        }

        // Try X-Real-IP
        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = IpAddr::from_str(ip_str) {
                    return Some(ip);
                }
            }
        }

        None
    }

    /// Extract user agent from headers
    fn extract_user_agent(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }

    /// Add header to response
    fn add_header(&self, headers: &mut HeaderMap, name: &str, value: &str) {
        if let (Ok(header_name), Ok(header_value)) =
            (HeaderName::try_from(name), HeaderValue::try_from(value))
        {
            headers.insert(header_name, header_value);
        }
    }

    /// Get security statistics
    pub async fn get_stats(&self) -> SecurityStats {
        self.request_stats.read().await.clone()
    }

    /// Reset security statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.request_stats.write().await;
        *stats = SecurityStats {
            last_reset: Instant::now(),
            ..Default::default()
        };
    }
}

/// Authentication layer for Axum
pub struct AuthenticationLayer {
    middleware: Arc<SecurityMiddleware>,
}

impl AuthenticationLayer {
    pub fn new(middleware: Arc<SecurityMiddleware>) -> Self {
        Self { middleware }
    }
}

impl<S> Layer<S> for AuthenticationLayer {
    type Service = AuthenticationMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthenticationMiddleware {
            inner,
            middleware: self.middleware.clone(),
        }
    }
}

/// Authentication middleware service
pub struct AuthenticationMiddleware<S> {
    inner: S,
    middleware: Arc<SecurityMiddleware>,
}

impl<S> Service<Request> for AuthenticationMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let middleware = self.middleware.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Update total request stats
            {
                let mut stats = middleware.request_stats.write().await;
                stats.total_requests += 1;
            }

            // Validate input
            if let Err(status) = middleware.validate_input(&req).await {
                let mut response = Response::builder()
                    .status(status)
                    .body(axum::body::Body::empty())
                    .unwrap();
                middleware.add_security_headers(&mut response);
                return Ok(response);
            }

            // Authenticate request
            let (req, auth_result) = match middleware.authenticate_request(req).await {
                Ok((req, auth)) => (req, auth),
                Err(status) => {
                    let mut response = Response::builder()
                        .status(status)
                        .body(axum::body::Body::empty())
                        .unwrap();
                    middleware.add_security_headers(&mut response);
                    return Ok(response);
                }
            };

            // Rate limiting
            if let Err(status) = middleware.rate_limit_request(&req).await {
                let mut response = Response::builder()
                    .status(status)
                    .body(axum::body::Body::empty())
                    .unwrap();
                middleware.add_security_headers(&mut response);
                return Ok(response);
            }

            // Call inner service
            let mut response = inner.call(req).await?;

            // Add security headers
            middleware.add_security_headers(&mut response);

            Ok(response)
        })
    }
}

/// Authorization layer for Axum
pub struct AuthorizationLayer {
    middleware: Arc<SecurityMiddleware>,
    resource: String,
    action: String,
}

impl AuthorizationLayer {
    pub fn new(middleware: Arc<SecurityMiddleware>, resource: String, action: String) -> Self {
        Self {
            middleware,
            resource,
            action,
        }
    }
}

impl<S> Layer<S> for AuthorizationLayer {
    type Service = AuthorizationMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizationMiddleware {
            inner,
            middleware: self.middleware.clone(),
            resource: self.resource.clone(),
            action: self.action.clone(),
        }
    }
}

/// Authorization middleware service
pub struct AuthorizationMiddleware<S> {
    inner: S,
    middleware: Arc<SecurityMiddleware>,
    resource: String,
    action: String,
}

impl<S> Service<Request> for AuthorizationMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let middleware = self.middleware.clone();
        let resource = self.resource.clone();
        let action = self.action.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Check for authentication result in request extensions
            if let Some(auth_result) = req.extensions().get::<AuthenticationResult>() {
                if let Err(status) = middleware
                    .authorize_request(&req, auth_result, &resource, &action)
                    .await
                {
                    let mut response = Response::builder()
                        .status(status)
                        .body(axum::body::Body::empty())
                        .unwrap();
                    middleware.add_security_headers(&mut response);
                    return Ok(response);
                }
            } else {
                // No authentication found
                let mut response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(axum::body::Body::empty())
                    .unwrap();
                middleware.add_security_headers(&mut response);
                return Ok(response);
            }

            // Call inner service
            let mut response = inner.call(req).await?;

            // Add security headers
            middleware.add_security_headers(&mut response);

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_security_middleware_config() {
        let config = SecurityMiddlewareConfig::default();
        assert!(config.enable_authentication);
        assert!(config.enable_authorization);
        assert!(config.enable_rate_limiting);
        assert!(config.enable_input_validation);
        assert!(config.enable_security_headers);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.burst_multiplier, 1.5);
        assert!(config.per_user_limiting);
        assert!(config.per_ip_limiting);
    }

    #[test]
    fn test_security_headers_config() {
        let config = SecurityHeadersConfig::default();
        assert!(!config.content_security_policy.is_empty());
        assert!(!config.strict_transport_security.is_empty());
        assert_eq!(config.x_frame_options, "DENY");
        assert_eq!(config.x_content_type_options, "nosniff");
    }

    #[test]
    fn test_input_validation_config() {
        let config = InputValidationConfig::default();
        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
        assert_eq!(config.max_header_count, 100);
        assert_eq!(config.max_header_value_length, 8192);
        assert!(!config.blocked_user_agents.is_empty());
    }
}
