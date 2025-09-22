//! Rate limiting middleware for the Federation Service
//!
//! This module provides rate limiting middleware for protecting the federation service
//! from abuse and ensuring fair resource allocation across clients based on their tiers
//! and configured limits.

use crate::config::RateLimitingConfig;
use crate::models::FederationError;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitConfig {
    /// Requests per second limit
    pub requests_per_second: u32,
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Requests per hour limit
    pub requests_per_hour: u32,
    /// Concurrent requests limit
    pub concurrent_requests: u32,
    /// Window size for rate limiting
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            requests_per_minute: 600,
            requests_per_hour: 36000,
            concurrent_requests: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

/// Rate limiting middleware
#[derive(Debug, Clone)]
pub struct RateLimitMiddleware {
    /// Configuration
    config: RateLimitingConfig,
    /// Rate limit trackers by client ID
    client_trackers: Arc<DashMap<Uuid, Arc<RwLock<ClientRateTracker>>>>,
    /// Global rate tracker
    global_tracker: Arc<RwLock<GlobalRateTracker>>,
}

/// Client-specific rate tracking
#[derive(Debug, Clone)]
pub struct ClientRateTracker {
    /// Client ID
    pub client_id: Uuid,
    /// Current requests per second
    pub requests_per_second: u32,
    /// Current requests per minute
    pub requests_per_minute: u32,
    /// Current requests per hour
    pub requests_per_hour: u32,
    /// Current concurrent requests
    pub concurrent_requests: u32,
    /// Last second reset
    pub last_second_reset: DateTime<Utc>,
    /// Last minute reset
    pub last_minute_reset: DateTime<Utc>,
    /// Last hour reset
    pub last_hour_reset: DateTime<Utc>,
    /// Request timestamps for sliding window
    pub request_timestamps: Vec<DateTime<Utc>>,
    /// Rate limit configuration for this client
    pub config: RateLimitConfig,
}

/// Global rate tracking
#[derive(Debug, Clone, Default)]
pub struct GlobalRateTracker {
    /// Total requests per second
    pub total_requests_per_second: u32,
    /// Total requests per minute
    pub total_requests_per_minute: u32,
    /// Total requests per hour
    pub total_requests_per_hour: u32,
    /// Total concurrent requests
    pub total_concurrent_requests: u32,
    /// Last reset timestamps
    pub last_second_reset: Option<DateTime<Utc>>,
    pub last_minute_reset: Option<DateTime<Utc>>,
    pub last_hour_reset: Option<DateTime<Utc>>,
}

/// Rate limit violation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitViolation {
    /// Requests per second exceeded
    RequestsPerSecond,
    /// Requests per minute exceeded
    RequestsPerMinute,
    /// Requests per hour exceeded
    RequestsPerHour,
    /// Concurrent requests exceeded
    ConcurrentRequests,
    /// Global rate limit exceeded
    GlobalLimit,
}

/// Rate limit response
#[derive(Debug, Serialize)]
pub struct RateLimitResponse {
    /// Error message
    pub error: String,
    /// Violation type
    pub violation_type: RateLimitViolation,
    /// Current usage
    pub current_usage: u32,
    /// Limit value
    pub limit: u32,
    /// Retry after seconds
    pub retry_after: u64,
    /// Reset time
    pub reset_time: u64,
}

impl RateLimitMiddleware {
    /// Create new rate limiting middleware
    pub async fn new(config: &RateLimitingConfig) -> Result<Self, FederationError> {
        Ok(Self {
            config: config.clone(),
            client_trackers: Arc::new(DashMap::new()),
            global_tracker: Arc::new(RwLock::new(GlobalRateTracker::default())),
        })
    }

    /// Check if client can make request
    pub async fn check_rate_limit(&self, client_id: &Uuid) -> Result<bool, FederationError> {
        // Get or create client tracker
        let tracker = self.get_or_create_client_tracker(client_id).await?;
        let mut tracker_guard = tracker.write().await;

        let now = Utc::now();

        // Reset counters if time windows have passed
        self.reset_counters_if_needed(&mut tracker_guard, now);

        // Check global limits first
        if !self.check_global_limits().await? {
            return Ok(false);
        }

        // Check client-specific limits
        let config = &tracker_guard.config;

        // Check requests per second
        if tracker_guard.requests_per_second >= config.requests_per_second {
            return Ok(false);
        }

        // Check requests per minute
        if tracker_guard.requests_per_minute >= config.requests_per_minute {
            return Ok(false);
        }

        // Check requests per hour
        if tracker_guard.requests_per_hour >= config.requests_per_hour {
            return Ok(false);
        }

        // Check concurrent requests
        if tracker_guard.concurrent_requests >= config.concurrent_requests {
            return Ok(false);
        }

        // Increment counters
        tracker_guard.requests_per_second += 1;
        tracker_guard.requests_per_minute += 1;
        tracker_guard.requests_per_hour += 1;
        tracker_guard.concurrent_requests += 1;
        tracker_guard.request_timestamps.push(now);

        // Clean up old timestamps
        self.cleanup_old_timestamps(&mut tracker_guard, now);

        // Update global counters
        self.update_global_counters().await?;

        Ok(true)
    }

