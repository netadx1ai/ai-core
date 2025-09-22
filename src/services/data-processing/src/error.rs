//! Error handling module for the Data Processing Service
//!
//! This module defines all error types used throughout the data processing service,
//! providing comprehensive error handling for stream processing, batch operations,
//! analytics, and system-level failures.

use std::fmt;
use thiserror::Error;

/// Result type alias for data processing operations
pub type Result<T> = std::result::Result<T, DataProcessingError>;

/// Comprehensive error types for the data processing service
#[derive(Error, Debug)]
pub enum DataProcessingError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Kafka-related errors
    #[error("Kafka error: {source}")]
    Kafka {
        #[from]
        source: KafkaError,
    },

    /// ClickHouse-related errors
    #[error("ClickHouse error: {source}")]
    ClickHouse {
        #[from]
        source: ClickHouseError,
    },

    /// Stream processing errors
    #[error("Stream processing error: {source}")]
    StreamProcessing {
        #[from]
        source: StreamProcessingError,
    },

    /// Batch processing errors
    #[error("Batch processing error: {source}")]
    BatchProcessing {
        #[from]
        source: BatchProcessingError,
    },

    /// Data transformation errors
    #[error("Data transformation error: {source}")]
    DataTransformation {
        #[from]
        source: TransformationError,
    },

    /// Serialization/Deserialization errors
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Network and I/O errors
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// Database connection errors
    #[error("Database connection error: {message}")]
    DatabaseConnection { message: String },

    /// Authentication and authorization errors
    #[error("Authentication error: {message}")]
    Authentication { message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    /// Resource exhaustion errors
    #[error("Resource exhausted: {resource_type} - {message}")]
    ResourceExhausted {
        resource_type: String,
        message: String,
    },

    /// Timeout errors
    #[error("Operation timed out: {operation} after {timeout_secs}s")]
    Timeout {
        operation: String,
        timeout_secs: u64,
    },

    /// Validation errors
    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    /// Internal system errors
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// External service errors
    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    /// Concurrent processing errors
    #[error("Concurrency error: {message}")]
    Concurrency { message: String },

    /// Data quality errors
    #[error("Data quality error: {issue} - {details}")]
    DataQuality { issue: String, details: String },

    /// Schema-related errors
    #[error("Schema error: {schema_name} - {message}")]
    Schema {
        schema_name: String,
        message: String,
    },

    /// Metrics and monitoring errors
    #[error("Metrics error: {message}")]
    Metrics { message: String },

    /// Health check errors
    #[error("Health check failed: {component} - {message}")]
    HealthCheck { component: String, message: String },
}

/// Kafka-specific errors
#[derive(Error, Debug)]
pub enum KafkaError {
    #[error("Connection failed: {message}")]
    Connection { message: String },

    #[error("Producer error: {message}")]
    Producer { message: String },

    #[error("Consumer error: {message}")]
    Consumer { message: String },

    #[error("Topic error: {topic} - {message}")]
    Topic { topic: String, message: String },

    #[error("Offset error: {message}")]
    Offset { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Timeout error: {operation} - {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },

    #[error("Configuration error: {parameter} - {message}")]
    Configuration { parameter: String, message: String },
}

/// ClickHouse-specific errors
#[derive(Error, Debug)]
pub enum ClickHouseError {
    #[error("Connection failed: {message}")]
    Connection { message: String },

    #[error("Query error: {query} - {message}")]
    Query { query: String, message: String },

    #[error("Insert error: {table} - {message}")]
    Insert { table: String, message: String },

    #[error("Schema error: {table} - {message}")]
    Schema { table: String, message: String },

    #[error("Type conversion error: {from_type} to {to_type} - {message}")]
    TypeConversion {
        from_type: String,
        to_type: String,
        message: String,
    },

    #[error("Transaction error: {message}")]
    Transaction { message: String },

    #[error("Performance error: {operation} took {duration_ms}ms")]
    Performance { operation: String, duration_ms: u64 },
}

/// Stream processing specific errors
#[derive(Error, Debug)]
pub enum StreamProcessingError {
    #[error("Worker error: {worker_id} - {message}")]
    Worker { worker_id: String, message: String },

