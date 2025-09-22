//! # Webhook Collector
//!
//! The webhook collector is responsible for receiving incoming webhook events,
//! performing initial validation, and queuing them for processing. It provides
//! high-throughput event ingestion with backpressure handling and quality-of-service
//! guarantees.

use super::{WebhookConfig, WebhookError, WebhookEvent, WebhookResult};
use crate::error::{IntegrationError, IntegrationResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Webhook collection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollectionStats {
    /// Total events collected
    pub total_collected: u64,
    /// Events currently in queue
    pub queue_depth: u64,
    /// Events dropped due to queue full
    pub dropped_events: u64,
    /// Average collection time in microseconds
    pub avg_collection_time_us: f64,
    /// Peak queue depth observed
    pub peak_queue_depth: u64,
    /// Collection rate (events per second)
    pub collection_rate: f64,
    /// Last collection timestamp
    pub last_collection_at: Option<DateTime<Utc>>,
}

/// Event queue entry with metadata
#[derive(Debug, Clone)]
struct QueuedEvent {
    event: WebhookEvent,
    queued_at: Instant,
    queue_priority: u8,
}

impl PartialEq for QueuedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.event.id == other.event.id
    }
}

impl Eq for QueuedEvent {}

impl PartialOrd for QueuedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier queued time
        other
            .queue_priority
            .cmp(&self.queue_priority)
            .then_with(|| self.queued_at.cmp(&other.queued_at))
    }
}

/// Trait for event queue backends
#[async_trait]
pub trait EventQueue: Send + Sync {
    /// Push an event to the queue
    async fn push(&self, event: WebhookEvent) -> WebhookResult<()>;

    /// Pop an event from the queue
    async fn pop(&self) -> WebhookResult<Option<WebhookEvent>>;

    /// Get current queue depth
    async fn depth(&self) -> WebhookResult<usize>;

    /// Check if queue is full
    async fn is_full(&self) -> WebhookResult<bool>;

    /// Clear all events from queue
    async fn clear(&self) -> WebhookResult<()>;
}

/// In-memory priority queue implementation
pub struct MemoryEventQueue {
    queue: Arc<RwLock<VecDeque<QueuedEvent>>>,
    max_size: usize,
    stats: Arc<RwLock<CollectionStats>>,
}

impl MemoryEventQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            max_size,
            stats: Arc::new(RwLock::new(CollectionStats::default())),
        }
    }

    pub fn get_stats(&self) -> CollectionStats {
        self.stats.read().clone()
    }
}

#[async_trait]
impl EventQueue for MemoryEventQueue {
    async fn push(&self, event: WebhookEvent) -> WebhookResult<()> {
        let mut queue = self.queue.write();

        if queue.len() >= self.max_size {
            let mut stats = self.stats.write();
            stats.dropped_events += 1;
            return Err(WebhookError::QueueFull);
        }

        let queue_priority = match event.priority {
            super::EventPriority::Critical => 4,
            super::EventPriority::High => 3,
            super::EventPriority::Normal => 2,
            super::EventPriority::Low => 1,
        };

        let queued_event = QueuedEvent {
            event,
            queued_at: Instant::now(),
            queue_priority,
        };

        // Insert in priority order
        let mut insert_pos = queue.len();
        for (i, existing) in queue.iter().enumerate() {
            if queued_event < *existing {
                insert_pos = i;
                break;
            }
        }

        queue.insert(insert_pos, queued_event);

        // Update stats
        let mut stats = self.stats.write();
        stats.total_collected += 1;
        stats.queue_depth = queue.len() as u64;
        if stats.queue_depth > stats.peak_queue_depth {
            stats.peak_queue_depth = stats.queue_depth;
        }
        stats.last_collection_at = Some(Utc::now());

        Ok(())
    }

    async fn pop(&self) -> WebhookResult<Option<WebhookEvent>> {
        let mut queue = self.queue.write();
        let event = queue.pop_front().map(|queued| queued.event);

        if event.is_some() {
            let mut stats = self.stats.write();
            stats.queue_depth = queue.len() as u64;
        }

        Ok(event)
    }

    async fn depth(&self) -> WebhookResult<usize> {
        Ok(self.queue.read().len())
    }

    async fn is_full(&self) -> WebhookResult<bool> {
        Ok(self.queue.read().len() >= self.max_size)
    }

    async fn clear(&self) -> WebhookResult<()> {
        let mut queue = self.queue.write();
        queue.clear();

        let mut stats = self.stats.write();
        stats.queue_depth = 0;

        Ok(())
    }
}

