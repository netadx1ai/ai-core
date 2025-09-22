//! MCP Client Module
//!
//! This module provides client functionality for communicating with MCP servers,
//! including connection management, request handling, and protocol implementation.

use crate::{
    models::ServerInfo,
    protocol::{McpProtocol, ProtocolConfig},
    McpError, Result,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// MCP client for communicating with servers
#[derive(Debug, Clone)]
pub struct McpClient {
    /// Protocol handler
    protocol: Arc<McpProtocol>,

    /// Client configuration
    config: ClientConfig,

    /// Active connections
    connections: Arc<RwLock<std::collections::HashMap<Uuid, ServerConnection>>>,
}

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Default request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum concurrent connections per server
    pub max_connections_per_server: u32,

    /// Enable connection pooling
    pub enable_pooling: bool,

    /// Connection pool size
    pub pool_size: u32,

    /// Enable automatic reconnection
    pub auto_reconnect: bool,

    /// Reconnect delay in seconds
    pub reconnect_delay_seconds: u64,
}

/// Server connection information
#[derive(Debug)]
pub struct ServerConnection {
    /// Server information
    pub server: ServerInfo,

    /// Connection status
    pub status: ConnectionStatus,

    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,

    /// Connection attempts
    pub connection_attempts: u32,

    /// Error count
    pub error_count: u32,
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connection is being established
    Connecting,

    /// Connection is active and healthy
    Connected,

    /// Connection is temporarily unavailable
    Disconnected,

    /// Connection has failed
    Failed,

