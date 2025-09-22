//! Redis connection and operations module
//!
//! Provides Redis connection management, caching operations, and pub/sub functionality
//! for the AI-CORE intelligent automation platform.

use anyhow::Result;
use futures::StreamExt;
use redis::{
    aio::{ConnectionManager, MultiplexedConnection},
    AsyncCommands, Client, RedisError,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{broadcast, RwLock},
    time::timeout,
};
use tracing::{debug, error, info, warn};

use crate::DatabaseError;

/// Redis connection configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub response_timeout_seconds: u64,
    pub retry_attempts: u32,
    pub enable_cluster: bool,
    pub default_ttl_seconds: u64,
    pub max_pool_size: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            max_connections: 20,
            min_connections: 5,
            connection_timeout_seconds: 10,
            response_timeout_seconds: 5,
            retry_attempts: 3,
            enable_cluster: false,
            default_ttl_seconds: 3600, // 1 hour
            max_pool_size: 50,
        }
    }
}

/// Redis connection statistics
#[derive(Debug, Clone, Serialize)]
pub struct RedisStats {
    pub total_connections: u32,
    pub active_connections: u32,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_sets: u64,
    pub cache_deletes: u64,
    pub pub_messages: u64,
    pub sub_messages: u64,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: Option<u64>,
}

impl Default for RedisStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_sets: 0,
            cache_deletes: 0,
            pub_messages: 0,
            sub_messages: 0,
            last_error: None,
            uptime_seconds: 0,
            memory_usage_bytes: None,
        }
    }
}

/// Pub/Sub message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubMessage<T> {
    pub channel: String,
    pub data: T,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message_id: uuid::Uuid,
}

impl<T> PubSubMessage<T> {
    pub fn new(channel: String, data: T) -> Self {
        Self {
            channel,
            data,
            timestamp: chrono::Utc::now(),
            message_id: uuid::Uuid::new_v4(),
        }
    }
}

/// Redis connection manager with caching and pub/sub support
pub struct RedisConnection {
    connection_manager: Arc<ConnectionManager>,
    pub_connection: Arc<RwLock<MultiplexedConnection>>,
    config: RedisConfig,
    stats: Arc<RwLock<RedisStats>>,
    created_at: Instant,
    pub_channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl RedisConnection {
    /// Create a new Redis connection with the given configuration
    pub async fn new(config: RedisConfig) -> Result<Self, DatabaseError> {
        info!("Connecting to Redis at {}", config.url);

        // Create Redis client
        let client = Client::open(config.url.clone()).map_err(|e| {
            DatabaseError::Connection(format!("Failed to create Redis client: {}", e))
        })?;

        // Create connection manager for regular operations
        let connection_manager = ConnectionManager::new(client.clone()).await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to create connection manager: {}", e))
        })?;

