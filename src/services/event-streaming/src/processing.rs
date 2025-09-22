//! # Processing Pipeline Module
//!
//! This module provides the core event processing pipeline for the event streaming service.
//! It handles event routing, filtering, transformation, and coordination between different
//! messaging systems (Kafka, Redis Streams, etc.).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{EventStreamingError, Result},
    events::Event,
    kafka::KafkaManager,
    metrics::MetricsCollector,
    redis_streams::RedisStreamManager,
    routing::EventRouter,
    storage::EventStorage,
    types::{ComponentHealth, EventCategory, EventStatus, HealthStatus, ProcessingStats},
};

/// Main event processing pipeline
#[derive(Clone)]
pub struct ProcessingPipeline {
    config: Arc<Config>,
    kafka_manager: Arc<KafkaManager>,
    redis_manager: Arc<RedisStreamManager>,
    event_storage: Arc<EventStorage>,
    event_router: Arc<EventRouter>,
    metrics_collector: Arc<MetricsCollector>,
    processing_semaphore: Arc<Semaphore>,
    worker_handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    shutdown_tx: Arc<RwLock<Option<broadcast::Sender<()>>>>,
    health_status: Arc<RwLock<HealthStatus>>,
    processing_stats: Arc<RwLock<ProcessingStats>>,
    replay_jobs: Arc<RwLock<HashMap<Uuid, ReplayJob>>>,
}

/// Event processing context
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    pub event: Event,
    pub source_stream: String,
    pub processing_attempt: u32,
    pub started_at: DateTime<Utc>,
    pub timeout: Duration,
}

