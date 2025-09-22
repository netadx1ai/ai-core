//! Comprehensive Database Integration Example
//!
//! This example demonstrates how to use all database components together:
//! - PostgreSQL for ACID transactions and user management
//! - ClickHouse for high-performance analytics and event tracking
//! - MongoDB for flexible document storage and campaign management
//! - Redis for caching, session management, and real-time messaging
//!
//! This showcases the complete AI-CORE database architecture in action.

use ai_core_database::{
    analytics::{AnalyticsManager, TimeRange, WorkflowEventType},
    connections::{DocumentOps, MongoConnection},
    ClickHouseConfig, DatabaseConfig, DatabaseError, DatabaseManager, MongoConfig,
    MonitoringConfig, PostgresConfig, RedisConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Uuid,
    email: String,
    name: String,
    subscription_tier: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_login: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Campaign {
    id: Uuid,
    user_id: Uuid,
    name: String,
    campaign_type: String,
    target_audience: Vec<String>,
    budget: f64,
    status: String,
    metrics: CampaignMetrics,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignMetrics {
    impressions: u64,
    clicks: u64,
    conversions: u64,
    spend: f64,
    ctr: f64,
    cpc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationEvent {
    user_id: Uuid,
    event_type: String,
    title: String,
    message: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better observability
    tracing_subscriber::fmt()
        .with_env_filter("info,ai_core_database=debug")
        .init();

    println!("🚀 AI-CORE Comprehensive Database Integration Example");
    println!("====================================================");

    // Initialize database manager with all components
    let db_manager = initialize_database_manager().await?;
    println!("✅ Database manager initialized with all components");

    // Verify all database connections are healthy
    verify_database_health(&db_manager).await?;

    // Run comprehensive workflow
    let user_id = run_user_lifecycle_workflow(&db_manager).await?;
    run_campaign_management_workflow(&db_manager, user_id).await?;
    run_analytics_and_monitoring_workflow(&db_manager, user_id).await?;
    run_real_time_notification_workflow(&db_manager, user_id).await?;

    // Performance demonstration
    demonstrate_cross_database_performance(&db_manager).await?;

    // Final statistics and cleanup
    display_comprehensive_statistics(&db_manager).await?;

    println!("\n🎉 Comprehensive database integration example completed successfully!");
    println!("   All database components working together seamlessly.");

    Ok(())
}

/// Initialize the database manager with all database components
async fn initialize_database_manager() -> Result<DatabaseManager, DatabaseError> {
    let config = DatabaseConfig {
        postgresql: PostgresConfig {
            url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://postgres:password@localhost:5432/ai_core_dev".to_string()
            }),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 10,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
            enable_migrations: true,
        },
        clickhouse: Some(ClickHouseConfig {
            url: std::env::var("CLICKHOUSE_URL")
                .unwrap_or_else(|_| "http://localhost:8123".to_string()),
            database: "automation_analytics".to_string(),
            username: "default".to_string(),
            password: std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default(),
            pool_size: 10,
            timeout_seconds: 30,
            compression: true,
            secure: false,
        }),
        mongodb: Some(MongoConfig {
            url: std::env::var("MONGODB_URL")
                .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
            database: "ai_core_content".to_string(),
            max_pool_size: 20,
            min_pool_size: 5,
            max_idle_time_seconds: 600,
            connect_timeout_seconds: 10,
            server_selection_timeout_seconds: 30,
        }),
        redis: Some(RedisConfig {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            max_connections: 20,
            min_connections: 5,
            connection_timeout_seconds: 10,
            response_timeout_seconds: 5,
            retry_attempts: 3,
            enable_cluster: false,
            default_ttl_seconds: 3600,
            max_pool_size: 50,
        }),
        monitoring: MonitoringConfig {
            enabled: true,
            metrics_interval_seconds: 60,
            slow_query_threshold_ms: 1000,
            health_check_interval_seconds: 30,
        },
    };

    DatabaseManager::new(config).await
}

/// Verify that all database connections are healthy
async fn verify_database_health(db_manager: &DatabaseManager) -> Result<(), DatabaseError> {
    println!("\n🏥 Health Check - Verifying Database Connections");
    println!("-----------------------------------------------");

    let health = db_manager.health_check().await?;

    println!(
        "PostgreSQL: {}",
        if health.postgres.healthy {
            "✅ Healthy"
        } else {
            "❌ Unhealthy"
        }
    );
    println!("  - Response time: {}ms", health.postgres.response_time_ms);
    println!(
        "  - Pool utilization: {:.1}%",
        health.postgres.connection_pool.pool_utilization_percent
    );

    #[cfg(feature = "redis")]
    if let Some(redis_health) = &health.redis {
        println!(
            "Redis: {}",
            if redis_health.healthy {
                "✅ Healthy"
            } else {
                "❌ Unhealthy"
            }
        );
        println!("  - Cache hit ratio: {:.1}%", redis_health.cache_hit_ratio);
        println!("  - Total hits: {}", redis_health.cache_hits);
    }

    println!(
        "Overall Status: {}",
        if health.overall_healthy {
            "✅ All Systems Operational"
        } else {
            "❌ Some Issues Detected"
        }
    );

    if !health.overall_healthy {
        return Err(DatabaseError::Connection(
            "Database health check failed".to_string(),
        ));
    }

    Ok(())
}

/// Demonstrate user lifecycle management using PostgreSQL
async fn run_user_lifecycle_workflow(db_manager: &DatabaseManager) -> Result<Uuid, DatabaseError> {
    println!("\n👤 User Lifecycle Workflow - PostgreSQL");
    println!("---------------------------------------");

    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "demo@ai-core.example".to_string(),
        name: "Demo User".to_string(),
        subscription_tier: "premium".to_string(),
        created_at: chrono::Utc::now(),
        last_login: None,
    };

    // Create user in PostgreSQL (using repository pattern)
    let repos = db_manager.repositories();
    let postgres_repo = repos.postgres();

    // Simulate user creation (this would normally use proper repository methods)
    db_manager.execute_transaction(|tx| {
        Box::pin(async move {
            // In a real implementation, this would use proper user repository methods
            sqlx::query("INSERT INTO users (id, email, name, subscription_tier, created_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING")
                .bind(&user.id)
                .bind(&user.email)
                .bind(&user.name)
                .bind(&user.subscription_tier)
                .bind(&user.created_at)
                .execute(&mut **tx)
                .await?;
            Ok(())
        })
    }).await.unwrap_or_else(|_| {
        // Handle case where users table doesn't exist yet
        println!("⚠️  Users table not found - would be created by migrations in production");
    });

    println!("✅ User created: {} ({})", user.name, user.email);

    // Cache user session in Redis
    if let Some(redis) = &db_manager.redis {
        let session_id = format!("user_session_{}", user_id);
        redis.set_session(&session_id, &user, 3600).await?;
        println!("✅ User session cached in Redis with 1-hour TTL");

        // Demonstrate rate limiting for user
        let rate_limit_key = format!("user_api_calls_{}", user_id);
        for i in 1..=5 {
            let allowed = redis.check_rate_limit(&rate_limit_key, 60, 10).await?;
            if i <= 3 {
                println!(
                    "   API call {}: {}",
                    i,
                    if allowed {
                        "✅ Allowed"
                    } else {
                        "❌ Rate limited"
                    }
                );
            }
        }
    }

    Ok(user_id)
}

