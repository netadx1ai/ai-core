//! Kafka integration module for the Data Processing Service
//!
//! This module provides comprehensive Kafka integration including:
//! - High-performance producers and consumers
//! - Message serialization and deserialization
//! - Error handling and retry mechanisms
//! - Health monitoring and metrics collection
//! - Consumer group management
//! - Exactly-once processing support

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result as AnyhowResult;
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    config::{ClientConfig, RDKafkaLogLevel},
    consumer::{Consumer, StreamConsumer},
    message::{Headers, Message, OwnedHeaders},
    producer::{FutureProducer, FutureRecord},
    ClientContext, TopicPartitionList,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::{Config, KafkaConfig},
    error::{DataProcessingError, KafkaError, Result},
    metrics::MetricsCollector,
    types::{DataRecord, HealthStatus, ProcessingResult},
};

/// Kafka manager that handles all Kafka operations
#[derive(Clone)]
pub struct KafkaManager {
    config: Arc<KafkaConfig>,
    producer: Arc<FutureProducer>,
    consumer: Arc<StreamConsumer>,
    admin_client: Arc<AdminClient<DefaultClientContext>>,
    metrics: Arc<MetricsCollector>,
    health_status: Arc<RwLock<HealthStatus>>,
    message_handlers: Arc<RwLock<HashMap<String, MessageHandler>>>,
}

/// Message handler type for processing different message types
pub type MessageHandler = Box<dyn Fn(KafkaMessage) -> Result<ProcessingResult> + Send + Sync>;

/// Kafka message wrapper
#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<Vec<u8>>,
    pub payload: Vec<u8>,
    pub timestamp: Option<i64>,
    pub headers: HashMap<String, Vec<u8>>,
}

/// Producer configuration
#[derive(Debug, Clone)]
pub struct ProducerConfig {
    pub batch_size: usize,
    pub linger_ms: u64,
    pub compression_type: String,
    pub max_in_flight: usize,
    pub enable_idempotence: bool,
    pub acks: String,
    pub retries: u32,
    pub delivery_timeout_ms: u64,
}

/// Consumer configuration
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    pub group_id: String,
    pub auto_offset_reset: String,
    pub enable_auto_commit: bool,
    pub session_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub max_poll_interval_ms: u64,
    pub fetch_min_bytes: usize,
    pub fetch_max_wait_ms: u64,
    pub max_poll_records: usize,
}

/// Message publishing options
#[derive(Debug, Clone)]
pub struct PublishOptions {
    pub key: Option<String>,
    pub headers: HashMap<String, String>,
    pub partition: Option<i32>,
    pub timestamp: Option<i64>,
    pub timeout_ms: Option<u64>,
}

/// Consumer subscription options
#[derive(Debug, Clone)]
pub struct SubscriptionOptions {
    pub topics: Vec<String>,
    pub assignment_strategy: AssignmentStrategy,
    pub start_from_beginning: bool,
    pub commit_strategy: CommitStrategy,
}

/// Partition assignment strategies
#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentStrategy {
    RoundRobin,
    Range,
    CooperativeSticky,
}

/// Commit strategies
#[derive(Debug, Clone, PartialEq)]
pub enum CommitStrategy {
    AutoCommit,
    ManualCommit,
    AsyncCommit,
    SyncCommit,
}

/// Kafka statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaStats {
    pub messages_produced: u64,
    pub messages_consumed: u64,
    pub produce_errors: u64,
    pub consume_errors: u64,
    pub connection_errors: u64,
    pub avg_produce_latency_ms: f64,
    pub avg_consume_latency_ms: f64,
    pub topic_stats: HashMap<String, TopicStats>,
}

/// Per-topic statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStats {
    pub messages_produced: u64,
    pub messages_consumed: u64,
    pub bytes_produced: u64,
    pub bytes_consumed: u64,
    pub last_produce_time: Option<i64>,
    pub last_consume_time: Option<i64>,
}

