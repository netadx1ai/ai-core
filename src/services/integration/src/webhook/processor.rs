//! # Event Processor
//!
//! The event processor is responsible for processing queued webhook events with
//! configurable concurrency limits, timeout handling, and error recovery. It
//! coordinates with registered processors to handle different event types.

use super::{WebhookConfig, WebhookEvent, WebhookProcessor, WebhookResult};
use crate::error::{IntegrationError, IntegrationResult};
// use crate::models::IntegrationEvent;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Event processing statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Total events processed
    pub total_processed: u64,
    /// Events currently being processed
    pub currently_processing: u64,
    /// Events processed successfully
    pub successful_processes: u64,
    /// Events failed processing
    pub failed_processes: u64,
    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,
    /// Peak concurrent processing
    pub peak_concurrent: u64,
    /// Processing rate (events per second)
    pub processing_rate: f64,
    /// Last processing timestamp
    pub last_processed_at: Option<DateTime<Utc>>,
    /// Processor-specific stats
    pub processor_stats: HashMap<String, ProcessorStats>,
}

/// Statistics for individual processors
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessorStats {
    /// Events processed by this processor
    pub events_processed: u64,
    /// Successful processing count
    pub success_count: u64,
    /// Failed processing count
    pub failure_count: u64,
    /// Average processing time
    pub avg_time_ms: f64,
    /// Last processing timestamp
    pub last_processed_at: Option<DateTime<Utc>>,
}

/// Processing task metadata
#[derive(Debug, Clone)]
struct ProcessingTask {
    event: WebhookEvent,
    processor_name: String,
    started_at: Instant,
    timeout: Duration,
}

