//! Rate limiting middleware using Redis-backed sliding window algorithm

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use crate::services::rate_limiter::RateLimitResult;
use crate::{
    error::{ApiError, Result},
    middleware_layer::auth::{extract_user_context, UserContext},
    state::AppState,
};
use ai_core_shared::types::core::SubscriptionTier;

/// Rate limiting middleware using sliding window algorithm
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response> {
    // Extract user context if available
    let user_context = extract_user_context(&request);

    // Determine rate limit key and limits
    let (limit_key, limits) = get_rate_limit_info(&request, user_context)?;

    // Check rate limit
    // Get rate limiter service
    let rate_limiter = state
        .rate_limiter
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("rate_limiter"))?;

    let rate_limit_result = rate_limiter
        .check_rate_limit(&limit_key, limits.per_minute, Duration::from_secs(60))
        .await?;

    debug!(
        key = %limit_key,
        allowed = rate_limit_result.allowed,
        remaining = rate_limit_result.remaining,
        limit = rate_limit_result.limit,
        "Rate limit check completed"
    );

    // Record metrics
    if !rate_limit_result.allowed {
        let user_id = user_context
            .map(|ctx| ctx.user_id.as_str())
            .unwrap_or("anonymous");
        let tier = user_context
            .map(|ctx| ctx.subscription_tier().to_string())
            .unwrap_or_else(|| "free".to_string());

        state.metrics.record_rate_limit_hit(user_id, &tier);

        warn!(
            key = %limit_key,
            user_id = user_id,
            tier = tier,
            "Rate limit exceeded"
        );
    }

    // Continue with request if allowed
    let mut response = if rate_limit_result.allowed {
        next.run(request).await
    } else {
        return Err(ApiError::rate_limit(format!(
            "Rate limit exceeded. Try again in {} seconds",
            rate_limit_result.retry_after.unwrap_or(60)
        )));
    };

    // Add rate limit headers to response
    add_rate_limit_headers(response.headers_mut(), &rate_limit_result)?;

    Ok(response)
}

/// Rate limit configuration for different subscription tiers
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub per_minute: u32,
    pub per_hour: u32,
    pub burst_multiplier: f64,
}

impl RateLimitConfig {
    /// Get rate limit configuration for subscription tier
    pub fn for_subscription_tier(tier: &SubscriptionTier) -> Self {
        match tier {
            SubscriptionTier::Free => Self {
                per_minute: 10,
                per_hour: 100,
                burst_multiplier: 1.2,
            },
            SubscriptionTier::Pro => Self {
                per_minute: 100,
                per_hour: 2000,
                burst_multiplier: 1.5,
            },
            SubscriptionTier::Enterprise => Self {
                per_minute: 500,
                per_hour: 10000,
                burst_multiplier: 2.0,
            },
        }
    }

    /// Get default rate limits for anonymous users
    pub fn default() -> Self {
        Self {
            per_minute: 5,
            per_hour: 50,
            burst_multiplier: 1.0,
        }
    }

    /// Calculate burst limit
    pub fn burst_limit(&self) -> u32 {
        (self.per_minute as f64 * self.burst_multiplier) as u32
    }
}

/// Determine rate limit key and configuration
fn get_rate_limit_info(
    request: &Request,
    user_context: Option<&UserContext>,
) -> Result<(String, RateLimitConfig)> {
    match user_context {
        Some(ctx) => {
            // Authenticated user - use user ID
            let key = format!("rate_limit:user:{}", ctx.user_id);
            let limits = RateLimitConfig::for_subscription_tier(&ctx.subscription_tier);
            Ok((key, limits))
        }
        None => {
            // Anonymous user - use IP address
            let ip = extract_client_ip(request)?;
            let key = format!("rate_limit:ip:{}", ip);
            let limits = RateLimitConfig::default();
            Ok((key, limits))
        }
    }
}

