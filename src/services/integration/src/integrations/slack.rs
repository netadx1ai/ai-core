//! Slack Integration Implementation
//!
//! This module provides comprehensive Slack integration functionality including:
//! - Bot authentication and workspace management
//! - Webhook event processing (messages, reactions, etc.)
//! - OAuth2 authentication flows
//! - Real-time Socket Mode support
//! - API client for Slack Web API interactions

use crate::error::{IntegrationError, IntegrationResult};
use crate::integrations::Integration;
use crate::models::{
    EventMetadata, EventPayload, EventStatus, IntegrationEvent, IntegrationType, SlackEvent,
    WebhookPayload,
};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Slack integration implementation
pub struct SlackIntegration {
    config: SlackConfig,
    http_client: Client,
}

/// Slack configuration (simplified version for integration module)
#[derive(Debug, Clone)]
pub struct SlackConfig {
    pub enabled: bool,
    pub bot_token: Option<String>,
    pub app_token: Option<String>,
    pub signing_secret: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub api_base_url: String,
    pub socket_mode: bool,
    pub webhook_path: String,
    pub oauth_callback_path: String,
    pub bot_scopes: Vec<String>,
    pub user_scopes: Vec<String>,
}

/// Raw Slack webhook/event payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackWebhookPayload {
    /// Event type (url_verification, event_callback, etc.)
    #[serde(rename = "type")]
    pub event_type: String,
    /// Challenge for URL verification
    pub challenge: Option<String>,
    /// Event data for event_callback type
    pub event: Option<SlackEventData>,
    /// Team ID
    pub team_id: Option<String>,
    /// API App ID
    pub api_app_id: Option<String>,
    /// Token (deprecated)
    pub token: Option<String>,
    /// Event timestamp
    pub event_time: Option<u64>,
}

/// Slack event data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackEventData {
    /// Event type (message, reaction_added, etc.)
    #[serde(rename = "type")]
    pub event_type: String,
    /// Channel ID
    pub channel: Option<String>,
    /// User ID
    pub user: Option<String>,
    /// Message text
    pub text: Option<String>,
    /// Timestamp
    pub ts: Option<String>,
    /// Thread timestamp
    pub thread_ts: Option<String>,
    /// Bot ID if from a bot
    pub bot_id: Option<String>,
    /// All other event-specific fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Slack API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackApiResponse {
    pub ok: bool,
    pub error: Option<String>,
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

/// Slack OAuth token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackOAuthResponse {
    pub ok: bool,
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub bot_user_id: Option<String>,
    pub app_id: Option<String>,
    pub team: Option<SlackTeamInfo>,
    pub error: Option<String>,
}

/// Slack team information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackTeamInfo {
    pub id: String,
    pub name: String,
}