    /// Record request completion
    pub async fn record_request_completion(&self, client_id: &Uuid) -> Result<(), FederationError> {
        if let Some(tracker) = self.client_trackers.get(client_id) {
            let mut tracker_guard = tracker.write().await;
            if tracker_guard.concurrent_requests > 0 {
                tracker_guard.concurrent_requests -= 1;
            }
        }

        // Update global concurrent counter
        let mut global_tracker = self.global_tracker.write().await;
        if global_tracker.total_concurrent_requests > 0 {
            global_tracker.total_concurrent_requests -= 1;
        }

        Ok(())
    }

    /// Get client rate limit status
    pub async fn get_rate_limit_status(
        &self,
        client_id: &Uuid,
    ) -> Result<RateLimitStatus, FederationError> {
        let tracker = self.get_or_create_client_tracker(client_id).await?;
        let tracker_guard = tracker.read().await;

        Ok(RateLimitStatus {
            client_id: *client_id,
            requests_per_second: tracker_guard.requests_per_second,
            requests_per_minute: tracker_guard.requests_per_minute,
            requests_per_hour: tracker_guard.requests_per_hour,
            concurrent_requests: tracker_guard.concurrent_requests,
            limits: tracker_guard.config.clone(),
            reset_times: RateLimitResetTimes {
                second_reset: tracker_guard.last_second_reset + Duration::from_secs(1),
                minute_reset: tracker_guard.last_minute_reset + Duration::from_secs(60),
                hour_reset: tracker_guard.last_hour_reset + Duration::from_secs(3600),
            },
        })
    }

    // Private helper methods

    async fn get_or_create_client_tracker(
        &self,
        client_id: &Uuid,
    ) -> Result<Arc<RwLock<ClientRateTracker>>, FederationError> {
        if let Some(tracker) = self.client_trackers.get(client_id) {
            Ok(tracker.clone())
        } else {
            let now = Utc::now();
            let tracker = Arc::new(RwLock::new(ClientRateTracker {
                client_id: *client_id,
                requests_per_second: 0,
                requests_per_minute: 0,
                requests_per_hour: 0,
                concurrent_requests: 0,
                last_second_reset: now,
                last_minute_reset: now,
                last_hour_reset: now,
                request_timestamps: Vec::new(),
                config: RateLimitConfig::default(), // This would be loaded from client settings
            }));

            self.client_trackers.insert(*client_id, tracker.clone());
            Ok(tracker)
        }
    }

    fn reset_counters_if_needed(&self, tracker: &mut ClientRateTracker, now: DateTime<Utc>) {
        // Reset second counter
        if (now - tracker.last_second_reset).num_seconds() >= 1 {
            tracker.requests_per_second = 0;
            tracker.last_second_reset = now;
        }

        // Reset minute counter
        if (now - tracker.last_minute_reset).num_seconds() >= 60 {
            tracker.requests_per_minute = 0;
            tracker.last_minute_reset = now;
        }

        // Reset hour counter
        if (now - tracker.last_hour_reset).num_seconds() >= 3600 {
            tracker.requests_per_hour = 0;
            tracker.last_hour_reset = now;
        }
    }

    fn cleanup_old_timestamps(&self, tracker: &mut ClientRateTracker, now: DateTime<Utc>) {
        // Keep only timestamps from the last hour
        let hour_ago = now - chrono::Duration::seconds(3600);
        tracker.request_timestamps.retain(|&ts| ts > hour_ago);

        // Limit the number of stored timestamps
        if tracker.request_timestamps.len() > 1000 {
            tracker
                .request_timestamps
                .drain(..tracker.request_timestamps.len() - 1000);
        }
    }

    async fn check_global_limits(&self) -> Result<bool, FederationError> {
        let global_tracker = self.global_tracker.read().await;

        // Check global limits
        if global_tracker.total_requests_per_second >= self.config.global.requests_per_second {
            return Ok(false);
        }

        if global_tracker.total_requests_per_minute >= self.config.global.requests_per_minute {
            return Ok(false);
        }

        if global_tracker.total_requests_per_hour >= self.config.global.requests_per_hour {
            return Ok(false);
        }

        if global_tracker.total_concurrent_requests >= self.config.global.concurrent_requests {
            return Ok(false);
        }

        Ok(true)
    }

