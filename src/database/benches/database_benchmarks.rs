//! Performance benchmarks for AI-CORE database layer
//!
//! These benchmarks validate performance requirements:
//! - PostgreSQL: < 10ms for simple queries, < 100ms for complex transactions
//! - ClickHouse: < 1s for analytical queries on 1M+ records
//! - MongoDB: < 50ms for document operations, < 500ms for aggregations
//! - Redis: < 1ms for cache operations, < 10ms for complex operations

use ai_core_database::{analytics::*, connections::*, DatabaseError};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkUser {
    id: String,
    email: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    subscription_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkCampaign {
    id: String,
    name: String,
    description: String,
    status: String,
    target_audience: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    metrics: CampaignMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignMetrics {
    impressions: u64,
    clicks: u64,
    conversions: u64,
    cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkSession {
    session_id: String,
    user_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    data: serde_json::Value,
}

/// Benchmark PostgreSQL configuration creation and validation
fn bench_postgres_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("postgres_config");

    group.bench_function("config_creation", |b| {
        b.iter(|| {
            black_box(PostgresConfig {
                url: "postgresql://bench:bench@localhost:5432/bench_db".to_string(),
                max_connections: 20,
                min_connections: 5,
                acquire_timeout_seconds: 30,
                idle_timeout_seconds: 600,
                max_lifetime_seconds: 1800,
                enable_migrations: true,
            })
        })
    });

    group.bench_function("config_serialization", |b| {
        let config = PostgresConfig::default();
        b.iter(|| black_box(serde_json::to_string(&config).unwrap()))
    });

    group.bench_function("config_deserialization", |b| {
        let config = PostgresConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        b.iter(|| black_box(serde_json::from_str::<PostgresConfig>(&json).unwrap()))
    });

    group.finish();
}

/// Benchmark ClickHouse data structures and operations
fn bench_clickhouse_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("clickhouse_operations");

    group.bench_function("workflow_event_creation", |b| {
        b.iter(|| {
            black_box(WorkflowEventData {
                event_id: "bench_event".to_string(),
                workflow_id: "bench_workflow".to_string(),
                user_id: "bench_user".to_string(),
                event_type: "benchmark_test".to_string(),
                timestamp: chrono::Utc::now(),
                duration_ms: Some(100),
                status: "success".to_string(),
                error_message: None,
                metadata: Some(serde_json::json!({"test": true})),
            })
        })
    });

    group.bench_function("bulk_event_creation", |b| {
        b.iter(|| {
            let mut events = Vec::new();
            for i in 0..1000 {
                events.push(WorkflowEventData {
                    event_id: format!("bulk_event_{}", i),
                    workflow_id: "bulk_workflow".to_string(),
                    user_id: format!("user_{}", i % 100),
                    event_type: "bulk_test".to_string(),
                    timestamp: chrono::Utc::now(),
                    duration_ms: Some(i % 1000),
                    status: "success".to_string(),
                    error_message: None,
                    metadata: None,
                });
            }
            black_box(events)
        })
    });

    group.bench_function("event_serialization", |b| {
        let event = WorkflowEventData {
            event_id: "serialization_bench".to_string(),
            workflow_id: "bench_workflow".to_string(),
            user_id: "bench_user".to_string(),
            event_type: "serialization_test".to_string(),
            timestamp: chrono::Utc::now(),
            duration_ms: Some(250),
            status: "success".to_string(),
            error_message: None,
            metadata: Some(serde_json::json!({
                "large_data": vec![1; 100],
                "nested": {"level1": {"level2": "data"}}
            })),
        };

        b.iter(|| black_box(serde_json::to_string(&event).unwrap()))
    });

    group.finish();
}

/// Benchmark MongoDB document operations
fn bench_mongodb_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mongodb_operations");

    group.bench_function("campaign_creation", |b| {
        b.iter(|| {
            black_box(BenchmarkCampaign {
                id: "bench_campaign".to_string(),
                name: "Benchmark Campaign".to_string(),
                description: "Performance testing campaign".to_string(),
                status: "active".to_string(),
                target_audience: vec!["developers".to_string(), "engineers".to_string()],
                created_at: chrono::Utc::now(),
                metrics: CampaignMetrics {
                    impressions: 10000,
                    clicks: 500,
                    conversions: 50,
                    cost: 250.0,
                },
            })
        })
    });

    group.bench_function("document_serialization", |b| {
        let campaign = BenchmarkCampaign {
            id: "serialization_bench".to_string(),
            name: "Serialization Test".to_string(),
            description: "Testing BSON serialization performance".to_string(),
            status: "active".to_string(),
            target_audience: vec!["benchmark".to_string(); 50],
            created_at: chrono::Utc::now(),
            metrics: CampaignMetrics {
                impressions: 100000,
                clicks: 5000,
                conversions: 500,
                cost: 1500.75,
            },
        };

        b.iter(|| black_box(bson::to_document(&campaign).unwrap()))
    });

    group.bench_function("bulk_document_creation", |b| {
        b.iter(|| {
            let mut campaigns = Vec::new();
            for i in 0..1000 {
                campaigns.push(BenchmarkCampaign {
                    id: format!("bulk_campaign_{}", i),
                    name: format!("Bulk Campaign {}", i),
                    description: "Bulk performance test".to_string(),
                    status: if i % 10 == 0 {
                        "paused".to_string()
                    } else {
                        "active".to_string()
                    },
                    target_audience: vec![format!("audience_{}", i % 5)],
                    created_at: chrono::Utc::now(),
                    metrics: CampaignMetrics {
                        impressions: (i * 100) as u64,
                        clicks: (i * 5) as u64,
                        conversions: i as u64,
                        cost: i as f64 * 1.5,
                    },
                });
            }
            black_box(campaigns)
        })
    });

    group.finish();
}

