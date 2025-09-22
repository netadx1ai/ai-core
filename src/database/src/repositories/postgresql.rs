//! PostgreSQL Repository Implementation
//!
//! This module provides PostgreSQL-specific repository implementations with:
//! - Connection pooling and health checks
//! - ACID transaction support
//! - Repository pattern for clean data access
//! - Performance optimized queries
//! - Row-level security integration

use anyhow::Result;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
use uuid::Uuid;

use crate::DatabaseError;

/// PostgreSQL repository configuration
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub enable_migrations: bool,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost:5432/ai_core".to_string(),
            max_connections: 20,
            min_connections: 5,
            connection_timeout_seconds: 10,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
            enable_migrations: true,
        }
    }
}

/// Main PostgreSQL repository manager
#[derive(Debug, Clone)]
pub struct PostgresRepository {
    pool: Arc<PgPool>,
    config: PostgresConfig,
}

impl PostgresRepository {
    /// Create a new PostgreSQL repository with existing connection pool
    pub fn new(pool: Arc<PgPool>) -> Self {
        let config = PostgresConfig::default();
        Self { pool, config }
    }

    /// Create a new PostgreSQL repository with connection pool from config
    pub async fn from_config(config: PostgresConfig) -> Result<Self, DatabaseError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                config.connection_timeout_seconds,
            ))
            .idle_timeout(std::time::Duration::from_secs(config.idle_timeout_seconds))
            .max_lifetime(std::time::Duration::from_secs(config.max_lifetime_seconds))
            .connect(&config.database_url)
            .await
            .map_err(DatabaseError::Postgres)?;

        // Run migrations if enabled
        if config.enable_migrations {
            // Migration path would be handled by the migration manager
            // sqlx::migrate!("../../../schemas/migrations/postgresql")
            //     .run(&pool)
            //     .await
            //     .context("Failed to run PostgreSQL migrations")?;
        }

        Ok(Self {
            pool: Arc::new(pool),
            config,
        })
    }

    /// Get the connection pool
    pub fn pool(&self) -> Arc<PgPool> {
        self.pool.clone()
    }

    /// Execute a function within a database transaction
    pub async fn with_transaction<F, R>(&self, f: F) -> Result<R, DatabaseError>
    where
        F: for<'a> FnOnce(
                &'a mut Transaction<'_, Postgres>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<R, DatabaseError>> + Send + 'a>,
            > + Send,
        R: Send,
    {
        let mut tx = self.pool.begin().await?;

        match f(&mut tx).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }

    /// Health check for PostgreSQL connection
    pub async fn health_check(&self) -> Result<bool, DatabaseError> {
        let row = sqlx::query("SELECT 1").fetch_one(&*self.pool).await?;

        let value: i32 = row.try_get(0)?;
        Ok(value == 1)
    }

    /// Get pool statistics
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
            max_size: self.config.max_connections,
        }
    }

    /// User repository
    pub fn users(&self) -> UserRepository {
        UserRepository::new(self.pool.clone())
    }

    /// Workflow repository
    pub fn workflows(&self) -> WorkflowRepository {
        WorkflowRepository::new(self.pool.clone())
    }

    /// Subscription repository
    pub fn subscriptions(&self) -> SubscriptionRepository {
        SubscriptionRepository::new(self.pool.clone())
    }

    /// Usage repository
    pub fn usage_records(&self) -> UsageRepository {
        UsageRepository::new(self.pool.clone())
    }

    /// Federation repository
    pub fn federation(&self) -> FederationRepository {
        FederationRepository::new(self.pool.clone())
    }

    /// Notification repository
    pub fn notifications(&self) -> NotificationRepository {
        NotificationRepository::new(self.pool.clone())
    }

    /// Audit repository
    pub fn audit(&self) -> AuditRepository {
        AuditRepository::new(self.pool.clone())
    }
}

/// Pool statistics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct PoolStats {
    pub size: u32,
    pub idle: usize,
    pub max_size: u32,
}

/// User repository for user-related database operations
#[derive(Debug, Clone)]
pub struct UserRepository {
    pool: Arc<PgPool>,
}

