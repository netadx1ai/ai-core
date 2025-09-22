//! MongoDB document storage example for AI-CORE platform
//!
//! This example demonstrates how to use MongoDB for:
//! - Document storage and retrieval
//! - Campaign management
//! - Content templates
//! - User profiles
//! - Aggregation pipelines
//! - Indexing strategies

use ai_core_database::{
    connections::mongodb::content::{
        Campaign, CampaignContent, CampaignMetrics, CampaignStatus, ContentTemplate,
        TargetAudience, UserProfile,
    },
    connections::{AggregationOps, DocumentOps, MongoConfig, MongoConnection},
    DatabaseConfig, MonitoringConfig, PostgresConfig,
};
use chrono::Utc;
use mongodb::bson::{doc, oid::ObjectId, Document};
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🚀 AI-CORE MongoDB Document Storage Example");

    // 1. Create MongoDB configuration
    let mongo_config = MongoConfig {
        url: std::env::var("MONGODB_URL")
            .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
        database: "ai_core_content".to_string(),
        max_pool_size: 20,
        min_pool_size: 5,
        max_idle_time_seconds: 600,
        connect_timeout_seconds: 10,
        server_selection_timeout_seconds: 30,
    };

    println!("📊 Configuration loaded:");
    println!("  - Database: {}", mongo_config.database);
    println!("  - Max pool size: {}", mongo_config.max_pool_size);
    println!(
        "  - Connection URL: {}",
        mask_mongodb_credentials(&mongo_config.url)
    );

    // 2. Initialize MongoDB connection
    println!("\n🔌 Connecting to MongoDB...");

    match MongoConnection::new(mongo_config).await {
        Ok(connection) => {
            println!("✅ MongoDB connection established successfully!");

            // 3. Perform health check
            println!("\n🏥 Performing health check...");
            match connection.health_check().await {
                Ok(health) => {
                    if health.healthy {
                        println!("✅ Health check passed!");
                        println!("  - Response time: {}ms", health.response_time_ms);
                    } else {
                        println!("❌ Health check failed: {:?}", health.error_message);
                        return Ok(());
                    }
                }
                Err(e) => {
                    println!("❌ Health check error: {}", e);
                    return Ok(());
                }
            }

            // 4. Get connection statistics
            println!("\n📊 Connection statistics:");
            match connection.connection_stats().await {
                Ok(stats) => {
                    println!("  - Current connections: {}", stats.current_connections);
                    println!("  - Available connections: {}", stats.available_connections);
                    println!("  - Total created: {}", stats.total_created);
                    println!("  - Max pool size: {}", stats.max_pool_size);
                }
                Err(e) => {
                    println!("  - Could not get stats: {}", e);
                }
            }

            // 5. List existing collections
            println!("\n📚 Existing collections:");
            match connection.list_collections().await {
                Ok(collections) => {
                    if collections.is_empty() {
                        println!("  - No collections found, will create them");
                    } else {
                        for collection in &collections {
                            println!("  - {}", collection);
                        }
                    }
                }
                Err(e) => {
                    println!("  - Could not list collections: {}", e);
                }
            }

            // 6. Create collections and indexes
            println!("\n🔧 Setting up collections and indexes...");
            if let Err(e) = setup_collections_and_indexes(&connection).await {
                println!("❌ Failed to setup collections: {}", e);
            } else {
                println!("✅ Collections and indexes created successfully");
            }

            // 7. Campaign management operations
            println!("\n📋 Campaign Management Operations:");
            if let Err(e) = demo_campaign_operations(&connection).await {
                println!("❌ Campaign operations failed: {}", e);
            } else {
                println!("✅ Campaign operations completed successfully");
            }

            // 8. Content template operations
            println!("\n📝 Content Template Operations:");
            if let Err(e) = demo_template_operations(&connection).await {
                println!("❌ Template operations failed: {}", e);
            } else {
                println!("✅ Template operations completed successfully");
            }

            // 9. User profile operations
            println!("\n👤 User Profile Operations:");
            if let Err(e) = demo_user_profile_operations(&connection).await {
                println!("❌ User profile operations failed: {}", e);
            } else {
                println!("✅ User profile operations completed successfully");
            }

            // 10. Aggregation pipeline examples
            println!("\n🔍 Aggregation Pipeline Examples:");
            if let Err(e) = demo_aggregation_operations(&connection).await {
                println!("❌ Aggregation operations failed: {}", e);
            } else {
                println!("✅ Aggregation operations completed successfully");
            }

            // 11. Get database statistics
            println!("\n📈 Database Statistics:");
            match connection.database_stats().await {
                Ok(stats) => {
                    println!("  - Collections: {}", stats.collections);
                    println!("  - Objects: {}", stats.objects);
                    println!("  - Data size: {} bytes", stats.data_size);
                    println!("  - Storage size: {} bytes", stats.storage_size);
                    println!("  - Index size: {} bytes", stats.index_size);
                    println!("  - Indexes: {}", stats.indexes);
                }
                Err(e) => {
                    println!("  - Could not get database stats: {}", e);
                }
            }

            // 12. Cleanup (optional)
            println!("\n🧹 Cleanup operations...");
            if std::env::var("CLEANUP_AFTER_DEMO").is_ok() {
                if let Err(e) = cleanup_demo_data(&connection).await {
                    println!("❌ Cleanup failed: {}", e);
                } else {
                    println!("✅ Demo data cleaned up");
                }
            } else {
                println!("  - Skipping cleanup (set CLEANUP_AFTER_DEMO=1 to enable)");
            }

            // 13. Close connection
            println!("\n🔄 Closing MongoDB connection...");
            connection.close().await;
            println!("✅ Connection closed cleanly");

            println!("\n🎉 MongoDB example completed successfully!");
        }
        Err(e) => {
            println!("❌ Failed to connect to MongoDB: {}", e);
            println!("\n💡 Tips:");
            println!("  - Make sure MongoDB is running on localhost:27017");
            println!("  - Or set MONGODB_URL environment variable for different connection");
            println!("  - Example: export MONGODB_URL='mongodb://user:pass@localhost:27017/mydb'");

            return Err(e.into());
        }
    }

    Ok(())
}

