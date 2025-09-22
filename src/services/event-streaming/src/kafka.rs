//! # Kafka Integration Module
//!
//! This module provides Kafka integration for the event streaming service.
//! It handles Kafka producers, consumers, topic management, and health monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    error::{KafkaError, KafkaResult},
    message::{BorrowedMessage, Message},
    producer::{FutureProducer, FutureRecord},
    topic_partition_list::TopicPartitionList,
    util::get_rdkafka_version,
};
use serde_json;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::{Config, KafkaConfig},
    error::{EventStreamingError, Result},
    events::Event,
    metrics::MetricsCollector,
    types::{ComponentHealth, HealthStatus},
};

/// Kafka manager for handling producers and consumers
#[derive(Clone)]
pub struct KafkaManager {
    config: Arc<KafkaConfig>,
    producer: Arc<FutureProducer>,
    consumers: Arc<RwLock<HashMap<String, Arc<StreamConsumer>>>>,
    metrics_collector: Arc<MetricsCollector>,
    shutdown_tx: Arc<RwLock<Option<broadcast::Sender<()>>>>,
    health_status: Arc<RwLock<HealthStatus>>,
}

impl KafkaManager {
    /// Create a new Kafka manager
    pub async fn new(
        config: &Config,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Result<Self> {
        info!("Initializing Kafka Manager");
        debug!("Using rdkafka version: {}", get_rdkafka_version());

        let kafka_config = &config.kafka;

        // Create producer configuration
        let mut producer_config = ClientConfig::new();
        producer_config
            .set("bootstrap.servers", kafka_config.bootstrap_servers.join(","))
            .set("client.id", &kafka_config.producer_client_id)
            .set("acks", kafka_acks_to_string(&kafka_config.producer.acks))
            .set("retries", kafka_config.producer.retries.to_string())
            .set("batch.size", kafka_config.producer.batch_size.to_string())
            .set("linger.ms", kafka_config.producer.linger_ms.to_string())
            .set("buffer.memory", kafka_config.producer.buffer_memory.to_string())
            .set("compression.type", compression_type_to_string(&kafka_config.producer.compression_type))
            .set("request.timeout.ms", kafka_config.producer.request_timeout_ms.to_string())
            .set("delivery.timeout.ms", kafka_config.producer.delivery_timeout_ms.to_string())
            .set("enable.idempotence", "true")
            .set("max.in.flight.requests.per.connection", "5");

        // Add security configuration if present
        if let Some(security) = &kafka_config.security {
            add_security_config(&mut producer_config, security)?;
        }

        // Create producer
        let producer: FutureProducer = producer_config
            .create()
            .map_err(|e| EventStreamingError::kafka(format!("Failed to create producer: {}", e)))?;

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config: Arc::new(kafka_config.clone()),
            producer: Arc::new(producer),
            consumers: Arc::new(RwLock::new(HashMap::new())),
            metrics_collector,
            shutdown_tx: Arc::new(RwLock::new(Some(shutdown_tx))),
            health_status: Arc::new(RwLock::new(HealthStatus::Healthy)),
        })
    }

    /// Start the Kafka manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting Kafka Manager");

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Healthy;
        }

        // Create topics if they don't exist
        self.create_topics().await?;

        // Start consumers for configured topics
        for (topic_name, stream_config) in &self.config.topics {
            self.create_consumer(topic_name, stream_config).await?;
        }

        // Start health monitoring
        self.start_health_monitoring().await?;

        info!("Kafka Manager started successfully");
        Ok(())
    }

    /// Stop the Kafka manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Kafka Manager");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(());
        }

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Unhealthy;
        }

        // Stop all consumers
        {
            let mut consumers = self.consumers.write().await;
            consumers.clear();
        }

        info!("Kafka Manager stopped");
        Ok(())
    }

    /// Publish an event to Kafka
    pub async fn publish_event(
        &self,
        topic: &str,
        event: &Event,
        key: Option<&str>,
    ) -> Result<()> {
        let start_time = Instant::now();

        // Serialize event
        let payload = serde_json::to_vec(event)
            .map_err(|e| EventStreamingError::kafka(format!("Failed to serialize event: {}", e)))?;

        // Create record
        let mut record = FutureRecord::to(topic).payload(&payload);

        if let Some(key) = key {
            record = record.key(key);
        } else {
            record = record.key(&event.id.to_string());
        }

        // Add headers
        let headers = create_event_headers(event)?;
        for (key, value) in headers {
            record = record.headers(rdkafka::message::OwnedHeaders::new().insert(rdkafka::message::Header {
                key: &key,
                value: Some(&value),
            }));
        }

        // Send message
        match self.producer.send(record, Duration::from_secs(30)).await {
            Ok((partition, offset)) => {
                let duration = start_time.elapsed();
                debug!(
                    "Event {} published to topic {} partition {} offset {} in {:?}",
                    event.id, topic, partition, offset, duration
                );

                // Record metrics
                self.metrics_collector
                    .record_kafka_publish_success(topic, duration)
                    .await?;

                Ok(())
            }
            Err((kafka_error, _)) => {
                let duration = start_time.elapsed();
                error!(
                    "Failed to publish event {} to topic {}: {}",
                    event.id, topic, kafka_error
                );

                // Record metrics
                self.metrics_collector
                    .record_kafka_publish_error(topic, duration, &kafka_error.to_string())
                    .await?;

                Err(EventStreamingError::kafka(format!(
                    "Failed to publish to topic {}: {}",
                    topic, kafka_error
                )))
            }
        }
    }

    /// Create a consumer for a topic
    pub async fn create_consumer(
        &self,
        topic: &str,
        _stream_config: &crate::types::StreamConfig,
    ) -> Result<()> {
        info!("Creating consumer for topic: {}", topic);

        // Create consumer configuration
        let mut consumer_config = ClientConfig::new();
        consumer_config
            .set("bootstrap.servers", self.config.bootstrap_servers.join(","))
            .set("group.id", &self.config.consumer_group_id)
            .set("client.id", format!("{}-consumer-{}", self.config.consumer_group_id, topic))
            .set("session.timeout.ms", self.config.consumer.session_timeout_ms.to_string())
            .set("heartbeat.interval.ms", self.config.consumer.heartbeat_interval_ms.to_string())
            .set("auto.offset.reset", auto_offset_reset_to_string(&self.config.consumer.auto_offset_reset))
            .set("enable.auto.commit", self.config.consumer.enable_auto_commit.to_string())
            .set("auto.commit.interval.ms", self.config.consumer.auto_commit_interval_ms.to_string())
            .set("max.poll.records", self.config.consumer.max_poll_records.to_string())
            .set("fetch.min.bytes", self.config.consumer.fetch_min_bytes.to_string())
            .set("fetch.max.wait.ms", self.config.consumer.fetch_max_wait_ms.to_string());

        // Add security configuration if present
        if let Some(security) = &self.config.security {
            add_security_config(&mut consumer_config, security)?;
        }

        // Create consumer
        let consumer: StreamConsumer = consumer_config
            .create()
            .map_err(|e| EventStreamingError::kafka(format!("Failed to create consumer: {}", e)))?;

        // Subscribe to topic
        let topic_partition_list = TopicPartitionList::with_topics(&[topic]);
        consumer
            .subscribe(&topic_partition_list)
            .map_err(|e| EventStreamingError::kafka(format!("Failed to subscribe to topic {}: {}", topic, e)))?;

        // Store consumer
        {
            let mut consumers = self.consumers.write().await;
            consumers.insert(topic.to_string(), Arc::new(consumer));
        }

        info!("Consumer created for topic: {}", topic);
        Ok(())
    }

    /// Get a consumer for a topic
    pub async fn get_consumer(&self, topic: &str) -> Option<Arc<StreamConsumer>> {
        let consumers = self.consumers.read().await;
        consumers.get(topic).cloned()
    }

    /// List all available topics
    pub async fn list_topics(&self) -> Result<Vec<String>> {
        let metadata = self.producer
            .client()
            .fetch_metadata(None, Duration::from_secs(10))
            .map_err(|e| EventStreamingError::kafka(format!("Failed to fetch metadata: {}", e)))?;

        let topics = metadata
            .topics()
            .iter()
            .map(|topic| topic.name().to_string())
            .collect();

        Ok(topics)
    }

    /// Get topic information
    pub async fn get_topic_info(&self, topic: &str) -> Result<serde_json::Value> {
        let metadata = self.producer
            .client()
            .fetch_metadata(Some(topic), Duration::from_secs(10))
            .map_err(|e| EventStreamingError::kafka(format!("Failed to fetch metadata for topic {}: {}", topic, e)))?;

        let topic_metadata = metadata
            .topics()
            .iter()
            .find(|t| t.name() == topic)
            .ok_or_else(|| EventStreamingError::kafka(format!("Topic {} not found", topic)))?;

        let partitions: Vec<serde_json::Value> = topic_metadata
            .partitions()
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id(),
                    "leader": p.leader(),
                    "replicas": p.replicas(),
                    "isr": p.isr(),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "name": topic,
            "partitions": partitions,
            "partition_count": partitions.len(),
            "error": topic_metadata.error().map(|e| e.to_string()),
        }))
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<ComponentHealth> {
        let start_time = Instant::now();

        match self.producer.client().fetch_metadata(None, Duration::from_secs(5)) {
            Ok(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(ComponentHealth {
                    component: "kafka".to_string(),
                    status: HealthStatus::Healthy,
                    last_check: chrono::Utc::now(),
                    response_time_ms: response_time,
                    details: [
                        ("brokers".to_string(), self.config.bootstrap_servers.join(",")),
                        ("producer_client_id".to_string(), self.config.producer_client_id.clone()),
                        ("consumer_group_id".to_string(), self.config.consumer_group_id.clone()),
                    ].into(),
                })
            }
            Err(e) => {
                error!("Kafka health check failed: {}", e);
                Ok(ComponentHealth {
                    component: "kafka".to_string(),
                    status: HealthStatus::Unhealthy,
                    last_check: chrono::Utc::now(),
                    response_time_ms: 0,
                    details: [("error".to_string(), e.to_string())].into(),
                })
            }
        }
    }

    /// Start health monitoring
    async fn start_health_monitoring(&self) -> Result<()> {
        let manager = self.clone();
        let interval = Duration::from_secs(30);

        tokio::spawn(async move {
            let mut shutdown_rx = manager.shutdown_tx.read().await.as_ref().unwrap().subscribe();
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        match manager.perform_health_check().await {
                            Ok(health) => {
                                let mut status = manager.health_status.write().await;
                                *status = health.status;
                            }
                            Err(e) => {
                                error!("Kafka health monitoring failed: {}", e);
                                let mut status = manager.health_status.write().await;
                                *status = HealthStatus::Unhealthy;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Kafka health monitor received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Perform comprehensive health check
    async fn perform_health_check(&self) -> Result<ComponentHealth> {
        self.health_check().await
    }

    /// Create topics if they don't exist
    async fn create_topics(&self) -> Result<()> {
        // Note: Topic creation would typically be handled by Kafka administration tools
        // or during deployment. This is a placeholder for automatic topic creation.
        debug!("Topic creation is handled externally");
        Ok(())
    }
}

/// Convert Kafka acks enum to string
fn kafka_acks_to_string(acks: &crate::config::KafkaAcks) -> &'static str {
    match acks {
        crate::config::KafkaAcks::None => "0",
        crate::config::KafkaAcks::Leader => "1",
        crate::config::KafkaAcks::All => "all",
    }
}

/// Convert compression type to string
fn compression_type_to_string(compression: &crate::types::CompressionType) -> &'static str {
    match compression {
        crate::types::CompressionType::None => "none",
        crate::types::CompressionType::Gzip => "gzip",
        crate::types::CompressionType::Lz4 => "lz4",
        crate::types::CompressionType::Zstd => "zstd",
        crate::types::CompressionType::Snappy => "snappy",
    }
}

/// Convert auto offset reset enum to string
fn auto_offset_reset_to_string(reset: &crate::config::AutoOffsetReset) -> &'static str {
    match reset {
        crate::config::AutoOffsetReset::Earliest => "earliest",
        crate::config::AutoOffsetReset::Latest => "latest",
        crate::config::AutoOffsetReset::None => "none",
    }
}

/// Add security configuration to client config
fn add_security_config(
    config: &mut ClientConfig,
    security: &crate::config::KafkaSecurityConfig,
) -> Result<()> {
    match &security.protocol {
        crate::config::KafkaSecurityProtocol::Plaintext => {
            config.set("security.protocol", "PLAINTEXT");
        }
        crate::config::KafkaSecurityProtocol::Ssl => {
            config.set("security.protocol", "SSL");
            if let Some(ssl) = &security.ssl {
                add_ssl_config(config, ssl)?;
            }
        }
        crate::config::KafkaSecurityProtocol::SaslPlaintext => {
            config.set("security.protocol", "SASL_PLAINTEXT");
            add_sasl_config(config, security)?;
        }
        crate::config::KafkaSecurityProtocol::SaslSsl => {
            config.set("security.protocol", "SASL_SSL");
            add_sasl_config(config, security)?;
            if let Some(ssl) = &security.ssl {
                add_ssl_config(config, ssl)?;
            }
        }
    }

    Ok(())
}

/// Add SASL configuration
fn add_sasl_config(
    config: &mut ClientConfig,
    security: &crate::config::KafkaSecurityConfig,
) -> Result<()> {
    if let Some(mechanism) = &security.sasl_mechanism {
        config.set("sasl.mechanism", mechanism);
    }

    if let Some(username) = &security.sasl_username {
        config.set("sasl.username", username);
    }

    if let Some(password) = &security.sasl_password {
        config.set("sasl.password", password);
    }

    Ok(())
}

/// Add SSL configuration
fn add_ssl_config(config: &mut ClientConfig, ssl: &crate::config::KafkaSslConfig) -> Result<()> {
    if let Some(ca_cert_path) = &ssl.ca_cert_path {
        config.set("ssl.ca.location", ca_cert_path);
    }

    if let Some(cert_path) = &ssl.cert_path {
        config.set("ssl.certificate.location", cert_path);
    }

    if let Some(key_path) = &ssl.key_path {
        config.set("ssl.key.location", key_path);
    }

    if let Some(key_password) = &ssl.key_password {
        config.set("ssl.key.password", key_password);
    }

    if !ssl.verify_certificates {
        config.set("ssl.endpoint.identification.algorithm", "none");
    }

    Ok(())
}

/// Create headers for an event
fn create_event_headers(event: &Event) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();

    headers.insert("event_id".to_string(), event.id.to_string());
    headers.insert("event_type".to_string(), event.event_type.clone());
    headers.insert("category".to_string(), serde_json::to_string(&event.category)?);
    headers.insert("priority".to_string(), serde_json::to_string(&event.priority)?);
    headers.insert("source_service".to_string(), event.source.service.clone());
    headers.insert("correlation_id".to_string(), event.correlation.correlation_id.to_string());
    headers.insert("created_at".to_string(), event.created_at.to_rfc3339());

    if let Some(tenant_id) = &event.metadata.tenant_id {
        headers.insert("tenant_id".to_string(), tenant_id.clone());
    }

    if let Some(environment) = &event.metadata.environment {
        headers.insert("environment".to_string(), environment.clone());
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::events::{Event, EventPayload};
    use crate::types::{EventCategory, EventSource};

    #[tokio::test]
    async fn test_kafka_manager_creation() {
        let config = Config::default();
        let metrics = Arc::new(MetricsCollector::new(&config).await.unwrap());
        let result = KafkaManager::new(&config, metrics).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_kafka_acks_conversion() {
        assert_eq!(kafka_acks_to_string(&crate::config::KafkaAcks::None), "0");
        assert_eq!(kafka_acks_to_string(&crate::config::KafkaAcks::Leader), "1");
        assert_eq!(kafka_acks_to_string(&crate::config::KafkaAcks::All), "all");
    }

    #[test]
    fn test_compression_type_conversion() {
        assert_eq!(compression_type_to_string(&crate::types::CompressionType::None), "none");
        assert_eq!(compression_type_to_string(&crate::types::CompressionType::Lz4), "lz4");
        assert_eq!(compression_type_to_string(&crate::types::CompressionType::Gzip), "gzip");
    }

    #[test]
    fn test_auto_offset_reset_conversion() {
        assert_eq!(auto_offset_reset_to_string(&crate::config::AutoOffsetReset::Earliest), "earliest");
        assert_eq!(auto_offset_reset_to_string(&crate::config::AutoOffsetReset::Latest), "latest");
        assert_eq!(auto_offset_reset_to_string(&crate::config::AutoOffsetReset::None), "none");
    }

    #[tokio::test]
    async fn test_event_headers_creation() {
        let source = EventSource {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: None,
            hostname: None,
            metadata: std::collections::HashMap::new(),
        };

        let payload = EventPayload::Custom(serde_json::json!({"test": "data"}));
        let event = Event::new("test.event", EventCategory::System, source, payload);

        let headers = create_event_headers(&event).unwrap();

        assert!(headers.contains_key("event_id"));
        assert!(headers.contains_key("event_type"));
        assert!(headers.contains_key("category"));
        assert_eq!(headers.get("event_type").unwrap(), "test.event");
    }
}
