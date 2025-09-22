//! # Notification Service
//!
//! Multi-channel notification service for the AI-CORE platform providing:
//! - Email notifications via SMTP
//! - SMS notifications via Twilio/AWS SNS
//! - Push notifications (Web Push, FCM)
//! - Webhook notifications
//! - Real-time WebSocket notifications
//! - Template management and personalization
//! - Delivery tracking and retry mechanisms
//! - Subscription management
//!
//! ## Features
//!
//! - **Multi-channel delivery**: Support for email, SMS, push, webhooks, and WebSocket
//! - **Template engine**: Handlebars-based templating with personalization
//! - **Delivery tracking**: Comprehensive tracking of delivery attempts and status
//! - **Retry mechanisms**: Configurable retry logic with exponential backoff
//! - **Rate limiting**: Per-channel and per-user rate limiting
//! - **Real-time updates**: WebSocket connections for instant notifications
//! - **Subscription management**: User preference management and opt-out support
//! - **Analytics**: Delivery statistics and performance metrics
//!
//! ## Usage
//!
//! ```rust,no_run
//! use notification_service::{NotificationService, NotificationConfig};
//! use ai_core_shared::types::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = NotificationConfig::default();
//!     let service = NotificationService::new(config).await?;
//!
//!     let request = CreateNotificationRequest {
//!         recipient_id: "user123".to_string(),
//!         notification_type: NotificationType::WorkflowCompleted,
//!         title: "Workflow Complete".to_string(),
//!         content: "Your automation workflow has completed successfully.".to_string(),
//!         channels: vec![NotificationChannel::Email, NotificationChannel::Push],
//!         priority: NotificationPriority::Normal,
//!         template_id: None,
//!         template_data: None,
//!         scheduled_at: None,
//!         expires_at: None,
//!         metadata: None,
//!     };
//!
//!     let notification = service.send_notification(request).await?;
//!     println!("Notification sent: {}", notification.id);
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

pub mod channels;
pub mod config;
pub mod error;
pub mod handlers;
pub mod manager;
pub mod metrics;
pub mod routes;
pub mod scheduler;
pub mod templates;
pub mod websocket;

pub use config::NotificationConfig;
pub use error::{NotificationError, Result};
pub use manager::NotificationManager;

// Re-export shared types for convenience
pub use ai_core_shared::types::{
    BulkNotificationRequest, BulkNotificationResponse, BulkNotificationResult, BulkOperationStatus,
    ChannelStats, CreateNotificationRequest, CreateSubscriptionRequest, CreateTemplateRequest,
    DeliveryAttempt, DeliveryStatus, NotificationChannel, NotificationFrequency,
    NotificationPreferences, NotificationPriority, NotificationResponse, NotificationStats,
    NotificationStatus, NotificationSubscription, NotificationTemplate, NotificationType,
    QuietHours, TemplateVariable, UpdateSubscriptionRequest, UpdateTemplateRequest, VariableType,
    WebSocketMessage, WebSocketMessageType,
};

/// Main notification service struct that coordinates all notification operations
#[derive(Clone)]
pub struct NotificationService {
    manager: Arc<NotificationManager>,
}

impl NotificationService {
    /// Create a new notification service with the given configuration
    pub async fn new(config: NotificationConfig) -> Result<Self> {
        let manager = Arc::new(NotificationManager::new(config).await?);

        Ok(Self { manager })
    }

    /// Send a single notification
    pub async fn send_notification(
        &self,
        request: CreateNotificationRequest,
    ) -> Result<NotificationResponse> {
        self.manager.send_notification(request).await
    }

    /// Send multiple notifications in a batch
    pub async fn send_bulk_notifications(
        &self,
        request: BulkNotificationRequest,
    ) -> Result<BulkNotificationResponse> {
        self.manager.send_bulk_notifications(request).await
    }

    /// Get notification by ID
    pub async fn get_notification(&self, id: &str) -> Result<Option<NotificationResponse>> {
        self.manager.get_notification(id).await
    }

    /// List notifications for a user with optional filtering
    pub async fn list_notifications(
        &self,
        user_id: &str,
        status: Option<NotificationStatus>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<NotificationResponse>> {
        self.manager
            .list_notifications(user_id, status, limit, offset)
            .await
    }

    /// Cancel a pending notification
    pub async fn cancel_notification(&self, id: &str) -> Result<bool> {
        self.manager.cancel_notification(id).await
    }

    /// Create a notification template
    pub async fn create_template(
        &self,
        request: CreateTemplateRequest,
    ) -> Result<NotificationTemplate> {
        self.manager.create_template(request).await
    }

    /// Update a notification template
    pub async fn update_template(
        &self,
        id: &str,
        request: UpdateTemplateRequest,
    ) -> Result<NotificationTemplate> {
        self.manager.update_template(id, request).await
    }

    /// Get template by ID
    pub async fn get_template(&self, id: &str) -> Result<Option<NotificationTemplate>> {
        self.manager.get_template(id).await
    }

    /// List available templates
    pub async fn list_templates(
        &self,
        notification_type: Option<NotificationType>,
        is_active: Option<bool>,
    ) -> Result<Vec<NotificationTemplate>> {
        self.manager
            .list_templates(notification_type, is_active)
            .await
    }

    /// Delete a template
    pub async fn delete_template(&self, id: &str) -> Result<bool> {
        self.manager.delete_template(id).await
    }

    /// Create a notification subscription
    pub async fn create_subscription(
        &self,
        user_id: &str,
        request: CreateSubscriptionRequest,
    ) -> Result<NotificationSubscription> {
        self.manager.create_subscription(user_id, request).await
    }

    /// Update a notification subscription
    pub async fn update_subscription(
        &self,
        id: &str,
        request: UpdateSubscriptionRequest,
    ) -> Result<NotificationSubscription> {
        self.manager.update_subscription(id, request).await
    }

    /// Get subscription by ID
    pub async fn get_subscription(&self, id: &str) -> Result<Option<NotificationSubscription>> {
        self.manager.get_subscription(id).await
    }

    /// List subscriptions for a user
    pub async fn list_user_subscriptions(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationSubscription>> {
        self.manager.list_user_subscriptions(user_id).await
    }

    /// Delete a subscription
    pub async fn delete_subscription(&self, id: &str) -> Result<bool> {
        self.manager.delete_subscription(id).await
    }

    /// Get notification statistics
    pub async fn get_notification_stats(
        &self,
        user_id: Option<&str>,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<NotificationStats> {
        self.manager
            .get_notification_stats(user_id, start_date, end_date)
            .await
    }

    /// Get the notification manager for advanced operations
    pub fn manager(&self) -> &NotificationManager {
        &self.manager
    }

    /// Start the background scheduler for processing notifications
    pub async fn start_scheduler(&self) -> Result<()> {
        self.manager.start_scheduler().await
    }

    /// Stop the background scheduler
    pub async fn stop_scheduler(&self) -> Result<()> {
        self.manager.stop_scheduler().await
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<serde_json::Value> {
        self.manager.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_service_creation() {
        let config = NotificationConfig::default();
        let service = NotificationService::new(config).await;
        assert!(service.is_ok());
    }
}
