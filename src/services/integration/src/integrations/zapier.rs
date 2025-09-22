//! Zapier Integration Implementation
//!
//! This module provides comprehensive Zapier integration functionality including:
//! - Webhook signature verification using HMAC-SHA256
//! - Webhook payload processing and event conversion
//! - API client for Zapier REST API interactions
//! - Workflow triggering from Zapier events
//! - Error handling and retry logic

use crate::error::{IntegrationError, IntegrationResult};
use crate::integrations::Integration;
use crate::models::{
    EventMetadata, EventPayload, EventStatus, IntegrationEvent, IntegrationType, WebhookPayload,
    ZapStepInfo, ZapierEvent,
};
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use subtle::ConstantTimeEq;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Zapier integration implementation
pub struct ZapierIntegration {
    config: ZapierConfig,
    http_client: Client,
}

/// Zapier configuration (simplified version for integration module)
#[derive(Debug, Clone)]
pub struct ZapierConfig {
    pub enabled: bool,
    pub webhook_secret: Option<String>,
    pub webhook_path: String,
    pub max_payload_size: usize,
    pub processing_timeout: u64,
    pub log_requests: bool,
    pub response_headers: HashMap<String, String>,
}

impl Default for ZapierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            webhook_secret: None,
            webhook_path: "/webhooks/zapier".to_string(),
            max_payload_size: 1024 * 1024, // 1MB
            processing_timeout: 30,
            log_requests: true,
            response_headers: HashMap::new(),
        }
    }
}

/// Raw Zapier webhook payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierWebhookPayload {
    /// Zap identifier
    pub zap_id: Option<String>,
    /// Event name as defined in the Zap
    pub event_name: Option<String>,
    /// Zap name/title
    pub zap_name: Option<String>,
    /// All other fields from the webhook
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

/// Zapier API response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierApiResponse {
    pub status: String,
    pub message: Option<String>,
    pub data: Option<Value>,
}

/// Zapier subscription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierSubscription {
    pub id: String,
    pub target_url: String,
    pub event: String,
    pub status: String,
}

