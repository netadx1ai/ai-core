//! # Webhook and Event Handling System
//!
//! This module provides a comprehensive webhook and event handling system for the AI-CORE platform
//! that includes:
//! - Advanced webhook receiver with multi-provider signature validation
//! - Event routing and processing pipeline with configurable rules
//! - Retry logic with exponential backoff and circuit breaker patterns
//! - Dead letter queue handling for failed events
//! - Event replay and audit capabilities
//! - Real-time event streaming integration

pub mod collector;
pub mod processor;
pub mod queue;
pub mod retry;
pub mod router;
pub mod validator;

use crate::error::{IntegrationError, IntegrationResult};
use crate::models::{IntegrationEvent, WebhookPayload};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Webhook event processing status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEventStatus {
    /// Event received and queued for processing
    Received,
    /// Event is being processed
    Processing,
    /// Event processed successfully
    Completed,
    /// Event failed processing but can be retried
    Failed,
    /// Event failed permanently after all retries
    DeadLettered,
    /// Event processing was cancelled
    Cancelled,
}

/// Webhook event priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Webhook event with processing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// Original webhook payload
    pub payload: WebhookPayload,
    /// Event processing status
    pub status: WebhookEventStatus,
    /// Event priority level
    pub priority: EventPriority,
    /// Number of processing attempts
    pub attempt_count: u32,
    /// Maximum retry attempts allowed
    pub max_attempts: u32,
    /// Next retry timestamp (if applicable)
    pub next_retry_at: Option<DateTime<Utc>>,
    /// Event creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last processing attempt timestamp
    pub updated_at: DateTime<Utc>,
    /// Processing error details (if failed)
    pub error: Option<String>,
    /// Event routing information
    pub routing_key: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl WebhookEvent {
    /// Create a new webhook event
    pub fn new(payload: WebhookPayload, priority: EventPriority) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            payload,
            status: WebhookEventStatus::Received,
            priority,
            attempt_count: 0,
            max_attempts: 3,
            next_retry_at: None,
            created_at: now,
            updated_at: now,
            error: None,
            routing_key: None,
            metadata: HashMap::new(),
        }
    }

    /// Mark event as processing
    pub fn mark_processing(&mut self) {
        self.status = WebhookEventStatus::Processing;
        self.updated_at = Utc::now();
        self.attempt_count += 1;
    }

    /// Mark event as completed
    pub fn mark_completed(&mut self) {
        self.status = WebhookEventStatus::Completed;
        self.updated_at = Utc::now();
        self.error = None;
    }

    /// Mark event as failed with error details
    pub fn mark_failed(&mut self, error: String, next_retry: Option<DateTime<Utc>>) {
        self.status = if self.attempt_count >= self.max_attempts {
            WebhookEventStatus::DeadLettered
        } else {
            WebhookEventStatus::Failed
        };
        self.updated_at = Utc::now();
        self.error = Some(error);
        self.next_retry_at = next_retry;
    }

    /// Check if event can be retried
    pub fn can_retry(&self) -> bool {
        matches!(self.status, WebhookEventStatus::Failed) && self.attempt_count < self.max_attempts
    }

    /// Check if event is ready for retry
    pub fn is_ready_for_retry(&self) -> bool {
        self.can_retry()
            && self
                .next_retry_at
                .map_or(true, |retry_at| Utc::now() >= retry_at)
    }
}

/// Webhook processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Maximum concurrent webhook processing
    pub max_concurrent_processing: usize,
    /// Default retry attempts for failed events
    pub default_max_attempts: u32,
    /// Initial retry delay in seconds
    pub initial_retry_delay: u64,
    /// Maximum retry delay in seconds
    pub max_retry_delay: u64,
    /// Retry backoff multiplier
    pub retry_backoff_multiplier: f64,
    /// Dead letter queue retention hours
    pub dead_letter_retention_hours: u64,
    /// Event batch size for processing
    pub batch_size: usize,
    /// Processing timeout in seconds
    pub processing_timeout: u64,
    /// Enable event compression
    pub enable_compression: bool,
    /// Webhook signature validation timeout
    pub signature_validation_timeout: u64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            max_concurrent_processing: 100,
            default_max_attempts: 3,
            initial_retry_delay: 5,
            max_retry_delay: 300,
            retry_backoff_multiplier: 2.0,
            dead_letter_retention_hours: 72,
            batch_size: 10,
            processing_timeout: 30,
            enable_compression: true,
            signature_validation_timeout: 5,
        }
    }
}

/// Webhook processing statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebhookStats {
    /// Total events received
    pub total_received: u64,
    /// Total events processed successfully
    pub total_processed: u64,
    /// Total events failed
    pub total_failed: u64,
    /// Total events in dead letter queue
    pub total_dead_lettered: u64,
    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,
    /// Events currently being processed
    pub currently_processing: u64,
    /// Events waiting in queue
    pub queue_depth: u64,
    /// Last processing timestamp
    pub last_processed_at: Option<DateTime<Utc>>,
}

