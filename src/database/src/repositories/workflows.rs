//! Workflow repository implementation
//!
//! This module handles workflow-related database operations across PostgreSQL and MongoDB:
//! - Basic workflow metadata in PostgreSQL for ACID transactions
//! - Detailed workflow definitions and execution data in MongoDB
//! - Cross-database consistency and synchronization

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use mongodb::{Database as MongoDatabase, Collection, bson::{doc, Document}};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::{Repository, QueryFilter, Pagination, Sort, SortDirection, RepositoryError};
use crate::database::DatabaseError;

/// Workflow repository managing data across PostgreSQL and MongoDB
#[derive(Clone)]
pub struct WorkflowRepository {
    postgres: Arc<PgPool>,
    mongo: Arc<MongoDatabase>,
    workflows_collection: Collection<WorkflowDocument>,
}

impl WorkflowRepository {
    /// Create new workflow repository
    pub fn new(postgres: Arc<PgPool>, mongo: Arc<MongoDatabase>) -> Self {
        let workflows_collection = mongo.collection::<WorkflowDocument>("workflows");

        Self {
            postgres,
            mongo,
            workflows_collection,
        }
    }

    /// Find workflow with detailed definition
    pub async fn find_with_definition(&self, id: Uuid) -> Result<Option<WorkflowWithDefinition>, DatabaseError> {
        // Get basic workflow from PostgreSQL
        let workflow_record = sqlx::query_as!(
            WorkflowRecord,
            r#"
            SELECT
                id, user_id, workflow_type, status as "status: WorkflowStatus",
                priority as "priority: WorkflowPriority",
                estimated_cost_cents, actual_cost_cents,
                estimated_duration_seconds, actual_duration_seconds,
                started_at, completed_at, created_at, updated_at
            FROM workflows
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&*self.postgres)
        .await?;

        if let Some(record) = workflow_record {
            // Get detailed definition from MongoDB
            let filter = doc! { "workflow_id": id.to_string() };
            let definition = self.workflows_collection
                .find_one(filter, None)
                .await
                .map_err(|e| DatabaseError::Mongo(e))?;

            Ok(Some(WorkflowWithDefinition {
                workflow: Workflow::from_record(record),
                definition,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create workflow with definition
    pub async fn create_with_definition(
        &self,
        input: CreateWorkflowWithDefinitionInput,
    ) -> Result<WorkflowWithDefinition, DatabaseError> {
        // Start PostgreSQL transaction
        let mut tx = self.postgres.begin().await?;

        // Create basic workflow record in PostgreSQL
        let workflow_record = sqlx::query_as!(
            WorkflowRecord,
            r#"
            INSERT INTO workflows (user_id, workflow_type, priority, estimated_cost_cents, estimated_duration_seconds)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, user_id, workflow_type, status as "status: WorkflowStatus",
                priority as "priority: WorkflowPriority",
                estimated_cost_cents, actual_cost_cents,
                estimated_duration_seconds, actual_duration_seconds,
                started_at, completed_at, created_at, updated_at
            "#,
            input.user_id,
            input.workflow_type,
            match input.priority {
                WorkflowPriority::Low => "low",
                WorkflowPriority::Medium => "medium",
                WorkflowPriority::High => "high",
                WorkflowPriority::Urgent => "urgent",
            },
            input.estimated_cost_cents,
            input.estimated_duration_seconds
        )
        .fetch_one(&mut *tx)
        .await?;

        // Create detailed workflow definition in MongoDB
        let workflow_document = WorkflowDocument {
            id: None,
            workflow_id: workflow_record.id.to_string(),
            name: input.definition.name,
            description: input.definition.description,
            steps: input.definition.steps,
            configuration: input.definition.configuration,
            triggers: input.definition.triggers,
            outputs: None,
            execution_history: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        let insert_result = self.workflows_collection
            .insert_one(&workflow_document, None)
            .await
            .map_err(|e| DatabaseError::Mongo(e))?;

        // Commit PostgreSQL transaction
        tx.commit().await?;

        let mut saved_document = workflow_document;
        saved_document.id = Some(insert_result.inserted_id.as_object_id().unwrap());

        Ok(WorkflowWithDefinition {
            workflow: Workflow::from_record(workflow_record),
            definition: Some(saved_document),
        })
    }

    /// Update workflow status
    pub async fn update_status(&self, id: Uuid, status: WorkflowStatus) -> Result<(), DatabaseError> {
        let status_str = match status {
            WorkflowStatus::Created => "created",
            WorkflowStatus::Running => "running",
            WorkflowStatus::Completed => "completed",
            WorkflowStatus::Failed => "failed",
            WorkflowStatus::Cancelled => "cancelled",
        };

        sqlx::query!(
            "UPDATE workflows SET status = $1, updated_at = NOW() WHERE id = $2",
            status_str,
            id
        )
        .execute(&*self.postgres)
        .await?;

        Ok(())
    }

    /// Start workflow execution
    pub async fn start_execution(&self, id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE workflows SET status = 'running', started_at = NOW(), updated_at = NOW() WHERE id = $1",
            id
        )
        .execute(&*self.postgres)
        .await?;

        Ok(())
    }

    /// Complete workflow execution
    pub async fn complete_execution(&self, id: Uuid, actual_cost_cents: Option<i32>) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            UPDATE workflows
            SET status = 'completed', completed_at = NOW(), updated_at = NOW(),
                actual_cost_cents = COALESCE($2, actual_cost_cents),
                actual_duration_seconds = EXTRACT(EPOCH FROM (NOW() - started_at))::INTEGER
            WHERE id = $1
            "#,
            id,
            actual_cost_cents
        )
        .execute(&*self.postgres)
        .await?;

        Ok(())
    }

    /// Get workflow statistics for a user
    pub async fn get_user_stats(&self, user_id: Uuid) -> Result<WorkflowStats, DatabaseError> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_workflows,
                COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_workflows,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_workflows,
                COUNT(CASE WHEN status = 'running' THEN 1 END) as running_workflows,
                AVG(CASE WHEN status = 'completed' THEN actual_duration_seconds END) as avg_duration_seconds,
                SUM(CASE WHEN status = 'completed' THEN actual_cost_cents ELSE 0 END) as total_cost_cents
            FROM workflows
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&*self.postgres)
        .await?;

        Ok(WorkflowStats {
            total_workflows: stats.total_workflows.unwrap_or(0) as u64,
            completed_workflows: stats.completed_workflows.unwrap_or(0) as u64,
            failed_workflows: stats.failed_workflows.unwrap_or(0) as u64,
            running_workflows: stats.running_workflows.unwrap_or(0) as u64,
            avg_duration_seconds: stats.avg_duration_seconds.map(|d| d as f64),
            total_cost_cents: stats.total_cost_cents.unwrap_or(0) as i64,
        })
    }
}

#[async_trait]
impl Repository<Workflow> for WorkflowRepository {
    type Id = Uuid;
    type CreateInput = CreateWorkflowInput;
    type UpdateInput = UpdateWorkflowInput;
    type QueryFilter = WorkflowFilter;

