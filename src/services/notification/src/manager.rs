//! Notification Manager
//!
//! The core component that coordinates all notification operations including:
//! - Multi-channel delivery (email, SMS, push, webhook, WebSocket)
//! - Template management and rendering
//! - Delivery tracking and retry logic
//! - Subscription management
//! - Analytics and metrics collection

use crate::channels::{
    EmailChannel, NotificationChannel, PushChannel, SmsChannel, WebSocketChannel, WebhookChannel,
};
use crate::config::NotificationConfig;
use crate::error::{NotificationError, Result};
use crate::metrics::NotificationMetrics;
use crate::scheduler::NotificationScheduler;
use crate::templates::TemplateManager;

use ai_core_shared::types::*;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use mongodb::{
    bson::{doc, DateTime as BsonDateTime},
    options::{ClientOptions, FindOptions},
    Client as MongoClient, Collection, Database,
};
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::RwLock as TokioRwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Main notification manager that coordinates all notification operations
pub struct NotificationManager {
    config: NotificationConfig,

    // Database connections
    postgres: Option<PgPool>,
    mongo: Option<Database>,
    redis: Option<ConnectionManager>,

    // Channel handlers
    email_channel: Option<EmailChannel>,
    sms_channel: Option<SmsChannel>,
    push_channel: Option<PushChannel>,
    webhook_channel: WebhookChannel,
    websocket_channel: WebSocketChannel,

    // Core components
    template_manager: TemplateManager,
    scheduler: Option<NotificationScheduler>,
    metrics: NotificationMetrics,

    // Active connections and state
    active_connections: Arc<DashMap<String, DateTime<Utc>>>,
    notification_counter: AtomicU64,

    // Rate limiting
    rate_limiters: Arc<
        DashMap<
            String,
            Arc<
                TokioRwLock<
                    governor::RateLimiter<
                        governor::state::NotKeyed,
                        governor::state::InMemoryState,
                        governor::clock::DefaultClock,
                    >,
                >,
            >,
        >,
    >,
}

impl NotificationManager {
    /// Create a new notification manager
    pub async fn new(config: NotificationConfig) -> Result<Self> {
        info!("Initializing notification manager");

        // Validate configuration
        config.validate().map_err(NotificationError::config)?;

        // Initialize database connections
        let postgres = Self::init_postgres(&config).await?;
        let mongo = Self::init_mongo(&config).await?;
        let redis = Self::init_redis(&config).await?;

        // Initialize channels
        let email_channel = if config.email.enabled {
            Some(EmailChannel::new(&config.email).await?)
        } else {
            None
        };

        let sms_channel = if config.sms.enabled {
            Some(SmsChannel::new(&config.sms).await?)
        } else {
            None
        };

        let push_channel = if config.push.enabled {
            Some(PushChannel::new(&config.push).await?)
        } else {
            None
        };

        let webhook_channel = WebhookChannel::new(&config.webhook).await?;
        let websocket_channel = WebSocketChannel::new(&config.websocket).await?;

        // Initialize template manager
        let template_manager = TemplateManager::new(&config.template).await?;

        // Initialize metrics
        let metrics = NotificationMetrics::new(&config.metrics)?;

        // Initialize scheduler
        let scheduler = if config.scheduler.enabled {
            Some(NotificationScheduler::new(&config.scheduler).await?)
        } else {
            None
        };

        info!("Notification manager initialized successfully");

        Ok(Self {
            config,
            postgres,
            mongo,
            redis,
            email_channel,
            sms_channel,
            push_channel,
            webhook_channel,
            websocket_channel,
            template_manager,
            scheduler,
            metrics,
            active_connections: Arc::new(DashMap::new()),
            notification_counter: AtomicU64::new(0),
            rate_limiters: Arc::new(DashMap::new()),
        })
    }

