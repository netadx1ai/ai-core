//! MCP Proxy for the Federation Service
//!
//! This module provides MCP (Model Context Protocol) proxy capabilities for the federation service,
//! enabling seamless integration with client MCP servers, request proxying, connection management,
//! and protocol translation between different MCP server implementations.

use crate::config::ProxyConfig;
use crate::models::FederationError;
use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use reqwest::Client;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

/// MCP proxy for handling client MCP server integration
#[derive(Debug, Clone)]
pub struct McpProxy {
    /// Proxy configuration
    config: ProxyConfig,
    /// HTTP client for making requests
    http_client: Arc<Client>,
    /// Connection pool manager
    connection_pool: Arc<ConnectionPool>,
    /// Request router
    request_router: Arc<RequestRouter>,
    /// Protocol translator
    protocol_translator: Arc<ProtocolTranslator>,
    /// Proxy statistics
    stats: Arc<RwLock<ProxyStats>>,
}

/// Connection pool for managing MCP server connections
#[derive(Debug)]
pub struct ConnectionPool {
    /// Active connections by server ID
    connections: Arc<DashMap<Uuid, Arc<ServerConnection>>>,
    /// Connection configuration
    config: ProxyConfig,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionPoolStats>>,
}

/// Request router for intelligent request routing
#[derive(Debug)]
pub struct RequestRouter {
    /// Routing rules
    routing_rules: Arc<DashMap<String, RoutingRule>>,
    /// Load balancer
    load_balancer: Arc<ProxyLoadBalancer>,
}

/// Protocol translator for MCP protocol compatibility
#[derive(Debug)]
pub struct ProtocolTranslator {
    /// Protocol versions supported
    supported_versions: Vec<String>,
    /// Translation rules
    translation_rules: Arc<DashMap<String, TranslationRule>>,
}

/// Server connection representation
#[derive(Debug)]
pub struct ServerConnection {
    /// Server ID
    pub server_id: Uuid,
    /// Connection URL
    pub url: String,
    /// Connection status
    pub status: Arc<Mutex<ConnectionStatus>>,
    /// Last activity timestamp
    pub last_activity: Arc<Mutex<DateTime<Utc>>>,
    /// Connection metrics
    pub metrics: Arc<Mutex<ConnectionMetrics>>,
}

/// Connection status enumeration
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    /// Connection is active and healthy
    Active,
    /// Connection is idle
    Idle,
    /// Connection is experiencing issues
    Degraded,
    /// Connection is broken
    Broken,
    /// Connection is being established
    Connecting,
}

/// Connection metrics
#[derive(Debug, Clone, Default)]
pub struct ConnectionMetrics {
    /// Total requests made
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time
    pub avg_response_time: f64,
    /// Last request timestamp
    pub last_request: Option<DateTime<Utc>>,
}

/// Routing rule for request routing
#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Rule name
    pub name: String,
    /// Path pattern
    pub path_pattern: String,
    /// Target server ID
    pub target_server_id: Option<Uuid>,
    /// Load balancing strategy
    pub load_balance_strategy: LoadBalanceStrategy,
    /// Rule priority
    pub priority: u32,
}

/// Load balancing strategies for proxy
#[derive(Debug, Clone)]
pub enum LoadBalanceStrategy {
    /// Round robin
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Random selection
    Random,
    /// Weighted distribution
    Weighted(HashMap<Uuid, f64>),
}

/// Translation rule for protocol translation
#[derive(Debug, Clone)]
pub struct TranslationRule {
    /// Source version
    pub source_version: String,
    /// Target version
    pub target_version: String,
    /// Translation function name
    pub translation_fn: String,
}

/// Proxy load balancer
#[derive(Debug)]
pub struct ProxyLoadBalancer {
    /// Current index for round robin
    current_index: Arc<std::sync::atomic::AtomicUsize>,
}

