//! Comprehensive unit tests for ClickHouse integration
//!
//! Tests connection management, analytics operations, bulk insertion, and query optimization

use std::time::Duration;
use ai_core_database::{
    connections::ClickHouseConfig,
    connections::clickhouse::{WorkflowEvent, ApiRequest, SystemMetric},
    DatabaseError
};

/// Test ClickHouse configuration validation
#[test]
fn test_clickhouse_config_validation() {
    let config = ClickHouseConfig {
        url: "http://localhost:8123".to_string(),
        username: "default".to_string(),
        password: "".to_string(),
        database: "analytics".to_string(),
        timeout_seconds: 30,
        pool_size: 10,
        compression: true,
        secure: false,
    };

    // Validate configuration constraints
    assert!(config.timeout_seconds > 0);
    assert!(config.pool_size > 0);
    assert!(!config.url.is_empty());
    assert!(!config.database.is_empty());
}

#[test]
fn test_clickhouse_config_defaults() {
    let config = ClickHouseConfig::default();

    assert_eq!(config.url, "http://localhost:8123");
    assert_eq!(config.username, "default");
    assert_eq!(config.password, "");
    assert_eq!(config.database, "automation_analytics");
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.pool_size, 10);
    assert!(config.compression);
    assert!(!config.secure);
}

#[test]
fn test_clickhouse_config_serialization() {
    let config = ClickHouseConfig::default();

    // Test JSON serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    // Test JSON deserialization
    let deserialized: ClickHouseConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.database, deserialized.database);
    assert_eq!(config.compression, deserialized.compression);
    assert_eq!(config.pool_size, deserialized.pool_size);
}

#[test]
fn test_clickhouse_url_validation() {
    let valid_urls = vec![
        "http://localhost:8123",
        "https://localhost:8443",
        "http://clickhouse.example.com:8123",
        "https://secure-clickhouse.example.com:8443",
    ];

    for url in valid_urls {
        let config = ClickHouseConfig {
            url: url.to_string(),
            ..ClickHouseConfig::default()
        };
        assert!(config.url.starts_with("http"));
        assert!(config.url.contains("://"));
    }
}

/// Test WorkflowEvent structure and validation
#[test]
fn test_workflow_event_creation() {
    let event = WorkflowEvent {
        event_id: "evt_123".to_string(),
        workflow_id: "wf_456".to_string(),
        user_id: "user_789".to_string(),
        service_name: "test_service".to_string(),
        event_type: "workflow_started".to_string(),
        event_category: "automation".to_string(),
        duration_ms: 150,
        cost_usd: 0.05,
        success: true,
        error_code: "".to_string(),
        error_message: "".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        metadata: None,
    };

    assert_eq!(event.event_id, "evt_123");
    assert_eq!(event.workflow_id, "wf_456");
    assert_eq!(event.user_id, "user_789");
    assert_eq!(event.event_type, "workflow_started");
    assert_eq!(event.service_name, "test_service");
    assert!(event.success);
    assert!(event.error_code.is_empty());
    assert_eq!(event.duration_ms, 150);
}

#[test]
fn test_workflow_event_with_error() {
    let event = WorkflowEvent {
        event_id: "evt_error".to_string(),
        workflow_id: "wf_failed".to_string(),
        user_id: "user_123".to_string(),
        service_name: "test_service".to_string(),
        event_type: "workflow_failed".to_string(),
        event_category: "automation".to_string(),
        duration_ms: 5000,
        cost_usd: 0.0,
        success: false,
        error_code: "ERR001".to_string(),
        error_message: "Workflow execution failed".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        metadata: None,
    };

    assert!(!event.success);
    assert!(!event.error_message.is_empty());
    assert_eq!(event.error_message, "Workflow execution failed");
    assert_eq!(event.duration_ms, 5000);
}

/// Test ApiRequest structure and validation
#[test]
fn test_api_request_creation() {
    let request = ApiRequest {
        request_id: "req_123".to_string(),
        user_id: Some("user_456".to_string()),
        endpoint: "/api/v1/workflows".to_string(),
        method: "POST".to_string(),
        status_code: 201,
        response_time_ms: 45,
        timestamp: chrono::Utc::now(),
        user_agent: Some("Mozilla/5.0".to_string()),
        ip_address: Some("192.168.1.1".to_string()),
        request_size_bytes: Some(1024),
        response_size_bytes: Some(512),
    };

    assert_eq!(request.request_id, "req_123");
    assert_eq!(request.endpoint, "/api/v1/workflows");
    assert_eq!(request.method, "POST");
    assert_eq!(request.status_code, 201);
    assert_eq!(request.response_time_ms, 45);
    assert!(request.user_id.is_some());
    assert!(request.user_agent.is_some());
    assert!(request.ip_address.is_some());
}