/// Setup collections and indexes for the demo
async fn setup_collections_and_indexes(connection: &MongoConnection) -> anyhow::Result<()> {
    // Create campaigns collection with indexes
    let campaigns_collection: mongodb::Collection<Campaign> =
        connection.typed_collection("campaigns");

    // Create indexes for campaigns
    connection
        .create_index(
            "campaigns",
            doc! { "campaign_id": 1 },
            Some(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .build(),
            ),
        )
        .await?;

    connection
        .create_index("campaigns", doc! { "status": 1, "created_at": -1 }, None)
        .await?;

    connection
        .create_index("campaigns", doc! { "target_audience.interests": 1 }, None)
        .await?;

    // Create content templates collection with indexes
    connection
        .create_index(
            "content_templates",
            doc! { "template_id": 1 },
            Some(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .build(),
            ),
        )
        .await?;

    connection
        .create_index(
            "content_templates",
            doc! { "category": 1, "is_active": 1 },
            None,
        )
        .await?;

    // Create user profiles collection with indexes
    connection
        .create_index(
            "user_profiles",
            doc! { "user_id": 1 },
            Some(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .build(),
            ),
        )
        .await?;

    connection
        .create_index(
            "user_profiles",
            doc! { "segments": 1, "last_activity": -1 },
            None,
        )
        .await?;

    println!("  - Created collections and indexes");
    Ok(())
}

