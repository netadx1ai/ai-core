//! # Integration Tests for Event Streaming Service
//!
//! This module contains comprehensive integration tests for the event streaming service.
//! These tests verify the end-to-end functionality of the service components.

use std::time::Duration;

use chrono::Utc;
use serde_json;
use tokio::time::sleep;
use uuid::Uuid;

use event_streaming_service::{
    config::Config,
    events::{Event, EventPayload, EventType},
    server::EventStreamingService,
    types::{EventCategory, EventPriority, EventSource},
};

/// Test configuration for integration tests
fn create_test_config() -> Config {
    let mut config = Config::default();

    // Override settings for testing
    config.server.port = 0; // Use random port
    config.processing.worker_threads = 2;
    config.processing.batch_size = 10;
    config.kafka.bootstrap_servers = vec!["localhost:9092".to_string()];
    config.redis.url = "redis://localhost:6379".to_string();
    config.storage.database_url = "postgresql://localhost:5432/event_streaming_test".to_string();
    config.environment.name = "test".to_string();
    config.environment.debug = true;

    config
}

/// Create a test event
fn create_test_event() -> Event {
    let source = EventSource {
        service: "integration-test".to_string(),
        version: "1.0.0".to_string(),
        instance_id: Some("test-instance".to_string()),
        hostname: Some("test-host".to_string()),
        metadata: std::collections::HashMap::new(),
    };

    let payload = EventPayload::Custom(serde_json::json!({
        "test_data": "integration_test",
        "timestamp": Utc::now().to_rfc3339(),
        "sequence": 1
    }));

    Event::new("integration.test", EventCategory::System, source, payload)
}

/// Create a workflow test event
fn create_workflow_event() -> Event {
    let source = EventSource {
        service: "workflow-service".to_string(),
        version: "1.0.0".to_string(),
        instance_id: Some("workflow-instance".to_string()),
        hostname: Some("test-host".to_string()),
        metadata: std::collections::HashMap::new(),
    };

    let workflow_id = Uuid::new_v4();
    let data = serde_json::json!({
        "step": "validation",
        "progress": 0.5,
        "metadata": {
            "user_id": "test-user",
            "request_id": "test-request"
        }
    });

    Event::workflow_event(workflow_id, "started", source, data)
}

#[tokio::test]
async fn test_service_initialization() {
    let config = create_test_config();

    // Create service instance
    let service = EventStreamingService::new(config).await;

    // Service creation should succeed even without external dependencies
    assert!(service.is_ok(), "Service initialization should succeed");
}

#[tokio::test]
async fn test_service_health_check() {
    let config = create_test_config();

    if let Ok(service) = EventStreamingService::new(config).await {
        let health = service.health().await;
        assert!(health.is_ok(), "Health check should return OK");

        let health_data = health.unwrap();
        assert!(
            health_data.is_object(),
            "Health data should be a JSON object"
        );
        assert!(
            health_data.get("service").is_some(),
            "Health data should include service info"
        );
        assert!(
            health_data.get("timestamp").is_some(),
            "Health data should include timestamp"
        );
    }
}

#[tokio::test]
async fn test_event_creation_and_validation() {
    let event = create_test_event();

    // Verify event structure
    assert!(!event.id.is_nil(), "Event should have a valid ID");
    assert_eq!(event.event_type, "integration.test");
    assert_eq!(event.category, EventCategory::System);
    assert_eq!(event.priority, EventPriority::Normal);
    assert_eq!(event.source.service, "integration-test");

    // Verify event validation
    let validation_result = event.validate();
    assert!(validation_result.is_ok(), "Event validation should pass");
}

#[tokio::test]
async fn test_workflow_event_creation() {
    let event = create_workflow_event();

    // Verify workflow event structure
    assert_eq!(event.event_type, "workflow.action");
    assert_eq!(event.category, EventCategory::Workflow);

    // Verify workflow payload
    if let EventPayload::Workflow(payload) = &event.payload {
        assert_eq!(payload.action, "started");
        assert!(payload.workflow_id != Uuid::nil());
        assert!(payload.data.get("step").is_some());
    } else {
        panic!("Expected workflow payload");
    }
}