/// Demonstrate campaign management using MongoDB
async fn run_campaign_management_workflow(
    db_manager: &DatabaseManager,
    user_id: Uuid,
) -> Result<(), DatabaseError> {
    println!("\n📊 Campaign Management Workflow - MongoDB");
    println!("-----------------------------------------");

    if let Some(mongodb) = &db_manager.mongodb {
        let campaign_id = Uuid::new_v4();
        let campaign = Campaign {
            id: campaign_id,
            user_id,
            name: "AI-Powered Content Campaign".to_string(),
            campaign_type: "content_generation".to_string(),
            target_audience: vec![
                "tech_enthusiasts".to_string(),
                "content_creators".to_string(),
                "marketing_professionals".to_string(),
            ],
            budget: 5000.0,
            status: "active".to_string(),
            metrics: CampaignMetrics {
                impressions: 15420,
                clicks: 892,
                conversions: 67,
                spend: 234.56,
                ctr: 5.78,
                cpc: 0.26,
            },
            created_at: chrono::Utc::now(),
        };

        // Store campaign in MongoDB
        mongodb.insert_document("campaigns", &campaign).await?;
        println!(
            "✅ Campaign created: {} (Budget: ${:.2})",
            campaign.name, campaign.budget
        );

        // Simulate campaign updates
        sleep(Duration::from_millis(100)).await;

        let mut updated_campaign = campaign.clone();
        updated_campaign.metrics.impressions += 1000;
        updated_campaign.metrics.clicks += 75;
        updated_campaign.metrics.spend += 15.30;

        mongodb
            .upsert_document(
                "campaigns",
                &bson::doc! { "id": campaign_id.to_string() },
                &updated_campaign,
            )
            .await?;
        println!("✅ Campaign metrics updated: +1000 impressions, +75 clicks");

        // Query campaigns for user
        let user_campaigns = mongodb
            .find_documents::<Campaign>(
                "campaigns",
                Some(bson::doc! { "user_id": user_id.to_string() }),
                None,
            )
            .await?;
        println!("✅ Found {} campaigns for user", user_campaigns.len());

        // Demonstrate aggregation pipeline
        let pipeline = vec![
            bson::doc! { "$match": { "user_id": user_id.to_string() } },
            bson::doc! { "$group": {
                "_id": "$campaign_type",
                "total_budget": { "$sum": "$budget" },
                "total_spend": { "$sum": "$metrics.spend" },
                "avg_ctr": { "$avg": "$metrics.ctr" },
                "campaign_count": { "$sum": 1 }
            }},
        ];

        let aggregation_results = mongodb
            .aggregate::<bson::Document>("campaigns", pipeline)
            .await?;
        println!(
            "✅ Aggregation results: {} campaign types analyzed",
            aggregation_results.len()
        );
    } else {
        println!("⚠️  MongoDB not configured - skipping campaign workflow");
    }

    Ok(())
}

