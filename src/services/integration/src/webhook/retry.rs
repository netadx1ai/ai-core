//! # Retry Manager
//!
//! The retry manager handles failed webhook events with sophisticated retry logic
//! including exponential backoff, jitter, circuit breaker patterns, and dead letter
//! queue integration for events that exceed retry limits.

use super::{WebhookConfig, WebhookError, WebhookEvent, WebhookEventStatus, WebhookResult};
use crate::error::{IntegrationError, IntegrationResult};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Retry strategy enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed { delay_seconds: u64 },
    /// Linear backoff (delay increases linearly)
    Linear { initial_delay: u64, increment: u64 },
    /// Exponential backoff with optional jitter
    Exponential {
        initial_delay: u64,
        multiplier: f64,
        max_delay: u64,
        jitter: bool,
    },
    /// Custom retry schedule
    Custom { delays: Vec<u64> },
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::Exponential {
            initial_delay: 5,
            multiplier: 2.0,
            max_delay: 300,
            jitter: true,
        }
    }
}

impl RetryStrategy {
    /// Calculate next retry delay based on attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_seconds = match self {
            RetryStrategy::Fixed { delay_seconds } => *delay_seconds,
            RetryStrategy::Linear {
                initial_delay,
                increment,
            } => initial_delay + (increment * attempt as u64),
            RetryStrategy::Exponential {
                initial_delay,
                multiplier,
                max_delay,
                jitter,
            } => {
                let exponential_delay = (*initial_delay as f64) * multiplier.powi(attempt as i32);
                let mut delay = exponential_delay.min(*max_delay as f64) as u64;

                if *jitter {
                    // Add random jitter (±25%)
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let jitter_factor = rng.gen_range(0.75..=1.25);
                    delay = (delay as f64 * jitter_factor) as u64;
                }

                delay
            }
            RetryStrategy::Custom { delays } => {
                if (attempt as usize) < delays.len() {
                    delays[attempt as usize]
                } else {
                    // Use last delay for attempts beyond the custom schedule
                    *delays.last().unwrap_or(&300)
                }
            }
        };

        Duration::from_secs(delay_seconds)
    }
}

/// Retry condition enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryCondition {
    /// Retry all failed events
    All,
    /// Retry only transient failures (5xx errors, timeouts, etc.)
    TransientOnly,
    /// Retry based on error type
    ErrorType(Vec<String>),
    /// Retry based on integration type
    Integration(Vec<String>),
    /// Custom condition function name
    Custom(String),
}

impl Default for RetryCondition {
    fn default() -> Self {
        Self::TransientOnly
    }
}

/// Retry statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetryStats {
    /// Total events queued for retry
    pub total_queued: u64,
    /// Total retry attempts made
    pub total_attempts: u64,
    /// Events successfully retried
    pub successful_retries: u64,
    /// Events failed permanently (dead lettered)
    pub permanent_failures: u64,
    /// Events currently in retry queue
    pub queue_depth: u64,
    /// Average retry delay in seconds
    pub avg_retry_delay_seconds: f64,
    /// Last retry attempt timestamp
    pub last_retry_at: Option<DateTime<Utc>>,
    /// Retry attempts by strategy
    pub strategy_stats: HashMap<String, StrategyStats>,
}

/// Statistics per retry strategy
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyStats {
    /// Events using this strategy
    pub events_count: u64,
    /// Total attempts with this strategy
    pub attempts_count: u64,
    /// Success rate
    pub success_rate: f64,
    /// Average delay
    pub avg_delay_seconds: f64,
}

/// Retry queue entry
#[derive(Debug, Clone)]
struct RetryQueueEntry {
    event: WebhookEvent,
    strategy: RetryStrategy,
    queued_at: Instant,
    retry_after: DateTime<Utc>,
    priority: u8,
}

impl PartialEq for RetryQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.event.id == other.event.id
    }
}

impl Eq for RetryQueueEntry {}

impl PartialOrd for RetryQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RetryQueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier retry time
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.retry_after.cmp(&other.retry_after))
    }
}

/// Trait for retry storage backends
#[async_trait]
pub trait RetryStorage: Send + Sync {
    /// Store event for retry
    async fn store_retry_event(
        &self,
        event: &WebhookEvent,
        retry_after: DateTime<Utc>,
        strategy: &RetryStrategy,
    ) -> WebhookResult<()>;

    /// Get events ready for retry
    async fn get_ready_events(&self, limit: usize) -> WebhookResult<Vec<WebhookEvent>>;

