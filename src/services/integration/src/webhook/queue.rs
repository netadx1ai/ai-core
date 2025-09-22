//! # Dead Letter Queue
//!
//! The dead letter queue handles webhook events that have permanently failed processing
//! after exhausting all retry attempts. It provides storage, analysis, and replay
//! capabilities for failed events to enable debugging and recovery.

use super::{WebhookConfig, WebhookError, WebhookEvent, WebhookEventStatus, WebhookResult};
use crate::error::{IntegrationError, IntegrationResult};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Dead letter queue entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    /// Original webhook event
    pub event: WebhookEvent,
    /// Reason for dead lettering
    pub reason: String,
    /// Failure analysis
    pub failure_analysis: FailureAnalysis,
    /// Timestamp when added to DLQ
    pub dead_lettered_at: DateTime<Utc>,
    /// Number of replay attempts
    pub replay_attempts: u32,
    /// Maximum replay attempts allowed
    pub max_replay_attempts: u32,
    /// Last replay attempt timestamp
    pub last_replay_at: Option<DateTime<Utc>>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DeadLetterEntry {
    /// Create a new dead letter entry
    pub fn new(event: WebhookEvent, reason: String) -> Self {
        let failure_analysis = FailureAnalysis::analyze(&event, &reason);

        Self {
            event,
            reason,
            failure_analysis,
            dead_lettered_at: Utc::now(),
            replay_attempts: 0,
            max_replay_attempts: 3,
            last_replay_at: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Check if entry can be replayed
    pub fn can_replay(&self) -> bool {
        self.replay_attempts < self.max_replay_attempts
    }

    /// Mark as replayed
    pub fn mark_replayed(&mut self) {
        self.replay_attempts += 1;
        self.last_replay_at = Some(Utc::now());
    }

    /// Add tag for categorization
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Get age in hours
    pub fn age_hours(&self) -> i64 {
        let now = Utc::now();
        (now - self.dead_lettered_at).num_hours()
    }
}

/// Failure analysis information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAnalysis {
    /// Failure category
    pub category: FailureCategory,
    /// Severity level
    pub severity: FailureSeverity,
    /// Root cause analysis
    pub root_cause: String,
    /// Suggested remediation
    pub remediation: Vec<String>,
    /// Error patterns detected
    pub error_patterns: Vec<String>,
    /// Affected components
    pub affected_components: Vec<String>,
}

impl FailureAnalysis {
    /// Analyze a failed event to determine failure characteristics
    pub fn analyze(event: &WebhookEvent, reason: &str) -> Self {
        let category = Self::categorize_failure(reason);
        let severity = Self::assess_severity(event, reason);
        let root_cause = Self::determine_root_cause(reason);
        let remediation = Self::suggest_remediation(&category, reason);
        let error_patterns = Self::extract_error_patterns(reason);
        let affected_components = Self::identify_affected_components(event, reason);

        Self {
            category,
            severity,
            root_cause,
            remediation,
            error_patterns,
            affected_components,
        }
    }

    fn categorize_failure(reason: &str) -> FailureCategory {
        let reason_lower = reason.to_lowercase();

        if reason_lower.contains("timeout") {
            FailureCategory::Timeout
        } else if reason_lower.contains("network") || reason_lower.contains("connection") {
            FailureCategory::Network
        } else if reason_lower.contains("authentication") || reason_lower.contains("unauthorized") {
            FailureCategory::Authentication
        } else if reason_lower.contains("validation") || reason_lower.contains("invalid") {
            FailureCategory::Validation
        } else if reason_lower.contains("rate") || reason_lower.contains("throttle") {
            FailureCategory::RateLimit
        } else if reason_lower.contains("configuration") || reason_lower.contains("config") {
            FailureCategory::Configuration
        } else if reason_lower.contains("external") || reason_lower.contains("downstream") {
            FailureCategory::External
        } else {
            FailureCategory::Unknown
        }
    }

    fn assess_severity(_event: &WebhookEvent, reason: &str) -> FailureSeverity {
        let reason_lower = reason.to_lowercase();

        if reason_lower.contains("critical") || reason_lower.contains("fatal") {
            FailureSeverity::Critical
        } else if reason_lower.contains("error") || reason_lower.contains("fail") {
            FailureSeverity::High
        } else if reason_lower.contains("warn") || reason_lower.contains("timeout") {
            FailureSeverity::Medium
        } else {
            FailureSeverity::Low
        }
    }

    fn determine_root_cause(reason: &str) -> String {
        // Simple root cause extraction - could be enhanced with ML
        if reason.contains("timeout") {
            "Processing timeout exceeded configured limits".to_string()
        } else if reason.contains("network") {
            "Network connectivity issues prevented processing".to_string()
        } else if reason.contains("authentication") {
            "Authentication or authorization failure".to_string()
        } else if reason.contains("validation") {
            "Data validation failed due to invalid input".to_string()
        } else {
            format!("Processing failed: {}", reason)
        }
    }

    fn suggest_remediation(category: &FailureCategory, _reason: &str) -> Vec<String> {
        match category {
            FailureCategory::Timeout => vec![
                "Increase processing timeout configuration".to_string(),
                "Optimize event processing logic".to_string(),
                "Check system resource utilization".to_string(),
            ],
            FailureCategory::Network => vec![
                "Check network connectivity".to_string(),
                "Verify firewall and security group settings".to_string(),
                "Consider implementing circuit breaker pattern".to_string(),
            ],
            FailureCategory::Authentication => vec![
                "Verify API keys and credentials".to_string(),
                "Check token expiration and refresh logic".to_string(),
                "Review authentication configuration".to_string(),
            ],
            FailureCategory::Validation => vec![
                "Review webhook payload schema".to_string(),
                "Validate input data format".to_string(),
                "Check required field mappings".to_string(),
            ],
            FailureCategory::RateLimit => vec![
                "Implement backoff and retry logic".to_string(),
                "Consider rate limiting configuration".to_string(),
                "Distribute load across time periods".to_string(),
            ],
            FailureCategory::Configuration => vec![
                "Review service configuration".to_string(),
                "Validate environment variables".to_string(),
                "Check integration settings".to_string(),
            ],
            FailureCategory::External => vec![
                "Check external service availability".to_string(),
                "Verify API endpoints and versions".to_string(),
                "Implement graceful degradation".to_string(),
            ],
            FailureCategory::Unknown => vec![
                "Enable detailed error logging".to_string(),
                "Review application logs for patterns".to_string(),
                "Consider manual investigation".to_string(),
            ],
        }
    }

    fn extract_error_patterns(reason: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Extract common error patterns
        if reason.contains("HTTP") {
            patterns.push("HTTP_ERROR".to_string());
        }
        if reason.contains("JSON") {
            patterns.push("JSON_ERROR".to_string());
        }
        if reason.contains("timeout") {
            patterns.push("TIMEOUT_ERROR".to_string());
        }
        if reason.contains("5")
            && (reason.contains("0") || reason.contains("1") || reason.contains("2"))
        {
            patterns.push("SERVER_ERROR".to_string());
        }

        patterns
    }

    fn identify_affected_components(_event: &WebhookEvent, reason: &str) -> Vec<String> {
        let mut components = Vec::new();

        // Identify affected components based on error
        if reason.contains("database") {
            components.push("database".to_string());
        }
        if reason.contains("cache") || reason.contains("redis") {
            components.push("cache".to_string());
        }
        if reason.contains("network") {
            components.push("network".to_string());
        }
        if reason.contains("external") || reason.contains("api") {
            components.push("external_api".to_string());
        }

        components
    }
}

/// Failure category enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCategory {
    Timeout,
    Network,
    Authentication,
    Validation,
    RateLimit,
    Configuration,
    External,
    Unknown,
}

