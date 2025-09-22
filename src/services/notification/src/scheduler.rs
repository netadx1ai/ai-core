//! Notification scheduler module
//!
//! This module provides scheduling functionality for delayed notifications:
//! - Scheduled notification processing
//! - Retry logic with exponential backoff
//! - Cleanup of expired notifications
//! - Background task management
//! - Cron-like scheduling support

use crate::config::SchedulerConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::NotificationResponse;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Notification scheduler for handling delayed and recurring notifications
#[derive(Clone)]
pub struct NotificationScheduler {
    config: SchedulerConfig,
    scheduler: Arc<RwLock<Option<JobScheduler>>>,
    scheduled_notifications: Arc<RwLock<HashMap<String, ScheduledNotification>>>,
    task_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    is_running: Arc<RwLock<bool>>,
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

#[derive(Debug, Clone)]
struct ScheduledNotification {
    id: String,
    notification: NotificationResponse,
    scheduled_at: DateTime<Utc>,
    retry_count: u32,
    next_retry_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl NotificationScheduler {
    /// Create a new notification scheduler
    pub async fn new(config: &SchedulerConfig) -> Result<Self> {
        info!("Initializing notification scheduler");

        let scheduler = if config.enabled {
            match JobScheduler::new().await {
                Ok(sched) => Some(sched),
                Err(e) => {
                    warn!(
                        "Failed to create job scheduler: {}, using basic scheduler",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        info!("Notification scheduler initialized successfully");

        Ok(Self {
            config: config.clone(),
            scheduler: Arc::new(RwLock::new(scheduler)),
            scheduled_notifications: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
            shutdown_tx: Arc::new(RwLock::new(None)),
        })
    }

    /// Start the scheduler background tasks
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }

        info!("Starting notification scheduler");

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Start the job scheduler if available
        if let Some(ref scheduler) = *self.scheduler.read().await {
            scheduler.start().await.map_err(|e| {
                NotificationError::internal(format!("Failed to start job scheduler: {}", e))
            })?;
        }

        // Start background processing task
        let processing_task = self.start_processing_task(shutdown_rx).await;

        // Start cleanup task
        let cleanup_task = self.start_cleanup_task().await;

        // Store task handles
        let mut handles = self.task_handles.write().await;
        handles.push(processing_task);
        handles.push(cleanup_task);

        *is_running = true;
        info!("Notification scheduler started successfully");

        Ok(())
    }

    /// Stop the scheduler and all background tasks
    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        info!("Stopping notification scheduler");

        // Send shutdown signal
        if let Some(ref tx) = *self.shutdown_tx.read().await {
            let _ = tx.send(()).await;
        }

        // Stop job scheduler
        if let Some(ref mut scheduler) = *self.scheduler.write().await {
            scheduler.shutdown().await.map_err(|e| {
                NotificationError::internal(format!("Failed to stop job scheduler: {}", e))
            })?;
        }

        // Wait for tasks to complete
        let mut handles = self.task_handles.write().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        *is_running = false;
        info!("Notification scheduler stopped successfully");

        Ok(())
    }

    /// Schedule a notification for future delivery
    pub async fn schedule_notification(&self, notification: &NotificationResponse) -> Result<()> {
        let scheduled_at = notification.scheduled_at.ok_or_else(|| {
            NotificationError::validation("scheduled_at", "Required for scheduled notifications")
        })?;

        if scheduled_at <= Utc::now() {
            return Err(NotificationError::validation(
                "scheduled_at",
                "Must be in the future",
            ));
        }

        let scheduled_notification = ScheduledNotification {
            id: notification.id.clone(),
            notification: notification.clone(),
            scheduled_at,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now(),
        };

        // Store in memory
        let mut scheduled = self.scheduled_notifications.write().await;
        scheduled.insert(notification.id.clone(), scheduled_notification);

        info!(
            "Notification scheduled for delivery at {}: {}",
            scheduled_at, notification.id
        );

        Ok(())
    }

    /// Cancel a scheduled notification
    pub async fn cancel_scheduled_notification(&self, notification_id: &str) -> Result<bool> {
        let mut scheduled = self.scheduled_notifications.write().await;
        if scheduled.remove(notification_id).is_some() {
            info!("Cancelled scheduled notification: {}", notification_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Schedule a notification for retry
    pub async fn schedule_retry(
        &self,
        notification: &NotificationResponse,
        retry_count: u32,
    ) -> Result<()> {
        let delay_seconds = self.calculate_retry_delay(retry_count);
        let next_retry_at = Utc::now() + chrono::Duration::seconds(delay_seconds as i64);

        let scheduled_notification = ScheduledNotification {
            id: notification.id.clone(),
            notification: notification.clone(),
            scheduled_at: next_retry_at,
            retry_count,
            next_retry_at: Some(next_retry_at),
            created_at: Utc::now(),
        };

        let mut scheduled = self.scheduled_notifications.write().await;
        scheduled.insert(notification.id.clone(), scheduled_notification);

        info!(
            "Notification scheduled for retry {} at {}: {}",
            retry_count, next_retry_at, notification.id
        );

        Ok(())
    }

    /// Get scheduled notification statistics
    pub async fn get_scheduler_stats(&self) -> serde_json::Value {
        let scheduled = self.scheduled_notifications.read().await;
        let now = Utc::now();

        let mut pending = 0;
        let mut ready = 0;
        let mut retries = 0;

        for notification in scheduled.values() {
            if notification.retry_count > 0 {
                retries += 1;
            }

            if notification.scheduled_at <= now {
                ready += 1;
            } else {
                pending += 1;
            }
        }

        serde_json::json!({
            "total_scheduled": scheduled.len(),
            "pending": pending,
            "ready_for_delivery": ready,
            "retries": retries,
            "is_running": *self.is_running.read().await
        })
    }

    /// Add a recurring notification job (requires cron scheduler)
    pub async fn add_recurring_job(
        &self,
        cron_expression: &str,
        job_fn: Arc<dyn Fn() -> Result<()> + Send + Sync>,
    ) -> Result<Uuid> {
        let scheduler_guard = self.scheduler.read().await;
        let scheduler = scheduler_guard
            .as_ref()
            .ok_or_else(|| NotificationError::config("Job scheduler not available"))?;

        let job = Job::new_async(cron_expression, move |_uuid, _l| {
            let job_fn_clone = job_fn.clone();
            Box::pin(async move {
                if let Err(e) = job_fn_clone() {
                    error!("Recurring job failed: {}", e);
                }
            })
        })
        .map_err(|e| NotificationError::config(format!("Invalid cron expression: {}", e)))?;

        let job_id = scheduler.add(job).await.map_err(|e| {
            NotificationError::internal(format!("Failed to add recurring job: {}", e))
        })?;

        info!(
            "Added recurring job with cron expression '{}': {}",
            cron_expression, job_id
        );
        Ok(job_id)
    }

    /// Remove a recurring job
    pub async fn remove_recurring_job(&self, job_id: Uuid) -> Result<bool> {
        let scheduler_guard = self.scheduler.read().await;
        let scheduler = scheduler_guard
            .as_ref()
            .ok_or_else(|| NotificationError::config("Job scheduler not available"))?;

        match scheduler.remove(&job_id).await {
            Ok(_) => {
                info!("Removed recurring job: {}", job_id);
                Ok(true)
            }
            Err(e) => {
                warn!("Failed to remove recurring job {}: {}", job_id, e);
                Ok(false)
            }
        }
    }

    // Private helper methods

    async fn start_processing_task(
        &self,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        let scheduled_notifications = self.scheduled_notifications.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.check_interval_seconds));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::process_scheduled_notifications(&scheduled_notifications, &config).await {
                            error!("Error processing scheduled notifications: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Notification processing task shutting down");
                        break;
                    }
                }
            }
        })
    }

