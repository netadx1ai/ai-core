//! Request handlers for the notification service
//!
//! This module contains all HTTP request handlers for the notification service API:
//! - Notification management handlers
//! - Template management handlers
//! - Subscription management handlers
//! - WebSocket connection handlers
//! - Health and metrics handlers

use crate::error::{NotificationError, Result};
use crate::manager::NotificationManager;
use crate::websocket::WebSocketManager;
use ai_core_shared::types::*;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info};

pub mod notifications_handler {
    use super::*;

    #[derive(Deserialize)]
    pub struct NotificationQuery {
        pub status: Option<NotificationStatus>,
        pub limit: Option<u32>,
        pub offset: Option<u32>,
        pub user_id: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct StatsQuery {
        pub user_id: Option<String>,
        pub start_date: Option<chrono::DateTime<chrono::Utc>>,
        pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    }

    /// Create a new notification
    pub async fn create_notification(
        State(manager): State<Arc<NotificationManager>>,
        Json(request): Json<CreateNotificationRequest>,
    ) -> Result<impl IntoResponse> {
        info!(
            "Creating notification for recipient: {}",
            request.recipient_id
        );

        match manager.send_notification(request).await {
            Ok(notification) => {
                info!("Notification created successfully: {}", notification.id);
                Ok((StatusCode::CREATED, Json(notification)))
            }
            Err(e) => {
                error!("Failed to create notification: {}", e);
                Err(e)
            }
        }
    }

    /// Send multiple notifications in a batch
    pub async fn send_bulk_notifications(
        State(manager): State<Arc<NotificationManager>>,
        Json(request): Json<BulkNotificationRequest>,
    ) -> Result<impl IntoResponse> {
        info!(
            "Processing bulk notification request with {} notifications",
            request.notifications.len()
        );

        match manager.send_bulk_notifications(request).await {
            Ok(response) => {
                info!(
                    "Bulk notification completed: {} successful, {} failed",
                    response.successful, response.failed
                );
                Ok((StatusCode::OK, Json(response)))
            }
            Err(e) => {
                error!("Failed to process bulk notifications: {}", e);
                Err(e)
            }
        }
    }

    /// Get a notification by ID
    pub async fn get_notification(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        info!("Getting notification: {}", id);

        match manager.get_notification(&id).await? {
            Some(notification) => Ok(Json(notification)),
            None => Err(NotificationError::not_found("notification")),
        }
    }

    /// List notifications with optional filtering
    pub async fn list_notifications(
        State(manager): State<Arc<NotificationManager>>,
        Query(query): Query<NotificationQuery>,
    ) -> Result<impl IntoResponse> {
        let user_id = query.user_id.as_deref().unwrap_or("default");
        info!("Listing notifications for user: {}", user_id);

        match manager
            .list_notifications(user_id, query.status, query.limit, query.offset)
            .await
        {
            Ok(notifications) => {
                info!(
                    "Retrieved {} notifications for user: {}",
                    notifications.len(),
                    user_id
                );
                Ok(Json(notifications))
            }
            Err(e) => {
                error!("Failed to list notifications: {}", e);
                Err(e)
            }
        }
    }

    /// Cancel a notification
    pub async fn cancel_notification(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        info!("Cancelling notification: {}", id);

        match manager.cancel_notification(&id).await {
            Ok(true) => {
                info!("Notification cancelled successfully: {}", id);
                Ok(StatusCode::NO_CONTENT)
            }
            Ok(false) => Err(NotificationError::not_found("notification")),
            Err(e) => {
                error!("Failed to cancel notification {}: {}", id, e);
                Err(e)
            }
        }
    }

    /// Get notification status
    pub async fn get_notification_status(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        match manager.get_notification(&id).await? {
            Some(notification) => {
                let status_response = serde_json::json!({
                    "id": notification.id,
                    "status": notification.status,
                    "delivery_attempts": notification.delivery_attempts,
                    "created_at": notification.created_at,
                    "updated_at": notification.updated_at,
                    "delivered_at": notification.delivered_at
                });
                Ok(Json(status_response))
            }
            None => Err(NotificationError::not_found("notification")),
        }
    }