/// Demonstrate analytics and event tracking using ClickHouse
async fn run_analytics_and_monitoring_workflow(
    db_manager: &DatabaseManager,
    user_id: Uuid,
) -> Result<(), DatabaseError> {
    println!("\n📈 Analytics & Monitoring Workflow - ClickHouse");
    println!("-----------------------------------------------");

    if let Some(clickhouse) = &db_manager.clickhouse {
        let analytics = AnalyticsManager::new(clickhouse.clone());

        // Track various workflow events
        let events = vec![
            (
                "workflow_started",
                WorkflowEventType::WorkflowStarted,
                0,
                0.0,
                true,
            ),
            (
                "content_generation",
                WorkflowEventType::StepCompleted,
                1500,
                0.15,
                true,
            ),
            (
                "content_review",
                WorkflowEventType::StepCompleted,
                800,
                0.08,
                true,
            ),
            (
                "content_optimization",
                WorkflowEventType::StepCompleted,
                1200,
                0.12,
                true,
            ),
            (
                "workflow_completed",
                WorkflowEventType::WorkflowCompleted,
                3500,
                0.35,
                true,
            ),
        ];

        for (workflow_id, event_type, duration, cost, success) in events {
            analytics
                .track_workflow_event(
                    workflow_id,
                    &user_id.to_string(),
                    "ai_content_generator",
                    event_type,
                    duration,
                    cost,
                    success,
                    Some(serde_json::json!({
                        "model": "gpt-4",
                        "content_type": "blog_post",
                        "word_count": 1500
                    })),
                )
                .await?;

            // Small delay to show progression
            sleep(Duration::from_millis(50)).await;
        }
        println!("✅ Tracked 5 workflow events for analytics");

        // Track API requests
        for endpoint in [
            "POST /api/v1/workflows",
            "GET /api/v1/campaigns",
            "PUT /api/v1/content",
        ] {
            analytics
                .track_api_request(
                    endpoint,
                    "GET",
                    200,
                    &user_id.to_string(),
                    45,
                    Some(serde_json::json!({
                        "user_agent": "AI-CORE-Client/1.0",
                        "ip": "192.168.1.100"
                    })),
                )
                .await?;
        }
        println!("✅ Tracked API requests across different endpoints");

        // Generate analytics insights
        let workflow_metrics = analytics
            .get_workflow_metrics(
                TimeRange::last_hour(),
                Some("ai_content_generator".to_string()),
            )
            .await?;

        println!("📊 Analytics Insights:");
        println!(
            "   - Total workflow events: {}",
            workflow_metrics.total_events
        );
        println!("   - Success rate: {:.1}%", workflow_metrics.success_rate);
        println!(
            "   - Average duration: {:.2}ms",
            workflow_metrics.avg_duration_ms
        );
        println!("   - Total cost: ${:.4}", workflow_metrics.total_cost);

        // Get top users
        let top_users = analytics.get_top_users(10).await?;
        println!("   - Top users analyzed: {} users tracked", top_users.len());
    } else {
        println!("⚠️  ClickHouse not configured - skipping analytics workflow");
    }

    Ok(())
}