/// Webhook validator trait
#[async_trait]
pub trait WebhookValidator: Send + Sync {
    /// Validate incoming webhook event
    async fn validate(&self, event: &WebhookEvent) -> WebhookResult<()>;
}

/// Basic webhook validator
pub struct BasicValidator {
    max_payload_size: usize,
    required_headers: Vec<String>,
    allowed_integrations: Option<Vec<String>>,
}

impl BasicValidator {
    pub fn new(
        max_payload_size: usize,
        required_headers: Vec<String>,
        allowed_integrations: Option<Vec<String>>,
    ) -> Self {
        Self {
            max_payload_size,
            required_headers,
            allowed_integrations,
        }
    }
}

#[async_trait]
impl WebhookValidator for BasicValidator {
    async fn validate(&self, event: &WebhookEvent) -> WebhookResult<()> {
        // Check payload size
        let payload_size = serde_json::to_vec(&event.payload)
            .map_err(|e| {
                WebhookError::ValidationFailed(format!("JSON serialization failed: {}", e))
            })?
            .len();

        if payload_size > self.max_payload_size {
            return Err(WebhookError::ValidationFailed(format!(
                "Payload size {} exceeds maximum {}",
                payload_size, self.max_payload_size
            )));
        }

        // Check required headers
        for header in &self.required_headers {
            if !event.payload.headers.contains_key(header) {
                return Err(WebhookError::ValidationFailed(format!(
                    "Required header '{}' missing",
                    header
                )));
            }
        }

        // Check allowed integrations
        if let Some(allowed) = &self.allowed_integrations {
            if !allowed.contains(&event.payload.integration) {
                return Err(WebhookError::ValidationFailed(format!(
                    "Integration '{}' not allowed",
                    event.payload.integration
                )));
            }
        }

        Ok(())
    }
}

/// Webhook collector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Maximum payload size in bytes
    pub max_payload_size: usize,
    /// Collection timeout in seconds
    pub collection_timeout: u64,
    /// Enable validation
    pub enable_validation: bool,
    /// Required headers for validation
    pub required_headers: Vec<String>,
    /// Allowed integrations (None = all allowed)
    pub allowed_integrations: Option<Vec<String>>,
    /// Rate limiting: max events per second
    pub max_events_per_second: u64,
    /// Burst capacity for rate limiting
    pub burst_capacity: u64,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            max_payload_size: 10 * 1024 * 1024, // 10MB
            collection_timeout: 5,
            enable_validation: true,
            required_headers: vec![],
            allowed_integrations: None,
            max_events_per_second: 1000,
            burst_capacity: 100,
        }
    }
}

/// Main webhook collector
pub struct WebhookCollector {
    config: WebhookConfig,
    collector_config: CollectorConfig,
    queue: Arc<dyn EventQueue>,
    validator: Arc<dyn WebhookValidator>,
    stats: Arc<RwLock<CollectionStats>>,
    rate_limiter: Arc<Semaphore>,
    running: Arc<AtomicBool>,
    collection_count: Arc<AtomicU64>,
    last_rate_reset: Arc<RwLock<Instant>>,
    event_sender: mpsc::UnboundedSender<WebhookEvent>,
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<WebhookEvent>>>>,
}

impl WebhookCollector {
    /// Create a new webhook collector
    pub fn new(config: WebhookConfig) -> Self {
        let collector_config = CollectorConfig::default();

        let queue = Arc::new(MemoryEventQueue::new(collector_config.max_queue_size));

        let validator = Arc::new(BasicValidator::new(
            collector_config.max_payload_size,
            collector_config.required_headers.clone(),
            collector_config.allowed_integrations.clone(),
        ));

        let rate_limiter = Arc::new(Semaphore::new(collector_config.burst_capacity as usize));

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            collector_config,
            queue,
            validator,
            stats: Arc::new(RwLock::new(CollectionStats::default())),
            rate_limiter,
            running: Arc::new(AtomicBool::new(false)),
            collection_count: Arc::new(AtomicU64::new(0)),
            last_rate_reset: Arc::new(RwLock::new(Instant::now())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
        }
    }

    /// Start the collector
    pub async fn start(&self) -> IntegrationResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        info!("Starting webhook collector");

        // Start background processing task
        let collector = self.clone();
        tokio::spawn(async move {
            if let Err(e) = collector.run().await {
                error!("Webhook collector error: {}", e);
            }
        });

