//! Rate Limiting Module
//!
//! Provides rate limiting capabilities for API endpoints and users.

use crate::errors::SecurityResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Requests per hour limit
    pub requests_per_hour: u32,
    /// Burst multiplier for temporary spikes
    pub burst_multiplier: f64,
    /// Enable per-user rate limiting
    pub per_user_limiting: bool,
    /// Enable per-IP rate limiting
    pub per_ip_limiting: bool,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            burst_multiplier: 1.5,
            per_user_limiting: true,
            per_ip_limiting: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Rate limit result
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request allowed
    Allowed,
    /// Rate limit exceeded
    Exceeded {
        /// Time until reset
        retry_after: Duration,
        /// Limit type that was exceeded
        limit_type: String,
    },
}

/// Rate limit entry
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Number of requests in current minute
    requests_this_minute: u32,
    /// Number of requests in current hour
    requests_this_hour: u32,
    /// Current minute window start
    minute_window_start: Instant,
    /// Current hour window start
    hour_window_start: Instant,
    /// Last request time
    last_request: Instant,
}

impl RateLimitEntry {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            requests_this_minute: 0,
            requests_this_hour: 0,
            minute_window_start: now,
            hour_window_start: now,
            last_request: now,
        }
    }

    fn update(&mut self, now: Instant) {
        // Reset minute window if needed
        if now.duration_since(self.minute_window_start) >= Duration::from_secs(60) {
            self.requests_this_minute = 0;
            self.minute_window_start = now;
        }

        // Reset hour window if needed
        if now.duration_since(self.hour_window_start) >= Duration::from_secs(3600) {
            self.requests_this_hour = 0;
            self.hour_window_start = now;
        }

        self.requests_this_minute += 1;
        self.requests_this_hour += 1;
        self.last_request = now;
    }

    fn check_limits(&self, config: &RateLimitConfig) -> RateLimitResult {
        let burst_limit_minute =
            (config.requests_per_minute as f64 * config.burst_multiplier) as u32;

        if self.requests_this_minute > burst_limit_minute {
            let retry_after =
                Duration::from_secs(60) - Instant::now().duration_since(self.minute_window_start);
            return RateLimitResult::Exceeded {
                retry_after,
                limit_type: "requests_per_minute".to_string(),
            };
        }

        if self.requests_this_hour > config.requests_per_hour {
            let retry_after =
                Duration::from_secs(3600) - Instant::now().duration_since(self.hour_window_start);
            return RateLimitResult::Exceeded {
                retry_after,
                limit_type: "requests_per_hour".to_string(),
            };
        }

        RateLimitResult::Allowed
    }

    fn is_expired(&self, cleanup_threshold: Duration) -> bool {
        Instant::now().duration_since(self.last_request) > cleanup_threshold
    }
}

/// In-memory rate limiter implementation
pub struct RateLimiter {
    config: RateLimitConfig,
    user_limits: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    ip_limits: Arc<RwLock<HashMap<IpAddr, RateLimitEntry>>>,
    endpoint_limits: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            user_limits: Arc::new(RwLock::new(HashMap::new())),
            ip_limits: Arc::new(RwLock::new(HashMap::new())),
            endpoint_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check rate limit for user
    pub async fn check_user_limit(&self, user_id: &str) -> SecurityResult<RateLimitResult> {
        if !self.config.per_user_limiting {
            return Ok(RateLimitResult::Allowed);
        }

        let mut limits = self.user_limits.write().await;
        let entry = limits
            .entry(user_id.to_string())
            .or_insert_with(RateLimitEntry::new);

        let now = Instant::now();
        let result = entry.check_limits(&self.config);

        match result {
            RateLimitResult::Allowed => {
                entry.update(now);
                Ok(RateLimitResult::Allowed)
            }
            exceeded => Ok(exceeded),
        }
    }

    /// Check rate limit for IP address
    pub async fn check_ip_limit(&self, ip: IpAddr) -> SecurityResult<RateLimitResult> {
        if !self.config.per_ip_limiting {
            return Ok(RateLimitResult::Allowed);
        }

        let mut limits = self.ip_limits.write().await;
        let entry = limits.entry(ip).or_insert_with(RateLimitEntry::new);

        let now = Instant::now();
        let result = entry.check_limits(&self.config);

        match result {
            RateLimitResult::Allowed => {
                entry.update(now);
                Ok(RateLimitResult::Allowed)
            }
            exceeded => Ok(exceeded),
        }
    }

    /// Check rate limit for endpoint
    pub async fn check_endpoint_limit(&self, endpoint: &str) -> SecurityResult<RateLimitResult> {
        let mut limits = self.endpoint_limits.write().await;
        let entry = limits
            .entry(endpoint.to_string())
            .or_insert_with(RateLimitEntry::new);

        let now = Instant::now();
        let result = entry.check_limits(&self.config);

        match result {
            RateLimitResult::Allowed => {
                entry.update(now);
                Ok(RateLimitResult::Allowed)
            }
            exceeded => Ok(exceeded),
        }
    }

    /// Get current stats for user
    pub async fn get_user_stats(&self, user_id: &str) -> Option<(u32, u32)> {
        let limits = self.user_limits.read().await;
        limits
            .get(user_id)
            .map(|entry| (entry.requests_this_minute, entry.requests_this_hour))
    }