    /// Send a single notification
    pub async fn send_notification(
        &self,
        request: CreateNotificationRequest,
    ) -> Result<NotificationResponse> {
        let notification_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Increment counter
        self.notification_counter.fetch_add(1, Ordering::Relaxed);

        // Validate request
        self.validate_notification_request(&request)?;

        // Check rate limits
        self.check_rate_limits(&request.recipient_id, &request.channels)
            .await?;

        // Get user preferences
        let user_preferences = self.get_user_preferences(&request.recipient_id).await?;

        // Filter channels based on user preferences and quiet hours
        let filtered_channels = if let Some(prefs) = &user_preferences {
            self.filter_channels_by_preferences(&request.channels, prefs, &now)?
        } else {
            request.channels.clone()
        };

        if filtered_channels.is_empty() {
            return Err(NotificationError::business_logic(
                "All channels filtered out by user preferences",
            ));
        }

        // Render content using template if specified
        let (title, content) = if let Some(template_id) = &request.template_id {
            self.render_notification_content(template_id, &request.template_data)
                .await?
        } else {
            (request.title.clone(), request.content.clone())
        };

        // Create notification record
        let mut notification = NotificationResponse {
            id: notification_id.clone(),
            recipient_id: request.recipient_id.clone(),
            notification_type: request.notification_type,
            title,
            content,
            channels: filtered_channels.clone(),
            priority: request.priority,
            status: NotificationStatus::Queued,
            delivery_attempts: Vec::new(),
            created_at: now,
            updated_at: now,
            scheduled_at: request.scheduled_at,
            delivered_at: None,
            expires_at: request.expires_at,
            metadata: request.metadata,
        };

        // Store notification in database
        self.store_notification(&notification).await?;

        // If scheduled for future, add to scheduler
        if let Some(scheduled_at) = request.scheduled_at {
            if scheduled_at > now {
                if let Some(ref scheduler) = self.scheduler {
                    scheduler.schedule_notification(&notification).await?;
                    return Ok(notification);
                }
            }
        }

        // Send immediately
        self.process_notification(&mut notification).await?;

        // Update notification status
        self.update_notification_status(&notification).await?;

        // Update metrics
        self.metrics.record_notification_sent(&notification).await;

        Ok(notification)
    }

    /// Send multiple notifications in a batch
    pub async fn send_bulk_notifications(
        &self,
        request: BulkNotificationRequest,
    ) -> Result<BulkNotificationResponse> {
        let batch_id = request
            .batch_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let total_notifications = request.notifications.len();
        let mut results = Vec::with_capacity(total_notifications);
        let mut successful = 0;
        let mut failed = 0;

        info!(
            "Processing bulk notification request with {} notifications",
            total_notifications
        );

        // Process notifications in parallel (with concurrency limit)
        let semaphore = Arc::new(tokio::sync::Semaphore::new(10)); // Limit to 10 concurrent operations
        let mut handles = Vec::new();

        for (index, notification_request) in request.notifications.into_iter().enumerate() {
            let manager = self.clone();
            let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
                NotificationError::internal(format!("Failed to acquire semaphore: {}", e))
            })?;

            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold the permit for the duration of the task
                let result = manager.send_notification(notification_request).await;

