//! Workflow handlers for CRUD operations, execution, and monitoring

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{ApiError, Result},
    middleware_layer::auth::{require_user_context, UserContext},
    state::AppState,
};
use ai_core_shared::types::core::{Permission, WorkflowStatus, WorkflowTrigger};

/// Create workflow request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateWorkflowRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Title must be between 1 and 200 characters"
    ))]
    pub title: String,

    #[validate(length(max = 1000, message = "Description must be less than 1000 characters"))]
    pub description: Option<String>,

    /// Natural language workflow definition
    #[validate(length(
        min = 10,
        message = "Workflow definition must be at least 10 characters"
    ))]
    pub definition: String,

    /// Workflow triggers
    pub triggers: Vec<WorkflowTrigger>,

    /// Input schema for the workflow
    pub input_schema: Option<serde_json::Value>,

    /// Output schema for the workflow
    pub output_schema: Option<serde_json::Value>,

    /// Workflow configuration
    pub config: Option<WorkflowConfig>,

    /// Tags for organization
    pub tags: Option<Vec<String>>,

    /// Whether workflow should be active immediately
    #[serde(default = "default_true")]
    pub is_active: bool,
}

/// Update workflow request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateWorkflowRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Title must be between 1 and 200 characters"
    ))]
    pub title: Option<String>,

    #[validate(length(max = 1000, message = "Description must be less than 1000 characters"))]
    pub description: Option<String>,

    pub definition: Option<String>,
    pub triggers: Option<Vec<WorkflowTrigger>>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub config: Option<WorkflowConfig>,
    pub tags: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

/// Execute workflow request
#[derive(Debug, Deserialize, Validate)]
pub struct ExecuteWorkflowRequest {
    /// Input data for the workflow
    pub input: serde_json::Value,

    /// Optional execution context
    pub context: Option<ExecutionContext>,

    /// Priority level for execution (1-10, higher is more priority)
    #[validate(range(min = 1, max = 10, message = "Priority must be between 1 and 10"))]
    pub priority: Option<u8>,

    /// Optional callback URL for completion notification
    #[validate(url(message = "Invalid callback URL"))]
    pub callback_url: Option<String>,

    /// Maximum execution time in seconds
    #[validate(range(
        min = 1,
        max = 3600,
        message = "Timeout must be between 1 and 3600 seconds"
    ))]
    pub timeout_seconds: Option<u32>,
}

/// Workflow configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowConfig {
    /// Maximum execution time in seconds
    pub timeout_seconds: Option<u32>,

    /// Maximum number of retries on failure
    pub max_retries: Option<u32>,

    /// Retry delay in seconds
    pub retry_delay_seconds: Option<u32>,

    /// Environment variables
    pub environment: Option<HashMap<String, String>>,

    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,

    /// Notification settings
    pub notifications: Option<NotificationConfig>,
}

/// Rate limit configuration for workflows
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RateLimitConfig {
    /// Maximum executions per minute
    pub per_minute: Option<u32>,
    /// Maximum executions per hour
    pub per_hour: Option<u32>,
    /// Maximum concurrent executions
    pub max_concurrent: Option<u32>,
}

/// Notification configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationConfig {
    /// Send notification on completion
    pub on_completion: bool,
    /// Send notification on failure
    pub on_failure: bool,
    /// Webhook URL for notifications
    pub webhook_url: Option<String>,
    /// Email addresses for notifications
    pub email_addresses: Option<Vec<String>>,
}

/// Execution context
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionContext {
    /// User-defined context variables
    pub variables: Option<HashMap<String, serde_json::Value>>,
    /// Execution environment
    pub environment: Option<String>,
    /// Parent execution ID for nested workflows
    pub parent_execution_id: Option<String>,
    /// Correlation ID for tracking
    pub correlation_id: Option<String>,
}

