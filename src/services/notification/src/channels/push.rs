//! Push notification channel implementation using Web Push and FCM

use crate::channels::{ChannelInfo, NotificationChannel as NotificationChannelTrait};
use crate::config::PushConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::NotificationResponse;
use async_trait::async_trait;
use tracing::{error, info, warn};

/// Push notification channel for sending web push and FCM notifications
#[derive(Clone)]
pub struct PushChannel {
    config: PushConfig,
}

impl PushChannel {
    /// Create a new push channel with the given configuration
    pub async fn new(config: &PushConfig) -> Result<Self> {
        info!("Initializing push channel");

        if !config.enabled {
            return Err(NotificationError::config("Push channel is disabled"));
        }

        info!("Push channel initialized successfully");

        Ok(Self {
            config: config.clone(),
        })
    }

    /// Send push notification via Web Push
    async fn send_web_push(
        &self,
        notification: &NotificationResponse,
        endpoint: &str,
    ) -> Result<()> {
        // Stub implementation - in production this would use the web-push crate
        info!(
            "Sending Web Push notification to {} for notification {}",
            endpoint, notification.id
        );

        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        Ok(())
    }

    /// Send push notification via FCM
    async fn send_fcm(&self, notification: &NotificationResponse, token: &str) -> Result<()> {
        // Stub implementation - in production this would use FCM SDK
        info!(
            "Sending FCM notification to {} for notification {}",
            token, notification.id
        );

        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        Ok(())
    }

    /// Get recipient push endpoints/tokens from user ID
    async fn get_recipient_push_info(&self, recipient_id: &str) -> Result<Vec<String>> {
        // In a real implementation, this would query the database for user's push subscriptions
        // For now, we'll use a placeholder
        warn!(
            "Using placeholder push endpoints for recipient {}",
            recipient_id
        );
        Ok(vec![format!("push_endpoint_{}", recipient_id)])
    }
}

#[async_trait]
impl NotificationChannelTrait for PushChannel {
    async fn send_notification(&self, notification: &NotificationResponse) -> Result<()> {
        info!("Sending push notification: {}", notification.id);

        // Get recipient push endpoints/tokens
        let push_endpoints = self
            .get_recipient_push_info(&notification.recipient_id)
            .await?;

        if push_endpoints.is_empty() {
            return Err(NotificationError::push(
                "No push endpoints found for recipient",
            ));
        }

        // Send to all endpoints
        for endpoint in push_endpoints {
            // For simplicity, assume web push for now
            // In production, you'd determine the type based on endpoint format
            if let Err(e) = self.send_web_push(notification, &endpoint).await {
                error!("Failed to send push notification to {}: {}", endpoint, e);
                // Continue with other endpoints
            }
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        // In production, this would test push service connections
        info!("Push channel health check passed");
        Ok(true)
    }

    fn get_channel_info(&self) -> ChannelInfo {
        ChannelInfo {
            name: "Push".to_string(),
            description: "Web Push and FCM notifications".to_string(),
            enabled: self.config.enabled,
            rate_limit_per_minute: Some(self.config.rate_limit_per_minute),
            supports_retry: true,
            supports_scheduling: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FcmConfig, PushConfig, WebPushConfig};
    use ai_core_shared::types::*;
    use chrono::Utc;

    fn create_test_config() -> PushConfig {
        PushConfig {
            enabled: true,
            web_push: Some(WebPushConfig {
                vapid_subject: "mailto:test@example.com".to_string(),
                vapid_public_key: "test_public_key".to_string(),
                vapid_private_key: "test_private_key".to_string(),
            }),
            fcm: Some(FcmConfig {
                server_key: "test_server_key".to_string(),
                project_id: "test_project".to_string(),
            }),
            timeout_seconds: 30,
            rate_limit_per_minute: 1000,
        }
    }

    fn create_test_notification() -> NotificationResponse {
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "user123".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test Push".to_string(),
            content: "This is a test push notification".to_string(),
            channels: vec![NotificationChannel::Push],
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
    async fn test_push_channel_creation() {
        let config = create_test_config();
        let channel = PushChannel::new(&config).await;
        assert!(channel.is_ok());
    }

    #[tokio::test]
    async fn test_get_recipient_push_info() {
        let config = create_test_config();
        let channel = PushChannel::new(&config).await.unwrap();

        let endpoints = channel.get_recipient_push_info("user123").await.unwrap();
        assert!(!endpoints.is_empty());
    }

    #[tokio::test]
    async fn test_send_notification() {
        let config = create_test_config();
        let channel = PushChannel::new(&config).await.unwrap();
        let notification = create_test_notification();

        let result = channel.send_notification(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_channel_info() {
        let config = create_test_config();
        let channel = PushChannel::new(&config).await.unwrap();
        let info = channel.get_channel_info();

        assert_eq!(info.name, "Push");
        assert!(info.enabled);
        assert!(info.supports_retry);
    }
}