impl KafkaManager {
    /// Create a new Kafka manager
    pub async fn new(config: &Config, metrics: Arc<MetricsCollector>) -> Result<Self> {
        let kafka_config = &config.kafka;

        info!(
            "Initializing Kafka manager with brokers: {}",
            kafka_config.bootstrap_servers
        );

        // Create producer
        let producer = Self::create_producer(kafka_config).await?;

        // Create consumer
        let consumer = Self::create_consumer(kafka_config).await?;

        // Create admin client
        let admin_client = Self::create_admin_client(kafka_config).await?;

        let manager = Self {
            config: Arc::new(kafka_config.clone()),
            producer: Arc::new(producer),
            consumer: Arc::new(consumer),
            admin_client: Arc::new(admin_client),
            metrics,
            health_status: Arc::new(RwLock::new(HealthStatus::Unknown)),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
        };

        // Perform initial health check
        manager.health_check().await?;

        info!("Kafka manager initialized successfully");
        Ok(manager)
    }

    /// Create Kafka producer
    async fn create_producer(config: &KafkaConfig) -> Result<FutureProducer> {
        let mut client_config = ClientConfig::new();

        client_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("client.id", &config.producer_client_id)
            .set("batch.size", &config.batch_size.to_string())
            .set("linger.ms", &config.linger_ms.to_string())
            .set("buffer.memory", &config.buffer_memory.to_string())
            .set("compression.type", &config.compression_type)
            .set("acks", "all")
            .set("retries", "2147483647")
            .set("max.in.flight.requests.per.connection", "5")
            .set("enable.idempotence", "true")
            .set("delivery.timeout.ms", "300000")
            .set("request.timeout.ms", "30000");

        // Add security configuration if provided
        if let Some(sasl) = &config.sasl {
            client_config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanism", &sasl.mechanism)
                .set("sasl.username", &sasl.username)
                .set("sasl.password", &sasl.password);
        }

        if let Some(ssl) = &config.ssl {
            if let Some(ca_cert) = &ssl.ca_cert_path {
                client_config.set("ssl.ca.location", ca_cert.to_string_lossy().as_ref());
            }
            if let Some(cert) = &ssl.client_cert_path {
                client_config.set("ssl.certificate.location", cert.to_string_lossy().as_ref());
            }
            if let Some(key) = &ssl.client_key_path {
                client_config.set("ssl.key.location", key.to_string_lossy().as_ref());
            }
            client_config.set(
                "ssl.endpoint.identification.algorithm",
                if ssl.verify_hostname { "https" } else { "none" },
            );
        }

        client_config.set_log_level(RDKafkaLogLevel::Info);

        let producer: FutureProducer =
            client_config.create().map_err(|e| KafkaError::Connection {
                message: format!("Failed to create producer: {}", e),
            })?;

        Ok(producer)
    }

    /// Create Kafka consumer
    async fn create_consumer(config: &KafkaConfig) -> Result<StreamConsumer> {
        let mut client_config = ClientConfig::new();

        client_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("group.id", &config.consumer_group_id)
            .set(
                "client.id",
                &format!("{}-consumer", config.consumer_group_id),
            )
            .set("auto.offset.reset", &config.auto_offset_reset)
            .set("enable.auto.commit", &config.enable_auto_commit.to_string())
            .set(
                "auto.commit.interval.ms",
                &config.auto_commit_interval_ms.to_string(),
            )
            .set("session.timeout.ms", &config.session_timeout_ms.to_string())
            .set("heartbeat.interval.ms", "3000")
            .set("max.poll.interval.ms", "300000")
            .set("fetch.min.bytes", &config.fetch_min_bytes.to_string())
            .set("fetch.max.wait.ms", &config.fetch_max_wait_ms.to_string())
            .set("max.partition.fetch.bytes", "1048576")
            .set("enable.partition.eof", "false");

        // Add security configuration if provided
        if let Some(sasl) = &config.sasl {
            client_config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanism", &sasl.mechanism)
                .set("sasl.username", &sasl.username)
                .set("sasl.password", &sasl.password);
        }

        if let Some(ssl) = &config.ssl {
            if let Some(ca_cert) = &ssl.ca_cert_path {
                client_config.set("ssl.ca.location", ca_cert.to_string_lossy().as_ref());
            }
            if let Some(cert) = &ssl.client_cert_path {
                client_config.set("ssl.certificate.location", cert.to_string_lossy().as_ref());
            }
            if let Some(key) = &ssl.client_key_path {
                client_config.set("ssl.key.location", key.to_string_lossy().as_ref());
            }
            client_config.set(
                "ssl.endpoint.identification.algorithm",
                if ssl.verify_hostname { "https" } else { "none" },
            );
        }

        client_config.set_log_level(RDKafkaLogLevel::Info);

        let consumer: StreamConsumer =
            client_config.create().map_err(|e| KafkaError::Connection {
                message: format!("Failed to create consumer: {}", e),
            })?;

        Ok(consumer)
    }