                match result {
                    Ok(notification) => BulkNotificationResult {
                        index,
                        status: BulkOperationStatus::Success,
                        notification_id: Some(notification.id),
                        error: None,
                    },
                    Err(e) => BulkNotificationResult {
                        index,
                        status: BulkOperationStatus::Failed,
                        notification_id: None,
                        error: Some(e.to_string()),
                    },
                }
            });

            handles.push(handle);
        }

        // Collect results
        for handle in handles {
            match handle.await {
                Ok(result) => {
                    match result.status {
                        BulkOperationStatus::Success => successful += 1,
                        BulkOperationStatus::Failed => failed += 1,
                        BulkOperationStatus::Skipped => {} // Not used in this implementation
                    }
                    results.push(result);
                }
                Err(e) => {
                    error!("Task panicked: {}", e);
                    failed += 1;
                    results.push(BulkNotificationResult {
                        index: results.len(),
                        status: BulkOperationStatus::Failed,
                        notification_id: None,
                        error: Some(format!("Task panicked: {}", e)),
                    });
                }
            }
        }

        // Sort results by index to maintain order
        results.sort_by_key(|r| r.index);

        let response = BulkNotificationResponse {
            batch_id,
            total_notifications,
            successful,
            failed,
            results,
        };

        info!(
            "Bulk notification completed: {} successful, {} failed",
            successful, failed
        );

        Ok(response)
    }

    /// Get notification by ID
    pub async fn get_notification(&self, id: &str) -> Result<Option<NotificationResponse>> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationResponse> = mongo.collection("notifications");
            let filter = doc! { "id": id };

            match collection.find_one(filter, None).await {
                Ok(result) => Ok(result),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    /// List notifications for a user
    pub async fn list_notifications(
        &self,
        user_id: &str,
        status: Option<NotificationStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<NotificationResponse>> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationResponse> = mongo.collection("notifications");
            let mut filter = doc! { "recipient_id": user_id };

            if let Some(status) = status {
                filter.insert("status", status.to_string());
            }

            let options = FindOptions::builder()
                .limit(limit.map(|l| l as i64))
                .skip(offset.map(|o| o as u64))
                .sort(doc! { "created_at": -1 })
                .build();

            match collection.find(filter, options).await {
                Ok(mut cursor) => {
                    let mut notifications = Vec::new();
                    while cursor
                        .advance()
                        .await
                        .map_err(|e| NotificationError::database(e.to_string()))?
                    {
                        let notification = cursor
                            .deserialize_current()
                            .map_err(|e| NotificationError::database(e.to_string()))?;
                        notifications.push(notification);
                    }
                    Ok(notifications)
                }
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    /// Cancel a pending notification
    pub async fn cancel_notification(&self, id: &str) -> Result<bool> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationResponse> = mongo.collection("notifications");
            let filter =
                doc! { "id": id, "status": { "$in": ["pending", "queued", "processing"] } };
            let update = doc! {
                "$set": {
                    "status": "cancelled",
                    "updated_at": BsonDateTime::now()
                }
            };

            match collection.update_one(filter, update, None).await {
                Ok(result) => Ok(result.modified_count > 0),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    // Template management methods

    /// Create a notification template
    pub async fn create_template(
        &self,
        request: CreateTemplateRequest,
    ) -> Result<NotificationTemplate> {
        self.template_manager.create_template(request).await
    }

    /// Update a notification template
    pub async fn update_template(
        &self,
        id: &str,
        request: UpdateTemplateRequest,
    ) -> Result<NotificationTemplate> {
        self.template_manager.update_template(id, request).await
    }

    /// Get template by ID
    pub async fn get_template(&self, id: &str) -> Result<Option<NotificationTemplate>> {
        self.template_manager.get_template(id).await
    }

    /// List templates
    pub async fn list_templates(
        &self,
        notification_type: Option<NotificationType>,
        is_active: Option<bool>,
    ) -> Result<Vec<NotificationTemplate>> {
        self.template_manager
            .list_templates(notification_type, is_active)
            .await
    }

    /// Delete a template
    pub async fn delete_template(&self, id: &str) -> Result<bool> {
        self.template_manager.delete_template(id).await
    }

    // Subscription management methods

    /// Create a notification subscription
    pub async fn create_subscription(
        &self,
        user_id: &str,
        request: CreateSubscriptionRequest,
    ) -> Result<NotificationSubscription> {
        let subscription_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let subscription = NotificationSubscription {
            id: subscription_id,
            user_id: user_id.to_string(),
            notification_types: request.notification_types,
            channels: request.channels,
            preferences: request.preferences,
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        self.store_subscription(&subscription).await?;
        Ok(subscription)
    }

    /// Update a notification subscription
    pub async fn update_subscription(
        &self,
        id: &str,
        request: UpdateSubscriptionRequest,
    ) -> Result<NotificationSubscription> {
        let mut subscription = self
            .get_subscription(id)
            .await?
            .ok_or_else(|| NotificationError::not_found("subscription"))?;

        if let Some(types) = request.notification_types {
            subscription.notification_types = types;
        }
        if let Some(channels) = request.channels {
            subscription.channels = channels;
        }
        if let Some(preferences) = request.preferences {
            subscription.preferences = preferences;
        }
        if let Some(is_active) = request.is_active {
            subscription.is_active = is_active;
        }

        subscription.updated_at = Utc::now();
        self.update_subscription_record(&subscription).await?;
        Ok(subscription)
    }

    /// Get subscription by ID
    pub async fn get_subscription(&self, id: &str) -> Result<Option<NotificationSubscription>> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationSubscription> =
                mongo.collection("subscriptions");
            let filter = doc! { "id": id };

            match collection.find_one(filter, None).await {
                Ok(result) => Ok(result),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    /// List subscriptions for a user
    pub async fn list_user_subscriptions(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationSubscription>> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationSubscription> =
                mongo.collection("subscriptions");
            let filter = doc! { "user_id": user_id };

            match collection.find(filter, None).await {
                Ok(mut cursor) => {
                    let mut subscriptions = Vec::new();
                    while cursor
                        .advance()
                        .await
                        .map_err(|e| NotificationError::database(e.to_string()))?
                    {
                        let subscription = cursor
                            .deserialize_current()
                            .map_err(|e| NotificationError::database(e.to_string()))?;
                        subscriptions.push(subscription);
                    }
                    Ok(subscriptions)
                }
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    /// Delete a subscription
    pub async fn delete_subscription(&self, id: &str) -> Result<bool> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationSubscription> =
                mongo.collection("subscriptions");
            let filter = doc! { "id": id };

            match collection.delete_one(filter, None).await {
                Ok(result) => Ok(result.deleted_count > 0),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    /// Get notification statistics
    pub async fn get_notification_stats(
        &self,
        user_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<NotificationStats> {
        if let Some(ref mongo) = self.mongo {
            let _collection: Collection<NotificationResponse> = mongo.collection("notifications");

            let mut match_stage = doc! {};

            if let Some(user_id) = user_id {
                match_stage.insert("recipient_id", user_id);
            }

            if let Some(start_date) = start_date {
                match_stage.insert(
                    "created_at",
                    doc! { "$gte": BsonDateTime::from_system_time(start_date.into()) },
                );
            }

            if let Some(end_date) = end_date {
                let mut date_filter = match_stage
                    .get_document_mut("created_at")
                    .unwrap_or(&mut doc! {})
                    .clone();
                date_filter.insert("$lte", BsonDateTime::from_system_time(end_date.into()));
                match_stage.insert("created_at", date_filter);
            }

            let _pipeline = vec![
                doc! { "$match": match_stage },
                doc! {
                    "$group": {
                        "_id": null,
                        "total_sent": { "$sum": 1 },
                        "total_delivered": { "$sum": { "$cond": [{ "$eq": ["$status", "delivered"] }, 1, 0] } },
                        "total_failed": { "$sum": { "$cond": [{ "$eq": ["$status", "failed"] }, 1, 0] } },
                        "channels": { "$push": "$channels" }
                    }
                },
            ];

            // This is a simplified implementation - in a real system you'd want more complex aggregation
            let stats = NotificationStats {
                total_sent: 0,
                total_delivered: 0,
                total_failed: 0,
                delivery_rate: 0.0,
                average_delivery_time: None,
                channel_stats: HashMap::new(),
            };

            Ok(stats)
        } else {
            Err(NotificationError::service_unavailable("MongoDB"))
        }
    }

    /// Start the background scheduler
    pub async fn start_scheduler(&self) -> Result<()> {
        if let Some(ref scheduler) = self.scheduler {
            scheduler.start().await
        } else {
            Err(NotificationError::config("Scheduler is not enabled"))
        }
    }

    /// Stop the background scheduler
    pub async fn stop_scheduler(&self) -> Result<()> {
        if let Some(ref scheduler) = self.scheduler {
            scheduler.stop().await
        } else {
            Ok(()) // Already stopped
        }
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let mut health = serde_json::json!({
            "service": "notification",
            "status": "healthy",
            "timestamp": Utc::now(),
            "components": {}
        });

        let components = health["components"].as_object_mut().unwrap();

        // Check database connections
        if let Some(ref postgres) = self.postgres {
            let pg_healthy = sqlx::query("SELECT 1").execute(postgres).await.is_ok();
            components.insert(
                "postgres".to_string(),
                serde_json::json!({
                    "status": if pg_healthy { "healthy" } else { "unhealthy" }
                }),
            );
        }

        if let Some(ref redis) = self.redis {
            let redis_healthy = redis::cmd("PING")
                .query_async::<_, String>(&mut redis.clone())
                .await
                .is_ok();
            components.insert(
                "redis".to_string(),
                serde_json::json!({
                    "status": if redis_healthy { "healthy" } else { "unhealthy" }
                }),
            );
        }

        // Check channel health
        components.insert(
            "channels".to_string(),
            serde_json::json!({
                "email": self.email_channel.is_some(),
                "sms": self.sms_channel.is_some(),
                "push": self.push_channel.is_some(),
                "webhook": true,
                "websocket": true
            }),
        );

        // Add metrics
        components.insert(
            "metrics".to_string(),
            serde_json::json!({
                "total_notifications": self.notification_counter.load(Ordering::Relaxed),
                "active_connections": self.active_connections.len()
            }),
        );

        Ok(health)
    }

    // Private helper methods

    async fn init_postgres(config: &NotificationConfig) -> Result<Option<PgPool>> {
        if !config.database.postgres_url.is_empty() {
            match PgPoolOptions::new()
                .max_connections(config.database.max_pool_size)
                .min_connections(config.database.min_pool_size)
                .acquire_timeout(Duration::from_secs(
                    config.database.connection_timeout_seconds,
                ))
                .idle_timeout(Duration::from_secs(config.database.idle_timeout_seconds))
                .connect(&config.database.postgres_url)
                .await
            {
                Ok(pool) => {
                    info!("Connected to PostgreSQL");
                    Ok(Some(pool))
                }
                Err(e) => {
                    warn!("Failed to connect to PostgreSQL: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn init_mongo(config: &NotificationConfig) -> Result<Option<Database>> {
        if !config.database.mongo_url.is_empty() {
            match ClientOptions::parse(&config.database.mongo_url).await {
                Ok(options) => match MongoClient::with_options(options) {
                    Ok(client) => {
                        let database = client.database("aicore");
                        info!("Connected to MongoDB");
                        Ok(Some(database))
                    }
                    Err(e) => {
                        warn!("Failed to connect to MongoDB: {}", e);
                        Ok(None)
                    }
                },
                Err(e) => {
                    warn!("Failed to parse MongoDB URL: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn init_redis(config: &NotificationConfig) -> Result<Option<ConnectionManager>> {
        if !config.redis.url.is_empty() {
            match RedisClient::open(config.redis.url.as_str()) {
                Ok(client) => match ConnectionManager::new(client).await {
                    Ok(manager) => {
                        info!("Connected to Redis");
                        Ok(Some(manager))
                    }
                    Err(e) => {
                        warn!("Failed to connect to Redis: {}", e);
                        Ok(None)
                    }
                },
                Err(e) => {
                    warn!("Failed to create Redis client: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn validate_notification_request(&self, request: &CreateNotificationRequest) -> Result<()> {
        if request.recipient_id.is_empty() {
            return Err(NotificationError::validation(
                "recipient_id",
                "cannot be empty",
            ));
        }

        if request.title.is_empty() {
            return Err(NotificationError::validation("title", "cannot be empty"));
        }

        if request.content.is_empty() {
            return Err(NotificationError::validation("content", "cannot be empty"));
        }

        if request.channels.is_empty() {
            return Err(NotificationError::validation(
                "channels",
                "at least one channel required",
            ));
        }

        // Validate channels are supported
        for channel in &request.channels {
            match channel {
                ai_core_shared::types::NotificationChannel::Email => {
                    if self.email_channel.is_none() {
                        return Err(NotificationError::config("Email channel not configured"));
                    }
                }
                ai_core_shared::types::NotificationChannel::Sms => {
                    if self.sms_channel.is_none() {
                        return Err(NotificationError::config("SMS channel not configured"));
                    }
                }
                ai_core_shared::types::NotificationChannel::Push => {
                    if self.push_channel.is_none() {
                        return Err(NotificationError::config("Push channel not configured"));
                    }
                }
                _ => {} // Webhook and WebSocket are always available
            }
        }

        Ok(())
    }

    async fn check_rate_limits(
        &self,
        user_id: &str,
        channels: &[ai_core_shared::types::NotificationChannel],
    ) -> Result<()> {
        if !self.config.rate_limit.enabled {
            return Ok(());
        }

        for channel in channels {
            let channel_key = format!("{}:{}", user_id, channel_to_string(channel));

            // Get or create rate limiter for this user/channel combination
            let rate_limiter = self.get_rate_limiter(&channel_key, channel).await?;

            // Check rate limit
            if rate_limiter.read().await.check().is_err() {
                return Err(NotificationError::rate_limit(format!(
                    "Rate limit exceeded for {} channel",
                    channel_to_string(channel)
                )));
            }
        }

        Ok(())
    }

    async fn get_rate_limiter(
        &self,
        key: &str,
        channel: &ai_core_shared::types::NotificationChannel,
    ) -> Result<
        Arc<
            TokioRwLock<
                governor::RateLimiter<
                    governor::state::NotKeyed,
                    governor::state::InMemoryState,
                    governor::clock::DefaultClock,
                >,
            >,
        >,
    > {
        if let Some(limiter) = self.rate_limiters.get(key) {
            Ok(limiter.clone())
        } else {
            let channel_str = channel_to_string(channel);
            let limit = self
                .config
                .rate_limit
                .channel_limits
                .get(&channel_str)
                .map(|l| l.per_minute)
                .unwrap_or(self.config.rate_limit.default_per_minute);

            let quota = governor::Quota::per_minute(
                std::num::NonZeroU32::new(limit).unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
            );
            let limiter = Arc::new(TokioRwLock::new(governor::RateLimiter::direct(quota)));

            self.rate_limiters.insert(key.to_string(), limiter.clone());
            Ok(limiter)
        }
    }

    async fn get_user_preferences(&self, user_id: &str) -> Result<Option<NotificationPreferences>> {
        // Try to get from cache first
        if let Some(ref mut redis) = self.redis.clone() {
            let cache_key = format!("{}user_prefs:{}", self.config.redis.key_prefix, user_id);
            if let Ok(cached) = redis.get::<_, String>(cache_key).await {
                if let Ok(preferences) = serde_json::from_str::<NotificationPreferences>(&cached) {
                    return Ok(Some(preferences));
                }
            }
        }

        // Fallback to database
        if let Some(ref postgres) = self.postgres {
            let query = "SELECT notification_preferences FROM user_preferences WHERE user_id = $1";
            match sqlx::query_scalar::<_, serde_json::Value>(query)
                .bind(user_id)
                .fetch_optional(postgres)
                .await
            {
                Ok(Some(value)) => {
                    if let Ok(preferences) =
                        serde_json::from_value::<NotificationPreferences>(value)
                    {
                        return Ok(Some(preferences));
                    }
                }
                Ok(None) => return Ok(None),
                Err(e) => warn!("Failed to get user preferences from database: {}", e),
            }
        }

        Ok(None)
    }

    fn filter_channels_by_preferences(
        &self,
        channels: &[ai_core_shared::types::NotificationChannel],
        preferences: &NotificationPreferences,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ai_core_shared::types::NotificationChannel>> {
        // Check quiet hours
        if let Some(ref quiet_hours) = preferences.quiet_hours {
            if self.is_in_quiet_hours(now, quiet_hours)? {
                // Only allow critical notifications during quiet hours
                return Ok(vec![ai_core_shared::types::NotificationChannel::Push]);
                // Or filter by priority
            }
        }

        // Check channel preferences
        let mut filtered = Vec::new();
        for channel in channels {
            let allowed = match channel {
                ai_core_shared::types::NotificationChannel::Email => {
                    preferences.email_notifications
                }
                ai_core_shared::types::NotificationChannel::Sms => preferences.sms_notifications,
                ai_core_shared::types::NotificationChannel::Push => preferences.push_notifications,
                ai_core_shared::types::NotificationChannel::Webhook => {
                    preferences.webhook_notifications
                }
                ai_core_shared::types::NotificationChannel::Websocket => {
                    preferences.websocket_notifications
                }
            };

            if allowed {
                filtered.push(channel.clone());
            }
        }

        Ok(filtered)
    }

    fn is_in_quiet_hours(
        &self,
        _now: &chrono::DateTime<chrono::Utc>,
        _quiet_hours: &ai_core_shared::types::QuietHours,
    ) -> Result<bool> {
        // This is a simplified implementation
        // In a real system, you'd want proper timezone handling
        Ok(false) // For now, never in quiet hours
    }

    async fn render_notification_content(
        &self,
        template_id: &str,
        template_data: &Option<serde_json::Value>,
    ) -> Result<(String, String)> {
        self.template_manager
            .render_notification(template_id, template_data)
            .await
    }

    async fn store_notification(&self, notification: &NotificationResponse) -> Result<()> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationResponse> = mongo.collection("notifications");
            match collection.insert_one(notification, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            // Store in memory or skip if no database
            warn!("No MongoDB connection, notification not persisted");
            Ok(())
        }
    }

    async fn process_notification(&self, notification: &mut NotificationResponse) -> Result<()> {
        notification.status = NotificationStatus::Processing;
        notification.updated_at = Utc::now();

        let mut successful_channels = 0;
        let total_channels = notification.channels.len();

        // Process each channel
        for channel in &notification.channels {
            let attempt_id = Uuid::new_v4().to_string();
            let attempt_start = Utc::now();

            let delivery_result = match channel {
                ai_core_shared::types::NotificationChannel::Email => {
                    if let Some(ref email_channel) = self.email_channel {
                        email_channel.send_notification(notification).await
                    } else {
                        Err(NotificationError::config("Email channel not configured"))
                    }
                }
                ai_core_shared::types::NotificationChannel::Sms => {
                    if let Some(ref sms_channel) = self.sms_channel {
                        sms_channel.send_notification(notification).await
                    } else {
                        Err(NotificationError::config("SMS channel not configured"))
                    }
                }
                ai_core_shared::types::NotificationChannel::Push => {
                    if let Some(ref push_channel) = self.push_channel {
                        push_channel.send_notification(notification).await
                    } else {
                        Err(NotificationError::config("Push channel not configured"))
                    }
                }
                ai_core_shared::types::NotificationChannel::Webhook => {
                    self.webhook_channel.send_notification(notification).await
                }
                ai_core_shared::types::NotificationChannel::Websocket => {
                    self.websocket_channel.send_notification(notification).await
                }
            };

            let attempt = match delivery_result {
                Ok(_) => {
                    successful_channels += 1;
                    DeliveryAttempt {
                        id: attempt_id,
                        channel: channel.clone(),
                        attempted_at: attempt_start,
                        status: DeliveryStatus::Success,
                        response: Some("Delivered successfully".to_string()),
                        error: None,
                        retry_count: 0,
                        next_retry_at: None,
                    }
                }
                Err(e) => DeliveryAttempt {
                    id: attempt_id,
                    channel: channel.clone(),
                    attempted_at: attempt_start,
                    status: DeliveryStatus::Failed,
                    response: None,
                    error: Some(e.to_string()),
                    retry_count: 0,
                    next_retry_at: if e.is_retryable() {
                        Some(Utc::now() + chrono::Duration::seconds(60))
                    } else {
                        None
                    },
                },
            };

            notification.delivery_attempts.push(attempt);
        }

        // Update notification status based on delivery results
        if successful_channels == total_channels {
            notification.status = NotificationStatus::Delivered;
            notification.delivered_at = Some(Utc::now());
        } else if successful_channels > 0 {
            notification.status = NotificationStatus::PartiallyDelivered;
        } else {
            notification.status = NotificationStatus::Failed;
        }

        notification.updated_at = Utc::now();

        Ok(())
    }

    async fn update_notification_status(&self, notification: &NotificationResponse) -> Result<()> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationResponse> = mongo.collection("notifications");
            let filter = doc! { "id": &notification.id };
            let update = doc! {
                "$set": {
                    "status": notification.status.to_string(),
                    "delivery_attempts": mongodb::bson::to_bson(&notification.delivery_attempts).unwrap(),
                    "delivered_at": notification.delivered_at.map(|dt| BsonDateTime::from_system_time(dt.into())),
                    "updated_at": BsonDateTime::from_system_time(notification.updated_at.into())
                }
            };

            match collection.update_one(filter, update, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(())
        }
    }

    async fn store_subscription(&self, subscription: &NotificationSubscription) -> Result<()> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationSubscription> =
                mongo.collection("subscriptions");
            match collection.insert_one(subscription, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(())
        }
    }

    async fn update_subscription_record(
        &self,
        subscription: &NotificationSubscription,
    ) -> Result<()> {
        if let Some(ref mongo) = self.mongo {
            let collection: Collection<NotificationSubscription> =
                mongo.collection("subscriptions");
            let filter = doc! { "id": &subscription.id };
            let update = doc! {
                "$set": {
                    "notification_types": mongodb::bson::to_bson(&subscription.notification_types).unwrap(),
                    "channels": mongodb::bson::to_bson(&subscription.channels).unwrap(),
                    "preferences": mongodb::bson::to_bson(&subscription.preferences).unwrap(),
                    "is_active": subscription.is_active,
                    "updated_at": BsonDateTime::from_system_time(subscription.updated_at.into())
                }
            };

            match collection.update_one(filter, update, None).await {
                Ok(_) => Ok(()),
                Err(e) => Err(NotificationError::database(e.to_string())),
            }
        } else {
            Ok(())
        }
    }
}

impl Clone for NotificationManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            postgres: self.postgres.clone(),
            mongo: self.mongo.clone(),
            redis: self.redis.clone(),
            email_channel: self.email_channel.clone(),
            sms_channel: self.sms_channel.clone(),
            push_channel: self.push_channel.clone(),
            webhook_channel: self.webhook_channel.clone(),
            websocket_channel: self.websocket_channel.clone(),
            template_manager: self.template_manager.clone(),
            scheduler: self.scheduler.clone(),
            metrics: self.metrics.clone(),
            active_connections: self.active_connections.clone(),
            notification_counter: AtomicU64::new(self.notification_counter.load(Ordering::Relaxed)),
            rate_limiters: self.rate_limiters.clone(),
        }
    }
}

fn channel_to_string(channel: &ai_core_shared::types::NotificationChannel) -> String {
    match channel {
        ai_core_shared::types::NotificationChannel::Email => "email".to_string(),
        ai_core_shared::types::NotificationChannel::Sms => "sms".to_string(),
        ai_core_shared::types::NotificationChannel::Push => "push".to_string(),
        ai_core_shared::types::NotificationChannel::Webhook => "webhook".to_string(),
        ai_core_shared::types::NotificationChannel::Websocket => "websocket".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_validate_notification_request() {
        let config = NotificationConfig::default();
        let manager = NotificationManager::new(config).await.unwrap();

        let valid_request = CreateNotificationRequest {
            recipient_id: "user123".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test".to_string(),
            content: "Test content".to_string(),
            channels: vec![ai_core_shared::types::NotificationChannel::Email],
            priority: NotificationPriority::Normal,
            template_id: None,
            template_data: None,
            scheduled_at: None,
            expires_at: None,
            metadata: None,
        };

        assert!(manager
            .validate_notification_request(&valid_request)
            .is_ok());

        let invalid_request = CreateNotificationRequest {
            recipient_id: "".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test".to_string(),
            content: "Test content".to_string(),
            channels: vec![ai_core_shared::types::NotificationChannel::Email],
            priority: NotificationPriority::Normal,
            template_id: None,
            template_data: None,
            scheduled_at: None,
            expires_at: None,
            metadata: None,
        };

        assert!(manager
            .validate_notification_request(&invalid_request)
            .is_err());
    }
}
