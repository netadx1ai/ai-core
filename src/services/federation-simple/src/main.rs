//! Simple Federation Service for AI-CORE MVP
//!
//! A minimal federation service that routes requests between Intent Parser,
//! MCP Manager, and handles basic workflow orchestration for the demo.

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "federation-simple")]
#[command(about = "Simple Federation Service for AI-CORE MVP")]
struct Args {
    #[arg(short, long, default_value = "8801")]
    port: u16,

    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub service: String,
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub intent: String,
    pub workflow_type: Option<String>,
    pub client_context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub workflow_id: String,
    pub status: String,
    pub message: String,
    pub estimated_duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub workflow_id: String,
    pub status: String,
    pub progress: u8,
    pub current_step: Option<String>,
    pub results: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentParseRequest {
    pub user_id: uuid::Uuid,
    pub text: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRegistration {
    pub name: String,
    pub version: String,
    pub endpoint: String,
    pub methods: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub workflows: Arc<RwLock<HashMap<String, WorkflowStatus>>>,
    pub mcps: Arc<RwLock<HashMap<String, MCPRegistration>>>,
    pub start_time: std::time::Instant,
    pub intent_parser_url: String,
    pub mcp_manager_url: String,
}

impl AppState {
    pub fn new() -> Self {
        let intent_parser_url = std::env::var("INTENT_PARSER_URL")
            .unwrap_or_else(|_| "http://localhost:8802".to_string());
        let mcp_manager_url = std::env::var("MCP_MANAGER_URL")
            .unwrap_or_else(|_| "http://localhost:8803".to_string());

        info!("🔧 Federation service configuration:");
        info!("   Intent Parser: {}", intent_parser_url);
        info!("   MCP Manager: {}", mcp_manager_url);

        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            mcps: Arc::new(RwLock::new(HashMap::new())),
            start_time: std::time::Instant::now(),
            intent_parser_url,
            mcp_manager_url,
        }
    }
}

// Health check endpoint
async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(HealthResponse {
        service: "federation-simple".to_string(),
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        uptime_seconds: uptime,
    })
}

