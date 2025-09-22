//! WebSocket manager module for real-time notification delivery
//!
//! This module provides WebSocket connection management for real-time notifications:
//! - WebSocket connection lifecycle management
//! - User session tracking
//! - Real-time notification broadcasting
//! - Connection pooling and cleanup
//! - Message routing and delivery

use crate::error::{NotificationError, Result};
use ai_core_shared::{api::NotificationWebSocketMessage, types::WebSocketMessageType};

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

/// WebSocket connection manager
#[derive(Clone)]
pub struct WebSocketManager {
    /// Active WebSocket connections indexed by connection ID
    connections: Arc<DashMap<String, WebSocketConnection>>,
    /// User to connection mapping for quick lookups
    user_connections: Arc<DashMap<String, Vec<String>>>,
    /// Broadcast channel for system-wide messages
    broadcast_tx: Arc<broadcast::Sender<NotificationWebSocketMessage>>,
    /// Message statistics
    stats: Arc<WebSocketStats>,
}

/// Represents an active WebSocket connection
#[derive(Debug)]
struct WebSocketConnection {
    connection_id: String,
    user_id: String,
    sender: mpsc::UnboundedSender<Message>,
    connected_at: chrono::DateTime<chrono::Utc>,
    last_ping: chrono::DateTime<chrono::Utc>,
}

/// WebSocket connection statistics
#[derive(Debug, Default)]
struct WebSocketStats {
    total_connections: std::sync::atomic::AtomicU64,
    messages_sent: std::sync::atomic::AtomicU64,
    messages_received: std::sync::atomic::AtomicU64,
    connection_errors: std::sync::atomic::AtomicU64,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub async fn new() -> Result<Self> {
        info!("Initializing WebSocket manager");

        let (broadcast_tx, _) = broadcast::channel(1000);

        let manager = Self {
            connections: Arc::new(DashMap::new()),
            user_connections: Arc::new(DashMap::new()),
            broadcast_tx: Arc::new(broadcast_tx),
            stats: Arc::new(WebSocketStats::default()),
        };

        info!("WebSocket manager initialized successfully");
        Ok(manager)
    }