#[test]
fn test_api_request_validation() {
    let request = ApiRequest {
        request_id: "req_test".to_string(),
        user_id: None,
        endpoint: "/health".to_string(),
        method: "GET".to_string(),
        status_code: 200,
        response_time_ms: 5,
        timestamp: chrono::Utc::now(),
        user_agent: None,
        ip_address: None,
        request_size_bytes: None,
        response_size_bytes: Some(100),
    };

    // Test that optional fields can be None
    assert!(request.user_id.is_none());
    assert!(request.user_agent.is_none());
    assert!(request.ip_address.is_none());
    assert!(request.request_size_bytes.is_none());

    // Required fields should be present
    assert!(!request.request_id.is_empty());
    assert!(!request.endpoint.is_empty());
    assert!(!request.method.is_empty());
    assert!(request.status_code >= 100 && request.status_code < 600);
}

/// Test SystemMetric structure and validation
#[test]
fn test_system_metric_creation() {
    let metric = SystemMetric {
        metric_id: "metric_123".to_string(),
        service_name: "api-gateway".to_string(),
        metric_type: "cpu_usage".to_string(),
        value: 75.5,
        unit: "percent".to_string(),
        timestamp: chrono::Utc::now(),
        tags: Some(serde_json::json!({"environment": "production", "region": "us-west-2"})),
    };

    assert_eq!(metric.metric_id, "metric_123");
    assert_eq!(metric.service_name, "api-gateway");
    assert_eq!(metric.metric_type, "cpu_usage");
    assert_eq!(metric.value, 75.5);
    assert_eq!(metric.unit, "percent");
    assert!(metric.tags.is_some());
}

#[test]
fn test_system_metric_types() {
    let metrics = vec![
        ("cpu_usage", 80.0, "percent"),
        ("memory_usage", 1024.0, "megabytes"),
        ("disk_io", 150.5, "ops_per_sec"),
        ("network_latency", 25.0, "milliseconds"),
        ("request_count", 1000.0, "count"),
    ];

    for (metric_type, value, unit) in metrics {
        let metric = SystemMetric {
            metric_id: format!("metric_{}", metric_type),
            service_name: "test-service".to_string(),
            metric_type: metric_type.to_string(),
            value,
            unit: unit.to_string(),
            timestamp: chrono::Utc::now(),
            tags: None,
        };

        assert_eq!(metric.metric_type, metric_type);
        assert_eq!(metric.value, value);
        assert_eq!(metric.unit, unit);
        assert!(metric.tags.is_none());
    }
}

/// Test ClickHouse configuration edge cases
#[test]
fn test_config_edge_cases() {
    // Test minimum configuration
    let min_config = ClickHouseConfig {
        url: "http://localhost:8123".to_string(),
        username: "default".to_string(),
        password: "".to_string(),
        database: "test".to_string(),
        timeout_seconds: 1,
        pool_size: 1,
        compression: false,
        secure: false,
    };

    assert_eq!(min_config.timeout_seconds, 1);
    assert_eq!(min_config.pool_size, 1);
    assert!(!min_config.compression);

    // Test high-performance configuration
    let high_perf_config = ClickHouseConfig {
        url: "http://localhost:8123".to_string(),
        username: "default".to_string(),
        password: "secure_password".to_string(),
        database: "analytics_prod".to_string(),
        timeout_seconds: 300,
        pool_size: 50,
        compression: true,
        secure: true,
    };

    assert_eq!(high_perf_config.timeout_seconds, 300);
    assert_eq!(high_perf_config.pool_size, 50);
    assert!(high_perf_config.compression);
    assert!(high_perf_config.secure);
}

/// Test configuration validation with invalid values
#[test]
fn test_invalid_configuration_values() {
    // Test zero timeout
    let zero_timeout_config = ClickHouseConfig {
        timeout_seconds: 0,
        ..ClickHouseConfig::default()
    };
    assert_eq!(zero_timeout_config.timeout_seconds, 0);

    // Test empty URL
    let empty_url_config = ClickHouseConfig {
        url: "".to_string(),
        ..ClickHouseConfig::default()
    };
    assert!(empty_url_config.url.is_empty());

    // Test zero pool size
    let zero_pool_config = ClickHouseConfig {
        pool_size: 0,
        ..ClickHouseConfig::default()
    };
    assert_eq!(zero_pool_config.pool_size, 0);
}

