//! GitHub Integration Implementation
//!
//! This module provides comprehensive GitHub integration functionality including:
//! - GitHub App authentication and repository management
//! - Webhook event processing (push, PR, issues, releases, etc.)
//! - OAuth2 authentication flows for users
//! - API client for GitHub REST API interactions
//! - Repository monitoring and automated actions

use crate::error::{IntegrationError, IntegrationResult};
use crate::integrations::Integration;
use crate::models::{
    EventMetadata, EventPayload, EventStatus, GitHubEvent, GitHubOrganization, GitHubRepository,
    GitHubUser, IntegrationEvent, IntegrationType, WebhookPayload,
};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// GitHub integration implementation
pub struct GitHubIntegration {
    config: GitHubConfig,
    http_client: Client,
}

/// GitHub configuration (simplified version for integration module)
#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub enabled: bool,
    pub app_id: Option<u64>,
    pub private_key: Option<String>,
    pub webhook_secret: Option<String>,
    pub api_base_url: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub webhook_path: String,
    pub oauth_callback_path: String,
    pub default_permissions: Vec<String>,
    pub webhook_events: Vec<String>,
}

/// Raw GitHub webhook payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubWebhookPayload {
    /// Event action (opened, closed, etc.)
    pub action: Option<String>,
    /// Repository information
    pub repository: Option<GitHubRepositoryData>,
    /// Sender information
    pub sender: Option<GitHubUserData>,
    /// Installation ID (for GitHub Apps)
    pub installation: Option<GitHubInstallation>,
    /// Organization information
    pub organization: Option<GitHubOrganizationData>,
    /// All other event-specific fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// GitHub repository data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepositoryData {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub default_branch: String,
    pub private: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// GitHub user data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUserData {
    pub id: u64,
    pub login: String,
    pub avatar_url: String,
    #[serde(rename = "type")]
    pub user_type: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// GitHub organization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOrganizationData {
    pub id: u64,
    pub login: String,
    pub avatar_url: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// GitHub installation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubInstallation {
    pub id: u64,
    pub account: GitHubUserData,
}

/// GitHub API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubApiResponse {
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

/// GitHub OAuth token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOAuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
}