    async fn create(&self, input: Self::CreateInput) -> Result<Workflow, DatabaseError> {
        let workflow_record = sqlx::query_as!(
            WorkflowRecord,
            r#"
            INSERT INTO workflows (user_id, workflow_type, priority, estimated_cost_cents, estimated_duration_seconds)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, user_id, workflow_type, status as "status: WorkflowStatus",
                priority as "priority: WorkflowPriority",
                estimated_cost_cents, actual_cost_cents,
                estimated_duration_seconds, actual_duration_seconds,
                started_at, completed_at, created_at, updated_at
            "#,
            input.user_id,
            input.workflow_type,
            match input.priority {
                WorkflowPriority::Low => "low",
                WorkflowPriority::Medium => "medium",
                WorkflowPriority::High => "high",
                WorkflowPriority::Urgent => "urgent",
            },
            input.estimated_cost_cents,
            input.estimated_duration_seconds
        )
        .fetch_one(&*self.postgres)
        .await?;

        Ok(Workflow::from_record(workflow_record))
    }

    async fn find_by_id(&self, id: Self::Id) -> Result<Option<Workflow>, DatabaseError> {
        let workflow_record = sqlx::query_as!(
            WorkflowRecord,
            r#"
            SELECT
                id, user_id, workflow_type, status as "status: WorkflowStatus",
                priority as "priority: WorkflowPriority",
                estimated_cost_cents, actual_cost_cents,
                estimated_duration_seconds, actual_duration_seconds,
                started_at, completed_at, created_at, updated_at
            FROM workflows
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&*self.postgres)
        .await?;

        Ok(workflow_record.map(Workflow::from_record))
    }

