//! MCP Protocol Communication Module
//!
//! This module handles Model Context Protocol (MCP) communication, including
//! message serialization/deserialization, protocol validation, and client/server
//! communication patterns.

use crate::{
    models::{McpError, ServerInfo},
    McpError as ServiceError, Result,
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// MCP protocol version
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// MCP JSON-RPC version
pub const JSONRPC_VERSION: &str = "2.0";

/// MCP protocol handler
#[derive(Debug, Clone)]
pub struct McpProtocol {
    /// HTTP client for communication
    client: Client,

    /// Protocol configuration
    config: ProtocolConfig,

    /// Message ID counter
    message_id_counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Retry backoff multiplier
    pub retry_backoff_multiplier: f64,

    /// Enable request/response logging
    pub enable_logging: bool,

    /// Maximum message size in bytes
    pub max_message_size: usize,
}

/// MCP request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version
    pub jsonrpc: String,

    /// Request ID
    pub id: String,

    /// Method name
    pub method: String,

    /// Request parameters
    pub params: Option<Value>,
}

/// MCP response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version
    pub jsonrpc: String,

    /// Request ID
    pub id: String,

    /// Response result (on success)
    pub result: Option<Value>,

    /// Response error (on failure)
    pub error: Option<McpError>,
}

/// MCP notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    /// JSON-RPC version
    pub jsonrpc: String,

    /// Method name
    pub method: String,

    /// Notification parameters
    pub params: Option<Value>,
}

/// Communication result
#[derive(Debug, Clone)]
pub struct CommunicationResult {
    /// Response from server
    pub response: McpResponse,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,
}

/// Standard MCP methods
pub mod methods {
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "initialized";
    pub const PING: &str = "ping";
    pub const LIST_TOOLS: &str = "tools/list";
    pub const CALL_TOOL: &str = "tools/call";
    pub const LIST_RESOURCES: &str = "resources/list";
    pub const READ_RESOURCE: &str = "resources/read";
    pub const SUBSCRIBE_RESOURCE: &str = "resources/subscribe";
    pub const UNSUBSCRIBE_RESOURCE: &str = "resources/unsubscribe";
    pub const LIST_PROMPTS: &str = "prompts/list";
    pub const GET_PROMPT: &str = "prompts/get";
    pub const COMPLETE: &str = "completion/complete";
    pub const SET_LEVEL: &str = "logging/setLevel";
}

/// Standard MCP error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_START: i32 = -32099;
    pub const SERVER_ERROR_END: i32 = -32000;
}

impl McpProtocol {
    /// Create a new MCP protocol handler
    pub fn new(config: ProtocolConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("AI-CORE MCP Manager")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            message_id_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Send a request to an MCP server
    pub async fn send_request(
        &self,
        server: &ServerInfo,
        method: &str,
        params: Option<Value>,
    ) -> Result<CommunicationResult> {
        let request = self.create_request(method, params);
        self.send_request_with_retries(server, request).await
    }

    /// Send a notification to an MCP server (no response expected)
    pub async fn send_notification(
        &self,
        server: &ServerInfo,
        method: &str,
        params: Option<Value>,
    ) -> Result<()> {
        let notification = McpNotification {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
        };

        let url = &server.config.endpoint;

        if self.config.enable_logging {
            debug!(
                server_id = %server.id,
                method = method,
                url = url,
                "Sending MCP notification"
            );
        }

        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.client.post(url).json(&notification).send(),
        )
        .await
        .map_err(|_| ServiceError::Protocol("Request timeout".to_string()))?
        .map_err(|e| ServiceError::Http(e))?;

        if !response.status().is_success() {
            return Err(ServiceError::Protocol(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Initialize connection with an MCP server
    pub async fn initialize_server(&self, server: &ServerInfo) -> Result<Value> {
        let params = serde_json::json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {},
                "logging": {}
            },
            "clientInfo": {
                "name": "AI-CORE MCP Manager",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let result = self
            .send_request(server, methods::INITIALIZE, Some(params))
            .await?;

        // Send initialized notification
        self.send_notification(server, methods::INITIALIZED, None)
            .await?;

        info!(
            server_id = %server.id,
            server_name = %server.name,
            "MCP server initialized successfully"
        );

        result
            .response
            .result
            .ok_or_else(|| ServiceError::Protocol("Initialize response missing result".to_string()))
    }

    /// Ping an MCP server to check connectivity
    pub async fn ping_server(&self, server: &ServerInfo) -> Result<u64> {
        let start_time = std::time::Instant::now();

        let _result = self.send_request(server, methods::PING, None).await?;

        let response_time = start_time.elapsed().as_millis() as u64;

        debug!(
            server_id = %server.id,
            response_time_ms = response_time,
            "MCP server ping successful"
        );

        Ok(response_time)
    }

    /// List available tools from an MCP server
    pub async fn list_tools(&self, server: &ServerInfo) -> Result<Vec<Value>> {
        let result = self.send_request(server, methods::LIST_TOOLS, None).await?;

        let tools = result
            .response
            .result
            .and_then(|r| r.get("tools").cloned())
            .and_then(|t| t.as_array().cloned())
            .unwrap_or_default();

        debug!(
            server_id = %server.id,
            tool_count = tools.len(),
            "Listed tools from MCP server"
        );

        Ok(tools)
    }

    /// Call a tool on an MCP server
    pub async fn call_tool(
        &self,
        server: &ServerInfo,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments.unwrap_or(Value::Object(serde_json::Map::new()))
        });

        let result = self
            .send_request(server, methods::CALL_TOOL, Some(params))
            .await?;

        debug!(
            server_id = %server.id,
            tool_name = tool_name,
            "Called tool on MCP server"
        );

        result
            .response
            .result
            .ok_or_else(|| ServiceError::Protocol("Tool call response missing result".to_string()))
    }

    /// List available resources from an MCP server
    pub async fn list_resources(&self, server: &ServerInfo) -> Result<Vec<Value>> {
        let result = self
            .send_request(server, methods::LIST_RESOURCES, None)
            .await?;

        let resources = result
            .response
            .result
            .and_then(|r| r.get("resources").cloned())
            .and_then(|r| r.as_array().cloned())
            .unwrap_or_default();

        debug!(
            server_id = %server.id,
            resource_count = resources.len(),
            "Listed resources from MCP server"
        );

        Ok(resources)
    }

    /// Read a resource from an MCP server
    pub async fn read_resource(&self, server: &ServerInfo, resource_uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "uri": resource_uri
        });

        let result = self
            .send_request(server, methods::READ_RESOURCE, Some(params))
            .await?;

        debug!(
            server_id = %server.id,
            resource_uri = resource_uri,
            "Read resource from MCP server"
        );

        result.response.result.ok_or_else(|| {
            ServiceError::Protocol("Resource read response missing result".to_string())
        })
    }