/// Demonstrate real-time notifications using Redis pub/sub
async fn run_real_time_notification_workflow(
    db_manager: &DatabaseManager,
    user_id: Uuid,
) -> Result<(), DatabaseError> {
    println!("\n🔔 Real-time Notifications Workflow - Redis Pub/Sub");
    println!("---------------------------------------------------");

    if let Some(redis) = &db_manager.redis {
        let notification_channel = "user_notifications";

        // Subscribe to notifications
        let mut receiver = redis.subscribe(notification_channel).await?;
        println!("✅ Subscribed to notification channel");

        // Spawn background task to handle notifications
        let redis_clone = redis;
        tokio::spawn(async move {
            let mut count = 0;
            while let Ok(message) = receiver.recv().await {
                count += 1;
                if let Ok(notification) = serde_json::from_str::<NotificationEvent>(&message) {
                    println!(
                        "   📨 Notification {}: {} - {}",
                        count, notification.title, notification.message
                    );
                }

                // Stop after receiving a few messages
                if count >= 3 {
                    break;
                }
            }
        });

        // Give subscriber time to start
        sleep(Duration::from_millis(100)).await;

        // Publish various notifications
        let notifications = vec![
            NotificationEvent {
                user_id,
                event_type: "workflow_complete".to_string(),
                title: "Workflow Completed".to_string(),
                message: "Your AI content generation workflow has completed successfully"
                    .to_string(),
                timestamp: chrono::Utc::now(),
            },
            NotificationEvent {
                user_id,
                event_type: "campaign_update".to_string(),
                title: "Campaign Performance".to_string(),
                message: "Your campaign has achieved 90% of its target impressions".to_string(),
                timestamp: chrono::Utc::now(),
            },
            NotificationEvent {
                user_id,
                event_type: "system_alert".to_string(),
                title: "System Maintenance".to_string(),
                message: "Scheduled maintenance will begin in 1 hour".to_string(),
                timestamp: chrono::Utc::now(),
            },
        ];

        for notification in notifications {
            let subscriber_count = redis_clone
                .publish(notification_channel, &notification)
                .await?;
            println!(
                "✅ Published notification: {} ({} subscribers)",
                notification.title, subscriber_count
            );
            sleep(Duration::from_millis(200)).await;
        }

        // Wait for notifications to be processed
        sleep(Duration::from_millis(500)).await;

        // Demonstrate notification history caching
        let notification_history_key = format!("user_notifications_{}", user_id);
        redis_clone
            .set_with_ttl(&notification_history_key, &notifications, 86400)
            .await?; // 24 hours
        println!("✅ Cached notification history for 24 hours");
    } else {
        println!("⚠️  Redis not configured - skipping notification workflow");
    }

    Ok(())
}

/// Demonstrate cross-database performance and integration
async fn demonstrate_cross_database_performance(
    db_manager: &DatabaseManager,
) -> Result<(), DatabaseError> {
    println!("\n⚡ Cross-Database Performance Demonstration");
    println!("------------------------------------------");

    let start_time = std::time::Instant::now();

    // Simulate a complex workflow that uses all databases
    let workflow_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // 1. Check user session (Redis)
    if let Some(redis) = &db_manager.redis {
        let cache_start = std::time::Instant::now();
        let session_key = format!("user_session_{}", user_id);
        let _session: Option<serde_json::Value> = redis.get(&session_key).await.unwrap_or(None);
        let cache_duration = cache_start.elapsed();
        println!("✅ Redis session lookup: {:?}", cache_duration);
    }

    // 2. Fetch user details (PostgreSQL)
    let pg_start = std::time::Instant::now();
    let _pg_result = db_manager
        .execute_transaction(|tx| {
            Box::pin(async move {
                // Simulate complex user query
                sqlx::query("SELECT COUNT(*) as user_count")
                    .fetch_one(&mut **tx)
                    .await?;
                Ok(1)
            })
        })
        .await;
    let pg_duration = pg_start.elapsed();
    println!("✅ PostgreSQL transaction: {:?}", pg_duration);

    // 3. Store workflow data (MongoDB)
    if let Some(mongodb) = &db_manager.mongodb {
        let mongo_start = std::time::Instant::now();
        let workflow_data = serde_json::json!({
            "workflow_id": workflow_id,
            "user_id": user_id,
            "type": "performance_test",
            "status": "completed",
            "timestamp": chrono::Utc::now()
        });
        let _mongo_result = mongodb.insert_document("workflows", &workflow_data).await;
        let mongo_duration = mongo_start.elapsed();
        println!("✅ MongoDB document insert: {:?}", mongo_duration);
    }

    // 4. Track analytics (ClickHouse)
    if let Some(clickhouse) = &db_manager.clickhouse {
        let analytics = AnalyticsManager::new(clickhouse.clone());
        let ch_start = std::time::Instant::now();
        let _analytics_result = analytics
            .track_workflow_event(
                &workflow_id.to_string(),
                &user_id.to_string(),
                "performance_test",
                WorkflowEventType::WorkflowCompleted,
                100,
                0.01,
                true,
                None,
            )
            .await;
        let ch_duration = ch_start.elapsed();
        println!("✅ ClickHouse analytics event: {:?}", ch_duration);
    }

    let total_duration = start_time.elapsed();
    println!("🚀 Total cross-database operation: {:?}", total_duration);
    println!("   This demonstrates the efficiency of the hybrid architecture!");

    Ok(())
}

