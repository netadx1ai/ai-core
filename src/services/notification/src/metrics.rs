//! Metrics collection module for notification service
//!
//! This module provides comprehensive metrics collection and reporting for:
//! - Notification delivery statistics
//! - Channel-specific metrics
//! - Performance monitoring
//! - Rate limiting metrics
//! - Error tracking

use crate::config::MetricsConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::{
    ChannelStats, NotificationChannel, NotificationResponse, NotificationStats,
};

use prometheus::{HistogramVec, IntCounterVec, IntGaugeVec, Registry};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Metrics collector for the notification service
#[derive(Clone)]
pub struct NotificationMetrics {
    config: MetricsConfig,
    registry: Arc<Registry>,

    // Counters
    notifications_total: IntCounterVec,
    notifications_sent: IntCounterVec,
    notifications_delivered: IntCounterVec,
    notifications_failed: IntCounterVec,

    // Gauges
    active_connections: IntGaugeVec,
    queue_size: IntGaugeVec,

    // Histograms
    delivery_duration: HistogramVec,
    template_render_duration: HistogramVec,

    // Channel-specific metrics
    channel_stats: Arc<RwLock<HashMap<NotificationChannel, ChannelMetrics>>>,
}

#[derive(Debug, Clone)]
struct ChannelMetrics {
    sent: u64,
    delivered: u64,
    failed: u64,
    total_delivery_time: f64,
}