    async fn update_global_counters(&self) -> Result<(), FederationError> {
        let mut global_tracker = self.global_tracker.write().await;
        let now = Utc::now();

        // Initialize reset times if not set
        if global_tracker.last_second_reset.is_none() {
            global_tracker.last_second_reset = Some(now);
            global_tracker.last_minute_reset = Some(now);
            global_tracker.last_hour_reset = Some(now);
        }

        // Reset global counters if needed
        if let Some(last_second) = global_tracker.last_second_reset {
            if (now - last_second).num_seconds() >= 1 {
                global_tracker.total_requests_per_second = 0;
                global_tracker.last_second_reset = Some(now);
            }
        }

        if let Some(last_minute) = global_tracker.last_minute_reset {
            if (now - last_minute).num_seconds() >= 60 {
                global_tracker.total_requests_per_minute = 0;
                global_tracker.last_minute_reset = Some(now);
            }
        }

        if let Some(last_hour) = global_tracker.last_hour_reset {
            if (now - last_hour).num_seconds() >= 3600 {
                global_tracker.total_requests_per_hour = 0;
                global_tracker.last_hour_reset = Some(now);
            }
        }

        // Increment global counters
        global_tracker.total_requests_per_second += 1;
        global_tracker.total_requests_per_minute += 1;
        global_tracker.total_requests_per_hour += 1;
        global_tracker.total_concurrent_requests += 1;

        Ok(())
    }
}

/// Rate limit status information
#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    /// Client ID
    pub client_id: Uuid,
    /// Current requests per second
    pub requests_per_second: u32,
    /// Current requests per minute
    pub requests_per_minute: u32,
    /// Current requests per hour
    pub requests_per_hour: u32,
    /// Current concurrent requests
    pub concurrent_requests: u32,
    /// Rate limit configuration
    pub limits: RateLimitConfig,
    /// Reset times
    pub reset_times: RateLimitResetTimes,
}

/// Rate limit reset times
#[derive(Debug, Serialize)]
pub struct RateLimitResetTimes {
    /// Next second reset
    pub second_reset: DateTime<Utc>,
    /// Next minute reset
    pub minute_reset: DateTime<Utc>,
    /// Next hour reset
    pub hour_reset: DateTime<Utc>,
}

/// Rate limiting middleware function
pub async fn rate_limit_middleware(
    State(rate_limiter): State<RateLimitMiddleware>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip rate limiting for health endpoints
    let path = request.uri().path();
    if path.starts_with("/health") || path.starts_with("/metrics") {
        return Ok(next.run(request).await);
    }

    // Extract client ID from request (this would come from auth context)
    // For now, use a dummy client ID
    let client_id = Uuid::new_v4();

    // Check rate limits
    match rate_limiter.check_rate_limit(&client_id).await {
        Ok(true) => {
            // Process request
            let response = next.run(request).await;

            // Record request completion
            if let Err(e) = rate_limiter.record_request_completion(&client_id).await {
                tracing::error!("Failed to record request completion: {}", e);
            }

            Ok(response)
        }
        Ok(false) => {
            // Rate limit exceeded
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
        Err(e) => {
            tracing::error!("Rate limiting error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_middleware_creation() {
        let config = RateLimitingConfig::default();
        let middleware = RateLimitMiddleware::new(&config).await.unwrap();
        assert!(middleware.client_trackers.is_empty());
    }

    #[tokio::test]
    async fn test_rate_limit_check() {
        let config = RateLimitingConfig::default();
        let middleware = RateLimitMiddleware::new(&config).await.unwrap();
        let client_id = Uuid::new_v4();

        // First request should be allowed
        let result = middleware.check_rate_limit(&client_id).await.unwrap();
        assert!(result);

        // Record completion
        middleware
            .record_request_completion(&client_id)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_rate_limit_status() {
        let config = RateLimitingConfig::default();
        let middleware = RateLimitMiddleware::new(&config).await.unwrap();
        let client_id = Uuid::new_v4();

        // Make a request to initialize tracker
        middleware.check_rate_limit(&client_id).await.unwrap();

        // Get status
        let status = middleware.get_rate_limit_status(&client_id).await.unwrap();
        assert_eq!(status.client_id, client_id);
        assert!(status.requests_per_second > 0);
    }
}
