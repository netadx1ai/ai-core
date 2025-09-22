//! Rate Limiting Middleware
//!
//! This module provides rate limiting middleware for the MCP Manager Service,
//! implementing configurable rate limits per IP address and globally.

use crate::server::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::net::IpAddr;
use tracing::debug;

/// Rate limiting middleware
///
/// Applies rate limiting based on the service configuration.
/// Currently returns a stub implementation.
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip rate limiting if disabled
    if !state.config.rate_limiting.enabled {
        return Ok(next.run(request).await);
    }

    // Extract client IP
    let client_ip = extract_client_ip(&request);

    debug!(
        client_ip = ?client_ip,
        "Rate limiting check"
    );

    // TODO: Implement actual rate limiting logic
    // This would typically involve:
    // - Checking request count for client IP
    // - Checking global request rate
    // - Using a token bucket or sliding window algorithm
    // - Storing rate limit data in Redis or in-memory store

    // For now, just log and pass through
    debug!("Rate limiting check passed (stub implementation)");

    Ok(next.run(request).await)
}

/// Extract client IP address from request
fn extract_client_ip<T>(request: &Request<T>) -> Option<IpAddr> {
    // Try to get IP from X-Forwarded-For header first
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // Take the first IP in the chain
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    // TODO: Extract from connection info when available
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn test_extract_client_ip_from_forwarded_for() {
        let request = Request::builder()
            .header("x-forwarded-for", "192.168.1.1, 10.0.0.1")
            .body(())
            .unwrap();

        let ip = extract_client_ip(&request);
        assert_eq!(ip, Some("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn test_extract_client_ip_from_real_ip() {
        let request = Request::builder()
            .header("x-real-ip", "10.0.0.1")
            .body(())
            .unwrap();

        let ip = extract_client_ip(&request);
        assert_eq!(ip, Some("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_extract_client_ip_none() {
        let request = Request::builder().body(()).unwrap();

        let ip = extract_client_ip(&request);
        assert_eq!(ip, None);
    }
}