impl GitHubIntegration {
    /// Create a new GitHub integration instance
    pub fn new(config: &crate::config::GitHubConfig) -> IntegrationResult<Self> {
        if !config.enabled {
            return Err(IntegrationError::service_unavailable("github"));
        }

        if config.app_id.is_none() {
            return Err(IntegrationError::configuration("GitHub App ID is required"));
        }

        if config.private_key.is_none() {
            return Err(IntegrationError::configuration(
                "GitHub private key is required",
            ));
        }

        if config.webhook_secret.is_none() {
            return Err(IntegrationError::configuration(
                "GitHub webhook secret is required",
            ));
        }

        let github_config = GitHubConfig {
            enabled: config.enabled,
            app_id: config.app_id,
            private_key: config.private_key.clone(),
            webhook_secret: config.webhook_secret.clone(),
            api_base_url: config.api_base_url.clone(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
            webhook_path: config.webhook_path.clone(),
            oauth_callback_path: config.oauth_callback_path.clone(),
            default_permissions: config.default_permissions.clone(),
            webhook_events: config.webhook_events.clone(),
        };

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("AI-CORE-Integration-Service/1.0")
            .build()
            .map_err(|e| {
                IntegrationError::internal(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            config: github_config,
            http_client,
        })
    }

    /// Parse GitHub webhook payload
    fn parse_payload(&self, payload: WebhookPayload) -> IntegrationResult<GitHubEvent> {
        debug!("Parsing GitHub webhook payload");

        let github_payload: GitHubWebhookPayload = serde_json::from_value(payload.data.clone())
            .map_err(|e| {
                IntegrationError::invalid_payload("github", format!("JSON parsing error: {}", e))
            })?;

        // Extract repository information
        let repository = if let Some(repo_data) = github_payload.repository {
            GitHubRepository {
                id: repo_data.id,
                name: repo_data.name,
                full_name: repo_data.full_name,
                html_url: repo_data.html_url,
                default_branch: repo_data.default_branch,
                private: repo_data.private,
            }
        } else {
            // Default repository for events without repo context
            GitHubRepository {
                id: 0,
                name: "unknown".to_string(),
                full_name: "unknown/unknown".to_string(),
                html_url: "".to_string(),
                default_branch: "main".to_string(),
                private: false,
            }
        };

        // Extract sender information
        let sender = if let Some(sender_data) = github_payload.sender {
            GitHubUser {
                id: sender_data.id,
                login: sender_data.login,
                avatar_url: sender_data.avatar_url,
                user_type: sender_data.user_type,
            }
        } else {
            // Default sender for events without sender context
            GitHubUser {
                id: 0,
                login: "unknown".to_string(),
                avatar_url: "".to_string(),
                user_type: "User".to_string(),
            }
        };

        // Extract organization information
        let organization = github_payload
            .organization
            .map(|org_data| GitHubOrganization {
                id: org_data.id,
                login: org_data.login,
                avatar_url: org_data.avatar_url,
            });

        // Extract installation ID
        let installation_id = github_payload.installation.map(|inst| inst.id);

        let github_event = GitHubEvent {
            action: github_payload.action,
            repository,
            sender,
            installation_id,
            organization,
            event_data: serde_json::to_value(&github_payload.extra)
                .unwrap_or(Value::Object(serde_json::Map::new())),
        };

        Ok(github_event)
    }

    /// Create event metadata from webhook payload
    fn create_event_metadata(
        &self,
        payload: &WebhookPayload,
        github_event: &GitHubEvent,
    ) -> EventMetadata {
        let mut tags = HashMap::new();
        tags.insert("integration".to_string(), "github".to_string());
        tags.insert(
            "repository".to_string(),
            github_event.repository.full_name.clone(),
        );
        tags.insert("sender".to_string(), github_event.sender.login.clone());

        if let Some(ref action) = github_event.action {
            tags.insert("action".to_string(), action.clone());
        }

        if let Some(ref org) = github_event.organization {
            tags.insert("organization".to_string(), org.login.clone());
        }

        if let Some(installation_id) = github_event.installation_id {
            tags.insert("installation_id".to_string(), installation_id.to_string());
        }

        EventMetadata {
            source_id: github_event.repository.full_name.clone(),
            user_id: Some(github_event.sender.login.clone()),
            organization_id: github_event.organization.as_ref().map(|o| o.login.clone()),
            request_id: payload.id.to_string(),
            tags,
        }
    }

    /// Process GitHub event
    async fn process_event(&self, event: &IntegrationEvent) -> IntegrationResult<()> {
        info!(
            event_id = %event.id,
            repository = %event.metadata.source_id,
            event_type = %event.event_type,
            "Processing GitHub event"
        );

        // TODO: Integrate with workflow engine
        // This would typically involve:
        // 1. Looking up workflow templates triggered by this event type
        // 2. Preparing workflow parameters from the event data
        // 3. Submitting workflow execution requests
        // 4. Tracking execution status

        // For now, we'll log the successful processing
        debug!("GitHub event processed successfully");

        Ok(())
    }

    /// Generate JWT token for GitHub App authentication
    fn generate_jwt_token(&self) -> IntegrationResult<String> {
        // TODO: Implement JWT token generation for GitHub App
        // This would require:
        // 1. Parse the private key
        // 2. Create JWT claims with app_id and expiration
        // 3. Sign with RS256 algorithm
        // For now, return a placeholder
        Ok("placeholder-jwt-token".to_string())
    }

    /// Get installation access token
    async fn get_installation_token(&self, installation_id: u64) -> IntegrationResult<String> {
        let jwt_token = self.generate_jwt_token()?;

        let url = format!(
            "{}/app/installations/{}/access_tokens",
            self.config.api_base_url, installation_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(jwt_token)
            .send()
            .await
            .map_err(|e| IntegrationError::external_api("github", 0, e.to_string()))?;

        let status_code = response.status().as_u16();

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IntegrationError::external_api(
                "github",
                status_code,
                error_text,
            ));
        }

        let token_response: Value = response
            .json()
            .await
            .map_err(|e| IntegrationError::external_api("github", status_code, e.to_string()))?;

        let token = token_response
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| IntegrationError::github("Failed to extract access token"))?;

        Ok(token.to_string())
    }

    /// Make API calls to GitHub
    async fn api_request(
        &self,
        endpoint: &str,
        method: reqwest::Method,
        token: &str,
        body: Option<Value>,
    ) -> IntegrationResult<Value> {
        let url = format!("{}/{}", self.config.api_base_url, endpoint);

        let mut request = self.http_client.request(method, &url).bearer_auth(token);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| IntegrationError::external_api("github", 0, e.to_string()))?;

        let status_code = response.status().as_u16();

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IntegrationError::external_api(
                "github",
                status_code,
                error_text,
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| IntegrationError::external_api("github", status_code, e.to_string()))?;