/// Demonstrate campaign management operations
async fn demo_campaign_operations(connection: &MongoConnection) -> anyhow::Result<()> {
    let campaigns_collection = connection.typed_collection::<Campaign>("campaigns");
    let campaign_ops = DocumentOps::new(campaigns_collection);

    // Create sample campaigns
    let campaigns = vec![
        Campaign {
            id: None,
            campaign_id: format!("camp_{}", Uuid::new_v4().to_string()[..8].to_string()),
            name: "Summer Sale 2024".to_string(),
            description: Some("Promote summer products with special discounts".to_string()),
            status: CampaignStatus::Active,
            target_audience: TargetAudience {
                demographics: doc! {
                    "age_range": "25-45",
                    "gender": "all",
                    "income": "middle_high"
                },
                interests: vec!["shopping".to_string(), "fashion".to_string(), "discounts".to_string()],
                behaviors: vec!["frequent_shopper".to_string(), "price_conscious".to_string()],
                custom_segments: vec!["summer_buyers".to_string()],
            },
            content: CampaignContent {
                content_type: "promotional".to_string(),
                title: "Summer Sale - Up to 50% Off!".to_string(),
                body: "Don't miss out on our biggest summer sale with discounts up to 50% on selected items.".to_string(),
                media_urls: vec!["https://example.com/summer-sale-banner.jpg".to_string()],
                call_to_action: Some("Shop Now".to_string()),
                metadata: doc! {
                    "platform": "multi",
                    "budget": 5000.0,
                    "duration_days": 30
                },
            },
            metrics: CampaignMetrics {
                impressions: 15420,
                clicks: 892,
                conversions: 127,
                cost_usd: 1234.56,
                revenue_usd: 3456.78,
                last_updated: Utc::now(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: "marketing_team".to_string(),
        },
        Campaign {
            id: None,
            campaign_id: format!("camp_{}", Uuid::new_v4().to_string()[..8].to_string()),
            name: "Tech Product Launch".to_string(),
            description: Some("Launch campaign for new tech product".to_string()),
            status: CampaignStatus::Draft,
            target_audience: TargetAudience {
                demographics: doc! {
                    "age_range": "18-35",
                    "gender": "all",
                    "tech_savvy": true
                },
                interests: vec!["technology".to_string(), "gadgets".to_string(), "innovation".to_string()],
                behaviors: vec!["early_adopter".to_string(), "tech_enthusiast".to_string()],
                custom_segments: vec!["tech_audience".to_string()],
            },
            content: CampaignContent {
                content_type: "product_launch".to_string(),
                title: "Revolutionary New Tech Product".to_string(),
                body: "Experience the future with our groundbreaking new technology.".to_string(),
                media_urls: vec!["https://example.com/tech-product-demo.mp4".to_string()],
                call_to_action: Some("Learn More".to_string()),
                metadata: doc! {
                    "platform": "digital",
                    "budget": 10000.0,
                    "launch_date": "2024-06-01"
                },
            },
            metrics: CampaignMetrics {
                impressions: 0,
                clicks: 0,
                conversions: 0,
                cost_usd: 0.0,
                revenue_usd: 0.0,
                last_updated: Utc::now(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: "product_team".to_string(),
        },
    ];

    // Insert campaigns
    let campaign_ids = campaign_ops.insert_many(campaigns).await?;
    println!("  - Inserted {} campaigns", campaign_ids.len());

    // Find active campaigns
    let active_campaigns = campaign_ops.find(doc! { "status": "Active" }).await?;
    println!("  - Found {} active campaigns", active_campaigns.len());

    // Update campaign metrics
    let updated_count = campaign_ops
        .update_many(
            doc! { "status": "Active" },
            doc! { "$inc": { "metrics.impressions": 1000 } },
        )
        .await?;
    println!("  - Updated metrics for {} campaigns", updated_count);

    // Count campaigns by status
    let draft_count = campaign_ops
        .count_documents(doc! { "status": "Draft" })
        .await?;
    let active_count = campaign_ops
        .count_documents(doc! { "status": "Active" })
        .await?;
    println!(
        "  - Campaign counts: {} draft, {} active",
        draft_count, active_count
    );

    Ok(())
}

/// Demonstrate content template operations
async fn demo_template_operations(connection: &MongoConnection) -> anyhow::Result<()> {
    let templates_collection = connection.typed_collection::<ContentTemplate>("content_templates");
    let template_ops = DocumentOps::new(templates_collection);

    // Create sample templates
    let templates = vec![
        ContentTemplate {
            id: None,
            template_id: format!("tmpl_{}", Uuid::new_v4().to_string()[..8].to_string()),
            name: "Email Newsletter Template".to_string(),
            category: "email".to_string(),
            content_type: "newsletter".to_string(),
            template_data: doc! {
                "subject": "{{subject}}",
                "header": "{{header}}",
                "body": "{{body}}",
                "footer": "{{footer}}",
                "unsubscribe_link": "{{unsubscribe_url}}"
            },
            variables: vec![
                "subject".to_string(),
                "header".to_string(),
                "body".to_string(),
                "footer".to_string(),
                "unsubscribe_url".to_string(),
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            is_active: true,
        },
        ContentTemplate {
            id: None,
            template_id: format!("tmpl_{}", Uuid::new_v4().to_string()[..8].to_string()),
            name: "Social Media Post Template".to_string(),
            category: "social".to_string(),
            content_type: "post".to_string(),
            template_data: doc! {
                "text": "{{message}}",
                "hashtags": "{{hashtags}}",
                "media": "{{media_urls}}",
                "platform": "{{platform}}"
            },
            variables: vec![
                "message".to_string(),
                "hashtags".to_string(),
                "media_urls".to_string(),
                "platform".to_string(),
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            is_active: true,
        },
    ];

    // Insert templates
    let template_ids = template_ops.insert_many(templates).await?;
    println!("  - Inserted {} content templates", template_ids.len());

    // Find templates by category
    let email_templates = template_ops
        .find(doc! {
            "category": "email",
            "is_active": true
        })
        .await?;
    println!("  - Found {} email templates", email_templates.len());

    // Update template version
    let updated_count = template_ops
        .update_one(
            doc! { "category": "email" },
            doc! { "$inc": { "version": 1 }, "$set": { "updated_at": Utc::now() } },
        )
        .await?;
    println!("  - Updated version for {} template", updated_count);

    Ok(())
}

/// Demonstrate user profile operations
async fn demo_user_profile_operations(connection: &MongoConnection) -> anyhow::Result<()> {
    let profiles_collection = connection.typed_collection::<UserProfile>("user_profiles");
    let profile_ops = DocumentOps::new(profiles_collection);

    // Create sample user profiles
    let profiles = vec![
        UserProfile {
            id: None,
            user_id: format!("user_{}", Uuid::new_v4().to_string()[..8].to_string()),
            profile_data: doc! {
                "name": "John Doe",
                "email": "john.doe@example.com",
                "age": 32,
                "location": "New York",
                "preferences": {
                    "communication": "email",
                    "frequency": "weekly",
                    "topics": ["technology", "business"]
                }
            },
            preferences: doc! {
                "email_notifications": true,
                "push_notifications": false,
                "newsletter": true,
                "promotional": true
            },
            behavior_history: vec![
                doc! {
                    "action": "click",
                    "campaign_id": "camp_12345",
                    "timestamp": Utc::now(),
                    "metadata": { "device": "mobile", "platform": "ios" }
                },
                doc! {
                    "action": "conversion",
                    "campaign_id": "camp_12345",
                    "timestamp": Utc::now(),
                    "value": 99.99
                },
            ],
            segments: vec!["tech_enthusiast".to_string(), "premium_user".to_string()],
            last_activity: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        UserProfile {
            id: None,
            user_id: format!("user_{}", Uuid::new_v4().to_string()[..8].to_string()),
            profile_data: doc! {
                "name": "Jane Smith",
                "email": "jane.smith@example.com",
                "age": 28,
                "location": "California",
                "preferences": {
                    "communication": "push",
                    "frequency": "daily",
                    "topics": ["fashion", "lifestyle"]
                }
            },
            preferences: doc! {
                "email_notifications": false,
                "push_notifications": true,
                "newsletter": false,
                "promotional": true
            },
            behavior_history: vec![doc! {
                "action": "view",
                "campaign_id": "camp_67890",
                "timestamp": Utc::now(),
                "metadata": { "device": "desktop", "duration": 45 }
            }],
            segments: vec!["fashion_lover".to_string(), "mobile_user".to_string()],
            last_activity: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ];

    // Insert profiles
    let profile_ids = profile_ops.insert_many(profiles).await?;
    println!("  - Inserted {} user profiles", profile_ids.len());

    // Find users by segment
    let tech_users = profile_ops
        .find(doc! {
            "segments": { "$in": ["tech_enthusiast"] }
        })
        .await?;
    println!("  - Found {} tech enthusiast users", tech_users.len());

    // Add behavior to user profile
    let updated_count = profile_ops
        .update_one(
            doc! { "segments": { "$in": ["tech_enthusiast"] } },
            doc! {
                "$push": {
                    "behavior_history": doc! {
                        "action": "email_open",
                        "campaign_id": "camp_newsletter",
                        "timestamp": Utc::now()
                    }
                },
                "$set": { "last_activity": Utc::now() }
            },
        )
        .await?;
    println!("  - Added behavior to {} user profile", updated_count);

    Ok(())
}

/// Demonstrate aggregation pipeline operations
async fn demo_aggregation_operations(connection: &MongoConnection) -> anyhow::Result<()> {
    // Campaign analytics aggregation
    let campaigns_collection = connection.collection::<Campaign>("campaigns");
    let campaign_agg_ops = AggregationOps::new(campaigns_collection);

    // Aggregate campaign performance by status
    let status_aggregation = campaign_agg_ops
        .aggregate(vec![
            doc! {
                "$group": {
                    "_id": "$status",
                    "total_campaigns": { "$sum": 1 },
                    "total_impressions": { "$sum": "$metrics.impressions" },
                    "total_clicks": { "$sum": "$metrics.clicks" },
                    "total_conversions": { "$sum": "$metrics.conversions" },
                    "total_cost": { "$sum": "$metrics.cost_usd" },
                    "total_revenue": { "$sum": "$metrics.revenue_usd" },
                    "avg_ctr": {
                        "$avg": {
                            "$cond": [
                                { "$gt": ["$metrics.impressions", 0] },
                                { "$divide": ["$metrics.clicks", "$metrics.impressions"] },
                                0
                            ]
                        }
                    }
                }
            },
            doc! { "$sort": { "total_revenue": -1 } },
        ])
        .await?;

    println!("  - Campaign performance by status:");
    for result in status_aggregation {
        if let Ok(status) = result.get_str("_id") {
            let campaigns = result.get_i32("total_campaigns").unwrap_or(0);
            let impressions = result.get_i64("total_impressions").unwrap_or(0);
            let clicks = result.get_i64("total_clicks").unwrap_or(0);
            let revenue = result.get_f64("total_revenue").unwrap_or(0.0);
            println!(
                "    {}: {} campaigns, {} impressions, {} clicks, ${:.2} revenue",
                status, campaigns, impressions, clicks, revenue
            );
        }
    }

    // User segment analysis
    let profiles_collection = connection.collection::<UserProfile>("user_profiles");
    let profile_agg_ops = AggregationOps::new(profiles_collection);

    let segment_counts = profile_agg_ops.count_by_field("segments").await?;
    println!("  - User segments:");
    for result in segment_counts {
        if let (Ok(segment), Ok(count)) = (result.get_str("_id"), result.get_i32("count")) {
            println!("    {}: {} users", segment, count);
        }
    }

    // Content template usage analysis
    let templates_collection = connection.collection::<ContentTemplate>("content_templates");
    let template_agg_ops = AggregationOps::new(templates_collection);

    let category_analysis = template_agg_ops
        .aggregate(vec![
            doc! { "$match": { "is_active": true } },
            doc! {
                "$group": {
                    "_id": "$category",
                    "template_count": { "$sum": 1 },
                    "avg_version": { "$avg": "$version" },
                    "latest_update": { "$max": "$updated_at" }
                }
            },
            doc! { "$sort": { "template_count": -1 } },
        ])
        .await?;

    println!("  - Template categories:");
    for result in category_analysis {
        if let Ok(category) = result.get_str("_id") {
            let count = result.get_i32("template_count").unwrap_or(0);
            let avg_version = result.get_f64("avg_version").unwrap_or(0.0);
            println!(
                "    {}: {} templates, avg version {:.1}",
                category, count, avg_version
            );
        }
    }

    Ok(())
}

/// Cleanup demo data
async fn cleanup_demo_data(connection: &MongoConnection) -> anyhow::Result<()> {
    let campaigns_collection = connection.collection::<Document>("campaigns");
    let templates_collection = connection.collection::<Document>("content_templates");
    let profiles_collection = connection.collection::<Document>("user_profiles");

    // Delete all demo data
    let campaigns_deleted = campaigns_collection.delete_many(doc! {}, None).await?;
    let templates_deleted = templates_collection.delete_many(doc! {}, None).await?;
    let profiles_deleted = profiles_collection.delete_many(doc! {}, None).await?;

    println!("  - Deleted {} campaigns", campaigns_deleted.deleted_count);
    println!("  - Deleted {} templates", templates_deleted.deleted_count);
    println!(
        "  - Deleted {} user profiles",
        profiles_deleted.deleted_count
    );

    Ok(())
}

/// Mask MongoDB credentials for safe logging
fn mask_mongodb_credentials(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut masked = parsed.clone();
        if masked.password().is_some() {
            let _ = masked.set_password(Some("***"));
        }
        masked.to_string()
    } else {
        // If URL parsing fails, just mask everything after mongodb://
        if let Some(pos) = url.find("mongodb://") {
            format!("{}***", &url[..pos + 10])
        } else {
            "***".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_mongodb_credentials() {
        let url = "mongodb://user:password@localhost:27017/db";
        let masked = mask_mongodb_credentials(url);
        assert!(masked.contains("***"));
        assert!(!masked.contains("password"));

        let no_password = "mongodb://localhost:27017/db";
        let masked_no_pass = mask_mongodb_credentials(no_password);
        assert_eq!(masked_no_pass, no_password);
    }
}
