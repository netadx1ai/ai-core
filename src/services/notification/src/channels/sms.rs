//! SMS notification channel implementation using Twilio or AWS SNS

use crate::channels::{ChannelInfo, NotificationChannel as NotificationChannelTrait};
use crate::config::SmsConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::NotificationResponse;
use async_trait::async_trait;
use tracing::{info, warn};

/// SMS channel for sending notifications via Twilio or AWS SNS
#[derive(Clone)]
pub struct SmsChannel {
    config: SmsConfig,
}

impl SmsChannel {
    /// Create a new SMS channel with the given configuration
    pub async fn new(config: &SmsConfig) -> Result<Self> {
        info!("Initializing SMS channel");

        // Validate configuration
        if !config.enabled {
            return Err(NotificationError::config("SMS channel is disabled"));
        }

        info!("SMS channel initialized successfully");

        Ok(Self {
            config: config.clone(),
        })
    }

    /// Send SMS via Twilio
    async fn send_via_twilio(
        &self,
        notification: &NotificationResponse,
        phone: &str,
    ) -> Result<()> {
        // Stub implementation - in production this would use the Twilio API
        info!(
            "Sending SMS via Twilio to {} for notification {}",
            phone, notification.id
        );

        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// Send SMS via AWS SNS
    async fn send_via_aws_sns(
        &self,
        notification: &NotificationResponse,
        phone: &str,
    ) -> Result<()> {
        // Stub implementation - in production this would use AWS SNS SDK
        info!(
            "Sending SMS via AWS SNS to {} for notification {}",
            phone, notification.id
        );

        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        Ok(())
    }

    /// Get recipient phone number from user ID
    async fn get_recipient_phone(&self, recipient_id: &str) -> Result<String> {
        // In a real implementation, this would query the database to get the user's phone
        // For now, we'll use a placeholder
        if recipient_id.starts_with('+') || recipient_id.chars().all(|c| c.is_ascii_digit()) {
            Ok(recipient_id.to_string())
        } else {
            warn!(
                "Recipient ID '{}' is not a phone number, using placeholder",
                recipient_id
            );
            Ok("+1234567890".to_string())
        }
    }
}

#[async_trait]
impl NotificationChannelTrait for SmsChannel {
    async fn send_notification(&self, notification: &NotificationResponse) -> Result<()> {
        info!("Sending SMS notification: {}", notification.id);

        // Get recipient phone number
        let recipient_phone = self.get_recipient_phone(&notification.recipient_id).await?;

        // Send SMS based on configured provider
        match self.config.provider {
            crate::config::SmsProvider::Twilio => {
                self.send_via_twilio(notification, &recipient_phone).await
            }
            crate::config::SmsProvider::AwsSns => {
                self.send_via_aws_sns(notification, &recipient_phone).await
            }
        }
    }

    async fn health_check(&self) -> Result<bool> {
        // In production, this would test the SMS provider connection
        info!("SMS channel health check passed");
        Ok(true)
    }

    fn get_channel_info(&self) -> ChannelInfo {
        ChannelInfo {
            name: "SMS".to_string(),
            description: format!("SMS notifications via {:?}", self.config.provider),
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
    use crate::config::{SmsConfig, SmsProvider};
    use ai_core_shared::types::*;
    use chrono::Utc;

    fn create_test_config() -> SmsConfig {
        SmsConfig {
            enabled: true,
            provider: SmsProvider::Twilio,
            twilio: None,
            aws_sns: None,
            timeout_seconds: 30,
            rate_limit_per_minute: 60,
        }
    }

    fn create_test_notification() -> NotificationResponse {
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "+1234567890".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test SMS".to_string(),
            content: "This is a test SMS".to_string(),
            channels: vec![NotificationChannel::Sms],
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
    async fn test_sms_channel_creation() {
        let config = create_test_config();
        let channel = SmsChannel::new(&config).await;
        assert!(channel.is_ok());
    }

    #[tokio::test]
    async fn test_get_recipient_phone() {
        let config = create_test_config();
        let channel = SmsChannel::new(&config).await.unwrap();

        let phone = channel.get_recipient_phone("+1234567890").await.unwrap();
        assert_eq!(phone, "+1234567890");

        let phone = channel.get_recipient_phone("1234567890").await.unwrap();
        assert_eq!(phone, "1234567890");
    }

    #[tokio::test]
    async fn test_send_notification() {
        let config = create_test_config();
        let channel = SmsChannel::new(&config).await.unwrap();
        let notification = create_test_notification();

        let result = channel.send_notification(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_channel_info() {
        let config = create_test_config();
        let channel = SmsChannel::new(&config).await.unwrap();
        let info = channel.get_channel_info();

        assert_eq!(info.name, "SMS");
        assert!(info.enabled);
        assert!(info.supports_retry);
    }
}