        Ok(response_json)
    }

    /// Get repository information
    pub async fn get_repository(
        &self,
        owner: &str,
        repo: &str,
        installation_id: u64,
    ) -> IntegrationResult<Value> {
        let token = self.get_installation_token(installation_id).await?;
        let endpoint = format!("repos/{}/{}", owner, repo);
        self.api_request(&endpoint, reqwest::Method::GET, &token, None)
            .await
    }

    /// Create an issue comment
    pub async fn create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
        installation_id: u64,
    ) -> IntegrationResult<Value> {
        let token = self.get_installation_token(installation_id).await?;
        let endpoint = format!("repos/{}/{}/issues/{}/comments", owner, repo, issue_number);
        let comment_body = serde_json::json!({
            "body": body
        });
        self.api_request(&endpoint, reqwest::Method::POST, &token, Some(comment_body))
            .await
    }

    /// Update commit status
    pub async fn update_commit_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        state: &str,
        description: &str,
        installation_id: u64,
    ) -> IntegrationResult<Value> {
        let token = self.get_installation_token(installation_id).await?;
        let endpoint = format!("repos/{}/{}/statuses/{}", owner, repo, sha);
        let status_body = serde_json::json!({
            "state": state,
            "description": description,
            "context": "AI-CORE Integration"
        });
        self.api_request(&endpoint, reqwest::Method::POST, &token, Some(status_body))
            .await
    }

    /// Exchange OAuth code for access token
    pub async fn exchange_oauth_code(&self, code: &str) -> IntegrationResult<GitHubOAuthResponse> {
        let client_id =
            self.config.client_id.as_ref().ok_or_else(|| {
                IntegrationError::configuration("GitHub client ID not configured")
            })?;

        let client_secret = self.config.client_secret.as_ref().ok_or_else(|| {
            IntegrationError::configuration("GitHub client secret not configured")
        })?;

        let body = serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code
        });

        let response = self
            .http_client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| IntegrationError::external_api("github", 0, e.to_string()))?;

        let oauth_response: GitHubOAuthResponse = response
            .json()
            .await
            .map_err(|e| IntegrationError::external_api("github", 0, e.to_string()))?;

        Ok(oauth_response)
    }
}

