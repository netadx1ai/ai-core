//! Workflow service for CRUD operations and workflow management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    handlers::workflows::{WorkflowConfig, WorkflowResponse},
};
use ai_core_shared::types::core::{WorkflowStatus, WorkflowTrigger};

/// Workflow service for managing workflow lifecycle
#[derive(Clone)]
pub struct WorkflowService {
    db_pool: PgPool,
}

/// Internal workflow representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub definition: String,
    pub parsed_definition: serde_json::Value,
    pub triggers: Vec<WorkflowTrigger>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub config: Option<WorkflowConfig>,
    pub tags: Vec<String>,
    pub status: WorkflowStatus,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
    pub execution_count: u64,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub success_count: u64,
    pub failure_count: u64,
}

impl Workflow {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.execution_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.execution_count as f64
        }
    }
}

impl From<Workflow> for WorkflowResponse {
    fn from(workflow: Workflow) -> Self {
        WorkflowResponse {
            id: workflow.id.clone(),
            title: workflow.title.clone(),
            description: workflow.description.clone(),
            definition: workflow.definition.clone(),
            triggers: workflow.triggers.clone(),
            input_schema: workflow.input_schema.clone(),
            output_schema: workflow.output_schema.clone(),
            config: workflow.config.clone(),
            tags: workflow.tags.clone(),
            status: workflow.status.clone(),
            is_active: workflow.is_active,
            created_by: workflow.created_by.clone(),
            created_at: workflow.created_at,
            updated_at: workflow.updated_at,
            version: workflow.version,
            execution_count: workflow.execution_count,
            last_executed_at: workflow.last_executed_at,
            success_rate: workflow.success_rate(),
        }
    }
}

impl WorkflowService {
    /// Create new workflow service
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Create a new workflow
    pub async fn create_workflow(
        &self,
        id: &str,
        title: &str,
        description: Option<&str>,
        definition: &str,
        parsed_definition: &serde_json::Value,
        triggers: &[WorkflowTrigger],
        input_schema: Option<&serde_json::Value>,
        output_schema: Option<&serde_json::Value>,
        config: Option<&WorkflowConfig>,
        tags: &[String],
        is_active: bool,
        created_by: &str,
    ) -> Result<WorkflowResponse> {
        info!("Creating workflow: {} by user: {}", title, created_by);

        let now = Utc::now();
        let triggers_json = serde_json::to_value(triggers)
            .map_err(|e| ApiError::internal(format!("Failed to serialize triggers: {}", e)))?;
        let tags_json = serde_json::to_value(tags)
            .map_err(|e| ApiError::internal(format!("Failed to serialize tags: {}", e)))?;
        let config_json = config
            .map(|c| serde_json::to_value(c))
            .transpose()
            .map_err(|e| ApiError::internal(format!("Failed to serialize config: {}", e)))?;

        // Insert workflow into database
        // Mock workflow creation (replace with actual database call when database is ready)
        let workflow = MockWorkflowRecord {
            id: id.to_string(),
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
            definition: definition.to_string(),
            parsed_definition: parsed_definition.clone(),
            triggers: triggers_json,
            input_schema: input_schema.cloned(),
            output_schema: output_schema.cloned(),
            config: config_json,
            tags: tags_json,
            status: WorkflowStatus::Created,
            is_active,
            created_by: created_by.to_string(),
            created_at: now,
            updated_at: now,
            version: 1,
            execution_count: 0,
            last_executed_at: None,
            success_count: 0,
            failure_count: 0,
        };

        info!("Workflow created successfully: {}", id);
        Ok(workflow.into_workflow()?.into())
    }

    /// Get workflow by ID
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowResponse>> {
        debug!("Getting workflow: {}", workflow_id);

        // Mock workflow retrieval (replace with actual database call when database is ready)
        let workflow = if workflow_id == "mock-workflow-1" {
            Some(MockWorkflowRecord {
                id: workflow_id.to_string(),
                title: "Mock Workflow 1".to_string(),
                description: Some("A mock workflow for testing".to_string()),
                definition: "Create a blog post about AI".to_string(),
                parsed_definition: serde_json::json!({"steps": ["research", "write", "publish"]}),
                triggers: serde_json::json!([]),
                input_schema: None,
                output_schema: None,
                config: None,
                tags: serde_json::json!(["test", "mock"]),
                status: WorkflowStatus::Created,
                is_active: true,
                created_by: "admin-user-id".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: 1,
                execution_count: 0,
                last_executed_at: None,
                success_count: 0,
                failure_count: 0,
            })
        } else {
            None
        };

        match workflow {
            Some(w) => Ok(Some(w.into_workflow()?.into())),
            None => Ok(None),
        }
    }