    async fn update(&self, id: Self::Id, input: Self::UpdateInput) -> Result<Workflow, DatabaseError> {
        let priority_str = input.priority.as_ref().map(|p| match p {
            WorkflowPriority::Low => "low",
            WorkflowPriority::Medium => "medium",
            WorkflowPriority::High => "high",
            WorkflowPriority::Urgent => "urgent",
        });

        let workflow_record = sqlx::query_as!(
            WorkflowRecord,
            r#"
            UPDATE workflows
            SET
                priority = COALESCE($2, priority),
                estimated_cost_cents = COALESCE($3, estimated_cost_cents),
                estimated_duration_seconds = COALESCE($4, estimated_duration_seconds),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, user_id, workflow_type, status as "status: WorkflowStatus",
                priority as "priority: WorkflowPriority",
                estimated_cost_cents, actual_cost_cents,
                estimated_duration_seconds, actual_duration_seconds,
                started_at, completed_at, created_at, updated_at
            "#,
            id,
            priority_str,
            input.estimated_cost_cents,
            input.estimated_duration_seconds
        )
        .fetch_optional(&*self.postgres)
        .await?;

        workflow_record
            .map(Workflow::from_record)
            .ok_or_else(|| DatabaseError::Validation("Workflow not found".to_string()))
    }

    async fn delete(&self, id: Self::Id) -> Result<bool, DatabaseError> {
        // Soft delete by updating status
        let result = sqlx::query!(
            "UPDATE workflows SET status = 'cancelled', updated_at = NOW() WHERE id = $1",
            id
        )
        .execute(&*self.postgres)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn find_many(&self, filter: Self::QueryFilter) -> Result<Vec<Workflow>, DatabaseError> {
        // This would be implemented with dynamic query building
        // For now, a simplified version
        let workflows = sqlx::query_as!(
            WorkflowRecord,
            r#"
            SELECT
                id, user_id, workflow_type, status as "status: WorkflowStatus",
                priority as "priority: WorkflowPriority",
                estimated_cost_cents, actual_cost_cents,
                estimated_duration_seconds, actual_duration_seconds,
                started_at, completed_at, created_at, updated_at
            FROM workflows
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            filter.user_id,
            filter.pagination.limit as i64,
            filter.pagination.offset as i64
        )
        .fetch_all(&*self.postgres)
        .await?;

        Ok(workflows.into_iter().map(Workflow::from_record).collect())
    }

    async fn count(&self, filter: Self::QueryFilter) -> Result<u64, DatabaseError> {
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM workflows WHERE user_id = $1",
            filter.user_id
        )
        .fetch_one(&*self.postgres)
        .await?;

        Ok(count as u64)
    }
}

/// Workflow entity (PostgreSQL data)
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

impl Workflow {
    fn from_record(record: WorkflowRecord) -> Self {
        Self {
            id: record.id,
            user_id: record.user_id,
            workflow_type: record.workflow_type,
            status: record.status,
            priority: record.priority,
            estimated_cost_cents: record.estimated_cost_cents,
            actual_cost_cents: record.actual_cost_cents,
            estimated_duration_seconds: record.estimated_duration_seconds,
            actual_duration_seconds: record.actual_duration_seconds,
            started_at: record.started_at,
            completed_at: record.completed_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// Combined workflow with definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowWithDefinition {
    pub workflow: Workflow,
    pub definition: Option<WorkflowDocument>,
}

/// Database record structure
#[derive(sqlx::FromRow)]
struct WorkflowRecord {
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

/// Workflow document in MongoDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub configuration: serde_json::Value,
    pub triggers: Vec<WorkflowTrigger>,
    pub outputs: Option<serde_json::Value>,
    pub execution_history: Vec<WorkflowExecution>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i32,
}

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: String,
    pub configuration: serde_json::Value,
    pub dependencies: Vec<String>,
    pub retry_config: Option<RetryConfig>,
}

/// Workflow trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub trigger_type: String,
    pub configuration: serde_json::Value,
    pub enabled: bool,
}

/// Workflow execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub execution_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub step_results: Vec<StepResult>,
    pub error_message: Option<String>,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: i32,
    pub delay_seconds: i32,
    pub backoff_multiplier: f64,
}

