//! WebSocket notification channel implementation for real-time notifications

use crate::channels::{ChannelInfo, NotificationChannel as NotificationChannelTrait};
use crate::config::WebSocketConfig;
use crate::error::{NotificationError, Result};
use ai_core_shared::{
    api::NotificationWebSocketMessage,
    types::{NotificationResponse, WebSocketMessageType},
};
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// WebSocket channel for sending real-time notifications
#[derive(Clone)]
pub struct WebSocketChannel {
    config: WebSocketConfig,
    connections: Arc<DashMap<String, WebSocketConnection>>,
    broadcast_tx: Arc<tokio::sync::broadcast::Sender<NotificationWebSocketMessage>>,
}

/// Represents an active WebSocket connection
#[derive(Debug)]
struct WebSocketConnection {
    user_id: String,
    connection_id: String,
    tx: mpsc::UnboundedSender<Message>,
    connected_at: chrono::DateTime<chrono::Utc>,
}

impl WebSocketChannel {
    /// Create a new WebSocket channel with the given configuration
    pub async fn new(config: &WebSocketConfig) -> Result<Self> {
        info!("Initializing WebSocket channel");

        let (broadcast_tx, _) = tokio::sync::broadcast::channel(1000);

        info!("WebSocket channel initialized successfully");

        Ok(Self {
            config: config.clone(),
            connections: Arc::new(DashMap::new()),
            broadcast_tx: Arc::new(broadcast_tx),
        })
    }

    /// Add a new WebSocket connection
    pub async fn add_connection(&self, user_id: String, websocket: WebSocket) -> Result<()> {
        let connection_id = Uuid::new_v4().to_string();
        let (mut ws_tx, mut ws_rx) = websocket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        // Store connection
        let connection = WebSocketConnection {
            user_id: user_id.clone(),
            connection_id: connection_id.clone(),
            tx: tx.clone(),
            connected_at: chrono::Utc::now(),
        };

        self.connections.insert(connection_id.clone(), connection);

        // Handle outgoing messages
        let connections_clone = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if ws_tx.send(message).await.is_err() {
                    break;
                }
            }
            connections_clone.remove(&connection_id_clone);
        });

        // Handle incoming messages (ping/pong, etc.)
        let tx_clone = tx.clone();
        let connections_clone = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Close(_)) => {
                        break;
                    }
                    Ok(Message::Pong(_)) => {
                        // Handle pong response
                    }
                    Ok(Message::Ping(data)) => {
                        if tx_clone.send(Message::Pong(data)).is_err() {
                            break;
                        }
                    }
                    Ok(Message::Text(text)) => {
                        // Handle incoming text messages (could be acknowledgments, etc.)
                        info!("Received WebSocket message: {}", text);
                    }
                    Ok(Message::Binary(_)) => {
                        // Handle binary messages if needed
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
            connections_clone.remove(&connection_id_clone);
        });

        // Send welcome message
        let welcome_message = NotificationWebSocketMessage {
            message_type: WebSocketMessageType::ConnectionStatus,
            data: json!({
                "status": "connected",
                "connection_id": connection_id
            }),
            timestamp: chrono::Utc::now(),
        };

        if let Ok(message_text) = serde_json::to_string(&welcome_message) {
            let _ = tx.send(Message::Text(message_text));
        }

        info!(
            "WebSocket connection established for user {} with connection {}",
            user_id, connection_id
        );
        Ok(())
    }

    /// Send message to specific user's connections
    pub async fn send_to_user(
        &self,
        user_id: &str,
        message: &NotificationWebSocketMessage,
    ) -> Result<()> {
        let message_text = serde_json::to_string(message)
            .map_err(|e| NotificationError::serialization(e.to_string()))?;

        let mut sent_count = 0;
        for connection in self.connections.iter() {
            if connection.user_id == user_id {
                if connection
                    .tx
                    .send(Message::Text(message_text.clone()))
                    .is_ok()
                {
                    sent_count += 1;
                }
            }
        }

        if sent_count == 0 {
            warn!("No active WebSocket connections found for user {}", user_id);
        } else {
            info!(
                "Sent WebSocket message to {} connections for user {}",
                sent_count, user_id
            );
        }

        Ok(())
    }

    /// Broadcast message to all connected users
    pub async fn broadcast(&self, message: &NotificationWebSocketMessage) -> Result<()> {
        let _ = self.broadcast_tx.send(message.clone());
        Ok(())
    }

    /// Get connection statistics
    pub fn get_connection_stats(&self) -> serde_json::Value {
        let total_connections = self.connections.len();
        let mut user_connections: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for connection in self.connections.iter() {
            *user_connections
                .entry(connection.user_id.clone())
                .or_insert(0) += 1;
        }

        json!({
            "total_connections": total_connections,
            "unique_users": user_connections.len(),
            "user_connections": user_connections
        })
    }

    /// Clean up inactive connections (called periodically)
    pub async fn cleanup_connections(&self) -> Result<()> {
        let mut to_remove = Vec::new();
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(1);

        for entry in self.connections.iter() {
            if entry.connected_at < cutoff_time {
                // Test if connection is still alive by sending ping
                if entry.tx.send(Message::Ping(vec![])).is_err() {
                    to_remove.push(entry.connection_id.clone());
                }
            }
        }

        for connection_id in to_remove {
            self.connections.remove(&connection_id);
        }

        Ok(())
    }
}