    /// Connection is being closed
    Closing,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(config: ClientConfig) -> Result<Self> {
        let protocol_config = ProtocolConfig {
            timeout_seconds: config.timeout_seconds,
            max_retries: 3,
            retry_backoff_multiplier: 2.0,
            enable_logging: true,
            max_message_size: 1024 * 1024, // 1MB
        };

        let protocol = Arc::new(McpProtocol::new(protocol_config));

        Ok(Self {
            protocol,
            config,
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Connect to an MCP server
    pub async fn connect(&self, server: ServerInfo) -> Result<()> {
        info!(
            server_id = %server.id,
            server_name = %server.name,
            endpoint = %server.config.endpoint,
            "Connecting to MCP server"
        );

        // Update connection status
        {
            let mut connections = self.connections.write().await;
            connections.insert(
                server.id,
                ServerConnection {
                    server: server.clone(),
                    status: ConnectionStatus::Connecting,
                    last_activity: chrono::Utc::now(),
                    connection_attempts: 0,
                    error_count: 0,
                },
            );
        }

        // Attempt to initialize the server
        match self.protocol.initialize_server(&server).await {
            Ok(_) => {
                // Update connection status to connected
                if let Some(connection) = self.connections.write().await.get_mut(&server.id) {
                    connection.status = ConnectionStatus::Connected;
                    connection.last_activity = chrono::Utc::now();
                }

                info!(
                    server_id = %server.id,
                    "Successfully connected to MCP server"
                );

                Ok(())
            }
            Err(e) => {
                // Update connection status to failed
                if let Some(connection) = self.connections.write().await.get_mut(&server.id) {
                    connection.status = ConnectionStatus::Failed;
                    connection.error_count += 1;
                }

                error!(
                    server_id = %server.id,
                    error = %e,
                    "Failed to connect to MCP server"
                );

                Err(e)
            }
        }
    }

    /// Disconnect from an MCP server
    pub async fn disconnect(&self, server_id: &Uuid) -> Result<()> {
        info!(server_id = %server_id, "Disconnecting from MCP server");

        // Update connection status
        if let Some(connection) = self.connections.write().await.get_mut(server_id) {
            connection.status = ConnectionStatus::Closing;
        }

        // Remove connection
        self.connections.write().await.remove(server_id);

        info!(server_id = %server_id, "Disconnected from MCP server");
        Ok(())
    }

    /// Send a request to an MCP server
    pub async fn send_request(
        &self,
        server_id: &Uuid,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        // Check if we have an active connection
        let server = {
            let connections = self.connections.read().await;
            let connection = connections.get(server_id).ok_or_else(|| {
                McpError::ServerManagement(format!("No connection to server {}", server_id))
            })?;

            if connection.status != ConnectionStatus::Connected {
                return Err(McpError::ServerManagement(format!(
                    "Server {} is not connected",
                    server_id
                )));
            }

            connection.server.clone()
        };

        // Send the request
        debug!(
            server_id = %server_id,
            method = method,
            "Sending MCP request"
        );

        match self.protocol.send_request(&server, method, params).await {
            Ok(result) => {
                // Update last activity
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.last_activity = chrono::Utc::now();
                }

                Ok(result.response.result.unwrap_or(Value::Null))
            }
            Err(e) => {
                // Update error count
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.error_count += 1;
                }

                error!(
                    server_id = %server_id,
                    method = method,
                    error = %e,
                    "MCP request failed"
                );

                Err(e)
            }
        }
    }

    /// Send a notification to an MCP server
    pub async fn send_notification(
        &self,
        server_id: &Uuid,
        method: &str,
        params: Option<Value>,
    ) -> Result<()> {
        // Check if we have an active connection
        let server = {
            let connections = self.connections.read().await;
            let connection = connections.get(server_id).ok_or_else(|| {
                McpError::ServerManagement(format!("No connection to server {}", server_id))
            })?;

            if connection.status != ConnectionStatus::Connected {
                return Err(McpError::ServerManagement(format!(
                    "Server {} is not connected",
                    server_id
                )));
            }

            connection.server.clone()
        };

        // Send the notification
        debug!(
            server_id = %server_id,
            method = method,
            "Sending MCP notification"
        );

        match self
            .protocol
            .send_notification(&server, method, params)
            .await
        {
            Ok(_) => {
                // Update last activity
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.last_activity = chrono::Utc::now();
                }

                Ok(())
            }
            Err(e) => {
                // Update error count
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.error_count += 1;
                }

                error!(
                    server_id = %server_id,
                    method = method,
                    error = %e,
                    "MCP notification failed"
                );

                Err(e)
            }
        }
    }

    /// Ping an MCP server
    pub async fn ping(&self, server_id: &Uuid) -> Result<u64> {
        let server = {
            let connections = self.connections.read().await;
            let connection = connections.get(server_id).ok_or_else(|| {
                McpError::ServerManagement(format!("No connection to server {}", server_id))
            })?;

            connection.server.clone()
        };

        match self.protocol.ping_server(&server).await {
            Ok(response_time) => {
                // Update last activity
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.last_activity = chrono::Utc::now();
                }

                Ok(response_time)
            }
            Err(e) => {
                // Update error count
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.error_count += 1;
                }

                Err(e)
            }
        }
    }

    /// Get connection status for a server
    pub async fn get_connection_status(&self, server_id: &Uuid) -> Option<ConnectionStatus> {
        let connections = self.connections.read().await;
        connections.get(server_id).map(|conn| conn.status)
    }

    /// Get all active connections
    pub async fn get_active_connections(&self) -> Vec<Uuid> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, conn)| conn.status == ConnectionStatus::Connected)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Check connection health
    pub async fn check_connection_health(&self, server_id: &Uuid) -> Result<bool> {
        match self.ping(server_id).await {
            Ok(_) => Ok(true),
            Err(_) => {
                // Mark connection as disconnected
                if let Some(connection) = self.connections.write().await.get_mut(server_id) {
                    connection.status = ConnectionStatus::Disconnected;
                }
                Ok(false)
            }
        }
    }

    /// Reconnect to a server if auto-reconnect is enabled
    pub async fn try_reconnect(&self, server_id: &Uuid) -> Result<()> {
        if !self.config.auto_reconnect {
            return Err(McpError::ServerManagement(
                "Auto-reconnect is disabled".to_string(),
            ));
        }

        let server = {
            let connections = self.connections.read().await;
            let connection = connections.get(server_id).ok_or_else(|| {
                McpError::ServerManagement(format!("No connection record for server {}", server_id))
            })?;

            connection.server.clone()
        };

        warn!(server_id = %server_id, "Attempting to reconnect to MCP server");

        // Wait before reconnecting
        tokio::time::sleep(std::time::Duration::from_secs(
            self.config.reconnect_delay_seconds,
        ))
        .await;

        // Attempt to reconnect
        self.connect(server).await
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_connections_per_server: 10,
            enable_pooling: true,
            pool_size: 20,
            auto_reconnect: true,
            reconnect_delay_seconds: 5,
        }
    }
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Connecting => write!(f, "connecting"),
            ConnectionStatus::Connected => write!(f, "connected"),
            ConnectionStatus::Disconnected => write!(f, "disconnected"),
            ConnectionStatus::Failed => write!(f, "failed"),
            ConnectionStatus::Closing => write!(f, "closing"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServerCapabilities, ServerConfig};
    use std::collections::HashMap;

    fn create_test_server() -> ServerInfo {
        ServerInfo::new(
            "test-server".to_string(),
            "1.0.0".to_string(),
            "test".to_string(),
            ServerConfig {
                endpoint: "http://localhost:8080".to_string(),
                port: 8080,
                host: "localhost".to_string(),
                timeout_seconds: 30,
                max_connections: 100,
                settings: HashMap::new(),
                environment: HashMap::new(),
                auth: None,
                ssl: None,
            },
            ServerCapabilities {
                protocol_version: "2024-11-05".to_string(),
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
                features: Vec::new(),
                max_request_size: None,
                max_response_size: None,
                content_types: Vec::new(),
            },
        )
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = ClientConfig::default();
        let client = McpClient::new(config).unwrap();

        assert_eq!(client.get_active_connections().await.len(), 0);
    }

    #[tokio::test]
    async fn test_connection_status() {
        let config = ClientConfig::default();
        let client = McpClient::new(config).unwrap();
        let server = create_test_server();

        // Initially no connection
        assert_eq!(client.get_connection_status(&server.id).await, None);

        // After failed connection attempt, status should be recorded
        let _ = client.connect(server.clone()).await;
        assert!(client.get_connection_status(&server.id).await.is_some());
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "disconnected");
        assert_eq!(ConnectionStatus::Failed.to_string(), "failed");
    }
}