    /// List workflows with filtering and pagination
    pub async fn list_workflows(
        &self,
        created_by: Option<&str>,
        status: Option<&WorkflowStatus>,
        tags: Option<&[String]>,
        search: Option<&str>,
        _created_by_filter: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<WorkflowResponse>, u64)> {
        debug!(
            "Listing workflows: created_by={:?}, status={:?}, tags={:?}, search={:?}",
            created_by, status, tags, search
        );

        // Mock workflow listing (replace with actual database query when database is ready)
        let mut workflows = vec![];
        let total_count = 1u64;

        // Create mock workflow if criteria match
        if created_by.map_or(true, |cb| cb == "admin-user-id")
            && status.map_or(true, |s| *s == WorkflowStatus::Created)
            && tags.map_or(true, |t| t.iter().any(|tag| tag == "test" || tag == "mock"))
            && search.map_or(true, |s| {
                "Mock Workflow".to_lowercase().contains(&s.to_lowercase())
            })
        {
            workflows.push(MockWorkflowRecord {
                id: "mock-workflow-1".to_string(),
                title: "Mock Workflow 1".to_string(),
                description: Some("A mock workflow for testing".to_string()),
                definition: "Create a blog post about AI".to_string(),
                parsed_definition: serde_json::json!({"steps": ["research", "write", "publish"]}),
                triggers: serde_json::json!([]),
                input_schema: None,
                output_schema: None,
                config: None,
                tags: serde_json::json!(["test", "mock"]),
                status: WorkflowStatus::Created,
                is_active: true,
                created_by: "admin-user-id".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: 1,
                execution_count: 0,
                last_executed_at: None,
                success_count: 0,
                failure_count: 0,
            });
        }

        let workflow_responses = workflows
            .into_iter()
            .map(|w| w.into_workflow().map(|workflow| workflow.into()))
            .collect::<Result<Vec<_>>>()?;

        Ok((workflow_responses, total_count as u64))
    }

    /// Update an existing workflow
    pub async fn update_workflow(
        &self,
        workflow_id: &str,
        title: Option<&str>,
        description: Option<&str>,
        definition: Option<&str>,
        parsed_definition: Option<&serde_json::Value>,
        triggers: Option<&[WorkflowTrigger]>,
        input_schema: Option<&serde_json::Value>,
        output_schema: Option<&serde_json::Value>,
        config: Option<&WorkflowConfig>,
        tags: Option<&[String]>,
        is_active: Option<bool>,
    ) -> Result<WorkflowResponse> {
        info!("Updating workflow: {}", workflow_id);

        // Check if any updates provided
        let has_updates = title.is_some()
            || description.is_some()
            || definition.is_some()
            || parsed_definition.is_some()
            || triggers.is_some()
            || input_schema.is_some()
            || output_schema.is_some()
            || config.is_some()
            || tags.is_some()
            || is_active.is_some();

        if !has_updates {
            return Err(ApiError::bad_request("No updates provided"));
        }

        // Mock workflow update (replace with actual database call when database is ready)
        let workflow = if workflow_id == "mock-workflow-1" {
            MockWorkflowRecord {
                id: workflow_id.to_string(),
                title: title
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Mock Workflow 1".to_string()),
                description: description
                    .map(|s| s.to_string())
                    .or_else(|| Some("A mock workflow for testing".to_string())),
                definition: definition
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Create a blog post about AI".to_string()),
                parsed_definition: parsed_definition.cloned().unwrap_or_else(
                    || serde_json::json!({"steps": ["research", "write", "publish"]}),
                ),
                triggers: triggers
                    .map(|t| serde_json::to_value(t).unwrap())
                    .unwrap_or_else(|| serde_json::json!([])),
                input_schema: input_schema.cloned(),
                output_schema: output_schema.cloned(),
                config: config.map(|c| serde_json::to_value(c).unwrap()),
                tags: tags
                    .map(|t| serde_json::to_value(t).unwrap())
                    .unwrap_or_else(|| serde_json::json!(["test", "mock"])),
                status: WorkflowStatus::Created,
                is_active: is_active.unwrap_or(true),
                created_by: "admin-user-id".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: 1,
                execution_count: 0,
                last_executed_at: None,
                success_count: 0,
                failure_count: 0,
            }
        } else {
            return Err(ApiError::not_found("Workflow not found"));
        };

        info!("Workflow updated successfully: {}", workflow_id);
        Ok(workflow.into_workflow()?.into())
    }

