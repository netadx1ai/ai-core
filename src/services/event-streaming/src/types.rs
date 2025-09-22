//! # Event Streaming Types
//!
//! Core types, enums, and data structures for the event streaming service.
//! This module defines all the fundamental types used throughout the event streaming system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Event priority levels for processing order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPriority {
    /// Critical events that must be processed immediately
    Critical,
    /// High priority events for important operations
    High,
    /// Normal priority events for standard operations
    Normal,
    /// Low priority events for background operations
    Low,
}

impl Default for EventPriority {
    fn default() -> Self {
        EventPriority::Normal
    }
}

/// Event categories for classification and routing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// Workflow-related events
    Workflow,
    /// System operation events
    System,
    /// User activity and interaction events
    UserActivity,
    /// Security and authentication events
    Security,
    /// Integration and external service events
    Integration,
    /// Data processing and analytics events
    DataProcessing,
    /// Notification and communication events
    Notification,
    /// Error and failure events
    Error,
    /// Audit and compliance events
    Audit,
    /// Custom application-specific events
    Custom(String),
}

/// Event processing status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// Event is pending processing
    Pending,
    /// Event is currently being processed
    Processing,
    /// Event was processed successfully
    Completed,
    /// Event processing failed
    Failed,
    /// Event was retried
    Retried,
    /// Event was moved to dead letter queue
    DeadLetter,
    /// Event was skipped due to filtering
    Skipped,
}

/// Event source information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSource {
    /// Service or component that generated the event
    pub service: String,

    /// Version of the service
    pub version: String,

    /// Instance ID of the service
    pub instance_id: Option<String>,

    /// Hostname or container ID
    pub hostname: Option<String>,

    /// Additional source metadata
    pub metadata: HashMap<String, String>,
}

/// Event destination configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDestination {
    /// Target service or endpoint
    pub target: String,

    /// Routing key or topic
    pub routing_key: Option<String>,

    /// Destination-specific configuration
    pub config: HashMap<String, String>,
}

/// Event correlation information for tracking related events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct EventCorrelation {
    /// Correlation ID to group related events
    pub correlation_id: Uuid,

    /// Causation ID for event chains
    pub causation_id: Option<Uuid>,

    /// Parent event ID if this is a child event
    pub parent_event_id: Option<Uuid>,

    /// Trace ID for distributed tracing
    pub trace_id: Option<String>,

    /// Span ID for distributed tracing
    pub span_id: Option<String>,
}

/// Event retry configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Backoff strategy
    pub backoff_strategy: BackoffStrategy,

    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,

    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Backoff strategies for retry mechanisms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Linear increase in delay
    Linear,
    /// Exponential increase in delay
    Exponential,
    /// Custom backoff with jitter
    ExponentialWithJitter,
}

/// Stream configuration for different messaging systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Stream name or topic
    pub name: String,

    /// Number of partitions for Kafka topics
    pub partitions: Option<u32>,

    /// Replication factor for Kafka topics
    pub replication_factor: Option<u16>,

    /// Retention settings
    pub retention: Option<RetentionConfig>,

    /// Compression settings
    pub compression: Option<CompressionType>,

    /// Additional stream-specific configuration
    pub properties: HashMap<String, String>,
}

/// Retention configuration for events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Retention period in seconds
    pub retention_seconds: Option<u64>,

    /// Maximum size in bytes
    pub max_size_bytes: Option<u64>,

    /// Maximum number of events
    pub max_events: Option<u64>,
}

/// Compression types for event storage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionType {
    /// No compression
    None,
    /// GZIP compression
    Gzip,
    /// LZ4 compression
    Lz4,
    /// Zstandard compression
    Zstd,
    /// Snappy compression
    Snappy,
}

/// Event filter configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter name/identifier
    pub name: String,

    /// Event categories to include/exclude
    pub categories: Option<Vec<EventCategory>>,

    /// Event priorities to include/exclude
    pub priorities: Option<Vec<EventPriority>>,

    /// Source services to include/exclude
    pub sources: Option<Vec<String>>,

    /// JSONPath expressions for content filtering
    pub content_filters: Option<Vec<String>>,

    /// Whether this is an inclusion or exclusion filter
    pub include: bool,
}

/// Event transformation configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTransformation {
    /// Transformation name/identifier
    pub name: String,

    /// Transformation type
    pub transform_type: TransformationType,

    /// Transformation configuration
    pub config: HashMap<String, String>,

    /// Whether transformation is required or optional
    pub required: bool,
}

/// Types of event transformations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformationType {
    /// Field mapping and renaming
    FieldMapping,
    /// Data format conversion
    FormatConversion,
    /// Content enrichment
    Enrichment,
    /// Data sanitization
    Sanitization,
    /// Custom transformation script
    Custom,
}

/// Dead letter queue configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadLetterConfig {
    /// Dead letter queue name
    pub queue_name: String,

    /// Maximum time to keep events in DLQ
    pub retention_seconds: u64,

    /// Whether to enable automatic replay from DLQ
    pub auto_replay: bool,

    /// Replay configuration if auto_replay is enabled
    pub replay_config: Option<ReplayConfig>,
}

/// Event replay configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// Replay batch size
    pub batch_size: u32,

    /// Delay between batches in milliseconds
    pub batch_delay_ms: u64,

    /// Maximum concurrent replays
    pub max_concurrent: u32,

    /// Whether to preserve original timestamps
    pub preserve_timestamps: bool,
}

/// Health check status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Service status is unknown
    Unknown,
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStatus::Pending => write!(f, "pending"),
            EventStatus::Processing => write!(f, "processing"),
            EventStatus::Completed => write!(f, "completed"),
            EventStatus::Failed => write!(f, "failed"),
            EventStatus::Retried => write!(f, "retried"),
            EventStatus::DeadLetter => write!(f, "dead_letter"),
            EventStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Component health information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub component: String,

    /// Health status
    pub status: HealthStatus,

    /// Last check timestamp
    pub last_check: DateTime<Utc>,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Additional health details
    pub details: HashMap<String, String>,
}

/// Service metrics snapshot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total events processed
    pub events_processed: u64,

    /// Events per second (current rate)
    pub events_per_second: f64,

    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,

    /// Error rate percentage
    pub error_rate: f64,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Active connections
    pub active_connections: u32,

    /// Timestamp of metrics collection
    pub timestamp: DateTime<Utc>,
}

/// Event processing statistics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Total events received
    pub total_received: u64,

    /// Total events processed successfully
    pub total_processed: u64,

    /// Total events failed
    pub total_failed: u64,

    /// Total events in dead letter queue
    pub total_dead_letter: u64,

    /// Total events filtered out
    pub total_filtered: u64,

    /// Processing statistics by category
    pub by_category: HashMap<EventCategory, u64>,

    /// Processing statistics by priority
    pub by_priority: HashMap<EventPriority, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_priority_default() {
        assert_eq!(EventPriority::default(), EventPriority::Normal);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.backoff_strategy, BackoffStrategy::Exponential);
    }

    #[test]
    fn test_event_source_validation() {
        let source = EventSource {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: Some("instance-1".to_string()),
            hostname: Some("host-1".to_string()),
            metadata: HashMap::new(),
        };

        assert!(source.validate().is_ok());
    }

    #[test]
    fn test_event_category_custom() {
        let category = EventCategory::Custom("my-custom-event".to_string());
        match category {
            EventCategory::Custom(name) => assert_eq!(name, "my-custom-event"),
            _ => panic!("Expected custom category"),
        }
    }
}