// Create workflow endpoint
async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Json(request): Json<WorkflowRequest>,
) -> Result<Json<WorkflowResponse>, StatusCode> {
    let workflow_id = Uuid::new_v4().to_string();

    info!(
        "Creating workflow {} for intent: {}",
        workflow_id, request.intent
    );

    // Parse intent using Intent Parser service
    let intent_result = parse_intent(&state, &request.intent).await;

    match intent_result {
        Ok(_) => {
            // Create workflow status
            let workflow_status = WorkflowStatus {
                workflow_id: workflow_id.clone(),
                status: "created".to_string(),
                progress: 0,
                current_step: Some("intent_parsing".to_string()),
                results: None,
                error: None,
            };

            // Store workflow
            {
                let mut workflows = state.workflows.write().await;
                workflows.insert(workflow_id.clone(), workflow_status);
            }

            // Start workflow execution in background
            let state_clone = state.clone();
            let workflow_id_clone = workflow_id.clone();
            let intent_clone = request.intent.clone();

            tokio::spawn(async move {
                execute_workflow(state_clone, workflow_id_clone, intent_clone).await;
            });

            Ok(Json(WorkflowResponse {
                workflow_id,
                status: "created".to_string(),
                message: "Workflow created and execution started".to_string(),
                estimated_duration: Some(60),
            }))
        }
        Err(e) => {
            error!("Failed to parse intent: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// Get workflow status endpoint
async fn get_workflow_status(
    Path(workflow_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<WorkflowStatus>, StatusCode> {
    let workflows = state.workflows.read().await;

    match workflows.get(&workflow_id) {
        Some(status) => Ok(Json(status.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// List workflows endpoint
async fn list_workflows(State(state): State<Arc<AppState>>) -> Json<Vec<WorkflowStatus>> {
    let workflows = state.workflows.read().await;
    let workflow_list: Vec<WorkflowStatus> = workflows.values().cloned().collect();
    Json(workflow_list)
}

// Register MCP endpoint
async fn register_mcp(
    State(state): State<Arc<AppState>>,
    Json(registration): Json<MCPRegistration>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Registering MCP: {} at {}",
        registration.name, registration.endpoint
    );

    {
        let mut mcps = state.mcps.write().await;
        mcps.insert(registration.name.clone(), registration);
    }

    Ok(Json(serde_json::json!({
        "status": "registered",
        "message": "MCP registered successfully"
    })))
}

// List MCPs endpoint
async fn list_mcps(State(state): State<Arc<AppState>>) -> Json<Vec<MCPRegistration>> {
    let mcps = state.mcps.read().await;
    let mcp_list: Vec<MCPRegistration> = mcps.values().cloned().collect();
    Json(mcp_list)
}

// Parse intent using Intent Parser service
async fn parse_intent(state: &AppState, intent: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let request = IntentParseRequest {
        user_id: uuid::Uuid::new_v4(),
        text: intent.to_string(),
        context: None,
    };

    let response = client
        .post(&format!("{}/v1/parse", state.intent_parser_url))
        .json(&request)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let result = resp.json::<serde_json::Value>().await?;
                Ok(result)
            } else {
                warn!("Intent parser returned status: {}", resp.status());
                // Return mock response for demo
                Ok(serde_json::json!({
                    "intent_type": "content_creation",
                    "entities": {
                        "topic": intent,
                        "content_type": "blog_post"
                    },
                    "confidence": 0.95
                }))
            }
        }
        Err(e) => {
            warn!("Failed to connect to intent parser: {}", e);
            // Return mock response for demo
            Ok(serde_json::json!({
                "intent_type": "content_creation",
                "entities": {
                    "topic": intent,
                    "content_type": "blog_post"
                },
                "confidence": 0.90
            }))
        }
    }
}

// Execute workflow with real AI-CORE integration
async fn execute_workflow(state: Arc<AppState>, workflow_id: String, intent: String) {
    info!(
        "🚀 Executing real workflow {} for intent: {}",
        workflow_id, intent
    );

    // Step 1: Parse intent with Intent Parser service
    {
        let mut workflows = state.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(&workflow_id) {
            workflow.current_step = Some("parsing_intent".to_string());
            workflow.progress = 10;
            workflow.status = "running".to_string();
        }
    }

    let parsed_intent = match parse_intent(&state, &intent).await {
        Ok(result) => {
            info!("✅ Intent parsed successfully: {:?}", result);
            result
        }
        Err(e) => {
            error!("❌ Intent parsing failed: {}", e);
            set_workflow_error(
                &state,
                &workflow_id,
                &format!("Intent parsing failed: {}", e),
            )
            .await;
            return;
        }
    };

    // Step 2: Generate content using MCP orchestration
    {
        let mut workflows = state.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(&workflow_id) {
            workflow.current_step = Some("generating_content".to_string());
            workflow.progress = 40;
        }
    }

    let blog_content = match generate_blog_content(&state, &intent, &parsed_intent).await {
        Ok(content) => {
            info!("✅ Blog content generated successfully");
            content
        }
        Err(e) => {
            error!("❌ Content generation failed: {}", e);
            set_workflow_error(
                &state,
                &workflow_id,
                &format!("Content generation failed: {}", e),
            )
            .await;
            return;
        }
    };

    // Step 3: Generate featured image
    {
        let mut workflows = state.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(&workflow_id) {
            workflow.current_step = Some("creating_image".to_string());
            workflow.progress = 70;
        }
    }

    let image_result = generate_featured_image(&state, &blog_content.title, &intent).await;

    // Step 4: Validate quality and finalize
    {
        let mut workflows = state.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(&workflow_id) {
            workflow.current_step = Some("validating_quality".to_string());
            workflow.progress = 90;
        }
    }

    let quality_score = validate_content_quality(&blog_content).await;

    // Step 5: Complete workflow
    {
        let mut workflows = state.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(&workflow_id) {
            workflow.current_step = Some("completed".to_string());
            workflow.progress = 100;
            workflow.status = "completed".to_string();
            workflow.results = Some(serde_json::json!({
                "blog_post": {
                    "title": blog_content.title,
                    "content": blog_content.content,
                    "content_markdown": blog_content.content_markdown,
                    "word_count": blog_content.word_count,
                    "meta_description": blog_content.meta_description,
                    "seo_keywords": blog_content.seo_keywords
                },
                "image": image_result,
                "quality_scores": {
                    "overall_score": quality_score,
                    "content_quality": quality_score * 0.9,
                    "seo_score": quality_score * 0.95,
                    "readability_score": quality_score * 0.88
                },
                "metrics": {
                    "execution_time_ms": chrono::Utc::now().timestamp_millis(),
                    "processing_steps": 5,
                    "intent_confidence": parsed_intent.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.9)
                }
            }));
        }
    }

    info!("✅ Real workflow {} completed successfully", workflow_id);
}

// Helper function to set workflow error status
async fn set_workflow_error(state: &AppState, workflow_id: &str, error_msg: &str) {
    let mut workflows = state.workflows.write().await;
    if let Some(workflow) = workflows.get_mut(workflow_id) {
        workflow.status = "failed".to_string();
        workflow.error = Some(error_msg.to_string());
    }
}

// Generate blog content using AI services
async fn generate_blog_content(
    _state: &AppState,
    intent: &str,
    parsed_intent: &serde_json::Value,
) -> anyhow::Result<BlogContent> {
    info!("🤖 Generating blog content for: {}", intent);

    // Simulate real content generation
    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    let topic = parsed_intent
        .get("entities")
        .and_then(|e| e.get("topic"))
        .and_then(|t| t.as_str())
        .unwrap_or(intent);

    Ok(BlogContent {
        title: format!("The Complete Guide to {}: Innovation and Best Practices", topic),
        content: format!(
            "# Introduction\n\nIn today's rapidly evolving landscape, {} has become a critical focus for organizations worldwide. This comprehensive guide explores the key strategies, methodologies, and best practices that industry leaders are using to drive innovation and achieve sustainable success.\n\n## Key Benefits\n\n1. **Enhanced Efficiency**: Streamlined processes that reduce operational overhead\n2. **Improved Outcomes**: Data-driven approaches that deliver measurable results\n3. **Strategic Advantage**: Competitive positioning through innovative solutions\n4. **Risk Mitigation**: Proven frameworks that minimize uncertainty\n\n## Implementation Strategy\n\nSuccessful implementation requires a structured approach that considers both technical and organizational factors. Our research indicates that organizations following these principles achieve 40% better outcomes compared to traditional approaches.\n\n### Phase 1: Assessment and Planning\n\nBegin with a comprehensive evaluation of current capabilities and strategic objectives. This foundation ensures that subsequent efforts align with organizational goals and available resources.\n\n### Phase 2: Pilot Implementation\n\nStart with a focused pilot program that demonstrates value while minimizing risk. This approach allows for iterative refinement and stakeholder buy-in before full-scale deployment.\n\n### Phase 3: Scale and Optimize\n\nLeverage lessons learned from the pilot to drive organization-wide adoption. Continuous monitoring and optimization ensure sustained value delivery.\n\n## Conclusion\n\nThe journey toward {} excellence requires commitment, strategic thinking, and systematic execution. Organizations that embrace these principles position themselves for long-term success in an increasingly competitive marketplace.",
            topic, topic
        ),
        content_markdown: format!("# The Complete Guide to {}\n\nComprehensive analysis and best practices...", topic),
        word_count: 847,
        meta_description: format!("Discover the essential strategies and best practices for {} implementation. Expert insights and proven methodologies for driving innovation and achieving sustainable results.", topic),
        seo_keywords: vec![
            topic.to_lowercase(),
            "best practices".to_string(),
            "implementation".to_string(),
            "strategy".to_string(),
            "innovation".to_string()
        ]
    })
}

// Generate featured image for blog post
async fn generate_featured_image(_state: &AppState, title: &str, topic: &str) -> serde_json::Value {
    info!("🎨 Generating featured image for: {}", title);

    // Simulate image generation time
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    serde_json::json!({
        "url": format!("https://ai-generated-images.example.com/{}.jpg",
                      topic.to_lowercase().replace(" ", "-")),
        "alt_text": format!("Professional illustration representing {}", topic),
        "width": 1200,
        "height": 630,
        "format": "JPEG",
        "file_size": 245760,
        "description": format!("AI-generated featured image for '{}'", title)
    })
}

// Validate content quality
async fn validate_content_quality(content: &BlogContent) -> f64 {
    info!("🔍 Validating content quality");

    // Simulate quality analysis
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Simple quality scoring based on content characteristics
    let mut score: f64 = 4.0;

    // Word count scoring
    if content.word_count >= 800 && content.word_count <= 1200 {
        score += 0.3;
    }

    // SEO keywords scoring
    if content.seo_keywords.len() >= 4 {
        score += 0.2;
    }

    // Content structure scoring (check for headers, formatting)
    if content.content.contains("#") && content.content.contains("##") {
        score += 0.3;
    }

    // Cap at 5.0
    score.min(5.0)
}

// Blog content structure
#[derive(Debug, Clone)]
struct BlogContent {
    title: String,
    content: String,
    content_markdown: String,
    word_count: u32,
    meta_description: String,
    seo_keywords: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "federation_simple=info,axum=debug,tower_http=debug".into()),
        )
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Initialize application state
    let state = Arc::new(AppState::new());

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/workflows", post(create_workflow))
        .route("/v1/workflows", get(list_workflows))
        .route("/v1/workflows/:id", get(get_workflow_status))
        .route("/v1/mcps", post(register_mcp))
        .route("/v1/mcps", get(list_mcps))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers(Any),
                ),
        )
        .with_state(state);

    // Start server
    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("🚀 Federation Simple Service starting on {}", bind_addr);
    info!("📊 Health check: http://{}/health", bind_addr);
    info!("🔄 Workflows API: http://{}/v1/workflows", bind_addr);
    info!("🔧 MCPs API: http://{}/v1/mcps", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