#[tokio::test]
async fn test_event_status_updates() {
    let mut event = create_test_event();

    // Initial status should be pending
    assert_eq!(event.status, crate::types::EventStatus::Pending);
    assert_eq!(event.processing_history.len(), 0);

    // Update status to processing
    event.update_status(
        crate::types::EventStatus::Processing,
        Some("Processing started".to_string()),
    );

    assert_eq!(event.status, crate::types::EventStatus::Processing);
    assert_eq!(event.processing_history.len(), 1);

    // Update status to completed
    event.update_status(
        crate::types::EventStatus::Completed,
        Some("Processing completed successfully".to_string()),
    );

    assert_eq!(event.status, crate::types::EventStatus::Completed);
    assert_eq!(event.processing_history.len(), 2);
}

#[tokio::test]
async fn test_event_retry_logic() {
    let mut event = create_test_event();

    // Event should not retry when pending
    assert!(!event.should_retry(3));

    // Mark event as failed
    let error = crate::events::EventError {
        error_type: "processing_error".to_string(),
        message: "Test processing error".to_string(),
        code: Some("PROC_001".to_string()),
        retry_after: None,
        retryable: true,
        occurred_at: Utc::now(),
    };

    event.mark_failed(error);

    // Event should be retryable
    assert!(event.should_retry(3));
    assert_eq!(event.attempt_count, 1);
    assert_eq!(event.status, crate::types::EventStatus::Failed);

    // After max attempts, should not retry
    event.attempt_count = 3;
    assert!(!event.should_retry(3));
}

#[tokio::test]
async fn test_event_expiration() {
    let mut event = create_test_event();

    // Event without expiration should not be expired
    assert!(!event.is_expired());

    // Set expiration in the past
    event.expires_at = Some(Utc::now() - chrono::Duration::minutes(5));
    assert!(event.is_expired());

    // Set expiration in the future
    event.expires_at = Some(Utc::now() + chrono::Duration::minutes(5));
    assert!(!event.is_expired());
}

#[tokio::test]
async fn test_configuration_validation() {
    let config = create_test_config();

    // Valid configuration should pass validation
    let result = config.validate();
    assert!(result.is_ok(), "Valid configuration should pass validation");
}

#[tokio::test]
async fn test_configuration_defaults() {
    let config = Config::default();

    // Check default values
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.processing.batch_size, 100);
    assert_eq!(config.processing.worker_threads, num_cpus::get());
    assert_eq!(config.kafka.consumer_group_id, "event-streaming-service");
    assert_eq!(config.redis.url, "redis://localhost:6379");
}

#[tokio::test]
async fn test_event_serialization() {
    let event = create_test_event();

    // Test JSON serialization
    let json_result = serde_json::to_string(&event);
    assert!(
        json_result.is_ok(),
        "Event JSON serialization should succeed"
    );

    let json_str = json_result.unwrap();
    assert!(json_str.contains(&event.id.to_string()));
    assert!(json_str.contains("integration.test"));

    // Test JSON deserialization
    let deserialized_result: Result<Event, _> = serde_json::from_str(&json_str);
    assert!(
        deserialized_result.is_ok(),
        "Event JSON deserialization should succeed"
    );

    let deserialized_event = deserialized_result.unwrap();
    assert_eq!(deserialized_event.id, event.id);
    assert_eq!(deserialized_event.event_type, event.event_type);
    assert_eq!(deserialized_event.category, event.category);
}

#[tokio::test]
async fn test_event_type_conversions() {
    // Test event type string conversions
    assert_eq!(EventType::WorkflowCreated.as_str(), "workflow.created");
    assert_eq!(EventType::UserLoggedIn.as_str(), "user.logged_in");
    assert_eq!(EventType::ServiceStarted.as_str(), "service.started");

    // Test custom event type
    let custom = EventType::Custom("my.custom.event".to_string());
    assert_eq!(custom.as_str(), "my.custom.event");

    // Test conversion to string
    let event_type_str: String = EventType::WorkflowStarted.into();
    assert_eq!(event_type_str, "workflow.started");
}

#[tokio::test]
async fn test_concurrent_event_creation() {
    let mut handles = Vec::new();

    // Create multiple events concurrently
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let mut event = create_test_event();
            event.event_type = format!("concurrent.test.{}", i);
            event
        });
        handles.push(handle);
    }

    // Wait for all events to be created
    let mut events = Vec::new();
    for handle in handles {
        let event = handle.await.unwrap();
        events.push(event);
    }

    // Verify all events were created correctly
    assert_eq!(events.len(), 10);

    // Verify all events have unique IDs
    let mut ids = std::collections::HashSet::new();
    for event in &events {
        assert!(ids.insert(event.id), "All event IDs should be unique");
    }
}

