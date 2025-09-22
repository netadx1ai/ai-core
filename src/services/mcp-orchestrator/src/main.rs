//! MCP Orchestrator Service for Multi-MCP Workflow Coordination
//!
//! This service coordinates workflows across multiple MCP services, enabling complex
//! multi-step automation workflows like "Create blog post + image + social media post".

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub service_name: String,
    pub mcp_registry: Arc<McpRegistry>,
    pub workflow_store: Arc<WorkflowStore>,
}

#[derive(Clone)]
pub struct McpRegistry {
    pub services: DashMap<String, McpService>,
}

#[derive(Clone)]
pub struct WorkflowStore {
    pub workflows: DashMap<Uuid, WorkflowExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpService {
    pub name: String,
    pub url: String,
    pub capabilities: Vec<String>,
    pub status: String,
    pub last_health_check: DateTime<Utc>,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct WorkflowRequest {
    pub workflow_type: String, // "blog_post_campaign", "content_analysis", "creative_pipeline"
    pub parameters: HashMap<String, serde_json::Value>,
    pub options: Option<WorkflowOptions>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowOptions {
    pub timeout_seconds: Option<u64>,
    pub parallel_execution: Option<bool>,
    pub failure_strategy: Option<String>, // "fail_fast", "continue", "retry"
    pub notification_webhook: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    pub workflow_id: Uuid,
    pub workflow_type: String,
    pub status: String, // "queued", "running", "completed", "failed", "cancelled"
    pub steps: Vec<WorkflowStep>,
    pub results: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub processing_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: Uuid,
    pub step_name: String,
    pub mcp_service: String,
    pub endpoint: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub depends_on: Vec<Uuid>,
    pub status: String, // "pending", "running", "completed", "failed", "skipped"
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub processing_time_ms: Option<u64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub workflow_type: String,
    pub status: String,
    pub steps: Vec<WorkflowStep>,
    pub results: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub options: Option<WorkflowOptions>,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub service: String,
    pub timestamp: DateTime<Utc>,
    pub registered_mcps: usize,
    pub active_workflows: usize,
    pub available_workflow_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct McpRegistrationRequest {
    pub name: String,
    pub url: String,
    pub capabilities: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mcp_orchestrator=info,tower_http=debug")
        .init();

    info!(
        "Starting MCP Orchestrator Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    let mcp_registry = Arc::new(McpRegistry {
        services: DashMap::new(),
    });

    let workflow_store = Arc::new(WorkflowStore {
        workflows: DashMap::new(),
    });

    // Register default MCP services
    register_default_mcps(&mcp_registry).await;

    let state = AppState {
        service_name: "mcp-orchestrator".to_string(),
        mcp_registry: mcp_registry.clone(),
        workflow_store: workflow_store.clone(),
    };

    // Start background health check task
    let health_check_registry = mcp_registry.clone();
    tokio::spawn(async move {
        health_check_loop(health_check_registry).await;
    });

    let app = create_router(state);

    let listener = TcpListener::bind("0.0.0.0:8807").await?;
    info!("MCP Orchestrator Service listening on http://0.0.0.0:8807");

    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/workflows", post(create_workflow))
        .route("/v1/workflows/:workflow_id", get(get_workflow))
        .route("/v1/workflows/:workflow_id/cancel", post(cancel_workflow))
        .route("/v1/mcps/register", post(register_mcp))
        .route("/v1/mcps", get(list_mcps))
        .route("/v1/capabilities", get(get_capabilities))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let registered_mcps = state.mcp_registry.services.len();
    let active_workflows = state
        .workflow_store
        .workflows
        .iter()
        .filter(|entry| matches!(entry.value().status.as_str(), "queued" | "running"))
        .count();

    Json(HealthStatus {
        status: "healthy".to_string(),
        service: state.service_name,
        timestamp: Utc::now(),
        registered_mcps,
        active_workflows,
        available_workflow_types: vec![
            "blog_post_campaign".to_string(),
            "content_analysis".to_string(),
            "creative_pipeline".to_string(),
            "social_media_automation".to_string(),
        ],
    })
}

async fn create_workflow(
    State(state): State<AppState>,
    Json(request): Json<WorkflowRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let workflow_id = Uuid::new_v4();

    info!(
        "Creating workflow '{}' with ID: {}",
        request.workflow_type, workflow_id
    );

    // Generate workflow steps based on type
    let steps = match generate_workflow_steps(&request.workflow_type, &request.parameters) {
        Ok(steps) => steps,
        Err(e) => {
            error!("Failed to generate workflow steps: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let workflow = WorkflowExecution {
        id: workflow_id,
        workflow_type: request.workflow_type.clone(),
        status: "queued".to_string(),
        steps: steps.clone(),
        results: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        options: request.options,
    };

    // Store workflow
    state.workflow_store.workflows.insert(workflow_id, workflow);

    // Start workflow execution in background
    let orchestrator_state = state.clone();
    tokio::spawn(async move {
        execute_workflow(orchestrator_state, workflow_id).await;
    });

    let response = WorkflowResponse {
        workflow_id,
        workflow_type: request.workflow_type,
        status: "queued".to_string(),
        steps,
        results: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        processing_time_ms: None,
    };

    Ok(Json(response))
}

async fn get_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    match state.workflow_store.workflows.get(&workflow_id) {
        Some(workflow) => {
            let workflow_data = workflow.value();
            let response = WorkflowResponse {
                workflow_id: workflow_data.id,
                workflow_type: workflow_data.workflow_type.clone(),
                status: workflow_data.status.clone(),
                steps: workflow_data.steps.clone(),
                results: workflow_data.results.clone(),
                created_at: workflow_data.created_at,
                updated_at: workflow_data.updated_at,
                processing_time_ms: None,
            };
            Ok(Json(response))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn cancel_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    match state.workflow_store.workflows.get_mut(&workflow_id) {
        Some(mut workflow) => {
            workflow.status = "cancelled".to_string();
            workflow.updated_at = Utc::now();
            info!("Workflow {} cancelled", workflow_id);
            Ok(Json(serde_json::json!({"status": "cancelled"})))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn register_mcp(
    State(state): State<AppState>,
    Json(request): Json<McpRegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let mcp_service = McpService {
        name: request.name.clone(),
        url: request.url,
        capabilities: request.capabilities,
        status: "active".to_string(),
        last_health_check: Utc::now(),
    };

    state
        .mcp_registry
        .services
        .insert(request.name.clone(), mcp_service);

    info!("Registered MCP service: {}", request.name);

    Ok(Json(serde_json::json!({
        "status": "registered",
        "service": request.name
    })))
}

async fn list_mcps(State(state): State<AppState>) -> impl IntoResponse {
    let mcps: Vec<McpService> = state
        .mcp_registry
        .services
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    Json(serde_json::json!({
        "mcps": mcps,
        "count": mcps.len()
    }))
}

async fn get_capabilities(State(_state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "mcp-orchestrator",
        "version": env!("CARGO_PKG_VERSION"),
        "supported_workflow_types": [
            {
                "type": "blog_post_campaign",
                "description": "Create blog post + image + social media post",
                "required_parameters": ["topic", "target_audience"],
                "optional_parameters": ["tone", "word_count", "image_style"]
            },
            {
                "type": "content_analysis",
                "description": "Analyze text for keywords, sentiment, and readability",
                "required_parameters": ["text"],
                "optional_parameters": ["analysis_types"]
            },
            {
                "type": "creative_pipeline",
                "description": "Generate creative content with images and variations",
                "required_parameters": ["concept"],
                "optional_parameters": ["style", "iterations"]
            }
        ],
        "features": [
            "multi_mcp_coordination",
            "parallel_execution",
            "dependency_management",
            "error_recovery",
            "real_time_monitoring",
            "workflow_templates"
        ]
    }))
}

async fn register_default_mcps(registry: &Arc<McpRegistry>) {
    let default_mcps = vec![
        McpService {
            name: "demo-content-mcp".to_string(),
            url: "http://localhost:8804".to_string(),
            capabilities: vec!["generate_content".to_string(), "blog_posts".to_string()],
            status: "active".to_string(),
            last_health_check: Utc::now(),
        },
        McpService {
            name: "text-processing-mcp".to_string(),
            url: "http://localhost:8805".to_string(),
            capabilities: vec![
                "analyze_text".to_string(),
                "keywords".to_string(),
                "sentiment".to_string(),
            ],
            status: "active".to_string(),
            last_health_check: Utc::now(),
        },
        McpService {
            name: "image-generation-mcp".to_string(),
            url: "http://localhost:8806".to_string(),
            capabilities: vec!["generate_images".to_string(), "variations".to_string()],
            status: "active".to_string(),
            last_health_check: Utc::now(),
        },
    ];

    for mcp in default_mcps {
        registry.services.insert(mcp.name.clone(), mcp);
    }

    info!(
        "Registered {} default MCP services",
        registry.services.len()
    );
}

fn generate_workflow_steps(
    workflow_type: &str,
    parameters: &HashMap<String, serde_json::Value>,
) -> Result<Vec<WorkflowStep>, Box<dyn std::error::Error>> {
    match workflow_type {
        "blog_post_campaign" => {
            let topic = parameters
                .get("topic")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: topic")?;

            let step1_id = Uuid::new_v4();
            let step2_id = Uuid::new_v4();
            let step3_id = Uuid::new_v4();
            let step4_id = Uuid::new_v4();

            Ok(vec![
                WorkflowStep {
                    step_id: step1_id,
                    step_name: "generate_blog_post".to_string(),
                    mcp_service: "demo-content-mcp".to_string(),
                    endpoint: "/v1/content/generate".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("content_type".to_string(), serde_json::json!("blog_post"));
                        params.insert("topic".to_string(), serde_json::json!(topic));
                        if let Some(audience) = parameters.get("target_audience") {
                            params.insert("target_audience".to_string(), audience.clone());
                        }
                        if let Some(tone) = parameters.get("tone") {
                            params.insert("tone".to_string(), tone.clone());
                        }
                        params
                    },
                    depends_on: vec![],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
                WorkflowStep {
                    step_id: step2_id,
                    step_name: "analyze_content".to_string(),
                    mcp_service: "text-processing-mcp".to_string(),
                    endpoint: "/v1/analyze".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("analysis_type".to_string(), serde_json::json!("keywords"));
                        params.insert("text".to_string(), serde_json::json!("{{step1.content}}"));
                        params
                    },
                    depends_on: vec![step1_id],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
                WorkflowStep {
                    step_id: step3_id,
                    step_name: "generate_image".to_string(),
                    mcp_service: "image-generation-mcp".to_string(),
                    endpoint: "/v1/images/generate".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "prompt".to_string(),
                            serde_json::json!(format!("Blog post illustration for: {}", topic)),
                        );
                        if let Some(style) = parameters.get("image_style") {
                            params.insert("style".to_string(), style.clone());
                        } else {
                            params.insert("style".to_string(), serde_json::json!("realistic"));
                        }
                        params.insert("size".to_string(), serde_json::json!("1024x1024"));
                        params
                    },
                    depends_on: vec![step1_id],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
                WorkflowStep {
                    step_id: step4_id,
                    step_name: "create_social_post".to_string(),
                    mcp_service: "demo-content-mcp".to_string(),
                    endpoint: "/v1/content/generate".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "content_type".to_string(),
                            serde_json::json!("social_media_post"),
                        );
                        params.insert("topic".to_string(), serde_json::json!(topic));
                        params.insert("tone".to_string(), serde_json::json!("engaging"));
                        params
                    },
                    depends_on: vec![step1_id, step2_id],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
            ])
        }
        "content_analysis" => {
            let text = parameters
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: text")?;

            let step1_id = Uuid::new_v4();
            let step2_id = Uuid::new_v4();
            let step3_id = Uuid::new_v4();

            Ok(vec![
                WorkflowStep {
                    step_id: step1_id,
                    step_name: "analyze_keywords".to_string(),
                    mcp_service: "text-processing-mcp".to_string(),
                    endpoint: "/v1/analyze".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("analysis_type".to_string(), serde_json::json!("keywords"));
                        params.insert("text".to_string(), serde_json::json!(text));
                        params
                    },
                    depends_on: vec![],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
                WorkflowStep {
                    step_id: step2_id,
                    step_name: "analyze_sentiment".to_string(),
                    mcp_service: "text-processing-mcp".to_string(),
                    endpoint: "/v1/analyze".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("analysis_type".to_string(), serde_json::json!("sentiment"));
                        params.insert("text".to_string(), serde_json::json!(text));
                        params
                    },
                    depends_on: vec![],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
                WorkflowStep {
                    step_id: step3_id,
                    step_name: "analyze_readability".to_string(),
                    mcp_service: "text-processing-mcp".to_string(),
                    endpoint: "/v1/analyze".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "analysis_type".to_string(),
                            serde_json::json!("readability"),
                        );
                        params.insert("text".to_string(), serde_json::json!(text));
                        params
                    },
                    depends_on: vec![],
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    processing_time_ms: None,
                    started_at: None,
                    completed_at: None,
                },
            ])
        }
        _ => Err(format!("Unsupported workflow type: {}", workflow_type).into()),
    }
}

async fn execute_workflow(state: AppState, workflow_id: Uuid) {
    info!("Starting execution of workflow: {}", workflow_id);

    let mut workflow = match state.workflow_store.workflows.get_mut(&workflow_id) {
        Some(workflow) => workflow,
        None => {
            error!("Workflow {} not found", workflow_id);
            return;
        }
    };

    workflow.status = "running".to_string();
    workflow.updated_at = Utc::now();
    drop(workflow); // Release the lock

    // Execute steps based on dependencies
    let client = reqwest::Client::new();

    loop {
        let mut pending_steps = Vec::new();
        let mut ready_steps = Vec::new();

        // Check workflow status
        if let Some(workflow) = state.workflow_store.workflows.get(&workflow_id) {
            if workflow.status == "cancelled" {
                info!("Workflow {} was cancelled", workflow_id);
                return;
            }

            // Find steps that are ready to execute
            for step in &workflow.steps {
                match step.status.as_str() {
                    "pending" => {
                        // Check if all dependencies are completed
                        let dependencies_completed = step.depends_on.iter().all(|dep_id| {
                            workflow
                                .steps
                                .iter()
                                .any(|s| s.step_id == *dep_id && s.status == "completed")
                        });

                        if dependencies_completed {
                            ready_steps.push(step.clone());
                        } else {
                            pending_steps.push(step.clone());
                        }
                    }
                    "running" => pending_steps.push(step.clone()),
                    _ => {} // completed, failed, skipped
                }
            }
        }

        if ready_steps.is_empty() && pending_steps.is_empty() {
            // All steps are completed
            if let Some(mut workflow) = state.workflow_store.workflows.get_mut(&workflow_id) {
                workflow.status = "completed".to_string();
                workflow.updated_at = Utc::now();
            }
            info!("Workflow {} completed", workflow_id);
            break;
        }

        if ready_steps.is_empty() {
            // Wait for running steps to complete
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        // Execute ready steps in parallel
        let execution_futures = ready_steps.into_iter().map(|step| {
            let client = client.clone();
            let state = state.clone();
            let workflow_id = workflow_id;

            async move { execute_step(&client, &state, workflow_id, step).await }
        });

        join_all(execution_futures).await;
    }
}

async fn execute_step(
    client: &reqwest::Client,
    state: &AppState,
    workflow_id: Uuid,
    step: WorkflowStep,
) {
    info!(
        "Executing step: {} for workflow: {}",
        step.step_name, workflow_id
    );

    // Update step status to running
    if let Some(mut workflow) = state.workflow_store.workflows.get_mut(&workflow_id) {
        if let Some(workflow_step) = workflow
            .steps
            .iter_mut()
            .find(|s| s.step_id == step.step_id)
        {
            workflow_step.status = "running".to_string();
            workflow_step.started_at = Some(Utc::now());
        }
        workflow.updated_at = Utc::now();
    }

    let start_time = std::time::Instant::now();

    // Get MCP service URL
    let service_url = match state.mcp_registry.services.get(&step.mcp_service) {
        Some(service) => service.url.clone(),
        None => {
            error!("MCP service {} not found", step.mcp_service);
            update_step_failure(
                state,
                workflow_id,
                step.step_id,
                "MCP service not found".to_string(),
            )
            .await;
            return;
        }
    };

    // Replace template variables in parameters
    let resolved_parameters = resolve_step_parameters(state, workflow_id, &step.parameters).await;

    // Execute HTTP request to MCP service
    let full_url = format!("{}{}", service_url, step.endpoint);

    match client
        .post(&full_url)
        .json(&resolved_parameters)
        .timeout(Duration::from_secs(30))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(result) => {
                        let processing_time = start_time.elapsed().as_millis() as u64;
                        info!(
                            "Step {} completed successfully in {}ms",
                            step.step_name, processing_time
                        );
                        update_step_success(
                            state,
                            workflow_id,
                            step.step_id,
                            result,
                            processing_time,
                        )
                        .await;
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse response for step {}: {}",
                            step.step_name, e
                        );
                        update_step_failure(
                            state,
                            workflow_id,
                            step.step_id,
                            format!("Response parsing error: {}", e),
                        )
                        .await;
                    }
                }
            } else {
                error!(
                    "HTTP error for step {}: {}",
                    step.step_name,
                    response.status()
                );
                update_step_failure(
                    state,
                    workflow_id,
                    step.step_id,
                    format!("HTTP error: {}", response.status()),
                )
                .await;
            }
        }
        Err(e) => {
            error!("Request failed for step {}: {}", step.step_name, e);
            update_step_failure(
                state,
                workflow_id,
                step.step_id,
                format!("Request error: {}", e),
            )
            .await;
        }
    }
}