    async fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let scheduled_notifications = self.scheduled_notifications.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.cleanup_interval_hours * 3600));

            loop {
                interval.tick().await;
                if let Err(e) =
                    Self::cleanup_expired_notifications(&scheduled_notifications, &config).await
                {
                    error!("Error cleaning up expired notifications: {}", e);
                }
            }
        })
    }

    async fn process_scheduled_notifications(
        scheduled_notifications: &Arc<RwLock<HashMap<String, ScheduledNotification>>>,
        config: &SchedulerConfig,
    ) -> Result<()> {
        let now = Utc::now();
        let mut to_process = Vec::new();

        // Collect notifications ready for processing
        {
            let scheduled = scheduled_notifications.read().await;
            for notification in scheduled.values() {
                if notification.scheduled_at <= now {
                    to_process.push(notification.clone());
                }
            }
        }

        if to_process.is_empty() {
            return Ok(());
        }

        info!("Processing {} scheduled notifications", to_process.len());

        // Process notifications in batches
        for chunk in to_process.chunks(config.batch_size) {
            for scheduled_notification in chunk {
                // In a real implementation, this would call back to the NotificationManager
                // to actually send the notification
                info!(
                    "Processing scheduled notification: {}",
                    scheduled_notification.id
                );

                // Remove from scheduled list after processing
                let mut scheduled = scheduled_notifications.write().await;
                scheduled.remove(&scheduled_notification.id);
            }

            // Small delay between batches to avoid overwhelming the system
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    async fn cleanup_expired_notifications(
        scheduled_notifications: &Arc<RwLock<HashMap<String, ScheduledNotification>>>,
        config: &SchedulerConfig,
    ) -> Result<()> {
        let cutoff_time = Utc::now() - chrono::Duration::days(config.retention_days as i64);
        let mut to_remove = Vec::new();

        // Find expired notifications
        {
            let scheduled = scheduled_notifications.read().await;
            for (id, notification) in scheduled.iter() {
                if notification.created_at < cutoff_time {
                    to_remove.push(id.clone());
                }
            }
        }

        if !to_remove.is_empty() {
            let mut scheduled = scheduled_notifications.write().await;
            for id in &to_remove {
                scheduled.remove(id);
            }
            info!(
                "Cleaned up {} expired scheduled notifications",
                to_remove.len()
            );
        }

        Ok(())
    }

    fn calculate_retry_delay(&self, retry_count: u32) -> u64 {
        let base_delay = 1; // Default initial delay in seconds
        let max_delay = 300; // Default max delay in seconds (5 minutes)
        let multiplier: f64 = 2.0; // Default backoff multiplier

        let delay = (base_delay as f64 * multiplier.powi(retry_count as i32)) as u64;
        delay.min(max_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SchedulerConfig;
    use ai_core_shared::types::*;

    fn create_test_config() -> SchedulerConfig {
        SchedulerConfig {
            enabled: true,
            worker_threads: 2,
            check_interval_seconds: 10,
            batch_size: 50,
            cleanup_interval_hours: 24,
            retention_days: 7,
        }
    }

    fn create_test_notification() -> NotificationResponse {
        let now = Utc::now();
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "user123".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test Scheduled Notification".to_string(),
            content: "This is a test scheduled notification".to_string(),
            channels: vec![NotificationChannel::Email],
            priority: NotificationPriority::Normal,
            status: NotificationStatus::Pending,
            delivery_attempts: vec![],
            created_at: now,
            updated_at: now,
            scheduled_at: Some(now + chrono::Duration::minutes(5)),
            delivered_at: None,
            expires_at: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await;
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_schedule_notification() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();
        let notification = create_test_notification();

        let result = scheduler.schedule_notification(&notification).await;
        assert!(result.is_ok());

        let stats = scheduler.get_scheduler_stats().await;
        assert_eq!(stats["total_scheduled"], 1);
    }

    #[tokio::test]
    async fn test_cancel_scheduled_notification() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();
        let notification = create_test_notification();

        scheduler
            .schedule_notification(&notification)
            .await
            .unwrap();

        let cancelled = scheduler
            .cancel_scheduled_notification(&notification.id)
            .await
            .unwrap();
        assert!(cancelled);

        let stats = scheduler.get_scheduler_stats().await;
        assert_eq!(stats["total_scheduled"], 0);
    }

    #[tokio::test]
    async fn test_schedule_retry() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();
        let notification = create_test_notification();

        let result = scheduler.schedule_retry(&notification, 1).await;
        assert!(result.is_ok());

        let stats = scheduler.get_scheduler_stats().await;
        assert_eq!(stats["total_scheduled"], 1);
        assert_eq!(stats["retries"], 1);
    }

    #[tokio::test]
    async fn test_calculate_retry_delay() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();

        // Test exponential backoff (assuming defaults)
        let delay1 = scheduler.calculate_retry_delay(1);
        let delay2 = scheduler.calculate_retry_delay(2);
        let delay3 = scheduler.calculate_retry_delay(3);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
    }

    #[tokio::test]
    async fn test_scheduler_stats() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();

        let stats = scheduler.get_scheduler_stats().await;
        assert_eq!(stats["total_scheduled"], 0);
        assert_eq!(stats["pending"], 0);
        assert_eq!(stats["ready_for_delivery"], 0);
        assert_eq!(stats["retries"], 0);
        assert_eq!(stats["is_running"], false);
    }

    #[tokio::test]
    async fn test_start_stop_scheduler() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();

        // Start scheduler
        let result = scheduler.start().await;
        assert!(result.is_ok());

        let stats = scheduler.get_scheduler_stats().await;
        assert_eq!(stats["is_running"], true);

        // Stop scheduler
        let result = scheduler.stop().await;
        assert!(result.is_ok());

        let stats = scheduler.get_scheduler_stats().await;
        assert_eq!(stats["is_running"], false);
    }

    #[tokio::test]
    async fn test_scheduler_validation() {
        let config = create_test_config();
        let scheduler = NotificationScheduler::new(&config).await.unwrap();

        let mut notification = create_test_notification();
        notification.scheduled_at = Some(Utc::now() - chrono::Duration::hours(1)); // Past time

        let result = scheduler.schedule_notification(&notification).await;
        assert!(result.is_err());
    }
}