/// Trait for webhook event processing
#[async_trait]
pub trait WebhookProcessor: Send + Sync {
    /// Process a webhook event
    async fn process_event(&self, event: &WebhookEvent) -> IntegrationResult<IntegrationEvent>;

    /// Get processor name
    fn name(&self) -> &str;

    /// Check if processor can handle the event
    fn can_handle(&self, event: &WebhookEvent) -> bool;

    /// Get processing timeout for this processor
    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
}

/// Trait for webhook event routing
#[async_trait]
pub trait EventRouter: Send + Sync {
    /// Route an event to appropriate processors
    async fn route_event(&self, event: &WebhookEvent) -> IntegrationResult<Vec<String>>;

    /// Get routing configuration
    fn get_routing_config(&self) -> HashMap<String, Vec<String>>;
}

/// Trait for webhook event storage
#[async_trait]
pub trait EventStorage: Send + Sync {
    /// Store a webhook event
    async fn store_event(&self, event: &WebhookEvent) -> IntegrationResult<()>;

    /// Retrieve an event by ID
    async fn get_event(&self, id: Uuid) -> IntegrationResult<Option<WebhookEvent>>;

    /// Update event status
    async fn update_event(&self, event: &WebhookEvent) -> IntegrationResult<()>;

    /// Get events ready for retry
    async fn get_retry_events(&self, limit: usize) -> IntegrationResult<Vec<WebhookEvent>>;

    /// Get dead letter queue events
    async fn get_dead_letter_events(&self, limit: usize) -> IntegrationResult<Vec<WebhookEvent>>;

    /// Clean up old events
    async fn cleanup_old_events(&self, retention_hours: u64) -> IntegrationResult<u64>;

    /// Get processing statistics
    async fn get_stats(&self) -> IntegrationResult<WebhookStats>;
}

/// Main webhook handling system
pub struct WebhookHandler {
    config: WebhookConfig,
    processors: Arc<RwLock<HashMap<String, Arc<dyn WebhookProcessor>>>>,
    router: Arc<dyn EventRouter>,
    storage: Arc<dyn EventStorage>,
    collector: Arc<collector::WebhookCollector>,
    processor: Arc<processor::EventProcessor>,
    retry_manager: Arc<retry::RetryManager>,
    dead_letter_queue: Arc<queue::DeadLetterQueue>,
    stats: Arc<RwLock<WebhookStats>>,
}

impl WebhookHandler {
    /// Create a new webhook handler
    pub fn new(
        config: WebhookConfig,
        router: Arc<dyn EventRouter>,
        storage: Arc<dyn EventStorage>,
    ) -> Self {
        let collector = Arc::new(collector::WebhookCollector::new(config.clone()));
        let processor = Arc::new(processor::EventProcessor::new(config.clone()));
        let retry_manager = Arc::new(retry::RetryManager::new(config.clone()));
        let dead_letter_queue = Arc::new(queue::DeadLetterQueue::new(config.clone()));

        Self {
            config,
            processors: Arc::new(RwLock::new(HashMap::new())),
            router,
            storage,
            collector,
            processor,
            retry_manager,
            dead_letter_queue,
            stats: Arc::new(RwLock::new(WebhookStats::default())),
        }
    }

    /// Register a webhook processor
    pub async fn register_processor(&self, processor: Arc<dyn WebhookProcessor>) {
        let mut processors = self.processors.write().await;
        processors.insert(processor.name().to_string(), processor);
    }

    /// Handle incoming webhook
    pub async fn handle_webhook(&self, payload: WebhookPayload) -> IntegrationResult<Uuid> {
        // Create webhook event
        let event = WebhookEvent::new(payload, EventPriority::Normal);
        let event_id = event.id;

        // Collect event for processing
        self.collector.collect(event).await?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_received += 1;
        stats.queue_depth += 1;

        Ok(event_id)
    }

    /// Process webhook events from queue
    pub async fn process_events(&self) -> IntegrationResult<()> {
        self.processor.process_batch().await
    }

    /// Handle retry logic for failed events
    pub async fn handle_retries(&self) -> IntegrationResult<()> {
        self.retry_manager.process_retries().await
    }

    /// Process dead letter queue
    pub async fn process_dead_letters(&self) -> IntegrationResult<()> {
        self.dead_letter_queue.process_queue().await
    }

    /// Get webhook processing statistics
    pub async fn get_stats(&self) -> IntegrationResult<WebhookStats> {
        self.storage.get_stats().await
    }

    /// Clean up old processed events
    pub async fn cleanup_events(&self) -> IntegrationResult<u64> {
        self.storage
            .cleanup_old_events(self.config.dead_letter_retention_hours)
            .await
    }

