//! ClickHouse Analytics Example for AI-CORE Database Layer
//!
//! This example demonstrates how to use ClickHouse for high-performance analytics:
//! - Initialize ClickHouse connection
//! - Track workflow and API events
//! - Perform real-time analytics queries
//! - Create materialized views for dashboards
//! - Bulk data insertion for high performance

use ai_core_database::{
    analytics::{
        AnalyticsManager, SystemMetricType, TimeRange, WorkflowEventData, WorkflowEventMetadata,
        WorkflowEventType,
    },
    ClickHouseConfig, DatabaseConfig, DatabaseManager, MonitoringConfig, PostgresConfig,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🚀 AI-CORE ClickHouse Analytics Example");

    // 1. Create database configuration with ClickHouse enabled
    let config = DatabaseConfig {
        postgresql: PostgresConfig::default(),
        monitoring: MonitoringConfig::default(),
        clickhouse: Some(ClickHouseConfig {
            url: std::env::var("CLICKHOUSE_URL")
                .unwrap_or_else(|_| "http://localhost:8123".to_string()),
            database: "automation_analytics".to_string(),
            username: std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string()),
            password: std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default(),
            pool_size: 5,
            timeout_seconds: 30,
            compression: true,
            secure: false,
        }),
        #[cfg(feature = "mongodb")]
        mongodb: None,
        #[cfg(feature = "redis")]
        redis: None,
    };

    println!("📊 Configuration loaded:");
    println!(
        "  - ClickHouse URL: {}",
        config.clickhouse.as_ref().unwrap().url
    );
    println!(
        "  - Database: {}",
        config.clickhouse.as_ref().unwrap().database
    );

    // 2. Initialize database manager
    println!("\n🔌 Initializing database connections...");

    match DatabaseManager::new(config).await {
        Ok(manager) => {
            if let Some(clickhouse_connection) = &manager.clickhouse {
                println!("✅ ClickHouse connection established successfully!");

                // Create analytics manager
                let analytics = AnalyticsManager::new(clickhouse_connection.clone());

                // 3. Perform health check (PostgreSQL)
                println!("\n🏥 Performing health check...");
                match manager.health_check().await {
                    Ok(health) => {
                        println!("✅ Health check passed!");
                        println!("  - PostgreSQL healthy: {}", health.postgres.healthy);
                        println!("  - Response time: {}ms", health.postgres.response_time_ms);
                    }
                    Err(e) => {
                        println!("❌ Health check failed: {}", e);
                    }
                }

                // 4. Create database tables and views
                println!("\n📋 Setting up analytics tables...");
                match setup_clickhouse_tables(&analytics).await {
                    Ok(_) => println!("✅ Tables and views created successfully"),
                    Err(e) => println!("⚠️  Tables setup failed (may already exist): {}", e),
                }

                // 5. Track some sample workflow events
                println!("\n📈 Tracking sample workflow events...");
                track_sample_events(&analytics).await?;

                // 6. Perform analytics queries
                println!("\n📊 Running analytics queries...");
                run_analytics_examples(&analytics).await?;

                // 7. Demonstrate bulk data insertion
                println!("\n⚡ Demonstrating high-performance bulk insertion...");
                demonstrate_bulk_insertion(&analytics).await?;

                // 8. Show real-time metrics
                println!("\n📈 Real-time metrics summary...");
                show_metrics_summary(&analytics).await?;

                // 9. Connection statistics
                println!("\n📊 ClickHouse connection statistics:");
                let stats = analytics.get_connection_stats().await;
                println!("  - Queries executed: {}", stats.queries_executed);
                println!("  - Total query time: {}ms", stats.total_query_time_ms);
                println!(
                    "  - Average query time: {:.2}ms",
                    stats.average_query_time_ms()
                );
                println!("  - Bulk inserts: {}", stats.bulk_inserts);
                println!("  - Total rows inserted: {}", stats.total_rows_inserted);
                println!(
                    "  - Average rows per insert: {:.0}",
                    stats.rows_per_insert()
                );

                // 10. Table optimization
                println!("\n🔧 Optimizing tables for better performance...");
                match analytics.optimize_tables().await {
                    Ok(_) => println!("✅ Tables optimized successfully"),
                    Err(e) => println!("⚠️  Table optimization failed: {}", e),
                }

                println!("\n🎉 Analytics example completed successfully!");
            } else {
                println!("❌ ClickHouse connection not available");
                println!("💡 Make sure ClickHouse feature is enabled and configured");
                return Ok(());
            }

            // Graceful shutdown
            println!("\n🔄 Shutting down gracefully...");
            manager.shutdown().await?;
            println!("✅ Database connections closed cleanly");
        }
        Err(e) => {
            println!("❌ Failed to initialize database: {}", e);
            println!("\n💡 Tips:");
            println!("  - Make sure ClickHouse is running on localhost:8123");
            println!("  - Create database: CREATE DATABASE automation_analytics");
            println!("  - Set CLICKHOUSE_URL environment variable if using different connection");
            println!(
                "  - Set CLICKHOUSE_USER and CLICKHOUSE_PASSWORD if authentication is required"
            );

            return Err(e);
        }
    }

    Ok(())
}