#[async_trait]
impl Integration for GitHubIntegration {
    fn name(&self) -> &'static str {
        "github"
    }

    async fn process_webhook(
        &self,
        payload: WebhookPayload,
    ) -> IntegrationResult<IntegrationEvent> {
        if !self.config.enabled {
            return Err(IntegrationError::service_unavailable("github"));
        }

        debug!(
            payload_id = %payload.id,
            integration = %payload.integration,
            event_type = %payload.event_type,
            "Processing GitHub webhook"
        );

        // Parse the GitHub-specific payload
        let github_event = self.parse_payload(payload.clone())?;

        // Create event metadata
        let metadata = self.create_event_metadata(&payload, &github_event);

        // Create the integration event
        let mut integration_event = IntegrationEvent {
            id: Uuid::new_v4(),
            integration: IntegrationType::GitHub,
            event_type: payload.event_type.clone(),
            metadata,
            payload: EventPayload::GitHub(github_event),
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
                    "GitHub webhook processed successfully"
                );
            }
            Err(e) => {
                integration_event.status = EventStatus::Failed;
                integration_event.error_message = Some(e.to_string());
                integration_event.updated_at = Utc::now();
                error!(
                    event_id = %integration_event.id,
                    error = %e,
                    "GitHub webhook processing failed"
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

        // Use security module for GitHub signature verification
        if let Some(ref webhook_secret) = self.config.webhook_secret {
            crate::security::SecurityUtils::verify_github_signature(
                payload,
                headers,
                webhook_secret,
            )
        } else {
            warn!("GitHub webhook secret not configured, skipping signature verification");
            Ok(true)
        }
    }

    async fn health_check(&self) -> IntegrationResult<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        // Check if we have the necessary configuration
        let has_app_config = self.config.app_id.is_some() && self.config.private_key.is_some();
        let has_webhook_secret = self.config.webhook_secret.is_some();

        if !has_app_config || !has_webhook_secret {
            return Ok(false);
        }

        // TODO: Add actual API health check
        // This could involve generating a JWT token and making a test API call
        // For now, we'll just check configuration

        Ok(true)
    }

    fn supported_events(&self) -> Vec<String> {
        vec![
            "push".to_string(),
            "pull_request".to_string(),
            "issues".to_string(),
            "issue_comment".to_string(),
            "pull_request_review".to_string(),
            "pull_request_review_comment".to_string(),
            "release".to_string(),
            "workflow_run".to_string(),
            "workflow_job".to_string(),
            "check_run".to_string(),
            "check_suite".to_string(),
            "deployment".to_string(),
            "deployment_status".to_string(),
            "repository".to_string(),
            "organization".to_string(),
            "installation".to_string(),
            "installation_repositories".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GitHubConfig as MainGitHubConfig;
    use serde_json::json;

    fn create_test_config() -> MainGitHubConfig {
        MainGitHubConfig {
            enabled: true,
            app_id: Some(12345),
            private_key: Some(
                "-----BEGIN RSA PRIVATE KEY-----\ntest-key\n-----END RSA PRIVATE KEY-----"
                    .to_string(),
            ),
            webhook_secret: Some("test-webhook-secret".to_string()),
            api_base_url: "https://api.github.com".to_string(),
            client_id: Some("test-client-id".to_string()),
            client_secret: Some("test-client-secret".to_string()),
            redirect_uri: Some("https://example.com/callback".to_string()),
            webhook_path: "/webhooks/github".to_string(),
            oauth_callback_path: "/oauth/github/callback".to_string(),
            default_permissions: vec!["contents".to_string(), "issues".to_string()],
            webhook_events: vec!["push".to_string(), "pull_request".to_string()],
        }
    }

    fn create_test_payload(event_type: &str) -> WebhookPayload {
        WebhookPayload {
            id: Uuid::new_v4(),
            integration: "github".to_string(),
            event_type: event_type.to_string(),
            timestamp: Utc::now(),
            data: json!({
                "action": "opened",
                "repository": {
                    "id": 123456789,
                    "name": "test-repo",
                    "full_name": "test-org/test-repo",
                    "html_url": "https://github.com/test-org/test-repo",
                    "default_branch": "main",
                    "private": false
                },
                "sender": {
                    "id": 987654321,
                    "login": "test-user",
                    "avatar_url": "https://avatars.githubusercontent.com/u/987654321",
                    "type": "User"
                },
                "installation": {
                    "id": 12345678,
                    "account": {
                        "id": 987654321,
                        "login": "test-user",
                        "avatar_url": "https://avatars.githubusercontent.com/u/987654321",
                        "type": "User"
                    }
                },
                "pull_request": {
                    "number": 42,
                    "title": "Test PR",
                    "body": "This is a test pull request"
                }
            }),
            headers: HashMap::new(),
            source_ip: Some("192.30.252.1".to_string()),
            user_agent: Some("GitHub-Hookshot/abc123".to_string()),
        }
    }

    #[test]
    fn test_github_integration_creation() {
        let config = create_test_config();
        let result = GitHubIntegration::new(&config);
        assert!(result.is_ok());

        let integration = result.unwrap();
        assert_eq!(integration.name(), "github");
        assert!(integration.config.enabled);
    }

    #[test]
    fn test_github_integration_creation_missing_app_id() {
        let mut config = create_test_config();
        config.app_id = None;

        let result = GitHubIntegration::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_payload_parsing() {
        let config = create_test_config();
        let integration = GitHubIntegration::new(&config).unwrap();
        let payload = create_test_payload("pull_request");

        let result = integration.parse_payload(payload);
        assert!(result.is_ok());

        let github_event = result.unwrap();
        assert_eq!(github_event.action, Some("opened".to_string()));
        assert_eq!(github_event.repository.name, "test-repo");
        assert_eq!(github_event.repository.full_name, "test-org/test-repo");
        assert_eq!(github_event.sender.login, "test-user");
        assert_eq!(github_event.installation_id, Some(12345678));
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = create_test_config();
        let integration = GitHubIntegration::new(&config).unwrap();

        let result = integration.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should be healthy with proper config
    }

    #[test]
    fn test_supported_events() {
        let config = create_test_config();
        let integration = GitHubIntegration::new(&config).unwrap();

        let events = integration.supported_events();
        assert!(!events.is_empty());
        assert!(events.contains(&"push".to_string()));
        assert!(events.contains(&"pull_request".to_string()));
        assert!(events.contains(&"issues".to_string()));
        assert!(events.contains(&"release".to_string()));
    }

    #[tokio::test]
    async fn test_process_webhook_end_to_end() {
        let config = create_test_config();
        let integration = GitHubIntegration::new(&config).unwrap();
        let payload = create_test_payload("pull_request");

        let result = integration.process_webhook(payload).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.integration, IntegrationType::GitHub);
        assert_eq!(event.status, EventStatus::Completed);
        assert!(event.error_message.is_none());
    }

    #[test]
    fn test_event_metadata_creation() {
        let config = create_test_config();
        let integration = GitHubIntegration::new(&config).unwrap();
        let payload = create_test_payload("pull_request");

        let github_event = integration.parse_payload(payload.clone()).unwrap();
        let metadata = integration.create_event_metadata(&payload, &github_event);

        assert_eq!(metadata.source_id, "test-org/test-repo");
        assert_eq!(metadata.user_id, Some("test-user".to_string()));
        assert!(metadata.tags.contains_key("integration"));
        assert_eq!(
            metadata.tags.get("integration"),
            Some(&"github".to_string())
        );
        assert_eq!(
            metadata.tags.get("repository"),
            Some(&"test-org/test-repo".to_string())
        );
        assert_eq!(metadata.tags.get("sender"), Some(&"test-user".to_string()));
        assert_eq!(metadata.tags.get("action"), Some(&"opened".to_string()));
    }
}
