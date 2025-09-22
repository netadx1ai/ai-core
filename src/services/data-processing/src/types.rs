//! Type definitions for the Data Processing Service
//!
//! This module contains all the data structures and types used throughout
//! the data processing service, including stream processing records, batch jobs,
//! analytics queries, and system monitoring types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A data record for stream processing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataRecord {
    /// Unique record identifier
    pub id: Uuid,
    /// Record timestamp
    pub timestamp: DateTime<Utc>,
    /// Data source identifier
    pub source: String,
    /// Record type/category
    pub record_type: String,
    /// Main data payload
    pub data: serde_json::Value,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Processing context
    pub context: ProcessingContext,
    /// Record schema version
    pub schema_version: String,
    /// Data quality score (0-100)
    pub quality_score: Option<f64>,
    /// Partition key for distributed processing
    pub partition_key: String,
}

/// Processing context for data records
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessingContext {
    /// Trace ID for distributed tracing
    pub trace_id: String,
    /// User or system that initiated the processing
    pub initiator: String,
    /// Processing pipeline identifier
    pub pipeline_id: String,
    /// Processing stage
    pub stage: ProcessingStage,
    /// Retry attempt number
    pub retry_count: u32,
    /// Processing priority (1-10, 10 being highest)
    pub priority: u8,
    /// Processing deadline
    pub deadline: Option<DateTime<Utc>>,
    /// Custom processing flags
    pub flags: HashMap<String, bool>,
}

/// Processing stages in the data pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStage {
    /// Initial ingestion stage
    Ingestion,
    /// Data validation and cleaning
    Validation,
    /// Data transformation
    Transformation,
    /// Data enrichment
    Enrichment,
    /// Analytics processing
    Analytics,
    /// Final output stage
    Output,
}

/// Result of data record processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// Original record ID
    pub record_id: Uuid,
    /// Processing status
    pub status: ProcessingStatus,
    /// Transformed/processed data
    pub processed_data: Option<serde_json::Value>,
    /// Processing metrics
    pub metrics: ProcessingMetrics,
    /// Any errors encountered
    pub errors: Vec<ProcessingError>,
    /// Warnings generated during processing
    pub warnings: Vec<ProcessingWarning>,
    /// Output destinations
    pub outputs: Vec<OutputDestination>,
}

/// Processing status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStatus {
    /// Processing completed successfully
    Success,
    /// Processing failed
    Failed,
    /// Processing partially successful with warnings
    PartialSuccess,
    /// Processing skipped due to conditions
    Skipped,
    /// Processing retries exhausted
    Exhausted,
}

/// Metrics collected during processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetrics {
    /// Processing start time
    pub start_time: DateTime<Utc>,
    /// Processing end time
    pub end_time: DateTime<Utc>,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU time in milliseconds
    pub cpu_time_ms: u64,
    /// Number of transformations applied
    pub transformations_count: u32,
    /// Data size before processing
    pub input_size_bytes: u64,
    /// Data size after processing
    pub output_size_bytes: u64,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Processing error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Field or component that caused the error
    pub field: Option<String>,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
}

/// Processing warning information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
    /// Field or component that generated the warning
    pub field: Option<String>,
    /// Timestamp when warning occurred
    pub timestamp: DateTime<Utc>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    /// Low severity - processing can continue
    Low,
    /// Medium severity - may affect quality
    Medium,
    /// High severity - processing should be retried
    High,
    /// Critical severity - processing must stop
    Critical,
}

/// Output destination for processed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDestination {
    /// Destination type (kafka, clickhouse, file, etc.)
    pub destination_type: String,
    /// Destination identifier/address
    pub destination: String,
    /// Data format for output
    pub format: String,
    /// Compression type if any
    pub compression: Option<String>,
    /// Additional destination options
    pub options: HashMap<String, String>,
}

/// Batch processing job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJob {
    /// Unique job identifier
    pub id: Uuid,
    /// Job name
    pub name: String,
    /// Job description
    pub description: String,
    /// Job type
    pub job_type: BatchJobType,
    /// Input data configuration
    pub input_config: BatchInputConfig,
    /// Output data configuration
    pub output_config: BatchOutputConfig,
    /// Processing configuration
    pub processing_config: BatchProcessingConfig,
    /// Job schedule if recurring
    pub schedule: Option<JobSchedule>,
    /// Job priority (1-10)
    pub priority: u8,
    /// Job timeout in seconds
    pub timeout_secs: u64,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Job metadata
    pub metadata: HashMap<String, String>,
    /// Job creation time
    pub created_at: DateTime<Utc>,
    /// Job created by
    pub created_by: String,
}