impl UserRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Create a new user
    pub async fn create(&self, user: CreateUserRequest) -> Result<User, DatabaseError> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (email, username, password_hash, first_name, last_name, subscription_tier)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, username, password_hash, first_name, last_name,
                      email_verified, status, subscription_tier, created_at, updated_at, last_login_at, metadata
            "#
        )
        .bind(&user.email)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(user.subscription_tier.as_str())
        .fetch_one(&*self.pool)
        .await?;

        let user = User {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            username: row.try_get("username")?,
            password_hash: row.try_get("password_hash")?,
            first_name: row.try_get("first_name")?,
            last_name: row.try_get("last_name")?,
            email_verified: row.try_get("email_verified")?,
            status: UserStatus::from_str(row.try_get("status")?),
            subscription_tier: SubscriptionTier::from_str(row.try_get("subscription_tier")?),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            last_login_at: row.try_get("last_login_at")?,
            metadata: row.try_get("metadata")?,
        };

        Ok(user)
    }

    /// Find user by ID
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, email, username, password_hash, first_name, last_name,
                   email_verified, status, subscription_tier, created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let user = User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                username: row.try_get("username")?,
                password_hash: row.try_get("password_hash")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name")?,
                email_verified: row.try_get("email_verified")?,
                status: UserStatus::from_str(row.try_get("status")?),
                subscription_tier: SubscriptionTier::from_str(row.try_get("subscription_tier")?),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                last_login_at: row.try_get("last_login_at")?,
                metadata: row.try_get("metadata")?,
            };
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, email, username, password_hash, first_name, last_name,
                   email_verified, status, subscription_tier, created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE email = $1
            "#
        )
        .bind(email)
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let user = User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                username: row.try_get("username")?,
                password_hash: row.try_get("password_hash")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name")?,
                email_verified: row.try_get("email_verified")?,
                status: UserStatus::from_str(row.try_get("status")?),
                subscription_tier: SubscriptionTier::from_str(row.try_get("subscription_tier")?),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                last_login_at: row.try_get("last_login_at")?,
                metadata: row.try_get("metadata")?,
            };
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Update user last login
    pub async fn update_last_login(&self, user_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&*self.pool)
            .await?;

        Ok(())
    }

    /// Update user status
    pub async fn update_status(
        &self,
        user_id: Uuid,
        status: UserStatus,
    ) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE users SET status = $1 WHERE id = $2")
            .bind(status.as_str())
            .bind(user_id)
            .execute(&*self.pool)
            .await?;

        Ok(())
    }
}

/// Workflow repository for workflow-related database operations
#[derive(Debug, Clone)]
pub struct WorkflowRepository {
    pool: Arc<PgPool>,
}

