//! MCP Manager Service Library
//!
//! This library provides the core functionality for managing Model Context Protocol (MCP) servers
//! in the AI-CORE platform. It handles server registry, lifecycle management, health monitoring,
//! and load balancing for MCP servers.
//!
//! # Features
//!
//! - **Server Registry**: Centralized registry for MCP server instances
//! - **Lifecycle Management**: Start, stop, restart, and monitor MCP servers
//! - **Health Monitoring**: Continuous health checks and automatic recovery
//! - **Load Balancing**: Distribute requests across healthy server instances
//! - **Protocol Communication**: Handle MCP protocol messages and routing
//! - **Integration**: Seamless integration with Intent Parser Service
//!
//! # Architecture
//!
//! The MCP Manager Service follows a modular architecture:
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   API Gateway   │────│  MCP Manager    │────│  MCP Servers    │
//! │                 │    │   Service       │    │                 │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//!                               │
//!                        ┌─────────────────┐
//!                        │ Intent Parser   │
//!                        │   Service       │
//!                        └─────────────────┘
//! ```

use thiserror::Error;

/// MCP Manager Service error types
#[derive(Error, Debug)]
pub enum McpError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Server management error
    #[error("Server management error: {0}")]
    ServerManagement(String),

    /// Protocol communication error
    #[error("Protocol communication error: {0}")]
    Protocol(String),

    /// Health monitoring error
    #[error("Health monitoring error: {0}")]
    HealthMonitoring(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Redis error
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(#[from] validator::ValidationErrors),

    /// Generic error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Anyhow error
    #[error("Error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Type alias for Result with McpError
pub type Result<T> = std::result::Result<T, McpError>;

// Public modules
pub mod client;
pub mod config;
pub mod handlers;
pub mod health;
pub mod load_balancer;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod protocol;
pub mod registry;
pub mod server;
pub mod telemetry;
pub mod utils;

// Re-exports for convenience
pub use config::Config;
pub use health::HealthMonitor;
pub use load_balancer::LoadBalancer;
pub use models::{
    HealthCheck, HealthStatus, McpMessage, RegisterServerRequest, RegisterServerResponse,
    ServerInfo, ServerStatus, UpdateServerRequest,
};
pub use protocol::{McpProtocol, McpRequest, McpResponse};
pub use registry::ServerRegistry;
pub use server::McpManagerServer;