/// Types of batch jobs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchJobType {
    /// ETL (Extract, Transform, Load) job
    Etl,
    /// Analytics computation job
    Analytics,
    /// Data migration job
    Migration,
    /// Data quality check job
    QualityCheck,
    /// Report generation job
    ReportGeneration,
    /// Data archival job
    Archival,
    /// Custom processing job
    Custom { job_class: String },
}

/// Batch job input configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInputConfig {
    /// Input source type
    pub source_type: String,
    /// Input source configuration
    pub source_config: HashMap<String, String>,
    /// Data format
    pub format: String,
    /// Data schema
    pub schema: Option<String>,
    /// Partition configuration
    pub partitions: Option<Vec<String>>,
    /// Date range for time-based data
    pub date_range: Option<DateRange>,
    /// Filters to apply
    pub filters: Vec<DataFilter>,
}

/// Batch job output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOutputConfig {
    /// Output destination type
    pub destination_type: String,
    /// Output destination configuration
    pub destination_config: HashMap<String, String>,
    /// Output format
    pub format: String,
    /// Compression settings
    pub compression: Option<CompressionConfig>,
    /// Partitioning strategy
    pub partitioning: Option<PartitioningConfig>,
    /// Output mode (overwrite, append, merge)
    pub mode: OutputMode,
}

/// Batch job processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessingConfig {
    /// Processing transformations
    pub transformations: Vec<TransformationConfig>,
    /// Aggregations to perform
    pub aggregations: Vec<AggregationConfig>,
    /// Quality checks to perform
    pub quality_checks: Vec<QualityCheckConfig>,
    /// Parallel processing configuration
    pub parallelism: ParallelismConfig,
    /// Checkpointing configuration
    pub checkpointing: Option<CheckpointConfig>,
}

/// Date range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start date (inclusive)
    pub start: DateTime<Utc>,
    /// End date (exclusive)
    pub end: DateTime<Utc>,
}

/// Data filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFilter {
    /// Field name to filter on
    pub field: String,
    /// Filter operator
    pub operator: FilterOperator,
    /// Filter value(s)
    pub value: serde_json::Value,
}

/// Filter operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterOperator {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal to
    Gte,
    /// Less than
    Lt,
    /// Less than or equal to
    Lte,
    /// In list
    In,
    /// Not in list
    NotIn,
    /// Contains
    Contains,
    /// Starts with
    StartsWith,
    /// Ends with
    EndsWith,
    /// Regular expression match
    Regex,
    /// Is null
    IsNull,
    /// Is not null
    IsNotNull,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Compression level (1-9)
    pub level: Option<u8>,
    /// Block size for compression
    pub block_size: Option<usize>,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// GZIP compression
    Gzip,
    /// LZ4 compression
    Lz4,
    /// ZSTD compression
    Zstd,
    /// Snappy compression
    Snappy,
}

/// Partitioning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitioningConfig {
    /// Partitioning strategy
    pub strategy: PartitioningStrategy,
    /// Partition columns
    pub columns: Vec<String>,
    /// Number of partitions (for hash partitioning)
    pub partition_count: Option<usize>,
}

/// Partitioning strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PartitioningStrategy {
    /// Hash-based partitioning
    Hash,
    /// Range-based partitioning
    Range,
    /// Time-based partitioning
    Time { unit: TimeUnit },
    /// Custom partitioning
    Custom { expression: String },
}

/// Time units for time-based partitioning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeUnit {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// Output modes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputMode {
    /// Overwrite existing data
    Overwrite,
    /// Append to existing data
    Append,
    /// Merge with existing data
    Merge,
    /// Error if data exists
    ErrorIfExists,
}

/// Transformation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationConfig {
    /// Transformation name
    pub name: String,
    /// Transformation type
    pub transformation_type: TransformationType,
    /// Transformation parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Fields to apply transformation to
    pub fields: Option<Vec<String>>,
    /// Condition for applying transformation
    pub condition: Option<String>,
}

/// Types of transformations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransformationType {
    /// Field mapping/renaming
    FieldMapping,
    /// Data type conversion
    TypeConversion,
    /// Value formatting
    Formatting,
    /// Data validation
    Validation,
    /// Data enrichment
    Enrichment,
    /// Custom transformation
    Custom { class_name: String },
}

/// Aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Aggregation name
    pub name: String,
    /// Aggregation function
    pub function: AggregationFunction,
    /// Fields to aggregate
    pub fields: Vec<String>,
    /// Group by fields
    pub group_by: Vec<String>,
    /// Filter condition
    pub filter: Option<String>,
}

/// Aggregation functions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    Variance,
    Percentile { percentile: f64 },
    CountDistinct,
    Custom { expression: String },
}

/// Quality check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheckConfig {
    /// Check name
    pub name: String,
    /// Check type
    pub check_type: QualityCheckType,
    /// Fields to check
    pub fields: Vec<String>,
    /// Check parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Severity of check failure
    pub severity: ErrorSeverity,
    /// Action to take on failure
    pub failure_action: QualityCheckAction,
}

/// Types of quality checks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityCheckType {
    /// Check for null values
    NotNull,
    /// Check value ranges
    Range,
    /// Check data uniqueness
    Unique,
    /// Check referential integrity
    ReferentialIntegrity,
    /// Check data format
    Format,
    /// Check data consistency
    Consistency,
    /// Custom quality check
    Custom { check_class: String },
}

/// Actions to take on quality check failure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityCheckAction {
    /// Continue processing with warning
    Warn,
    /// Skip the record/row
    Skip,
    /// Fail the entire job
    Fail,
    /// Quarantine the data
    Quarantine,
}

/// Parallelism configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismConfig {
    /// Number of parallel tasks
    pub parallelism: usize,
    /// Partitioning strategy for parallelism
    pub partitioning: ParallelismPartitioning,
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
}

/// Parallelism partitioning strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParallelismPartitioning {
    /// Round-robin partitioning
    RoundRobin,
    /// Hash-based partitioning
    Hash { fields: Vec<String> },
    /// Range-based partitioning
    Range { field: String },
    /// Custom partitioning
    Custom { strategy: String },
}

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Equal distribution
    Equal,
    /// Weighted distribution
    Weighted { weights: Vec<f64> },
    /// Dynamic load balancing
    Dynamic,
}

/// Checkpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Enable checkpointing
    pub enabled: bool,
    /// Checkpoint interval in seconds
    pub interval_secs: u64,
    /// Checkpoint storage location
    pub storage_location: String,
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: usize,
}

/// Job schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSchedule {
    /// Schedule type
    pub schedule_type: ScheduleType,
    /// Schedule parameters
    pub parameters: ScheduleParameters,
    /// Schedule timezone
    pub timezone: String,
    /// Schedule start date
    pub start_date: Option<DateTime<Utc>>,
    /// Schedule end date
    pub end_date: Option<DateTime<Utc>>,
}

/// Schedule types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScheduleType {
    /// One-time execution
    Once,
    /// Recurring execution
    Recurring,
    /// Event-triggered execution
    EventTriggered,
}

/// Schedule parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleParameters {
    /// Cron expression for recurring jobs
    pub cron_expression: Option<String>,
    /// Interval in seconds for recurring jobs
    pub interval_secs: Option<u64>,
    /// Event pattern for event-triggered jobs
    pub event_pattern: Option<String>,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Retry interval in seconds
    pub retry_interval_secs: u64,
}

/// Resource requirements for batch jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores required
    pub cpu_cores: f64,
    /// Memory in MB required
    pub memory_mb: u64,
    /// Disk space in MB required
    pub disk_mb: u64,
    /// GPU count required
    pub gpu_count: Option<u32>,
    /// Network bandwidth in Mbps
    pub network_mbps: Option<u64>,
    /// Custom resource requirements
    pub custom_resources: HashMap<String, String>,
}

/// Batch job status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJobStatus {
    /// Job ID
    pub job_id: Uuid,
    /// Current job state
    pub state: JobState,
    /// Job progress (0-100)
    pub progress: f64,
    /// Job start time
    pub started_at: Option<DateTime<Utc>>,
    /// Job completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Current processing stage
    pub current_stage: Option<String>,
    /// Records processed so far
    pub records_processed: u64,
    /// Total records to process
    pub total_records: Option<u64>,
    /// Job metrics
    pub metrics: JobMetrics,
    /// Job errors
    pub errors: Vec<ProcessingError>,
    /// Job warnings
    pub warnings: Vec<ProcessingWarning>,
    /// Job logs URL
    pub logs_url: Option<String>,
}