/// Failure severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FailureSeverity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Dead letter queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeadLetterStats {
    /// Total events in dead letter queue
    pub total_entries: u64,
    /// Events added in last 24 hours
    pub entries_last_24h: u64,
    /// Total replay attempts
    pub total_replays: u64,
    /// Successful replays
    pub successful_replays: u64,
    /// Failed replays
    pub failed_replays: u64,
    /// Average age of entries in hours
    pub avg_age_hours: f64,
    /// Entries by failure category
    pub category_breakdown: HashMap<String, u64>,
    /// Entries by severity
    pub severity_breakdown: HashMap<String, u64>,
    /// Top error patterns
    pub top_error_patterns: Vec<(String, u64)>,
    /// Last analysis timestamp
    pub last_analyzed_at: Option<DateTime<Utc>>,
}

/// Trait for dead letter queue storage
#[async_trait]
pub trait DeadLetterStorage: Send + Sync {
    /// Store dead letter entry
    async fn store_entry(&self, entry: DeadLetterEntry) -> WebhookResult<()>;

    /// Get entry by ID
    async fn get_entry(&self, id: Uuid) -> WebhookResult<Option<DeadLetterEntry>>;

    /// Update entry
    async fn update_entry(&self, entry: &DeadLetterEntry) -> WebhookResult<()>;