        // Start rate limiter reset task
        let collector = self.clone();
        tokio::spawn(async move {
            collector.rate_limiter_task().await;
        });

        Ok(())
    }

    /// Stop the collector
    pub async fn stop(&self) -> IntegrationResult<()> {
        info!("Stopping webhook collector");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Collect a webhook event
    pub async fn collect(&self, event: WebhookEvent) -> IntegrationResult<()> {
        let start_time = Instant::now();

        // Check if collector is running
        if !self.running.load(Ordering::SeqCst) {
            return Err(IntegrationError::service_unavailable("webhook collector"));
        }

        // Rate limiting
        let _permit = timeout(
            Duration::from_secs(self.collector_config.collection_timeout),
            self.rate_limiter.acquire(),
        )
        .await
        .map_err(|_| IntegrationError::timeout(5))?
        .map_err(|_| IntegrationError::service_unavailable("rate limiter"))?;

        // Validate event if enabled
        if self.collector_config.enable_validation {
            self.validator
                .validate(&event)
                .await
                .map_err(|e| IntegrationError::from(e))?;
        }

        // Send event for processing
        let event_id = event.id;
        self.event_sender
            .send(event)
            .map_err(|_| IntegrationError::internal("Event sender closed"))?;

        // Update collection stats
        let collection_time = start_time.elapsed();
        let mut stats = self.stats.write();
        stats.total_collected += 1;
        stats.avg_collection_time_us = (stats.avg_collection_time_us
            * (stats.total_collected - 1) as f64
            + collection_time.as_micros() as f64)
            / stats.total_collected as f64;
        stats.last_collection_at = Some(Utc::now());

        self.collection_count.fetch_add(1, Ordering::SeqCst);

        debug!(
            event_id = %event_id,
            collection_time_us = collection_time.as_micros(),
            "Event collected successfully"
        );

        Ok(())
    }

    /// Get collection statistics
    pub async fn get_stats(&self) -> CollectionStats {
        let mut stats = self.stats.read().clone();
        stats.queue_depth = self.queue.depth().await.unwrap_or(0) as u64;

        // Calculate collection rate
        let now = Instant::now();
        let last_reset = *self.last_rate_reset.read();
        let time_elapsed = now.duration_since(last_reset).as_secs_f64();
        let count = self.collection_count.load(Ordering::SeqCst) as f64;

        if time_elapsed > 0.0 {
            stats.collection_rate = count / time_elapsed;
        }

        stats
    }

    /// Clone the collector (for background tasks)
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            collector_config: self.collector_config.clone(),
            queue: Arc::clone(&self.queue),
            validator: Arc::clone(&self.validator),
            stats: Arc::clone(&self.stats),
            rate_limiter: Arc::clone(&self.rate_limiter),
            running: Arc::clone(&self.running),
            collection_count: Arc::clone(&self.collection_count),
            last_rate_reset: Arc::clone(&self.last_rate_reset),
            event_sender: self.event_sender.clone(),
            event_receiver: Arc::clone(&self.event_receiver),
        }
    }

    /// Main processing loop
    async fn run(&self) -> IntegrationResult<()> {
        let mut receiver = self
            .event_receiver
            .write()
            .take()
            .ok_or_else(|| IntegrationError::internal("Event receiver already taken"))?;

        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                event = receiver.recv() => {
                    match event {
                        Some(event) => {
                            if let Err(e) = self.queue.push(event).await {
                                warn!("Failed to queue event: {}", e);
                                let mut stats = self.stats.write();
                                stats.dropped_events += 1;
                            }
                        }
                        None => break, // Channel closed
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic maintenance
                }
            }
        }

        info!("Webhook collector stopped");
        Ok(())
    }

    /// Rate limiter reset task
    async fn rate_limiter_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            // Reset collection count for rate calculation
            self.collection_count.store(0, Ordering::SeqCst);
            *self.last_rate_reset.write() = Instant::now();

            // Add permits back to semaphore (rate limiting)
            let current_permits = self.rate_limiter.available_permits();
            let max_permits = self.collector_config.max_events_per_second as usize;

            if current_permits < max_permits {
                let permits_to_add = max_permits - current_permits;
                self.rate_limiter.add_permits(permits_to_add);
            }
        }
    }

    /// Get events from queue for processing
    pub async fn get_events(&self, batch_size: usize) -> WebhookResult<Vec<WebhookEvent>> {
        let mut events = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            if let Some(event) = self.queue.pop().await? {
                events.push(event);
            } else {
                break;
            }
        }

        Ok(events)
    }

    /// Check if queue is empty
    pub async fn is_queue_empty(&self) -> bool {
        self.queue.depth().await.unwrap_or(1) == 0
    }

    /// Clear all events from queue
    pub async fn clear_queue(&self) -> WebhookResult<()> {
        self.queue.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WebhookPayload;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_event() -> WebhookEvent {
        use super::super::EventPriority;
        use uuid::Uuid;

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
        WebhookEvent::new(payload, EventPriority::Normal)
    }

    #[tokio::test]
    async fn test_memory_queue_operations() {
        let queue = MemoryEventQueue::new(10);

        // Test empty queue
        assert_eq!(queue.depth().await.unwrap(), 0);
        assert!(queue.pop().await.unwrap().is_none());

        // Test push and pop
        let event = create_test_event();
        let event_id = event.id;

        queue.push(event).await.unwrap();
        assert_eq!(queue.depth().await.unwrap(), 1);

        let popped = queue.pop().await.unwrap().unwrap();
        assert_eq!(popped.id, event_id);
        assert_eq!(queue.depth().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_priority_queue_ordering() {
        let queue = MemoryEventQueue::new(10);

        // Add events with different priorities
        use super::super::EventPriority;

        let low_event = {
            let mut event = create_test_event();
            event.priority = EventPriority::Low;
            event
        };

        let high_event = {
            let mut event = create_test_event();
            event.priority = EventPriority::High;
            event
        };

        let critical_event = {
            let mut event = create_test_event();
            event.priority = EventPriority::Critical;
            event
        };

        // Push in reverse priority order
        queue.push(low_event.clone()).await.unwrap();
        queue.push(high_event.clone()).await.unwrap();
        queue.push(critical_event.clone()).await.unwrap();

        // Should pop in priority order (critical first)
        let popped1 = queue.pop().await.unwrap().unwrap();
        assert_eq!(popped1.priority, EventPriority::Critical);

        let popped2 = queue.pop().await.unwrap().unwrap();
        assert_eq!(popped2.priority, EventPriority::High);

        let popped3 = queue.pop().await.unwrap().unwrap();
        assert_eq!(popped3.priority, EventPriority::Low);
    }

    #[tokio::test]
    async fn test_queue_capacity_limits() {
        let queue = MemoryEventQueue::new(2);

        // Fill queue to capacity
        queue.push(create_test_event()).await.unwrap();
        queue.push(create_test_event()).await.unwrap();

        assert!(queue.is_full().await.unwrap());

        // Next push should fail
        let result = queue.push(create_test_event()).await;
        assert!(matches!(result, Err(WebhookError::QueueFull)));
    }

    #[tokio::test]
    async fn test_basic_validator() {
        let validator = BasicValidator::new(
            1024,
            vec!["content-type".to_string()],
            Some(vec!["allowed-integration".to_string()]),
        );

        let mut event = create_test_event();

        // Should fail - missing required header
        let result = validator.validate(&event).await;
        assert!(matches!(result, Err(WebhookError::ValidationFailed(_))));

        // Add required header
        event
            .payload
            .headers
            .insert("content-type".to_string(), "application/json".to_string());

        // Should fail - integration not allowed
        let result = validator.validate(&event).await;
        assert!(matches!(result, Err(WebhookError::ValidationFailed(_))));

        // Fix integration name
        event.payload.integration = "allowed-integration".to_string();

        // Should pass now
        let result = validator.validate(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collector_lifecycle() {
        let config = WebhookConfig::default();
        let collector = WebhookCollector::new(config);

        // Start collector
        collector.start().await.unwrap();
        assert!(collector.running.load(Ordering::SeqCst));

        // Collect an event
        let event = create_test_event();
        collector.collect(event).await.unwrap();

        // Check stats
        let stats = collector.get_stats().await;
        assert_eq!(stats.total_collected, 1);

        // Stop collector
        collector.stop().await.unwrap();
        assert!(!collector.running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_collection_stats_tracking() {
        let config = WebhookConfig::default();
        let collector = WebhookCollector::new(config);

        collector.start().await.unwrap();

        // Collect multiple events
        for _ in 0..5 {
            let event = create_test_event();
            collector.collect(event).await.unwrap();
        }

        let stats = collector.get_stats().await;
        assert_eq!(stats.total_collected, 5);
        assert!(stats.avg_collection_time_us > 0.0);
        assert!(stats.last_collection_at.is_some());

        collector.stop().await.unwrap();
    }
}
