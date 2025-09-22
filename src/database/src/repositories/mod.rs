//! Repository pattern implementation for AI-CORE database layer
//!
//! This module provides a unified data access layer using the repository pattern
//! for all four database types: PostgreSQL, MongoDB, ClickHouse, and Redis.
//!
//! Architecture:
//! - Repository traits define common operations
//! - Concrete repositories implement database-specific logic
//! - Factory pattern for creating repository instances
//! - Transaction support for cross-database operations

// pub mod users;
// pub mod workflows;
pub mod postgresql;
// pub mod content;
// pub mod analytics;
// pub mod cache;
// pub mod billing;
// pub mod federation;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// pub use users::*;
// pub use workflows::*;
pub use postgresql::*;
// pub use content::*;
// pub use analytics::*;
// pub use cache::*;
// pub use billing::*;
// pub use federation::*;

use super::DatabaseError;

/// Repository factory for creating repository instances
#[derive(Clone)]
pub struct RepositoryFactory {
    postgres: Arc<PgPool>,
}

impl RepositoryFactory {
    /// Create new repository factory
    pub fn new(postgres: Arc<PgPool>) -> Self {
        Self { postgres }
    }

    /// Get PostgreSQL repository
    pub fn postgres(&self) -> PostgresRepository {
        PostgresRepository::new(self.postgres.clone())
    }

    // /// Get user repository
    // pub fn users(&self) -> UserRepository {
    //     UserRepository::new(self.postgres.clone())
    // }

    // /// Get workflow repository
    // pub fn workflows(&self) -> WorkflowRepository {
    //     WorkflowRepository::new(self.postgres.clone(), self.mongo.clone())
    // }

    // /// Get content repository
    // pub fn content(&self) -> ContentRepository {
    //     ContentRepository::new(self.mongo.clone())
    // }

    // /// Get analytics repository
    // pub fn analytics(&self) -> AnalyticsRepository {
    //     AnalyticsRepository::new(self.clickhouse.clone())
    // }

    // /// Get cache repository
    // pub fn cache(&self) -> CacheRepository {
    //     CacheRepository::new(self.redis.clone())
    // }

    // /// Get billing repository
    // pub fn billing(&self) -> BillingRepository {
    //     BillingRepository::new(self.postgres.clone())
    // }

    // /// Get federation repository
    // pub fn federation(&self) -> FederationRepository {
    //     FederationRepository::new(self.postgres.clone(), self.mongo.clone())
    // }
}

/// Base repository trait for common operations
#[async_trait]
pub trait Repository<T> {
    type Id;
    type CreateInput;
    type UpdateInput;
    type QueryFilter;

    /// Create a new entity
    async fn create(&self, input: Self::CreateInput) -> Result<T, DatabaseError>;

    /// Find entity by ID
    async fn find_by_id(&self, id: Self::Id) -> Result<Option<T>, DatabaseError>;

    /// Update entity
    async fn update(&self, id: Self::Id, input: Self::UpdateInput) -> Result<T, DatabaseError>;

    /// Delete entity
    async fn delete(&self, id: Self::Id) -> Result<bool, DatabaseError>;

    /// Find entities with filter
    async fn find_many(&self, filter: Self::QueryFilter) -> Result<Vec<T>, DatabaseError>;

    /// Count entities matching filter
    async fn count(&self, filter: Self::QueryFilter) -> Result<u64, DatabaseError>;
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub offset: u32,
}

impl Pagination {
    pub fn new(page: u32, limit: u32) -> Self {
        let limit = limit.min(100).max(1); // Enforce reasonable limits
        let offset = (page.saturating_sub(1)) * limit;
        Self {
            page,
            limit,
            offset,
        }
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self::new(1, 20)
    }
}

/// Sorting parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sort {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            field: "created_at".to_string(),
            direction: SortDirection::Desc,
        }
    }
}

