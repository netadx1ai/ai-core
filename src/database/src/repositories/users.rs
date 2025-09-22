//! User repository implementation for PostgreSQL
//!
//! This module handles all user-related database operations including:
//! - User CRUD operations
//! - Authentication and session management
//! - API key management
//! - User preferences and settings
//! - Audit logging for user actions

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bcrypt::{hash, verify, DEFAULT_COST};

use super::{Repository, QueryFilter, Pagination, Sort, SortDirection, PagedResult, PaginationMeta, RepositoryError};
use crate::database::DatabaseError;

/// User repository for PostgreSQL operations
#[derive(Clone)]
pub struct UserRepository {
    pool: Arc<PgPool>,
}

impl UserRepository {
    /// Create new user repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Authenticate user with email and password
    pub async fn authenticate(&self, email: &str, password: &str) -> Result<Option<User>, DatabaseError> {
        let user = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE email = $1 AND status = 'active'
            "#,
            email
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(user_record) = user {
            if verify(password, &user_record.password_hash).unwrap_or(false) {
                // Update last login
                sqlx::query!(
                    "UPDATE users SET last_login_at = NOW() WHERE id = $1",
                    user_record.id
                )
                .execute(&*self.pool)
                .await?;

                Ok(Some(User::from_record(user_record)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DatabaseError> {
        let user = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(user.map(User::from_record))
    }

    /// Find user by username
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, DatabaseError> {
        let user = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(user.map(User::from_record))
    }

    /// Update user password
    pub async fn update_password(&self, user_id: Uuid, new_password: &str) -> Result<(), DatabaseError> {
        let password_hash = hash(new_password, DEFAULT_COST)
            .map_err(|e| DatabaseError::Validation(format!("Password hashing failed: {}", e)))?;

        sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2",
            password_hash,
            user_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Verify user email
    pub async fn verify_email(&self, user_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE users SET email_verified = true, updated_at = NOW() WHERE id = $1",
            user_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Update user status
    pub async fn update_status(&self, user_id: Uuid, status: UserStatus) -> Result<(), DatabaseError> {
        let status_str = match status {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        };

        sqlx::query!(
            "UPDATE users SET status = $1, updated_at = NOW() WHERE id = $2",
            status_str,
            user_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Update subscription tier
    pub async fn update_subscription_tier(&self, user_id: Uuid, tier: SubscriptionTier) -> Result<(), DatabaseError> {
        let tier_str = match tier {
            SubscriptionTier::Free => "free",
            SubscriptionTier::Pro => "pro",
            SubscriptionTier::Enterprise => "enterprise",
        };

        sqlx::query!(
            "UPDATE users SET subscription_tier = $1, updated_at = NOW() WHERE id = $2",
            tier_str,
            user_id
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Get users by subscription tier
    pub async fn find_by_subscription_tier(&self, tier: SubscriptionTier) -> Result<Vec<User>, DatabaseError> {
        let tier_str = match tier {
            SubscriptionTier::Free => "free",
            SubscriptionTier::Pro => "pro",
            SubscriptionTier::Enterprise => "enterprise",
        };

        let users = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE subscription_tier = $1 AND status = 'active'
            ORDER BY created_at DESC
            "#,
            tier_str
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(users.into_iter().map(User::from_record).collect())
    }

    /// Get user statistics
    pub async fn get_stats(&self) -> Result<UserStats, DatabaseError> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_users,
                COUNT(CASE WHEN status = 'active' THEN 1 END) as active_users,
                COUNT(CASE WHEN status = 'suspended' THEN 1 END) as suspended_users,
                COUNT(CASE WHEN email_verified = true THEN 1 END) as verified_users,
                COUNT(CASE WHEN subscription_tier = 'pro' THEN 1 END) as pro_users,
                COUNT(CASE WHEN subscription_tier = 'enterprise' THEN 1 END) as enterprise_users,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '30 days' THEN 1 END) as recent_signups
            FROM users
            "#
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(UserStats {
            total_users: stats.total_users.unwrap_or(0) as u64,
            active_users: stats.active_users.unwrap_or(0) as u64,
            suspended_users: stats.suspended_users.unwrap_or(0) as u64,
            verified_users: stats.verified_users.unwrap_or(0) as u64,
            pro_users: stats.pro_users.unwrap_or(0) as u64,
            enterprise_users: stats.enterprise_users.unwrap_or(0) as u64,
            recent_signups: stats.recent_signups.unwrap_or(0) as u64,
        })
    }
}

#[async_trait]
impl Repository<User> for UserRepository {
    type Id = Uuid;
    type CreateInput = CreateUserInput;
    type UpdateInput = UpdateUserInput;
    type QueryFilter = UserFilter;

    async fn create(&self, input: Self::CreateInput) -> Result<User, DatabaseError> {
        // Hash password
        let password_hash = hash(&input.password, DEFAULT_COST)
            .map_err(|e| DatabaseError::Validation(format!("Password hashing failed: {}", e)))?;

        // Check for existing email/username
        if let Some(_) = self.find_by_email(&input.email).await? {
            return Err(DatabaseError::Validation("Email already exists".to_string()));
        }

        if let Some(_) = self.find_by_username(&input.username).await? {
            return Err(DatabaseError::Validation("Username already exists".to_string()));
        }

        let user = sqlx::query_as!(
            UserRecord,
            r#"
            INSERT INTO users (email, username, password_hash, first_name, last_name, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            "#,
            input.email,
            input.username,
            password_hash,
            input.first_name,
            input.last_name,
            serde_json::to_value(&input.metadata).unwrap_or_default()
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(User::from_record(user))
    }

    async fn find_by_id(&self, id: Self::Id) -> Result<Option<User>, DatabaseError> {
        let user = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(user.map(User::from_record))
    }

    async fn update(&self, id: Self::Id, input: Self::UpdateInput) -> Result<User, DatabaseError> {
        let user = sqlx::query_as!(
            UserRecord,
            r#"
            UPDATE users
            SET
                email = COALESCE($2, email),
                username = COALESCE($3, username),
                first_name = COALESCE($4, first_name),
                last_name = COALESCE($5, last_name),
                metadata = COALESCE($6, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, email, username, password_hash, first_name, last_name,
                email_verified, status as "status: UserStatus",
                subscription_tier as "subscription_tier: SubscriptionTier",
                created_at, updated_at, last_login_at, metadata
            "#,
            id,
            input.email,
            input.username,
            input.first_name,
            input.last_name,
            input.metadata.map(|m| serde_json::to_value(m).unwrap_or_default())
        )
        .fetch_optional(&*self.pool)
        .await?;

        user.map(User::from_record)
            .ok_or_else(|| DatabaseError::Validation("User not found".to_string()))
    }

    async fn delete(&self, id: Self::Id) -> Result<bool, DatabaseError> {
        let result = sqlx::query!(
            "UPDATE users SET status = 'deleted', updated_at = NOW() WHERE id = $1",
            id
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn find_many(&self, filter: Self::QueryFilter) -> Result<Vec<User>, DatabaseError> {
        let mut query = String::from(
            r#"
            SELECT
                id, email, username, password_hash, first_name, last_name,
                email_verified, status, subscription_tier,
                created_at, updated_at, last_login_at, metadata
            FROM users
            WHERE 1=1
            "#
        );

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 1;

        // Apply filters
        if let Some(status) = &filter.status {
            conditions.push(format!("AND status = ${}", param_count));
            params.push(Box::new(match status {
                UserStatus::Active => "active",
                UserStatus::Suspended => "suspended",
                UserStatus::Deleted => "deleted",
            }));
            param_count += 1;
        }

        if let Some(subscription_tier) = &filter.subscription_tier {
            conditions.push(format!("AND subscription_tier = ${}", param_count));
            params.push(Box::new(match subscription_tier {
                SubscriptionTier::Free => "free",
                SubscriptionTier::Pro => "pro",
                SubscriptionTier::Enterprise => "enterprise",
            }));
            param_count += 1;
        }

        if let Some(email_verified) = filter.email_verified {
            conditions.push(format!("AND email_verified = ${}", param_count));
            params.push(Box::new(email_verified));
            param_count += 1;
        }

        if let Some(search) = &filter.search {
            conditions.push(format!("AND (email ILIKE ${0} OR username ILIKE ${0} OR first_name ILIKE ${0} OR last_name ILIKE ${0})", param_count));
            params.push(Box::new(format!("%{}%", search)));
            param_count += 1;
        }

        // Add conditions to query
        for condition in conditions {
            query.push_str(&condition);
        }

        // Add sorting
        let sort_field = match filter.sort.field.as_str() {
            "email" => "email",
            "username" => "username",
            "created_at" => "created_at",
            "updated_at" => "updated_at",
            "last_login_at" => "last_login_at",
            _ => "created_at",
        };

        let sort_direction = match filter.sort.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        query.push_str(&format!(" ORDER BY {} {}", sort_field, sort_direction));

        // Add pagination
        query.push_str(&format!(" LIMIT {} OFFSET {}", filter.pagination.limit, filter.pagination.offset));

        // This is a simplified version - in practice, you'd use a query builder
        // or macro to handle dynamic queries more safely
        let users = sqlx::query(&query)
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|row| User {
                id: row.get("id"),
                email: row.get("email"),
                username: row.get("username"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                email_verified: row.get("email_verified"),
                status: match row.get::<String, _>("status").as_str() {
                    "active" => UserStatus::Active,
                    "suspended" => UserStatus::Suspended,
                    "deleted" => UserStatus::Deleted,
                    _ => UserStatus::Active,
                },
                subscription_tier: match row.get::<String, _>("subscription_tier").as_str() {
                    "free" => SubscriptionTier::Free,
                    "pro" => SubscriptionTier::Pro,
                    "enterprise" => SubscriptionTier::Enterprise,
                    _ => SubscriptionTier::Free,
                },
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_login_at: row.get("last_login_at"),
                metadata: row.get::<serde_json::Value, _>("metadata"),
            })
            .collect();

        Ok(users)
    }

    async fn count(&self, filter: Self::QueryFilter) -> Result<u64, DatabaseError> {
        // Simplified count query - similar dynamic building would be needed
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status != 'deleted'")
            .fetch_one(&*self.pool)
            .await?;

        Ok(count as u64)
    }
}

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
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

impl User {
    fn from_record(record: UserRecord) -> Self {
        Self {
            id: record.id,
            email: record.email,
            username: record.username,
            first_name: record.first_name,
            last_name: record.last_name,
            email_verified: record.email_verified,
            status: record.status,
            subscription_tier: record.subscription_tier,
            created_at: record.created_at,
            updated_at: record.updated_at,
            last_login_at: record.last_login_at,
            metadata: record.metadata,
        }
    }

    pub fn full_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => self.username.clone(),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    pub fn is_premium(&self) -> bool {
        matches!(self.subscription_tier, SubscriptionTier::Pro | SubscriptionTier::Enterprise)
    }
}

/// Database record structure
#[derive(sqlx::FromRow)]
struct UserRecord {
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

/// User status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum UserStatus {
    #[sqlx(rename = "active")]
    Active,
    #[sqlx(rename = "suspended")]
    Suspended,
    #[sqlx(rename = "deleted")]
    Deleted,
}

/// Subscription tier enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum SubscriptionTier {
    #[sqlx(rename = "free")]
    Free,
    #[sqlx(rename = "pro")]
    Pro,
    #[sqlx(rename = "enterprise")]
    Enterprise,
}

/// Create user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub email: String,
    pub username: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub metadata: serde_json::Value,
}

/// Update user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// User filter for queries
#[derive(Debug, Clone)]
pub struct UserFilter {
    pub status: Option<UserStatus>,
    pub subscription_tier: Option<SubscriptionTier>,
    pub email_verified: Option<bool>,
    pub search: Option<String>,
    pub pagination: Pagination,
    pub sort: Sort,
}

impl Default for UserFilter {
    fn default() -> Self {
        Self {
            status: Some(UserStatus::Active),
            subscription_tier: None,
            email_verified: None,
            search: None,
            pagination: Pagination::default(),
            sort: Sort::default(),
        }
    }
}

impl QueryFilter for UserFilter {
    fn apply_pagination(&mut self, pagination: Pagination) {
        self.pagination = pagination;
    }

    fn apply_sort(&mut self, sort: Sort) {
        self.sort = sort;
    }

    fn get_pagination(&self) -> &Pagination {
        &self.pagination
    }

    fn get_sort(&self) -> &Sort {
        &self.sort
    }
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub total_users: u64,
    pub active_users: u64,
    pub suspended_users: u64,
    pub verified_users: u64,
    pub pro_users: u64,
    pub enterprise_users: u64,
    pub recent_signups: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_full_name() {
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            email_verified: true,
            status: UserStatus::Active,
            subscription_tier: SubscriptionTier::Free,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
            metadata: serde_json::Value::Null,
        };

        assert_eq!(user.full_name(), "John Doe");
        assert!(user.is_active());
        assert!(!user.is_premium());
    }

    #[test]
    fn test_pagination() {
        let pagination = Pagination::new(2, 10);
        assert_eq!(pagination.offset, 10);
    }
}