        // Create multiplexed connection for pub/sub
        let pub_connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to create pub/sub connection: {}", e))
            })?;

        let redis_conn = Self {
            connection_manager: Arc::new(connection_manager),
            pub_connection: Arc::new(RwLock::new(pub_connection)),
            config,
            stats: Arc::new(RwLock::new(RedisStats::default())),
            created_at: Instant::now(),
            pub_channels: Arc::new(RwLock::new(HashMap::new())),
        };

        // Test the connection
        redis_conn
            .health_check()
            .await
            .map_err(|e| DatabaseError::Connection(format!("Redis health check failed: {}", e)))?;

        info!("Successfully connected to Redis");
        Ok(redis_conn)
    }

    /// Perform health check on Redis connection
    pub async fn health_check(&self) -> Result<bool, DatabaseError> {
        let timeout_duration = Duration::from_secs(self.config.response_timeout_seconds);

        let result = timeout(timeout_duration, async {
            let mut conn = (*self.connection_manager).clone();
            let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
            Ok::<String, RedisError>(pong)
        })
        .await;

        match result {
            Ok(Ok(pong)) if pong == "PONG" => {
                debug!("Redis health check successful");
                Ok(true)
            }
            Ok(Ok(unexpected)) => {
                warn!(
                    "Redis health check returned unexpected response: {}",
                    unexpected
                );
                Ok(false)
            }
            Ok(Err(e)) => {
                error!("Redis health check failed: {}", e);
                self.update_stats_error(format!("Health check error: {}", e))
                    .await;
                Err(DatabaseError::Connection(format!(
                    "Redis health check failed: {}",
                    e
                )))
            }
            Err(_) => {
                error!(
                    "Redis health check timed out after {} seconds",
                    self.config.response_timeout_seconds
                );
                Err(DatabaseError::Connection(
                    "Redis health check timed out".to_string(),
                ))
            }
        }
    }

    /// Get Redis connection statistics
    pub async fn get_stats(&self) -> RedisStats {
        let mut stats = self.stats.read().await.clone();
        stats.uptime_seconds = self.created_at.elapsed().as_secs();

        // Try to get Redis memory usage
        if let Ok(info) = self.get_redis_info().await {
            stats.memory_usage_bytes = info.get("used_memory").and_then(|v| v.parse().ok());
        }

        stats
    }

    /// Get Redis server information
    async fn get_redis_info(&self) -> Result<HashMap<String, String>, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let info_str: String = redis::cmd("INFO")
            .arg("memory")
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to get Redis info: {}", e)))?;

        let mut info = HashMap::new();
        for line in info_str.lines() {
            if let Some((key, value)) = line.split_once(':') {
                info.insert(key.to_string(), value.to_string());
            }
        }
        Ok(info)
    }

    /// Update statistics with error information
    async fn update_stats_error(&self, error: String) {
        let mut stats = self.stats.write().await;
        stats.last_error = Some(error);
    }

    // ===== CACHING OPERATIONS =====

    /// Set a value in Redis cache with TTL
    pub async fn set_with_ttl<T>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: u64,
    ) -> Result<(), DatabaseError>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)
            .map_err(|e| DatabaseError::Validation(format!("Failed to serialize value: {}", e)))?;

        let mut conn = (*self.connection_manager).clone();
        let _: () = conn
            .set_ex(key, serialized, ttl_seconds)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to set cache value: {}", e)))?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.cache_sets += 1;

        debug!("Set cache key '{}' with TTL {} seconds", key, ttl_seconds);
        Ok(())
    }

    /// Set a value in Redis cache with default TTL
    pub async fn set<T>(&self, key: &str, value: &T) -> Result<(), DatabaseError>
    where
        T: Serialize,
    {
        self.set_with_ttl(key, value, self.config.default_ttl_seconds)
            .await
    }

    /// Get a value from Redis cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, DatabaseError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = (*self.connection_manager).clone();
        let result: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to get cache value: {}", e)))?;

        let mut stats = self.stats.write().await;

        match result {
            Some(serialized) => {
                stats.cache_hits += 1;
                let value = serde_json::from_str(&serialized).map_err(|e| {
                    DatabaseError::Validation(format!("Failed to deserialize value: {}", e))
                })?;
                debug!("Cache hit for key '{}'", key);
                Ok(Some(value))
            }
            None => {
                stats.cache_misses += 1;
                debug!("Cache miss for key '{}'", key);
                Ok(None)
            }
        }
    }

    /// Delete a key from Redis cache
    pub async fn delete(&self, key: &str) -> Result<bool, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let deleted: u32 = conn
            .del(key)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to delete cache key: {}", e)))?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.cache_deletes += 1;

        debug!("Deleted cache key '{}', result: {}", key, deleted > 0);
        Ok(deleted > 0)
    }

    /// Check if a key exists in Redis
    pub async fn exists(&self, key: &str) -> Result<bool, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let exists: bool = conn.exists(key).await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to check key existence: {}", e))
        })?;

        debug!("Key '{}' exists: {}", key, exists);
        Ok(exists)
    }

    /// Set TTL for an existing key
    pub async fn expire(&self, key: &str, ttl_seconds: u64) -> Result<bool, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let result: bool = conn
            .expire(key, ttl_seconds as i64)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to set TTL: {}", e)))?;

        debug!(
            "Set TTL for key '{}' to {} seconds, result: {}",
            key, ttl_seconds, result
        );
        Ok(result)
    }

    /// Get TTL for a key
    pub async fn ttl(&self, key: &str) -> Result<i64, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let ttl: i64 = conn
            .ttl(key)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to get TTL: {}", e)))?;

        debug!("TTL for key '{}': {}", key, ttl);
        Ok(ttl)
    }

    /// Increment a numeric value
    pub async fn incr(&self, key: &str, delta: i64) -> Result<i64, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let new_value: i64 = if delta == 1 {
            conn.incr(key, 1).await
        } else {
            conn.incr(key, delta).await
        }
        .map_err(|e| DatabaseError::Connection(format!("Failed to increment key: {}", e)))?;

        debug!(
            "Incremented key '{}' by {}, new value: {}",
            key, delta, new_value
        );
        Ok(new_value)
    }

    /// Set multiple key-value pairs atomically
    pub async fn mset<T>(&self, pairs: &[(&str, &T)]) -> Result<(), DatabaseError>
    where
        T: Serialize,
    {
        let mut conn = (*self.connection_manager).clone();
        let mut redis_pairs = Vec::new();

        for (key, value) in pairs {
            let serialized = serde_json::to_string(value).map_err(|e| {
                DatabaseError::Validation(format!("Failed to serialize value: {}", e))
            })?;
            redis_pairs.push((*key, serialized));
        }

        let _: () = conn.mset(&redis_pairs).await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to set multiple values: {}", e))
        })?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.cache_sets += pairs.len() as u64;

        debug!("Set {} key-value pairs", pairs.len());
        Ok(())
    }

    /// Get multiple values by keys
    pub async fn mget<T>(&self, keys: &[&str]) -> Result<Vec<Option<T>>, DatabaseError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = (*self.connection_manager).clone();
        let results: Vec<Option<String>> = conn.mget(keys).await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to get multiple values: {}", e))
        })?;

        let mut values = Vec::new();
        let mut hits = 0;
        let mut misses = 0;

        for result in results {
            match result {
                Some(serialized) => {
                    let value = serde_json::from_str(&serialized).map_err(|e| {
                        DatabaseError::Validation(format!("Failed to deserialize value: {}", e))
                    })?;
                    values.push(Some(value));
                    hits += 1;
                }
                None => {
                    values.push(None);
                    misses += 1;
                }
            }
        }

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.cache_hits += hits;
        stats.cache_misses += misses;

        debug!(
            "Got {} values ({} hits, {} misses)",
            keys.len(),
            hits,
            misses
        );
        Ok(values)
    }

    // ===== PUB/SUB OPERATIONS =====

    /// Publish a message to a channel
    pub async fn publish<T>(&self, channel: &str, message: &T) -> Result<u32, DatabaseError>
    where
        T: Serialize,
    {
        let pub_message = PubSubMessage::new(channel.to_string(), message);
        let serialized = serde_json::to_string(&pub_message).map_err(|e| {
            DatabaseError::Validation(format!("Failed to serialize message: {}", e))
        })?;

        let mut conn = (*self.connection_manager).clone();
        let subscriber_count: u32 = conn
            .publish(channel, serialized)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to publish message: {}", e)))?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.pub_messages += 1;

        info!(
            "Published message to channel '{}', {} subscribers notified",
            channel, subscriber_count
        );
        Ok(subscriber_count)
    }

    /// Subscribe to a channel and return a receiver for messages
    pub async fn subscribe(
        &self,
        channel: &str,
    ) -> Result<broadcast::Receiver<String>, DatabaseError> {
        let mut channels = self.pub_channels.write().await;

        // Check if we already have a sender for this channel
        if let Some(sender) = channels.get(channel) {
            let receiver = sender.subscribe();
            debug!("Subscribed to existing channel '{}'", channel);
            return Ok(receiver);
        }

        // Create new channel broadcaster
        let (tx, rx) = broadcast::channel(1000); // Buffer up to 1000 messages
        channels.insert(channel.to_string(), tx.clone());

        // Spawn task to handle Redis subscription
        let channel_name = channel.to_string();
        let pub_connection = Arc::clone(&self.pub_connection);
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            if let Err(e) =
                Self::handle_redis_subscription(channel_name, pub_connection, tx, stats).await
            {
                error!("Redis subscription handler failed: {}", e);
            }
        });

        debug!("Created new subscription for channel '{}'", channel);
        Ok(rx)
    }

    /// Handle Redis subscription in a separate task
    async fn handle_redis_subscription(
        channel: String,
        _pub_connection: Arc<RwLock<MultiplexedConnection>>,
        broadcaster: broadcast::Sender<String>,
        stats: Arc<RwLock<RedisStats>>,
    ) -> Result<(), DatabaseError> {
        // Create a new connection for pubsub (simpler approach)
        let client = Client::open(format!("redis://localhost:6379")).map_err(|e| {
            DatabaseError::Connection(format!("Failed to create pubsub client: {}", e))
        })?;

        let conn = client.get_async_connection().await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to get async connection: {}", e))
        })?;

        let mut pubsub = conn.into_pubsub();

        pubsub.subscribe(&channel).await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to subscribe to channel: {}", e))
        })?;

        info!("Started Redis subscription for channel '{}'", channel);

        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(msg) => {
                    if let Ok(payload) = msg.get_payload::<String>() {
                        // Update statistics
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.sub_messages += 1;
                        }

                        // Broadcast to local subscribers
                        if let Err(_e) = broadcaster.send(payload) {
                            // If all receivers are dropped, we can exit
                            if broadcaster.receiver_count() == 0 {
                                info!(
                                    "No more subscribers for channel '{}', ending subscription",
                                    channel
                                );
                                break;
                            }
                        }
                    }
                }
                None => {
                    warn!("Redis subscription stream ended for channel '{}'", channel);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Unsubscribe from a channel
    pub async fn unsubscribe(&self, channel: &str) -> Result<(), DatabaseError> {
        let mut channels = self.pub_channels.write().await;
        channels.remove(channel);

        debug!("Unsubscribed from channel '{}'", channel);
        Ok(())
    }

    /// Get list of active channels
    pub async fn get_active_channels(&self) -> Vec<String> {
        let channels = self.pub_channels.read().await;
        channels.keys().cloned().collect()
    }

    // ===== ADVANCED OPERATIONS =====

    /// Execute a Lua script
    pub async fn eval<T>(
        &self,
        script: &str,
        keys: &[&str],
        args: &[&str],
    ) -> Result<T, DatabaseError>
    where
        T: redis::FromRedisValue,
    {
        let mut conn = (*self.connection_manager).clone();
        let result = redis::Script::new(script)
            .key(keys)
            .arg(args)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to execute Lua script: {}", e))
            })?;

        debug!(
            "Executed Lua script with {} keys and {} args",
            keys.len(),
            args.len()
        );
        Ok(result)
    }

    /// Get Redis server configuration
    pub async fn get_config(&self, parameter: &str) -> Result<Vec<String>, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let config: Vec<String> = redis::cmd("CONFIG")
            .arg("GET")
            .arg(parameter)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to get Redis config: {}", e)))?;

        debug!(
            "Got Redis config for parameter '{}': {:?}",
            parameter, config
        );
        Ok(config)
    }

    /// Flush all data from current database
    pub async fn flush_db(&self) -> Result<(), DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to flush database: {}", e)))?;

        warn!("Flushed all data from current Redis database");
        Ok(())
    }

    /// Get database size (number of keys)
    pub async fn db_size(&self) -> Result<u64, DatabaseError> {
        let mut conn = (*self.connection_manager).clone();
        let size: u64 = redis::cmd("DBSIZE")
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to get database size: {}", e))
            })?;

        debug!("Redis database contains {} keys", size);
        Ok(size)
    }
}