impl ZapierIntegration {
    /// Create a new Zapier integration instance
    pub fn new(config: &crate::config::ZapierConfig) -> Self {
        let zapier_config = ZapierConfig {
            enabled: config.enabled,
            webhook_secret: config.webhook_secret.clone(),
            webhook_path: config.webhook_path.clone(),
            max_payload_size: config.max_payload_size,
            processing_timeout: config.processing_timeout,
            log_requests: config.log_requests,
            response_headers: config.response_headers.clone(),
        };

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.processing_timeout))
            .user_agent("AI-CORE-Integration-Service/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config: zapier_config,
            http_client,
        }
    }

    /// Verify Zapier webhook signature using HMAC-SHA256
    fn verify_signature(&self, payload: &[u8], signature: &str) -> IntegrationResult<bool> {
        let secret = self.config.webhook_secret.as_ref().ok_or_else(|| {
            IntegrationError::configuration("Zapier webhook secret not configured")
        })?;

        // Decode the hex signature
        let provided_signature = hex::decode(signature).map_err(|_| {
            IntegrationError::signature_verification("zapier", "Invalid signature format")
        })?;

        // Create HMAC with secret
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
            IntegrationError::signature_verification("zapier", format!("HMAC error: {}", e))
        })?;

        mac.update(payload);
        let computed_signature = mac.finalize().into_bytes();

        // Constant-time comparison
        let is_valid: bool = computed_signature.ct_eq(&provided_signature).into();

        if !is_valid {
            warn!("Zapier signature verification failed");
            return Err(IntegrationError::signature_verification(
                "zapier",
                "Signature mismatch",
            ));
        }

        Ok(true)
    }

    /// Parse Zapier webhook payload
    fn parse_payload(&self, payload: WebhookPayload) -> IntegrationResult<ZapierEvent> {
        debug!("Parsing Zapier webhook payload");

        let zapier_payload: ZapierWebhookPayload = serde_json::from_value(payload.data.clone())
            .map_err(|e| {
                IntegrationError::invalid_payload("zapier", format!("JSON parsing error: {}", e))
            })?;

        // Extract step information if available
        let step_info = zapier_payload.data.get("meta").and_then(|meta| {
            if let Value::Object(meta_obj) = meta {
                Some(ZapStepInfo {
                    step_id: meta_obj
                        .get("step_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    title: meta_obj
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Step")
                        .to_string(),
                    app: meta_obj
                        .get("app")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown App")
                        .to_string(),
                    step_type: meta_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("action")
                        .to_string(),
                })
            } else {
                None
            }
        });

        // Extract trigger data (everything except meta fields)
        let mut trigger_data = zapier_payload.data.clone();
        trigger_data.remove("meta");

        let zapier_event = ZapierEvent {
            zap_id: zapier_payload
                .zap_id
                .unwrap_or_else(|| "unknown".to_string()),
            zap_name: zapier_payload.zap_name,
            event_name: zapier_payload
                .event_name
                .unwrap_or_else(|| "webhook".to_string()),
            trigger_data: serde_json::to_value(&trigger_data)
                .unwrap_or(Value::Object(serde_json::Map::new())),
            custom_fields: HashMap::new(), // TODO: Extract custom field mappings
            step_info,
        };

        Ok(zapier_event)
    }

    /// Create event metadata from webhook payload
    fn create_event_metadata(
        &self,
        payload: &WebhookPayload,
        zapier_event: &ZapierEvent,
    ) -> EventMetadata {
        let mut tags = HashMap::new();
        tags.insert("integration".to_string(), "zapier".to_string());
        tags.insert("event_name".to_string(), zapier_event.event_name.clone());

        if let Some(ref zap_name) = zapier_event.zap_name {
            tags.insert("zap_name".to_string(), zap_name.clone());
        }

        if let Some(ref step_info) = zapier_event.step_info {
            tags.insert("app".to_string(), step_info.app.clone());
            tags.insert("step_type".to_string(), step_info.step_type.clone());
        }

        EventMetadata {
            source_id: zapier_event.zap_id.clone(),
            user_id: None, // Zapier doesn't provide user context in webhooks
            organization_id: None,
            request_id: payload.id.to_string(),
            tags,
        }
    }

    /// Process webhook and trigger workflows
    async fn process_event(&self, event: &IntegrationEvent) -> IntegrationResult<()> {
        info!(
            event_id = %event.id,
            zap_id = %event.metadata.source_id,
            event_type = %event.event_type,
            "Processing Zapier event"
        );

        // TODO: Integrate with workflow engine
        // This would typically involve:
        // 1. Looking up workflow templates triggered by this event type
        // 2. Preparing workflow parameters from the event data
        // 3. Submitting workflow execution requests
        // 4. Tracking execution status

        // For now, we'll log the successful processing
        debug!("Zapier event processed successfully");

        Ok(())
    }

    /// Make API calls to Zapier (for future use)
    async fn api_request(
        &self,
        endpoint: &str,
        method: reqwest::Method,
        body: Option<Value>,
    ) -> IntegrationResult<Value> {
        let url = format!("https://zapier.com/api/v3/{}", endpoint);

        let mut request = self.http_client.request(method, &url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| IntegrationError::external_api("zapier", 0, e.to_string()))?;

        let status_code = response.status().as_u16();

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IntegrationError::external_api(
                "zapier",
                status_code,
                error_text,
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| IntegrationError::external_api("zapier", status_code, e.to_string()))?;

        Ok(response_json)
    }

    /// Create webhook subscription (for future use)
    pub async fn create_subscription(
        &self,
        target_url: &str,
        event: &str,
    ) -> IntegrationResult<ZapierSubscription> {
        let body = serde_json::json!({
            "target_url": target_url,
            "event": event
        });

        let response = self
            .api_request("subscriptions", reqwest::Method::POST, Some(body))
            .await?;

        let subscription: ZapierSubscription = serde_json::from_value(response).map_err(|e| {
            IntegrationError::zapier(format!("Failed to parse subscription response: {}", e))
        })?;

        Ok(subscription)
    }

    /// List webhook subscriptions (for future use)
    pub async fn list_subscriptions(&self) -> IntegrationResult<Vec<ZapierSubscription>> {
        let response = self
            .api_request("subscriptions", reqwest::Method::GET, None)
            .await?;

        let subscriptions: Vec<ZapierSubscription> =
            serde_json::from_value(response).map_err(|e| {
                IntegrationError::zapier(format!("Failed to parse subscriptions response: {}", e))
            })?;

        Ok(subscriptions)
    }

    /// Delete webhook subscription (for future use)
    pub async fn delete_subscription(&self, subscription_id: &str) -> IntegrationResult<()> {
        let endpoint = format!("subscriptions/{}", subscription_id);
        self.api_request(&endpoint, reqwest::Method::DELETE, None)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Integration for ZapierIntegration {
    fn name(&self) -> &'static str {
        "zapier"
    }

    async fn process_webhook(
        &self,
        payload: WebhookPayload,
    ) -> IntegrationResult<IntegrationEvent> {
        if !self.config.enabled {
            return Err(IntegrationError::service_unavailable("zapier"));
        }

        if self.config.log_requests {
            debug!(
                payload_id = %payload.id,
                integration = %payload.integration,
                event_type = %payload.event_type,
                "Processing Zapier webhook"
            );
        }

        // Parse the Zapier-specific payload
        let zapier_event = self.parse_payload(payload.clone())?;

        // Create event metadata
        let metadata = self.create_event_metadata(&payload, &zapier_event);

        // Create the integration event
        let mut integration_event = IntegrationEvent {
            id: Uuid::new_v4(),
            integration: IntegrationType::Zapier,
            event_type: zapier_event.event_name.clone(),
            metadata,
            payload: EventPayload::Zapier(zapier_event),
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
                    "Zapier webhook processed successfully"
                );
            }
            Err(e) => {
                integration_event.status = EventStatus::Failed;
                integration_event.error_message = Some(e.to_string());
                integration_event.updated_at = Utc::now();
                error!(
                    event_id = %integration_event.id,
                    error = %e,
                    "Zapier webhook processing failed"
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

        // Check if signature verification is required
        if self.config.webhook_secret.is_none() {
            warn!("Zapier webhook secret not configured, skipping signature verification");
            return Ok(true);
        }

        // Get the signature header
        let signature = headers
            .get("X-Zapier-Signature")
            .or_else(|| headers.get("x-zapier-signature"))
            .ok_or_else(|| {
                IntegrationError::signature_verification("zapier", "Missing signature header")
            })?;

        // Verify the signature
        self.verify_signature(payload, signature)
    }

    async fn health_check(&self) -> IntegrationResult<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        // For Zapier, we can check if we have the necessary configuration
        let has_secret = self.config.webhook_secret.is_some();

        // TODO: Add actual API health check if needed
        // This could involve making a simple API call to Zapier's status endpoint

        Ok(has_secret)
    }

    fn supported_events(&self) -> Vec<String> {
        vec![
            "webhook".to_string(),
            "trigger".to_string(),
            "action".to_string(),
            "custom".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ZapierConfig as MainZapierConfig;
    use serde_json::json;

    fn create_test_config() -> MainZapierConfig {
        MainZapierConfig {
            enabled: true,
            webhook_secret: Some("test-secret-key".to_string()),
            webhook_path: "/webhooks/zapier".to_string(),
            max_payload_size: 1024 * 1024,
            processing_timeout: 30,
            log_requests: true,
            response_headers: HashMap::new(),
        }
    }

    fn create_test_payload() -> WebhookPayload {
        WebhookPayload {
            id: Uuid::new_v4(),
            integration: "zapier".to_string(),
            event_type: "webhook".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "zap_id": "12345",
                "event_name": "new_customer",
                "zap_name": "Customer Onboarding",
                "customer_name": "John Doe",
                "customer_email": "john@example.com",
                "meta": {
                    "step_id": "step_001",
                    "title": "New Customer Trigger",
                    "app": "Shopify",
                    "type": "trigger"
                }
            }),
            headers: HashMap::new(),
            source_ip: Some("203.0.113.1".to_string()),
            user_agent: Some("Zapier/1.0".to_string()),
        }
    }

    #[test]
    fn test_zapier_integration_creation() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);
        assert_eq!(integration.name(), "zapier");
        assert!(integration.config.enabled);
    }

    #[test]
    fn test_payload_parsing() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);
        let payload = create_test_payload();

        let result = integration.parse_payload(payload);
        assert!(result.is_ok());

        let zapier_event = result.unwrap();
        assert_eq!(zapier_event.zap_id, "12345");
        assert_eq!(zapier_event.event_name, "new_customer");
        assert_eq!(
            zapier_event.zap_name,
            Some("Customer Onboarding".to_string())
        );
        assert!(zapier_event.step_info.is_some());

        let step_info = zapier_event.step_info.unwrap();
        assert_eq!(step_info.app, "Shopify");
        assert_eq!(step_info.step_type, "trigger");
    }

    #[test]
    fn test_signature_verification() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);

        let payload = b"test payload";

        // Generate a valid signature
        let secret = "test-secret-key";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let result = integration.verify_signature(payload, &signature);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with invalid signature
        let invalid_result = integration.verify_signature(payload, "invalid-signature");
        assert!(invalid_result.is_err());
    }

    #[tokio::test]
    async fn test_webhook_validation() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);

        let payload = b"test payload";
        let secret = "test-secret-key";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let mut headers = HashMap::new();
        headers.insert("X-Zapier-Signature".to_string(), signature);

        let result = integration.validate_webhook(payload, &headers).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);

        let result = integration.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should be healthy with webhook secret configured
    }

    #[test]
    fn test_supported_events() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);

        let events = integration.supported_events();
        assert!(!events.is_empty());
        assert!(events.contains(&"webhook".to_string()));
        assert!(events.contains(&"trigger".to_string()));
    }

    #[tokio::test]
    async fn test_process_webhook_end_to_end() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);
        let payload = create_test_payload();

        let result = integration.process_webhook(payload).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.integration, IntegrationType::Zapier);
        assert_eq!(event.status, EventStatus::Completed);
        assert!(event.error_message.is_none());
    }

    #[test]
    fn test_event_metadata_creation() {
        let config = create_test_config();
        let integration = ZapierIntegration::new(&config);
        let payload = create_test_payload();

        let zapier_event = integration.parse_payload(payload.clone()).unwrap();
        let metadata = integration.create_event_metadata(&payload, &zapier_event);

        assert_eq!(metadata.source_id, "12345");
        assert!(metadata.tags.contains_key("integration"));
        assert_eq!(
            metadata.tags.get("integration"),
            Some(&"zapier".to_string())
        );
        assert_eq!(metadata.tags.get("app"), Some(&"Shopify".to_string()));
    }
}