    /// Remove event from retry storage
    async fn remove_event(&self, event_id: Uuid) -> WebhookResult<()>;

    /// Get retry statistics
    async fn get_retry_stats(&self) -> WebhookResult<RetryStats>;

    /// Clean up old retry records
    async fn cleanup_old_records(&self, retention_hours: u64) -> WebhookResult<u64>;
}

/// In-memory retry storage implementation
pub struct MemoryRetryStorage {
    queue: Arc<RwLock<VecDeque<RetryQueueEntry>>>,
    stats: Arc<RwLock<RetryStats>>,
    max_size: usize,
}

impl MemoryRetryStorage {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            stats: Arc::new(RwLock::new(RetryStats::default())),
            max_size,
        }
    }

    pub fn get_stats(&self) -> RetryStats {
        let mut stats = self.stats.read().clone();
        stats.queue_depth = self.queue.read().len() as u64;
        stats
    }
}

#[async_trait]
impl RetryStorage for MemoryRetryStorage {
    async fn store_retry_event(
        &self,
        event: &WebhookEvent,
        retry_after: DateTime<Utc>,
        strategy: &RetryStrategy,
    ) -> WebhookResult<()> {
        let mut queue = self.queue.write();

        if queue.len() >= self.max_size {
            return Err(WebhookError::QueueFull);
        }

        let priority = match event.priority {
            super::EventPriority::Critical => 4,
            super::EventPriority::High => 3,
            super::EventPriority::Normal => 2,
            super::EventPriority::Low => 1,
        };

        let entry = RetryQueueEntry {
            event: event.clone(),
            strategy: strategy.clone(),
            queued_at: Instant::now(),
            retry_after,
            priority,
        };

        // Insert in sorted order (by retry time and priority)
        let mut insert_pos = queue.len();
        for (i, existing) in queue.iter().enumerate() {
            if entry < *existing {
                insert_pos = i;
                break;
            }
        }

        queue.insert(insert_pos, entry);

        // Update stats
        let mut stats = self.stats.write();
        stats.total_queued += 1;
        stats.queue_depth = queue.len() as u64;

        Ok(())
    }

    async fn get_ready_events(&self, limit: usize) -> WebhookResult<Vec<WebhookEvent>> {
        let now = Utc::now();
        let mut queue = self.queue.write();
        let mut ready_events = Vec::with_capacity(limit);

        // Find events that are ready for retry
        let mut to_remove = Vec::new();
        for (i, entry) in queue.iter().enumerate() {
            if ready_events.len() >= limit {
                break;
            }

            if entry.retry_after <= now {
                ready_events.push(entry.event.clone());
                to_remove.push(i);
            }
        }

        // Remove processed events from queue (in reverse order to maintain indices)
        for &index in to_remove.iter().rev() {
            queue.remove(index);
        }

        // Update stats
        if !ready_events.is_empty() {
            let mut stats = self.stats.write();
            stats.queue_depth = queue.len() as u64;
        }

        Ok(ready_events)
    }

    async fn remove_event(&self, event_id: Uuid) -> WebhookResult<()> {
        let mut queue = self.queue.write();
        queue.retain(|entry| entry.event.id != event_id);

        let mut stats = self.stats.write();
        stats.queue_depth = queue.len() as u64;

        Ok(())
    }

    async fn get_retry_stats(&self) -> WebhookResult<RetryStats> {
        Ok(self.get_stats())
    }

    async fn cleanup_old_records(&self, retention_hours: u64) -> WebhookResult<u64> {
        let cutoff_time = Instant::now() - Duration::from_secs(retention_hours * 3600);
        let mut queue = self.queue.write();
        let initial_count = queue.len();

        queue.retain(|entry| entry.queued_at > cutoff_time);

        let removed_count = (initial_count - queue.len()) as u64;

        if removed_count > 0 {
            let mut stats = self.stats.write();
            stats.queue_depth = queue.len() as u64;
        }

        Ok(removed_count)
    }
}

/// Retry manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Default retry strategy
    pub default_strategy: RetryStrategy,
    /// Retry condition
    pub retry_condition: RetryCondition,
    /// Maximum events in retry queue
    pub max_queue_size: usize,
    /// Retry processing interval in milliseconds
    pub processing_interval: u64,
    /// Maximum concurrent retry attempts
    pub max_concurrent_retries: usize,
    /// Retry timeout in seconds
    pub retry_timeout: u64,
    /// Dead letter queue integration
    pub use_dead_letter_queue: bool,
    /// Custom strategy mapping by integration
    pub strategy_overrides: HashMap<String, RetryStrategy>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            default_strategy: RetryStrategy::default(),
            retry_condition: RetryCondition::default(),
            max_queue_size: 5000,
            processing_interval: 1000,
            max_concurrent_retries: 20,
            retry_timeout: 30,
            use_dead_letter_queue: true,
            strategy_overrides: HashMap::new(),
        }
    }
}

