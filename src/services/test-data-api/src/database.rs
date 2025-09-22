// AI-CORE Test Data API Database Manager
// Multi-database support with PostgreSQL, MongoDB, Redis, and ClickHouse
// Backend Agent Implementation - T2.2

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clickhouse::{Client as ClickHouseClient, Row};
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, FindOptions},
    Client as MongoClient, Collection, Database as MongoDatabase,
};
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgRow},
    ConnectOptions, FromRow, Row as SqlxRow,
};
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::models::*;
use crate::AppConfig;

// ============================================================================
// Database Manager - Central coordination for all databases
// ============================================================================

pub struct DatabaseManager {
    pub postgres: PostgresManager,
    pub mongodb: MongoManager,
    pub redis: RedisManager,
    pub clickhouse: ClickHouseManager,
    config: AppConfig,
}

impl DatabaseManager {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        info!("Initializing DatabaseManager with multi-database support");

        let postgres = PostgresManager::new(&config.database_url).await?;
        let mongodb = MongoManager::new(&config.mongodb_url).await?;
        let redis = RedisManager::new(&config.redis_url).await?;
        let clickhouse = ClickHouseManager::new(&config.clickhouse_url).await?;

        info!("All database connections established successfully");

        Ok(Self {
            postgres,
            mongodb,
            redis,
            clickhouse,
            config: config.clone(),
        })
    }

    // ========================================================================
    // Test User Management
    // ========================================================================

    pub async fn create_test_user(&self, request: CreateTestUserRequest) -> Result<TestUser> {
        debug!("Creating test user: {}", request.username);

        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let password_hash = self.hash_password(&request.password)?;

        let cleanup_after = request.ttl_hours.map(|hours| {
            now + chrono::Duration::hours(hours as i64)
        });

        let user = TestUser {
            id: user_id,
            username: request.username.clone(),
            email: request.email.clone(),
            password_hash,
            first_name: request.first_name.clone(),
            last_name: request.last_name.clone(),
            role: request.role.clone(),
            permissions: request.permissions.clone(),
            metadata: request.metadata.unwrap_or(serde_json::Value::Null),
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            test_environment: request.test_environment.clone(),
            cleanup_after,
        };

        // Store in PostgreSQL (primary)
        self.postgres.create_test_user(&user).await?;

        // Cache in Redis for quick access
        self.redis.cache_test_user(&user).await?;

        // Store metadata in MongoDB for flexible querying
        self.mongodb.store_user_metadata(&user).await?;

        // Log creation event in ClickHouse for analytics
        self.clickhouse.log_user_creation(&user).await?;

        info!("Test user created successfully: {} ({})", user.username, user.id);
        Ok(user)
    }

    pub async fn get_test_users(&self, environment: &str, limit: i64) -> Result<Vec<TestUser>> {
        debug!("Fetching test users for environment: {}, limit: {}", environment, limit);

        // Try Redis cache first for better performance
        if let Ok(cached_users) = self.redis.get_cached_users(environment, limit).await {
            if !cached_users.is_empty() {
                debug!("Retrieved {} users from Redis cache", cached_users.len());
                return Ok(cached_users);
            }
        }

        // Fallback to PostgreSQL
        let users = self.postgres.get_test_users(environment, limit).await?;

        // Cache the result in Redis for future requests
        for user in &users {
            let _ = self.redis.cache_test_user(user).await;
        }

        info!("Retrieved {} test users from PostgreSQL", users.len());
        Ok(users)
    }

    pub async fn delete_test_user(&self, user_id: Uuid) -> Result<bool> {
        debug!("Deleting test user: {}", user_id);

        // Delete from all databases
        let postgres_deleted = self.postgres.delete_test_user(user_id).await?;
        self.redis.remove_cached_user(user_id).await?;
        self.mongodb.delete_user_metadata(user_id).await?;
        self.clickhouse.log_user_deletion(user_id).await?;

        info!("Test user deleted from all databases: {}", user_id);
        Ok(postgres_deleted)
    }

    // ========================================================================
    // Test Environment Management
    // ========================================================================

    pub async fn create_test_environment(&self, request: CreateEnvironmentRequest) -> Result<TestEnvironment> {
        debug!("Creating test environment: {}", request.name);

        let environment_id = Uuid::new_v4();
        let now = Utc::now();
        let user_id = Uuid::new_v4(); // In real implementation, get from auth context

        let expires_at = request.expires_after_hours.map(|hours| {
            now + chrono::Duration::hours(hours as i64)
        });

        let environment = TestEnvironment {
            id: environment_id,
            name: request.name.clone(),
            description: request.description.clone(),
            environment_type: request.environment_type,
            configuration: request.configuration,
            database_configs: request.database_configs,
            service_configs: request.service_configs,
            status: EnvironmentStatus::Provisioning,
            created_by: user_id,
            created_at: now,
            updated_at: now,
            expires_at,
            auto_cleanup: request.auto_cleanup,
        };

        // Store in PostgreSQL
        self.postgres.create_test_environment(&environment).await?;

        // Store configuration in MongoDB for flexibility
        self.mongodb.store_environment_config(&environment).await?;

        // Cache in Redis
        self.redis.cache_environment(&environment).await?;

        info!("Test environment created: {} ({})", environment.name, environment.id);
        Ok(environment)
    }

    pub async fn get_test_environments(&self) -> Result<Vec<TestEnvironment>> {
        debug!("Fetching all test environments");

        let environments = self.postgres.get_test_environments().await?;

        info!("Retrieved {} test environments", environments.len());
        Ok(environments)
    }

    // ========================================================================
    // Utility Methods
    // ========================================================================

    fn hash_password(&self, password: &str) -> Result<String> {
        use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
        use argon2::password_hash::{rand_core::OsRng, SaltString};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?;

        Ok(password_hash.to_string())
    }

    pub async fn health_check(&self) -> Result<DatabaseHealthStatus> {
        debug!("Performing database health checks");

        let postgresql = self.postgres.health_check().await;
        let mongodb = self.mongodb.health_check().await;
        let redis = self.redis.health_check().await;
        let clickhouse = self.clickhouse.health_check().await;

        Ok(DatabaseHealthStatus {
            postgresql,
            mongodb,
            redis,
            clickhouse,
        })
    }
}