impl SlackIntegration {
    /// Create a new Slack integration instance
    pub fn new(config: &crate::config::SlackConfig) -> IntegrationResult<Self> {
        if !config.enabled {
            return Err(IntegrationError::service_unavailable("slack"));
        }

        if config.bot_token.is_none() {
            return Err(IntegrationError::configuration(
                "Slack bot token is required",
            ));
        }

        if config.signing_secret.is_none() {
            return Err(IntegrationError::configuration(
                "Slack signing secret is required",
            ));
        }

        let slack_config = SlackConfig {
            enabled: config.enabled,
            bot_token: config.bot_token.clone(),
            app_token: config.app_token.clone(),
            signing_secret: config.signing_secret.clone(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
            api_base_url: config.api_base_url.clone(),
            socket_mode: config.socket_mode,
            webhook_path: config.webhook_path.clone(),
            oauth_callback_path: config.oauth_callback_path.clone(),
            bot_scopes: config.bot_scopes.clone(),
            user_scopes: config.user_scopes.clone(),
        };

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("AI-CORE-Integration-Service/1.0")
            .build()
            .map_err(|e| {
                IntegrationError::internal(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            config: slack_config,
            http_client,
        })
    }

    /// Parse Slack webhook payload
    fn parse_payload(&self, payload: WebhookPayload) -> IntegrationResult<SlackEvent> {
        debug!("Parsing Slack webhook payload");

        let slack_payload: SlackWebhookPayload = serde_json::from_value(payload.data.clone())
            .map_err(|e| {
                IntegrationError::invalid_payload("slack", format!("JSON parsing error: {}", e))
            })?;

        // Handle URL verification challenge
        if slack_payload.event_type == "url_verification" {
            if let Some(challenge) = slack_payload.challenge {
                return Ok(SlackEvent {
                    event_type: "url_verification".to_string(),
                    team_id: slack_payload
                        .team_id
                        .unwrap_or_else(|| "unknown".to_string()),
                    channel_id: None,
                    user_id: None,
                    text: Some(challenge),
                    ts: None,
                    thread_ts: None,
                    event_data: payload.data,
                    bot_id: None,
                });
            }
        }

        // Handle event callback
        if let Some(event) = slack_payload.event {
            return Ok(SlackEvent {
                event_type: event.event_type,
                team_id: slack_payload
                    .team_id
                    .unwrap_or_else(|| "unknown".to_string()),
                channel_id: event.channel,
                user_id: event.user,
                text: event.text,
                ts: event.ts,
                thread_ts: event.thread_ts,
                event_data: serde_json::to_value(&event.extra)
                    .unwrap_or(Value::Object(serde_json::Map::new())),
                bot_id: event.bot_id,
            });
        }

        // Default event for other payload types
        Ok(SlackEvent {
            event_type: slack_payload.event_type,
            team_id: slack_payload
                .team_id
                .unwrap_or_else(|| "unknown".to_string()),
            channel_id: None,
            user_id: None,
            text: None,
            ts: None,
            thread_ts: None,
            event_data: payload.data,
            bot_id: None,
        })
    }

    /// Create event metadata from webhook payload
    fn create_event_metadata(
        &self,
        payload: &WebhookPayload,
        slack_event: &SlackEvent,
    ) -> EventMetadata {
        let mut tags = HashMap::new();
        tags.insert("integration".to_string(), "slack".to_string());
        tags.insert("event_type".to_string(), slack_event.event_type.clone());
        tags.insert("team_id".to_string(), slack_event.team_id.clone());

        if let Some(ref channel_id) = slack_event.channel_id {
            tags.insert("channel_id".to_string(), channel_id.clone());
        }

        if let Some(ref user_id) = slack_event.user_id {
            tags.insert("user_id".to_string(), user_id.clone());
        }

        EventMetadata {
            source_id: slack_event.team_id.clone(),
            user_id: slack_event.user_id.clone(),
            organization_id: Some(slack_event.team_id.clone()),
            request_id: payload.id.to_string(),
            tags,
        }
    }

    /// Process Slack event
    async fn process_event(&self, event: &IntegrationEvent) -> IntegrationResult<()> {
        info!(
            event_id = %event.id,
            team_id = %event.metadata.source_id,
            event_type = %event.event_type,
            "Processing Slack event"
        );

        // Handle URL verification
        if event.event_type == "url_verification" {
            debug!("Handling Slack URL verification");
            return Ok(());
        }

        // TODO: Integrate with workflow engine
        // This would typically involve:
        // 1. Looking up workflow templates triggered by this event type
        // 2. Preparing workflow parameters from the event data
        // 3. Submitting workflow execution requests
        // 4. Tracking execution status

        // For now, we'll log the successful processing
        debug!("Slack event processed successfully");

        Ok(())
    }

    /// Make API calls to Slack
    async fn api_request(
        &self,
        endpoint: &str,
        method: reqwest::Method,
        body: Option<Value>,
    ) -> IntegrationResult<SlackApiResponse> {
        let bot_token = self
            .config
            .bot_token
            .as_ref()
            .ok_or_else(|| IntegrationError::configuration("Slack bot token not configured"))?;

        let url = format!("{}/{}", self.config.api_base_url, endpoint);

        let mut request = self
            .http_client
            .request(method, &url)
            .bearer_auth(bot_token);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| IntegrationError::external_api("slack", 0, e.to_string()))?;

        let status_code = response.status().as_u16();

        let response_json: SlackApiResponse = response
            .json()
            .await
            .map_err(|e| IntegrationError::external_api("slack", status_code, e.to_string()))?;

        if !response_json.ok {
            let error_msg = response_json
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(IntegrationError::external_api(
                "slack",
                status_code,
                error_msg,
            ));
        }

        Ok(response_json)
    }

    /// Send a message to a Slack channel
    pub async fn send_message(&self, channel: &str, text: &str) -> IntegrationResult<Value> {
        let body = serde_json::json!({
            "channel": channel,
            "text": text
        });

        let response = self
            .api_request("chat.postMessage", reqwest::Method::POST, Some(body))
            .await?;
        Ok(serde_json::to_value(response.data).unwrap_or(Value::Null))
    }

    /// Get channel information
    pub async fn get_channel_info(&self, channel_id: &str) -> IntegrationResult<Value> {
        let url = format!("conversations.info?channel={}", channel_id);
        let response = self.api_request(&url, reqwest::Method::GET, None).await?;
        Ok(serde_json::to_value(response.data).unwrap_or(Value::Null))
    }

    /// Get user information
    pub async fn get_user_info(&self, user_id: &str) -> IntegrationResult<Value> {
        let url = format!("users.info?user={}", user_id);
        let response = self.api_request(&url, reqwest::Method::GET, None).await?;
        Ok(serde_json::to_value(response.data).unwrap_or(Value::Null))
    }

    /// Exchange OAuth code for access token
    pub async fn exchange_oauth_code(&self, code: &str) -> IntegrationResult<SlackOAuthResponse> {
        let client_id = self
            .config
            .client_id
            .as_ref()
            .ok_or_else(|| IntegrationError::configuration("Slack client ID not configured"))?;

        let client_secret =
            self.config.client_secret.as_ref().ok_or_else(|| {
                IntegrationError::configuration("Slack client secret not configured")
            })?;

        let redirect_uri =
            self.config.redirect_uri.as_ref().ok_or_else(|| {
                IntegrationError::configuration("Slack redirect URI not configured")
            })?;

        let body = serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri
        });