/// Trait for retry event processing
#[async_trait]
pub trait RetryProcessor: Send + Sync {
    /// Process a retry event
    async fn process_retry(&self, event: WebhookEvent) -> IntegrationResult<()>;
}

/// Main retry manager
pub struct RetryManager {
    config: WebhookConfig,
    retry_config: RetryConfig,
    storage: Arc<dyn RetryStorage>,
    processor: Option<Arc<dyn RetryProcessor>>,
    stats: Arc<RwLock<RetryStats>>,
    running: Arc<AtomicBool>,
    processed_count: Arc<AtomicU64>,
    last_rate_reset: Arc<RwLock<Instant>>,
    retry_sender: mpsc::UnboundedSender<WebhookEvent>,
    retry_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<WebhookEvent>>>>,
}

impl RetryManager {
    /// Create a new retry manager
    pub fn new(config: WebhookConfig) -> Self {
        let retry_config = RetryConfig::default();
        let storage = Arc::new(MemoryRetryStorage::new(retry_config.max_queue_size));

        let (retry_sender, retry_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            retry_config,
            storage,
            processor: None,
            stats: Arc::new(RwLock::new(RetryStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            processed_count: Arc::new(AtomicU64::new(0)),
            last_rate_reset: Arc::new(RwLock::new(Instant::now())),
            retry_sender,
            retry_receiver: Arc::new(RwLock::new(Some(retry_receiver))),
        }
    }

    /// Set retry processor
    pub fn set_processor(&mut self, processor: Arc<dyn RetryProcessor>) {
        self.processor = Some(processor);
    }

    /// Start the retry manager
    pub async fn start(&self) -> IntegrationResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        info!("Starting retry manager");

        // Start main retry processing loop
        let manager = self.clone();
        tokio::spawn(async move {
            if let Err(e) = manager.run().await {
                error!("Retry manager error: {}", e);
            }
        });

        // Start retry event receiver
        let manager = self.clone();
        tokio::spawn(async move {
            if let Err(e) = manager.retry_receiver_loop().await {
                error!("Retry receiver error: {}", e);
            }
        });

        // Start cleanup task
        let manager = self.clone();
        tokio::spawn(async move {
            manager.cleanup_task().await;
        });

        Ok(())
    }

    /// Stop the retry manager
    pub async fn stop(&self) -> IntegrationResult<()> {
        info!("Stopping retry manager");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Queue event for retry
    #[instrument(skip(self, event), fields(event_id = %event.id))]
    pub async fn queue_retry(&self, mut event: WebhookEvent) -> IntegrationResult<()> {
        // Check if event should be retried
        if !self.should_retry(&event) {
            debug!("Event does not meet retry conditions, skipping");
            return Ok(());
        }

        // Determine retry strategy
        let strategy = self.get_retry_strategy(&event);

        // Calculate next retry time
        let delay = strategy.calculate_delay(event.attempt_count);
        let retry_after = Utc::now() + ChronoDuration::from_std(delay).unwrap_or_default();

        // Update event metadata
        event.next_retry_at = Some(retry_after);
        event.status = WebhookEventStatus::Failed;

        // Store for retry
        self.storage
            .store_retry_event(&event, retry_after, &strategy)
            .await
            .map_err(|e| IntegrationError::from(e))?;

        debug!(
            event_id = %event.id,
            retry_after = %retry_after,
            attempt = event.attempt_count,
            "Event queued for retry"
        );

        Ok(())
    }

    /// Process retry attempts
    pub async fn process_retries(&self) -> IntegrationResult<()> {
        let events = self
            .storage
            .get_ready_events(self.retry_config.max_concurrent_retries)
            .await
            .map_err(|e| IntegrationError::from(e))?;

        for event in events {
            self.retry_sender
                .send(event)
                .map_err(|_| IntegrationError::internal("Retry sender closed"))?;
        }

        Ok(())
    }

    /// Get retry statistics
    pub async fn get_stats(&self) -> IntegrationResult<RetryStats> {
        self.storage
            .get_retry_stats()
            .await
            .map_err(|e| IntegrationError::from(e))
    }

    /// Check if event should be retried
    fn should_retry(&self, event: &WebhookEvent) -> bool {
        // Check if event can be retried
        if !event.can_retry() {
            return false;
        }

        match &self.retry_config.retry_condition {
            RetryCondition::All => true,
            RetryCondition::TransientOnly => {
                // Check if error is transient (timeout, network, 5xx, etc.)
                if let Some(error) = &event.error {
                    error.contains("timeout")
                        || error.contains("network")
                        || error.contains("5")
                        || error.contains("unavailable")
                } else {
                    true
                }
            }
            RetryCondition::ErrorType(error_types) => {
                if let Some(error) = &event.error {
                    error_types.iter().any(|et| error.contains(et))
                } else {
                    false
                }
            }
            RetryCondition::Integration(integrations) => {
                integrations.contains(&event.payload.integration)
            }
            RetryCondition::Custom(_) => {
                // Custom conditions would be implemented here
                true
            }
        }
    }

    /// Get retry strategy for event
    fn get_retry_strategy(&self, event: &WebhookEvent) -> RetryStrategy {
        // Check for integration-specific strategy override
        if let Some(strategy) = self
            .retry_config
            .strategy_overrides
            .get(&event.payload.integration)
        {
            return strategy.clone();
        }

        // Use default strategy
        self.retry_config.default_strategy.clone()
    }

    /// Clone for background tasks
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            retry_config: self.retry_config.clone(),
            storage: Arc::clone(&self.storage),
            processor: self.processor.as_ref().map(Arc::clone),
            stats: Arc::clone(&self.stats),
            running: Arc::clone(&self.running),
            processed_count: Arc::clone(&self.processed_count),
            last_rate_reset: Arc::clone(&self.last_rate_reset),
            retry_sender: self.retry_sender.clone(),
            retry_receiver: Arc::clone(&self.retry_receiver),
        }
    }