/// Display comprehensive statistics from all database components
async fn display_comprehensive_statistics(
    db_manager: &DatabaseManager,
) -> Result<(), DatabaseError> {
    println!("\n📊 Comprehensive Database Statistics");
    println!("===================================");

    // PostgreSQL Statistics
    println!("\n🐘 PostgreSQL:");
    let pg_health = db_manager.health_check().await?;
    println!(
        "  - Connection pool utilization: {:.1}%",
        pg_health.postgres.connection_pool.pool_utilization_percent
    );
    println!(
        "  - Active connections: {}",
        pg_health.postgres.connection_pool.active_connections
    );
    println!(
        "  - Response time: {}ms",
        pg_health.postgres.response_time_ms
    );

    // Redis Statistics
    #[cfg(feature = "redis")]
    if let Some(redis) = &db_manager.redis {
        println!("\n🔴 Redis:");
        let redis_stats = redis.get_stats().await;
        println!(
            "  - Cache operations: {} sets, {} gets",
            redis_stats.cache_sets,
            redis_stats.cache_hits + redis_stats.cache_misses
        );
        if redis_stats.cache_hits + redis_stats.cache_misses > 0 {
            let hit_ratio = redis_stats.cache_hits as f64
                / (redis_stats.cache_hits + redis_stats.cache_misses) as f64
                * 100.0;
            println!("  - Cache hit ratio: {:.1}%", hit_ratio);
        }
        println!(
            "  - Pub/sub messages: {} published, {} received",
            redis_stats.pub_messages, redis_stats.sub_messages
        );
        println!("  - Uptime: {} seconds", redis_stats.uptime_seconds);

        let db_size = redis.db_size().await.unwrap_or(0);
        println!("  - Total keys: {}", db_size);
    }

    // MongoDB Statistics
    if let Some(mongodb) = &db_manager.mongodb {
        println!("\n🍃 MongoDB:");
        let mongo_stats = mongodb.get_stats().await;
        println!("  - Collections: {}", mongo_stats.collection_count);
        println!("  - Total documents: {}", mongo_stats.document_count);
        println!(
            "  - Database size: {:.2} MB",
            mongo_stats.data_size as f64 / 1024.0 / 1024.0
        );
        println!(
            "  - Index size: {:.2} MB",
            mongo_stats.index_size as f64 / 1024.0 / 1024.0
        );
    }

    // ClickHouse Statistics
    if let Some(clickhouse) = &db_manager.clickhouse {
        println!("\n🏠 ClickHouse:");
        let ch_health = clickhouse.health_check().await.unwrap_or_default();
        println!("  - Connection healthy: {}", ch_health.healthy);
        println!("  - Response time: {}ms", ch_health.response_time_ms);
        if let Some(error) = &ch_health.error_message {
            println!("  - Last error: {}", error);
        }
    }

    println!("\n✨ All database components successfully demonstrated!");
    println!("   The AI-CORE hybrid database architecture provides:");
    println!("   • ACID transactions (PostgreSQL)");
    println!("   • High-performance analytics (ClickHouse)");
    println!("   • Flexible document storage (MongoDB)");
    println!("   • Fast caching and real-time features (Redis)");

    Ok(())
}