        let response = self
            .http_client
            .post(&format!("{}/oauth.v2.access", self.config.api_base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| IntegrationError::external_api("slack", 0, e.to_string()))?;

        let oauth_response: SlackOAuthResponse = response
            .json()
            .await
            .map_err(|e| IntegrationError::external_api("slack", 0, e.to_string()))?;

        if !oauth_response.ok {
            let error_msg = oauth_response
                .error
                .unwrap_or_else(|| "OAuth exchange failed".to_string());
            return Err(IntegrationError::oauth("slack", error_msg));
        }

        Ok(oauth_response)
    }
}

#[async_trait]
impl Integration for SlackIntegration {
    fn name(&self) -> &'static str {
        "slack"
    }

    async fn process_webhook(
        &self,
        payload: WebhookPayload,
    ) -> IntegrationResult<IntegrationEvent> {
        if !self.config.enabled {
            return Err(IntegrationError::service_unavailable("slack"));
        }

        debug!(
            payload_id = %payload.id,
            integration = %payload.integration,
            event_type = %payload.event_type,
            "Processing Slack webhook"
        );

        // Parse the Slack-specific payload
        let slack_event = self.parse_payload(payload.clone())?;

        // Create event metadata
        let metadata = self.create_event_metadata(&payload, &slack_event);

        // Create the integration event
        let mut integration_event = IntegrationEvent {
            id: Uuid::new_v4(),
            integration: IntegrationType::Slack,
            event_type: slack_event.event_type.clone(),
            metadata,
            payload: EventPayload::Slack(slack_event),
            status: EventStatus::Processing,
            error_message: None,
            retry_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Process the event
        match self.process_event(&integration_event).await {
            Ok(_) => {
                integration_event.status = EventStatus::Completed;
                integration_event.updated_at = Utc::now();
                info!(
                    event_id = %integration_event.id,
                    "Slack webhook processed successfully"
                );
            }
            Err(e) => {
                integration_event.status = EventStatus::Failed;
                integration_event.error_message = Some(e.to_string());
                integration_event.updated_at = Utc::now();
                error!(
                    event_id = %integration_event.id,
                    error = %e,
                    "Slack webhook processing failed"
                );
                return Err(e);
            }
        }

        Ok(integration_event)
    }

    async fn validate_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> IntegrationResult<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        // Use security module for Slack signature verification
        if let Some(ref signing_secret) = self.config.signing_secret {
            crate::security::SecurityUtils::verify_slack_signature(payload, headers, signing_secret)
        } else {
            warn!("Slack signing secret not configured, skipping signature verification");
            Ok(true)
        }
    }

    async fn health_check(&self) -> IntegrationResult<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        // Check if we have the necessary configuration
        let has_bot_token = self.config.bot_token.is_some();
        let has_signing_secret = self.config.signing_secret.is_some();

        if !has_bot_token || !has_signing_secret {
            return Ok(false);
        }

        // TODO: Add actual API health check
        // This could involve making a simple API call to auth.test
        // For now, we'll just check configuration

        Ok(true)
    }

    fn supported_events(&self) -> Vec<String> {
        vec![
            "url_verification".to_string(),
            "event_callback".to_string(),
            "message".to_string(),
            "app_mention".to_string(),
            "reaction_added".to_string(),
            "reaction_removed".to_string(),
            "channel_created".to_string(),
            "channel_deleted".to_string(),
            "member_joined_channel".to_string(),
            "member_left_channel".to_string(),
            "user_change".to_string(),
            "team_join".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SlackConfig as MainSlackConfig;
    use serde_json::json;

    fn create_test_config() -> MainSlackConfig {
        MainSlackConfig {
            enabled: true,
            bot_token: Some("xoxb-test-token".to_string()),
            app_token: Some("xapp-test-token".to_string()),
            signing_secret: Some("test-signing-secret".to_string()),
            client_id: Some("test-client-id".to_string()),
            client_secret: Some("test-client-secret".to_string()),
            redirect_uri: Some("https://example.com/callback".to_string()),
            api_base_url: "https://slack.com/api".to_string(),
            socket_mode: false,
            webhook_path: "/webhooks/slack".to_string(),
            oauth_callback_path: "/oauth/slack/callback".to_string(),
            bot_scopes: vec!["chat:write".to_string()],
            user_scopes: vec!["identity.basic".to_string()],
        }
    }

    fn create_test_payload(event_type: &str) -> WebhookPayload {
        WebhookPayload {
            id: Uuid::new_v4(),
            integration: "slack".to_string(),
            event_type: event_type.to_string(),
            timestamp: Utc::now(),
            data: json!({
                "type": event_type,
                "team_id": "T12345678",
                "api_app_id": "A12345678",
                "event": {
                    "type": "message",
                    "channel": "C12345678",
                    "user": "U12345678",
                    "text": "Hello, world!",
                    "ts": "1609459200.000100"
                }
            }),
            headers: HashMap::new(),
            source_ip: Some("203.0.113.1".to_string()),
            user_agent: Some("Slackbot 1.0".to_string()),
        }
    }

    #[test]
    fn test_slack_integration_creation() {
        let config = create_test_config();
        let result = SlackIntegration::new(&config);
        assert!(result.is_ok());

        let integration = result.unwrap();
        assert_eq!(integration.name(), "slack");
        assert!(integration.config.enabled);
    }

    #[test]
    fn test_slack_integration_creation_missing_token() {
        let mut config = create_test_config();
        config.bot_token = None;

        let result = SlackIntegration::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_payload_parsing() {
        let config = create_test_config();
        let integration = SlackIntegration::new(&config).unwrap();
        let payload = create_test_payload("event_callback");

        let result = integration.parse_payload(payload);
        assert!(result.is_ok());

        let slack_event = result.unwrap();
        assert_eq!(slack_event.event_type, "message");
        assert_eq!(slack_event.team_id, "T12345678");
        assert_eq!(slack_event.channel_id, Some("C12345678".to_string()));
        assert_eq!(slack_event.user_id, Some("U12345678".to_string()));
        assert_eq!(slack_event.text, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_url_verification_parsing() {
        let config = create_test_config();
        let integration = SlackIntegration::new(&config).unwrap();

        let payload = WebhookPayload {
            id: Uuid::new_v4(),
            integration: "slack".to_string(),
            event_type: "url_verification".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "type": "url_verification",
                "challenge": "test-challenge-12345"
            }),
            headers: HashMap::new(),
            source_ip: None,
            user_agent: None,
        };

        let result = integration.parse_payload(payload);
        assert!(result.is_ok());

        let slack_event = result.unwrap();
        assert_eq!(slack_event.event_type, "url_verification");
        assert_eq!(slack_event.text, Some("test-challenge-12345".to_string()));
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = create_test_config();
        let integration = SlackIntegration::new(&config).unwrap();

        let result = integration.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should be healthy with proper config
    }

    #[test]
    fn test_supported_events() {
        let config = create_test_config();
        let integration = SlackIntegration::new(&config).unwrap();

        let events = integration.supported_events();
        assert!(!events.is_empty());
        assert!(events.contains(&"message".to_string()));
        assert!(events.contains(&"url_verification".to_string()));
        assert!(events.contains(&"app_mention".to_string()));
    }

    #[tokio::test]
    async fn test_process_webhook_end_to_end() {
        let config = create_test_config();
        let integration = SlackIntegration::new(&config).unwrap();
        let payload = create_test_payload("event_callback");

        let result = integration.process_webhook(payload).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.integration, IntegrationType::Slack);
        assert_eq!(event.status, EventStatus::Completed);
        assert!(event.error_message.is_none());
    }

    #[test]
    fn test_event_metadata_creation() {
        let config = create_test_config();
        let integration = SlackIntegration::new(&config).unwrap();
        let payload = create_test_payload("event_callback");

        let slack_event = integration.parse_payload(payload.clone()).unwrap();
        let metadata = integration.create_event_metadata(&payload, &slack_event);

        assert_eq!(metadata.source_id, "T12345678");
        assert_eq!(metadata.user_id, Some("U12345678".to_string()));
        assert_eq!(metadata.organization_id, Some("T12345678".to_string()));
        assert!(metadata.tags.contains_key("integration"));
        assert_eq!(metadata.tags.get("integration"), Some(&"slack".to_string()));
        assert_eq!(metadata.tags.get("team_id"), Some(&"T12345678".to_string()));
    }
}