    /// Delete a workflow
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<()> {
        info!("Deleting workflow: {}", workflow_id);

        // Mock workflow deletion (replace with actual database call when database is ready)
        if workflow_id != "mock-workflow-1" {
            return Err(ApiError::not_found("Workflow not found"));
        }

        info!("Workflow deleted successfully: {}", workflow_id);
        Ok(())
    }

    /// Increment execution count for a workflow
    pub async fn increment_execution_count(&self, workflow_id: &str, success: bool) -> Result<()> {
        debug!("Incrementing execution count for workflow: {}", workflow_id);

        // Mock execution count increment (replace with actual database call when database is ready)
        if workflow_id != "mock-workflow-1" {
            return Err(ApiError::not_found("Workflow not found"));
        }
        // In a real implementation, this would update the database
        debug!(
            "Mock execution count increment: workflow={}, success={}",
            workflow_id, success
        );

        Ok(())
    }
}

/// Mock database record for workflows (will be replaced with actual SQLx record)
struct MockWorkflowRecord {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub definition: String,
    pub parsed_definition: serde_json::Value,
    pub triggers: serde_json::Value,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
    pub tags: serde_json::Value,
    pub status: WorkflowStatus,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i32,
    pub execution_count: i64,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub success_count: i64,
    pub failure_count: i64,
}

impl MockWorkflowRecord {
    /// Convert database record to workflow
    pub fn into_workflow(self) -> Result<Workflow> {
        let triggers: Vec<WorkflowTrigger> = serde_json::from_value(self.triggers)
            .map_err(|e| ApiError::internal(format!("Failed to deserialize triggers: {}", e)))?;

        let tags: Vec<String> = serde_json::from_value(self.tags)
            .map_err(|e| ApiError::internal(format!("Failed to deserialize tags: {}", e)))?;

        let config: Option<WorkflowConfig> = self
            .config
            .map(|c| serde_json::from_value(c))
            .transpose()
            .map_err(|e| ApiError::internal(format!("Failed to deserialize config: {}", e)))?;

        Ok(Workflow {
            id: self.id,
            title: self.title,
            description: self.description,
            definition: self.definition,
            parsed_definition: self.parsed_definition,
            triggers,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            config,
            tags,
            status: self.status,
            is_active: self.is_active,
            created_by: self.created_by,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version as u32,
            execution_count: self.execution_count as u64,
            last_executed_at: self.last_executed_at,
            success_count: self.success_count as u64,
            failure_count: self.failure_count as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::env;

    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/ai_core_test".to_string());

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_workflow_success_rate_calculation() {
        let workflow = Workflow {
            id: "test-id".to_string(),
            title: "Test Workflow".to_string(),
            description: None,
            definition: "test definition".to_string(),
            parsed_definition: serde_json::json!({}),
            triggers: vec![],
            input_schema: None,
            output_schema: None,
            config: None,
            tags: vec![],
            status: WorkflowStatus::Created,
            is_active: true,
            created_by: "test-user".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            execution_count: 10,
            last_executed_at: None,
            success_count: 8,
            failure_count: 2,
        };

        assert_eq!(workflow.success_rate(), 0.8);
    }

    #[tokio::test]
    async fn test_workflow_success_rate_zero_executions() {
        let workflow = Workflow {
            id: "test-id".to_string(),
            title: "Test Workflow".to_string(),
            description: None,
            definition: "test definition".to_string(),
            parsed_definition: serde_json::json!({}),
            triggers: vec![],
            input_schema: None,
            output_schema: None,
            config: None,
            tags: vec![],
            status: WorkflowStatus::Created,
            is_active: true,
            created_by: "test-user".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            execution_count: 0,
            last_executed_at: None,
            success_count: 0,
            failure_count: 0,
        };

        assert_eq!(workflow.success_rate(), 0.0);
    }
}