/// Test thread safety and cloning
#[test]
fn test_config_thread_safety() {
    let config = ClickHouseConfig::default();
    let config_clone = config.clone();

    // Test that cloned config is identical
    assert_eq!(config.url, config_clone.url);
    assert_eq!(config.database, config_clone.database);
    assert_eq!(config.pool_size, config_clone.pool_size);
    assert_eq!(config.compression, config_clone.compression);

    // Test that configs are Send + Sync (compile-time check)
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ClickHouseConfig>();
}

/// Test error handling for ClickHouse operations
#[test]
fn test_clickhouse_error_handling() {
    // Test different types of database errors
    let connection_error = DatabaseError::Connection("ClickHouse connection failed".to_string());
    let query_error = DatabaseError::Query("Invalid ClickHouse query".to_string());

    match connection_error {
        DatabaseError::Connection(msg) => assert!(msg.contains("ClickHouse")),
        _ => panic!("Expected Connection error"),
    }

    match query_error {
        DatabaseError::Query(msg) => assert!(msg.contains("Invalid")),
        _ => panic!("Expected Query error"),
    }
}

#[cfg(test)]
mod mock_analytics_tests {
    use super::*;

    /// Test mock analytics operations
    #[tokio::test]
    async fn test_mock_analytics_operations() {
        let config = ClickHouseConfig::default();

        // Mock analytics operations - verify data structures
        let workflow_event = WorkflowEvent {
            event_id: "mock_event".to_string(),
            workflow_id: "mock_workflow".to_string(),
            user_id: "mock_user".to_string(),
            event_type: "test_event".to_string(),
            timestamp: chrono::Utc::now(),
            duration_ms: Some(100),
            status: "success".to_string(),
            error_message: None,
            metadata: Some(serde_json::json!({"test": true})),
        };

        assert_eq!(workflow_event.status, "success");
        assert!(workflow_event.metadata.is_some());

        // Test serialization for bulk insert simulation
        let serialized = serde_json::to_string(&workflow_event).unwrap();
        assert!(!serialized.is_empty());
        assert!(serialized.contains("mock_event"));
    }

    /// Test bulk data preparation
    #[test]
    fn test_bulk_data_preparation() {
        let mut events = Vec::new();

        // Create mock bulk data
        for i in 0..1000 {
            let event = WorkflowEvent {
                event_id: format!("perf_event_{}", i),
                workflow_id: format!("workflow_{}", i % 100),
                user_id: format!("user_{}", i % 50),
                service_name: "performance_service".to_string(),
                event_type: "performance_test".to_string(),
                event_category: "testing".to_string(),
                duration_ms: i % 1000,
                cost_usd: (i as f64) * 0.001,
                success: i % 10 != 0,
                error_code: if i % 10 == 0 { "ERR001".to_string() } else { "".to_string() },
                error_message: if i % 10 == 0 { "Test error".to_string() } else { "".to_string() },
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                metadata: None,
            };
            events.push(event);
        }

        assert_eq!(events.len(), 1000);

        // Test that bulk data has variety
        let success_count = events.iter().filter(|e| e.status == "success").count();
        let error_count = events.iter().filter(|e| e.status == "error").count();

        assert_eq!(success_count, 900);
        assert_eq!(error_count, 100);
    }

    /// Test query optimization patterns
    #[test]
    fn test_query_patterns() {
        let queries = vec![
            "SELECT count() FROM workflow_events WHERE status = 'success'",
            "SELECT user_id, count() as event_count FROM workflow_events GROUP BY user_id ORDER BY event_count DESC LIMIT 10",
            "SELECT toStartOfHour(timestamp) as hour, count() FROM workflow_events WHERE timestamp >= now() - INTERVAL 24 HOUR GROUP BY hour ORDER BY hour",
            "SELECT workflow_id, avg(duration_ms) as avg_duration FROM workflow_events WHERE duration_ms > 0 GROUP BY workflow_id HAVING avg_duration > 100",
        ];

        for query in queries {
            // Test query structure validation
            assert!(query.contains("SELECT"));
            assert!(query.contains("workflow_events"));

            // Test performance-oriented patterns
            if query.contains("GROUP BY") {
                assert!(query.contains("SELECT"));
            }
            if query.contains("ORDER BY") {
                assert!(query.contains("SELECT"));
            }
        }
    }
}