/// Convenience functions for common caching patterns
impl RedisConnection {
    /// Cache-aside pattern: get from cache, or compute and cache if not found
    pub async fn get_or_set<T, F, Fut>(&self, key: &str, compute_fn: F) -> Result<T, DatabaseError>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, DatabaseError>>,
    {
        // Try to get from cache first
        if let Some(cached_value) = self.get(key).await? {
            return Ok(cached_value);
        }

        // Compute the value
        let computed_value = compute_fn().await?;

        // Cache the computed value
        self.set(key, &computed_value).await?;

        Ok(computed_value)
    }

    /// Cache-aside pattern with custom TTL
    pub async fn get_or_set_with_ttl<T, F, Fut>(
        &self,
        key: &str,
        ttl_seconds: u64,
        compute_fn: F,
    ) -> Result<T, DatabaseError>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, DatabaseError>>,
    {
        // Try to get from cache first
        if let Some(cached_value) = self.get(key).await? {
            return Ok(cached_value);
        }

        // Compute the value
        let computed_value = compute_fn().await?;

        // Cache the computed value with TTL
        self.set_with_ttl(key, &computed_value, ttl_seconds).await?;

        Ok(computed_value)
    }

    /// Session storage pattern
    pub async fn set_session<T>(
        &self,
        session_id: &str,
        data: &T,
        ttl_seconds: u64,
    ) -> Result<(), DatabaseError>
    where
        T: Serialize,
    {
        let key = format!("session:{}", session_id);
        self.set_with_ttl(&key, data, ttl_seconds).await
    }