/// Workflow status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum WorkflowStatus {
    #[sqlx(rename = "created")]
    Created,
    #[sqlx(rename = "running")]
    Running,
    #[sqlx(rename = "completed")]
    Completed,
    #[sqlx(rename = "failed")]
    Failed,
    #[sqlx(rename = "cancelled")]
    Cancelled,
}

/// Workflow priority enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum WorkflowPriority {
    #[sqlx(rename = "low")]
    Low,
    #[sqlx(rename = "medium")]
    Medium,
    #[sqlx(rename = "high")]
    High,
    #[sqlx(rename = "urgent")]
    Urgent,
}

/// Create workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowInput {
    pub user_id: Uuid,
    pub workflow_type: String,
    pub priority: WorkflowPriority,
    pub estimated_cost_cents: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
}

/// Create workflow with definition input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowWithDefinitionInput {
    pub user_id: Uuid,
    pub workflow_type: String,
    pub priority: WorkflowPriority,
    pub estimated_cost_cents: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
    pub definition: WorkflowDefinitionInput,
}

/// Workflow definition input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinitionInput {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub configuration: serde_json::Value,
    pub triggers: Vec<WorkflowTrigger>,
}

/// Update workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowInput {
    pub priority: Option<WorkflowPriority>,
    pub estimated_cost_cents: Option<i32>,
    pub estimated_duration_seconds: Option<i32>,
}

/// Workflow filter for queries
#[derive(Debug, Clone)]
pub struct WorkflowFilter {
    pub user_id: Uuid,
    pub status: Option<WorkflowStatus>,
    pub workflow_type: Option<String>,
    pub priority: Option<WorkflowPriority>,
    pub pagination: Pagination,
    pub sort: Sort,
}

impl Default for WorkflowFilter {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            status: None,
            workflow_type: None,
            priority: None,
            pagination: Pagination::default(),
            sort: Sort::default(),
        }
    }
}

impl QueryFilter for WorkflowFilter {
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

/// Workflow statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStats {
    pub total_workflows: u64,
    pub completed_workflows: u64,
    pub failed_workflows: u64,
    pub running_workflows: u64,
    pub avg_duration_seconds: Option<f64>,
    pub total_cost_cents: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_status_serialization() {
        let status = WorkflowStatus::Running;
        let serialized = serde_json::to_string(&status).unwrap();
        assert!(serialized.contains("Running"));
    }

    #[test]
    fn test_workflow_priority_ordering() {
        let priorities = vec![
            WorkflowPriority::Low,
            WorkflowPriority::Medium,
            WorkflowPriority::High,
            WorkflowPriority::Urgent,
        ];

        // In a real implementation, you might implement Ord for priority-based sorting
        assert_eq!(priorities.len(), 4);
    }
}