    /// Start webhook processing background tasks
    pub async fn start(&self) -> IntegrationResult<()> {
        // Start collector
        self.collector.start().await?;

        // Start processor
        self.processor.start().await?;

        // Start retry manager
        self.retry_manager.start().await?;

        // Start dead letter queue processor
        self.dead_letter_queue.start().await?;

        Ok(())
    }

    /// Stop webhook processing
    pub async fn stop(&self) -> IntegrationResult<()> {
        // Stop all components gracefully
        self.collector.stop().await?;
        self.processor.stop().await?;
        self.retry_manager.stop().await?;
        self.dead_letter_queue.stop().await?;

        Ok(())
    }

    /// Get event by ID
    pub async fn get_event(&self, id: Uuid) -> IntegrationResult<Option<WebhookEvent>> {
        self.storage.get_event(id).await
    }

    /// Replay a dead lettered event
    pub async fn replay_event(&self, id: Uuid) -> IntegrationResult<()> {
        if let Some(mut event) = self.storage.get_event(id).await? {
            // Reset event for replay
            event.status = WebhookEventStatus::Received;
            event.attempt_count = 0;
            event.error = None;
            event.next_retry_at = None;
            event.updated_at = Utc::now();

            // Store updated event
            self.storage.update_event(&event).await?;

            // Re-queue for processing
            self.collector.collect(event).await?;
        }

        Ok(())
    }
}

/// Error types specific to webhook handling
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("Webhook validation failed: {0}")]
    ValidationFailed(String),

    #[error("Event processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Event routing failed: {0}")]
    RoutingFailed(String),

    #[error("Storage operation failed: {0}")]
    StorageFailed(String),

    #[error("Retry limit exceeded for event {event_id}")]
    RetryLimitExceeded { event_id: Uuid },

    #[error("Event not found: {event_id}")]
    EventNotFound { event_id: Uuid },

    #[error("Processing timeout exceeded")]
    TimeoutExceeded,

    #[error("Queue is full")]
    QueueFull,

    #[error("Invalid event state: {0}")]
    InvalidState(String),
}

impl From<WebhookError> for IntegrationError {
    fn from(err: WebhookError) -> Self {
        match err {
            WebhookError::ValidationFailed(msg) => IntegrationError::SignatureVerification {
                integration: "webhook".to_string(),
                reason: msg,
            },
            WebhookError::ProcessingFailed(msg) => {
                IntegrationError::WebhookProcessing { message: msg }
            }
            WebhookError::StorageFailed(msg) => IntegrationError::internal(msg),
            WebhookError::TimeoutExceeded => IntegrationError::timeout(30),
            _ => IntegrationError::internal(err.to_string()),
        }
    }
}

/// Result type for webhook operations
pub type WebhookResult<T> = Result<T, WebhookError>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn create_test_payload() -> WebhookPayload {
        WebhookPayload {
            id: Uuid::new_v4(),
            integration: "test".to_string(),
            event_type: "test.event".to_string(),
            timestamp: Utc::now(),
            data: json!({"test": "data"}),
            headers: HashMap::new(),
            source_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
        }
    }

    #[test]
    fn test_webhook_event_creation() {
        let payload = create_test_payload();
        let event = WebhookEvent::new(payload, EventPriority::High);

        assert_eq!(event.status, WebhookEventStatus::Received);
        assert_eq!(event.priority, EventPriority::High);
        assert_eq!(event.attempt_count, 0);
        assert!(event.can_retry());
    }

    #[test]
    fn test_webhook_event_retry_logic() {
        let payload = create_test_payload();
        let mut event = WebhookEvent::new(payload, EventPriority::Normal);

        // Mark as processing
        event.mark_processing();
        assert_eq!(event.status, WebhookEventStatus::Processing);
        assert_eq!(event.attempt_count, 1);

        // Mark as failed
        let next_retry = Some(Utc::now() + chrono::Duration::seconds(5));
        event.mark_failed("Test error".to_string(), next_retry);
        assert_eq!(event.status, WebhookEventStatus::Failed);
        assert!(event.can_retry());

        // Exceed retry limit
        event.attempt_count = event.max_attempts;
        event.mark_failed("Final error".to_string(), None);
        assert_eq!(event.status, WebhookEventStatus::DeadLettered);
        assert!(!event.can_retry());
    }

    #[test]
    fn test_webhook_config_defaults() {
        let config = WebhookConfig::default();
        assert_eq!(config.max_concurrent_processing, 100);
        assert_eq!(config.default_max_attempts, 3);
        assert_eq!(config.initial_retry_delay, 5);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_event_priority_ordering() {
        assert!(EventPriority::Critical > EventPriority::High);
        assert!(EventPriority::High > EventPriority::Normal);
        assert!(EventPriority::Normal > EventPriority::Low);
    }
}
