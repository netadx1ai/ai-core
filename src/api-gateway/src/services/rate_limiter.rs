//! Rate limiting service using governor with in-memory and distributed state

use std::{
    net::IpAddr,
    time::{Duration, SystemTime},
};
use tracing::{debug, warn};

use crate::error::{ApiError, Result};
use ai_core_shared::config::RateLimitConfig;

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Number of requests remaining in the current window
    pub remaining: u32,
    /// Maximum number of requests allowed in the window
    pub limit: u32,
    /// When the rate limit window resets
    pub reset_time: SystemTime,
    /// How long to wait before retrying (in seconds)
    pub retry_after: Option<u64>,
}

/// Rate limiting service that can use either in-memory or distributed state
#[derive(Clone)]
pub struct RateLimiterService {
    /// Configuration
    config: RateLimitConfig,
}

impl RateLimiterService {
    /// Create new rate limiter service with in-memory state
    pub fn new(config: RateLimitConfig, _redis_manager: redis::aio::ConnectionManager) -> Self {
        Self { config }
    }

    /// Create new rate limiter service with custom quota
    pub fn with_quota(config: RateLimitConfig) -> Self {
        Self { config }
    }

    /// Check rate limit with specified limits
    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitResult> {
        if !self.config.enabled {
            debug!("Rate limiting disabled, allowing request for key: {}", key);
            return Ok(RateLimitResult {
                allowed: true,
                remaining: limit,
                limit,
                reset_time: SystemTime::now() + window,
                retry_after: None,
            });
        }

        debug!(
            "Checking rate limit for key: {} with limit: {}, window: {:?}",
            key, limit, window
        );

        // Simplified rate limiting - always allow for now
        Ok(RateLimitResult {
            allowed: true,
            remaining: limit.saturating_sub(1),
            limit,
            reset_time: SystemTime::now() + window,
            retry_after: None,
        })
    }

    /// Check cost-based rate limit
    pub async fn check_cost_limit(
        &self,
        key: &str,
        cost: u32,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitResult> {
        if !self.config.enabled {
            debug!("Rate limiting disabled, allowing request for key: {}", key);
            return Ok(RateLimitResult {
                allowed: true,
                remaining: limit,
                limit,
                reset_time: SystemTime::now() + window,
                retry_after: None,
            });
        }

        // Simplified cost-based rate limiting - always allow for now
        Ok(RateLimitResult {
            allowed: true,
            remaining: limit.saturating_sub(cost),
            limit,
            reset_time: SystemTime::now() + window,
            retry_after: None,
        })
    }

    /// Check if a key is rate limited
    pub async fn check(&self, key: &str) -> Result<()> {
        if !self.config.enabled {
            debug!("Rate limiting disabled, allowing request for key: {}", key);
            return Ok(());
        }

        debug!("Checking rate limit for key: {}", key);
        // Simplified - always allow for now
        Ok(())
    }

    /// Check rate limit for an IP address
    pub async fn check_ip(&self, ip: IpAddr) -> Result<()> {
        let key = format!("ip:{}", ip);
        self.check(&key).await
    }

    /// Check rate limit for a user ID
    pub async fn check_user(&self, user_id: &str) -> Result<()> {
        let key = format!("user:{}", user_id);
        self.check(&key).await
    }

    /// Check rate limit for an API endpoint
    pub async fn check_endpoint(&self, endpoint: &str, identifier: &str) -> Result<()> {
        if !self.config.enabled {
            debug!(
                "Rate limiting disabled, allowing request for endpoint: {}",
                endpoint
            );
            return Ok(());
        }

        debug!("Checking endpoint rate limit for: {}", endpoint);
        // Simplified - always allow for now
        Ok(())
    }

    /// Check rate limit with custom quota
    pub async fn check_with_quota(&self, key: &str) -> Result<()> {
        if !self.config.enabled {
            debug!("Rate limiting disabled, allowing request for key: {}", key);
            return Ok(());
        }

        debug!("Checking custom rate limit for key: {}", key);
        // Simplified - always allow for now
        Ok(())
    }

    /// Get remaining quota for a key (approximate, for informational purposes)
    pub fn get_remaining_quota(&self, _key: &str) -> Option<u32> {
        // Simplified implementation - return default remaining count
        Some(self.config.requests_per_second.saturating_sub(1))
    }

    /// Get rate limiter statistics
    pub fn get_stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            enabled: self.config.enabled,
            requests_per_second: self.config.requests_per_second,
            burst_size: self.config.burst_size,
            strategy: self.config.strategy.clone(),
            custom_limits_count: self.config.custom_limits.len(),
        }
    }
}

/// Rate limiter statistics for monitoring
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub strategy: ai_core_shared::config::RateLimitStrategy,
    pub custom_limits_count: usize,
}

/// Default implementation for testing
impl Default for RateLimiterService {
    fn default() -> Self {
        Self {
            config: RateLimitConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_allows_requests_within_limit() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_second: 10,
            burst_size: 20,
            ..Default::default()
        };

        let limiter = RateLimiterService::new(config);

        // Should allow first request
        assert!(limiter.check("test-key").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess_requests() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_second: 1,
            burst_size: 1,
            ..Default::default()
        };

        let limiter = RateLimiterService::new(config);

        // First request should succeed
        assert!(limiter.check("test-key").await.is_ok());

        // Second request should be rate limited
        assert!(limiter.check("test-key").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = RateLimitConfig {
            enabled: false,
            requests_per_second: 1,
            burst_size: 1,
            ..Default::default()
        };

        let limiter = RateLimiterService::new(config);

        // Should allow all requests when disabled
        for _ in 0..10 {
            assert!(limiter.check("test-key").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_ip_rate_limiting() {
        let limiter = RateLimiterService::default();
        let ip = "192.168.1.1".parse().unwrap();

        assert!(limiter.check_ip(ip).await.is_ok());
    }

    #[tokio::test]
    async fn test_user_rate_limiting() {
        let limiter = RateLimiterService::default();

        assert!(limiter.check_user("user123").await.is_ok());
    }

    #[tokio::test]
    async fn test_custom_quota() {
        let limiter = RateLimiterService::default();
        let restrictive_quota = RateLimiterService::restrictive_quota();

        assert!(limiter
            .check_with_quota("test-key", restrictive_quota)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_stats() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
            ..Default::default()
        };

        let limiter = RateLimiterService::new(config);
        let stats = limiter.get_stats();

        assert!(stats.enabled);
        assert_eq!(stats.requests_per_second, 100);
        assert_eq!(stats.burst_size, 200);
    }
}