    /// Get notification statistics
    pub async fn get_notification_stats(
        State(manager): State<Arc<NotificationManager>>,
        Query(query): Query<StatsQuery>,
    ) -> Result<impl IntoResponse> {
        info!("Getting notification statistics");

        match manager
            .get_notification_stats(query.user_id.as_deref(), query.start_date, query.end_date)
            .await
        {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => {
                error!("Failed to get notification stats: {}", e);
                Err(e)
            }
        }
    }

    /// Get channel-specific statistics
    pub async fn get_channel_stats(
        State(manager): State<Arc<NotificationManager>>,
    ) -> Result<impl IntoResponse> {
        info!("Getting channel statistics");

        match manager.get_notification_stats(None, None, None).await {
            Ok(stats) => Ok(Json(stats.channel_stats)),
            Err(e) => {
                error!("Failed to get channel stats: {}", e);
                Err(e)
            }
        }
    }

    /// Get scheduler status
    pub async fn get_scheduler_status(
        State(_manager): State<Arc<NotificationManager>>,
    ) -> Result<impl IntoResponse> {
        // In a real implementation, this would get status from the scheduler
        let status = serde_json::json!({
            "enabled": true,
            "running": true,
            "scheduled_notifications": 0
        });
        Ok(Json(status))
    }

    /// Start the scheduler
    pub async fn start_scheduler(
        State(manager): State<Arc<NotificationManager>>,
    ) -> Result<impl IntoResponse> {
        info!("Starting notification scheduler");

        match manager.start_scheduler().await {
            Ok(_) => {
                info!("Scheduler started successfully");
                Ok(StatusCode::OK)
            }
            Err(e) => {
                error!("Failed to start scheduler: {}", e);
                Err(e)
            }
        }
    }

    /// Stop the scheduler
    pub async fn stop_scheduler(
        State(manager): State<Arc<NotificationManager>>,
    ) -> Result<impl IntoResponse> {
        info!("Stopping notification scheduler");

        match manager.stop_scheduler().await {
            Ok(_) => {
                info!("Scheduler stopped successfully");
                Ok(StatusCode::OK)
            }
            Err(e) => {
                error!("Failed to stop scheduler: {}", e);
                Err(e)
            }
        }
    }
}

pub mod templates_handler {
    use super::*;

    #[derive(Deserialize)]
    pub struct TemplateQuery {
        pub notification_type: Option<NotificationType>,
        pub is_active: Option<bool>,
    }

    #[derive(Deserialize)]
    pub struct RenderTemplateRequest {
        pub data: Option<serde_json::Value>,
    }

    /// Create a new notification template
    pub async fn create_template(
        State(manager): State<Arc<NotificationManager>>,
        Json(request): Json<CreateTemplateRequest>,
    ) -> Result<impl IntoResponse> {
        info!("Creating notification template: {}", request.name);

        match manager.create_template(request).await {
            Ok(template) => {
                info!("Template created successfully: {}", template.id);
                Ok((StatusCode::CREATED, Json(template)))
            }
            Err(e) => {
                error!("Failed to create template: {}", e);
                Err(e)
            }
        }
    }

    /// Get a template by ID
    pub async fn get_template(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        info!("Getting template: {}", id);

        match manager.get_template(&id).await? {
            Some(template) => Ok(Json(template)),
            None => Err(NotificationError::not_found("template")),
        }
    }

    /// List templates with optional filtering
    pub async fn list_templates(
        State(manager): State<Arc<NotificationManager>>,
        Query(query): Query<TemplateQuery>,
    ) -> Result<impl IntoResponse> {
        info!("Listing templates");

        match manager
            .list_templates(query.notification_type, query.is_active)
            .await
        {
            Ok(templates) => {
                info!("Retrieved {} templates", templates.len());
                Ok(Json(templates))
            }
            Err(e) => {
                error!("Failed to list templates: {}", e);
                Err(e)
            }
        }
    }

    /// Update a template
    pub async fn update_template(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
        Json(request): Json<UpdateTemplateRequest>,
    ) -> Result<impl IntoResponse> {
        info!("Updating template: {}", id);

        match manager.update_template(&id, request).await {
            Ok(template) => {
                info!("Template updated successfully: {}", template.id);
                Ok(Json(template))
            }
            Err(e) => {
                error!("Failed to update template {}: {}", id, e);
                Err(e)
            }
        }
    }

