//! # Data Processing Service
//!
//! A high-performance data processing service for the AI-CORE platform that provides:
//! - High-throughput stream processing with Kafka integration
//! - Batch processing for analytics workloads with ClickHouse
//! - Real-time aggregations and metric computation
//! - Data transformation and enrichment pipelines
//! - Performance monitoring and auto-scaling capabilities
//!
//! ## Features
//!
//! ### Stream Processing
//! - Kafka consumer groups for distributed processing
//! - Real-time event stream processing with windowing
//! - Backpressure handling and flow control
//! - Exactly-once processing guarantees
//!
//! ### Analytics Processing
//! - Batch processing for large-scale analytics
//! - Integration with ClickHouse for columnar analytics
//! - Apache Arrow for efficient columnar operations
//! - DataFusion SQL engine for complex queries
//!
//! ### Data Transformation
//! - Schema evolution and data migration
//! - ETL pipelines with validation and error handling
//! - Data enrichment from multiple sources
//! - Format conversion (JSON, Avro, Parquet, CSV)
//!
//! ### Performance & Monitoring
//! - Real-time performance metrics
//! - Auto-scaling based on load
//! - Health monitoring and alerting
//! - Resource optimization and tuning
//!
//! ## Usage
//!
//! ```rust
//! use data_processing_service::{DataProcessingService, Config};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::from_env()?;
//!     let service = DataProcessingService::new(config).await?;
//!     service.start().await?;
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use thiserror::Error;

pub mod analytics;
pub mod batch;
pub mod config;
pub mod enrichment;
pub mod error;
pub mod health;
pub mod kafka;
pub mod metrics;
pub mod server;
pub mod stream;
pub mod transformations;
pub mod types;
pub mod windowing;

// Re-exports for convenience
pub use config::Config;
pub use error::{DataProcessingError, Result};
pub use metrics::MetricsCollector;
pub use server::DataProcessingServer;
pub use types::*;

/// Main data processing service that orchestrates all components
#[derive(Clone)]
pub struct DataProcessingService {
    config: Arc<Config>,
    metrics: Arc<MetricsCollector>,
    stream_processor: Arc<stream::StreamProcessor>,
    batch_processor: Arc<batch::BatchProcessor>,
    kafka_manager: Arc<kafka::KafkaManager>,
    health_checker: Arc<health::HealthChecker>,
}

impl DataProcessingService {
    /// Create a new data processing service instance
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let metrics = Arc::new(MetricsCollector::new(&config)?);

        // Initialize Kafka manager
        let kafka_manager = Arc::new(kafka::KafkaManager::new(&config, metrics.clone()).await?);

        // Initialize stream processor
        let stream_processor = Arc::new(
            stream::StreamProcessor::new(&config, metrics.clone(), kafka_manager.clone()).await?,
        );

        // Initialize batch processor
        let batch_processor = Arc::new(batch::BatchProcessor::new(&config, metrics.clone()).await?);

        // Initialize health checker with proper health checker instances
        let kafka_health_checker = Arc::new(health::KafkaHealthChecker::new(
            "kafka_manager".to_string(),
            config.kafka.bootstrap_servers.clone(),
        ))
            as Arc<dyn health::ComponentHealthChecker + Send + Sync>;

        let stream_health_checker = Arc::new(health::StreamProcessorHealthChecker::new(
            "stream_processor".to_string(),
        ))
            as Arc<dyn health::ComponentHealthChecker + Send + Sync>;

        let batch_health_checker = Arc::new(health::BatchProcessorHealthChecker::new(
            "batch_processor".to_string(),
        ))
            as Arc<dyn health::ComponentHealthChecker + Send + Sync>;

        let health_checker = Arc::new(
            health::HealthChecker::new(
                config.clone(),
                metrics.clone(),
                vec![
                    ("kafka_manager", kafka_health_checker),
                    ("stream_processor", stream_health_checker),
                    ("batch_processor", batch_health_checker),
                ],
            )
            .await?,
        );

        Ok(Self {
            config,
            metrics,
            stream_processor,
            batch_processor,
            kafka_manager,
            health_checker,
        })
    }

    /// Start all service components
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting Data Processing Service");

        // Start health checker first
        self.health_checker.start().await?;

        // Start Kafka manager
        self.kafka_manager.start().await?;

        // Start stream processor
        self.stream_processor.start().await?;

        // Start batch processor
        self.batch_processor.start().await?;

        tracing::info!("Data Processing Service started successfully");
        Ok(())
    }

    /// Stop all service components gracefully
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping Data Processing Service");

        // Stop components in reverse order
        if let Err(e) = self.batch_processor.stop().await {
            tracing::error!("Error stopping batch processor: {}", e);
        }

        if let Err(e) = self.stream_processor.stop().await {
            tracing::error!("Error stopping stream processor: {}", e);
        }

        if let Err(e) = self.kafka_manager.stop().await {
            tracing::error!("Error stopping Kafka manager: {}", e);
        }

        if let Err(e) = self.health_checker.stop().await {
            tracing::error!("Error stopping health checker: {}", e);
        }

        tracing::info!("Data Processing Service stopped");
        Ok(())
    }

    /// Get service metrics
    pub fn metrics(&self) -> Arc<MetricsCollector> {
        self.metrics.clone()
    }

    /// Get service configuration
    pub fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    /// Get overall service health status
    pub async fn health(&self) -> types::ServiceHealth {
        self.health_checker.get_health().await
    }

    /// Process a single data record (for testing/debugging)
    pub async fn process_record(&self, record: DataRecord) -> Result<ProcessingResult> {
        self.stream_processor.process_record(record).await
    }

    /// Submit a batch processing job
    pub async fn submit_batch_job(&self, job: BatchJob) -> Result<String> {
        self.batch_processor.submit_job(job).await
    }

    /// Get batch job status
    pub async fn get_batch_job_status(&self, job_id: &str) -> Result<BatchJobStatus> {
        self.batch_processor.get_job_status(job_id).await
    }
}

/// Builder for creating DataProcessingService with custom configuration
pub struct DataProcessingServiceBuilder {
    config: Config,
}

impl DataProcessingServiceBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Set Kafka configuration
    pub fn with_kafka_config(mut self, kafka_config: config::KafkaConfig) -> Self {
        self.config.kafka = kafka_config;
        self
    }

    /// Set ClickHouse configuration
    pub fn with_clickhouse_config(mut self, clickhouse_config: config::ClickHouseConfig) -> Self {
        self.config.clickhouse = clickhouse_config;
        self
    }

    /// Set stream processing configuration
    pub fn with_stream_config(mut self, stream_config: config::StreamConfig) -> Self {
        self.config.stream = stream_config;
        self
    }

    /// Set batch processing configuration
    pub fn with_batch_config(mut self, batch_config: config::BatchConfig) -> Self {
        self.config.batch = batch_config;
        self
    }

    /// Build the service
    pub async fn build(self) -> Result<DataProcessingService> {
        DataProcessingService::new(self.config).await
    }
}

impl Default for DataProcessingServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_service_builder() {
        let service = DataProcessingServiceBuilder::new().build().await;

        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_service_lifecycle() {
        let config = Config::default();
        let service = DataProcessingService::new(config).await;

        match service {
            Ok(service) => {
                // Test start/stop cycle
                let start_result = service.start().await;
                assert!(start_result.is_ok());

                let stop_result = service.stop().await;
                assert!(stop_result.is_ok());
            }
            Err(_) => {
                // Service creation may fail in test environment without Kafka/ClickHouse
                // This is acceptable for unit tests
            }
        }
    }
}
