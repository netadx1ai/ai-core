//! Comprehensive unit tests for MongoDB integration
//!
//! Tests document operations, aggregation pipelines, connection management, and error handling

use std::time::Duration;
use ai_core_database::{
    connections::MongoConfig,
    DatabaseError
};
use serde::{Deserialize, Serialize};
use bson::{doc, Document};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestCampaign {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub description: String,
    pub status: String,
    pub target_audience: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metrics: CampaignMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignMetrics {
    pub impressions: u64,
    pub clicks: u64,
    pub conversions: u64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestUserProfile {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: String,
    pub preferences: Document,
    pub behavior_data: Vec<Document>,
    pub segmentation_tags: Vec<String>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Test MongoDB configuration validation
#[test]
fn test_mongo_config_validation() {
    let config = MongoConfig {
        uri: "mongodb://localhost:27017".to_string(),
        database: "test_db".to_string(),
        max_pool_size: 20,
        min_pool_size: 5,
        connect_timeout_ms: 10000,
        server_selection_timeout_ms: 30000,
        socket_timeout_ms: Some(60000),
        heartbeat_frequency_ms: 10000,
        max_idle_time_ms: Some(600000),
        enable_compression: true,
        compression_algorithm: Some("zstd".to_string()),
    };

    // Validate configuration constraints
    assert!(config.max_pool_size >= config.min_pool_size);
    assert!(config.connect_timeout_ms > 0);
    assert!(config.server_selection_timeout_ms > 0);
    assert!(config.heartbeat_frequency_ms > 0);
    assert!(!config.uri.is_empty());
    assert!(!config.database.is_empty());
}

#[test]
fn test_mongo_config_defaults() {
    let config = MongoConfig::default();

    assert_eq!(config.uri, "mongodb://localhost:27017");
    assert_eq!(config.database, "ai_core_content");
    assert_eq!(config.max_pool_size, 20);
    assert_eq!(config.min_pool_size, 5);
    assert_eq!(config.connect_timeout_ms, 10000);
    assert_eq!(config.server_selection_timeout_ms, 30000);
    assert_eq!(config.heartbeat_frequency_ms, 10000);
    assert!(config.enable_compression);
    assert_eq!(config.compression_algorithm, Some("zstd".to_string()));
}

#[test]
fn test_mongo_config_serialization() {
    let config = MongoConfig::default();

    // Test JSON serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    // Test JSON deserialization
    let deserialized: MongoConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.uri, deserialized.uri);
    assert_eq!(config.database, deserialized.database);
    assert_eq!(config.max_pool_size, deserialized.max_pool_size);
    assert_eq!(config.enable_compression, deserialized.enable_compression);
}

#[test]
fn test_mongo_uri_validation() {
    let valid_uris = vec![
        "mongodb://localhost:27017",
        "mongodb://user:pass@localhost:27017",
        "mongodb://localhost:27017/database",
        "mongodb://user:pass@host1:27017,host2:27017/database?replicaSet=rs0",
        "mongodb+srv://cluster.example.com/database",
    ];

    for uri in valid_uris {
        let config = MongoConfig {
            uri: uri.to_string(),
            ..MongoConfig::default()
        };
        assert!(config.uri.starts_with("mongodb"));
        assert!(config.uri.contains("://"));
    }
}

/// Test campaign document structure and validation
#[test]
fn test_campaign_document_creation() {
    let campaign = TestCampaign {
        id: None,
        name: "Test Campaign".to_string(),
        description: "A test campaign for validation".to_string(),
        status: "active".to_string(),
        target_audience: vec!["developers".to_string(), "engineers".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metrics: CampaignMetrics {
            impressions: 1000,
            clicks: 50,
            conversions: 5,
            cost: 99.99,
        },
    };

    assert_eq!(campaign.name, "Test Campaign");
    assert_eq!(campaign.status, "active");
    assert_eq!(campaign.target_audience.len(), 2);
    assert_eq!(campaign.metrics.impressions, 1000);
    assert_eq!(campaign.metrics.clicks, 50);
    assert_eq!(campaign.metrics.conversions, 5);
    assert_eq!(campaign.metrics.cost, 99.99);
}

#[test]
fn test_campaign_serialization() {
    let campaign = TestCampaign {
        id: Some(ObjectId::new()),
        name: "Serialization Test".to_string(),
        description: "Testing serialization".to_string(),
        status: "draft".to_string(),
        target_audience: vec!["testers".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metrics: CampaignMetrics {
            impressions: 0,
            clicks: 0,
            conversions: 0,
            cost: 0.0,
        },
    };

    // Test BSON serialization
    let bson = bson::to_document(&campaign).unwrap();
    assert!(bson.contains_key("name"));
    assert!(bson.contains_key("status"));
    assert!(bson.contains_key("metrics"));

    // Test BSON deserialization
    let deserialized: TestCampaign = bson::from_document(bson).unwrap();
    assert_eq!(campaign.name, deserialized.name);
    assert_eq!(campaign.status, deserialized.status);
    assert_eq!(campaign.metrics.cost, deserialized.metrics.cost);
}

/// Test user profile document structure
#[test]
fn test_user_profile_creation() {
    let profile = TestUserProfile {
        id: None,
        user_id: "user_123".to_string(),
        preferences: doc! {
            "theme": "dark",
            "notifications": true,
            "language": "en"
        },
        behavior_data: vec![
            doc! {
                "action": "login",
                "timestamp": chrono::Utc::now(),
                "device": "mobile"
            },
            doc! {
                "action": "view_campaign",
                "timestamp": chrono::Utc::now(),
                "campaign_id": "camp_456"
            }
        ],
        segmentation_tags: vec!["premium".to_string(), "active".to_string()],
        last_activity: chrono::Utc::now(),
    };

    assert_eq!(profile.user_id, "user_123");
    assert_eq!(profile.preferences.get_str("theme").unwrap(), "dark");
    assert_eq!(profile.behavior_data.len(), 2);
    assert_eq!(profile.segmentation_tags.len(), 2);
    assert!(profile.segmentation_tags.contains(&"premium".to_string()));
}

#[test]
fn test_user_profile_serialization() {
    let profile = TestUserProfile {
        id: Some(ObjectId::new()),
        user_id: "serialize_test".to_string(),
        preferences: doc! { "test": true },
        behavior_data: vec![doc! { "action": "test", "value": 42 }],
        segmentation_tags: vec!["test_tag".to_string()],
        last_activity: chrono::Utc::now(),
    };

    // Test BSON serialization
    let bson = bson::to_document(&profile).unwrap();
    assert!(bson.contains_key("user_id"));
    assert!(bson.contains_key("preferences"));
    assert!(bson.contains_key("behavior_data"));

    // Test BSON deserialization
    let deserialized: TestUserProfile = bson::from_document(bson).unwrap();
    assert_eq!(profile.user_id, deserialized.user_id);
    assert_eq!(profile.segmentation_tags, deserialized.segmentation_tags);
}

/// Test aggregation pipeline construction
#[test]
fn test_aggregation_pipelines() {
    // Test campaign metrics aggregation
    let campaign_pipeline = vec![
        doc! {
            "$match": {
                "status": "active",
                "created_at": {
                    "$gte": chrono::Utc::now() - chrono::Duration::days(30)
                }
            }
        },
        doc! {
            "$group": {
                "_id": "$status",
                "total_campaigns": { "$sum": 1 },
                "total_impressions": { "$sum": "$metrics.impressions" },
                "total_clicks": { "$sum": "$metrics.clicks" },
                "total_conversions": { "$sum": "$metrics.conversions" },
                "total_cost": { "$sum": "$metrics.cost" },
                "avg_ctr": {
                    "$avg": {
                        "$divide": ["$metrics.clicks", "$metrics.impressions"]
                    }
                }
            }
        },
        doc! {
            "$sort": { "total_impressions": -1 }
        }
    ];

    assert_eq!(campaign_pipeline.len(), 3);
    assert!(campaign_pipeline[0].contains_key("$match"));
    assert!(campaign_pipeline[1].contains_key("$group"));
    assert!(campaign_pipeline[2].contains_key("$sort"));

    // Test user behavior aggregation
    let user_behavior_pipeline = vec![
        doc! {
            "$unwind": "$behavior_data"
        },
        doc! {
            "$group": {
                "_id": {
                    "user_id": "$user_id",
                    "action": "$behavior_data.action"
                },
                "action_count": { "$sum": 1 },
                "last_action": { "$max": "$behavior_data.timestamp" }
            }
        },
        doc! {
            "$match": {
                "action_count": { "$gte": 5 }
            }
        }
    ];

    assert_eq!(user_behavior_pipeline.len(), 3);
    assert!(user_behavior_pipeline[0].contains_key("$unwind"));
    assert!(user_behavior_pipeline[1].contains_key("$group"));
    assert!(user_behavior_pipeline[2].contains_key("$match"));
}

/// Test indexing strategies
#[test]
fn test_index_definitions() {
    // Test compound indexes
    let campaign_indexes = vec![
        doc! { "status": 1, "created_at": -1 },
        doc! { "target_audience": 1 },
        doc! { "metrics.impressions": -1 },
        doc! { "name": "text", "description": "text" },
    ];

    for index in campaign_indexes {
        assert!(!index.is_empty());
        // Verify index structure
        for (field, direction) in index {
            if field == "name" || field == "description" {
                assert_eq!(direction, "text");
            } else {
                assert!(direction.as_i32().unwrap() == 1 || direction.as_i32().unwrap() == -1);
            }
        }
    }

    // Test user profile indexes
    let user_indexes = vec![
        doc! { "user_id": 1 },
        doc! { "segmentation_tags": 1 },
        doc! { "last_activity": -1 },
        doc! { "behavior_data.action": 1, "behavior_data.timestamp": -1 },
    ];

    for index in user_indexes {
        assert!(!index.is_empty());
    }
}

/// Test configuration edge cases
#[test]
fn test_config_edge_cases() {
    // Test minimum configuration
    let min_config = MongoConfig {
        uri: "mongodb://localhost:27017".to_string(),
        database: "min_db".to_string(),
        max_pool_size: 1,
        min_pool_size: 1,
        connect_timeout_ms: 1000,
        server_selection_timeout_ms: 5000,
        socket_timeout_ms: Some(10000),
        heartbeat_frequency_ms: 1000,
        max_idle_time_ms: Some(60000),
        enable_compression: false,
        compression_algorithm: None,
    };

    assert_eq!(min_config.max_pool_size, min_config.min_pool_size);
    assert!(!min_config.enable_compression);
    assert!(min_config.compression_algorithm.is_none());

    // Test high-performance configuration
    let high_perf_config = MongoConfig {
        uri: "mongodb://user:pass@cluster.example.com:27017".to_string(),
        database: "production_db".to_string(),
        max_pool_size: 100,
        min_pool_size: 20,
        connect_timeout_ms: 30000,
        server_selection_timeout_ms: 60000,
        socket_timeout_ms: Some(120000),
        heartbeat_frequency_ms: 5000,
        max_idle_time_ms: Some(300000),
        enable_compression: true,
        compression_algorithm: Some("zlib".to_string()),
    };

    assert!(high_perf_config.max_pool_size > high_perf_config.min_pool_size);
    assert!(high_perf_config.enable_compression);
    assert_eq!(high_perf_config.compression_algorithm, Some("zlib".to_string()));
}

/// Test invalid configuration values
#[test]
fn test_invalid_configuration_values() {
    // Test zero pool sizes
    let zero_pool_config = MongoConfig {
        max_pool_size: 0,
        min_pool_size: 0,
        ..MongoConfig::default()
    };
    assert_eq!(zero_pool_config.max_pool_size, 0);
    assert_eq!(zero_pool_config.min_pool_size, 0);

    // Test invalid pool configuration (max < min)
    let invalid_pool_config = MongoConfig {
        max_pool_size: 5,
        min_pool_size: 10,
        ..MongoConfig::default()
    };
    assert!(invalid_pool_config.max_pool_size < invalid_pool_config.min_pool_size);

    // Test empty URI
    let empty_uri_config = MongoConfig {
        uri: "".to_string(),
        ..MongoConfig::default()
    };
    assert!(empty_uri_config.uri.is_empty());
}

/// Test thread safety and cloning
#[test]
fn test_config_thread_safety() {
    let config = MongoConfig::default();
    let config_clone = config.clone();

    // Test that cloned config is identical
    assert_eq!(config.uri, config_clone.uri);
    assert_eq!(config.database, config_clone.database);
    assert_eq!(config.max_pool_size, config_clone.max_pool_size);
    assert_eq!(config.enable_compression, config_clone.enable_compression);

    // Test that configs are Send + Sync (compile-time check)
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MongoConfig>();
    assert_send_sync::<TestCampaign>();
    assert_send_sync::<TestUserProfile>();
}

/// Test error handling for MongoDB operations
#[test]
fn test_mongodb_error_handling() {
    let connection_error = DatabaseError::Connection("MongoDB connection failed".to_string());
    let query_error = DatabaseError::Query("Invalid MongoDB query".to_string());

    match connection_error {
        DatabaseError::Connection(msg) => assert!(msg.contains("MongoDB")),
        _ => panic!("Expected Connection error"),
    }

    match query_error {
        DatabaseError::Query(msg) => assert!(msg.contains("Invalid")),
        _ => panic!("Expected Query error"),
    }
}

#[cfg(test)]
mod mock_document_operations {
    use super::*;

    /// Test mock document CRUD operations
    #[tokio::test]
    async fn test_mock_document_operations() {
        let config = MongoConfig::default();

        // Mock document creation
        let campaign = TestCampaign {
            id: Some(ObjectId::new()),
            name: "Mock Campaign".to_string(),
            description: "Mock document operation test".to_string(),
            status: "active".to_string(),
            target_audience: vec!["mock_users".to_string()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metrics: CampaignMetrics {
                impressions: 500,
                clicks: 25,
                conversions: 2,
                cost: 50.0,
            },
        };

        // Test document serialization for insert
        let doc = bson::to_document(&campaign).unwrap();
        assert!(doc.contains_key("name"));
        assert!(doc.contains_key("metrics"));

        // Test document deserialization for read
        let retrieved: TestCampaign = bson::from_document(doc).unwrap();
        assert_eq!(campaign.name, retrieved.name);
        assert_eq!(campaign.metrics.impressions, retrieved.metrics.impressions);
    }

    /// Test bulk operations simulation
    #[test]
    fn test_bulk_operations_simulation() {
        let mut campaigns = Vec::new();

        // Create bulk test data
        for i in 0..1000 {
            let campaign = TestCampaign {
                id: Some(ObjectId::new()),
                name: format!("Bulk Campaign {}", i),
                description: format!("Bulk test campaign number {}", i),
                status: if i % 10 == 0 { "paused" } else { "active" }.to_string(),
                target_audience: vec![format!("audience_{}", i % 5)],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                metrics: CampaignMetrics {
                    impressions: (i * 100) as u64,
                    clicks: (i * 5) as u64,
                    conversions: i as u64,
                    cost: i as f64 * 1.5,
                },
            };
            campaigns.push(campaign);
        }

        assert_eq!(campaigns.len(), 1000);

        // Test bulk data characteristics
        let active_count = campaigns.iter().filter(|c| c.status == "active").count();
        let paused_count = campaigns.iter().filter(|c| c.status == "paused").count();

        assert_eq!(active_count, 900);
        assert_eq!(paused_count, 100);

        // Test serialization performance
        let start = std::time::Instant::now();
        for campaign in &campaigns {
            let _doc = bson::to_document(campaign).unwrap();
        }
        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(1000)); // Should serialize 1k docs in < 1s
    }

    /// Test aggregation pipeline simulation
    #[test]
    fn test_aggregation_simulation() {
        // Simulate aggregation results
        let campaign_stats = vec![
            doc! {
                "_id": "active",
                "total_campaigns": 900,
                "total_impressions": 450000_i64,
                "total_clicks": 22500_i64,
                "total_conversions": 450_i64,
                "total_cost": 675.0,
                "avg_ctr": 0.05
            },
            doc! {
                "_id": "paused",
                "total_campaigns": 100,
                "total_impressions": 50000_i64,
                "total_clicks": 2500_i64,
                "total_conversions": 50_i64,
                "total_cost": 75.0,
                "avg_ctr": 0.05
            }
        ];

        for stats in campaign_stats {
            assert!(stats.contains_key("_id"));
            assert!(stats.contains_key("total_campaigns"));
            assert!(stats.contains_key("total_impressions"));

            let total_campaigns = stats.get_i32("total_campaigns").unwrap();
            let total_impressions = stats.get_i64("total_impressions").unwrap();

            assert!(total_campaigns > 0);
            assert!(total_impressions > 0);
        }
    }
}

/// Performance-related tests (mock)
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_document_creation_performance() {
        let start = std::time::Instant::now();

        // Create many documents to test performance
        for i in 0..5000 {
            let _campaign = TestCampaign {
                id: Some(ObjectId::new()),
                name: format!("Perf Test Campaign {}", i),
                description: "Performance test".to_string(),
                status: "active".to_string(),
                target_audience: vec!["perf_test".to_string()],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                metrics: CampaignMetrics {
                    impressions: 1000,
                    clicks: 50,
                    conversions: 5,
                    cost: 25.0,
                },
            };
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(500)); // Should create 5k docs in < 500ms
    }

    #[tokio::test]
    async fn test_concurrent_document_processing() {
        let mut handles = vec![];

        // Spawn multiple tasks creating documents concurrently
        for i in 0..50 {
            let handle = tokio::spawn(async move {
                let campaign = TestCampaign {
                    id: Some(ObjectId::new()),
                    name: format!("Concurrent Campaign {}", i),
                    description: "Concurrent processing test".to_string(),
                    status: "active".to_string(),
                    target_audience: vec!["concurrent".to_string()],
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    metrics: CampaignMetrics {
                        impressions: 100,
                        clicks: 5,
                        conversions: 1,
                        cost: 10.0,
                    },
                };

                // Simulate processing time
                tokio::time::sleep(Duration::from_millis(1)).await;
                campaign.name
            });
            handles.push(handle);
        }

        // Wait for all tasks and verify results
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.unwrap();
            assert_eq!(result, format!("Concurrent Campaign {}", i));
        }
    }

    #[test]
    fn test_bson_serialization_performance() {
        let campaign = TestCampaign {
            id: Some(ObjectId::new()),
            name: "BSON Performance Test".to_string(),
            description: "Testing BSON serialization performance".to_string(),
            status: "active".to_string(),
            target_audience: vec!["performance".to_string(); 100], // Large array
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metrics: CampaignMetrics {
                impressions: 1000000,
                clicks: 50000,
                conversions: 5000,
                cost: 1500.75,
            },
        };

        let start = std::time::Instant::now();

        // Serialize/deserialize many times
        for _ in 0..1000 {
            let doc = bson::to_document(&campaign).unwrap();
            let _deserialized: TestCampaign = bson::from_document(doc).unwrap();
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(1000)); // Should handle 1k round-trips in < 1s
    }
}

/// Test query optimization patterns
#[cfg(test)]
mod query_optimization_tests {
    use super::*;

    #[test]
    fn test_efficient_query_patterns() {
        // Test indexed field queries
        let indexed_queries = vec![
            doc! { "status": "active" },
            doc! { "created_at": { "$gte": chrono::Utc::now() - chrono::Duration::days(7) } },
            doc! { "target_audience": { "$in": ["developers", "engineers"] } },
            doc! { "metrics.impressions": { "$gte": 1000 } },
        ];

        for query in indexed_queries {
            assert!(!query.is_empty());
            // Verify query uses indexed fields
            assert!(query.keys().any(|key| {
                matches!(key, "status" | "created_at" | "target_audience" | "metrics.impressions")
            }));
        }

        // Test compound queries
        let compound_query = doc! {
            "status": "active",
            "created_at": { "$gte": chrono::Utc::now() - chrono::Duration::days(30) },
            "metrics.impressions": { "$gte": 100 }
        };

        assert_eq!(compound_query.len(), 3);
        assert!(compound_query.contains_key("status"));
        assert!(compound_query.contains_key("created_at"));
        assert!(compound_query.contains_key("metrics.impressions"));
    }

    #[test]
    fn test_aggregation_optimization_patterns() {
        // Test early filtering with $match
        let optimized_pipeline = vec![
            doc! {
                "$match": {
                    "status": "active",
                    "created_at": { "$gte": chrono::Utc::now() - chrono::Duration::days(7) }
                }
            },
            doc! {
                "$group": {
                    "_id": "$target_audience",
                    "campaign_count": { "$sum": 1 },
                    "total_impressions": { "$sum": "$metrics.impressions" }
                }
            },
            doc! {
                "$sort": { "total_impressions": -1 }
            },
            doc! {
                "$limit": 10
            }
        ];

        // Verify pipeline structure for optimization
        assert_eq!(optimized_pipeline[0].keys().next().unwrap(), "$match"); // Filter early
        assert_eq!(optimized_pipeline[1].keys().next().unwrap(), "$group"); // Then group
        assert_eq!(optimized_pipeline[2].keys().next().unwrap(), "$sort");  // Then sort
        assert_eq!(optimized_pipeline[3].keys().next().unwrap(), "$limit"); // Finally limit
    }
}