/// Proxy statistics
#[derive(Debug, Clone, Default)]
pub struct ProxyStats {
    /// Total requests proxied
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time
    pub avg_response_time: f64,
    /// Active connections
    pub active_connections: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Connection pool statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionPoolStats {
    /// Total connections created
    pub total_connections: u64,
    /// Active connections
    pub active_connections: u64,
    /// Idle connections
    pub idle_connections: u64,
    /// Connection failures
    pub connection_failures: u64,
    /// Pool utilization
    pub pool_utilization: f64,
}

impl McpProxy {
    /// Create a new MCP proxy
    pub async fn new(config: ProxyConfig) -> Result<Self, FederationError> {
        let http_client = Arc::new(
            Client::builder()
                .timeout(std::time::Duration::from_secs(config.request_timeout))
                .build()
                .map_err(|e| FederationError::InternalError {
                    message: format!("Failed to create HTTP client: {}", e),
                })?,
        );

        let connection_pool = Arc::new(ConnectionPool::new(config.clone()).await?);
        let request_router = Arc::new(RequestRouter::new().await?);
        let protocol_translator = Arc::new(ProtocolTranslator::new().await?);

        Ok(Self {
            config,
            http_client,
            connection_pool,
            request_router,
            protocol_translator,
            stats: Arc::new(RwLock::new(ProxyStats::default())),
        })
    }

    /// Proxy an MCP request
    pub async fn proxy_request(
        &self,
        server_id: &Uuid,
        path: &str,
        method: &str,
        headers: HashMap<String, String>,
        body: Option<serde_json::Value>,
    ) -> Result<ProxyResponse, FederationError> {
        debug!(
            "Proxying {} request to server {} at path {}",
            method, server_id, path
        );

        let start_time = Utc::now();

        // Get server connection
        let connection = self.connection_pool.get_connection(server_id).await?;

        // Route the request
        let target_url = format!("{}{}", connection.url, path);

        // Translate protocol if needed
        let translated_body = if let Some(body) = body {
            self.protocol_translator.translate_request(&body).await?
        } else {
            None
        };

        // Make the request
        let response = self
            .make_request(&target_url, method, headers, translated_body)
            .await?;

        // Update statistics
        let duration = (Utc::now() - start_time).num_milliseconds() as u64;
        self.update_stats(true, duration).await;
        self.connection_pool
            .update_connection_metrics(server_id, true)
            .await?;

        info!(
            "Successfully proxied request to server {} in {}ms",
            server_id, duration
        );

        Ok(response)
    }

    /// Get proxy health information
    pub async fn health(&self) -> Result<serde_json::Value, FederationError> {
        let stats = self.stats.read().await;
        let pool_stats = self.connection_pool.get_stats().await?;

        Ok(serde_json::json!({
            "status": "healthy",
            "proxy": {
                "total_requests": stats.total_requests,
                "successful_requests": stats.successful_requests,
                "failed_requests": stats.failed_requests,
                "success_rate": if stats.total_requests > 0 {
                    (stats.successful_requests as f64 / stats.total_requests as f64) * 100.0
                } else {
                    0.0
                },
                "avg_response_time": stats.avg_response_time
            },
            "connection_pool": {
                "total_connections": pool_stats.total_connections,
                "active_connections": pool_stats.active_connections,
                "idle_connections": pool_stats.idle_connections,
                "pool_utilization": pool_stats.pool_utilization
            }
        }))
    }

    /// Get proxy metrics
    pub async fn metrics(&self) -> Result<serde_json::Value, FederationError> {
        let stats = self.stats.read().await;

        Ok(serde_json::json!({
            "proxy_requests_total": stats.total_requests,
            "proxy_requests_successful": stats.successful_requests,
            "proxy_requests_failed": stats.failed_requests,
            "proxy_avg_response_time": stats.avg_response_time,
            "proxy_active_connections": stats.active_connections
        }))
    }

    /// Stop the proxy
    pub async fn stop(&self) -> Result<(), FederationError> {
        info!("Stopping MCP proxy");
        // Clean up connections and resources
        self.connection_pool.cleanup().await?;
        Ok(())
    }

    // Private helper methods

    async fn make_request(
        &self,
        url: &str,
        method: &str,
        headers: HashMap<String, String>,
        body: Option<serde_json::Value>,
    ) -> Result<ProxyResponse, FederationError> {
        let mut request_builder = match method.to_uppercase().as_str() {
            "GET" => self.http_client.get(url),
            "POST" => self.http_client.post(url),
            "PUT" => self.http_client.put(url),
            "DELETE" => self.http_client.delete(url),
            _ => {
                return Err(FederationError::InternalError {
                    message: format!("Unsupported HTTP method: {}", method),
                })
            }
        };

        // Add headers
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = body {
            request_builder = request_builder.json(&body);
        }

        // Execute request
        let response =
            request_builder
                .send()
                .await
                .map_err(|e| FederationError::ExternalServiceError {
                    service: "mcp_server".to_string(),
                    message: e.to_string(),
                })?;

        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response
            .json::<serde_json::Value>()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));