    /// Handle a new WebSocket connection
    pub async fn handle_connection(&self, user_id: String, websocket: WebSocket) -> Result<()> {
        let connection_id = Uuid::new_v4().to_string();
        info!(
            "Handling new WebSocket connection: {} for user: {}",
            connection_id, user_id
        );

        let (mut ws_sender, mut ws_receiver) = websocket.split();
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Message>();

        // Create connection record
        let connection = WebSocketConnection {
            connection_id: connection_id.clone(),
            user_id: user_id.clone(),
            sender: msg_tx.clone(),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
        };

        // Store connection
        self.connections.insert(connection_id.clone(), connection);

        // Update user connections mapping
        self.user_connections
            .entry(user_id.clone())
            .or_insert_with(Vec::new)
            .push(connection_id.clone());

        // Update stats
        self.stats
            .total_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Send welcome message
        let welcome_msg = NotificationWebSocketMessage {
            message_type: WebSocketMessageType::ConnectionStatus,
            data: json!({
                "status": "connected",
                "connection_id": &connection_id,
                "timestamp": chrono::Utc::now()
            }),
            timestamp: chrono::Utc::now(),
        };

        if let Ok(msg_text) = serde_json::to_string(&welcome_msg) {
            let _ = msg_tx.send(Message::Text(msg_text));
        }

        // Clone references for tasks
        let connections = self.connections.clone();
        let user_connections = self.user_connections.clone();
        let stats = self.stats.clone();
        let connection_id_clone = connection_id.clone();
        let user_id_clone = user_id.clone();

        // Handle outgoing messages
        let outgoing_task = {
            let connection_id = connection_id.clone();
            let connections = connections.clone();
            let stats = stats.clone();

            tokio::spawn(async move {
                while let Some(message) = msg_rx.recv().await {
                    if let Err(e) = ws_sender.send(message).await {
                        error!(
                            "Failed to send WebSocket message for connection {}: {}",
                            connection_id, e
                        );
                        stats
                            .connection_errors
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    stats
                        .messages_sent
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }

                // Clean up connection on task completion
                connections.remove(&connection_id);
                info!(
                    "Outgoing message task completed for connection: {}",
                    connection_id
                );
            })
        };

        // Handle incoming messages
        let incoming_task = {
            let connection_id = connection_id.clone();
            let connections = connections.clone();
            let user_connections = user_connections.clone();
            let stats = stats.clone();
            let msg_tx = msg_tx.clone();

            tokio::spawn(async move {
                while let Some(msg_result) = ws_receiver.next().await {
                    match msg_result {
                        Ok(Message::Close(_)) => {
                            info!("WebSocket connection closed by client: {}", connection_id);
                            break;
                        }
                        Ok(Message::Pong(_)) => {
                            // Update last ping time
                            if let Some(mut conn) = connections.get_mut(&connection_id) {
                                conn.last_ping = chrono::Utc::now();
                            }
                        }
                        Ok(Message::Ping(data)) => {
                            if msg_tx.send(Message::Pong(data)).is_err() {
                                break;
                            }
                        }
                        Ok(Message::Text(text)) => {
                            info!(
                                "Received WebSocket text message from {}: {}",
                                connection_id, text
                            );
                            stats
                                .messages_received
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                            // Handle incoming message (could be acknowledgments, etc.)
                            if let Err(e) =
                                Self::handle_incoming_message(&connection_id, &text).await
                            {
                                warn!("Failed to handle incoming message: {}", e);
                            }
                        }
                        Ok(Message::Binary(_)) => {
                            // Handle binary messages if needed
                            stats
                                .messages_received
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(e) => {
                            error!("WebSocket error for connection {}: {}", connection_id, e);
                            stats
                                .connection_errors
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                    }
                }

                // Clean up on task completion
                Self::cleanup_connection(
                    &connections,
                    &user_connections,
                    &connection_id_clone,
                    &user_id_clone,
                )
                .await;
                info!(
                    "Incoming message task completed for connection: {}",
                    connection_id
                );
            })
        };

        // Wait for either task to complete
        tokio::select! {
            _ = outgoing_task => {},
            _ = incoming_task => {},
        }

        info!(
            "WebSocket connection handler completed for user: {}",
            user_id
        );
        Ok(())
    }

    /// Send a message to a specific user's connections
    pub async fn send_to_user(
        &self,
        user_id: &str,
        message: &NotificationWebSocketMessage,
    ) -> Result<usize> {
        let message_text = serde_json::to_string(message)
            .map_err(|e| NotificationError::serialization(e.to_string()))?;

        let mut sent_count = 0;

        if let Some(connection_ids) = self.user_connections.get(user_id) {
            for connection_id in connection_ids.iter() {
                if let Some(connection) = self.connections.get(connection_id) {
                    if connection
                        .sender
                        .send(Message::Text(message_text.clone()))
                        .is_ok()
                    {
                        sent_count += 1;
                    }
                }
            }
        }

        if sent_count == 0 {
            warn!(
                "No active WebSocket connections found for user: {}",
                user_id
            );
        } else {
            info!(
                "Sent WebSocket message to {} connections for user: {}",
                sent_count, user_id
            );
        }

        Ok(sent_count)
    }

    /// Broadcast a message to all connected users
    pub async fn broadcast(&self, message: &NotificationWebSocketMessage) -> Result<usize> {
        let message_text = serde_json::to_string(message)
            .map_err(|e| NotificationError::serialization(e.to_string()))?;

        let mut sent_count = 0;

        for connection in self.connections.iter() {
            if connection
                .sender
                .send(Message::Text(message_text.clone()))
                .is_ok()
            {
                sent_count += 1;
            }
        }

        // Also send via broadcast channel
        let _ = self.broadcast_tx.send(message.clone());

        info!("Broadcasted message to {} connections", sent_count);
        Ok(sent_count)
    }

    /// Send a message to specific connections
    pub async fn send_to_connections(
        &self,
        connection_ids: &[String],
        message: &NotificationWebSocketMessage,
    ) -> Result<usize> {
        let message_text = serde_json::to_string(message)
            .map_err(|e| NotificationError::serialization(e.to_string()))?;

        let mut sent_count = 0;

        for connection_id in connection_ids {
            if let Some(connection) = self.connections.get(connection_id) {
                if connection
                    .sender
                    .send(Message::Text(message_text.clone()))
                    .is_ok()
                {
                    sent_count += 1;
                }
            }
        }

        Ok(sent_count)
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> serde_json::Value {
        json!({
            "total_connections": self.connections.len(),
            "unique_users": self.user_connections.len(),
            "messages_sent": self.stats.messages_sent.load(std::sync::atomic::Ordering::Relaxed),
            "messages_received": self.stats.messages_received.load(std::sync::atomic::Ordering::Relaxed),
            "connection_errors": self.stats.connection_errors.load(std::sync::atomic::Ordering::Relaxed),
        })
    }

    /// Get connections for a specific user
    pub fn get_user_connections(&self, user_id: &str) -> Vec<String> {
        self.user_connections
            .get(user_id)
            .map(|connections| connections.clone())
            .unwrap_or_default()
    }

    /// Check if a user has active connections
    pub fn has_active_connections(&self, user_id: &str) -> bool {
        self.user_connections.contains_key(user_id)
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self) -> usize {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::minutes(30);
        let mut to_remove = Vec::new();

        for entry in self.connections.iter() {
            if entry.last_ping < cutoff_time {
                to_remove.push((entry.connection_id.clone(), entry.user_id.clone()));
            }
        }

        let removed_count = to_remove.len();
        for (connection_id, user_id) in to_remove {
            Self::cleanup_connection(
                &self.connections,
                &self.user_connections,
                &connection_id,
                &user_id,
            )
            .await;
        }

        if removed_count > 0 {
            info!("Cleaned up {} stale WebSocket connections", removed_count);
        }

        removed_count
    }

    /// Send ping to all connections
    pub async fn ping_all_connections(&self) -> usize {
        let mut pinged_count = 0;

        for connection in self.connections.iter() {
            if connection.sender.send(Message::Ping(vec![])).is_ok() {
                pinged_count += 1;
            }
        }

        info!("Sent ping to {} connections", pinged_count);
        pinged_count
    }

    /// Get detailed connection information
    pub fn get_connection_details(&self) -> serde_json::Value {
        let mut user_details = serde_json::Map::new();

        for entry in self.user_connections.iter() {
            let user_id = entry.key();
            let connection_ids = entry.value();

            let mut connections = Vec::new();
            for connection_id in connection_ids {
                if let Some(connection) = self.connections.get(connection_id) {
                    connections.push(json!({
                        "connection_id": connection.connection_id,
                        "connected_at": connection.connected_at,
                        "last_ping": connection.last_ping
                    }));
                }
            }

            user_details.insert(
                user_id.clone(),
                json!({
                    "connection_count": connections.len(),
                    "connections": connections
                }),
            );
        }

        json!({
            "total_connections": self.connections.len(),
            "unique_users": self.user_connections.len(),
            "user_details": user_details,
            "stats": self.get_stats()
        })
    }

    // Private helper methods

    async fn handle_incoming_message(connection_id: &str, message: &str) -> Result<()> {
        // Parse and handle incoming messages
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(message) {
            if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                match msg_type {
                    "ping" => {
                        info!("Received ping from connection: {}", connection_id);
                    }
                    "ack" => {
                        info!("Received acknowledgment from connection: {}", connection_id);
                    }
                    "subscribe" => {
                        info!(
                            "Received subscription request from connection: {}",
                            connection_id
                        );
                    }
                    _ => {
                        info!(
                            "Received unknown message type '{}' from connection: {}",
                            msg_type, connection_id
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn cleanup_connection(
        connections: &Arc<DashMap<String, WebSocketConnection>>,
        user_connections: &Arc<DashMap<String, Vec<String>>>,
        connection_id: &str,
        user_id: &str,
    ) {
        // Remove connection
        connections.remove(connection_id);

        // Update user connections mapping
        if let Some(mut user_conns) = user_connections.get_mut(user_id) {
            user_conns.retain(|id| id != connection_id);
            if user_conns.is_empty() {
                drop(user_conns);
                user_connections.remove(user_id);
            }
        }

        info!(
            "Cleaned up WebSocket connection: {} for user: {}",
            connection_id, user_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let manager = WebSocketManager::new().await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let manager = WebSocketManager::new().await.unwrap();
        let stats = manager.get_stats();

        assert_eq!(stats["total_connections"], 0);
        assert_eq!(stats["unique_users"], 0);
        assert_eq!(stats["messages_sent"], 0);
        assert_eq!(stats["messages_received"], 0);
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let manager = WebSocketManager::new().await.unwrap();

        let message = NotificationWebSocketMessage {
            message_type: WebSocketMessageType::Notification,
            data: json!({"test": "data"}),
            timestamp: Utc::now(),
        };

        let result = manager.broadcast(&message).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // No connections yet
    }

    #[tokio::test]
    async fn test_has_active_connections() {
        let manager = WebSocketManager::new().await.unwrap();
        assert!(!manager.has_active_connections("user123"));
    }

    #[tokio::test]
    async fn test_cleanup_stale_connections() {
        let manager = WebSocketManager::new().await.unwrap();
        let cleaned = manager.cleanup_stale_connections().await;
        assert_eq!(cleaned, 0); // No connections to clean
    }

    #[tokio::test]
    async fn test_ping_all_connections() {
        let manager = WebSocketManager::new().await.unwrap();
        let pinged = manager.ping_all_connections().await;
        assert_eq!(pinged, 0); // No connections to ping
    }

    #[tokio::test]
    async fn test_get_user_connections() {
        let manager = WebSocketManager::new().await.unwrap();
        let connections = manager.get_user_connections("user123");
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_get_connection_details() {
        let manager = WebSocketManager::new().await.unwrap();
        let details = manager.get_connection_details();

        assert_eq!(details["total_connections"], 0);
        assert_eq!(details["unique_users"], 0);
        assert!(details["user_details"].is_object());
    }

    #[tokio::test]
    async fn test_send_to_user_no_connections() {
        let manager = WebSocketManager::new().await.unwrap();

        let message = NotificationWebSocketMessage {
            message_type: WebSocketMessageType::Notification,
            data: json!({"test": "data"}),
            timestamp: Utc::now(),
        };

        let result = manager.send_to_user("nonexistent_user", &message).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_handle_incoming_message() {
        let result = WebSocketManager::handle_incoming_message(
            "test_connection",
            r#"{"type": "ping", "data": {}}"#,
        )
        .await;
        assert!(result.is_ok());
    }
}