/// Setup ClickHouse tables and views
async fn setup_clickhouse_tables(analytics: &AnalyticsManager) -> anyhow::Result<()> {
    // Create dashboard materialized views
    analytics.create_dashboard_views().await?;
    Ok(())
}

/// Track sample events to demonstrate analytics
async fn track_sample_events(analytics: &AnalyticsManager) -> anyhow::Result<()> {
    println!("  📝 Tracking workflow events...");

    // Track successful workflow completion
    analytics
        .track_workflow_event(
            "workflow_123",
            "user_456",
            "content_generator",
            WorkflowEventType::WorkflowCompleted,
            2500, // 2.5 seconds
            0.05, // $0.05 cost
            true,
            Some(WorkflowEventMetadata {
                error_code: None,
                error_message: None,
                provider_id: Some("openai".to_string()),
                mcp_server_id: Some("mcp_server_1".to_string()),
                request_size: 1024,
                response_size: 2048,
            }),
        )
        .await?;

    // Track failed workflow
    analytics
        .track_workflow_event(
            "workflow_124",
            "user_789",
            "image_processor",
            WorkflowEventType::WorkflowFailed,
            1200,
            0.02,
            false,
            Some(WorkflowEventMetadata {
                error_code: Some("RATE_LIMIT".to_string()),
                error_message: Some("Rate limit exceeded".to_string()),
                provider_id: Some("stability_ai".to_string()),
                mcp_server_id: Some("mcp_server_2".to_string()),
                request_size: 2048,
                response_size: 512,
            }),
        )
        .await?;

    println!("  🌐 Tracking API requests...");

    // Track API requests
    analytics
        .track_api_request(
            "user_456",
            "/api/v1/workflows",
            "POST",
            200,
            150,
            1024,
            2048,
            "192.168.1.100",
            true,
        )
        .await?;

    analytics
        .track_api_request(
            "user_789",
            "/api/v1/workflows",
            "POST",
            429,
            50,
            512,
            256,
            "192.168.1.101",
            false,
        )
        .await?;

    println!("  📊 Tracking system metrics...");

    // Track system metrics
    analytics
        .track_system_metric(
            "api_gateway",
            "cpu_usage_percent",
            SystemMetricType::Gauge,
            75.5,
        )
        .await?;

    analytics
        .track_system_metric(
            "content_generator",
            "memory_usage_percent",
            SystemMetricType::Gauge,
            82.3,
        )
        .await?;

    analytics
        .track_system_metric(
            "api_gateway",
            "requests_per_second",
            SystemMetricType::Counter,
            145.0,
        )
        .await?;

    println!("  ✅ Sample events tracked successfully");
    Ok(())
}