/// Workflow response
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub definition: String,
    pub triggers: Vec<WorkflowTrigger>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub config: Option<WorkflowConfig>,
    pub tags: Vec<String>,
    pub status: WorkflowStatus,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: u32,
    pub execution_count: u64,
    pub last_executed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub success_rate: f64,
}

/// Workflow execution response
#[derive(Debug, Serialize)]
pub struct WorkflowExecutionResponse {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub context: Option<ExecutionContext>,
    pub priority: u8,
    pub timeout_seconds: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub retry_count: u32,
    pub progress: Option<ExecutionProgress>,
    pub logs: Vec<ExecutionLog>,
}

/// Execution status
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Execution progress information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionProgress {
    pub current_step: String,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub percentage: f64,
    pub estimated_remaining_seconds: Option<u64>,
}

/// Execution log entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
    pub step: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Log level
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// List workflows query parameters
#[derive(Debug, Deserialize, Validate)]
pub struct ListWorkflowsQuery {
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100"))]
    pub limit: Option<u32>,

    pub offset: Option<u32>,
    pub status: Option<WorkflowStatus>,
    pub tags: Option<String>, // Comma-separated tags
    pub search: Option<String>,
    pub created_by: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

/// Sort order
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// List executions query parameters
#[derive(Debug, Deserialize, Validate)]
pub struct ListExecutionsQuery {
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100"))]
    pub limit: Option<u32>,

    pub offset: Option<u32>,
    pub status: Option<ExecutionStatus>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

/// Paginated response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
    pub has_next: bool,
}

fn default_true() -> bool {
    true
}

/// POST /workflows - Create a new workflow
pub async fn create_workflow(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsCreate) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:create required",
        ));
    }

    // Validate input
    payload.validate().map_err(|e| {
        ApiError::validation("workflow", format!("Invalid workflow request: {}", e))
    })?;

    info!(
        "Creating workflow '{}' for user: {}",
        payload.title, user_context.user_id
    );

    // Parse and validate workflow definition
    let parsed_workflow = state
        .intent_parser
        .parse_workflow_definition(&payload.definition)
        .await?;

    // Create workflow
    let workflow_id = Uuid::new_v4().to_string();

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    let workflow = workflow_service
        .create_workflow(
            &workflow_id,
            &payload.title,
            payload.description.as_deref(),
            &payload.definition,
            &serde_json::to_value(&parsed_workflow).unwrap_or_default(),
            &payload.triggers,
            payload.input_schema.as_ref(),
            payload.output_schema.as_ref(),
            payload.config.as_ref(),
            payload.tags.as_ref().unwrap_or(&vec![]),
            payload.is_active,
            &user_context.user_id,
        )
        .await?;

    // Record metrics
    state.metrics.record_workflow_created(
        &workflow.id.to_string(),
        &user_context.user_id,
        &user_context.subscription_tier.to_string(),
        "automation", // Default workflow type since WorkflowResponse doesn't have workflow_type field
    );

    info!("Workflow created successfully: {}", workflow_id);

    Ok(Json(workflow))
}

/// GET /workflows - List workflows
pub async fn list_workflows(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Query(query): Query<ListWorkflowsQuery>,
) -> Result<Json<PaginatedResponse<WorkflowResponse>>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsRead) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:read required",
        ));
    }

    // Validate query parameters
    query.validate().map_err(|e| {
        ApiError::validation(
            "execution_query",
            format!("Invalid query parameters: {}", e),
        )
    })?;

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    // Parse tags if provided
    let tags = query.tags.as_ref().map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    });

    // List workflows
    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    let (workflows, total) = workflow_service
        .list_workflows(
            Some(&user_context.user_id), // Only show user's workflows unless admin
            query.status.as_ref(),
            tags.as_ref().map(|v| v.as_slice()),
            query.search.as_deref(),
            query.created_by.as_deref(),
            query.sort_by.as_deref(),
            query.sort_order.as_ref().map(|o| match o {
                SortOrder::Asc => "asc",
                SortOrder::Desc => "desc",
            }),
            limit,
            offset,
        )
        .await?;

    Ok(Json(PaginatedResponse {
        items: workflows,
        total,
        limit,
        offset,
        has_next: offset + limit < total as u32,
    }))
}