    #[error("Buffer overflow: {buffer_name} - capacity {capacity}")]
    BufferOverflow {
        buffer_name: String,
        capacity: usize,
    },

    #[error("Backpressure detected: {component} - {message}")]
    Backpressure { component: String, message: String },

    #[error("Window operation error: {window_type} - {message}")]
    Window {
        window_type: String,
        message: String,
    },

    #[error("Checkpoint error: {checkpoint_id} - {message}")]
    Checkpoint {
        checkpoint_id: String,
        message: String,
    },

    #[error("State management error: {state_key} - {message}")]
    State { state_key: String, message: String },

    #[error("Watermark error: {message}")]
    Watermark { message: String },

    #[error("Event ordering error: {message}")]
    EventOrdering { message: String },
}

/// Batch processing specific errors
#[derive(Error, Debug)]
pub enum BatchProcessingError {
    #[error("Job error: {job_id} - {message}")]
    Job { job_id: String, message: String },

    #[error("Queue full: {queue_name} - capacity {capacity}")]
    QueueFull { queue_name: String, capacity: usize },

    #[error("Worker pool error: {message}")]
    WorkerPool { message: String },

    #[error("Resource allocation error: {resource_type} - {message}")]
    ResourceAllocation {
        resource_type: String,
        message: String,
    },

    #[error("Schedule error: {schedule} - {message}")]
    Schedule { schedule: String, message: String },

    #[error("Chunk processing error: {chunk_id} - {message}")]
    ChunkProcessing { chunk_id: String, message: String },

    #[error("File system error: {operation} - {path} - {message}")]
    FileSystem {
        operation: String,
        path: String,
        message: String,
    },

    #[error("Memory limit exceeded: {limit_gb}GB - {current_gb}GB")]
    MemoryLimit { limit_gb: usize, current_gb: usize },
}

/// Data transformation specific errors
#[derive(Error, Debug)]
pub enum TransformationError {
    #[error("Parser error: {format} - {message}")]
    Parser { format: String, message: String },

    #[error("Converter error: {from_format} to {to_format} - {message}")]
    Converter {
        from_format: String,
        to_format: String,
        message: String,
    },

    #[error("Field error: {field_name} - {message}")]
    Field { field_name: String, message: String },

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Missing required field: {field_name}")]
    MissingField { field_name: String },

    #[error("Invalid value: {field_name} = {value} - {message}")]
    InvalidValue {
        field_name: String,
        value: String,
        message: String,
    },

    #[error("Schema evolution error: {old_version} to {new_version} - {message}")]
    SchemaEvolution {
        old_version: String,
        new_version: String,
        message: String,
    },

    #[error("Enrichment error: {enrichment_type} - {message}")]
    Enrichment {
        enrichment_type: String,
        message: String,
    },
}

impl DataProcessingError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Create a database connection error
    pub fn database_connection(message: impl Into<String>) -> Self {
        Self::DatabaseConnection {
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit {
            message: message.into(),
        }
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted(
        resource_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ResourceExhausted {
            resource_type: resource_type.into(),
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, timeout_secs: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            timeout_secs,
        }
    }

    /// Create a validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create an external service error
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create a data quality error
    pub fn data_quality(issue: impl Into<String>, details: impl Into<String>) -> Self {
        Self::DataQuality {
            issue: issue.into(),
            details: details.into(),
        }
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Kafka { source } => source.is_retryable(),
            Self::ClickHouse { source } => source.is_retryable(),
            Self::StreamProcessing { source } => source.is_retryable(),
            Self::BatchProcessing { source } => source.is_retryable(),
            Self::Timeout { .. } => true,
            Self::ResourceExhausted { .. } => true,
            Self::RateLimit { .. } => true,
            Self::ExternalService { .. } => true,
            Self::Concurrency { .. } => true,
            _ => false,
        }
    }

    /// Get the error category for metrics and logging
    pub fn category(&self) -> &'static str {
        match self {
            Self::Configuration { .. } => "configuration",
            Self::Kafka { .. } => "kafka",
            Self::ClickHouse { .. } => "clickhouse",
            Self::StreamProcessing { .. } => "stream_processing",
            Self::BatchProcessing { .. } => "batch_processing",
            Self::DataTransformation { .. } => "data_transformation",
            Self::Serialization { .. } => "serialization",
            Self::Io { .. } => "io",
            Self::DatabaseConnection { .. } => "database",
            Self::Authentication { .. } => "auth",
            Self::RateLimit { .. } => "rate_limit",
            Self::ResourceExhausted { .. } => "resource",
            Self::Timeout { .. } => "timeout",
            Self::Validation { .. } => "validation",
            Self::Internal { .. } => "internal",
            Self::ExternalService { .. } => "external",
            Self::Concurrency { .. } => "concurrency",
            Self::DataQuality { .. } => "data_quality",
            Self::Schema { .. } => "schema",
            Self::Metrics { .. } => "metrics",
            Self::HealthCheck { .. } => "health",
        }
    }
}