/// Extract client IP address from request
fn extract_client_ip(request: &Request<Body>) -> Result<String> {
    // Check X-Forwarded-For header first (for proxy/load balancer)
    if let Some(forwarded_for) = request.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // Take the first IP in the chain
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return Ok(first_ip.trim().to_string());
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Ok(ip_str.to_string());
        }
    }

    // Fallback to connection info (may not be available in middleware)
    Ok("unknown".to_string())
}

/// Add rate limit headers to response
fn add_rate_limit_headers(
    headers: &mut HeaderMap,
    rate_limit_result: &crate::services::rate_limiter::RateLimitResult,
) -> Result<()> {
    // X-RateLimit-Limit: Request limit per window
    headers.insert(
        HeaderName::from_static("x-ratelimit-limit"),
        HeaderValue::from_str(&rate_limit_result.limit.to_string())
            .map_err(|e| ApiError::internal(format!("Invalid header value: {}", e)))?,
    );

    // X-RateLimit-Remaining: Requests remaining in current window
    headers.insert(
        HeaderName::from_static("x-ratelimit-remaining"),
        HeaderValue::from_str(&rate_limit_result.remaining.to_string())
            .map_err(|e| ApiError::internal(format!("Invalid header value: {}", e)))?,
    );

    // X-RateLimit-Reset: Unix timestamp when window resets
    let reset_timestamp = rate_limit_result
        .reset_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();
    headers.insert(
        HeaderName::from_static("x-ratelimit-reset"),
        HeaderValue::from_str(&reset_timestamp.to_string())
            .map_err(|e| ApiError::internal(format!("Invalid header value: {}", e)))?,
    );

    // Retry-After header if rate limited
    if let Some(retry_after) = rate_limit_result.retry_after {
        headers.insert(
            HeaderName::from_static("retry-after"),
            HeaderValue::from_str(&retry_after.to_string())
                .map_err(|e| ApiError::internal(format!("Invalid header value: {}", e)))?,
        );
    }

    Ok(())
}