/// GET /workflows/{workflow_id} - Get workflow by ID
pub async fn get_workflow(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(workflow_id): Path<String>,
) -> Result<Json<WorkflowResponse>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsRead) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:read required",
        ));
    }

    // Get workflow
    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    let workflow = workflow_service
        .get_workflow(&workflow_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Workflow not found"))?;

    // Check ownership unless admin
    if !user_context.is_admin() && workflow.created_by != user_context.user_id {
        return Err(ApiError::authorization(
            "Access denied: workflow belongs to another user",
        ));
    }

    info!(
        "Retrieved workflow: {} for user: {}",
        workflow_id, user_context.user_id
    );

    Ok(Json(workflow))
}

/// PUT /workflows/{workflow_id} - Update workflow
pub async fn update_workflow(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(workflow_id): Path<String>,
    Json(payload): Json<UpdateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsUpdate) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:update required",
        ));
    }

    // Validate input
    payload.validate().map_err(|e| {
        ApiError::validation("workflow_update", format!("Invalid update request: {}", e))
    })?;

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Check workflow exists and ownership
    let existing_workflow = workflow_service
        .get_workflow(&workflow_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Workflow not found"))?;

    if !user_context.is_admin() && existing_workflow.created_by != user_context.user_id {
        return Err(ApiError::authorization(
            "Access denied: workflow belongs to another user",
        ));
    }

    info!(
        "Updating workflow: {} for user: {}",
        workflow_id, user_context.user_id
    );

    // Parse workflow definition if provided
    let parsed_workflow = if let Some(ref definition) = payload.definition {
        Some(
            state
                .intent_parser
                .parse_workflow_definition(definition)
                .await?,
        )
    } else {
        None
    };

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Update workflow
    let workflow = workflow_service
        .update_workflow(
            &workflow_id,
            payload.title.as_deref(),
            payload.description.as_deref(),
            payload.definition.as_deref(),
            parsed_workflow
                .as_ref()
                .map(|p| serde_json::to_value(p).unwrap_or_default())
                .as_ref(),
            payload.triggers.as_ref().map(|v| v.as_slice()),
            payload.input_schema.as_ref(),
            payload.output_schema.as_ref(),
            payload.config.as_ref(),
            payload.tags.as_ref().map(|v| v.as_slice()),
            payload.is_active,
        )
        .await?;

    info!("Workflow updated successfully: {}", workflow_id);

    Ok(Json(workflow))
}

/// DELETE /workflows/{workflow_id} - Delete workflow
pub async fn delete_workflow(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(workflow_id): Path<String>,
) -> Result<StatusCode> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsDelete) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:delete required",
        ));
    }

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Check workflow exists and ownership
    let existing_workflow = workflow_service
        .get_workflow(&workflow_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Workflow not found"))?;

    if !user_context.is_admin() && existing_workflow.created_by != user_context.user_id {
        return Err(ApiError::authorization(
            "Access denied: workflow belongs to another user",
        ));
    }

    info!(
        "Deleting workflow: {} for user: {}",
        workflow_id, user_context.user_id
    );

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Delete workflow
    workflow_service.delete_workflow(&workflow_id).await?;

    info!("Workflow deleted successfully: {}", workflow_id);

    Ok(StatusCode::NO_CONTENT)
}