impl KafkaError {
    /// Check if this Kafka error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Connection { .. } => true,
            Self::Timeout { .. } => true,
            Self::Topic { .. } => false,
            Self::Authentication { .. } => false,
            Self::Configuration { .. } => false,
            _ => true,
        }
    }
}

impl ClickHouseError {
    /// Check if this ClickHouse error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Connection { .. } => true,
            Self::Performance { .. } => true,
            Self::Transaction { .. } => true,
            Self::Schema { .. } => false,
            Self::TypeConversion { .. } => false,
            _ => true,
        }
    }
}

impl StreamProcessingError {
    /// Check if this stream processing error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::BufferOverflow { .. } => true,
            Self::Backpressure { .. } => true,
            Self::Checkpoint { .. } => true,
            Self::EventOrdering { .. } => false,
            _ => true,
        }
    }
}

impl BatchProcessingError {
    /// Check if this batch processing error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::QueueFull { .. } => true,
            Self::ResourceAllocation { .. } => true,
            Self::MemoryLimit { .. } => true,
            Self::FileSystem { .. } => true,
            _ => true,
        }
    }
}

// Implement conversions from common error types
impl From<serde_json::Error> for DataProcessingError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization(err.to_string())
    }
}

impl From<config::ConfigError> for DataProcessingError {
    fn from(err: config::ConfigError) -> Self {
        Self::configuration(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for DataProcessingError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::timeout("async operation", 0)
    }
}

impl From<rdkafka::error::KafkaError> for KafkaError {
    fn from(err: rdkafka::error::KafkaError) -> Self {
        match err {
            rdkafka::error::KafkaError::ClientConfig(_, msg, _, _) => Self::Configuration {
                parameter: "client_config".to_string(),
                message: msg,
            },
            _ => Self::Connection {
                message: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = DataProcessingError::configuration("test config error");
        assert_eq!(err.category(), "configuration");
    }

    #[test]
    fn test_retryable_errors() {
        let timeout_err = DataProcessingError::timeout("test operation", 30);
        assert!(timeout_err.is_retryable());

        let validation_err = DataProcessingError::validation("field", "invalid");
        assert!(!validation_err.is_retryable());
    }

    #[test]
    fn test_kafka_error_conversion() {
        let kafka_err = KafkaError::Connection {
            message: "connection failed".to_string(),
        };
        let processing_err: DataProcessingError = kafka_err.into();
        assert_eq!(processing_err.category(), "kafka");
    }

    #[test]
    fn test_error_categories() {
        let errors = [
            DataProcessingError::configuration("test"),
            DataProcessingError::serialization("test"),
            DataProcessingError::authentication("test"),
            DataProcessingError::timeout("test", 10),
        ];

        let categories: Vec<&str> = errors.iter().map(|e| e.category()).collect();
        assert_eq!(
            categories,
            ["configuration", "serialization", "auth", "timeout"]
        );
    }
}
