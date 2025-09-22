//! MongoDB connection management for AI-CORE platform
//!
//! This module handles the initialization, pooling, and lifecycle management
//! of MongoDB database connections for document storage.

use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, ConnectionString},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{ConnectionHealth, ConnectionStats, DatabaseError, MongoConfig};

/// MongoDB connection manager for document storage
pub struct MongoConnection {
    client: Arc<Client>,
    database: Database,
    config: MongoConfig,
    connection_id: String,
}

impl MongoConnection {
    /// Create new MongoDB connection manager
    pub async fn new(config: MongoConfig) -> Result<Self, DatabaseError> {
        info!("Initializing MongoDB connection to {}", config.url);

        let connection_id = Uuid::new_v4().to_string();
        let connection_string = ConnectionString::parse(&config.url).map_err(|e| {
            DatabaseError::Connection(format!("Invalid MongoDB connection string: {}", e))
        })?;

        let mut client_options = ClientOptions::parse_connection_string(connection_string)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to parse MongoDB options: {}", e))
            })?;

        // Configure connection pooling
        client_options.max_pool_size = Some(config.max_pool_size);
        client_options.min_pool_size = Some(config.min_pool_size);
        client_options.max_idle_time = Some(Duration::from_secs(config.max_idle_time_seconds));
        client_options.connect_timeout = Some(Duration::from_secs(config.connect_timeout_seconds));
        client_options.server_selection_timeout =
            Some(Duration::from_secs(config.server_selection_timeout_seconds));

        // Create client
        let client = Client::with_options(client_options).map_err(|e| {
            DatabaseError::Connection(format!("Failed to create MongoDB client: {}", e))
        })?;

        let database = client.database(&config.database);

        // Test the connection
        let start_time = Instant::now();
        database
            .run_command(doc! { "ping": 1 }, None)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("MongoDB connection test failed: {}", e))
            })?;

        let response_time = start_time.elapsed().as_millis() as u64;
        info!(
            "MongoDB connection established successfully in {}ms",
            response_time
        );

        Ok(Self {
            client: Arc::new(client),
            database,
            config,
            connection_id,
        })
    }

    /// Get MongoDB client
    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    /// Get database instance
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Get collection by name
    pub fn collection<T>(&self, name: &str) -> Collection<T>
    where
        T: Send + Sync,
    {
        self.database.collection::<T>(name)
    }

    /// Get typed collection with document structure
    pub fn typed_collection<T>(&self, name: &str) -> Collection<T>
    where
        T: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        self.database.collection::<T>(name)
    }

    /// Test connection health
    pub async fn health_check(&self) -> Result<ConnectionHealth, DatabaseError> {
        let start_time = Instant::now();

        match self.database.run_command(doc! { "ping": 1 }, None).await {
            Ok(_) => {
                let response_time_ms = start_time.elapsed().as_millis() as u64;
                debug!("MongoDB health check passed in {}ms", response_time_ms);
                Ok(ConnectionHealth {
                    healthy: true,
                    response_time_ms,
                    error_message: None,
                })
            }
            Err(e) => {
                let response_time_ms = start_time.elapsed().as_millis() as u64;
                warn!("MongoDB health check failed: {}", e);
                Ok(ConnectionHealth {
                    healthy: false,
                    response_time_ms,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Get connection statistics
    pub async fn connection_stats(&self) -> Result<MongoStats, DatabaseError> {
        // Get server status for connection information
        let server_status = self
            .database
            .run_command(doc! { "serverStatus": 1 }, None)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("MongoDB server status failed: {}", e))
            })?;

        let empty_doc = doc! {};
        let connections = server_status
            .get_document("connections")
            .unwrap_or(&empty_doc);

        Ok(MongoStats {
            connection_id: self.connection_id.clone(),
            database_name: self.config.database.clone(),
            current_connections: connections.get_i32("current").unwrap_or(0) as u32,
            available_connections: connections.get_i32("available").unwrap_or(0) as u32,
            total_created: connections.get_i32("totalCreated").unwrap_or(0) as u32,
            max_pool_size: self.config.max_pool_size,
            min_pool_size: self.config.min_pool_size,
        })
    }

    /// List all collections in the database
    pub async fn list_collections(&self) -> Result<Vec<String>, DatabaseError> {
        let collections = self
            .database
            .list_collection_names(None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to list collections: {}", e)))?;

        Ok(collections)
    }

    /// Create collection with options
    pub async fn create_collection(
        &self,
        name: &str,
        options: Option<mongodb::options::CreateCollectionOptions>,
    ) -> Result<(), DatabaseError> {
        self.database
            .create_collection(name, options)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to create collection: {}", e))
            })?;

        info!("Created MongoDB collection: {}", name);
        Ok(())
    }

    /// Drop collection
    pub async fn drop_collection(&self, name: &str) -> Result<(), DatabaseError> {
        let collection: Collection<Document> = self.database.collection(name);
        collection
            .drop(None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to drop collection: {}", e)))?;

        info!("Dropped MongoDB collection: {}", name);
        Ok(())
    }

    /// Create index on collection
    pub async fn create_index(
        &self,
        collection_name: &str,
        keys: Document,
        options: Option<mongodb::options::IndexOptions>,
    ) -> Result<String, DatabaseError> {
        let collection: Collection<Document> = self.database.collection(collection_name);
        let index_model = mongodb::IndexModel::builder()
            .keys(keys.clone())
            .options(options)
            .build();

        let result = collection
            .create_index(index_model, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to create index: {}", e)))?;

        info!(
            "Created index on collection {}: {:?}",
            collection_name, keys
        );
        Ok(result.index_name)
    }

    /// Get database statistics
    pub async fn database_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let stats = self
            .database
            .run_command(doc! { "dbStats": 1 }, None)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to get database stats: {}", e))
            })?;

        Ok(DatabaseStats {
            database_name: self.config.database.clone(),
            collections: stats.get_i32("collections").unwrap_or(0) as u32,
            objects: stats.get_i64("objects").unwrap_or(0) as u64,
            data_size: stats.get_i64("dataSize").unwrap_or(0) as u64,
            storage_size: stats.get_i64("storageSize").unwrap_or(0) as u64,
            index_size: stats.get_i64("indexSize").unwrap_or(0) as u64,
            indexes: stats.get_i32("indexes").unwrap_or(0) as u32,
        })
    }

    /// Close connection
    pub async fn close(&self) {
        info!("Closing MongoDB connection: {}", self.connection_id);
        // MongoDB Rust driver automatically handles connection cleanup
    }
}

