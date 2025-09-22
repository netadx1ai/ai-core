//! # Redis Streams Integration Module
//!
//! This module provides Redis Streams integration for the event streaming service.
//! It handles Redis stream producers, consumers, consumer groups, and health monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use redis::{
    streams::{StreamId, StreamReadOptions, StreamReadReply},
    AsyncCommands, Client, Connection, RedisResult,
};
use serde_json;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::{Config, RedisConfig},
    error::{EventStreamingError, Result},
    events::Event,
    metrics::MetricsCollector,
    types::{ComponentHealth, HealthStatus},
};

/// Redis Stream manager for handling producers and consumers
#[derive(Clone)]
pub struct RedisStreamManager {
    config: Arc<RedisConfig>,
    client: Arc<Client>,
    connection_pool: Arc<RwLock<Option<redis::aio::ConnectionManager>>>,
    consumer_groups: Arc<RwLock<HashMap<String, ConsumerGroupInfo>>>,
    metrics_collector: Arc<MetricsCollector>,
    shutdown_tx: Arc<RwLock<Option<broadcast::Sender<()>>>>,
    health_status: Arc<RwLock<HealthStatus>>,
}

/// Consumer group information
#[derive(Debug, Clone)]
pub struct ConsumerGroupInfo {
    pub group_name: String,
    pub consumer_name: String,
    pub stream_name: String,
    pub last_delivered_id: String,
    pub pending_count: u64,
}

