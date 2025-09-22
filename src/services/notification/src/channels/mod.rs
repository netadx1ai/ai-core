//! Notification channels module
//!
//! This module contains implementations for all notification delivery channels:
//! - Email channel (SMTP)
//! - SMS channel (Twilio/AWS SNS)
//! - Push notification channel (Web Push/FCM)
//! - Webhook channel (HTTP POST)
//! - WebSocket channel (real-time)

use crate::error::Result;
use ai_core_shared::types::NotificationResponse;
use async_trait::async_trait;

pub mod email;
pub mod push;
pub mod sms;
pub mod webhook;
pub mod websocket;

pub use email::EmailChannel;
pub use push::PushChannel;
pub use sms::SmsChannel;
pub use webhook::WebhookChannel;
pub use websocket::WebSocketChannel;

/// Trait that all notification channels must implement
#[async_trait]
pub trait NotificationChannel: Send + Sync + Clone {
    /// Send a notification through this channel
    async fn send_notification(&self, notification: &NotificationResponse) -> Result<()>;

    /// Check if the channel is healthy and ready to send notifications
    async fn health_check(&self) -> Result<bool>;

    /// Get channel-specific delivery information
    fn get_channel_info(&self) -> ChannelInfo;
}

/// Information about a notification channel
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub rate_limit_per_minute: Option<u32>,
    pub supports_retry: bool,
    pub supports_scheduling: bool,
}