    /// Remove entry
    async fn remove_entry(&self, id: Uuid) -> WebhookResult<()>;

    /// Get entries with pagination
    async fn get_entries(&self, limit: usize, offset: usize)
        -> WebhookResult<Vec<DeadLetterEntry>>;

    /// Get entries by category
    async fn get_entries_by_category(
        &self,
        category: FailureCategory,
        limit: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>>;

    /// Get entries by severity
    async fn get_entries_by_severity(
        &self,
        severity: FailureSeverity,
        limit: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>>;

    /// Search entries by tags
    async fn search_by_tags(
        &self,
        tags: &[String],
        limit: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>>;

    /// Get statistics
    async fn get_stats(&self) -> WebhookResult<DeadLetterStats>;

    /// Clean up old entries
    async fn cleanup_old_entries(&self, retention_hours: u64) -> WebhookResult<u64>;
}

/// In-memory dead letter queue storage
pub struct MemoryDeadLetterStorage {
    entries: Arc<RwLock<HashMap<Uuid, DeadLetterEntry>>>,
    stats: Arc<RwLock<DeadLetterStats>>,
    max_size: usize,
}

impl MemoryDeadLetterStorage {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::with_capacity(max_size))),
            stats: Arc::new(RwLock::new(DeadLetterStats::default())),
            max_size,
        }
    }

    fn update_stats(&self) {
        let entries = self.entries.read();
        let mut stats = self.stats.write();

        stats.total_entries = entries.len() as u64;

        let now = Utc::now();
        let last_24h = now - ChronoDuration::hours(24);

        stats.entries_last_24h = entries
            .values()
            .filter(|entry| entry.dead_lettered_at >= last_24h)
            .count() as u64;

        // Calculate average age
        if !entries.is_empty() {
            let total_age: i64 = entries.values().map(|entry| entry.age_hours()).sum();
            stats.avg_age_hours = total_age as f64 / entries.len() as f64;
        }

        // Category breakdown
        stats.category_breakdown.clear();
        for entry in entries.values() {
            let category = format!("{:?}", entry.failure_analysis.category);
            *stats.category_breakdown.entry(category).or_insert(0) += 1;
        }

        // Severity breakdown
        stats.severity_breakdown.clear();
        for entry in entries.values() {
            let severity = format!("{:?}", entry.failure_analysis.severity);
            *stats.severity_breakdown.entry(severity).or_insert(0) += 1;
        }

        // Top error patterns
        let mut pattern_counts: HashMap<String, u64> = HashMap::new();
        for entry in entries.values() {
            for pattern in &entry.failure_analysis.error_patterns {
                *pattern_counts.entry(pattern.clone()).or_insert(0) += 1;
            }
        }

        let mut patterns: Vec<_> = pattern_counts.into_iter().collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        stats.top_error_patterns = patterns.into_iter().take(10).collect();

        stats.last_analyzed_at = Some(now);
    }
}

#[async_trait]
impl DeadLetterStorage for MemoryDeadLetterStorage {
    async fn store_entry(&self, entry: DeadLetterEntry) -> WebhookResult<()> {
        let mut entries = self.entries.write();

        if entries.len() >= self.max_size {
            return Err(WebhookError::QueueFull);
        }

        entries.insert(entry.event.id, entry);
        drop(entries);

        self.update_stats();
        Ok(())
    }

    async fn get_entry(&self, id: Uuid) -> WebhookResult<Option<DeadLetterEntry>> {
        let entries = self.entries.read();
        Ok(entries.get(&id).cloned())
    }

    async fn update_entry(&self, entry: &DeadLetterEntry) -> WebhookResult<()> {
        let mut entries = self.entries.write();
        entries.insert(entry.event.id, entry.clone());
        drop(entries);

        self.update_stats();
        Ok(())
    }