    /// Get current stats for IP
    pub async fn get_ip_stats(&self, ip: IpAddr) -> Option<(u32, u32)> {
        let limits = self.ip_limits.read().await;
        limits
            .get(&ip)
            .map(|entry| (entry.requests_this_minute, entry.requests_this_hour))
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) -> SecurityResult<u32> {
        let cleanup_threshold = self.config.cleanup_interval * 2;
        let mut removed_count = 0;

        // Clean up user limits
        let mut user_limits = self.user_limits.write().await;
        let user_count = user_limits.len();
        user_limits.retain(|_, entry| !entry.is_expired(cleanup_threshold));
        removed_count += (user_count - user_limits.len()) as u32;

        // Clean up IP limits
        let mut ip_limits = self.ip_limits.write().await;
        let ip_count = ip_limits.len();
        ip_limits.retain(|_, entry| !entry.is_expired(cleanup_threshold));
        removed_count += (ip_count - ip_limits.len()) as u32;

        // Clean up endpoint limits
        let mut endpoint_limits = self.endpoint_limits.write().await;
        let endpoint_count = endpoint_limits.len();
        endpoint_limits.retain(|_, entry| !entry.is_expired(cleanup_threshold));
        removed_count += (endpoint_count - endpoint_limits.len()) as u32;

        Ok(removed_count)
    }

    /// Reset limits for user (for testing or admin purposes)
    pub async fn reset_user_limits(&self, user_id: &str) -> SecurityResult<()> {
        let mut limits = self.user_limits.write().await;
        limits.remove(user_id);
        Ok(())
    }

    /// Reset limits for IP (for testing or admin purposes)
    pub async fn reset_ip_limits(&self, ip: IpAddr) -> SecurityResult<()> {
        let mut limits = self.ip_limits.write().await;
        limits.remove(&ip);
        Ok(())
    }

    /// Get total number of tracked entries
    pub async fn get_stats(&self) -> (usize, usize, usize) {
        let user_count = self.user_limits.read().await.len();
        let ip_count = self.ip_limits.read().await.len();
        let endpoint_count = self.endpoint_limits.read().await.len();
        (user_count, ip_count, endpoint_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_user_limits() {
        let mut config = RateLimitConfig::default();
        config.requests_per_minute = 2;
        config.requests_per_hour = 10;

        let limiter = RateLimiter::new(config);
        let user_id = "test_user";

        // First request should be allowed
        let result = limiter.check_user_limit(user_id).await.unwrap();
        assert!(matches!(result, RateLimitResult::Allowed));

        // Second request should be allowed
        let result = limiter.check_user_limit(user_id).await.unwrap();
        assert!(matches!(result, RateLimitResult::Allowed));

        // Third request should exceed limit
        let result = limiter.check_user_limit(user_id).await.unwrap();
        assert!(matches!(result, RateLimitResult::Exceeded { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiter_ip_limits() {
        let mut config = RateLimitConfig::default();
        config.requests_per_minute = 2;

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // First request should be allowed
        let result = limiter.check_ip_limit(ip).await.unwrap();
        assert!(matches!(result, RateLimitResult::Allowed));

        // Second request should be allowed
        let result = limiter.check_ip_limit(ip).await.unwrap();
        assert!(matches!(result, RateLimitResult::Allowed));

        // Third request should exceed limit
        let result = limiter.check_ip_limit(ip).await.unwrap();
        assert!(matches!(result, RateLimitResult::Exceeded { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiter_stats() {
        let limiter = RateLimiter::with_defaults();
        let user_id = "test_user";

        // Make some requests
        limiter.check_user_limit(user_id).await.unwrap();
        limiter.check_user_limit(user_id).await.unwrap();

        // Check stats
        let stats = limiter.get_user_stats(user_id).await;
        assert!(stats.is_some());
        let (minute_count, hour_count) = stats.unwrap();
        assert_eq!(minute_count, 2);
        assert_eq!(hour_count, 2);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let mut config = RateLimitConfig::default();
        config.cleanup_interval = Duration::from_millis(100);

        let limiter = RateLimiter::new(config);
        let user_id = "test_user";

        // Make a request
        limiter.check_user_limit(user_id).await.unwrap();

        // Wait for expiration
        sleep(Duration::from_millis(250)).await;

        // Cleanup
        let removed = limiter.cleanup_expired().await.unwrap();
        assert_eq!(removed, 1);

        // Stats should be empty
        let (user_count, _, _) = limiter.get_stats().await;
        assert_eq!(user_count, 0);
    }

    #[tokio::test]
    async fn test_reset_limits() {
        let limiter = RateLimiter::with_defaults();
        let user_id = "test_user";
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // Make requests
        limiter.check_user_limit(user_id).await.unwrap();
        limiter.check_ip_limit(ip).await.unwrap();

        // Reset
        limiter.reset_user_limits(user_id).await.unwrap();
        limiter.reset_ip_limits(ip).await.unwrap();

        // Stats should be empty
        let stats = limiter.get_user_stats(user_id).await;
        assert!(stats.is_none());

        let stats = limiter.get_ip_stats(ip).await;
        assert!(stats.is_none());
    }
}