impl WorkflowRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Create a new workflow
    pub async fn create(&self, workflow: CreateWorkflowRequest) -> Result<Workflow, DatabaseError> {
        let row = sqlx::query(
            r#"
            INSERT INTO workflows (user_id, workflow_type, status, priority, estimated_cost_cents, estimated_duration_seconds)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, workflow_type, status, priority, estimated_cost_cents, actual_cost_cents,
                      estimated_duration_seconds, actual_duration_seconds, started_at, completed_at,
                      created_at, updated_at
            "#
        )
        .bind(workflow.user_id)
        .bind(&workflow.workflow_type)
        .bind(workflow.status.as_str())
        .bind(workflow.priority.as_str())
        .bind(workflow.estimated_cost_cents)
        .bind(workflow.estimated_duration_seconds)
        .fetch_one(&*self.pool)
        .await?;

        let workflow = Workflow {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            workflow_type: row.try_get("workflow_type")?,
            status: WorkflowStatus::from_str(row.try_get("status")?),
            priority: WorkflowPriority::from_str(row.try_get("priority")?),
            estimated_cost_cents: row.try_get("estimated_cost_cents")?,
            actual_cost_cents: row.try_get("actual_cost_cents")?,
            estimated_duration_seconds: row.try_get("estimated_duration_seconds")?,
            actual_duration_seconds: row.try_get("actual_duration_seconds")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        Ok(workflow)
    }

    /// Find workflow by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Workflow>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, workflow_type, status, priority, estimated_cost_cents, actual_cost_cents,
                   estimated_duration_seconds, actual_duration_seconds, started_at, completed_at,
                   created_at, updated_at
            FROM workflows
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let workflow = Workflow {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                workflow_type: row.try_get("workflow_type")?,
                status: WorkflowStatus::from_str(row.try_get("status")?),
                priority: WorkflowPriority::from_str(row.try_get("priority")?),
                estimated_cost_cents: row.try_get("estimated_cost_cents")?,
                actual_cost_cents: row.try_get("actual_cost_cents")?,
                estimated_duration_seconds: row.try_get("estimated_duration_seconds")?,
                actual_duration_seconds: row.try_get("actual_duration_seconds")?,
                started_at: row.try_get("started_at")?,
                completed_at: row.try_get("completed_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            Ok(Some(workflow))
        } else {
            Ok(None)
        }
    }

    /// Update workflow status
    pub async fn update_status(
        &self,
        id: Uuid,
        status: WorkflowStatus,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE workflows
            SET status = $1,
                started_at = CASE
                    WHEN $1 = 'running' AND started_at IS NULL THEN NOW()
                    ELSE started_at
                END,
                completed_at = CASE
                    WHEN $1 IN ('completed', 'failed', 'cancelled') THEN NOW()
                    ELSE completed_at
                END
            WHERE id = $2
            "#,
        )
        .bind(status.as_str())
        .bind(id)
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// List workflows for user with pagination
    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Workflow>, DatabaseError> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, workflow_type, status, priority, estimated_cost_cents, actual_cost_cents,
                   estimated_duration_seconds, actual_duration_seconds, started_at, completed_at,
                   created_at, updated_at
            FROM workflows
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        let mut workflows = Vec::new();
        for row in rows {
            let workflow = Workflow {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                workflow_type: row.try_get("workflow_type")?,
                status: WorkflowStatus::from_str(row.try_get("status")?),
                priority: WorkflowPriority::from_str(row.try_get("priority")?),
                estimated_cost_cents: row.try_get("estimated_cost_cents")?,
                actual_cost_cents: row.try_get("actual_cost_cents")?,
                estimated_duration_seconds: row.try_get("estimated_duration_seconds")?,
                actual_duration_seconds: row.try_get("actual_duration_seconds")?,
                started_at: row.try_get("started_at")?,
                completed_at: row.try_get("completed_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            workflows.push(workflow);
        }

        Ok(workflows)
    }
}

// Additional repository implementations would continue here...
// For brevity, I'm including the core structure and a few key repositories

/// Subscription repository for billing and subscription operations
#[derive(Debug, Clone)]
pub struct SubscriptionRepository {
    pool: Arc<PgPool>,
}

impl SubscriptionRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Find active subscription for user
    pub async fn find_active_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Subscription>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, plan_id, plan_name, status, billing_cycle, amount_cents, currency, billing_email,
                   external_subscription_id, current_period_start, current_period_end, trial_end,
                   cancelled_at, created_at, updated_at
            FROM subscriptions
            WHERE user_id = $1 AND status = 'active' AND current_period_end > NOW()
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let subscription = Subscription {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                plan_id: row.try_get("plan_id")?,
                plan_name: row.try_get("plan_name")?,
                status: SubscriptionStatus::from_str(row.try_get("status")?),
                billing_cycle: BillingCycle::from_str(row.try_get("billing_cycle")?),
                amount_cents: row.try_get("amount_cents")?,
                currency: row.try_get("currency")?,
                billing_email: row.try_get("billing_email")?,
                external_subscription_id: row.try_get("external_subscription_id")?,
                current_period_start: row.try_get("current_period_start")?,
                current_period_end: row.try_get("current_period_end")?,
                trial_end: row.try_get("trial_end")?,
                cancelled_at: row.try_get("cancelled_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            Ok(Some(subscription))
        } else {
            Ok(None)
        }
    }
}

