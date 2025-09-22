//! Integration implementations for third-party services
//!
//! This module provides concrete implementations for integrating with external services
//! including Zapier, Slack, and GitHub. Each integration provides webhook handling,
//! API client functionality, and event processing capabilities.

pub mod github;
pub mod slack;
pub mod zapier;

use crate::error::IntegrationResult;
use crate::models::{IntegrationEvent, WebhookPayload};
use async_trait::async_trait;

/// Trait defining the common interface for all integrations
#[async_trait]
pub trait Integration: Send + Sync {
    /// Get the integration name
    fn name(&self) -> &'static str;

    /// Process a webhook payload from the integration
    async fn process_webhook(&self, payload: WebhookPayload)
        -> IntegrationResult<IntegrationEvent>;

    /// Validate webhook signature if applicable
    async fn validate_webhook(
        &self,
        payload: &[u8],
        headers: &std::collections::HashMap<String, String>,
    ) -> IntegrationResult<bool>;

    /// Check if the integration is healthy
    async fn health_check(&self) -> IntegrationResult<bool>;

    /// Get supported event types for this integration
    fn supported_events(&self) -> Vec<String>;
}

/// Integration factory for creating integration instances
pub struct IntegrationFactory;

impl IntegrationFactory {
    /// Create a new Zapier integration instance
    pub fn create_zapier(config: &crate::config::ZapierConfig) -> Box<dyn Integration> {
        Box::new(zapier::ZapierIntegration::new(config))
    }

    /// Create a new Slack integration instance
    pub fn create_slack(
        config: &crate::config::SlackConfig,
    ) -> IntegrationResult<Box<dyn Integration>> {
        Ok(Box::new(slack::SlackIntegration::new(config)?))
    }

    /// Create a new GitHub integration instance
    pub fn create_github(
        config: &crate::config::GitHubConfig,
    ) -> IntegrationResult<Box<dyn Integration>> {
        Ok(Box::new(github::GitHubIntegration::new(config)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GitHubConfig, SlackConfig, ZapierConfig};

    #[test]
    fn test_integration_factory_zapier() {
        let config = ZapierConfig::default();
        let integration = IntegrationFactory::create_zapier(&config);
        assert_eq!(integration.name(), "zapier");
    }

    #[tokio::test]
    async fn test_integration_factory_slack() {
        let mut config = SlackConfig::default();
        config.bot_token = Some("xoxb-test-token".to_string());
        config.signing_secret = Some("test-secret".to_string());

        let result = IntegrationFactory::create_slack(&config);
        assert!(result.is_ok());

        let integration = result.unwrap();
        assert_eq!(integration.name(), "slack");
    }

    #[tokio::test]
    async fn test_integration_factory_github() {
        let mut config = GitHubConfig::default();
        config.app_id = Some(12345);
        config.private_key = Some(
            "-----BEGIN RSA PRIVATE KEY-----\ntest-key\n-----END RSA PRIVATE KEY-----".to_string(),
        );
        config.webhook_secret = Some("test-secret".to_string());

        let result = IntegrationFactory::create_github(&config);
        assert!(result.is_ok());

        let integration = result.unwrap();
        assert_eq!(integration.name(), "github");
    }
}