impl RedisStreamManager {
    /// Create a new Redis Stream manager
    pub async fn new(
        config: &Config,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Result<Self> {
        info!("Initializing Redis Stream Manager");

        let redis_config = &config.redis;

        // Create Redis client
        let client = Client::open(redis_config.url.as_str())
            .map_err(|e| EventStreamingError::redis(format!("Failed to create Redis client: {}", e)))?;

        // Test connection
        let mut conn = client
            .get_async_connection()
            .await
            .map_err(|e| EventStreamingError::redis(format!("Failed to connect to Redis: {}", e)))?;

        // Ping to verify connection
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| EventStreamingError::redis(format!("Redis ping failed: {}", e)))?;

        let connection_manager = redis::aio::ConnectionManager::new(client.clone())
            .await
            .map_err(|e| EventStreamingError::redis(format!("Failed to create connection manager: {}", e)))?;

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config: Arc::new(redis_config.clone()),
            client: Arc::new(client),
            connection_pool: Arc::new(RwLock::new(Some(connection_manager))),
            consumer_groups: Arc::new(RwLock::new(HashMap::new())),
            metrics_collector,
            shutdown_tx: Arc::new(RwLock::new(Some(shutdown_tx))),
            health_status: Arc::new(RwLock::new(HealthStatus::Healthy)),
        })
    }

    /// Start the Redis Stream manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting Redis Stream Manager");

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Healthy;
        }

        // Create streams and consumer groups
        self.create_streams_and_groups().await?;

        // Start health monitoring
        self.start_health_monitoring().await?;

        info!("Redis Stream Manager started successfully");
        Ok(())
    }

    /// Stop the Redis Stream manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Redis Stream Manager");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(());
        }

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Unhealthy;
        }

        // Clear connection pool
        {
            let mut pool = self.connection_pool.write().await;
            *pool = None;
        }

        info!("Redis Stream Manager stopped");
        Ok(())
    }

    /// Publish an event to a Redis stream
    pub async fn publish_event(
        &self,
        stream: &str,
        event: &Event,
    ) -> Result<String> {
        let start_time = Instant::now();

        // Serialize event
        let payload = serde_json::to_string(event)
            .map_err(|e| EventStreamingError::redis(format!("Failed to serialize event: {}", e)))?;

        // Create stream entry fields
        let mut fields = vec![
            ("event_id", event.id.to_string()),
            ("event_type", event.event_type.clone()),
            ("category", serde_json::to_string(&event.category).unwrap_or_default()),
            ("priority", serde_json::to_string(&event.priority).unwrap_or_default()),
            ("payload", payload),
            ("created_at", event.created_at.to_rfc3339()),
            ("correlation_id", event.correlation.correlation_id.to_string()),
        ];

        // Add optional fields
        if let Some(tenant_id) = &event.metadata.tenant_id {
            fields.push(("tenant_id", tenant_id.clone()));
        }

        if let Some(environment) = &event.metadata.environment {
            fields.push(("environment", environment.clone()));
        }

        // Get connection
        let mut conn = self.get_connection().await?;

        // Add to stream
        let stream_id: String = conn
            .xadd(stream, "*", &fields)
            .await
            .map_err(|e| EventStreamingError::redis(format!("Failed to add to stream {}: {}", stream, e)))?;

        let duration = start_time.elapsed();

        debug!(
            "Event {} published to stream {} with ID {} in {:?}",
            event.id, stream, stream_id, duration
        );

        // Record metrics
        self.metrics_collector
            .record_redis_publish_success(stream, duration)
            .await?;

        Ok(stream_id)
    }

    /// Read events from a Redis stream
    pub async fn read_events(
        &self,
        stream: &str,
        group: &str,
        consumer: &str,
        count: usize,
        block_ms: Option<usize>,
    ) -> Result<Vec<Event>> {
        let start_time = Instant::now();

        // Get connection
        let mut conn = self.get_connection().await?;

        // Read from stream using consumer group
        let opts = StreamReadOptions::default()
            .count(count)
            .block(block_ms.unwrap_or(0));

        let results: StreamReadReply = conn
            .xreadgroup_options(&[(stream, ">")], group, consumer, &opts)
            .await
            .map_err(|e| EventStreamingError::redis(format!("Failed to read from stream {}: {}", stream, e)))?;

        let mut events = Vec::new();

        for stream_data in results.keys {
            for stream_id in stream_data.ids {
                match self.parse_stream_entry(&stream_id).await {
                    Ok(event) => events.push(event),
                    Err(e) => {
                        warn!("Failed to parse stream entry {}: {}", stream_id.id, e);
                        // Continue processing other entries
                    }
                }
            }
        }

        let duration = start_time.elapsed();

        debug!(
            "Read {} events from stream {} group {} consumer {} in {:?}",
            events.len(), stream, group, consumer, duration
        );

        // Record metrics
        self.metrics_collector
            .record_redis_read_success(stream, events.len(), duration)
            .await?;

        Ok(events)
    }

    /// Acknowledge processed messages
    pub async fn acknowledge_messages(
        &self,
        stream: &str,
        group: &str,
        message_ids: &[String],
    ) -> Result<u64> {
        // Get connection
        let mut conn = self.get_connection().await?;

        // Acknowledge messages
        let ack_count: u64 = conn
            .xack(stream, group, message_ids)
            .await
            .map_err(|e| EventStreamingError::redis(format!("Failed to acknowledge messages: {}", e)))?;

        debug!(
            "Acknowledged {} messages in stream {} group {}",
            ack_count, stream, group
        );

        Ok(ack_count)
    }

    /// Get pending messages for a consumer group
    pub async fn get_pending_messages(
        &self,
        stream: &str,
        group: &str,
        consumer: Option<&str>,
    ) -> Result<serde_json::Value> {
        // Get connection
        let mut conn = self.get_connection().await?;

        // Get pending messages
        let pending_info = if let Some(consumer) = consumer {
            conn.xpending_consumer_count(stream, group, "-", "+", 100, consumer)
                .await
                .map_err(|e| EventStreamingError::redis(format!("Failed to get pending messages: {}", e)))?
        } else {
            conn.xpending_count(stream, group, "-", "+", 100)
                .await
                .map_err(|e| EventStreamingError::redis(format!("Failed to get pending messages: {}", e)))?
        };

        // Convert to JSON for easier handling
        Ok(serde_json::json!({
            "stream": stream,
            "group": group,
            "consumer": consumer,
            "pending_count": pending_info.len(),
            "messages": pending_info,
        }))
    }

    /// Create a consumer group
    pub async fn create_consumer_group(
        &self,
        stream: &str,
        group: &str,
        start_id: Option<&str>,
    ) -> Result<()> {
        // Get connection
        let mut conn = self.get_connection().await?;

        let id = start_id.unwrap_or("0");

        // Create consumer group
        match conn.xgroup_create_mkstream(stream, group, id).await {
            Ok(_) => {
                info!("Created consumer group {} for stream {}", group, stream);
            }
            Err(e) => {
                // Group might already exist, which is okay
                if e.to_string().contains("BUSYGROUP") {
                    debug!("Consumer group {} already exists for stream {}", group, stream);
                } else {
                    return Err(EventStreamingError::redis(format!(
                        "Failed to create consumer group {} for stream {}: {}",
                        group, stream, e
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get stream information
    pub async fn get_stream_info(&self, stream: &str) -> Result<serde_json::Value> {
        // Get connection
        let mut conn = self.get_connection().await?;

        // Get stream info
        let info: HashMap<String, redis::Value> = conn
            .xinfo_stream(stream)
            .await
            .map_err(|e| EventStreamingError::redis(format!("Failed to get stream info for {}: {}", stream, e)))?;

        // Convert to JSON
        let mut json_info = serde_json::Map::new();
        for (key, value) in info {
            json_info.insert(key, redis_value_to_json(value));
        }

        Ok(serde_json::Value::Object(json_info))
    }

    /// List all streams
    pub async fn list_streams(&self) -> Result<Vec<String>> {
        // Get connection
        let mut conn = self.get_connection().await?;

        // Use SCAN to find stream keys
        let mut cursor = 0;
        let mut streams = Vec::new();

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = conn
                .scan_match(cursor, "*")
                .await
                .map_err(|e| EventStreamingError::redis(format!("Failed to scan keys: {}", e)))?;

            for key in keys {
                // Check if key is a stream
                match conn.exists(&key).await {
                    Ok(exists) if exists => {
                        match conn.type_of(&key).await {
                            Ok(key_type) if key_type == "stream" => {
                                streams.push(key);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(streams)
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<ComponentHealth> {
        let start_time = Instant::now();

        match self.get_connection().await {
            Ok(mut conn) => {
                // Test connection with PING
                match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                    Ok(_) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        Ok(ComponentHealth {
                            component: "redis".to_string(),
                            status: HealthStatus::Healthy,
                            last_check: chrono::Utc::now(),
                            response_time_ms: response_time,
                            details: [
                                ("url".to_string(), self.config.url.clone()),
                                ("pool_size".to_string(), self.config.pool.max_size.to_string()),
                            ].into(),
                        })
                    }
                    Err(e) => {
                        error!("Redis ping failed: {}", e);
                        Ok(ComponentHealth {
                            component: "redis".to_string(),
                            status: HealthStatus::Unhealthy,
                            last_check: chrono::Utc::now(),
                            response_time_ms: 0,
                            details: [("error".to_string(), e.to_string())].into(),
                        })
                    }
                }
            }
            Err(e) => {
                error!("Redis connection failed: {}", e);
                Ok(ComponentHealth {
                    component: "redis".to_string(),
                    status: HealthStatus::Unhealthy,
                    last_check: chrono::Utc::now(),
                    response_time_ms: 0,
                    details: [("error".to_string(), e.to_string())].into(),
                })
            }
        }
    }

    /// Get Redis connection
    async fn get_connection(&self) -> Result<redis::aio::ConnectionManager> {
        let pool = self.connection_pool.read().await;
        match pool.as_ref() {
            Some(conn) => Ok(conn.clone()),
            None => Err(EventStreamingError::redis("No Redis connection available")),
        }
    }

    /// Parse stream entry into event
    async fn parse_stream_entry(&self, stream_id: &StreamId) -> Result<Event> {
        let fields: HashMap<String, String> = stream_id
            .map
            .iter()
            .map(|(k, v)| (k.clone(), redis_value_to_string(v)))
            .collect();

        // Get payload and deserialize
        let payload_str = fields
            .get("payload")
            .ok_or_else(|| EventStreamingError::redis("Missing payload field"))?;

        let event: Event = serde_json::from_str(payload_str)
            .map_err(|e| EventStreamingError::redis(format!("Failed to deserialize event: {}", e)))?;

        Ok(event)
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
                                error!("Redis health monitoring failed: {}", e);
                                let mut status = manager.health_status.write().await;
                                *status = HealthStatus::Unhealthy;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Redis health monitor received shutdown signal");
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

    /// Create streams and consumer groups
    async fn create_streams_and_groups(&self) -> Result<()> {
        for (stream_name, stream_config) in &self.config.streams {
            info!("Creating stream: {}", stream_name);

            // Create the stream by adding a dummy entry and then removing it
            let mut conn = self.get_connection().await?;

            // Try to get stream info first
            match conn.xinfo_stream(stream_name).await {
                Ok(_) => {
                    debug!("Stream {} already exists", stream_name);
                }
                Err(_) => {
                    // Stream doesn't exist, create it
                    let _: String = conn
                        .xadd(stream_name, "*", &[("init", "true")])
                        .await
                        .map_err(|e| EventStreamingError::redis(format!("Failed to create stream {}: {}", stream_name, e)))?;

                    // Remove the initialization entry
                    let _: u64 = conn
                        .xtrim(stream_name, redis::streams::StreamMaxlen::Equals(0))
                        .await
                        .unwrap_or(0);

                    info!("Created stream: {}", stream_name);
                }
            }
        }

        // Create consumer groups
        for (group_name, group_config) in &self.config.consumer_groups {
            self.create_consumer_group(&group_config.group_name, group_name, Some("0")).await?;

            // Store consumer group info
            let mut groups = self.consumer_groups.write().await;
            groups.insert(group_name.clone(), ConsumerGroupInfo {
                group_name: group_config.group_name.clone(),
                consumer_name: group_config.consumer_name.clone(),
                stream_name: group_name.clone(), // Assuming group name matches stream name
                last_delivered_id: "0".to_string(),
                pending_count: 0,
            });
        }

        Ok(())
    }
}

/// Convert Redis value to JSON value
fn redis_value_to_json(value: redis::Value) -> serde_json::Value {
    match value {
        redis::Value::Nil => serde_json::Value::Null,
        redis::Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(i)),
        redis::Value::Data(data) => {
            if let Ok(s) = String::from_utf8(data) {
                serde_json::Value::String(s)
            } else {
                serde_json::Value::Null
            }
        }
        redis::Value::Bulk(bulk) => {
            let array: Vec<serde_json::Value> = bulk.into_iter().map(redis_value_to_json).collect();
            serde_json::Value::Array(array)
        }
        redis::Value::Status(s) => serde_json::Value::String(s),
        redis::Value::Okay => serde_json::Value::String("OK".to_string()),
    }
}

/// Convert Redis value to string
fn redis_value_to_string(value: &redis::Value) -> String {
    match value {
        redis::Value::Data(data) => String::from_utf8_lossy(data).to_string(),
        redis::Value::Status(s) => s.clone(),
        redis::Value::Int(i) => i.to_string(),
        redis::Value::Okay => "OK".to_string(),
        redis::Value::Nil => String::new(),
        redis::Value::Bulk(_) => "bulk".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::events::{Event, EventPayload};
    use crate::types::{EventCategory, EventSource};

    #[tokio::test]
    async fn test_redis_manager_creation() {
        let config = Config::default();
        let metrics = Arc::new(MetricsCollector::new(&config).await.unwrap());

        // This test might fail without a Redis server running
        if let Ok(_manager) = RedisStreamManager::new(&config, metrics).await {
            // Test passed - Redis is available
        } else {
            // Test skipped - Redis not available
            println!("Redis not available, test skipped");
        }
    }

    #[test]
    fn test_redis_value_conversion() {
        let value = redis::Value::Data(b"test".to_vec());
        assert_eq!(redis_value_to_string(&value), "test");

        let value = redis::Value::Int(42);
        assert_eq!(redis_value_to_string(&value), "42");

        let value = redis::Value::Status("OK".to_string());
        assert_eq!(redis_value_to_string(&value), "OK");
    }

    #[test]
    fn test_redis_value_to_json() {
        let value = redis::Value::Data(b"test".to_vec());
        let json = redis_value_to_json(value);
        assert_eq!(json, serde_json::Value::String("test".to_string()));

        let value = redis::Value::Int(42);
        let json = redis_value_to_json(value);
        assert_eq!(json, serde_json::Value::Number(serde_json::Number::from(42)));

        let value = redis::Value::Nil;
        let json = redis_value_to_json(value);
        assert_eq!(json, serde_json::Value::Null);
    }
}