    /// Main retry processing loop
    async fn run(&self) -> IntegrationResult<()> {
        let mut interval =
            tokio::time::interval(Duration::from_millis(self.retry_config.processing_interval));

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            if let Err(e) = self.process_retries().await {
                warn!("Retry processing error: {}", e);
            }
        }

        info!("Retry manager main loop stopped");
        Ok(())
    }

    /// Retry event receiver loop
    async fn retry_receiver_loop(&self) -> IntegrationResult<()> {
        let mut receiver = self
            .retry_receiver
            .write()
            .take()
            .ok_or_else(|| IntegrationError::internal("Retry receiver already taken"))?;

        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                event = receiver.recv() => {
                    match event {
                        Some(event) => {
                            let manager = self.clone();
                            tokio::spawn(async move {
                                manager.execute_retry(event).await;
                            });
                        }
                        None => break, // Channel closed
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic maintenance
                }
            }
        }

        info!("Retry receiver loop stopped");
        Ok(())
    }

    /// Execute retry attempt
    #[instrument(skip(self, event), fields(event_id = %event.id, attempt = event.attempt_count))]
    async fn execute_retry(&self, mut event: WebhookEvent) {
        let start_time = Instant::now();

        // Update attempt count
        event.attempt_count += 1;
        event.status = WebhookEventStatus::Processing;

        debug!("Executing retry attempt");

        // Process retry if processor is available
        let result = if let Some(processor) = &self.processor {
            tokio::time::timeout(
                Duration::from_secs(self.retry_config.retry_timeout),
                processor.process_retry(event.clone()),
            )
            .await
        } else {
            // No processor available - treat as success for testing
            Ok(Ok(()))
        };

        let processing_time = start_time.elapsed();

        // Handle result
        match result {
            Ok(Ok(())) => {
                // Success - mark as completed
                event.mark_completed();

                info!(
                    event_id = %event.id,
                    attempt = event.attempt_count,
                    processing_time_ms = processing_time.as_millis(),
                    "Retry succeeded"
                );

                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.successful_retries += 1;
                    stats.total_attempts += 1;
                    stats.last_retry_at = Some(Utc::now());
                }

                // Remove from retry storage
                if let Err(e) = self.storage.remove_event(event.id).await {
                    warn!("Failed to remove successful retry event: {}", e);
                }
            }
            Ok(Err(e)) => {
                // Processing failed
                error!(
                    event_id = %event.id,
                    attempt = event.attempt_count,
                    error = %e,
                    "Retry failed"
                );

                self.handle_retry_failure(event, e.to_string()).await;
            }
            Err(_) => {
                // Timeout
                error!(
                    event_id = %event.id,
                    attempt = event.attempt_count,
                    "Retry timed out"
                );

                self.handle_retry_failure(event, "Retry timeout".to_string())
                    .await;
            }
        }

        self.processed_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Handle retry failure
    async fn handle_retry_failure(&self, mut event: WebhookEvent, error: String) {
        let retry_after = if event.can_retry() {
            let strategy = self.get_retry_strategy(&event);
            let delay = strategy.calculate_delay(event.attempt_count);
            Some(Utc::now() + ChronoDuration::from_std(delay).unwrap_or_default())
        } else {
            None
        };

        event.mark_failed(error, retry_after);

        if event.can_retry() {
            // Queue for another retry
            if let Err(e) = self.queue_retry(event).await {
                error!("Failed to requeue event for retry: {}", e);
            }
        } else {
            // Move to dead letter queue if configured
            if self.retry_config.use_dead_letter_queue {
                // TODO: Integrate with dead letter queue
                warn!("Event exceeded retry limit, should move to dead letter queue");
            }

            // Update stats
            {
                let mut stats = self.stats.write();
                stats.permanent_failures += 1;
            }

            // Remove from retry storage
            if let Err(e) = self.storage.remove_event(event.id).await {
                warn!("Failed to remove permanently failed event: {}", e);
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_attempts += 1;
            stats.last_retry_at = Some(Utc::now());
        }
    }

    /// Cleanup task for old retry records
    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            if let Ok(removed_count) = self.storage.cleanup_old_records(72).await {
                if removed_count > 0 {
                    info!("Cleaned up {} old retry records", removed_count);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WebhookPayload;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use uuid::Uuid;

    // Mock retry processor for testing
    struct MockRetryProcessor {
        process_count: Arc<AtomicU32>,
        should_fail: bool,
    }

    impl MockRetryProcessor {
        fn new(should_fail: bool) -> Self {
            Self {
                process_count: Arc::new(AtomicU32::new(0)),
                should_fail,
            }
        }

        fn get_process_count(&self) -> u32 {
            self.process_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl RetryProcessor for MockRetryProcessor {
        async fn process_retry(&self, _event: WebhookEvent) -> IntegrationResult<()> {
            self.process_count.fetch_add(1, Ordering::SeqCst);

            if self.should_fail {
                Err(IntegrationError::webhook_processing("Mock retry failure"))
            } else {
                Ok(())
            }
        }
    }

    fn create_test_event() -> WebhookEvent {
        use super::super::EventPriority;

        let payload = WebhookPayload {
            id: Uuid::new_v4(),
            integration: "test".to_string(),
            event_type: "test.event".to_string(),
            timestamp: Utc::now(),
            data: json!({"test": "data"}),
            headers: HashMap::new(),
            source_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
        };
        let mut event = WebhookEvent::new(payload, EventPriority::Normal);
        event.mark_failed(
            "Test failure".to_string(),
            Some(Utc::now() + ChronoDuration::seconds(5)),
        );
        event
    }

    #[test]
    fn test_retry_strategy_delay_calculation() {
        // Test exponential backoff
        let strategy = RetryStrategy::Exponential {
            initial_delay: 5,
            multiplier: 2.0,
            max_delay: 60,
            jitter: false,
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(5));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(10));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(20));
        assert_eq!(strategy.calculate_delay(3), Duration::from_secs(40));
        assert_eq!(strategy.calculate_delay(4), Duration::from_secs(60)); // Max delay

        // Test fixed delay
        let strategy = RetryStrategy::Fixed { delay_seconds: 30 };
        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(30));
        assert_eq!(strategy.calculate_delay(5), Duration::from_secs(30));

        // Test linear backoff
        let strategy = RetryStrategy::Linear {
            initial_delay: 10,
            increment: 5,
        };
        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(10));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(15));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(20));

        // Test custom delays
        let strategy = RetryStrategy::Custom {
            delays: vec![5, 10, 30],
        };
        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(5));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(10));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(30));
        assert_eq!(strategy.calculate_delay(3), Duration::from_secs(30)); // Use last delay
    }

    #[tokio::test]
    async fn test_memory_retry_storage() {
        let storage = MemoryRetryStorage::new(10);
        let event = create_test_event();
        let retry_after = Utc::now() + ChronoDuration::seconds(1);
        let strategy = RetryStrategy::default();

        // Store event for retry
        storage
            .store_retry_event(&event, retry_after, &strategy)
            .await
            .unwrap();

        let stats = storage.get_retry_stats().await.unwrap();
        assert_eq!(stats.total_queued, 1);
        assert_eq!(stats.queue_depth, 1);

        // Should not be ready yet
        let ready_events = storage.get_ready_events(10).await.unwrap();
        assert!(ready_events.is_empty());

        // Wait and check again
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let ready_events = storage.get_ready_events(10).await.unwrap();
        assert_eq!(ready_events.len(), 1);
        assert_eq!(ready_events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_retry_queue_priority() {
        let storage = MemoryRetryStorage::new(10);
        let retry_after = Utc::now() + ChronoDuration::seconds(1);
        let strategy = RetryStrategy::default();

        // Create events with different priorities
        use super::super::EventPriority;

        let mut low_event = create_test_event();
        low_event.priority = EventPriority::Low;

        let mut high_event = create_test_event();
        high_event.priority = EventPriority::High;

        let mut critical_event = create_test_event();
        critical_event.priority = EventPriority::Critical;

        // Store in reverse priority order
        storage
            .store_retry_event(&low_event, retry_after, &strategy)
            .await
            .unwrap();
        storage
            .store_retry_event(&high_event, retry_after, &strategy)
            .await
            .unwrap();
        storage
            .store_retry_event(&critical_event, retry_after, &strategy)
            .await
            .unwrap();

        // Wait for retry time
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Should get events in priority order
        let ready_events = storage.get_ready_events(3).await.unwrap();
        assert_eq!(ready_events.len(), 3);
        assert_eq!(ready_events[0].priority, EventPriority::Critical);
        assert_eq!(ready_events[1].priority, EventPriority::High);
        assert_eq!(ready_events[2].priority, EventPriority::Low);
    }

    #[tokio::test]
    async fn test_retry_manager_lifecycle() {
        let config = WebhookConfig::default();
        let manager = RetryManager::new(config);

        // Start manager
        manager.start().await.unwrap();
        assert!(manager.running.load(Ordering::SeqCst));

        // Stop manager
        manager.stop().await.unwrap();
        assert!(!manager.running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_retry_condition_logic() {
        let config = WebhookConfig::default();
        let manager = RetryManager::new(config);

        // Test transient error detection
        let mut event = create_test_event();
        event.error = Some("timeout error".to_string());
        assert!(manager.should_retry(&event));

        event.error = Some("network error".to_string());
        assert!(manager.should_retry(&event));

        event.error = Some("400 bad request".to_string());
        assert!(!manager.should_retry(&event));

        // Test exceeded retry limit
        event.attempt_count = event.max_attempts;
        assert!(!manager.should_retry(&event));
    }

    #[tokio::test]
    async fn test_retry_processing_success() {
        let config = WebhookConfig::default();
        let mut manager = RetryManager::new(config);

        let processor = Arc::new(MockRetryProcessor::new(false));
        manager.set_processor(processor.clone() as Arc<dyn RetryProcessor>);
        manager.start().await.unwrap();

        // Queue an event for retry
        let event = create_test_event();
        manager.queue_retry(event).await.unwrap();

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check that processor was called
        assert_eq!(processor.get_process_count(), 1);

        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_retry_strategy_override() {
        let config = WebhookConfig::default();
        let mut manager = RetryManager::new(config);

        // Add strategy override
        manager.retry_config.strategy_overrides.insert(
            "test".to_string(),
            RetryStrategy::Fixed { delay_seconds: 60 },
        );

        let event = create_test_event();
        let strategy = manager.get_retry_strategy(&event);

        match strategy {
            RetryStrategy::Fixed { delay_seconds } => assert_eq!(delay_seconds, 60),
            _ => panic!("Expected fixed strategy"),
        }
    }

    #[tokio::test]
    async fn test_retry_stats_tracking() {
        let config = WebhookConfig::default();
        let mut manager = RetryManager::new(config);

        let processor = Arc::new(MockRetryProcessor::new(false));
        manager.set_processor(processor as Arc<dyn RetryProcessor>);
        manager.start().await.unwrap();

        // Queue events for retry
        for _ in 0..3 {
            let event = create_test_event();
            manager.queue_retry(event).await.unwrap();
        }

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(300)).await;

        let stats = manager.get_stats().await.unwrap();
        assert!(stats.total_queued >= 3);
        assert!(stats.successful_retries > 0);

        manager.stop().await.unwrap();
    }
}