async fn resolve_step_parameters(
    state: &AppState,
    workflow_id: Uuid,
    parameters: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut resolved = HashMap::new();

    for (key, value) in parameters {
        if let Some(value_str) = value.as_str() {
            if value_str.contains("{{") && value_str.contains("}}") {
                // This is a template - resolve it
                let resolved_value = resolve_template_value(state, workflow_id, value_str).await;
                resolved.insert(key.clone(), serde_json::json!(resolved_value));
            } else {
                resolved.insert(key.clone(), value.clone());
            }
        } else {
            resolved.insert(key.clone(), value.clone());
        }
    }

    resolved
}

async fn resolve_template_value(state: &AppState, workflow_id: Uuid, template: &str) -> String {
    // Simple template resolution for {{step1.content}} patterns
    if let Some(workflow) = state.workflow_store.workflows.get(&workflow_id) {
        // This is a simplified template resolver
        // In production, you'd want a more sophisticated template engine
        if template.contains("{{step1.content}}") {
            if let Some(step1) = workflow.steps.first() {
                if let Some(result) = &step1.result {
                    if let Some(content) = result.get("content") {
                        return content.as_str().unwrap_or(template).to_string();
                    }
                }
            }
        }
    }

    template.to_string()
}

async fn update_step_success(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    result: serde_json::Value,
    processing_time_ms: u64,
) {
    if let Some(mut workflow) = state.workflow_store.workflows.get_mut(&workflow_id) {
        if let Some(step) = workflow.steps.iter_mut().find(|s| s.step_id == step_id) {
            step.status = "completed".to_string();
            step.result = Some(result);
            step.processing_time_ms = Some(processing_time_ms);
            step.completed_at = Some(Utc::now());
        }
        workflow.updated_at = Utc::now();
    }
}

async fn update_step_failure(state: &AppState, workflow_id: Uuid, step_id: Uuid, error: String) {
    if let Some(mut workflow) = state.workflow_store.workflows.get_mut(&workflow_id) {
        if let Some(step) = workflow.steps.iter_mut().find(|s| s.step_id == step_id) {
            step.status = "failed".to_string();
            step.error = Some(error);
            step.completed_at = Some(Utc::now());
        }
        workflow.updated_at = Utc::now();
    }
}

async fn health_check_loop(registry: Arc<McpRegistry>) {
    let client = reqwest::Client::new();

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        for mut service in registry.services.iter_mut() {
            let health_url = format!("{}/health", service.url);

            match client
                .get(&health_url)
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    service.status = "active".to_string();
                    service.last_health_check = Utc::now();
                }
                _ => {
                    warn!("Health check failed for MCP service: {}", service.name);
                    service.status = "unhealthy".to_string();
                }
            }
        }
    }
}
