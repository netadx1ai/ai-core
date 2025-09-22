//! Email notification channel implementation using SMTP

use crate::channels::{ChannelInfo, NotificationChannel as NotificationChannelTrait};
use crate::config::EmailConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::types::NotificationResponse;
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::{authentication::Credentials, PoolConfig},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::time::Duration;
use tracing::{error, info, warn};

/// Email channel for sending notifications via SMTP
#[derive(Clone)]
pub struct EmailChannel {
    config: EmailConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_mailbox: Mailbox,
}

impl EmailChannel {
    /// Create a new email channel with the given configuration
    pub async fn new(config: &EmailConfig) -> Result<Self> {
        info!("Initializing email channel");

        // Parse from address
        let from_mailbox = format!("{} <{}>", config.from_name, config.from_email)
            .parse::<Mailbox>()
            .map_err(|e| NotificationError::config(format!("Invalid from email address: {}", e)))?;

        // Build SMTP transport
        let mut transport_builder = if config.smtp_use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host).map_err(|e| {
                NotificationError::config(format!("Failed to create SMTP relay: {}", e))
            })?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
        };

        transport_builder = transport_builder.port(config.smtp_port);

        // Add credentials if provided
        if !config.smtp_username.is_empty() && !config.smtp_password.is_empty() {
            let creds =
                Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
            transport_builder = transport_builder.credentials(creds);
        }

        // Configure connection pooling
        transport_builder =
            transport_builder.pool_config(PoolConfig::new().max_size(10).min_idle(2));

        // Set timeout
        transport_builder =
            transport_builder.timeout(Some(Duration::from_secs(config.timeout_seconds)));

        let transport = transport_builder.build();

        info!("Email channel initialized successfully");

        Ok(Self {
            config: config.clone(),
            transport,
            from_mailbox,
        })
    }

    /// Build an email message from notification data
    fn build_message(
        &self,
        notification: &NotificationResponse,
        recipient_email: &str,
    ) -> Result<Message> {
        let to_mailbox = recipient_email
            .parse::<Mailbox>()
            .map_err(|e| NotificationError::email(format!("Invalid recipient email: {}", e)))?;

        let mut message_builder = Message::builder()
            .from(self.from_mailbox.clone())
            .to(to_mailbox)
            .subject(&notification.title);

        // Add reply-to if configured
        if let Some(ref reply_to) = self.config.reply_to {
            let reply_to_mailbox = reply_to
                .parse::<Mailbox>()
                .map_err(|e| NotificationError::email(format!("Invalid reply-to email: {}", e)))?;
            message_builder = message_builder.reply_to(reply_to_mailbox);
        }

        // Set content type based on content
        let message = if notification.content.contains("<html>")
            || notification.content.contains("<p>")
        {
            message_builder
                .header(ContentType::TEXT_HTML)
                .body(notification.content.clone())
        } else {
            message_builder
                .header(ContentType::TEXT_PLAIN)
                .body(notification.content.clone())
        }
        .map_err(|e| NotificationError::email(format!("Failed to build email message: {}", e)))?;

        Ok(message)
    }

    /// Get recipient email address from user ID
    async fn get_recipient_email(&self, recipient_id: &str) -> Result<String> {
        // In a real implementation, this would query the database to get the user's email
        // For now, we'll assume the recipient_id is an email address or use a placeholder
        if recipient_id.contains('@') {
            Ok(recipient_id.to_string())
        } else {
            // This is a placeholder - in production you'd query the user database
            warn!(
                "Recipient ID '{}' is not an email address, using placeholder",
                recipient_id
            );
            Ok(format!("{}@example.com", recipient_id))
        }
    }
}

#[async_trait]
impl NotificationChannelTrait for EmailChannel {
    async fn send_notification(&self, notification: &NotificationResponse) -> Result<()> {
        info!("Sending email notification: {}", notification.id);

        // Get recipient email address
        let recipient_email = self.get_recipient_email(&notification.recipient_id).await?;

        // Build email message
        let message = self.build_message(notification, &recipient_email)?;

        // Send email
        match self.transport.send(message).await {
            Ok(_response) => {
                info!(
                    "Email sent successfully: {} to {}",
                    notification.id, recipient_email
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to send email {}: {}", notification.id, e);
                Err(NotificationError::email(format!("SMTP error: {}", e)))
            }
        }
    }

    async fn health_check(&self) -> Result<bool> {
        // Test SMTP connection
        match self.transport.test_connection().await {
            Ok(is_connected) => {
                if is_connected {
                    info!("Email channel health check passed");
                    Ok(true)
                } else {
                    warn!("Email channel health check failed: not connected");
                    Ok(false)
                }
            }
            Err(e) => {
                error!("Email channel health check error: {}", e);
                Ok(false)
            }
        }
    }

    fn get_channel_info(&self) -> ChannelInfo {
        ChannelInfo {
            name: "Email".to_string(),
            description: "SMTP email notifications".to_string(),
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
    use crate::config::EmailConfig;
    use ai_core_shared::types::*;
    use chrono::Utc;

    fn create_test_config() -> EmailConfig {
        EmailConfig {
            enabled: true,
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "test".to_string(),
            smtp_password: "test".to_string(),
            smtp_use_tls: false,
            smtp_use_starttls: true,
            from_email: "test@example.com".to_string(),
            from_name: "Test Service".to_string(),
            reply_to: None,
            max_recipients_per_message: 50,
            timeout_seconds: 30,
            rate_limit_per_minute: 100,
        }
    }

    fn create_test_notification() -> NotificationResponse {
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "user@example.com".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test Notification".to_string(),
            content: "This is a test notification".to_string(),
            channels: vec![NotificationChannel::Email],
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
    async fn test_email_channel_creation() {
        let config = create_test_config();
        let channel = EmailChannel::new(&config).await;
        assert!(channel.is_ok());
    }

    #[tokio::test]
    async fn test_build_message() {
        let config = create_test_config();
        let channel = EmailChannel::new(&config).await.unwrap();
        let notification = create_test_notification();

        let message = channel.build_message(&notification, "recipient@example.com");
        assert!(message.is_ok());
    }

    #[tokio::test]
    async fn test_get_recipient_email() {
        let config = create_test_config();
        let channel = EmailChannel::new(&config).await.unwrap();

        // Test with email address
        let email = channel
            .get_recipient_email("user@example.com")
            .await
            .unwrap();
        assert_eq!(email, "user@example.com");

        // Test with user ID
        let email = channel.get_recipient_email("user123").await.unwrap();
        assert_eq!(email, "user123@example.com");
    }

    #[tokio::test]
    async fn test_channel_info() {
        let config = create_test_config();
        let channel = EmailChannel::new(&config).await.unwrap();
        let info = channel.get_channel_info();

        assert_eq!(info.name, "Email");
        assert!(info.enabled);
        assert!(info.supports_retry);
        assert!(info.supports_scheduling);
    }
}