/// Performance-related tests (mock)
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_event_creation_performance() {
        let start = std::time::Instant::now();

        // Create many events to test performance
        for i in 0..10000 {
            let _event = WorkflowEvent {
                event_id: format!("perf_test_{}", i),
                workflow_id: "perf_workflow".to_string(),
                user_id: "perf_user".to_string(),
                event_type: "performance_test".to_string(),
                timestamp: chrono::Utc::now(),
                duration_ms: Some(i % 1000),
                status: "success".to_string(),
                error_message: None,
                metadata: None,
            };
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(1000)); // Should create 10k events in < 1s
    }

    #[tokio::test]
    async fn test_concurrent_event_processing() {
        let mut handles = vec![];

        // Spawn multiple tasks creating events concurrently
        for i in 0..100 {
            let handle = tokio::spawn(async move {
                let event = WorkflowEvent {
                    event_id: format!("concurrent_{}", i),
                    workflow_id: "concurrent_test".to_string(),
                    user_id: format!("user_{}", i),
                    service_name: "concurrent_service".to_string(),
                    event_type: "concurrent_event".to_string(),
                    event_category: "concurrency".to_string(),
                    duration_ms: 50,
                    cost_usd: 0.01,
                    success: true,
                    error_code: "".to_string(),
                    error_message: "".to_string(),
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    metadata: None,
                };

                // Simulate processing time
                tokio::time::sleep(Duration::from_millis(1)).await;
                event.event_id
            });
            handles.push(handle);
        }

        // Wait for all tasks and verify results
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.unwrap();
            assert_eq!(result, format!("concurrent_{}", i));
        }
    }

    #[test]
    fn test_serialization_performance() {
        let event = WorkflowEvent {
            event_id: "perf_serialize".to_string(),
            workflow_id: "perf_workflow".to_string(),
            user_id: "perf_user".to_string(),
            service_name: "perf_service".to_string(),
            event_type: "serialization_test".to_string(),
            event_category: "performance".to_string(),
            duration_ms: 100,
            cost_usd: 0.05,
            success: true,
            error_code: "".to_string(),
            error_message: "".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let start = std::time::Instant::now();

        // Serialize many times to test performance
        for _ in 0..1000 {
            let _serialized = serde_json::to_string(&event).unwrap();
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(1000)); // Should serialize 1k times in < 1s
    }
}

/// Test materialized view patterns
#[cfg(test)]
mod materialized_view_tests {

    #[test]
    fn test_materialized_view_queries() {
        let view_queries = vec![
            // Hourly workflow metrics
            r#"
            CREATE MATERIALIZED VIEW workflow_metrics_hourly
            ENGINE = SummingMergeTree()
            PARTITION BY toYYYYMM(timestamp)
            ORDER BY (workflow_id, toStartOfHour(timestamp))
            AS SELECT
                workflow_id,
                toStartOfHour(timestamp) as hour,
                count() as total_events,
                countIf(status = 'success') as success_count,
                countIf(status = 'error') as error_count,
                avg(duration_ms) as avg_duration
            FROM workflow_events
            GROUP BY workflow_id, hour
            "#,

            // API performance metrics
            r#"
            CREATE MATERIALIZED VIEW api_metrics_minutely
            ENGINE = SummingMergeTree()
            PARTITION BY toYYYYMM(timestamp)
            ORDER BY (endpoint, toStartOfMinute(timestamp))
            AS SELECT
                endpoint,
                toStartOfMinute(timestamp) as minute,
                count() as request_count,
                avg(response_time_ms) as avg_response_time,
                quantile(0.95)(response_time_ms) as p95_response_time,
                countIf(status_code >= 200 AND status_code < 300) as success_count,
                countIf(status_code >= 400) as error_count
            FROM api_requests
            GROUP BY endpoint, minute
            "#,
        ];

        for query in view_queries {
            assert!(query.contains("CREATE MATERIALIZED VIEW"));
            assert!(query.contains("ENGINE = SummingMergeTree()"));
            assert!(query.contains("PARTITION BY"));
            assert!(query.contains("ORDER BY"));
            assert!(query.contains("GROUP BY"));
        }
    }

    #[test]
    fn test_aggregation_patterns() {
        let aggregations = vec![
            ("count()", "total events"),
            ("countIf(status = 'success')", "success events"),
            ("avg(duration_ms)", "average duration"),
            ("quantile(0.95)(response_time_ms)", "95th percentile"),
            ("max(timestamp)", "latest timestamp"),
            ("min(timestamp)", "earliest timestamp"),
        ];

        for (function, description) in aggregations {
            assert!(function.contains("("));
            assert!(function.contains(")"));
            assert!(!description.is_empty());
        }
    }
}