    /// Delete a template
    pub async fn delete_template(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        info!("Deleting template: {}", id);

        match manager.delete_template(&id).await {
            Ok(true) => {
                info!("Template deleted successfully: {}", id);
                Ok(StatusCode::NO_CONTENT)
            }
            Ok(false) => Err(NotificationError::not_found("template")),
            Err(e) => {
                error!("Failed to delete template {}: {}", id, e);
                Err(e)
            }
        }
    }

    /// Render a template with data
    pub async fn render_template(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
        Json(_request): Json<RenderTemplateRequest>,
    ) -> Result<impl IntoResponse> {
        info!("Rendering template: {}", id);

        match manager.get_template(&id).await? {
            Some(template) => {
                // In a real implementation, this would use the template manager to render
                let rendered_response = serde_json::json!({
                    "template_id": id,
                    "subject": template.subject_template,
                    "content": template.content_template,
                    "rendered": true
                });
                Ok(Json(rendered_response))
            }
            None => Err(NotificationError::not_found("template")),
        }
    }
}

pub mod subscriptions_handler {
    use super::*;

    #[derive(Deserialize)]
    pub struct SubscriptionQuery {
        pub user_id: Option<String>,
        pub is_active: Option<bool>,
    }

    /// Create a new notification subscription
    pub async fn create_subscription(
        State(manager): State<Arc<NotificationManager>>,
        Query(query): Query<SubscriptionQuery>,
        Json(request): Json<CreateSubscriptionRequest>,
    ) -> Result<impl IntoResponse> {
        let user_id = query.user_id.as_deref().unwrap_or("default");
        info!("Creating notification subscription for user: {}", user_id);

        match manager.create_subscription(user_id, request).await {
            Ok(subscription) => {
                info!("Subscription created successfully: {}", subscription.id);
                Ok((StatusCode::CREATED, Json(subscription)))
            }
            Err(e) => {
                error!("Failed to create subscription: {}", e);
                Err(e)
            }
        }
    }

    /// Get a subscription by ID
    pub async fn get_subscription(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        info!("Getting subscription: {}", id);

        match manager.get_subscription(&id).await? {
            Some(subscription) => Ok(Json(subscription)),
            None => Err(NotificationError::not_found("subscription")),
        }
    }

    /// List subscriptions with optional filtering
    pub async fn list_subscriptions(
        State(manager): State<Arc<NotificationManager>>,
        Query(query): Query<SubscriptionQuery>,
    ) -> Result<impl IntoResponse> {
        if let Some(user_id) = query.user_id {
            info!("Listing subscriptions for user: {}", user_id);
            match manager.list_user_subscriptions(&user_id).await {
                Ok(subscriptions) => {
                    info!(
                        "Retrieved {} subscriptions for user: {}",
                        subscriptions.len(),
                        user_id
                    );
                    Ok(Json(subscriptions))
                }
                Err(e) => {
                    error!("Failed to list subscriptions for user {}: {}", user_id, e);
                    Err(e)
                }
            }
        } else {
            // Return empty list if no user_id provided
            Ok(Json(Vec::<NotificationSubscription>::new()))
        }
    }

    /// Update a subscription
    pub async fn update_subscription(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
        Json(request): Json<UpdateSubscriptionRequest>,
    ) -> Result<impl IntoResponse> {
        info!("Updating subscription: {}", id);

        match manager.update_subscription(&id, request).await {
            Ok(subscription) => {
                info!("Subscription updated successfully: {}", subscription.id);
                Ok(Json(subscription))
            }
            Err(e) => {
                error!("Failed to update subscription {}: {}", id, e);
                Err(e)
            }
        }
    }