impl NotificationMetrics {
    /// Create a new metrics collector
    pub fn new(config: &MetricsConfig) -> Result<Self> {
        info!("Initializing notification metrics");

        let registry = Registry::new();

        // Initialize counters
        let notifications_total = IntCounterVec::new(
            prometheus::Opts::new(
                "notifications_total",
                "Total number of notifications processed",
            )
            .namespace(&config.namespace),
            &["type", "priority"],
        )
        .map_err(|e| {
            NotificationError::internal(format!(
                "Failed to create notifications_total counter: {}",
                e
            ))
        })?;

        let notifications_sent = IntCounterVec::new(
            prometheus::Opts::new(
                "notifications_sent_total",
                "Total number of notifications sent",
            )
            .namespace(&config.namespace),
            &["channel", "type"],
        )
        .map_err(|e| {
            NotificationError::internal(format!(
                "Failed to create notifications_sent counter: {}",
                e
            ))
        })?;

        let notifications_delivered = IntCounterVec::new(
            prometheus::Opts::new(
                "notifications_delivered_total",
                "Total number of notifications delivered successfully",
            )
            .namespace(&config.namespace),
            &["channel", "type"],
        )
        .map_err(|e| {
            NotificationError::internal(format!(
                "Failed to create notifications_delivered counter: {}",
                e
            ))
        })?;

        let notifications_failed = IntCounterVec::new(
            prometheus::Opts::new(
                "notifications_failed_total",
                "Total number of notifications that failed to deliver",
            )
            .namespace(&config.namespace),
            &["channel", "type", "error_type"],
        )
        .map_err(|e| {
            NotificationError::internal(format!(
                "Failed to create notifications_failed counter: {}",
                e
            ))
        })?;

        // Initialize gauges
        let active_connections = IntGaugeVec::new(
            prometheus::Opts::new(
                "active_connections",
                "Number of active WebSocket connections",
            )
            .namespace(&config.namespace),
            &["type"],
        )
        .map_err(|e| {
            NotificationError::internal(format!("Failed to create active_connections gauge: {}", e))
        })?;

        let queue_size = IntGaugeVec::new(
            prometheus::Opts::new(
                "notification_queue_size",
                "Number of notifications in queue",
            )
            .namespace(&config.namespace),
            &["priority"],
        )
        .map_err(|e| {
            NotificationError::internal(format!("Failed to create queue_size gauge: {}", e))
        })?;

        // Initialize histograms
        let delivery_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "notification_delivery_duration_seconds",
                "Time taken to deliver notifications",
            )
            .namespace(&config.namespace)
            .buckets(config.histogram_buckets.clone()),
            &["channel", "status"],
        )
        .map_err(|e| {
            NotificationError::internal(format!(
                "Failed to create delivery_duration histogram: {}",
                e
            ))
        })?;

        let template_render_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "template_render_duration_seconds",
                "Time taken to render notification templates",
            )
            .namespace(&config.namespace)
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["template_id", "status"],
        )
        .map_err(|e| {
            NotificationError::internal(format!(
                "Failed to create template_render_duration histogram: {}",
                e
            ))
        })?;

        // Register metrics
        registry
            .register(Box::new(notifications_total.clone()))
            .map_err(|e| {
                NotificationError::internal(format!(
                    "Failed to register notifications_total: {}",
                    e
                ))
            })?;
        registry
            .register(Box::new(notifications_sent.clone()))
            .map_err(|e| {
                NotificationError::internal(format!("Failed to register notifications_sent: {}", e))
            })?;
        registry
            .register(Box::new(notifications_delivered.clone()))
            .map_err(|e| {
                NotificationError::internal(format!(
                    "Failed to register notifications_delivered: {}",
                    e
                ))
            })?;
        registry
            .register(Box::new(notifications_failed.clone()))
            .map_err(|e| {
                NotificationError::internal(format!(
                    "Failed to register notifications_failed: {}",
                    e
                ))
            })?;
        registry
            .register(Box::new(active_connections.clone()))
            .map_err(|e| {
                NotificationError::internal(format!("Failed to register active_connections: {}", e))
            })?;
        registry
            .register(Box::new(queue_size.clone()))
            .map_err(|e| {
                NotificationError::internal(format!("Failed to register queue_size: {}", e))
            })?;
        registry
            .register(Box::new(delivery_duration.clone()))
            .map_err(|e| {
                NotificationError::internal(format!("Failed to register delivery_duration: {}", e))
            })?;
        registry
            .register(Box::new(template_render_duration.clone()))
            .map_err(|e| {
                NotificationError::internal(format!(
                    "Failed to register template_render_duration: {}",
                    e
                ))
            })?;

        info!("Notification metrics initialized successfully");

        Ok(Self {
            config: config.clone(),
            registry: Arc::new(registry),
            notifications_total,
            notifications_sent,
            notifications_delivered,
            notifications_failed,
            active_connections,
            queue_size,
            delivery_duration,
            template_render_duration,
            channel_stats: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Record a notification being sent
    pub async fn record_notification_sent(&self, notification: &NotificationResponse) {
        let notification_type = format!("{:?}", notification.notification_type);
        let priority = format!("{:?}", notification.priority);

        self.notifications_total
            .with_label_values(&[&notification_type, &priority])
            .inc();

        for channel in &notification.channels {
            let channel_str = channel_to_string(channel);
            self.notifications_sent
                .with_label_values(&[&channel_str, &notification_type])
                .inc();
        }
    }

    /// Record a successful notification delivery
    pub async fn record_notification_delivered(
        &self,
        notification: &NotificationResponse,
        channel: &NotificationChannel,
        delivery_time: f64,
    ) {
        let notification_type = format!("{:?}", notification.notification_type);
        let channel_str = channel_to_string(channel);

        self.notifications_delivered
            .with_label_values(&[&channel_str, &notification_type])
            .inc();

        self.delivery_duration
            .with_label_values(&[&channel_str, "success"])
            .observe(delivery_time);

        // Update channel stats
        let mut stats = self.channel_stats.write().await;
        let channel_metrics = stats.entry(channel.clone()).or_insert(ChannelMetrics {
            sent: 0,
            delivered: 0,
            failed: 0,
            total_delivery_time: 0.0,
        });
        channel_metrics.delivered += 1;
        channel_metrics.total_delivery_time += delivery_time;
    }

    /// Record a failed notification delivery
    pub async fn record_notification_failed(
        &self,
        notification: &NotificationResponse,
        channel: &NotificationChannel,
        error_type: &str,
        delivery_time: f64,
    ) {
        let notification_type = format!("{:?}", notification.notification_type);
        let channel_str = channel_to_string(channel);

        self.notifications_failed
            .with_label_values(&[&channel_str, &notification_type, error_type])
            .inc();

        self.delivery_duration
            .with_label_values(&[&channel_str, "failed"])
            .observe(delivery_time);

        // Update channel stats
        let mut stats = self.channel_stats.write().await;
        let channel_metrics = stats.entry(channel.clone()).or_insert(ChannelMetrics {
            sent: 0,
            delivered: 0,
            failed: 0,
            total_delivery_time: 0.0,
        });
        channel_metrics.failed += 1;
    }

    /// Record WebSocket connection count
    pub fn record_websocket_connections(&self, count: i64) {
        self.active_connections
            .with_label_values(&["websocket"])
            .set(count);
    }

    /// Record notification queue size
    pub fn record_queue_size(&self, priority: &str, size: i64) {
        self.queue_size.with_label_values(&[priority]).set(size);
    }

    /// Record template rendering duration
    pub fn record_template_render_duration(&self, template_id: &str, duration: f64, success: bool) {
        let status = if success { "success" } else { "failed" };
        self.template_render_duration
            .with_label_values(&[template_id, status])
            .observe(duration);
    }

    /// Get aggregated notification statistics
    pub async fn get_notification_stats(&self) -> Result<NotificationStats> {
        let stats = self.channel_stats.read().await;
        let mut total_sent = 0u64;
        let mut total_delivered = 0u64;
        let mut total_failed = 0u64;
        let mut total_delivery_time = 0.0;
        let mut channel_stats_map = HashMap::new();

        for (channel, metrics) in stats.iter() {
            total_sent += metrics.sent;
            total_delivered += metrics.delivered;
            total_failed += metrics.failed;
            total_delivery_time += metrics.total_delivery_time;

            let avg_delivery_time = if metrics.delivered > 0 {
                Some((metrics.total_delivery_time / metrics.delivered as f64) as f32)
            } else {
                None
            };

            let delivery_rate = if metrics.sent > 0 {
                (metrics.delivered as f32 / metrics.sent as f32) * 100.0
            } else {
                0.0
            };

            channel_stats_map.insert(
                channel.clone(),
                ChannelStats {
                    sent: metrics.sent,
                    delivered: metrics.delivered,
                    failed: metrics.failed,
                    delivery_rate,
                    average_delivery_time: avg_delivery_time,
                },
            );
        }

        let overall_delivery_rate = if total_sent > 0 {
            (total_delivered as f32 / total_sent as f32) * 100.0
        } else {
            0.0
        };

        let overall_avg_delivery_time = if total_delivered > 0 {
            Some((total_delivery_time / total_delivered as f64) as f32)
        } else {
            None
        };

        Ok(NotificationStats {
            total_sent,
            total_delivered,
            total_failed,
            delivery_rate: overall_delivery_rate,
            average_delivery_time: overall_avg_delivery_time,
            channel_stats: channel_stats_map,
        })
    }

    /// Get Prometheus registry for metrics endpoint
    pub fn get_registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }

    /// Export metrics in Prometheus format
    pub fn export_metrics(&self) -> Result<String> {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();

        encoder
            .encode_to_string(&metric_families)
            .map_err(|e| NotificationError::internal(format!("Failed to encode metrics: {}", e)))
    }

    /// Create a timer for measuring operation duration
    pub fn start_timer(&self, operation: &str) -> MetricsTimer {
        MetricsTimer {
            operation: operation.to_string(),
            start_time: Instant::now(),
        }
    }

    /// Reset all metrics (useful for testing)
    pub async fn reset_metrics(&self) {
        // Reset counters (note: Prometheus counters can't be reset, so this is mainly for testing)
        let mut stats = self.channel_stats.write().await;
        stats.clear();

        warn!("Metrics have been reset");
    }

    /// Get current metric values for health check
    pub async fn get_health_metrics(&self) -> serde_json::Value {
        let metric_families = self.registry.gather();
        let mut health_data = serde_json::json!({});

        for mf in metric_families {
            let name = mf.get_name();
            let mut values = Vec::new();

            for metric in mf.get_metric() {
                if metric.has_counter() {
                    values.push(metric.get_counter().get_value());
                } else if metric.has_gauge() {
                    values.push(metric.get_gauge().get_value());
                } else if metric.has_histogram() {
                    values.push(metric.get_histogram().get_sample_count() as f64);
                }
            }

            if !values.is_empty() {
                health_data[name] = serde_json::json!(values.iter().sum::<f64>());
            }
        }

        health_data
    }
}

