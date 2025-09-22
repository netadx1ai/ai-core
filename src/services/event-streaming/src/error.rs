//! # Error Handling Module
//!
//! This module defines comprehensive error types for the event streaming service.
//! It provides structured error handling with context, retry information, and
//! integration with the broader error handling ecosystem.

use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Main error type for the event streaming service
#[derive(Error, Debug)]
pub enum EventStreamingError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Kafka-related errors
    #[error("Kafka error: {message}")]
    Kafka {
        message: String,
        topic: Option<String>,
        partition: Option<i32>,
        offset: Option<i64>,
        retry_after: Option<u64>,
    },

    /// Redis Streams errors
    #[error("Redis error: {message}")]
    Redis {
        message: String,
        stream: Option<String>,
        consumer_group: Option<String>,
        retry_after: Option<u64>,
    },

    /// RabbitMQ errors
    #[error("RabbitMQ error: {message}")]
    RabbitMQ {
        message: String,
        exchange: Option<String>,
        queue: Option<String>,
        routing_key: Option<String>,
    },

    /// Event processing errors
    #[error("Processing error: {message}")]
    Processing {
        message: String,
        event_id: Option<Uuid>,
        event_type: Option<String>,
        processor: Option<String>,
        retryable: bool,
    },

    /// Event validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        value: Option<String>,
        event_id: Option<Uuid>,
    },

    /// Event serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        event_id: Option<Uuid>,
        format: Option<String>,
    },

    /// Storage and database errors
    #[error("Storage error: {message}")]
    Storage {
        message: String,
        operation: Option<String>,
        event_id: Option<Uuid>,
        table: Option<String>,
    },

    /// Network and connectivity errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        endpoint: Option<String>,
        status_code: Option<u16>,
        retry_after: Option<u64>,
    },

    /// Authentication and authorization errors
    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        user_id: Option<String>,
        token_type: Option<String>,
    },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        limit: u32,
        window_seconds: u64,
        retry_after: u64,
    },

    /// Timeout errors
    #[error("Timeout error: {message}")]
    Timeout {
        message: String,
        operation: String,
        timeout_seconds: u64,
    },

    /// Dead letter queue errors
    #[error("Dead letter queue error: {message}")]
    DeadLetter {
        message: String,
        event_id: Uuid,
        queue_name: String,
        reason: String,
    },

    /// Event filter errors
    #[error("Filter error: {message}")]
    Filter {
        message: String,
        filter_name: String,
        event_id: Option<Uuid>,
    },

    /// Event transformation errors
    #[error("Transformation error: {message}")]
    Transformation {
        message: String,
        transformer_name: String,
        event_id: Uuid,
        input_data: Option<String>,
    },

    /// Service discovery and health errors
    #[error("Service error: {message}")]
    Service {
        message: String,
        service_name: String,
        status: ServiceStatus,
    },

    /// Monitoring and metrics errors
    #[error("Monitoring error: {message}")]
    Monitoring {
        message: String,
        component: String,
        metric_name: Option<String>,
    },

    /// External integration errors
    #[error("Integration error: {message}")]
    Integration {
        message: String,
        service: String,
        operation: String,
        status_code: Option<u16>,
        retry_after: Option<u64>,
    },

    /// Resource exhaustion errors
    #[error("Resource exhaustion: {message}")]
    ResourceExhaustion {
        message: String,
        resource_type: String,
        current_usage: u64,
        limit: u64,
    },

    /// Internal service errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        error_code: Option<String>,
        context: Option<String>,
    },
}