    async fn remove_entry(&self, id: Uuid) -> WebhookResult<()> {
        let mut entries = self.entries.write();
        entries.remove(&id);
        drop(entries);

        self.update_stats();
        Ok(())
    }

    async fn get_entries(
        &self,
        limit: usize,
        offset: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>> {
        let entries = self.entries.read();
        let mut sorted_entries: Vec<_> = entries.values().cloned().collect();

        // Sort by dead lettered timestamp (newest first)
        sorted_entries.sort_by(|a, b| b.dead_lettered_at.cmp(&a.dead_lettered_at));

        let result = sorted_entries
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(result)
    }

    async fn get_entries_by_category(
        &self,
        category: FailureCategory,
        limit: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>> {
        let entries = self.entries.read();
        let filtered: Vec<_> = entries
            .values()
            .filter(|entry| entry.failure_analysis.category == category)
            .cloned()
            .take(limit)
            .collect();

        Ok(filtered)
    }

    async fn get_entries_by_severity(
        &self,
        severity: FailureSeverity,
        limit: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>> {
        let entries = self.entries.read();
        let filtered: Vec<_> = entries
            .values()
            .filter(|entry| entry.failure_analysis.severity == severity)
            .cloned()
            .take(limit)
            .collect();

        Ok(filtered)
    }

    async fn search_by_tags(
        &self,
        tags: &[String],
        limit: usize,
    ) -> WebhookResult<Vec<DeadLetterEntry>> {
        let entries = self.entries.read();
        let filtered: Vec<_> = entries
            .values()
            .filter(|entry| tags.iter().any(|tag| entry.tags.contains(tag)))
            .cloned()
            .take(limit)
            .collect();

        Ok(filtered)
    }

    async fn get_stats(&self) -> WebhookResult<DeadLetterStats> {
        Ok(self.stats.read().clone())
    }

    async fn cleanup_old_entries(&self, retention_hours: u64) -> WebhookResult<u64> {
        let cutoff_time = Utc::now() - ChronoDuration::hours(retention_hours as i64);
        let mut entries = self.entries.write();
        let initial_count = entries.len();

        entries.retain(|_, entry| entry.dead_lettered_at > cutoff_time);

        let removed_count = (initial_count - entries.len()) as u64;
        drop(entries);

        if removed_count > 0 {
            self.update_stats();
        }

        Ok(removed_count)
    }
}

/// Dead letter queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterConfig {
    /// Maximum entries in dead letter queue
    pub max_entries: usize,
    /// Retention period in hours
    pub retention_hours: u64,
    /// Enable automatic analysis
    pub enable_analysis: bool,
    /// Analysis interval in seconds
    pub analysis_interval: u64,
    /// Enable replay functionality
    pub enable_replay: bool,
    /// Maximum replay attempts per entry
    pub max_replay_attempts: u32,
    /// Cleanup interval in seconds
    pub cleanup_interval: u64,
}

impl Default for DeadLetterConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            retention_hours: 168, // 7 days
            enable_analysis: true,
            analysis_interval: 3600, // 1 hour
            enable_replay: true,
            max_replay_attempts: 3,
            cleanup_interval: 21600, // 6 hours
        }
    }
}

/// Main dead letter queue
pub struct DeadLetterQueue {
    config: WebhookConfig,
    dlq_config: DeadLetterConfig,
    storage: Arc<dyn DeadLetterStorage>,
    stats: Arc<RwLock<DeadLetterStats>>,
    running: Arc<AtomicBool>,
    processed_count: Arc<AtomicU64>,
    entry_sender: mpsc::UnboundedSender<DeadLetterEntry>,
    entry_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<DeadLetterEntry>>>>,
}