    /// Get session data
    pub async fn get_session<T>(&self, session_id: &str) -> Result<Option<T>, DatabaseError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let key = format!("session:{}", session_id);
        self.get(&key).await
    }

    /// Refresh session TTL
    pub async fn refresh_session(
        &self,
        session_id: &str,
        ttl_seconds: u64,
    ) -> Result<bool, DatabaseError> {
        let key = format!("session:{}", session_id);
        self.expire(&key, ttl_seconds).await
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<bool, DatabaseError> {
        let key = format!("session:{}", session_id);
        self.delete(&key).await
    }

    /// Rate limiting pattern
    pub async fn check_rate_limit(
        &self,
        identifier: &str,
        window_seconds: u64,
        limit: u64,
    ) -> Result<bool, DatabaseError> {
        let key = format!(
            "rate_limit:{}:{}",
            identifier,
            chrono::Utc::now().timestamp() / window_seconds as i64
        );
        let current_count = self.incr(&key, 1).await?;

        if current_count == 1 {
            // Set TTL for the window
            self.expire(&key, window_seconds).await?;
        }

        Ok(current_count <= limit as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u32,
        name: String,
        value: f64,
    }

    async fn create_test_connection() -> RedisConnection {
        let config = RedisConfig {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            ..Default::default()
        };

        RedisConnection::new(config)
            .await
            .expect("Failed to create Redis connection")
    }

    #[tokio::test]
    async fn test_redis_connection() {
        let conn = create_test_connection().await;
        assert!(conn.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let conn = create_test_connection().await;

        let test_data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };

        // Test set and get
        conn.set("test_key", &test_data).await.unwrap();
        let retrieved: Option<TestData> = conn.get("test_key").await.unwrap();
        assert_eq!(retrieved, Some(test_data.clone()));

        // Test exists
        assert!(conn.exists("test_key").await.unwrap());
        assert!(!conn.exists("nonexistent_key").await.unwrap());

        // Test delete
        assert!(conn.delete("test_key").await.unwrap());
        let retrieved: Option<TestData> = conn.get("test_key").await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_ttl_operations() {
        let conn = create_test_connection().await;

        let test_data = TestData {
            id: 2,
            name: "ttl_test".to_string(),
            value: 100.0,
        };

        // Set with TTL
        conn.set_with_ttl("ttl_key", &test_data, 10).await.unwrap();

        // Check TTL
        let ttl = conn.ttl("ttl_key").await.unwrap();
        assert!(ttl > 0 && ttl <= 10);

        // Extend TTL
        assert!(conn.expire("ttl_key", 20).await.unwrap());
        let new_ttl = conn.ttl("ttl_key").await.unwrap();
        assert!(new_ttl > 10);

        // Clean up
        conn.delete("ttl_key").await.unwrap();
    }

    #[tokio::test]
    async fn test_increment_operations() {
        let conn = create_test_connection().await;

        // Test increment
        let value1 = conn.incr("counter", 1).await.unwrap();
        assert_eq!(value1, 1);

        let value2 = conn.incr("counter", 5).await.unwrap();
        assert_eq!(value2, 6);

        // Clean up
        conn.delete("counter").await.unwrap();
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let conn = create_test_connection().await;

        let data1 = TestData {
            id: 1,
            name: "first".to_string(),
            value: 1.0,
        };
        let data2 = TestData {
            id: 2,
            name: "second".to_string(),
            value: 2.0,
        };

        // Test mset
        let pairs = [("batch_key1", &data1), ("batch_key2", &data2)];
        conn.mset(&pairs).await.unwrap();

        // Test mget
        let keys = ["batch_key1", "batch_key2", "nonexistent"];
        let results: Vec<Option<TestData>> = conn.mget(&keys).await.unwrap();

        assert_eq!(results[0], Some(data1));
        assert_eq!(results[1], Some(data2));
        assert_eq!(results[2], None);

        // Clean up
        conn.delete("batch_key1").await.unwrap();
        conn.delete("batch_key2").await.unwrap();
    }

    #[tokio::test]
    async fn test_session_operations() {
        let conn = create_test_connection().await;

        let session_data = TestData {
            id: 999,
            name: "session_user".to_string(),
            value: 3.14,
        };

        let session_id = "test_session_123";

        // Set session
        conn.set_session(session_id, &session_data, 3600)
            .await
            .unwrap();

        // Get session
        let retrieved: Option<TestData> = conn.get_session(session_id).await.unwrap();
        assert_eq!(retrieved, Some(session_data));

        // Refresh session
        assert!(conn.refresh_session(session_id, 7200).await.unwrap());

        // Delete session
        assert!(conn.delete_session(session_id).await.unwrap());
        let retrieved: Option<TestData> = conn.get_session(session_id).await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let conn = create_test_connection().await;

        let identifier = "test_user";
        let window = 10; // 10 seconds
        let limit = 3;

        // First 3 requests should pass
        for _ in 0..3 {
            assert!(conn
                .check_rate_limit(identifier, window, limit)
                .await
                .unwrap());
        }

        // 4th request should fail
        assert!(!conn
            .check_rate_limit(identifier, window, limit)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_cache_aside_pattern() {
        let conn = create_test_connection().await;

        let key = "cache_aside_test";
        let mut call_count = 0;

        let compute_fn = || {
            call_count += 1;
            async move {
                Ok(TestData {
                    id: call_count,
                    name: format!("computed_{}", call_count),
                    value: call_count as f64 * 10.0,
                })
            }
        };

        // First call should compute and cache
        let result1: TestData = conn.get_or_set(key, compute_fn).await.unwrap();
        assert_eq!(result1.id, 1);

        // Second call should return cached value
        let result2: TestData = conn
            .get_or_set(key, || async {
                Ok(TestData {
                    id: 999,
                    name: "should_not_be_called".to_string(),
                    value: 999.0,
                })
            })
            .await
            .unwrap();
        assert_eq!(result2, result1); // Should be the same as cached value

        // Clean up
        conn.delete(key).await.unwrap();
    }

    #[tokio::test]
    async fn test_stats() {
        let conn = create_test_connection().await;

        // Perform some operations
        conn.set("stats_test", &"test_value").await.unwrap();
        let _: Option<String> = conn.get("stats_test").await.unwrap();
        let _: Option<String> = conn.get("nonexistent").await.unwrap();
        conn.delete("stats_test").await.unwrap();

        let stats = conn.get_stats().await;
        assert!(stats.cache_sets > 0);
        assert!(stats.cache_hits > 0);
        assert!(stats.cache_misses > 0);
        assert!(stats.cache_deletes > 0);
        assert!(stats.uptime_seconds > 0);
    }
}