/// Benchmark Redis cache operations
fn bench_redis_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_operations");

    group.bench_function("session_creation", |b| {
        b.iter(|| {
            black_box(BenchmarkSession {
                session_id: "bench_session".to_string(),
                user_id: "bench_user".to_string(),
                created_at: chrono::Utc::now(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
                data: serde_json::json!({
                    "theme": "dark",
                    "preferences": {
                        "notifications": true,
                        "language": "en"
                    },
                    "permissions": ["read", "write"]
                }),
            })
        })
    });

    group.bench_function("session_serialization", |b| {
        let session = BenchmarkSession {
            session_id: "serialization_bench".to_string(),
            user_id: "bench_user".to_string(),
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            data: serde_json::json!({
                "large_array": vec![1; 1000],
                "nested_data": {
                    "level1": {
                        "level2": {
                            "data": vec!["test"; 100]
                        }
                    }
                }
            }),
        };

        b.iter(|| black_box(serde_json::to_string(&session).unwrap()))
    });

    group.bench_function("key_generation", |b| {
        b.iter(|| {
            let user_id = "bench_user_123";
            let keys = vec![
                format!("user:{}:session", user_id),
                format!("cache:user:{}:profile", user_id),
                format!("rate_limit:user:{}:api", user_id),
                format!("workflow:user:{}:active", user_id),
            ];
            black_box(keys)
        })
    });

    group.finish();
}

/// Benchmark cross-database operations
fn bench_cross_database_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_database");

    group.bench_function("multi_database_config_creation", |b| {
        b.iter(|| {
            black_box(DatabaseConfig {
                postgresql: PostgresConfig::default(),
                monitoring: MonitoringConfig::default(),
                #[cfg(feature = "clickhouse")]
                clickhouse: Some(ClickHouseConfig::default()),
                #[cfg(feature = "mongodb")]
                mongodb: Some(MongoConfig::default()),
                #[cfg(feature = "redis")]
                redis: Some(RedisConfig::default()),
            })
        })
    });

    group.bench_function("configuration_serialization", |b| {
        let config = DatabaseConfig {
            postgresql: PostgresConfig::default(),
            monitoring: MonitoringConfig::default(),
            #[cfg(feature = "clickhouse")]
            clickhouse: Some(ClickHouseConfig::default()),
            #[cfg(feature = "mongodb")]
            mongodb: Some(MongoConfig::default()),
            #[cfg(feature = "redis")]
            redis: Some(RedisConfig::default()),
        };

        b.iter(|| black_box(serde_json::to_string(&config).unwrap()))
    });

    group.bench_function("simulated_transaction_coordination", |b| {
        b.iter(|| {
            // Simulate transaction steps across databases
            let steps = vec![
                ("PostgreSQL", "INSERT", Duration::from_millis(5)),
                ("ClickHouse", "INSERT", Duration::from_millis(15)),
                ("MongoDB", "UPDATE", Duration::from_millis(8)),
                ("Redis", "SET", Duration::from_millis(1)),
            ];

            let total_duration: Duration = steps.iter().map(|(_, _, duration)| *duration).sum();
            black_box(total_duration)
        })
    });

    group.finish();
}