impl DeadLetterQueue {
    /// Create a new dead letter queue
    pub fn new(config: WebhookConfig) -> Self {
        let dlq_config = DeadLetterConfig::default();
        let storage = Arc::new(MemoryDeadLetterStorage::new(dlq_config.max_entries));

        let (entry_sender, entry_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            dlq_config,
            storage,
            stats: Arc::new(RwLock::new(DeadLetterStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            processed_count: Arc::new(AtomicU64::new(0)),
            entry_sender,
            entry_receiver: Arc::new(RwLock::new(Some(entry_receiver))),
        }
    }

    /// Start the dead letter queue
    pub async fn start(&self) -> IntegrationResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        info!("Starting dead letter queue");

        // Start entry processing loop
        let queue = self.clone();
        tokio::spawn(async move {
            if let Err(e) = queue.entry_processing_loop().await {
                error!("Dead letter queue processing error: {}", e);
            }
        });

        // Start analysis task if enabled
        if self.dlq_config.enable_analysis {
            let queue = self.clone();
            tokio::spawn(async move {
                queue.analysis_task().await;
            });
        }

        // Start cleanup task
        let queue = self.clone();
        tokio::spawn(async move {
            queue.cleanup_task().await;
        });

        Ok(())
    }

    /// Stop the dead letter queue
    pub async fn stop(&self) -> IntegrationResult<()> {
        info!("Stopping dead letter queue");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Add event to dead letter queue
    #[instrument(skip(self, event), fields(event_id = %event.id))]
    pub async fn add_event(&self, event: WebhookEvent, reason: String) -> IntegrationResult<()> {
        let mut entry = DeadLetterEntry::new(event, reason);

        // Add automatic tags based on failure analysis
        match entry.failure_analysis.category {
            FailureCategory::Timeout => entry.add_tag("timeout".to_string()),
            FailureCategory::Network => entry.add_tag("network".to_string()),
            FailureCategory::Authentication => entry.add_tag("auth".to_string()),
            FailureCategory::Validation => entry.add_tag("validation".to_string()),
            FailureCategory::RateLimit => entry.add_tag("rate_limit".to_string()),
            FailureCategory::Configuration => entry.add_tag("config".to_string()),
            FailureCategory::External => entry.add_tag("external".to_string()),
            FailureCategory::Unknown => entry.add_tag("unknown".to_string()),
        }

        // Send for processing
        self.entry_sender
            .send(entry)
            .map_err(|_| IntegrationError::internal("Entry sender closed"))?;

        debug!("Event added to dead letter queue");
        Ok(())
    }

    /// Replay a dead letter entry
    #[instrument(skip(self), fields(entry_id = %entry_id))]
    pub async fn replay_entry(&self, entry_id: Uuid) -> IntegrationResult<WebhookEvent> {
        if !self.dlq_config.enable_replay {
            return Err(IntegrationError::service_unavailable("replay"));
        }

        let mut entry = self
            .storage
            .get_entry(entry_id)
            .await
            .map_err(|e| IntegrationError::from(e))?
            .ok_or_else(|| {
                IntegrationError::not_found(format!("Dead letter entry not found: {}", entry_id))
            })?;

        if !entry.can_replay() {
            return Err(IntegrationError::webhook_processing(
                "Entry has exceeded maximum replay attempts",
            ));
        }

        // Mark as replayed
        entry.mark_replayed();

        // Update entry in storage
        self.storage
            .update_entry(&entry)
            .await
            .map_err(|e| IntegrationError::from(e))?;

        // Reset event for replay
        let mut event = entry.event.clone();
        event.status = WebhookEventStatus::Received;
        event.attempt_count = 0;
        event.error = None;
        event.next_retry_at = None;
        event.updated_at = Utc::now();

        info!(
            entry_id = %entry_id,
            replay_attempt = entry.replay_attempts,
            "Replaying dead letter entry"
        );

        Ok(event)
    }

    /// Get dead letter statistics
    pub async fn get_stats(&self) -> IntegrationResult<DeadLetterStats> {
        self.storage
            .get_stats()
            .await
            .map_err(|e| IntegrationError::from(e))
    }

    /// Get entries with pagination
    pub async fn get_entries(
        &self,
        limit: usize,
        offset: usize,
    ) -> IntegrationResult<Vec<DeadLetterEntry>> {
        self.storage
            .get_entries(limit, offset)
            .await
            .map_err(|e| IntegrationError::from(e))
    }

    /// Search entries by various criteria
    pub async fn search_entries(
        &self,
        category: Option<FailureCategory>,
        severity: Option<FailureSeverity>,
        tags: Option<Vec<String>>,
        limit: usize,
    ) -> IntegrationResult<Vec<DeadLetterEntry>> {
        if let Some(category) = category {
            self.storage
                .get_entries_by_category(category, limit)
                .await
                .map_err(|e| IntegrationError::from(e))
        } else if let Some(severity) = severity {
            self.storage
                .get_entries_by_severity(severity, limit)
                .await
                .map_err(|e| IntegrationError::from(e))
        } else if let Some(tags) = tags {
            self.storage
                .search_by_tags(&tags, limit)
                .await
                .map_err(|e| IntegrationError::from(e))
        } else {
            self.get_entries(limit, 0).await
        }
    }

    /// Process dead letter queue (placeholder)
    pub async fn process_queue(&self) -> IntegrationResult<()> {
        // This method can be used for background processing of dead letter entries
        // such as automatic analysis, alerting, or cleanup
        debug!("Processing dead letter queue");
        Ok(())
    }

    /// Clone for background tasks
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            dlq_config: self.dlq_config.clone(),
            storage: Arc::clone(&self.storage),
            stats: Arc::clone(&self.stats),
            running: Arc::clone(&self.running),
            processed_count: Arc::clone(&self.processed_count),
            entry_sender: self.entry_sender.clone(),
            entry_receiver: Arc::clone(&self.entry_receiver),
        }
    }