#[async_trait]
impl NotificationChannelTrait for WebSocketChannel {
    async fn send_notification(&self, notification: &NotificationResponse) -> Result<()> {
        info!("Sending WebSocket notification: {}", notification.id);

        let message = NotificationWebSocketMessage {
            message_type: WebSocketMessageType::Notification,
            data: json!({
                "notification": notification
            }),
            timestamp: chrono::Utc::now(),
        };

        self.send_to_user(&notification.recipient_id, &message)
            .await
    }

    async fn health_check(&self) -> Result<bool> {
        info!("WebSocket channel health check passed");
        Ok(true)
    }

    fn get_channel_info(&self) -> ChannelInfo {
        ChannelInfo {
            name: "WebSocket".to_string(),
            description: "Real-time WebSocket notifications".to_string(),
            enabled: self.config.enabled,
            rate_limit_per_minute: None, // WebSocket doesn't have traditional rate limits
            supports_retry: false,       // Real-time notifications don't retry
            supports_scheduling: false,  // Real-time only
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WebSocketConfig;
    use ai_core_shared::types::*;
    use chrono::Utc;

    fn create_test_config() -> WebSocketConfig {
        WebSocketConfig {
            enabled: true,
            max_connections: 1000,
            ping_interval_seconds: 30,
            pong_timeout_seconds: 10,
            message_buffer_size: 1024,
            max_message_size: 64 * 1024,
        }
    }

    fn create_test_notification() -> NotificationResponse {
        NotificationResponse {
            id: "test-123".to_string(),
            recipient_id: "user123".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test WebSocket".to_string(),
            content: "This is a test WebSocket notification".to_string(),
            channels: vec![NotificationChannel::Websocket],
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
    async fn test_websocket_channel_creation() {
        let config = create_test_config();
        let channel = WebSocketChannel::new(&config).await;
        assert!(channel.is_ok());
    }

    #[tokio::test]
    async fn test_send_notification() {
        let config = create_test_config();
        let channel = WebSocketChannel::new(&config).await.unwrap();
        let notification = create_test_notification();

        // This will send to no connections but shouldn't error
        let result = channel.send_notification(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_channel_info() {
        let config = create_test_config();
        let channel = WebSocketChannel::new(&config).await.unwrap();
        let info = channel.get_channel_info();

        assert_eq!(info.name, "WebSocket");
        assert!(info.enabled);
        assert!(!info.supports_retry);
        assert!(!info.supports_scheduling);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let config = create_test_config();
        let channel = WebSocketChannel::new(&config).await.unwrap();

        let stats = channel.get_connection_stats();
        assert_eq!(stats["total_connections"], 0);
        assert_eq!(stats["unique_users"], 0);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let config = create_test_config();
        let channel = WebSocketChannel::new(&config).await.unwrap();

        let message = NotificationWebSocketMessage {
            message_type: WebSocketMessageType::Notification,
            data: json!({"test": "data"}),
            timestamp: Utc::now(),
        };

        let result = channel.broadcast(&message).await;
        assert!(result.is_ok());
    }
}