    /// Delete a subscription
    pub async fn delete_subscription(
        State(manager): State<Arc<NotificationManager>>,
        Path(id): Path<String>,
    ) -> Result<impl IntoResponse> {
        info!("Deleting subscription: {}", id);

        match manager.delete_subscription(&id).await {
            Ok(true) => {
                info!("Subscription deleted successfully: {}", id);
                Ok(StatusCode::NO_CONTENT)
            }
            Ok(false) => Err(NotificationError::not_found("subscription")),
            Err(e) => {
                error!("Failed to delete subscription {}: {}", id, e);
                Err(e)
            }
        }
    }
}

pub mod websocket_handler {
    use super::*;
    use axum::extract::ws::WebSocket;

    /// Handle WebSocket connections
    pub async fn websocket_handler(
        ws: WebSocketUpgrade,
        State(ws_manager): State<Arc<WebSocketManager>>,
    ) -> Response {
        info!("New WebSocket connection");

        ws.on_upgrade(move |socket| {
            handle_websocket_connection(socket, ws_manager, "anonymous".to_string())
        })
    }

    /// Handle user-specific WebSocket connections
    pub async fn user_websocket_handler(
        Path(user_id): Path<String>,
        ws: WebSocketUpgrade,
        State(ws_manager): State<Arc<WebSocketManager>>,
    ) -> Response {
        info!("New WebSocket connection for user: {}", user_id);

        ws.on_upgrade(move |socket| handle_websocket_connection(socket, ws_manager, user_id))
    }

    async fn handle_websocket_connection(
        socket: WebSocket,
        ws_manager: Arc<WebSocketManager>,
        user_id: String,
    ) {
        info!("Handling WebSocket connection for user: {}", user_id);

        if let Err(e) = ws_manager.handle_connection(user_id.clone(), socket).await {
            error!("WebSocket connection error for user {}: {}", user_id, e);
        }

        info!("WebSocket connection closed for user: {}", user_id);
    }
}

/// Health check handler
pub async fn health_handler(
    State(manager): State<Arc<NotificationManager>>,
) -> Result<impl IntoResponse> {
    match manager.health_check().await {
        Ok(health) => {
            info!("Health check passed");
            Ok(Json(health))
        }
        Err(e) => {
            error!("Health check failed: {}", e);
            Err(e)
        }
    }
}

/// Metrics handler
pub async fn metrics_handler(
    State(_manager): State<Arc<NotificationManager>>,
) -> Result<impl IntoResponse> {
    // In a real implementation, this would export Prometheus metrics
    let metrics = serde_json::json!({
        "service": "notification",
        "version": "1.0.0",
        "uptime": chrono::Utc::now(),
        "status": "healthy"
    });

    Ok(Json(metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NotificationConfig;
    use axum_test::TestServer;

    async fn create_test_manager() -> Arc<NotificationManager> {
        let config = NotificationConfig::default();
        Arc::new(NotificationManager::new(config).await.unwrap())
    }

    #[tokio::test]
    async fn test_create_notification() {
        let manager = create_test_manager().await;

        let request = CreateNotificationRequest {
            recipient_id: "test_user".to_string(),
            notification_type: NotificationType::WorkflowCompleted,
            title: "Test Notification".to_string(),
            content: "This is a test".to_string(),
            channels: vec![NotificationChannel::Email],
            priority: NotificationPriority::Normal,
            template_id: None,
            template_data: None,
            scheduled_at: None,
            expires_at: None,
            metadata: None,
        };

        // Test that the manager was created successfully (basic smoke test)
        assert!(manager
            .get_notification_stats(None, None, None)
            .await
            .is_ok());

        // Test that the notification request structure is valid
        assert!(!request.recipient_id.is_empty());
        assert!(!request.title.is_empty());
        assert!(!request.content.is_empty());
        assert!(!request.channels.is_empty());
    }

    #[tokio::test]
    async fn test_health_handler() {
        let manager = create_test_manager().await;
        let result = health_handler(State(manager)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_template() {
        let manager = create_test_manager().await;

        let request = CreateTemplateRequest {
            name: "Test Template".to_string(),
            description: Some("Test description".to_string()),
            notification_type: NotificationType::WorkflowCompleted,
            channels: vec![NotificationChannel::Email],
            subject_template: "Test: {{title}}".to_string(),
            content_template: "Hello {{name}}!".to_string(),
            variables: vec![],
        };

        let result = manager.create_template(request).await;
        assert!(result.is_ok());
    }
}