    /// Entry processing loop
    async fn entry_processing_loop(&self) -> IntegrationResult<()> {
        let mut receiver = self
            .entry_receiver
            .write()
            .take()
            .ok_or_else(|| IntegrationError::internal("Entry receiver already taken"))?;

        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                entry = receiver.recv() => {
                    match entry {
                        Some(entry) => {
                            if let Err(e) = self.storage.store_entry(entry).await {
                                warn!("Failed to store dead letter entry: {}", e);
                            } else {
                                self.processed_count.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        None => break, // Channel closed
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic maintenance
                }
            }
        }

        info!("Dead letter queue processing loop stopped");
        Ok(())
    }

    /// Analysis task for pattern detection and insights
    async fn analysis_task(&self) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(self.dlq_config.analysis_interval));

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            // Perform analysis (e.g., detect patterns, generate alerts)
            if let Ok(stats) = self.storage.get_stats().await {
                debug!(
                    "Dead letter queue analysis - Total: {}, Last 24h: {}, Avg age: {:.2}h",
                    stats.total_entries, stats.entries_last_24h, stats.avg_age_hours
                );

                // Could trigger alerts based on thresholds
                if stats.entries_last_24h > 100 {
                    warn!(
                        "High number of dead letter entries in last 24h: {}",
                        stats.entries_last_24h
                    );
                }
            }
        }
    }

    /// Cleanup task for old entries
    async fn cleanup_task(&self) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(self.dlq_config.cleanup_interval));

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            if let Ok(removed_count) = self
                .storage
                .cleanup_old_entries(self.dlq_config.retention_hours)
                .await
            {
                if removed_count > 0 {
                    info!("Cleaned up {} old dead letter entries", removed_count);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WebhookPayload;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_event() -> WebhookEvent {
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
        WebhookEvent::new(payload, super::super::EventPriority::Normal)
    }

    #[test]
    fn test_failure_analysis() {
        let event = create_test_event();
        let reason = "timeout error while processing webhook";

        let analysis = FailureAnalysis::analyze(&event, reason);

        assert_eq!(analysis.category, FailureCategory::Timeout);
        assert!(!analysis.remediation.is_empty());
        assert!(analysis
            .error_patterns
            .contains(&"TIMEOUT_ERROR".to_string()));
    }

    #[test]
    fn test_dead_letter_entry_creation() {
        let event = create_test_event();
        let reason = "Network connection failed".to_string();

        let entry = DeadLetterEntry::new(event, reason);

        assert_eq!(entry.failure_analysis.category, FailureCategory::Network);
        assert!(entry.can_replay());
        assert_eq!(entry.replay_attempts, 0);
    }

    #[test]
    fn test_dead_letter_entry_replay_logic() {
        let event = create_test_event();
        let mut entry = DeadLetterEntry::new(event, "Test failure".to_string());

        // Should be able to replay initially
        assert!(entry.can_replay());

        // Mark as replayed multiple times
        entry.mark_replayed();
        entry.mark_replayed();
        entry.mark_replayed();

        // Should not be able to replay after max attempts
        assert!(!entry.can_replay());
        assert_eq!(entry.replay_attempts, 3);
        assert!(entry.last_replay_at.is_some());
    }

    #[tokio::test]
    async fn test_memory_storage_operations() {
        let storage = MemoryDeadLetterStorage::new(10);
        let event = create_test_event();
        let entry = DeadLetterEntry::new(event, "Test failure".to_string());
        let entry_id = entry.event.id;

        // Store entry
        storage.store_entry(entry.clone()).await.unwrap();

        // Retrieve entry
        let retrieved = storage.get_entry(entry_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().event.id, entry_id);

        // Update entry
        let mut updated_entry = entry.clone();
        updated_entry.mark_replayed();
        storage.update_entry(&updated_entry).await.unwrap();

        // Verify update
        let retrieved = storage.get_entry(entry_id).await.unwrap().unwrap();
        assert_eq!(retrieved.replay_attempts, 1);

        // Remove entry
        storage.remove_entry(entry_id).await.unwrap();
        let retrieved = storage.get_entry(entry_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_storage_search_and_filtering() {
        let storage = MemoryDeadLetterStorage::new(10);

        // Create entries with different categories
        let timeout_event = create_test_event();
        let timeout_entry = DeadLetterEntry::new(timeout_event, "timeout error".to_string());

        let network_event = create_test_event();
        let network_entry = DeadLetterEntry::new(network_event, "network error".to_string());

        storage.store_entry(timeout_entry).await.unwrap();
        storage.store_entry(network_entry).await.unwrap();

        // Search by category
        let timeout_entries = storage
            .get_entries_by_category(FailureCategory::Timeout, 10)
            .await
            .unwrap();
        assert_eq!(timeout_entries.len(), 1);
        assert_eq!(
            timeout_entries[0].failure_analysis.category,
            FailureCategory::Timeout
        );

        let network_entries = storage
            .get_entries_by_category(FailureCategory::Network, 10)
            .await
            .unwrap();
        assert_eq!(network_entries.len(), 1);
        assert_eq!(
            network_entries[0].failure_analysis.category,
            FailureCategory::Network
        );
    }

    #[tokio::test]
    async fn test_stats_calculation() {
        let storage = MemoryDeadLetterStorage::new(10);

        // Add several entries
        for i in 0..5 {
            let event = create_test_event();
            let reason = if i % 2 == 0 {
                "timeout error"
            } else {
                "network error"
            };
            let entry = DeadLetterEntry::new(event, reason.to_string());
            storage.store_entry(entry).await.unwrap();
        }

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.entries_last_24h, 5);
        assert!(stats.category_breakdown.len() >= 2);
        assert!(stats.category_breakdown.contains_key("Timeout"));
        assert!(stats.category_breakdown.contains_key("Network"));
    }

    #[tokio::test]
    async fn test_dead_letter_queue_lifecycle() {
        let config = WebhookConfig::default();
        let queue = DeadLetterQueue::new(config);

        // Start queue
        queue.start().await.unwrap();
        assert!(queue.running.load(Ordering::SeqCst));

        // Add an event
        let event = create_test_event();
        queue
            .add_event(event, "Test failure".to_string())
            .await
            .unwrap();

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check stats
        let stats = queue.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 1);

        // Stop queue
        queue.stop().await.unwrap();
        assert!(!queue.running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_entry_replay() {
        let config = WebhookConfig::default();
        let queue = DeadLetterQueue::new(config);
        queue.start().await.unwrap();

        // Add an event
        let event = create_test_event();
        let event_id = event.id;
        queue
            .add_event(event, "Test failure".to_string())
            .await
            .unwrap();

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Replay the entry
        let replayed_event = queue.replay_entry(event_id).await.unwrap();
        assert_eq!(replayed_event.id, event_id);
        assert_eq!(replayed_event.status, WebhookEventStatus::Received);
        assert_eq!(replayed_event.attempt_count, 0);

        queue.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_cleanup_old_entries() {
        let storage = MemoryDeadLetterStorage::new(10);

        // Create an entry with old timestamp
        let event = create_test_event();
        let mut entry = DeadLetterEntry::new(event, "Test failure".to_string());
        entry.dead_lettered_at = Utc::now() - ChronoDuration::hours(169); // Older than default retention

        storage.store_entry(entry).await.unwrap();

        // Cleanup old entries (retention period = 168 hours by default)
        let removed_count = storage.cleanup_old_entries(168).await.unwrap();
        assert_eq!(removed_count, 1);

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 0);
    }
}