/// Service health status for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStatus::Healthy => write!(f, "healthy"),
            ServiceStatus::Degraded => write!(f, "degraded"),
            ServiceStatus::Unhealthy => write!(f, "unhealthy"),
            ServiceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Error severity levels for categorization and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "low"),
            ErrorSeverity::Medium => write!(f, "medium"),
            ErrorSeverity::High => write!(f, "high"),
            ErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl EventStreamingError {
    /// Get the error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            EventStreamingError::Configuration { .. } => ErrorSeverity::High,
            EventStreamingError::Kafka { .. } => ErrorSeverity::Medium,
            EventStreamingError::Redis { .. } => ErrorSeverity::Medium,
            EventStreamingError::RabbitMQ { .. } => ErrorSeverity::Medium,
            EventStreamingError::Processing { .. } => ErrorSeverity::Medium,
            EventStreamingError::Validation { .. } => ErrorSeverity::Low,
            EventStreamingError::Serialization { .. } => ErrorSeverity::Low,
            EventStreamingError::Storage { .. } => ErrorSeverity::High,
            EventStreamingError::Network { .. } => ErrorSeverity::Medium,
            EventStreamingError::Authentication { .. } => ErrorSeverity::High,
            EventStreamingError::RateLimit { .. } => ErrorSeverity::Low,
            EventStreamingError::Timeout { .. } => ErrorSeverity::Medium,
            EventStreamingError::DeadLetter { .. } => ErrorSeverity::Medium,
            EventStreamingError::Filter { .. } => ErrorSeverity::Low,
            EventStreamingError::Transformation { .. } => ErrorSeverity::Medium,
            EventStreamingError::Service { status, .. } => match status {
                ServiceStatus::Healthy => ErrorSeverity::Low,
                ServiceStatus::Degraded => ErrorSeverity::Medium,
                ServiceStatus::Unhealthy => ErrorSeverity::High,
                ServiceStatus::Unknown => ErrorSeverity::Medium,
            },
            EventStreamingError::Monitoring { .. } => ErrorSeverity::Low,
            EventStreamingError::Integration { .. } => ErrorSeverity::Medium,
            EventStreamingError::ResourceExhaustion { .. } => ErrorSeverity::High,
            EventStreamingError::Internal { .. } => ErrorSeverity::Critical,
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            EventStreamingError::Configuration { .. } => false,
            EventStreamingError::Kafka { retry_after, .. } => retry_after.is_some(),
            EventStreamingError::Redis { retry_after, .. } => retry_after.is_some(),
            EventStreamingError::RabbitMQ { .. } => true,
            EventStreamingError::Processing { retryable, .. } => *retryable,
            EventStreamingError::Validation { .. } => false,
            EventStreamingError::Serialization { .. } => false,
            EventStreamingError::Storage { .. } => true,
            EventStreamingError::Network { .. } => true,
            EventStreamingError::Authentication { .. } => false,
            EventStreamingError::RateLimit { .. } => true,
            EventStreamingError::Timeout { .. } => true,
            EventStreamingError::DeadLetter { .. } => false,
            EventStreamingError::Filter { .. } => false,
            EventStreamingError::Transformation { .. } => false,
            EventStreamingError::Service { status, .. } => {
                matches!(status, ServiceStatus::Degraded | ServiceStatus::Unknown)
            }
            EventStreamingError::Monitoring { .. } => true,
            EventStreamingError::Integration { .. } => true,
            EventStreamingError::ResourceExhaustion { .. } => false,
            EventStreamingError::Internal { .. } => false,
        }
    }

    /// Get retry delay in seconds
    pub fn retry_delay_seconds(&self) -> Option<u64> {
        match self {
            EventStreamingError::Kafka { retry_after, .. } => *retry_after,
            EventStreamingError::Redis { retry_after, .. } => *retry_after,
            EventStreamingError::Network { retry_after, .. } => *retry_after,
            EventStreamingError::RateLimit { retry_after, .. } => Some(*retry_after),
            EventStreamingError::Integration { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Get the error category for metrics and monitoring
    pub fn category(&self) -> &'static str {
        match self {
            EventStreamingError::Configuration { .. } => "configuration",
            EventStreamingError::Kafka { .. } => "kafka",
            EventStreamingError::Redis { .. } => "redis",
            EventStreamingError::RabbitMQ { .. } => "rabbitmq",
            EventStreamingError::Processing { .. } => "processing",
            EventStreamingError::Validation { .. } => "validation",
            EventStreamingError::Serialization { .. } => "serialization",
            EventStreamingError::Storage { .. } => "storage",
            EventStreamingError::Network { .. } => "network",
            EventStreamingError::Authentication { .. } => "authentication",
            EventStreamingError::RateLimit { .. } => "rate_limit",
            EventStreamingError::Timeout { .. } => "timeout",
            EventStreamingError::DeadLetter { .. } => "dead_letter",
            EventStreamingError::Filter { .. } => "filter",
            EventStreamingError::Transformation { .. } => "transformation",
            EventStreamingError::Service { .. } => "service",
            EventStreamingError::Monitoring { .. } => "monitoring",
            EventStreamingError::Integration { .. } => "integration",
            EventStreamingError::ResourceExhaustion { .. } => "resource_exhaustion",
            EventStreamingError::Internal { .. } => "internal",
        }
    }

    /// Create a configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a configuration error with source
    pub fn configuration_with_source<S: Into<String>>(message: S, _source: S) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a Kafka error
    pub fn kafka<S: Into<String>>(message: S) -> Self {
        Self::Kafka {
            message: message.into(),
            topic: None,
            partition: None,
            offset: None,
            retry_after: None,
        }
    }

    /// Create a Redis error
    pub fn redis<S: Into<String>>(message: S) -> Self {
        Self::Redis {
            message: message.into(),
            stream: None,
            consumer_group: None,
            retry_after: None,
        }
    }

    /// Create a processing error
    pub fn processing<S: Into<String>>(message: S, event_id: Uuid, retryable: bool) -> Self {
        Self::Processing {
            message: message.into(),
            event_id: Some(event_id),
            event_type: None,
            processor: None,
            retryable,
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
            value: None,
            event_id: None,
        }
    }

    /// Create a storage error
    pub fn storage<S: Into<String>>(message: S) -> Self {
        Self::Storage {
            message: message.into(),
            operation: None,
            event_id: None,
            table: None,
        }
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(message: S, operation: S, timeout_seconds: u64) -> Self {
        Self::Timeout {
            message: message.into(),
            operation: operation.into(),
            timeout_seconds,
        }
    }

    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>>(
        message: S,
        limit: u32,
        window_seconds: u64,
        retry_after: u64,
    ) -> Self {
        Self::RateLimit {
            message: message.into(),
            limit,
            window_seconds,
            retry_after,
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
            error_code: None,
            context: None,
        }
    }
}

/// Result type for event streaming operations
pub type Result<T> = std::result::Result<T, EventStreamingError>;

/// Error context for additional error information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Event ID associated with the error
    pub event_id: Option<Uuid>,

    /// Correlation ID for tracking related errors
    pub correlation_id: Option<Uuid>,

    /// Service component where error occurred
    pub component: Option<String>,

    /// Operation being performed when error occurred
    pub operation: Option<String>,

    /// User ID if applicable
    pub user_id: Option<String>,

    /// Request ID for API requests
    pub request_id: Option<String>,

    /// Additional context data
    pub metadata: std::collections::HashMap<String, String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            event_id: None,
            correlation_id: None,
            component: None,
            operation: None,
            user_id: None,
            request_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the event ID
    pub fn with_event_id(mut self, event_id: Uuid) -> Self {
        self.event_id = Some(event_id);
        self
    }

    /// Set the correlation ID
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set the component
    pub fn with_component<S: Into<String>>(mut self, component: S) -> Self {
        self.component = Some(component.into());
        self
    }

    /// Set the operation
    pub fn with_operation<S: Into<String>>(mut self, operation: S) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Add metadata
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// Implement From traits for common error conversions

impl From<serde_json::Error> for EventStreamingError {
    fn from(err: serde_json::Error) -> Self {
        EventStreamingError::Serialization {
            message: err.to_string(),
            event_id: None,
            format: Some("json".to_string()),
        }
    }
}

impl From<sqlx::Error> for EventStreamingError {
    fn from(err: sqlx::Error) -> Self {
        EventStreamingError::Storage {
            message: err.to_string(),
            operation: None,
            event_id: None,
            table: None,
        }
    }
}

impl From<redis::RedisError> for EventStreamingError {
    fn from(err: redis::RedisError) -> Self {
        EventStreamingError::Redis {
            message: err.to_string(),
            stream: None,
            consumer_group: None,
            retry_after: Some(5), // Default 5 second retry
        }
    }
}

impl From<rdkafka::error::KafkaError> for EventStreamingError {
    fn from(err: rdkafka::error::KafkaError) -> Self {
        EventStreamingError::Kafka {
            message: err.to_string(),
            topic: None,
            partition: None,
            offset: None,
            retry_after: Some(10), // Default 10 second retry
        }
    }
}

impl From<reqwest::Error> for EventStreamingError {
    fn from(err: reqwest::Error) -> Self {
        let status_code = err.status().map(|s| s.as_u16());
        EventStreamingError::Network {
            message: err.to_string(),
            endpoint: err.url().map(|u| u.to_string()),
            status_code,
            retry_after: Some(30), // Default 30 second retry for network errors
        }
    }
}

impl From<tokio::time::error::Elapsed> for EventStreamingError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        EventStreamingError::Timeout {
            message: err.to_string(),
            operation: "unknown".to_string(),
            timeout_seconds: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        let config_error = EventStreamingError::configuration("test");
        assert_eq!(config_error.severity(), ErrorSeverity::High);

        let validation_error = EventStreamingError::validation("test");
        assert_eq!(validation_error.severity(), ErrorSeverity::Low);

        let internal_error = EventStreamingError::internal("test");
        assert_eq!(internal_error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_retryability() {
        let config_error = EventStreamingError::configuration("test");
        assert!(!config_error.is_retryable());

        let network_error = EventStreamingError::Network {
            message: "test".to_string(),
            endpoint: None,
            status_code: None,
            retry_after: Some(30),
        };
        assert!(network_error.is_retryable());

        let processing_error = EventStreamingError::processing("test", Uuid::new_v4(), true);
        assert!(processing_error.is_retryable());
    }

    #[test]
    fn test_error_category() {
        let kafka_error = EventStreamingError::kafka("test");
        assert_eq!(kafka_error.category(), "kafka");

        let redis_error = EventStreamingError::redis("test");
        assert_eq!(redis_error.category(), "redis");

        let storage_error = EventStreamingError::storage("test");
        assert_eq!(storage_error.category(), "storage");
    }

    #[test]
    fn test_retry_delay() {
        let rate_limit_error = EventStreamingError::rate_limit("test", 100, 60, 30);
        assert_eq!(rate_limit_error.retry_delay_seconds(), Some(30));

        let validation_error = EventStreamingError::validation("test");
        assert_eq!(validation_error.retry_delay_seconds(), None);
    }

    #[test]
    fn test_error_context() {
        let event_id = Uuid::new_v4();
        let correlation_id = Uuid::new_v4();

        let context = ErrorContext::new()
            .with_event_id(event_id)
            .with_correlation_id(correlation_id)
            .with_component("test-component")
            .with_operation("test-operation")
            .with_metadata("key", "value");

        assert_eq!(context.event_id, Some(event_id));
        assert_eq!(context.correlation_id, Some(correlation_id));
        assert_eq!(context.component, Some("test-component".to_string()));
        assert_eq!(context.operation, Some("test-operation".to_string()));
        assert_eq!(context.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_error_conversions() {
        let json_error =
            serde_json::Error::syntax(serde_json::error::ErrorCode::ExpectedColon, 1, 1);
        let streaming_error: EventStreamingError = json_error.into();
        assert!(matches!(
            streaming_error,
            EventStreamingError::Serialization { .. }
        ));

        let timeout_error = tokio::time::error::Elapsed::new();
        let streaming_timeout: EventStreamingError = timeout_error.into();
        assert!(matches!(
            streaming_timeout,
            EventStreamingError::Timeout { .. }
        ));
    }

    #[test]
    fn test_service_status_display() {
        assert_eq!(ServiceStatus::Healthy.to_string(), "healthy");
        assert_eq!(ServiceStatus::Degraded.to_string(), "degraded");
        assert_eq!(ServiceStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(ServiceStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Low < ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium < ErrorSeverity::High);
        assert!(ErrorSeverity::High < ErrorSeverity::Critical);
    }
}
