//! AI-CORE - Zapier Integration Models
//!
//! This module defines the data structures used for handling incoming webhooks
//! from the Zapier automation platform.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents a generic event payload from Zapier.
///
/// Zapier webhooks can send arbitrary JSON data. This struct is designed to be
/// flexible, capturing a couple of common fields (`zap_id`, `event_name`) and
/// storing the rest of the data in a `HashMap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierPayload {
    /// A unique identifier for the Zap run, useful for tracing.
    pub zap_id: String,
    /// The name of the event, as defined by the user in their Zap.
    pub event_name: String,
    /// A map to capture all other fields from the incoming JSON payload.
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

/// Defines the structure of the response sent back to Zapier upon successful
/// receipt of a webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapierHookResponse {
    /// A unique identifier for the processed request.
    pub request_id: String,
    /// A simple status message.
    pub status: String,
    /// A human-readable message.
    pub message: String,
}