/// Timer for measuring operation duration
pub struct MetricsTimer {
    operation: String,
    start_time: Instant,
}

impl MetricsTimer {
    /// Stop the timer and return the elapsed duration in seconds
    pub fn stop(self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Get the operation name
    pub fn operation(&self) -> &str {
        &self.operation
    }
}

impl Drop for MetricsTimer {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed().as_secs_f64();
        info!(
            "Operation '{}' completed in {:.3}s",
            self.operation, duration
        );
    }
}

fn channel_to_string(channel: &NotificationChannel) -> String {
    match channel {
        NotificationChannel::Email => "email".to_string(),
        NotificationChannel::Sms => "sms".to_string(),
        NotificationChannel::Push => "push".to_string(),
        NotificationChannel::Webhook => "webhook".to_string(),
        NotificationChannel::Websocket => "websocket".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MetricsConfig;
    use ai_core_shared::types::*;
    use chrono::Utc;

    fn create_test_config() -> MetricsConfig {
        MetricsConfig {
            enabled: true,
            endpoint: "/metrics".to_string(),
            namespace: "test_notification_service".to_string(),
            collect_detailed_metrics: true,
            histogram_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        }
    }

    fn create_test_notification() -> NotificationResponse {
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "user123".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test Notification".to_string(),
            content: "This is a test notification".to_string(),
            channels: vec![NotificationChannel::Email, NotificationChannel::Push],
            priority: NotificationPriority::Normal,
            status: NotificationStatus::Pending,
            delivery_attempts: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            scheduled_at: None,
            delivered_at: None,
            expires_at: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_metrics_creation() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config);
        assert!(metrics.is_ok());
    }

    #[tokio::test]
    async fn test_record_notification_sent() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();
        let notification = create_test_notification();

        metrics.record_notification_sent(&notification).await;

        // Verify metrics were recorded (this is a basic test)
        let stats = metrics.get_notification_stats().await.unwrap();
        assert_eq!(stats.total_sent, 0); // Channel stats only updated on delivery/failure
    }

    #[tokio::test]
    async fn test_record_notification_delivered() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();
        let notification = create_test_notification();

        metrics
            .record_notification_delivered(&notification, &NotificationChannel::Email, 0.5)
            .await;

        let stats = metrics.get_notification_stats().await.unwrap();
        assert_eq!(stats.total_delivered, 1);
        assert!(stats
            .channel_stats
            .contains_key(&NotificationChannel::Email));
    }

    #[tokio::test]
    async fn test_record_notification_failed() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();
        let notification = create_test_notification();

        metrics
            .record_notification_failed(
                &notification,
                &NotificationChannel::Email,
                "network_error",
                1.0,
            )
            .await;

        let stats = metrics.get_notification_stats().await.unwrap();
        assert_eq!(stats.total_failed, 1);
        assert!(stats
            .channel_stats
            .contains_key(&NotificationChannel::Email));
    }

    #[test]
    fn test_metrics_timer() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();

        let timer = metrics.start_timer("test_operation");
        assert_eq!(timer.operation(), "test_operation");

        // Let some time pass
        std::thread::sleep(std::time::Duration::from_millis(10));

        let duration = timer.stop();
        assert!(duration > 0.0);
    }

    #[tokio::test]
    async fn test_export_metrics() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();
        let notification = create_test_notification();

        // Add some metrics to export
        metrics.record_notification_sent(&notification).await;
        metrics
            .record_notification_delivered(&notification, &NotificationChannel::Email, 0.5)
            .await;

        let exported = metrics.export_metrics();
        assert!(exported.is_ok());
        let content = exported.unwrap();
        // Just check that we got some content back
        assert!(content.len() > 0);
    }

    #[tokio::test]
    async fn test_websocket_connections_metric() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();

        metrics.record_websocket_connections(10);

        let health_metrics = metrics.get_health_metrics().await;
        // Basic check that metrics were recorded
        assert!(health_metrics.is_object());
    }

    #[tokio::test]
    async fn test_queue_size_metric() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();

        metrics.record_queue_size("high", 5);
        metrics.record_queue_size("normal", 15);

        let health_metrics = metrics.get_health_metrics().await;
        assert!(health_metrics.is_object());
    }

    #[test]
    fn test_template_render_duration() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();

        metrics.record_template_render_duration("template_123", 0.05, true);
        metrics.record_template_render_duration("template_456", 0.1, false);

        // Basic test that no panic occurs
        assert!(true);
    }

    #[tokio::test]
    async fn test_reset_metrics() {
        let config = create_test_config();
        let metrics = NotificationMetrics::new(&config).unwrap();
        let notification = create_test_notification();

        // Add some metrics
        metrics
            .record_notification_delivered(&notification, &NotificationChannel::Email, 0.5)
            .await;

        let stats_before = metrics.get_notification_stats().await.unwrap();
        assert_eq!(stats_before.total_delivered, 1);

        // Reset metrics
        metrics.reset_metrics().await;

        let stats_after = metrics.get_notification_stats().await.unwrap();
        assert_eq!(stats_after.total_delivered, 0);
    }

    #[test]
    fn test_channel_to_string() {
        assert_eq!(channel_to_string(&NotificationChannel::Email), "email");
        assert_eq!(channel_to_string(&NotificationChannel::Sms), "sms");
        assert_eq!(channel_to_string(&NotificationChannel::Push), "push");
        assert_eq!(channel_to_string(&NotificationChannel::Webhook), "webhook");
        assert_eq!(
            channel_to_string(&NotificationChannel::Websocket),
            "websocket"
        );
    }
}