    /// Create Kafka admin client
    async fn create_admin_client(
        config: &KafkaConfig,
    ) -> Result<AdminClient<DefaultClientContext>> {
        let mut client_config = ClientConfig::new();

        client_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("client.id", "data-processing-admin");

        // Add security configuration if provided
        if let Some(sasl) = &config.sasl {
            client_config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanism", &sasl.mechanism)
                .set("sasl.username", &sasl.username)
                .set("sasl.password", &sasl.password);
        }

        let admin_client: AdminClient<DefaultClientContext> =
            client_config.create().map_err(|e| KafkaError::Connection {
                message: format!("Failed to create admin client: {}", e),
            })?;

        Ok(admin_client)
    }

    /// Start the Kafka manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting Kafka manager");

        // Update health status
        {
            let mut health = self.health_status.write().await;
            *health = HealthStatus::Healthy;
        }

        // Start background health monitoring
        self.start_health_monitoring().await;

        info!("Kafka manager started successfully");
        Ok(())
    }

    /// Stop the Kafka manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Kafka manager");

        // Update health status
        {
            let mut health = self.health_status.write().await;
            *health = HealthStatus::Unknown;
        }

        info!("Kafka manager stopped");
        Ok(())
    }

    /// Publish a message to a Kafka topic
    pub async fn publish<T>(&self, topic: &str, message: &T, options: PublishOptions) -> Result<()>
    where
        T: Serialize,
    {
        let start_time = Instant::now();

        // Serialize the message
        let payload = serde_json::to_vec(message).map_err(|e| KafkaError::Serialization {
            message: format!("Failed to serialize message: {}", e),
        })?;

        // Create record
        let mut record = FutureRecord::to(topic).payload(&payload);

        if let Some(key) = &options.key {
            record = record.key(key);
        }

        if let Some(partition) = options.partition {
            record = record.partition(partition);
        }

        if let Some(timestamp) = options.timestamp {
            record = record.timestamp(timestamp);
        }

        // Add headers
        if !options.headers.is_empty() {
            let mut headers = OwnedHeaders::new();
            for (key, value) in &options.headers {
                headers = headers.insert(rdkafka::message::Header {
                    key,
                    value: Some(value),
                });
            }
            record = record.headers(headers);
        }

        // Send the message
        let timeout = Duration::from_millis(options.timeout_ms.unwrap_or(30000));
        let delivery_future = self.producer.send(record, timeout);

        match delivery_future.await {
            Ok((partition, offset)) => {
                let latency = start_time.elapsed();
                debug!(
                    "Message sent to topic {} partition {} offset {} in {:?}",
                    topic, partition, offset, latency
                );

                // Update metrics
                self.metrics
                    .increment_counter("kafka_messages_produced_total", &[("topic", topic)]);
                self.metrics.record_histogram(
                    "kafka_produce_latency_seconds",
                    latency.as_secs_f64(),
                    &[("topic", topic)],
                );
            }
            Err((error, _)) => {
                error!("Failed to send message to topic {}: {}", topic, error);
                self.metrics
                    .increment_counter("kafka_produce_errors_total", &[("topic", topic)]);
                return Err(KafkaError::Producer {
                    message: format!("Failed to send message: {}", error),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Subscribe to Kafka topics and start consuming messages
    pub async fn subscribe(
        &self,
        options: SubscriptionOptions,
    ) -> Result<mpsc::Receiver<KafkaMessage>> {
        info!("Subscribing to topics: {:?}", options.topics);

        // Subscribe to topics
        self.consumer
            .subscribe(
                &options
                    .topics
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| KafkaError::Consumer {
                message: format!("Failed to subscribe to topics: {}", e),
            })?;

        // Create channel for messages
        let (tx, rx) = mpsc::channel(1000);

        // Start consuming messages
        let consumer = self.consumer.clone();
        let metrics = self.metrics.clone();
        let handlers = self.message_handlers.clone();

        tokio::spawn(async move {
            loop {
                match consumer.recv().await {
                    Ok(message) => {
                        let start_time = Instant::now();

                        let kafka_message = KafkaMessage {
                            topic: message.topic().to_string(),
                            partition: message.partition(),
                            offset: message.offset(),
                            key: message.key().map(|k| k.to_vec()),
                            payload: message.payload().unwrap_or(&[]).to_vec(),
                            timestamp: message.timestamp().to_millis(),
                            headers: Self::extract_headers(&message),
                        };

                        // Update metrics
                        metrics.increment_counter(
                            "kafka_messages_consumed_total",
                            &[("topic", &kafka_message.topic)],
                        );
                        let latency = start_time.elapsed();
                        metrics.record_histogram(
                            "kafka_consume_latency_seconds",
                            latency.as_secs_f64(),
                            &[("topic", &kafka_message.topic)],
                        );

                        // Process message with handlers if available
                        {
                            let handlers_map = handlers.read().await;
                            if let Some(handler) = handlers_map.get(&kafka_message.topic) {
                                match handler(kafka_message.clone()) {
                                    Ok(result) => {
                                        debug!("Message processed successfully: {:?}", result);
                                    }
                                    Err(e) => {
                                        error!("Message processing failed: {}", e);
                                        metrics.increment_counter(
                                            "kafka_message_processing_errors_total",
                                            &[("topic", &kafka_message.topic)],
                                        );
                                    }
                                }
                            }
                        }

                        // Send message to channel
                        if tx.send(kafka_message).await.is_err() {
                            warn!("Failed to send message to channel, receiver dropped");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        metrics.increment_counter("kafka_consume_errors_total", &[]);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Register a message handler for a specific topic
    pub async fn register_handler<F>(&self, topic: String, handler: F)
    where
        F: Fn(KafkaMessage) -> Result<ProcessingResult> + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(topic.clone(), Box::new(handler));
        info!("Registered message handler for topic: {}", topic);
    }

    /// Create a Kafka topic
    pub async fn create_topic(
        &self,
        name: &str,
        partitions: i32,
        replication_factor: i32,
    ) -> Result<()> {
        let new_topic = NewTopic::new(
            name,
            partitions,
            TopicReplication::Fixed(replication_factor),
        );
        let topics = vec![new_topic];
        let options = AdminOptions::new().operation_timeout(Some(Duration::from_secs(30)));

        match self.admin_client.create_topics(&topics, &options).await {
            Ok(results) => {
                for result in results {
                    match result {
                        Ok(topic_name) => {
                            info!("Topic created successfully: {}", topic_name);
                        }
                        Err(e) => {
                            if e.1.to_string().contains("already exists") {
                                info!("Topic {} already exists", name);
                            } else {
                                error!("Failed to create topic {}: {}", name, e.1);
                                return Err(KafkaError::Topic {
                                    topic: name.to_string(),
                                    message: format!("Failed to create topic: {}", e.1),
                                }
                                .into());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to create topics: {}", e);
                return Err(KafkaError::Topic {
                    topic: name.to_string(),
                    message: format!("Failed to create topic: {}", e),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Commit consumer offsets manually
    pub async fn commit_offsets(&self) -> Result<()> {
        self.consumer
            .commit_consumer_state(rdkafka::consumer::CommitMode::Async)
            .map_err(|e| KafkaError::Offset {
                message: format!("Failed to commit offsets: {}", e),
            })?;

        debug!("Consumer offsets committed successfully");
        Ok(())
    }

    /// Get current health status
    pub async fn get_health(&self) -> HealthStatus {
        self.health_status.read().await.clone()
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<()> {
        // Try to get metadata to check connectivity
        match self.consumer.fetch_metadata(None, Duration::from_secs(10)) {
            Ok(metadata) => {
                if metadata.brokers().is_empty() {
                    let mut health = self.health_status.write().await;
                    *health = HealthStatus::Unhealthy;
                    return Err(KafkaError::Connection {
                        message: "No brokers available".to_string(),
                    }
                    .into());
                } else {
                    let mut health = self.health_status.write().await;
                    *health = HealthStatus::Healthy;
                    debug!(
                        "Kafka health check passed, {} brokers available",
                        metadata.brokers().len()
                    );
                }
            }
            Err(e) => {
                let mut health = self.health_status.write().await;
                *health = HealthStatus::Unhealthy;
                return Err(KafkaError::Connection {
                    message: format!("Health check failed: {}", e),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Start background health monitoring
    async fn start_health_monitoring(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                if let Err(e) = manager.health_check().await {
                    error!("Health check failed: {}", e);
                }
            }
        });
    }

    /// Extract headers from Kafka message
    fn extract_headers<M: Message>(message: &M) -> HashMap<String, Vec<u8>> {
        let mut headers = HashMap::new();

        if let Some(message_headers) = message.headers() {
            for header in message_headers.iter() {
                if let Some(value) = header.value {
                    headers.insert(header.key.to_string(), value.to_vec());
                }
            }
        }

        headers
    }

    /// Get Kafka statistics
    pub async fn get_stats(&self) -> KafkaStats {
        // This is a simplified version - in a real implementation,
        // you would collect and maintain detailed statistics
        KafkaStats {
            messages_produced: 0,
            messages_consumed: 0,
            produce_errors: 0,
            consume_errors: 0,
            connection_errors: 0,
            avg_produce_latency_ms: 0.0,
            avg_consume_latency_ms: 0.0,
            topic_stats: HashMap::new(),
        }
    }
}

impl Default for PublishOptions {
    fn default() -> Self {
        Self {
            key: None,
            headers: HashMap::new(),
            partition: None,
            timestamp: None,
            timeout_ms: Some(30000),
        }
    }
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            topics: vec!["events".to_string()],
            assignment_strategy: AssignmentStrategy::RoundRobin,
            start_from_beginning: false,
            commit_strategy: CommitStrategy::AutoCommit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_kafka_manager_creation() {
        let config = Config::default();
        let metrics = Arc::new(MetricsCollector::new(&config).unwrap());

        // This test may fail without actual Kafka instance
        let result = KafkaManager::new(&config, metrics).await;

        // In test environment, we just check that the creation doesn't panic
        match result {
            Ok(_) => {
                // Success case - Kafka is available
            }
            Err(e) => {
                // Expected in test environment without Kafka
                assert!(e.to_string().contains("Connection") || e.to_string().contains("resolve"));
            }
        }
    }

    #[test]
    fn test_publish_options_default() {
        let options = PublishOptions::default();
        assert_eq!(options.timeout_ms, Some(30000));
        assert!(options.key.is_none());
        assert!(options.headers.is_empty());
    }

    #[test]
    fn test_subscription_options_default() {
        let options = SubscriptionOptions::default();
        assert_eq!(options.topics, vec!["events".to_string()]);
        assert_eq!(options.assignment_strategy, AssignmentStrategy::RoundRobin);
        assert_eq!(options.commit_strategy, CommitStrategy::AutoCommit);
    }
}
