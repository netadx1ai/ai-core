//! # AI-CORE Integration Service
//!
//! This service provides comprehensive third-party API integrations for the AI-CORE platform,
//! including Zapier webhook handling, Slack bot integration, and GitHub repository integration.
//!
//! ## Features
//!
//! - **Zapier Integration**: Webhook handling with signature verification and workflow triggers
//! - **Slack Integration**: Bot functionality, workspace management, and real-time messaging
//! - **GitHub Integration**: Repository events, workflow triggers, and automated actions
//! - **Security**: OAuth2 flows, signature verification, and secure token management
//! - **Observability**: Comprehensive metrics, logging, and health monitoring
//! - **Reliability**: Circuit breakers, retry logic, and graceful degradation
//!
//! ## Architecture
//!
//! The service is built using Axum for HTTP handling, with separate modules for each
//! integration type. All integrations support:
//!
//! - Webhook signature verification
//! - Rate limiting and circuit breaking
//! - Event routing to workflow engine
//! - Comprehensive error handling and logging
//! - Metrics collection and health monitoring
//!
//! ## Usage
//!
//! ```rust
//! use integration_service::{IntegrationService, IntegrationConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = IntegrationConfig::from_env()?;
//!     let service = IntegrationService::new(config).await?;
//!     service.start().await?;
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod handlers;
pub mod integrations;
pub mod metrics;
pub mod models;
pub mod security;
pub mod service;
pub mod utils;
pub mod webhook;

// Re-export main types for easier usage
pub use config::{GitHubConfig, IntegrationConfig, SlackConfig, ZapierConfig};
pub use error::{IntegrationError, IntegrationResult};
pub use models::{
    EventMetadata, GitHubEvent, IntegrationEvent, SlackEvent, WebhookPayload, ZapierEvent,
};
pub use service::IntegrationService;
pub use webhook::{
    EventPriority, EventRouter, EventStorage, WebhookConfig, WebhookError, WebhookEvent,
    WebhookEventStatus, WebhookHandler, WebhookProcessor, WebhookResult, WebhookStats,
};

/// Version information for the integration service
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVICE_NAME: &str = "integration-service";

/// Health check information
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub service: String,
    pub version: String,
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub integrations: std::collections::HashMap<String, bool>,
}

impl HealthStatus {
    pub fn healthy() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
            version: VERSION.to_string(),
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            integrations: std::collections::HashMap::new(),
        }
    }

    pub fn with_integration_status(mut self, name: &str, healthy: bool) -> Self {
        self.integrations.insert(name.to_string(), healthy);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(SERVICE_NAME, "integration-service");
    }

    #[test]
    fn test_health_status() {
        let health = HealthStatus::healthy()
            .with_integration_status("zapier", true)
            .with_integration_status("slack", false);

        assert_eq!(health.service, "integration-service");
        assert_eq!(health.status, "healthy");
        assert_eq!(health.integrations.get("zapier"), Some(&true));
        assert_eq!(health.integrations.get("slack"), Some(&false));
    }
}