// ============================================================================
// PostgreSQL Manager
// ============================================================================

pub struct PostgresManager {
    pool: PgPool,
}

impl PostgresManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to PostgreSQL: {}", database_url.split('@').last().unwrap_or("hidden"));

        let options = PgConnectOptions::from_str(database_url)?
            .log_statements(tracing::log::LevelFilter::Debug)
            .clone();

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect_with(options)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        info!("PostgreSQL connection established successfully");
        Ok(Self { pool })
    }

    pub async fn create_test_user(&self, user: &TestUser) -> Result<()> {
        let query = r#"
            INSERT INTO test_users (
                id, username, email, password_hash, first_name, last_name,
                role, permissions, metadata, is_active, created_at, updated_at,
                last_login_at, test_environment, cleanup_after
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#;

        sqlx::query(query)
            .bind(user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(&user.first_name)
            .bind(&user.last_name)
            .bind(&user.role)
            .bind(&user.permissions)
            .bind(&user.metadata)
            .bind(user.is_active)
            .bind(user.created_at)
            .bind(user.updated_at)
            .bind(user.last_login_at)
            .bind(&user.test_environment)
            .bind(user.cleanup_after)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_test_users(&self, environment: &str, limit: i64) -> Result<Vec<TestUser>> {
        let query = if environment.is_empty() {
            "SELECT * FROM test_users ORDER BY created_at DESC LIMIT $1"
        } else {
            "SELECT * FROM test_users WHERE test_environment = $2 ORDER BY created_at DESC LIMIT $1"
        };

        let mut query_builder = sqlx::query_as::<_, TestUser>(query).bind(limit);

        if !environment.is_empty() {
            query_builder = query_builder.bind(environment);
        }

        let users = query_builder.fetch_all(&self.pool).await?;
        Ok(users)
    }

    pub async fn delete_test_user(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM test_users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn create_test_environment(&self, env: &TestEnvironment) -> Result<()> {
        let query = r#"
            INSERT INTO test_environments (
                id, name, description, environment_type, configuration,
                database_configs, service_configs, status, created_by,
                created_at, updated_at, expires_at, auto_cleanup
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#;

        sqlx::query(query)
            .bind(env.id)
            .bind(&env.name)
            .bind(&env.description)
            .bind(&env.environment_type)
            .bind(serde_json::to_value(&env.configuration)?)
            .bind(serde_json::to_value(&env.database_configs)?)
            .bind(serde_json::to_value(&env.service_configs)?)
            .bind(&env.status)
            .bind(env.created_by)
            .bind(env.created_at)
            .bind(env.updated_at)
            .bind(env.expires_at)
            .bind(env.auto_cleanup)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_test_environments(&self) -> Result<Vec<TestEnvironment>> {
        let query = "SELECT * FROM test_environments ORDER BY created_at DESC";
        let environments = sqlx::query_as::<_, TestEnvironment>(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(environments)
    }

    pub async fn health_check(&self) -> ConnectionHealth {
        let start = std::time::Instant::now();

        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => {
                let duration = start.elapsed().as_millis() as i64;
                ConnectionHealth {
                    status: ServiceHealthStatus::Healthy,
                    connection_count: self.pool.size() as i32,
                    max_connections: 20, // From pool configuration
                    response_time_ms: duration,
                    last_error: None,
                }
            }
            Err(e) => ConnectionHealth {
                status: ServiceHealthStatus::Unhealthy,
                connection_count: self.pool.size() as i32,
                max_connections: 20,
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: Some(e.to_string()),
            },
        }
    }
}

// ============================================================================
// MongoDB Manager
// ============================================================================

pub struct MongoManager {
    client: MongoClient,
    database: MongoDatabase,
}

impl MongoManager {
    pub async fn new(mongodb_url: &str) -> Result<Self> {
        info!("Connecting to MongoDB");

        let client_options = ClientOptions::parse(mongodb_url).await?;
        let client = MongoClient::with_options(client_options)?;
        let database = client.database("aicore_test");

        // Test connection
        database.run_command(doc! {"ping": 1}, None).await?;

        info!("MongoDB connection established successfully");
        Ok(Self { client, database })
    }

    pub async fn store_user_metadata(&self, user: &TestUser) -> Result<()> {
        let collection: Collection<Document> = self.database.collection("user_metadata");

        let metadata_doc = doc! {
            "_id": user.id.to_string(),
            "username": &user.username,
            "email": &user.email,
            "role": user.role.to_string(),
            "permissions": &user.permissions,
            "metadata": mongodb::bson::to_bson(&user.metadata)?,
            "test_environment": &user.test_environment,
            "created_at": user.created_at,
            "tags": ["test_user", "generated"],
        };

        collection.insert_one(metadata_doc, None).await?;
        Ok(())
    }

    pub async fn delete_user_metadata(&self, user_id: Uuid) -> Result<()> {
        let collection: Collection<Document> = self.database.collection("user_metadata");
        collection.delete_one(doc! {"_id": user_id.to_string()}, None).await?;
        Ok(())
    }

    pub async fn store_environment_config(&self, env: &TestEnvironment) -> Result<()> {
        let collection: Collection<Document> = self.database.collection("environment_configs");

        let config_doc = doc! {
            "_id": env.id.to_string(),
            "name": &env.name,
            "environment_type": env.environment_type.to_string(),
            "configuration": mongodb::bson::to_bson(&env.configuration)?,
            "database_configs": mongodb::bson::to_bson(&env.database_configs)?,
            "service_configs": mongodb::bson::to_bson(&env.service_configs)?,
            "created_at": env.created_at,
            "tags": ["test_environment", "config"],
        };

        collection.insert_one(config_doc, None).await?;
        Ok(())
    }

    pub async fn health_check(&self) -> ConnectionHealth {
        let start = std::time::Instant::now();

        match self.database.run_command(doc! {"ping": 1}, None).await {
            Ok(_) => ConnectionHealth {
                status: ServiceHealthStatus::Healthy,
                connection_count: 1, // MongoDB client manages connections internally
                max_connections: 100, // Default MongoDB limit
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: None,
            },
            Err(e) => ConnectionHealth {
                status: ServiceHealthStatus::Unhealthy,
                connection_count: 0,
                max_connections: 100,
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: Some(e.to_string()),
            },
        }
    }
}

// ============================================================================
// Redis Manager
// ============================================================================

pub struct RedisManager {
    connection_manager: Arc<RwLock<ConnectionManager>>,
}

impl RedisManager {
    pub async fn new(redis_url: &str) -> Result<Self> {
        info!("Connecting to Redis");

        let client = RedisClient::open(redis_url)?;
        let connection_manager = ConnectionManager::new(client).await?;

        info!("Redis connection established successfully");
        Ok(Self {
            connection_manager: Arc::new(RwLock::new(connection_manager)),
        })
    }

    pub async fn cache_test_user(&self, user: &TestUser) -> Result<()> {
        let mut conn = self.connection_manager.write().await;
        let key = format!("user:{}", user.id);
        let serialized = serde_json::to_string(user)?;

        conn.set_ex(&key, serialized, 3600).await?; // Cache for 1 hour

        // Also cache by environment for quick lookups
        let env_key = format!("env:{}:users", user.test_environment);
        conn.sadd(&env_key, user.id.to_string()).await?;

        Ok(())
    }

    pub async fn get_cached_users(&self, environment: &str, limit: i64) -> Result<Vec<TestUser>> {
        let mut conn = self.connection_manager.write().await;
        let env_key = format!("env:{}:users", environment);

        let user_ids: Vec<String> = conn.smembers(&env_key).await?;
        let mut users = Vec::new();

        for user_id in user_ids.into_iter().take(limit as usize) {
            let key = format!("user:{}", user_id);
            if let Ok(serialized) = conn.get::<_, String>(&key).await {
                if let Ok(user) = serde_json::from_str::<TestUser>(&serialized) {
                    users.push(user);
                }
            }
        }

        Ok(users)
    }

    pub async fn remove_cached_user(&self, user_id: Uuid) -> Result<()> {
        let mut conn = self.connection_manager.write().await;
        let key = format!("user:{}", user_id);
        conn.del(&key).await?;
        Ok(())
    }

    pub async fn cache_environment(&self, env: &TestEnvironment) -> Result<()> {
        let mut conn = self.connection_manager.write().await;
        let key = format!("env:{}", env.id);
        let serialized = serde_json::to_string(env)?;

        conn.set_ex(&key, serialized, 7200).await?; // Cache for 2 hours
        Ok(())
    }

    pub async fn health_check(&self) -> ConnectionHealth {
        let start = std::time::Instant::now();
        let mut conn = self.connection_manager.write().await;

        match conn.ping().await {
            Ok(_) => ConnectionHealth {
                status: ServiceHealthStatus::Healthy,
                connection_count: 1,
                max_connections: 10,
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: None,
            },
            Err(e) => ConnectionHealth {
                status: ServiceHealthStatus::Unhealthy,
                connection_count: 0,
                max_connections: 10,
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: Some(e.to_string()),
            },
        }
    }
}

// ============================================================================
// ClickHouse Manager
// ============================================================================

pub struct ClickHouseManager {
    client: ClickHouseClient,
}

impl ClickHouseManager {
    pub async fn new(clickhouse_url: &str) -> Result<Self> {
        info!("Connecting to ClickHouse");

        let client = ClickHouseClient::default()
            .with_url(clickhouse_url)
            .with_database("aicore_test");

        // Test connection
        let result = client.query("SELECT 1").fetch_all::<u8>().await;
        match result {
            Ok(_) => info!("ClickHouse connection established successfully"),
            Err(e) => warn!("ClickHouse connection failed: {} (continuing without analytics)", e),
        }

        Ok(Self { client })
    }

    pub async fn log_user_creation(&self, user: &TestUser) -> Result<()> {
        let query = r#"
            INSERT INTO user_events (
                event_time, event_type, user_id, username,
                test_environment, metadata
            ) VALUES (?, ?, ?, ?, ?, ?)
        "#;

        let metadata = serde_json::json!({
            "role": user.role.to_string(),
            "permissions_count": user.permissions.len(),
            "has_cleanup": user.cleanup_after.is_some(),
        });

        let _ = self.client
            .query(query)
            .bind(Utc::now())
            .bind("user_created")
            .bind(user.id.to_string())
            .bind(&user.username)
            .bind(&user.test_environment)
            .bind(metadata.to_string())
            .execute()
            .await;

        Ok(())
    }

    pub async fn log_user_deletion(&self, user_id: Uuid) -> Result<()> {
        let query = r#"
            INSERT INTO user_events (
                event_time, event_type, user_id, metadata
            ) VALUES (?, ?, ?, ?)
        "#;

        let _ = self.client
            .query(query)
            .bind(Utc::now())
            .bind("user_deleted")
            .bind(user_id.to_string())
            .bind("{}")
            .execute()
            .await;

        Ok(())
    }

    pub async fn health_check(&self) -> ConnectionHealth {
        let start = std::time::Instant::now();

        match self.client.query("SELECT 1").fetch_all::<u8>().await {
            Ok(_) => ConnectionHealth {
                status: ServiceHealthStatus::Healthy,
                connection_count: 1,
                max_connections: 100,
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: None,
            },
            Err(e) => ConnectionHealth {
                status: ServiceHealthStatus::Degraded, // Not critical for core functionality
                connection_count: 0,
                max_connections: 100,
                response_time_ms: start.elapsed().as_millis() as i64,
                last_error: Some(e.to_string()),
            },
        }
    }
}

// ============================================================================
// Helper implementations for custom types
// ============================================================================

impl std::fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvironmentType::Development => write!(f, "development"),
            EnvironmentType::Testing => write!(f, "testing"),
            EnvironmentType::Staging => write!(f, "staging"),
            EnvironmentType::Integration => write!(f, "integration"),
            EnvironmentType::Performance => write!(f, "performance"),
            EnvironmentType::Chaos => write!(f, "chaos"),
            EnvironmentType::Sandbox => write!(f, "sandbox"),
        }
    }
}

impl std::fmt::Display for EnvironmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvironmentStatus::Provisioning => write!(f, "provisioning"),
            EnvironmentStatus::Ready => write!(f, "ready"),
            EnvironmentStatus::InUse => write!(f, "in_use"),
            EnvironmentStatus::Maintenance => write!(f, "maintenance"),
            EnvironmentStatus::Error => write!(f, "error"),
            EnvironmentStatus::Destroying => write!(f, "destroying"),
        }
    }
}