/// POST /workflows/{workflow_id}/execute - Execute workflow
pub async fn execute_workflow(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(workflow_id): Path<String>,
    Json(payload): Json<ExecuteWorkflowRequest>,
) -> Result<Json<WorkflowExecutionResponse>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsCreate) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:create required",
        ));
    }

    // Validate input
    payload.validate().map_err(|e| {
        ApiError::validation("execution", format!("Invalid execution request: {}", e))
    })?;

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Check workflow exists and has access
    let workflow = workflow_service
        .get_workflow(&workflow_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Workflow not found"))?;

    if !workflow.is_active {
        return Err(ApiError::bad_request("Workflow is not active"));
    }

    // Check ownership unless admin
    if !user_context.is_admin() && workflow.created_by != user_context.user_id {
        return Err(ApiError::authorization(
            "Access denied: workflow belongs to another user",
        ));
    }

    info!(
        "Executing workflow: {} for user: {}",
        workflow_id, user_context.user_id
    );

    // Execute workflow
    let execution_id = Uuid::new_v4().to_string();
    // Get workflow orchestrator
    let workflow_orchestrator = state
        .workflow_orchestrator
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow_orchestrator"))?;

    let execution = workflow_orchestrator
        .execute_workflow(
            &execution_id,
            &workflow_id,
            &payload.input,
            payload.context.as_ref(),
            payload.priority.unwrap_or(5),
            payload.callback_url.as_deref(),
            payload.timeout_seconds.unwrap_or(300),
            &user_context.user_id,
        )
        .await?;

    // Record metrics
    state.metrics.record_workflow_executed(
        &workflow_id,
        &user_context.user_id,
        &user_context.subscription_tier.to_string(),
        "automation", // Default workflow type since WorkflowResponse doesn't have workflow_type field
    );

    info!("Workflow execution started: {}", execution_id);

    Ok(Json(execution))
}

/// GET /workflows/{workflow_id}/executions - List workflow executions
pub async fn list_workflow_executions(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(workflow_id): Path<String>,
    Query(query): Query<ListExecutionsQuery>,
) -> Result<Json<PaginatedResponse<WorkflowExecutionResponse>>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsRead) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:read required",
        ));
    }

    // Validate query parameters
    query
        .validate()
        .map_err(|e| ApiError::validation("query", format!("Invalid query parameters: {}", e)))?;

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Check workflow exists and ownership
    let workflow = workflow_service
        .get_workflow(&workflow_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Workflow not found"))?;

    if !user_context.is_admin() && workflow.created_by != user_context.user_id {
        return Err(ApiError::authorization(
            "Access denied: workflow belongs to another user",
        ));
    }

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    // Get workflow orchestrator
    let workflow_orchestrator = state
        .workflow_orchestrator
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow_orchestrator"))?;

    // List executions
    let (executions, total) = workflow_orchestrator
        .list_workflow_executions(
            &workflow_id,
            query.status.as_ref(),
            query.started_after.as_ref(),
            query.started_before.as_ref(),
            query.sort_by.as_deref(),
            query.sort_order.as_ref().map(|o| match o {
                SortOrder::Asc => "asc",
                SortOrder::Desc => "desc",
            }),
            limit,
            offset,
        )
        .await?;

    Ok(Json(PaginatedResponse {
        items: executions,
        total,
        limit,
        offset,
        has_next: offset + limit < total as u32,
    }))
}