/// Run various analytics queries
async fn run_analytics_examples(analytics: &AnalyticsManager) -> anyhow::Result<()> {
    // Get workflow metrics for the last hour
    println!("  📈 Workflow metrics (last hour):");
    let workflow_metrics = analytics
        .get_workflow_metrics(
            TimeRange::last_hour(),
            None, // All services
        )
        .await?;

    println!("    - Total events: {}", workflow_metrics.total_events);
    println!("    - Success rate: {:.1}%", workflow_metrics.success_rate);
    println!(
        "    - Average duration: {:.0}ms",
        workflow_metrics.avg_duration_ms
    );
    println!(
        "    - P95 duration: {:.0}ms",
        workflow_metrics.p95_duration_ms
    );
    println!("    - Total cost: ${:.4}", workflow_metrics.total_cost_usd);

    // Get API performance metrics
    println!("\n  🌐 API metrics (last hour):");
    let api_metrics = analytics
        .get_api_metrics(
            TimeRange::last_hour(),
            None, // All endpoints
        )
        .await?;

    println!("    - Total requests: {}", api_metrics.total_requests);
    println!("    - Success rate: {:.1}%", api_metrics.success_rate);
    println!(
        "    - Average response time: {:.0}ms",
        api_metrics.avg_response_time_ms
    );
    println!(
        "    - P95 response time: {:.0}ms",
        api_metrics.p95_response_time_ms
    );
    println!("    - Total bytes: {}", api_metrics.total_bytes);

    // Get top users
    println!("\n  👥 Top users (last hour):");
    let top_users = analytics
        .get_top_users(
            TimeRange::last_hour(),
            5, // Top 5 users
        )
        .await?;

    for (i, user) in top_users.iter().enumerate() {
        println!(
            "    {}. User {}: {} events, {:.1}% success, ${:.4} cost",
            i + 1,
            user.user_id,
            user.total_events,
            user.success_rate,
            user.total_cost_usd
        );
    }

    // Get system overview
    println!("\n  🖥️  System overview (last hour):");
    let system_overview = analytics
        .get_system_overview(TimeRange::last_hour())
        .await?;

    println!("    - Total services: {}", system_overview.total_services);
    println!("    - Total requests: {}", system_overview.total_requests);
    println!("    - Total cost: ${:.4}", system_overview.total_cost_usd);

    println!("    - Service performance:");
    for service in &system_overview.service_performance {
        println!(
            "      • {}: {} requests, {:.1}% success, {:.0}ms avg",
            service.service_name,
            service.request_count,
            service.success_rate,
            service.avg_duration_ms
        );
    }

    if !system_overview.error_summary.is_empty() {
        println!("    - Error summary:");
        for error in &system_overview.error_summary {
            println!(
                "      • {}: {} occurrences in {}",
                error.error_code, error.error_count, error.service_name
            );
        }
    }

    Ok(())
}