/// Job execution states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobState {
    /// Job is queued for execution
    Queued,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
    /// Job is paused
    Paused,
    /// Job is retrying after failure
    Retrying,
}

/// Job execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    /// Execution duration in seconds
    pub duration_secs: Option<u64>,
    /// CPU time used in seconds
    pub cpu_time_secs: f64,
    /// Peak memory usage in MB
    pub peak_memory_mb: u64,
    /// Disk I/O in MB
    pub disk_io_mb: u64,
    /// Network I/O in MB
    pub network_io_mb: u64,
    /// Throughput (records/sec)
    pub throughput_rps: f64,
    /// Error rate (0-1)
    pub error_rate: f64,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Overall service status
    pub status: HealthStatus,
    /// Component health statuses
    pub components: HashMap<String, ComponentHealth>,
    /// Last health check time
    pub last_check: DateTime<Utc>,
    /// Health check duration in milliseconds
    pub check_duration_ms: u64,
    /// Service uptime in seconds
    pub uptime_secs: u64,
    /// Service version
    pub version: String,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component health status
    pub status: HealthStatus,
    /// Component-specific details
    pub details: HashMap<String, String>,
    /// Last successful operation time
    pub last_success: Option<DateTime<Utc>>,
    /// Error count in last period
    pub error_count: u32,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Component is degraded but functional
    Degraded,
    /// Component is unhealthy
    Unhealthy,
    /// Component status is unknown
    Unknown,
}

/// Stream processing window types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowType {
    /// Tumbling window (non-overlapping)
    Tumbling { size_secs: u64 },
    /// Sliding window (overlapping)
    Sliding { size_secs: u64, slide_secs: u64 },
    /// Session window (based on gaps)
    Session { gap_secs: u64 },
    /// Global window (all events)
    Global,
}

/// Stream processing watermark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watermark {
    /// Watermark timestamp
    pub timestamp: DateTime<Utc>,
    /// Source that generated the watermark
    pub source: String,
    /// Watermark type
    pub watermark_type: WatermarkType,
}

/// Watermark types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WatermarkType {
    /// Event time watermark
    EventTime,
    /// Processing time watermark
    ProcessingTime,
    /// Ingestion time watermark
    IngestionTime,
}

impl Default for DataRecord {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "unknown".to_string(),
            record_type: "default".to_string(),
            data: serde_json::Value::Null,
            metadata: HashMap::new(),
            context: ProcessingContext::default(),
            schema_version: "1.0.0".to_string(),
            quality_score: None,
            partition_key: "default".to_string(),
        }
    }
}

impl Default for ProcessingContext {
    fn default() -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            initiator: "system".to_string(),
            pipeline_id: "default".to_string(),
            stage: ProcessingStage::Ingestion,
            retry_count: 0,
            priority: 5,
            deadline: None,
            flags: HashMap::new(),
        }
    }
}

impl Default for BatchJob {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "default_job".to_string(),
            description: "Default batch job".to_string(),
            job_type: BatchJobType::Etl,
            input_config: BatchInputConfig::default(),
            output_config: BatchOutputConfig::default(),
            processing_config: BatchProcessingConfig::default(),
            schedule: None,
            priority: 5,
            timeout_secs: 3600,
            resources: ResourceRequirements::default(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            created_by: "system".to_string(),
        }
    }
}

impl Default for BatchInputConfig {
    fn default() -> Self {
        Self {
            source_type: "file".to_string(),
            source_config: HashMap::new(),
            format: "json".to_string(),
            schema: None,
            partitions: None,
            date_range: None,
            filters: Vec::new(),
        }
    }
}

impl Default for BatchOutputConfig {
    fn default() -> Self {
        Self {
            destination_type: "file".to_string(),
            destination_config: HashMap::new(),
            format: "json".to_string(),
            compression: None,
            partitioning: None,
            mode: OutputMode::Append,
        }
    }
}

impl Default for BatchProcessingConfig {
    fn default() -> Self {
        Self {
            transformations: Vec::new(),
            aggregations: Vec::new(),
            quality_checks: Vec::new(),
            parallelism: ParallelismConfig::default(),
            checkpointing: None,
        }
    }
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        Self {
            parallelism: num_cpus::get(),
            partitioning: ParallelismPartitioning::RoundRobin,
            load_balancing: LoadBalancingStrategy::Equal,
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_mb: 1024,
            disk_mb: 1024,
            gpu_count: None,
            network_mbps: None,
            custom_resources: HashMap::new(),
        }
    }
}