/// Endpoint-specific rate limiting middleware
pub fn endpoint_rate_limit_middleware(
    endpoint: String,
    per_minute_limit: u32,
) -> impl Fn(
    State<AppState>,
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send>>
       + Send
       + Sync {
    move |State(state): State<AppState>, request: Request, next: Next| {
        let endpoint = endpoint.clone();
        Box::pin(async move {
            // Extract user context if available
            let user_context = extract_user_context(&request);

            // Create endpoint-specific rate limit key
            let limit_key = match user_context {
                Some(ctx) => format!("rate_limit:endpoint:{}:user:{}", endpoint, ctx.user_id),
                None => {
                    let ip = extract_client_ip(&request)?;
                    format!("rate_limit:endpoint:{}:ip:{}", endpoint, ip)
                }
            };

            // Get rate limiter service
            let rate_limiter = state
                .rate_limiter
                .as_ref()
                .ok_or_else(|| ApiError::service_unavailable("rate_limiter"))?;

            // Check rate limit
            let rate_limit_result = rate_limiter
                .check_rate_limit(&limit_key, per_minute_limit, Duration::from_secs(60))
                .await?;

            if !rate_limit_result.allowed {
                return Err(ApiError::rate_limit(format!(
                    "Rate limit exceeded for endpoint {}. Try again in {} seconds",
                    endpoint,
                    rate_limit_result.retry_after.unwrap_or(60)
                )));
            }

            // Continue with request
            let mut response = next.run(request).await;
            add_rate_limit_headers(response.headers_mut(), &rate_limit_result)?;

            Ok(response)
        })
    }
}

/// Cost-based rate limiting for expensive operations
pub fn cost_based_rate_limit_middleware(
    cost: u32,
) -> impl Fn(
    State<AppState>,
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send>>
       + Send
       + Sync {
    move |State(state): State<AppState>, request: Request, next: Next| {
        Box::pin(async move {
            let user_context = extract_user_context(&request);

            // Determine cost limit based on subscription tier
            let (limit_key, cost_limit) = match user_context {
                Some(ctx) => {
                    let key = format!("cost_limit:user:{}", ctx.user_id);
                    let limit = match ctx.subscription_tier {
                        SubscriptionTier::Free => 100,         // 100 cost units per hour
                        SubscriptionTier::Pro => 1000,         // 1000 cost units per hour
                        SubscriptionTier::Enterprise => 10000, // 10000 cost units per hour
                    };
                    (key, limit)
                }
                None => {
                    let ip = extract_client_ip(&request)?;
                    let key = format!("cost_limit:ip:{}", ip);
                    (key, 10) // Very limited for anonymous users
                }
            };

            // Get rate limiter service
            let rate_limiter = state
                .rate_limiter
                .as_ref()
                .ok_or_else(|| ApiError::service_unavailable("rate_limiter"))?;

            // Check if user has enough cost budget
            let rate_limit_result = rate_limiter
                .check_cost_limit(&limit_key, cost, cost_limit, Duration::from_secs(3600))
                .await?;

            if !rate_limit_result.allowed {
                return Err(ApiError::rate_limit(format!(
                    "Cost limit exceeded. Current operation costs {} units, {} remaining",
                    cost, rate_limit_result.remaining
                )));
            }

            // Continue with request
            let mut response = next.run(request).await;
            add_rate_limit_headers(response.headers_mut(), &rate_limit_result)?;

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_core_shared::types::core::{Permission, TokenClaims};
    use axum::body::Body;
    use std::collections::HashSet;

    #[test]
    fn test_rate_limit_config_for_tiers() {
        let free_config = RateLimitConfig::for_subscription_tier(&SubscriptionTier::Free);
        assert_eq!(free_config.per_minute, 10);
        assert_eq!(free_config.per_hour, 100);

        let pro_config = RateLimitConfig::for_subscription_tier(&SubscriptionTier::Pro);
        assert_eq!(pro_config.per_minute, 100);
        assert_eq!(pro_config.per_hour, 2000);

        let enterprise_config =
            RateLimitConfig::for_subscription_tier(&SubscriptionTier::Enterprise);
        assert_eq!(enterprise_config.per_minute, 500);
        assert_eq!(enterprise_config.per_hour, 10000);
    }

    #[test]
    fn test_burst_limit_calculation() {
        let config = RateLimitConfig {
            per_minute: 100,
            per_hour: 1000,
            burst_multiplier: 1.5,
        };

        assert_eq!(config.burst_limit(), 150);
    }

    #[test]
    fn test_default_rate_limit_config() {
        let default_config = RateLimitConfig::default();
        assert_eq!(default_config.per_minute, 5);
        assert_eq!(default_config.per_hour, 50);
        assert_eq!(default_config.burst_multiplier, 1.0);
    }

    #[tokio::test]
    async fn test_extract_client_ip() {
        use axum::http::{HeaderValue, Request};

        // Test with X-Forwarded-For header
        let mut request: Request<Body> =
            Request::builder().uri("/test").body(Body::empty()).unwrap();

        request.headers_mut().insert(
            "X-Forwarded-For",
            HeaderValue::from_static("192.168.1.1, 10.0.0.1"),
        );

        let ip = extract_client_ip(&request).unwrap();
        assert_eq!(ip, "192.168.1.1");

        // Test with X-Real-IP header
        let mut request: Request<Body> =
            Request::builder().uri("/test").body(Body::empty()).unwrap();

        request
            .headers_mut()
            .insert("X-Real-IP", HeaderValue::from_static("10.0.0.1"));

        let ip = extract_client_ip(&request).unwrap();
        assert_eq!(ip, "10.0.0.1");

        // Test fallback
        let request: Request<Body> = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let ip = extract_client_ip(&request).unwrap();
        assert_eq!(ip, "unknown");
    }
}