/// Demonstrate bulk data insertion for high performance
async fn demonstrate_bulk_insertion(analytics: &AnalyticsManager) -> anyhow::Result<()> {
    let start_time = std::time::Instant::now();

    println!("  🚀 Generating 1000 sample events for bulk insertion...");

    // Generate sample workflow events
    let mut events = Vec::new();
    for i in 0..1000 {
        events.push(WorkflowEventData {
            workflow_id: format!("bulk_workflow_{}", i),
            user_id: format!("user_{}", i % 10), // 10 different users
            service_name: match i % 4 {
                0 => "content_generator".to_string(),
                1 => "image_processor".to_string(),
                2 => "social_publisher".to_string(),
                _ => "analytics_engine".to_string(),
            },
            event_type: if i % 10 == 0 {
                WorkflowEventType::WorkflowFailed
            } else {
                WorkflowEventType::WorkflowCompleted
            },
            duration_ms: 1000 + (i % 3000) as u64, // 1-4 seconds
            cost_usd: 0.01 + (i as f64 * 0.001),   // $0.01 to $1.00
            success: i % 10 != 0,                  // 90% success rate
            metadata: Some(WorkflowEventMetadata {
                error_code: if i % 10 == 0 {
                    Some("TIMEOUT".to_string())
                } else {
                    None
                },
                error_message: if i % 10 == 0 {
                    Some("Operation timed out".to_string())
                } else {
                    None
                },
                provider_id: Some(format!("provider_{}", i % 3)),
                mcp_server_id: Some(format!("mcp_{}", i % 5)),
                request_size: 500 + (i % 2000) as u32,
                response_size: 1000 + (i % 4000) as u32,
            }),
        });
    }

    println!("  ⚡ Performing bulk insertion...");

    // Perform bulk insertion
    let rows_inserted = analytics.batch_track_workflow_events(events).await?;
    let elapsed = start_time.elapsed();
    let rows_per_second = rows_inserted as f64 / elapsed.as_secs_f64();

    println!("  ✅ Bulk insertion completed:");
    println!("    - Rows inserted: {}", rows_inserted);
    println!("    - Time taken: {:?}", elapsed);
    println!("    - Insertion rate: {:.0} rows/second", rows_per_second);

    Ok(())
}

/// Show real-time metrics summary
async fn show_metrics_summary(analytics: &AnalyticsManager) -> anyhow::Result<()> {
    println!("  📊 Service-specific metrics:");

    // Content generator metrics
    let content_metrics = analytics
        .get_workflow_metrics(TimeRange::last_hour(), Some("content_generator"))
        .await?;

    println!("    Content Generator:");
    println!("      - Events: {}", content_metrics.total_events);
    println!("      - Success Rate: {:.1}%", content_metrics.success_rate);
    println!(
        "      - Avg Duration: {:.0}ms",
        content_metrics.avg_duration_ms
    );
    println!("      - Total Cost: ${:.4}", content_metrics.total_cost_usd);

    // API endpoint specific metrics
    let api_workflow_metrics = analytics
        .get_api_metrics(TimeRange::last_hour(), Some("/api/v1/workflows"))
        .await?;

    println!("    Workflow API Endpoint:");
    println!("      - Requests: {}", api_workflow_metrics.total_requests);
    println!(
        "      - Success Rate: {:.1}%",
        api_workflow_metrics.success_rate
    );
    println!(
        "      - Avg Response Time: {:.0}ms",
        api_workflow_metrics.avg_response_time_ms
    );
    println!(
        "      - Total Bandwidth: {} bytes",
        api_workflow_metrics.total_bytes
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = ClickHouseConfig::default();
        assert_eq!(config.database, "automation_analytics");
        assert_eq!(config.url, "http://localhost:8123");
        assert!(config.compression);
    }

    #[test]
    fn test_workflow_event_metadata() {
        let metadata = WorkflowEventMetadata {
            error_code: Some("TEST_ERROR".to_string()),
            error_message: Some("Test error message".to_string()),
            provider_id: Some("test_provider".to_string()),
            mcp_server_id: Some("test_mcp".to_string()),
            request_size: 1024,
            response_size: 2048,
        };

        assert_eq!(metadata.error_code.unwrap(), "TEST_ERROR");
        assert_eq!(metadata.request_size, 1024);
        assert_eq!(metadata.response_size, 2048);
    }

    #[test]
    fn test_time_ranges() {
        let last_hour = TimeRange::last_hour();
        let last_day = TimeRange::last_24_hours();
        let last_week = TimeRange::last_7_days();

        assert!(last_hour.end_time > last_hour.start_time);
        assert!(last_day.end_time > last_day.start_time);
        assert!(last_week.end_time > last_week.start_time);

        // Check durations are approximately correct
        let hour_duration = last_hour.end_time - last_hour.start_time;
        assert!(hour_duration.num_hours() <= 1);

        let day_duration = last_day.end_time - last_day.start_time;
        assert!(day_duration.num_hours() <= 24);
    }
}