    /// List available prompts from an MCP server
    pub async fn list_prompts(&self, server: &ServerInfo) -> Result<Vec<Value>> {
        let result = self
            .send_request(server, methods::LIST_PROMPTS, None)
            .await?;

        let prompts = result
            .response
            .result
            .and_then(|r| r.get("prompts").cloned())
            .and_then(|p| p.as_array().cloned())
            .unwrap_or_default();

        debug!(
            server_id = %server.id,
            prompt_count = prompts.len(),
            "Listed prompts from MCP server"
        );

        Ok(prompts)
    }

    /// Get a prompt from an MCP server
    pub async fn get_prompt(
        &self,
        server: &ServerInfo,
        prompt_name: &str,
        arguments: Option<HashMap<String, Value>>,
    ) -> Result<Value> {
        let params = serde_json::json!({
            "name": prompt_name,
            "arguments": arguments.unwrap_or_default()
        });

        let result = self
            .send_request(server, methods::GET_PROMPT, Some(params))
            .await?;

        debug!(
            server_id = %server.id,
            prompt_name = prompt_name,
            "Got prompt from MCP server"
        );

        result
            .response
            .result
            .ok_or_else(|| ServiceError::Protocol("Prompt get response missing result".to_string()))
    }

    /// Validate an MCP message
    pub fn validate_message(&self, message: &Value) -> Result<()> {
        // Check if it's a valid JSON-RPC message
        if !message.is_object() {
            return Err(ServiceError::Protocol(
                "Message must be an object".to_string(),
            ));
        }

        let obj = message.as_object().unwrap();

        // Check JSON-RPC version
        if let Some(jsonrpc) = obj.get("jsonrpc") {
            if jsonrpc.as_str() != Some(JSONRPC_VERSION) {
                return Err(ServiceError::Protocol(
                    "Invalid JSON-RPC version".to_string(),
                ));
            }
        } else {
            return Err(ServiceError::Protocol(
                "Missing JSON-RPC version".to_string(),
            ));
        }

        // Check message size
        let message_size = serde_json::to_string(message)
            .map_err(|e| ServiceError::Serialization(e))?
            .len();

        if message_size > self.config.max_message_size {
            return Err(ServiceError::Protocol(format!(
                "Message size {} exceeds maximum {}",
                message_size, self.config.max_message_size
            )));
        }

        Ok(())
    }

    // Private helper methods

    fn create_request(&self, method: &str, params: Option<Value>) -> McpRequest {
        let id = self.generate_message_id();

        McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }

    fn generate_message_id(&self) -> String {
        let counter = self
            .message_id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("mcp-{}-{}", Utc::now().timestamp_millis(), counter)
    }

