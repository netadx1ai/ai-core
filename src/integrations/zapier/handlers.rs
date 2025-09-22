//! Zapier Webhook Handlers
//!
//! This module contains the Axum handlers for receiving and processing
//! incoming webhooks from Zapier.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde_json::json;
use tracing::{debug, error, info, warn};

use crate::integrations::zapier::{models::ZapierPayload, security::verify_zapier_signature};
use crate::server::AppState; // Assuming a shared AppState from the API gateway or similar

/// Handles incoming webhooks from Zapier.
///
/// This handler performs the following steps:
/// 1. Verifies the incoming request's signature to ensure it's from Zapier.
/// 2. Deserializes the JSON payload into a structured `ZapierPayload`.
/// 3. Logs the received event for debugging and auditing.
/// 4. (TODO) Passes the event to a processing service or workflow.
/// 5. Returns an appropriate HTTP response to Zapier.
pub async fn zapier_webhook_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    debug!("Received Zapier webhook");

    // Note: The secret key should be stored securely, e.g., in environment variables or a secrets manager.
    // Here, we assume it's available in the application's configuration.
    let zapier_secret = match &state.config.integrations.zapier.secret_key {
        Some(key) => key,
        None => {
            error!("Zapier secret key is not configured in the application state.");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Integration not configured"})),
            )
                .into_response();
        }
    };

    // 1. Verify the signature
    if let Err(e) = verify_zapier_signature(&headers, &body, zapier_secret) {
        warn!("Zapier signature verification failed: {}", e);
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Signature verification failed"})),
        )
            .into_response();
    }
    info!("Zapier signature verified successfully");

    // 2. Deserialize the payload
    let payload: ZapierPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to deserialize Zapier payload: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid payload format"})),
            )
                .into_response();
        }
    };

    info!(zap_id = %payload.zap_id, event_name = %payload.event_name, "Processing Zapier event");

    // 4. (TODO) Process the event
    // This is where you would hand off the payload to a dedicated service,
    // a message queue (like RabbitMQ or Redis Streams), or a Temporal workflow
    // for robust, asynchronous processing.
    //
    // Example:
    // match state.workflow_service.dispatch_zapier_event(payload).await {
    //     Ok(_) => info!("Successfully dispatched Zapier event for processing."),
    //     Err(e) => {
    //         error!("Error dispatching Zapier event: {}", e);
    //         // Returning a 500-level error will signal to Zapier that it should
    //         // retry the webhook delivery according to its retry policy.
    //         return (
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             Json(json!({"error": "Failed to process event"})),
    //         ).into_response();
    //     }
    // }

    // 5. Return a success response to Zapier
    // Acknowledging receipt quickly is crucial.
    (StatusCode::OK, Json(json!({"status": "received"}))).into_response()
}

#[cfg(test)]
mod tests {
    // TODO: Add integration tests for the handler.
    // This will require:
    // - Mocking the AppState and providing a test configuration.
    // - Simulating incoming Axum requests with appropriate headers and a valid/invalid body.
    // - Generating a valid signature for a test payload to verify the success path.
    // - Testing failure paths (e.g., missing signature, invalid signature, bad payload).
}
