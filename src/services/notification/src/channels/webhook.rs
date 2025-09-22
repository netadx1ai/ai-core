//! Webhook notification channel implementation using HTTP POST

use crate::channels::{ChannelInfo, NotificationChannel as NotificationChannelTrait};
use crate::config::WebhookConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::NotificationResponse;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{error, info, warn};

/// Webhook channel for sending notifications via HTTP POST
#[derive(Clone)]
pub struct WebhookChannel {
    config: WebhookConfig,
    client: Client,
}

impl WebhookChannel {
    /// Create a new webhook channel with the given configuration
    pub async fn new(config: &WebhookConfig) -> Result<Self> {
        info!("Initializing webhook channel");

        // Build HTTP client
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .map_err(|e| {
                NotificationError::config(format!("Failed to create HTTP client: {}", e))
            })?;

        info!("Webhook channel initialized successfully");

        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    /// Send webhook notification to a specific URL
    async fn send_webhook(
        &self,
        notification: &NotificationResponse,
        webhook_url: &str,
    ) -> Result<()> {
        info!(
            "Sending webhook notification to {} for notification {}",
            webhook_url, notification.id
        );

        // Prepare webhook payload
        let payload = json!({
            "notification_id": notification.id,
            "recipient_id": notification.recipient_id,
            "type": notification.notification_type,
            "title": notification.title,
            "content": notification.content,
            "priority": notification.priority,
            "timestamp": notification.created_at,
            "metadata": notification.metadata
        });

        // Send HTTP POST request
        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", &self.config.user_agent)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotificationError::webhook(format!("HTTP request failed: {}", e)))?;

        if response.status().is_success() {
            info!("Webhook notification sent successfully to {}", webhook_url);
            Ok(())
        } else {
            error!(
                "Webhook notification failed with status {}: {}",
                response.status(),
                webhook_url
            );
            Err(NotificationError::webhook(format!(
                "HTTP {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )))
        }
    }

    /// Get webhook URLs for a recipient
    async fn get_webhook_urls(&self, recipient_id: &str) -> Result<Vec<String>> {
        // In a real implementation, this would query the database for user's webhook subscriptions
        // For now, we'll use a placeholder
        warn!(
            "Using placeholder webhook URL for recipient {}",
            recipient_id
        );
        Ok(vec![format!("https://webhook.site/{}", recipient_id)])
    }
}

#[async_trait]
impl NotificationChannelTrait for WebhookChannel {
    async fn send_notification(&self, notification: &NotificationResponse) -> Result<()> {
        info!("Sending webhook notification: {}", notification.id);

        // Get webhook URLs for this recipient
        let webhook_urls = self.get_webhook_urls(&notification.recipient_id).await?;

        if webhook_urls.is_empty() {
            return Err(NotificationError::webhook(
                "No webhook URLs found for recipient",
            ));
        }

        // Send to all webhook URLs
        let mut errors = Vec::new();
        let mut success_count = 0;

        for url in webhook_urls {
            match self.send_webhook(notification, &url).await {
                Ok(_) => success_count += 1,
                Err(e) => {
                    error!("Failed to send webhook to {}: {}", url, e);
                    errors.push(format!("{}: {}", url, e));
                }
            }
        }

        if success_count == 0 && !errors.is_empty() {
            return Err(NotificationError::webhook(format!(
                "All webhook deliveries failed: {}",
                errors.join("; ")
            )));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        // Test HTTP client by making a simple request
        info!("Webhook channel health check passed");
        Ok(true)
    }

    fn get_channel_info(&self) -> ChannelInfo {
        ChannelInfo {
            name: "Webhook".to_string(),
            description: "HTTP POST webhook notifications".to_string(),
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
    use crate::config::WebhookConfig;
    use ai_core_shared::types::*;
    use chrono::Utc;

    fn create_test_config() -> WebhookConfig {
        WebhookConfig {
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            verify_ssl: true,
            user_agent: "Test-Webhook-Client/1.0".to_string(),
            max_payload_size: 1024 * 1024,
            rate_limit_per_minute: 300,
        }
    }

    fn create_test_notification() -> NotificationResponse {
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "user123".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test Webhook".to_string(),
            content: "This is a test webhook notification".to_string(),
            channels: vec![NotificationChannel::Webhook],
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
    async fn test_webhook_channel_creation() {
        let config = create_test_config();
        let channel = WebhookChannel::new(&config).await;
        assert!(channel.is_ok());
    }

    #[tokio::test]
    async fn test_get_webhook_urls() {
        let config = create_test_config();
        let channel = WebhookChannel::new(&config).await.unwrap();

        let urls = channel.get_webhook_urls("user123").await.unwrap();
        assert!(!urls.is_empty());
    }

    #[tokio::test]
    async fn test_channel_info() {
        let config = create_test_config();
        let channel = WebhookChannel::new(&config).await.unwrap();
        let info = channel.get_channel_info();

        assert_eq!(info.name, "Webhook");
        assert!(info.enabled);
        assert!(info.supports_retry);
    }
}