    async fn send_request_with_retries(
        &self,
        server: &ServerInfo,
        request: McpRequest,
    ) -> Result<CommunicationResult> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.send_single_request(server, &request).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        let backoff_ms = (self.config.retry_backoff_multiplier.powi(attempt as i32)
                            * 1000.0) as u64;

                        warn!(
                            server_id = %server.id,
                            attempt = attempt + 1,
                            backoff_ms = backoff_ms,
                            error = %last_error.as_ref().unwrap(),
                            "MCP request failed, retrying"
                        );

                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    async fn send_single_request(
        &self,
        server: &ServerInfo,
        request: &McpRequest,
    ) -> Result<CommunicationResult> {
        let start_time = std::time::Instant::now();
        let url = &server.config.endpoint;

        if self.config.enable_logging {
            debug!(
                server_id = %server.id,
                method = %request.method,
                request_id = %request.id,
                url = url,
                "Sending MCP request"
            );
        }

        // Validate request
        let request_value =
            serde_json::to_value(request).map_err(|e| ServiceError::Serialization(e))?;
        self.validate_message(&request_value)?;

        // Send request
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.client.post(url).json(request).send(),
        )
        .await
        .map_err(|_| ServiceError::Protocol("Request timeout".to_string()))?
        .map_err(|e| ServiceError::Http(e))?;

        let response_time = start_time.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            return Err(ServiceError::Protocol(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // Parse response
        let response_text = response.text().await.map_err(|e| ServiceError::Http(e))?;

        let response_value: Value =
            serde_json::from_str(&response_text).map_err(|e| ServiceError::Serialization(e))?;

        // Validate response
        self.validate_message(&response_value)?;

        let mcp_response: McpResponse =
            serde_json::from_value(response_value).map_err(|e| ServiceError::Serialization(e))?;

        // Check for protocol errors
        if let Some(error) = &mcp_response.error {
            return Err(ServiceError::Protocol(format!(
                "MCP error {}: {}",
                error.code, error.message
            )));
        }

        if self.config.enable_logging {
            debug!(
                server_id = %server.id,
                request_id = %request.id,
                response_time_ms = response_time,
                "MCP request completed successfully"
            );
        }

        Ok(CommunicationResult {
            response: mcp_response,
            response_time_ms: response_time,
            timestamp: Utc::now(),
        })
    }
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            retry_backoff_multiplier: 2.0,
            enable_logging: true,
            max_message_size: 1024 * 1024, // 1MB
        }
    }
}

impl McpRequest {
    /// Create a new MCP request
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
        }
    }
}

impl McpResponse {
    /// Create a successful response
    pub fn success(id: String, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: String, error: McpError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Check if the response is an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

impl McpNotification {
    /// Create a new MCP notification
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServerCapabilities, ServerConfig};

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
                protocol_version: MCP_PROTOCOL_VERSION.to_string(),
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

    #[test]
    fn test_protocol_creation() {
        let config = ProtocolConfig::default();
        let protocol = McpProtocol::new(config);

        assert_eq!(protocol.config.timeout_seconds, 30);
        assert_eq!(protocol.config.max_retries, 3);
    }

    #[test]
    fn test_request_creation() {
        let config = ProtocolConfig::default();
        let protocol = McpProtocol::new(config);

        let request = protocol.create_request("test_method", None);

        assert_eq!(request.jsonrpc, JSONRPC_VERSION);
        assert_eq!(request.method, "test_method");
        assert!(request.id.starts_with("mcp-"));
    }

    #[test]
    fn test_message_validation() {
        let config = ProtocolConfig::default();
        let protocol = McpProtocol::new(config);

        // Valid message
        let valid_message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test",
            "method": "test_method"
        });

        assert!(protocol.validate_message(&valid_message).is_ok());

        // Invalid message (missing jsonrpc)
        let invalid_message = serde_json::json!({
            "id": "test",
            "method": "test_method"
        });

        assert!(protocol.validate_message(&invalid_message).is_err());
    }

    #[test]
    fn test_response_creation() {
        let response =
            McpResponse::success("test-id".to_string(), serde_json::json!({"status": "ok"}));

        assert!(response.is_success());
        assert!(!response.is_error());

        let error_response = McpResponse::error(
            "test-id".to_string(),
            McpError {
                code: error_codes::INTERNAL_ERROR,
                message: "Test error".to_string(),
                data: None,
            },
        );

        assert!(!error_response.is_success());
        assert!(error_response.is_error());
    }

    #[test]
    fn test_message_id_generation() {
        let config = ProtocolConfig::default();
        let protocol = McpProtocol::new(config);

        let id1 = protocol.generate_message_id();
        let id2 = protocol.generate_message_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("mcp-"));
        assert!(id2.starts_with("mcp-"));
    }
}