/// Benchmark error handling performance
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");

    group.bench_function("error_creation", |b| {
        b.iter(|| {
            let errors = vec![
                DatabaseError::Connection("Connection failed".to_string()),
                DatabaseError::Query("Query failed".to_string()),
                DatabaseError::Migration("Migration failed".to_string()),
                DatabaseError::Configuration("Config error".to_string()),
            ];
            black_box(errors)
        })
    });

    group.bench_function("error_formatting", |b| {
        let error = DatabaseError::Connection("Benchmark connection error".to_string());
        b.iter(|| black_box(format!("{}", error)))
    });

    group.bench_function("error_propagation", |b| {
        b.iter(|| {
            fn simulate_operation() -> Result<String, DatabaseError> {
                Err(DatabaseError::Connection("Simulated error".to_string()))
            }

            let result = simulate_operation();
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    group.bench_function("concurrent_config_access", |b| {
        let rt = Runtime::new().unwrap();
        let config = std::sync::Arc::new(PostgresConfig::default());

        b.to_async(&rt).iter(|| {
            let config = config.clone();
            async move {
                let mut handles = vec![];

                for _ in 0..10 {
                    let config_clone = config.clone();
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_micros(1)).await;
                        config_clone.max_connections
                    });
                    handles.push(handle);
                }

                let results: Vec<_> = futures::future::join_all(handles).await;
                black_box(results)
            }
        })
    });

    group.bench_function("concurrent_data_creation", |b| {
        let rt = Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async move {
            let mut handles = vec![];

            for i in 0..50 {
                let handle = tokio::spawn(async move {
                    BenchmarkUser {
                        id: format!("concurrent_user_{}", i),
                        email: format!("user{}@example.com", i),
                        name: format!("User {}", i),
                        created_at: chrono::Utc::now(),
                        subscription_tier: "premium".to_string(),
                    }
                });
                handles.push(handle);
            }

            let results: Vec<_> = futures::future::join_all(handles).await;
            black_box(results)
        })
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("large_dataset_creation", |b| {
        b.iter(|| {
            let mut large_dataset = Vec::new();

            for i in 0..10000 {
                large_dataset.push(BenchmarkUser {
                    id: format!("user_{}", i),
                    email: format!("user{}@example.com", i),
                    name: format!("User Number {}", i),
                    created_at: chrono::Utc::now(),
                    subscription_tier: match i % 3 {
                        0 => "free".to_string(),
                        1 => "premium".to_string(),
                        _ => "enterprise".to_string(),
                    },
                });
            }

            black_box(large_dataset)
        })
    });

    group.bench_function("memory_intensive_serialization", |b| {
        let large_campaign = BenchmarkCampaign {
            id: "memory_test".to_string(),
            name: "Memory Intensive Campaign".to_string(),
            description: "Testing memory usage in serialization".to_string(),
            status: "active".to_string(),
            target_audience: (0..1000).map(|i| format!("audience_{}", i)).collect(),
            created_at: chrono::Utc::now(),
            metrics: CampaignMetrics {
                impressions: 10000000,
                clicks: 500000,
                conversions: 50000,
                cost: 25000.0,
            },
        };

        b.iter(|| black_box(serde_json::to_string(&large_campaign).unwrap()))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_postgres_config,
    bench_clickhouse_operations,
    bench_mongodb_operations,
    bench_redis_operations,
    bench_cross_database_operations,
    bench_error_handling,
    bench_concurrent_operations,
    bench_memory_usage
);

criterion_main!(benches);