        Ok(ProxyResponse {
            status_code,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body,
        })
    }

    async fn update_stats(&self, success: bool, duration_ms: u64) {
        let mut stats = self.stats.write().await;

        stats.total_requests += 1;
        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        // Update average response time
        let total_time = stats.avg_response_time * (stats.total_requests - 1) as f64;
        stats.avg_response_time = (total_time + duration_ms as f64) / stats.total_requests as f64;

        stats.last_updated = Utc::now();
    }
}

impl ConnectionPool {
    async fn new(config: ProxyConfig) -> Result<Self, FederationError> {
        Ok(Self {
            connections: Arc::new(DashMap::new()),
            config,
            stats: Arc::new(RwLock::new(ConnectionPoolStats::default())),
        })
    }

    async fn get_connection(
        &self,
        server_id: &Uuid,
    ) -> Result<Arc<ServerConnection>, FederationError> {
        if let Some(connection) = self.connections.get(server_id) {
            Ok(connection.clone())
        } else {
            // Create new connection (stub implementation)
            let connection = Arc::new(ServerConnection {
                server_id: *server_id,
                url: format!("http://localhost:8080/{}", server_id), // Mock URL
                status: Arc::new(Mutex::new(ConnectionStatus::Active)),
                last_activity: Arc::new(Mutex::new(Utc::now())),
                metrics: Arc::new(Mutex::new(ConnectionMetrics::default())),
            });

            self.connections.insert(*server_id, connection.clone());
            Ok(connection)
        }
    }

    async fn update_connection_metrics(
        &self,
        server_id: &Uuid,
        success: bool,
    ) -> Result<(), FederationError> {
        if let Some(connection) = self.connections.get(server_id) {
            {
                let mut metrics = connection.metrics.lock().await;
                metrics.total_requests += 1;
                if success {
                    metrics.successful_requests += 1;
                } else {
                    metrics.failed_requests += 1;
                }
            }
            {
                let mut last_activity = connection.last_activity.lock().await;
                *last_activity = Utc::now();
            }
        }
        Ok(())
    }

    async fn get_stats(&self) -> Result<ConnectionPoolStats, FederationError> {
        Ok(self.stats.read().await.clone())
    }

    async fn cleanup(&self) -> Result<(), FederationError> {
        info!("Cleaning up connection pool");
        self.connections.clear();
        Ok(())
    }
}

impl RequestRouter {
    async fn new() -> Result<Self, FederationError> {
        Ok(Self {
            routing_rules: Arc::new(DashMap::new()),
            load_balancer: Arc::new(ProxyLoadBalancer {
                current_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }),
        })
    }
}

impl ProtocolTranslator {
    async fn new() -> Result<Self, FederationError> {
        Ok(Self {
            supported_versions: vec!["1.0".to_string(), "2.0".to_string()],
            translation_rules: Arc::new(DashMap::new()),
        })
    }

    async fn translate_request(
        &self,
        body: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, FederationError> {
        // Simple pass-through for demo
        // In real implementation, this would perform protocol translation
        Ok(Some(body.clone()))
    }
}

/// Response from proxied request
#[derive(Debug, Clone)]
pub struct ProxyResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_proxy_creation() {
        let config = ProxyConfig::default();
        let proxy = McpProxy::new(config).await.unwrap();
        assert!(proxy.connection_pool.connections.is_empty());
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let config = ProxyConfig::default();
        let pool = ConnectionPool::new(config).await.unwrap();
        assert!(pool.connections.is_empty());

        let server_id = Uuid::new_v4();
        let connection = pool.get_connection(&server_id).await.unwrap();
        assert_eq!(connection.server_id, server_id);
        assert_eq!(pool.connections.len(), 1);
    }

    #[tokio::test]
    async fn test_protocol_translator() {
        let translator = ProtocolTranslator::new().await.unwrap();
        assert_eq!(translator.supported_versions.len(), 2);

        let test_data = serde_json::json!({"test": "value"});
        let result = translator.translate_request(&test_data).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_data);
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            connection_pool_size: 10,
            request_timeout: 30,
            connection_timeout: 10,
            keep_alive: crate::config::KeepAliveConfig {
                enabled: true,
                timeout: 90,
                interval: 30,
            },
            retry: crate::config::RetryConfig {
                max_attempts: 3,
                base_delay: 1000,
                max_delay: 30000,
                backoff_multiplier: 2.0,
                enable_jitter: true,
            },
            circuit_breaker: crate::config::CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 5,
                success_threshold: 3,
                timeout: 60,
                half_open_max_calls: 3,
            },
        }
    }
}