#[tokio::test]
async fn test_event_metadata_handling() {
    let mut event = create_test_event();

    // Add metadata
    event.metadata.tags.push("integration".to_string());
    event.metadata.tags.push("test".to_string());
    event
        .metadata
        .properties
        .insert("environment".to_string(), "test".to_string());
    event.metadata.tenant_id = Some("test-tenant".to_string());

    // Verify metadata
    assert!(event.metadata.tags.contains(&"integration".to_string()));
    assert!(event.metadata.tags.contains(&"test".to_string()));
    assert_eq!(
        event.metadata.properties.get("environment"),
        Some(&"test".to_string())
    );
    assert_eq!(event.metadata.tenant_id, Some("test-tenant".to_string()));

    // Test serialization with metadata
    let json_result = serde_json::to_string(&event);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    assert!(json_str.contains("integration"));
    assert!(json_str.contains("test-tenant"));
}

#[tokio::test]
async fn test_error_handling() {
    use event_streaming_service::error::{ErrorSeverity, EventStreamingError};

    // Test error creation and properties
    let config_error = EventStreamingError::configuration("Invalid configuration");
    assert_eq!(config_error.severity(), ErrorSeverity::High);
    assert!(!config_error.is_retryable());
    assert_eq!(config_error.category(), "configuration");

    let processing_error =
        EventStreamingError::processing("Processing failed", Uuid::new_v4(), true);
    assert_eq!(processing_error.severity(), ErrorSeverity::Medium);
    assert!(processing_error.is_retryable());
    assert_eq!(processing_error.category(), "processing");

    let rate_limit_error = EventStreamingError::rate_limit("Too many requests", 100, 60, 30);
    assert_eq!(rate_limit_error.severity(), ErrorSeverity::Low);
    assert!(rate_limit_error.is_retryable());
    assert_eq!(rate_limit_error.retry_delay_seconds(), Some(30));
}

#[tokio::test]
async fn test_service_lifecycle() {
    let config = create_test_config();

    if let Ok(service) = EventStreamingService::new(config).await {
        // Test service start (may fail without external dependencies, which is OK)
        let start_result = service.start().await;
        if start_result.is_err() {
            println!("Service start failed (expected without external dependencies)");
        }

        // Test service stop
        let stop_result = service.stop().await;
        assert!(stop_result.is_ok(), "Service stop should succeed");
    }
}

// Utility function for testing with timeout
async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, &'static str>
where
    F: std::future::Future<Output = T>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => Err("Operation timed out"),
    }
}

#[tokio::test]
async fn test_operations_with_timeout() {
    let timeout_duration = Duration::from_secs(5);

    // Test service creation with timeout
    let config = create_test_config();
    let service_result = with_timeout(timeout_duration, async {
        EventStreamingService::new(config).await
    })
    .await;

    assert!(
        service_result.is_ok(),
        "Service creation should complete within timeout"
    );

    if let Ok(Ok(service)) = service_result {
        // Test health check with timeout
        let health_result = with_timeout(timeout_duration, service.health()).await;
        assert!(
            health_result.is_ok(),
            "Health check should complete within timeout"
        );
    }
}

#[tokio::test]
async fn test_large_event_payload() {
    let source = EventSource {
        service: "integration-test".to_string(),
        version: "1.0.0".to_string(),
        instance_id: Some("test-instance".to_string()),
        hostname: Some("test-host".to_string()),
        metadata: std::collections::HashMap::new(),
    };

    // Create a large payload
    let large_data = "x".repeat(1024 * 1024); // 1MB of data
    let payload = EventPayload::Custom(serde_json::json!({
        "large_field": large_data,
        "metadata": {
            "size": 1024 * 1024,
            "type": "performance_test"
        }
    }));

    let event = Event::new(
        "performance.large_payload",
        EventCategory::System,
        source,
        payload,
    );

    // Verify event can be created and serialized
    assert!(!event.id.is_nil());

    let serialization_result = serde_json::to_string(&event);
    assert!(
        serialization_result.is_ok(),
        "Large event should be serializable"
    );

    // Verify deserialization
    let json_str = serialization_result.unwrap();
    let deserialization_result: Result<Event, _> = serde_json::from_str(&json_str);
    assert!(
        deserialization_result.is_ok(),
        "Large event should be deserializable"
    );
}