/// Event processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Maximum concurrent processing tasks
    pub max_concurrent: usize,
    /// Default processing timeout in seconds
    pub default_timeout: u64,
    /// Batch processing size
    pub batch_size: usize,
    /// Processing interval in milliseconds
    pub processing_interval: u64,
    /// Enable retry on transient failures
    pub enable_retry_on_failure: bool,
    /// Maximum memory usage for processing queue (bytes)
    pub max_memory_usage: usize,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker reset timeout in seconds
    pub circuit_breaker_reset_timeout: u64,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 50,
            default_timeout: 30,
            batch_size: 10,
            processing_interval: 100,
            enable_retry_on_failure: true,
            max_memory_usage: 100 * 1024 * 1024, // 100MB
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_timeout: 60,
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for processor reliability
#[derive(Debug)]
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    last_failure_time: RwLock<Option<Instant>>,
    threshold: u32,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            last_failure_time: RwLock::new(None),
            threshold,
            reset_timeout,
        }
    }

    pub fn can_execute(&self) -> bool {
        let state = *self.state.read();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = *self.last_failure_time.read() {
                    if last_failure.elapsed() >= self.reset_timeout {
                        // Transition to half-open
                        *self.state.write() = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn on_success(&self) {
        self.success_count.fetch_add(1, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);

        let state = *self.state.read();
        if state == CircuitState::HalfOpen {
            *self.state.write() = CircuitState::Closed;
        }
    }

    pub fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.write() = Some(Instant::now());

        if failures >= self.threshold as u64 {
            *self.state.write() = CircuitState::Open;
        }
    }

    pub fn get_state(&self) -> CircuitState {
        *self.state.read()
    }
}

/// Trait for providing events to process
#[async_trait]
pub trait EventProvider: Send + Sync {
    /// Get next batch of events to process
    async fn get_events(&self, batch_size: usize) -> WebhookResult<Vec<WebhookEvent>>;

    /// Check if more events are available
    async fn has_events(&self) -> bool;
}

/// Default event provider using collector
pub struct CollectorEventProvider {
    collector: Arc<super::collector::WebhookCollector>,
}

impl CollectorEventProvider {
    pub fn new(collector: Arc<super::collector::WebhookCollector>) -> Self {
        Self { collector }
    }
}

#[async_trait]
impl EventProvider for CollectorEventProvider {
    async fn get_events(&self, batch_size: usize) -> WebhookResult<Vec<WebhookEvent>> {
        self.collector.get_events(batch_size).await
    }

    async fn has_events(&self) -> bool {
        !self.collector.is_queue_empty().await
    }
}

/// Main event processor
pub struct EventProcessor {
    config: WebhookConfig,
    processor_config: ProcessorConfig,
    processors: Arc<RwLock<HashMap<String, Arc<dyn WebhookProcessor>>>>,
    circuit_breakers: Arc<DashMap<String, Arc<CircuitBreaker>>>,
    event_provider: Option<Arc<dyn EventProvider>>,
    stats: Arc<RwLock<ProcessingStats>>,
    concurrency_limiter: Arc<Semaphore>,
    running: Arc<AtomicBool>,
    active_tasks: Arc<DashMap<Uuid, ProcessingTask>>,
    processed_count: Arc<AtomicU64>,
    last_rate_reset: Arc<RwLock<Instant>>,
    task_sender: mpsc::UnboundedSender<ProcessingTask>,
    task_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ProcessingTask>>>>,
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(config: WebhookConfig) -> Self {
        let processor_config = ProcessorConfig::default();
        let concurrency_limiter = Arc::new(Semaphore::new(processor_config.max_concurrent));

        let (task_sender, task_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            processor_config,
            processors: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(DashMap::new()),
            event_provider: None,
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
            concurrency_limiter,
            running: Arc::new(AtomicBool::new(false)),
            active_tasks: Arc::new(DashMap::new()),
            processed_count: Arc::new(AtomicU64::new(0)),
            last_rate_reset: Arc::new(RwLock::new(Instant::now())),
            task_sender,
            task_receiver: Arc::new(RwLock::new(Some(task_receiver))),
        }
    }

    /// Set the event provider
    pub fn set_event_provider(&mut self, provider: Arc<dyn EventProvider>) {
        self.event_provider = Some(provider);
    }

    /// Register a webhook processor
    pub async fn register_processor(&self, processor: Arc<dyn WebhookProcessor>) {
        let name = processor.name().to_string();

        // Create circuit breaker for this processor
        let circuit_breaker = Arc::new(CircuitBreaker::new(
            self.processor_config.circuit_breaker_threshold,
            Duration::from_secs(self.processor_config.circuit_breaker_reset_timeout),
        ));
        self.circuit_breakers.insert(name.clone(), circuit_breaker);

        // Register processor
        let mut processors = self.processors.write();
        processors.insert(name.clone(), processor);

        // Initialize stats
        let mut stats = self.stats.write();
        stats
            .processor_stats
            .insert(name.clone(), ProcessorStats::default());

        info!("Registered webhook processor: {}", name);
    }

    /// Start the processor
    pub async fn start(&self) -> IntegrationResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        info!("Starting event processor");

        // Start main processing loop
        let processor = self.clone();
        tokio::spawn(async move {
            if let Err(e) = processor.run().await {
                error!("Event processor error: {}", e);
            }
        });

        // Start task processing loop
        let processor = self.clone();
        tokio::spawn(async move {
            if let Err(e) = processor.task_processing_loop().await {
                error!("Task processing loop error: {}", e);
            }
        });

        // Start stats update task
        let processor = self.clone();
        tokio::spawn(async move {
            processor.stats_update_task().await;
        });

        Ok(())
    }

    /// Stop the processor
    pub async fn stop(&self) -> IntegrationResult<()> {
        info!("Stopping event processor");
        self.running.store(false, Ordering::SeqCst);

        // Wait for active tasks to complete
        let timeout_duration = Duration::from_secs(30);
        let start_time = Instant::now();

        while !self.active_tasks.is_empty() && start_time.elapsed() < timeout_duration {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        if !self.active_tasks.is_empty() {
            warn!(
                "Stopping processor with {} active tasks",
                self.active_tasks.len()
            );
        }

        Ok(())
    }

    /// Process a single batch of events
    pub async fn process_batch(&self) -> IntegrationResult<()> {
        if let Some(provider) = &self.event_provider {
            let events = provider
                .get_events(self.processor_config.batch_size)
                .await
                .map_err(|e| IntegrationError::from(e))?;

            for event in events {
                self.process_event(event).await?;
            }
        }

        Ok(())
    }

    /// Process a single event
    #[instrument(skip(self, event), fields(event_id = %event.id))]
    async fn process_event(&self, mut event: WebhookEvent) -> IntegrationResult<()> {
        // Find appropriate processor
        let processor = self.find_processor_for_event(&event).await?;
        let processor_name = processor.name().to_string();

        // Check circuit breaker
        let circuit_breaker = self.circuit_breakers.get(&processor_name).ok_or_else(|| {
            IntegrationError::internal(format!(
                "No circuit breaker for processor: {}",
                processor_name
            ))
        })?;

        if !circuit_breaker.can_execute() {
            warn!(
                processor = %processor_name,
                event_id = %event.id,
                "Circuit breaker open, skipping event"
            );
            return Err(IntegrationError::service_unavailable("circuit breaker"));
        }

        // Mark event as processing
        event.mark_processing();

        // Create processing task
        let task = ProcessingTask {
            event: event.clone(),
            processor_name: processor_name.clone(),
            started_at: Instant::now(),
            timeout: processor.timeout(),
        };

        // Add to active tasks
        self.active_tasks.insert(event.id, task.clone());

        // Send task for processing
        if let Err(_) = self.task_sender.send(task) {
            self.active_tasks.remove(&event.id);
            return Err(IntegrationError::internal("Task sender closed"));
        }

        Ok(())
    }

    /// Find appropriate processor for event
    async fn find_processor_for_event(
        &self,
        event: &WebhookEvent,
    ) -> IntegrationResult<Arc<dyn WebhookProcessor>> {
        let processors = self.processors.read();

        for processor in processors.values() {
            if processor.can_handle(event) {
                return Ok(Arc::clone(processor));
            }
        }

        Err(IntegrationError::webhook_processing(format!(
            "No processor found for event type: {}",
            event.payload.event_type
        )))
    }

    /// Clone the processor (for background tasks)
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            processor_config: self.processor_config.clone(),
            processors: Arc::clone(&self.processors),
            circuit_breakers: Arc::clone(&self.circuit_breakers),
            event_provider: self.event_provider.as_ref().map(Arc::clone),
            stats: Arc::clone(&self.stats),
            concurrency_limiter: Arc::clone(&self.concurrency_limiter),
            running: Arc::clone(&self.running),
            active_tasks: Arc::clone(&self.active_tasks),
            processed_count: Arc::clone(&self.processed_count),
            last_rate_reset: Arc::clone(&self.last_rate_reset),
            task_sender: self.task_sender.clone(),
            task_receiver: Arc::clone(&self.task_receiver),
        }
    }

    /// Main processing loop
    async fn run(&self) -> IntegrationResult<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(
            self.processor_config.processing_interval,
        ));

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            if let Some(provider) = &self.event_provider {
                if provider.has_events().await {
                    if let Err(e) = self.process_batch().await {
                        error!("Batch processing error: {}", e);
                    }
                }
            }
        }

        info!("Event processor main loop stopped");
        Ok(())
    }

    /// Task processing loop
    async fn task_processing_loop(&self) -> IntegrationResult<()> {
        let mut receiver = self
            .task_receiver
            .write()
            .take()
            .ok_or_else(|| IntegrationError::internal("Task receiver already taken"))?;

        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                task = receiver.recv() => {
                    match task {
                        Some(task) => {
                            let processor = self.clone();
                            tokio::spawn(async move {
                                processor.execute_task(task).await;
                            });
                        }
                        None => break, // Channel closed
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic maintenance - check for timed out tasks
                    self.cleanup_timed_out_tasks().await;
                }
            }
        }

        info!("Task processing loop stopped");
        Ok(())
    }

    /// Execute a processing task
    async fn execute_task(&self, task: ProcessingTask) {
        let event_id = task.event.id;
        let processor_name = task.processor_name.clone();

        // Acquire concurrency permit
        let _permit = match self.concurrency_limiter.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                error!("Failed to acquire concurrency permit");
                self.active_tasks.remove(&event_id);
                return;
            }
        };

        // Update current processing count
        {
            let mut stats = self.stats.write();
            stats.currently_processing += 1;
            if stats.currently_processing > stats.peak_concurrent {
                stats.peak_concurrent = stats.currently_processing;
            }
        }

        let start_time = Instant::now();
        let result = {
            let processor = {
                let processors = self.processors.read();
                processors.get(&processor_name).cloned()
            };

            if let Some(processor) = processor {
                timeout(task.timeout, processor.process_event(&task.event)).await
            } else {
                Ok(Err(IntegrationError::webhook_processing(
                    "No processor available",
                )))
            }
        };

        let processing_time = start_time.elapsed();

        // Remove from active tasks
        self.active_tasks.remove(&event_id);

        // Update statistics
        self.update_processing_stats(&processor_name, processing_time, result.is_ok())
            .await;

        // Handle result
        match result {
            Ok(Ok(_integration_event)) => {
                debug!(
                    event_id = %event_id,
                    processor = %processor_name,
                    processing_time_ms = processing_time.as_millis(),
                    "Event processed successfully"
                );

                // Circuit breaker success
                if let Some(cb) = self.circuit_breakers.get(&processor_name) {
                    cb.on_success();
                }
            }
            Ok(Err(e)) => {
                error!(
                    event_id = %event_id,
                    processor = %processor_name,
                    error = %e,
                    "Event processing failed"
                );

                // Circuit breaker failure
                if let Some(cb) = self.circuit_breakers.get(&processor_name) {
                    cb.on_failure();
                }
            }
            Err(_) => {
                error!(
                    event_id = %event_id,
                    processor = %processor_name,
                    "Event processing timed out"
                );

                // Circuit breaker failure for timeout
                if let Some(cb) = self.circuit_breakers.get(&processor_name) {
                    cb.on_failure();
                }
            }
        }

        // Update current processing count
        {
            let mut stats = self.stats.write();
            stats.currently_processing = stats.currently_processing.saturating_sub(1);
        }

        self.processed_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Update processing statistics
    async fn update_processing_stats(
        &self,
        processor_name: &str,
        processing_time: Duration,
        success: bool,
    ) {
        let mut stats = self.stats.write();

        // Update global stats
        stats.total_processed += 1;
        if success {
            stats.successful_processes += 1;
        } else {
            stats.failed_processes += 1;
        }

        stats.avg_processing_time_ms = (stats.avg_processing_time_ms
            * (stats.total_processed - 1) as f64
            + processing_time.as_millis() as f64)
            / stats.total_processed as f64;

        stats.last_processed_at = Some(Utc::now());

        // Update processor-specific stats
        let processor_stats = stats
            .processor_stats
            .entry(processor_name.to_string())
            .or_insert_with(ProcessorStats::default);

        processor_stats.events_processed += 1;
        if success {
            processor_stats.success_count += 1;
        } else {
            processor_stats.failure_count += 1;
        }

        processor_stats.avg_time_ms = (processor_stats.avg_time_ms
            * (processor_stats.events_processed - 1) as f64
            + processing_time.as_millis() as f64)
            / processor_stats.events_processed as f64;

        processor_stats.last_processed_at = Some(Utc::now());
    }

    /// Stats update background task
    async fn stats_update_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            // Calculate processing rate
            let now = Instant::now();
            let last_reset = *self.last_rate_reset.read();
            let time_elapsed = now.duration_since(last_reset).as_secs_f64();
            let count = self.processed_count.load(Ordering::SeqCst) as f64;

            if time_elapsed > 0.0 {
                let mut stats = self.stats.write();
                stats.processing_rate = count / time_elapsed;
            }

            // Reset counters
            self.processed_count.store(0, Ordering::SeqCst);
            *self.last_rate_reset.write() = now;
        }
    }

    /// Clean up timed out tasks
    async fn cleanup_timed_out_tasks(&self) {
        let now = Instant::now();
        let mut timed_out_tasks = Vec::new();

        // Find timed out tasks
        for entry in self.active_tasks.iter() {
            let task = entry.value();
            if now.duration_since(task.started_at) > task.timeout {
                timed_out_tasks.push(task.event.id);
            }
        }

        // Remove timed out tasks
        for task_id in timed_out_tasks {
            if let Some((_, task)) = self.active_tasks.remove(&task_id) {
                warn!(
                    event_id = %task_id,
                    processor = %task.processor_name,
                    "Cleaning up timed out task"
                );
            }
        }
    }

    /// Get processing statistics
    pub async fn get_stats(&self) -> ProcessingStats {
        self.stats.read().clone()
    }

    /// Get circuit breaker states
    pub async fn get_circuit_breaker_states(&self) -> HashMap<String, CircuitState> {
        let mut states = HashMap::new();
        for entry in self.circuit_breakers.iter() {
            states.insert(entry.key().clone(), entry.get_state());
        }
        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        EventMetadata, EventPayload, EventStatus, IntegrationEvent, IntegrationType,
        WebhookPayload, ZapierEvent,
    };
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use uuid::Uuid;

    // Mock processor for testing
    struct MockProcessor {
        name: String,
        process_count: Arc<AtomicU32>,
        should_fail: bool,
        processing_delay: Duration,
    }

    impl MockProcessor {
        fn new(name: &str, should_fail: bool, delay: Duration) -> Self {
            Self {
                name: name.to_string(),
                process_count: Arc::new(AtomicU32::new(0)),
                should_fail,
                processing_delay: delay,
            }
        }

        fn get_process_count(&self) -> u32 {
            self.process_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl WebhookProcessor for MockProcessor {
        async fn process_event(
            &self,
            _event: &WebhookEvent,
        ) -> IntegrationResult<IntegrationEvent> {
            self.process_count.fetch_add(1, Ordering::SeqCst);

            if self.processing_delay > Duration::from_millis(0) {
                tokio::time::sleep(self.processing_delay).await;
            }

            if self.should_fail {
                Err(IntegrationError::webhook_processing("Mock failure"))
            } else {
                Ok(IntegrationEvent {
                    id: Uuid::new_v4(),
                    integration: IntegrationType::Zapier,
                    event_type: "test".to_string(),
                    metadata: EventMetadata {
                        source_id: "test".to_string(),
                        user_id: None,
                        organization_id: None,
                        request_id: "test".to_string(),
                        tags: HashMap::new(),
                    },
                    payload: EventPayload::Zapier(ZapierEvent {
                        zap_id: "test".to_string(),
                        zap_name: Some("test".to_string()),
                        event_name: "test".to_string(),
                        trigger_data: json!({}),
                        custom_fields: HashMap::new(),
                        step_info: None,
                    }),
                    status: EventStatus::Completed,
                    error_message: None,
                    retry_count: 0,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn can_handle(&self, _event: &WebhookEvent) -> bool {
            true
        }

        fn timeout(&self) -> Duration {
            Duration::from_secs(5)
        }
    }

    fn create_test_event() -> WebhookEvent {
        use super::super::EventPriority;

        let payload = WebhookPayload {
            id: Uuid::new_v4(),
            integration: "test".to_string(),
            event_type: "test.event".to_string(),
            timestamp: Utc::now(),
            data: json!({"test": "data"}),
            headers: HashMap::new(),
            source_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
        };
        WebhookEvent::new(payload, EventPriority::Normal)
    }

    #[test]
    fn test_circuit_breaker_states() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(5));

        // Initially closed
        assert_eq!(breaker.get_state(), CircuitState::Closed);
        assert!(breaker.can_execute());

        // Trigger failures
        breaker.on_failure();
        breaker.on_failure();
        assert_eq!(breaker.get_state(), CircuitState::Closed);

        breaker.on_failure(); // Third failure should open circuit
        assert_eq!(breaker.get_state(), CircuitState::Open);
        assert!(!breaker.can_execute());

        // Success should close if half-open
        *breaker.state.write() = CircuitState::HalfOpen;
        breaker.on_success();
        assert_eq!(breaker.get_state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_processor_registration() {
        let config = WebhookConfig::default();
        let processor = EventProcessor::new(config);

        let mock_processor = Arc::new(MockProcessor::new("test", false, Duration::from_millis(0)));
        processor
            .register_processor(mock_processor.clone() as Arc<dyn WebhookProcessor>)
            .await;

        let stats = processor.get_stats().await;
        assert!(stats.processor_stats.contains_key("test"));
    }

    #[tokio::test]
    async fn test_processor_lifecycle() {
        let config = WebhookConfig::default();
        let processor = EventProcessor::new(config);

        // Start processor
        processor.start().await.unwrap();
        assert!(processor.running.load(Ordering::SeqCst));

        // Stop processor
        processor.stop().await.unwrap();
        assert!(!processor.running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_event_processing_success() {
        let config = WebhookConfig::default();
        let processor = EventProcessor::new(config);

        let mock_processor = Arc::new(MockProcessor::new("test", false, Duration::from_millis(0)));
        processor
            .register_processor(mock_processor.clone() as Arc<dyn WebhookProcessor>)
            .await;
        processor.start().await.unwrap();

        // Process an event
        let event = create_test_event();
        processor.process_event(event).await.unwrap();

        // Give some time for async processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check that processor was called
        assert_eq!(mock_processor.get_process_count(), 1);

        let stats = processor.get_stats().await;
        assert!(stats.total_processed > 0);

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        let config = WebhookConfig::default();
        let processor = EventProcessor::new(config);

        // Register failing processor
        let mock_processor = Arc::new(MockProcessor::new(
            "failing",
            true,
            Duration::from_millis(0),
        ));
        processor
            .register_processor(mock_processor as Arc<dyn WebhookProcessor>)
            .await;
        processor.start().await.unwrap();

        // Process multiple events to trigger circuit breaker
        for _ in 0..10 {
            let event = create_test_event();
            let _ = processor.process_event(event).await;
        }

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        let circuit_states = processor.get_circuit_breaker_states().await;
        assert!(circuit_states.contains_key("failing"));

        processor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_processing_stats_tracking() {
        let config = WebhookConfig::default();
        let processor = EventProcessor::new(config);

        let mock_processor = Arc::new(MockProcessor::new(
            "stats_test",
            false,
            Duration::from_millis(5),
        ));
        processor
            .register_processor(mock_processor as Arc<dyn WebhookProcessor>)
            .await;
        processor.start().await.unwrap();

        // Process multiple events
        for _ in 0..3 {
            let event = create_test_event();
            processor.process_event(event).await.unwrap();
        }

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = processor.get_stats().await;
        assert!(stats.total_processed >= 3);
        assert!(stats.avg_processing_time_ms > 0.0);
        assert!(stats.processor_stats.contains_key("stats_test"));

        let processor_stats = &stats.processor_stats["stats_test"];
        assert!(processor_stats.events_processed >= 3);

        processor.stop().await.unwrap();
    }
}