/// Query result with pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(page: u32, limit: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
        Self {
            page,
            limit,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Base filter trait for repositories
pub trait QueryFilter {
    fn apply_pagination(&mut self, pagination: Pagination);
    fn apply_sort(&mut self, sort: Sort);
    fn get_pagination(&self) -> &Pagination;
    fn get_sort(&self) -> &Sort;
}

/// Transaction context for repository operations
pub struct RepositoryTransaction {
    pub postgres_tx: Option<sqlx::Transaction<'static, sqlx::Postgres>>,
    pub repositories: RepositoryFactory,
}

impl RepositoryTransaction {
    pub fn new(
        postgres_tx: Option<sqlx::Transaction<'static, sqlx::Postgres>>,
        repositories: RepositoryFactory,
    ) -> Self {
        Self {
            postgres_tx,
            repositories,
        }
    }

    /// Commit the transaction
    pub async fn commit(self) -> Result<(), DatabaseError> {
        if let Some(tx) = self.postgres_tx {
            tx.commit().await?;
        }
        Ok(())
    }

    /// Rollback the transaction
    pub async fn rollback(self) -> Result<(), DatabaseError> {
        if let Some(tx) = self.postgres_tx {
            tx.rollback().await?;
        }
        Ok(())
    }
}

/// Common error types for repositories
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Concurrent modification: {0}")]
    ConcurrentModification(String),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
}

/// Entity trait for common entity properties
pub trait Entity {
    type Id;

    fn id(&self) -> Self::Id;
    fn created_at(&self) -> DateTime<Utc>;
    fn updated_at(&self) -> DateTime<Utc>;
}

/// Auditable entity trait for entities that track changes
pub trait AuditableEntity: Entity {
    fn created_by(&self) -> Option<Uuid>;
    fn updated_by(&self) -> Option<Uuid>;
}

/// Soft delete trait for entities that support soft deletion
#[async_trait]
pub trait SoftDeletable: Entity {
    async fn soft_delete(&mut self) -> Result<(), DatabaseError>;
    fn is_deleted(&self) -> bool;
    fn deleted_at(&self) -> Option<DateTime<Utc>>;
}

/// Cacheable entity trait for entities that can be cached
#[async_trait]
pub trait Cacheable: Entity {
    fn cache_key(&self) -> String;
    fn cache_ttl(&self) -> Option<u64>;

    // async fn invalidate_cache(&self, cache: &CacheRepository) -> Result<(), DatabaseError> {
    //     cache.delete(&self.cache_key()).await
    // }
}

/// Searchable entity trait for full-text search capabilities
#[async_trait]
pub trait Searchable: Entity {
    async fn index_for_search(&self) -> Result<(), DatabaseError>;
    fn search_fields(&self) -> Vec<String>;
}

/// Metrics for repository operations
#[derive(Debug, Clone, Default)]
pub struct RepositoryMetrics {
    pub queries_executed: u64,
    pub queries_cached: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_query_time_ms: f64,
    pub slow_queries: u64,
}

impl RepositoryMetrics {
    pub fn record_query(&mut self, duration_ms: f64, was_cached: bool) {
        self.queries_executed += 1;

        if was_cached {
            self.queries_cached += 1;
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }

        // Update running average
        self.average_query_time_ms =
            (self.average_query_time_ms * (self.queries_executed as f64 - 1.0) + duration_ms)
                / self.queries_executed as f64;

        // Track slow queries (>1 second)
        if duration_ms > 1000.0 {
            self.slow_queries += 1;
        }
    }
}

/// Repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub enable_metrics: bool,
    pub slow_query_threshold_ms: u64,
    pub max_batch_size: u32,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
            enable_metrics: true,
            slow_query_threshold_ms: 1000, // 1 second
            max_batch_size: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let pagination = Pagination::new(2, 10);
        assert_eq!(pagination.page, 2);
        assert_eq!(pagination.limit, 10);
        assert_eq!(pagination.offset, 10);
    }

    #[test]
    fn test_pagination_limits() {
        let pagination = Pagination::new(1, 200); // Should be capped at 100
        assert_eq!(pagination.limit, 100);

        let pagination = Pagination::new(1, 0); // Should be at least 1
        assert_eq!(pagination.limit, 1);
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(2, 10, 45);
        assert_eq!(meta.page, 2);
        assert_eq!(meta.limit, 10);
        assert_eq!(meta.total, 45);
        assert_eq!(meta.total_pages, 5);
        assert!(meta.has_next);
        assert!(meta.has_prev);
    }

    #[test]
    fn test_repository_metrics() {
        let mut metrics = RepositoryMetrics::default();
        metrics.record_query(500.0, false);
        metrics.record_query(1500.0, true);

        assert_eq!(metrics.queries_executed, 2);
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.cache_misses, 1);
        assert_eq!(metrics.slow_queries, 1);
        assert_eq!(metrics.average_query_time_ms, 1000.0);
    }
}