/// Replay job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayJob {
    pub id: Uuid,
    pub status: ReplayStatus,
    pub from_timestamp: DateTime<Utc>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub event_types: Option<Vec<String>>,
    pub categories: Option<Vec<EventCategory>>,
    pub batch_size: u32,
    pub total_events: u64,
    pub processed_events: u64,
    pub failed_events: u64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Replay job status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ProcessingPipeline {
    /// Create a new processing pipeline
    pub async fn new(
        config: &Config,
        kafka_manager: Arc<KafkaManager>,
        redis_manager: Arc<RedisStreamManager>,
        event_storage: Arc<EventStorage>,
        event_router: Arc<EventRouter>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Result<Self> {
        info!("Initializing Processing Pipeline");

        let processing_semaphore = Arc::new(Semaphore::new(config.processing.worker_threads));
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config: Arc::new(config.clone()),
            kafka_manager,
            redis_manager,
            event_storage,
            event_router,
            metrics_collector,
            processing_semaphore,
            worker_handles: Arc::new(RwLock::new(Vec::new())),
            shutdown_tx: Arc::new(RwLock::new(Some(shutdown_tx))),
            health_status: Arc::new(RwLock::new(HealthStatus::Healthy)),
            processing_stats: Arc::new(RwLock::new(ProcessingStats {
                total_received: 0,
                total_processed: 0,
                total_failed: 0,
                total_dead_letter: 0,
                total_filtered: 0,
                by_category: HashMap::new(),
                by_priority: HashMap::new(),
            })),
            replay_jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the processing pipeline
    pub async fn start(&self) -> Result<()> {
        info!("Starting Processing Pipeline");

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Healthy;
        }

        // Start worker tasks
        self.start_workers().await?;

        // Start metrics collection
        self.start_metrics_collection().await?;

        info!("Processing Pipeline started successfully");
        Ok(())
    }

    /// Stop the processing pipeline
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Processing Pipeline");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(());
        }

        // Update health status
        {
            let mut status = self.health_status.write().await;
            *status = HealthStatus::Unhealthy;
        }

        // Wait for workers to complete
        self.stop_workers().await;

        info!("Processing Pipeline stopped");
        Ok(())
    }

    /// Publish an event to the processing pipeline
    pub async fn publish_event(&self, event: Event) -> Result<()> {
        let start_time = Instant::now();

        debug!("Publishing event {} to processing pipeline", event.id);

        // Update stats
        {
            let mut stats = self.processing_stats.write().await;
            stats.total_received += 1;
            *stats.by_category.entry(event.category.clone()).or_insert(0) += 1;
            *stats.by_priority.entry(event.priority).or_insert(0) += 1;
        }

        // Route event to appropriate streams
        let destinations = self.event_router.route_event(&event).await?;

        // Publish to each destination
        for destination in &destinations {
            match destination.target.as_str() {
                target if target.starts_with("kafka:") => {
                    let topic = target.strip_prefix("kafka:").unwrap();
                    self.kafka_manager
                        .publish_event(topic, &event, None)
                        .await?;
                }
                target if target.starts_with("redis:") => {
                    let stream = target.strip_prefix("redis:").unwrap();
                    self.redis_manager.publish_event(stream, &event).await?;
                }
                _ => {
                    warn!("Unknown destination target: {}", destination.target);
                }
            }
        }

        // Store event for audit and replay
        self.event_storage.store_event(&event).await?;

        let duration = start_time.elapsed();

        // Record metrics
        self.metrics_collector
            .record_event_published(duration)
            .await?;

        debug!(
            "Event {} published to {} destinations in {:?}",
            event.id,
            destinations.len(),
            duration
        );

        Ok(())
    }

    /// Start event replay
    pub async fn start_replay(
        &self,
        from_timestamp: DateTime<Utc>,
        to_timestamp: Option<DateTime<Utc>>,
        event_types: Option<Vec<String>>,
        categories: Option<Vec<EventCategory>>,
        batch_size: u32,
    ) -> Result<(Uuid, u64)> {
        let job_id = Uuid::new_v4();

        info!("Starting replay job {} from {}", job_id, from_timestamp);

        // Estimate total events
        let estimated_events = self
            .event_storage
            .count_events(
                from_timestamp,
                to_timestamp.clone(),
                event_types.clone(),
                categories.clone(),
            )
            .await?;

        // Create replay job
        let replay_job = ReplayJob {
            id: job_id,
            status: ReplayStatus::Pending,
            from_timestamp,
            to_timestamp,
            event_types,
            categories,
            batch_size,
            total_events: estimated_events,
            processed_events: 0,
            failed_events: 0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        // Store replay job
        {
            let mut jobs = self.replay_jobs.write().await;
            jobs.insert(job_id, replay_job);
        }

        // Start replay task
        let pipeline = self.clone();
        tokio::spawn(async move {
            if let Err(e) = pipeline.execute_replay_job(job_id).await {
                error!("Replay job {} failed: {}", job_id, e);
                {
                    let mut jobs = pipeline.replay_jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id) {
                        job.status = ReplayStatus::Failed;
                        job.error_message = Some(e.to_string());
                        job.completed_at = Some(Utc::now());
                    }
                }
            }
        });

        Ok((job_id, estimated_events))
    }

    /// Get replay job status
    pub async fn get_replay_status(&self, job_id: Uuid) -> Result<Option<serde_json::Value>> {
        let jobs = self.replay_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(Some(serde_json::to_value(job).map_err(|e| {
                crate::error::EventStreamingError::internal(format!(
                    "Failed to serialize job: {}",
                    e
                ))
            })?))
        } else {
            Ok(None)
        }
    }

    /// Get processing statistics
    pub async fn get_processing_stats(&self) -> Result<ProcessingStats> {
        let stats = self.processing_stats.read().await;
        Ok(stats.clone())
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<ComponentHealth> {
        let start_time = Instant::now();
        let health_status = self.health_status.read().await.clone();
        let stats = self.processing_stats.read().await;

        let response_time = start_time.elapsed().as_millis() as u64;

        let details = [
            (
                "worker_threads".to_string(),
                self.config.processing.worker_threads.to_string(),
            ),
            (
                "total_processed".to_string(),
                stats.total_processed.to_string(),
            ),
            ("total_failed".to_string(), stats.total_failed.to_string()),
            (
                "total_filtered".to_string(),
                stats.total_filtered.to_string(),
            ),
        ]
        .into();

        Ok(ComponentHealth {
            component: "processing_pipeline".to_string(),
            status: health_status,
            last_check: chrono::Utc::now(),
            response_time_ms: response_time,
            details,
        })
    }

    /// Start worker tasks
    async fn start_workers(&self) -> Result<()> {
        let mut handles = self.worker_handles.write().await;

        for i in 0..self.config.processing.worker_threads {
            let pipeline = self.clone();
            let worker_id = i;

            let handle = tokio::spawn(async move {
                pipeline.worker_loop(worker_id).await;
            });

            handles.push(handle);
        }

        info!(
            "Started {} worker threads",
            self.config.processing.worker_threads
        );
        Ok(())
    }

    /// Stop worker tasks
    async fn stop_workers(&self) {
        let mut handles = self.worker_handles.write().await;

        for handle in handles.drain(..) {
            handle.abort();
        }

        info!("Stopped all worker threads");
    }

    /// Worker loop for processing events
    async fn worker_loop(&self, worker_id: usize) {
        debug!("Starting worker {}", worker_id);

        let mut shutdown_rx = self.shutdown_tx.read().await.as_ref().unwrap().subscribe();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    debug!("Worker {} received shutdown signal", worker_id);
                    break;
                }
                _ = self.process_batch(worker_id) => {
                    // Continue processing
                }
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        debug!("Worker {} stopped", worker_id);
    }

    /// Process a batch of events
    async fn process_batch(&self, worker_id: usize) {
        // Acquire processing permit
        let _permit = match self.processing_semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                // No permits available, wait a bit
                tokio::time::sleep(Duration::from_millis(100)).await;
                return;
            }
        };

        let batch_size = self.config.processing.batch_size as usize;

        // Try to read from Kafka
        if let Err(e) = self.process_kafka_batch(worker_id, batch_size).await {
            warn!("Worker {} Kafka processing failed: {}", worker_id, e);
        }

        // Try to read from Redis
        if let Err(e) = self.process_redis_batch(worker_id, batch_size).await {
            warn!("Worker {} Redis processing failed: {}", worker_id, e);
        }
    }

    /// Process Kafka batch
    async fn process_kafka_batch(&self, worker_id: usize, _batch_size: usize) -> Result<()> {
        // This is a simplified implementation - in a real system, we'd have
        // proper Kafka consumer integration
        debug!("Worker {} processing Kafka batch", worker_id);
        Ok(())
    }

    /// Process Redis batch
    async fn process_redis_batch(&self, worker_id: usize, _batch_size: usize) -> Result<()> {
        // This is a simplified implementation - in a real system, we'd have
        // proper Redis stream consumer integration
        debug!("Worker {} processing Redis batch", worker_id);
        Ok(())
    }

    /// Process a single event
    async fn process_event(&self, context: ProcessingContext) -> Result<()> {
        let start_time = Instant::now();
        let event_id = context.event.id;

        debug!("Processing event {}", event_id);

        // Apply filters
        if !self.should_process_event(&context.event).await? {
            debug!("Event {} filtered out", event_id);

            // Update stats
            {
                let mut stats = self.processing_stats.write().await;
                stats.total_filtered += 1;
            }

            return Ok(());
        }

        // Transform event if needed
        let transformed_event = self.transform_event(context.event.clone()).await?;

        // Process the event (business logic would go here)
        let processing_result = self.execute_event_processing(transformed_event).await;

        // Update processing stats
        {
            let mut stats = self.processing_stats.write().await;
            match processing_result {
                Ok(_) => {
                    stats.total_processed += 1;
                }
                Err(_) => {
                    stats.total_failed += 1;
                }
            }
        }

        let duration = start_time.elapsed();

        // Record metrics
        self.metrics_collector
            .record_event_processed(event_id, duration, processing_result.is_ok())
            .await?;

        processing_result
    }

    /// Check if event should be processed
    async fn should_process_event(&self, event: &Event) -> Result<bool> {
        for filter in &self.config.processing.filters {
            if !self.apply_filter(filter, event).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Apply a single filter to an event
    async fn apply_filter(
        &self,
        filter: &crate::types::EventFilter,
        event: &Event,
    ) -> Result<bool> {
        // Check categories
        if let Some(categories) = &filter.categories {
            let category_match = categories.contains(&event.category);
            if filter.include != category_match {
                return Ok(false);
            }
        }

        // Check priorities
        if let Some(priorities) = &filter.priorities {
            let priority_match = priorities.contains(&event.priority);
            if filter.include != priority_match {
                return Ok(false);
            }
        }

        // Check sources
        if let Some(sources) = &filter.sources {
            let source_match = sources.contains(&event.source.service);
            if filter.include != source_match {
                return Ok(false);
            }
        }

        // Additional filter logic would go here
        Ok(true)
    }

    /// Transform event using configured transformations
    async fn transform_event(&self, mut event: Event) -> Result<Event> {
        for transformation in &self.config.processing.transformations {
            event = self.apply_transformation(transformation, event).await?;
        }
        Ok(event)
    }

    /// Apply a single transformation to an event
    async fn apply_transformation(
        &self,
        transformation: &crate::types::EventTransformation,
        event: Event,
    ) -> Result<Event> {
        debug!(
            "Applying transformation {} to event {}",
            transformation.name, event.id
        );

        // Transformation logic would be implemented here based on the type
        // For now, just return the event unchanged
        Ok(event)
    }

    /// Execute the actual event processing
    async fn execute_event_processing(&self, event: Event) -> Result<()> {
        debug!("Executing processing for event {}", event.id);

        // Update event status
        let mut event = event;
        event.update_status(
            EventStatus::Processing,
            Some("Processing started".to_string()),
        );

        // Store updated event
        self.event_storage.update_event_status(&event).await?;

        // Simulate processing (in a real system, this would contain business logic)
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Mark as completed
        event.update_status(
            EventStatus::Completed,
            Some("Processing completed".to_string()),
        );
        self.event_storage.update_event_status(&event).await?;

        debug!("Completed processing for event {}", event.id);
        Ok(())
    }

    /// Execute a replay job
    async fn execute_replay_job(&self, job_id: Uuid) -> Result<()> {
        info!("Executing replay job {}", job_id);

        // Update job status
        {
            let mut jobs = self.replay_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = ReplayStatus::Running;
                job.started_at = Some(Utc::now());
            }
        }

        // Get job details
        let (from_timestamp, to_timestamp, event_types, categories, batch_size) = {
            let jobs = self.replay_jobs.read().await;
            let job = jobs.get(&job_id).ok_or_else(|| {
                EventStreamingError::internal(format!("Replay job {} not found", job_id))
            })?;
            (
                job.from_timestamp,
                job.to_timestamp,
                job.event_types.clone(),
                job.categories.clone(),
                job.batch_size,
            )
        };

        // Replay events in batches
        let mut offset = 0;
        loop {
            let events = self
                .event_storage
                .get_events_for_replay(
                    from_timestamp,
                    to_timestamp,
                    event_types.clone(),
                    categories.clone(),
                    batch_size,
                    offset,
                )
                .await?;

            if events.is_empty() {
                break;
            }

            // Process each event
            for event in events.iter() {
                if let Err(e) = self.publish_event(event.clone()).await {
                    warn!("Failed to replay event {}: {}", event.id, e);

                    // Update failed count
                    {
                        let mut jobs = self.replay_jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.failed_events += 1;
                        }
                    }
                } else {
                    // Update processed count
                    {
                        let mut jobs = self.replay_jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.processed_events += 1;
                        }
                    }
                }
            }

            offset += events.len() as u64;

            // Small delay between batches
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Mark job as completed
        {
            let mut jobs = self.replay_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = ReplayStatus::Completed;
                job.completed_at = Some(Utc::now());
            }
        }

        info!("Replay job {} completed", job_id);
        Ok(())
    }

    /// Start metrics collection
    async fn start_metrics_collection(&self) -> Result<()> {
        let pipeline = self.clone();
        let interval = Duration::from_secs(10);

        tokio::spawn(async move {
            let mut shutdown_rx = pipeline
                .shutdown_tx
                .read()
                .await
                .as_ref()
                .unwrap()
                .subscribe();
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = pipeline.collect_metrics().await {
                            warn!("Metrics collection failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Metrics collection received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Collect and record metrics
    async fn collect_metrics(&self) -> Result<()> {
        let stats = self.processing_stats.read().await;

        // Record processing metrics
        self.metrics_collector
            .record_processing_stats(&stats)
            .await?;

        // Record replay job metrics
        let jobs = self.replay_jobs.read().await;
        let active_jobs = jobs
            .values()
            .filter(|job| job.status == ReplayStatus::Running)
            .count();

        self.metrics_collector
            .record_replay_jobs(jobs.len(), active_jobs)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::events::{Event, EventPayload};
    use crate::types::{EventCategory, EventSource};

    #[tokio::test]
    async fn test_processing_pipeline_creation() {
        let config = Config::default();

        // Create mock dependencies
        let metrics = Arc::new(MetricsCollector::new(&config).await.unwrap());
        let kafka = Arc::new(KafkaManager::new(&config, metrics.clone()).await.unwrap());
        let redis = Arc::new(
            RedisStreamManager::new(&config, metrics.clone())
                .await
                .unwrap(),
        );
        let storage = Arc::new(EventStorage::new(&config).await.unwrap());
        let router = Arc::new(EventRouter::new(&config).await.unwrap());

        let result = ProcessingPipeline::new(&config, kafka, redis, storage, router, metrics).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replay_job_creation() {
        let config = Config::default();

        // Create mock dependencies
        let metrics = Arc::new(MetricsCollector::new(&config).await.unwrap());
        let kafka = Arc::new(KafkaManager::new(&config, metrics.clone()).await.unwrap());
        let redis = Arc::new(
            RedisStreamManager::new(&config, metrics.clone())
                .await
                .unwrap(),
        );
        let storage = Arc::new(EventStorage::new(&config).await.unwrap());
        let router = Arc::new(EventRouter::new(&config).await.unwrap());

        let pipeline = ProcessingPipeline::new(&config, kafka, redis, storage, router, metrics)
            .await
            .unwrap();

        let from_timestamp = Utc::now() - chrono::Duration::hours(1);
        let result = pipeline
            .start_replay(from_timestamp, None, None, None, 100)
            .await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_replay_status_serialization() {
        let status = ReplayStatus::Running;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"running\"");

        let deserialized: ReplayStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ReplayStatus::Running);
    }

    #[tokio::test]
    async fn test_event_filtering() {
        let config = Config::default();

        // Create mock dependencies
        let metrics = Arc::new(MetricsCollector::new(&config).await.unwrap());
        let kafka = Arc::new(KafkaManager::new(&config, metrics.clone()).await.unwrap());
        let redis = Arc::new(
            RedisStreamManager::new(&config, metrics.clone())
                .await
                .unwrap(),
        );
        let storage = Arc::new(EventStorage::new(&config).await.unwrap());
        let router = Arc::new(EventRouter::new(&config).await.unwrap());

        let pipeline = ProcessingPipeline::new(&config, kafka, redis, storage, router, metrics)
            .await
            .unwrap();

        // Create test event
        let source = EventSource {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            instance_id: None,
            hostname: None,
            metadata: std::collections::HashMap::new(),
        };

        let payload = EventPayload::Custom(serde_json::json!({"test": "data"}));
        let event = Event::new("test.event", EventCategory::System, source, payload);

        // Test filtering (should pass with default config)
        let should_process = pipeline.should_process_event(&event).await.unwrap();
        assert!(should_process);
    }
}