impl ConnectionStats for MongoConnection {
    fn connection_count(&self) -> u32 {
        // MongoDB doesn't expose current connections directly from client
        // This would need to be tracked separately or estimated
        self.config.max_pool_size
    }

    fn active_connections(&self) -> u32 {
        // Estimated based on configuration
        self.config.min_pool_size
    }

    fn idle_connections(&self) -> u32 {
        // Estimated based on configuration
        self.config.max_pool_size - self.config.min_pool_size
    }
}

/// MongoDB connection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoStats {
    pub connection_id: String,
    pub database_name: String,
    pub current_connections: u32,
    pub available_connections: u32,
    pub total_created: u32,
    pub max_pool_size: u32,
    pub min_pool_size: u32,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub database_name: String,
    pub collections: u32,
    pub objects: u64,
    pub data_size: u64,
    pub storage_size: u64,
    pub index_size: u64,
    pub indexes: u32,
}

/// Document operations helper
pub struct DocumentOps<T> {
    collection: Collection<T>,
}

impl<T> DocumentOps<T>
where
    T: Send + Sync + Unpin + Serialize + for<'de> Deserialize<'de>,
{
    /// Create new document operations helper
    pub fn new(collection: Collection<T>) -> Self {
        Self { collection }
    }

    /// Insert single document
    pub async fn insert_one(&self, document: &T) -> Result<String, DatabaseError> {
        let result = self
            .collection
            .insert_one(document, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to insert document: {}", e)))?;

        Ok(result.inserted_id.to_string())
    }

    /// Insert multiple documents
    pub async fn insert_many(&self, documents: Vec<T>) -> Result<Vec<String>, DatabaseError> {
        let result = self
            .collection
            .insert_many(documents, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to insert documents: {}", e)))?;

        let ids = result
            .inserted_ids
            .into_values()
            .map(|id| id.to_string())
            .collect();

        Ok(ids)
    }

    /// Find documents by filter
    pub async fn find(&self, filter: Document) -> Result<Vec<T>, DatabaseError> {
        let cursor =
            self.collection.find(filter, None).await.map_err(|e| {
                DatabaseError::Connection(format!("Failed to find documents: {}", e))
            })?;

        let documents = cursor.try_collect().await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to collect documents: {}", e))
        })?;

        Ok(documents)
    }

    /// Find single document by filter
    pub async fn find_one(&self, filter: Document) -> Result<Option<T>, DatabaseError> {
        let document =
            self.collection.find_one(filter, None).await.map_err(|e| {
                DatabaseError::Connection(format!("Failed to find document: {}", e))
            })?;

        Ok(document)
    }

    /// Update single document
    pub async fn update_one(
        &self,
        filter: Document,
        update: Document,
    ) -> Result<u64, DatabaseError> {
        let result = self
            .collection
            .update_one(filter, update, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to update document: {}", e)))?;

        Ok(result.modified_count)
    }

    /// Update multiple documents
    pub async fn update_many(
        &self,
        filter: Document,
        update: Document,
    ) -> Result<u64, DatabaseError> {
        let result = self
            .collection
            .update_many(filter, update, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to update documents: {}", e)))?;

        Ok(result.modified_count)
    }

    /// Delete single document
    pub async fn delete_one(&self, filter: Document) -> Result<u64, DatabaseError> {
        let result = self
            .collection
            .delete_one(filter, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to delete document: {}", e)))?;

        Ok(result.deleted_count)
    }

    /// Delete multiple documents
    pub async fn delete_many(&self, filter: Document) -> Result<u64, DatabaseError> {
        let result = self
            .collection
            .delete_many(filter, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to delete documents: {}", e)))?;

        Ok(result.deleted_count)
    }

    /// Count documents
    pub async fn count_documents(&self, filter: Document) -> Result<u64, DatabaseError> {
        let count = self
            .collection
            .count_documents(filter, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to count documents: {}", e)))?;

        Ok(count)
    }
}

/// Aggregation pipeline helper
pub struct AggregationOps<T> {
    collection: Collection<T>,
}

impl<T> AggregationOps<T>
where
    T: Send + Sync,
{
    /// Create new aggregation operations helper
    pub fn new(collection: Collection<T>) -> Self {
        Self { collection }
    }

    /// Execute aggregation pipeline
    pub async fn aggregate(&self, pipeline: Vec<Document>) -> Result<Vec<Document>, DatabaseError> {
        let cursor = self
            .collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to run aggregation: {}", e)))?;

        let results = cursor.try_collect().await.map_err(|e| {
            DatabaseError::Connection(format!("Failed to collect aggregation results: {}", e))
        })?;

        Ok(results)
    }

    /// Count documents with grouping
    pub async fn count_by_field(&self, field: &str) -> Result<Vec<Document>, DatabaseError> {
        let pipeline = vec![
            doc! {
                "$group": {
                    "_id": format!("${}", field),
                    "count": { "$sum": 1 }
                }
            },
            doc! { "$sort": { "count": -1 } },
        ];

        self.aggregate(pipeline).await
    }

    /// Get distinct values for a field
    pub async fn distinct(
        &self,
        field_name: &str,
        filter: Option<Document>,
    ) -> Result<Vec<mongodb::bson::Bson>, DatabaseError> {
        let distinct_values = self
            .collection
            .distinct(field_name, filter, None)
            .await
            .map_err(|e| {
                DatabaseError::Connection(format!("Failed to get distinct values: {}", e))
            })?;

        Ok(distinct_values)
    }
}

/// Content management collections and operations
pub mod content {
    use super::*;

    /// Campaign document structure
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Campaign {
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        pub id: Option<mongodb::bson::oid::ObjectId>,
        pub campaign_id: String,
        pub name: String,
        pub description: Option<String>,
        pub status: CampaignStatus,
        pub target_audience: TargetAudience,
        pub content: CampaignContent,
        pub metrics: CampaignMetrics,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub created_by: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum CampaignStatus {
        Draft,
        Active,
        Paused,
        Completed,
        Archived,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TargetAudience {
        pub demographics: Document,
        pub interests: Vec<String>,
        pub behaviors: Vec<String>,
        pub custom_segments: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CampaignContent {
        pub content_type: String,
        pub title: String,
        pub body: String,
        pub media_urls: Vec<String>,
        pub call_to_action: Option<String>,
        pub metadata: Document,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CampaignMetrics {
        pub impressions: u64,
        pub clicks: u64,
        pub conversions: u64,
        pub cost_usd: f64,
        pub revenue_usd: f64,
        pub last_updated: DateTime<Utc>,
    }

    /// Content template document structure
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ContentTemplate {
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        pub id: Option<mongodb::bson::oid::ObjectId>,
        pub template_id: String,
        pub name: String,
        pub category: String,
        pub content_type: String,
        pub template_data: Document,
        pub variables: Vec<String>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub version: u32,
        pub is_active: bool,
    }

    /// User profile document structure
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserProfile {
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        pub id: Option<mongodb::bson::oid::ObjectId>,
        pub user_id: String,
        pub profile_data: Document,
        pub preferences: Document,
        pub behavior_history: Vec<Document>,
        pub segments: Vec<String>,
        pub last_activity: DateTime<Utc>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mongo_config_default() {
        let config = MongoConfig::default();
        assert_eq!(config.database, "ai_core_content");
        assert_eq!(config.max_pool_size, 20);
        assert_eq!(config.min_pool_size, 5);
    }

    #[tokio::test]
    async fn test_document_ops_creation() {
        let config = MongoConfig::default();
        // This test would require a running MongoDB instance
        // In a real test environment, you would use testcontainers
        // let connection = MongoConnection::new(config).await.unwrap();
        // let collection = connection.typed_collection::<content::Campaign>("campaigns");
        // let ops = DocumentOps::new(collection);
        // assert_eq!(ops.collection.name(), "campaigns");
    }

    #[test]
    fn test_campaign_structure() {
        use content::*;

        let campaign = Campaign {
            id: None,
            campaign_id: "test-123".to_string(),
            name: "Test Campaign".to_string(),
            description: Some("A test campaign".to_string()),
            status: CampaignStatus::Draft,
            target_audience: TargetAudience {
                demographics: doc! { "age_range": "25-35" },
                interests: vec!["technology".to_string()],
                behaviors: vec!["online_shopper".to_string()],
                custom_segments: vec!["premium_users".to_string()],
            },
            content: CampaignContent {
                content_type: "social_media".to_string(),
                title: "Test Content".to_string(),
                body: "This is test content".to_string(),
                media_urls: vec![],
                call_to_action: Some("Learn More".to_string()),
                metadata: doc! { "platform": "facebook" },
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
            created_by: "test_user".to_string(),
        };

        assert_eq!(campaign.name, "Test Campaign");
        assert_eq!(campaign.campaign_id, "test-123");
    }
}