/// GET /executions/{execution_id} - Get execution by ID
pub async fn get_execution(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(execution_id): Path<String>,
) -> Result<Json<WorkflowExecutionResponse>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsRead) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:read required",
        ));
    }

    // Get execution
    // Get workflow orchestrator
    let workflow_orchestrator = state
        .workflow_orchestrator
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow_orchestrator"))?;

    let execution = workflow_orchestrator
        .get_execution(&execution_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Execution not found"))?;

    // Check workflow ownership unless admin
    if !user_context.is_admin() {
        // Get workflow service
        let workflow_service = state
            .workflow_service
            .as_ref()
            .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

        // Get workflow info for logging
        let workflow = workflow_service
            .get_workflow(&execution.workflow_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Associated workflow not found"))?;

        if workflow.created_by != user_context.user_id {
            return Err(ApiError::authorization(
                "Access denied: execution belongs to another user",
            ));
        }
    }

    Ok(Json(execution))
}

/// POST /executions/{execution_id}/cancel - Cancel execution
pub async fn cancel_execution(
    State(state): State<AppState>,
    Extension(user_context): Extension<UserContext>,
    Path(execution_id): Path<String>,
) -> Result<Json<WorkflowExecutionResponse>> {
    // Check permissions
    if !user_context.has_permission(&Permission::WorkflowsUpdate) {
        return Err(ApiError::authorization(
            "Permission denied: workflows:update required",
        ));
    }

    // Get workflow orchestrator
    let workflow_orchestrator = state
        .workflow_orchestrator
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow_orchestrator"))?;

    // Get execution and check ownership
    let execution = workflow_orchestrator
        .get_execution(&execution_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Execution not found"))?;

    // Get workflow service
    let workflow_service = state
        .workflow_service
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow"))?;

    // Check workflow ownership unless admin
    if !user_context.is_admin() {
        let workflow = workflow_service
            .get_workflow(&execution.workflow_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Associated workflow not found"))?;

        if workflow.created_by != user_context.user_id {
            return Err(ApiError::authorization(
                "Access denied: execution belongs to another user's workflow",
            ));
        }
    }

    info!(
        "Cancelling execution: {} for user: {}",
        execution_id, user_context.user_id
    );

    // Cancel execution
    // Get workflow orchestrator
    let workflow_orchestrator = state
        .workflow_orchestrator
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("workflow_orchestrator"))?;

    let cancelled_execution = workflow_orchestrator
        .cancel_execution(&execution_id)
        .await?;

    info!("Execution cancelled successfully: {}", execution_id);

    Ok(Json(cancelled_execution))
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_workflow_validation() {
        let valid_request = CreateWorkflowRequest {
            title: "Test Workflow".to_string(),
            description: Some("Test description".to_string()),
            definition: "Create a blog post about AI".to_string(),
            triggers: vec![],
            input_schema: None,
            output_schema: None,
            config: None,
            tags: None,
            is_active: true,
        };
        assert!(valid_request.validate().is_ok());

        let empty_title = CreateWorkflowRequest {
            title: "".to_string(),
            definition: "Create a blog post about AI".to_string(),
            triggers: vec![],
            input_schema: None,
            output_schema: None,
            config: None,
            tags: None,
            is_active: true,
            description: None,
        };
        assert!(empty_title.validate().is_err());

        let short_definition = CreateWorkflowRequest {
            title: "Test".to_string(),
            definition: "short".to_string(),
            triggers: vec![],
            input_schema: None,
            output_schema: None,
            config: None,
            tags: None,
            is_active: true,
            description: None,
        };
        assert!(short_definition.validate().is_err());
    }

    #[test]
    fn test_execute_workflow_validation() {
        let valid_request = ExecuteWorkflowRequest {
            input: serde_json::json!({"key": "value"}),
            context: None,
            priority: Some(5),
            callback_url: Some("https://example.com/callback".to_string()),
            timeout_seconds: Some(300),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_priority = ExecuteWorkflowRequest {
            input: serde_json::json!({"key": "value"}),
            context: None,
            priority: Some(11),
            callback_url: None,
            timeout_seconds: None,
        };
        assert!(invalid_priority.validate().is_err());

        let invalid_url = ExecuteWorkflowRequest {
            input: serde_json::json!({"key": "value"}),
            context: None,
            priority: None,
            callback_url: Some("not-a-url".to_string()),
            timeout_seconds: None,
        };
        assert!(invalid_url.validate().is_err());
    }

    #[test]
    fn test_list_workflows_query_validation() {
        let valid_query = ListWorkflowsQuery {
            limit: Some(50),
            offset: Some(0),
            status: None,
            tags: None,
            search: None,
            created_by: None,
            sort_by: None,
            sort_order: None,
        };
        assert!(valid_query.validate().is_ok());

        let invalid_limit = ListWorkflowsQuery {
            limit: Some(101),
            offset: None,
            status: None,
            tags: None,
            search: None,
            created_by: None,
            sort_by: None,
            sort_order: None,
        };
        assert!(invalid_limit.validate().is_err());
    }
}