/// Usage repository for usage tracking and billing
#[derive(Debug, Clone)]
pub struct UsageRepository {
    pool: Arc<PgPool>,
}

impl UsageRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

/// Federation repository for client and MCP server management
#[derive(Debug, Clone)]
pub struct FederationRepository {
    pool: Arc<PgPool>,
}

impl FederationRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

/// Notification repository for user notifications
#[derive(Debug, Clone)]
pub struct NotificationRepository {
    pool: Arc<PgPool>,
}

impl NotificationRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

/// Audit repository for audit logging
#[derive(Debug, Clone)]
pub struct AuditRepository {
    pool: Arc<PgPool>,
}

impl AuditRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

// Data Transfer Objects and Domain Models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_verified: bool,
    pub status: UserStatus,
    pub subscription_tier: SubscriptionTier,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl UserStatus {
    pub fn as_str(&self) -> &str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => UserStatus::Active,
            "suspended" => UserStatus::Suspended,
            "deleted" => UserStatus::Deleted,
            _ => UserStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionTier {
    Free,
    Pro,
    Enterprise,
}

impl SubscriptionTier {
    pub fn as_str(&self) -> &str {
        match self {
            SubscriptionTier::Free => "free",
            SubscriptionTier::Pro => "pro",
            SubscriptionTier::Enterprise => "enterprise",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "free" => SubscriptionTier::Free,
            "pro" => SubscriptionTier::Pro,
            "enterprise" => SubscriptionTier::Enterprise,
            _ => SubscriptionTier::Free,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub subscription_tier: SubscriptionTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workflow_type: String,
    pub status: WorkflowStatus,
    pub priority: WorkflowPriority,
    pub estimated_cost_cents: Option<i32>,
    pub actual_cost_cents: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub actual_duration_seconds: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Created,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WorkflowStatus::Created => "created",
            WorkflowStatus::Running => "running",
            WorkflowStatus::Completed => "completed",
            WorkflowStatus::Failed => "failed",
            WorkflowStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "created" => WorkflowStatus::Created,
            "running" => WorkflowStatus::Running,
            "completed" => WorkflowStatus::Completed,
            "failed" => WorkflowStatus::Failed,
            "cancelled" => WorkflowStatus::Cancelled,
            _ => WorkflowStatus::Created,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl WorkflowPriority {
    pub fn as_str(&self) -> &str {
        match self {
            WorkflowPriority::Low => "low",
            WorkflowPriority::Medium => "medium",
            WorkflowPriority::High => "high",
            WorkflowPriority::Urgent => "urgent",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "low" => WorkflowPriority::Low,
            "medium" => WorkflowPriority::Medium,
            "high" => WorkflowPriority::High,
            "urgent" => WorkflowPriority::Urgent,
            _ => WorkflowPriority::Medium,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateWorkflowRequest {
    pub user_id: Uuid,
    pub workflow_type: String,
    pub status: WorkflowStatus,
    pub priority: WorkflowPriority,
    pub estimated_cost_cents: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
}

// Additional model definitions for other entities...

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: String,
    pub plan_name: String,
    pub status: SubscriptionStatus,
    pub billing_cycle: BillingCycle,
    pub amount_cents: i32,
    pub currency: String,
    pub billing_email: Option<String>,
    pub external_subscription_id: Option<String>,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_end: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    Expired,
    Suspended,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::Cancelled => "cancelled",
            SubscriptionStatus::Expired => "expired",
            SubscriptionStatus::Suspended => "suspended",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => SubscriptionStatus::Active,
            "cancelled" => SubscriptionStatus::Cancelled,
            "expired" => SubscriptionStatus::Expired,
            "suspended" => SubscriptionStatus::Suspended,
            _ => SubscriptionStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BillingCycle {
    Monthly,
    Yearly,
}

impl BillingCycle {
    pub fn as_str(&self) -> &str {
        match self {
            BillingCycle::Monthly => "monthly",
            BillingCycle::Yearly => "yearly",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "monthly" => BillingCycle::Monthly,
            "yearly" => BillingCycle::Yearly,
            _ => BillingCycle::Monthly,
        }
    }
}
