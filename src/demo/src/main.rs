//! AI-CORE MVP Demo Orchestrator
//!
//! Complete end-to-end demonstration of the AI-CORE platform capabilities.
//! This service orchestrates the entire MVP demo flow from natural language
//! input to content generation and publishing simulation.

use axum::{
    extract::{ws::WebSocket, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::{net::TcpListener, sync::RwLock, time::sleep};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub config: DemoConfig,
    pub workflow_store: Arc<RwLock<HashMap<Uuid, WorkflowExecution>>>,
    pub demo_scenarios: Arc<Vec<DemoScenario>>,
    pub real_time_clients: Arc<RwLock<HashMap<Uuid, tokio::sync::mpsc::UnboundedSender<String>>>>,
}

#[derive(Debug, Clone)]
pub struct DemoConfig {
    pub host: String,
    pub port: u16,
    pub content_mcp_url: String,
    pub federation_url: String,
    pub service_name: String,
    pub version: String,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            content_mcp_url: "http://localhost:8081".to_string(),
            federation_url: "http://localhost:8082".to_string(),
            service_name: "ai-core-mvp-demo".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoScenario {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub input: String,
    pub expected_outcome: String,
    pub estimated_duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub scenario_id: Option<Uuid>,
    pub natural_language_input: String,
    pub parsed_intent: Option<ParsedIntent>,
    pub workflow_plan: Option<WorkflowPlan>,
    pub execution_steps: VecDeque<ExecutionStep>,
    pub current_step: usize,
    pub status: WorkflowStatus,
    pub progress_percentage: f32,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub results: Vec<StepResult>,
    pub cost_tracking: CostTracking,
    pub federation_info: Option<FederationInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedIntent {
    pub confidence: f32,
    pub domain: String,
    pub actions: Vec<String>,
    pub entities: HashMap<String, String>,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowPlan {
    pub total_steps: u32,
    pub estimated_duration_seconds: u32,
    pub estimated_cost_dollars: f32,
    pub required_services: Vec<String>,
    pub client_integrations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStep {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub service: String,
    pub estimated_duration_seconds: u32,
    pub status: StepStatus,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    pub step_id: Uuid,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub cost_dollars: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostTracking {
    pub total_cost_dollars: f32,
    pub breakdown: HashMap<String, f32>,
    pub token_usage: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FederationInfo {
    pub client_id: Uuid,
    pub client_name: String,
    pub routing_decisions: Vec<RoutingDecision>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingDecision {
    pub step: String,
    pub selected_provider: String,
    pub reason: String,
    pub cost_comparison: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum WorkflowStatus {
    Pending,
    Parsing,
    Planning,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct DemoRequest {
    pub input: String,
    pub scenario_id: Option<Uuid>,
    pub client_preferences: Option<ClientPreferences>,
}

#[derive(Debug, Deserialize)]
pub struct ClientPreferences {
    pub cost_optimization: bool,
    pub preferred_providers: Vec<String>,
    pub quality_threshold: f32,
}

#[derive(Debug, Serialize)]
pub struct DemoResponse {
    pub workflow_id: Uuid,
    pub status: WorkflowStatus,
    pub message: String,
    pub websocket_url: String,
}

#[derive(Debug, Serialize)]
pub struct ProgressUpdate {
    pub workflow_id: Uuid,
    pub status: WorkflowStatus,
    pub progress_percentage: f32,
    pub current_step: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub cost_so_far: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_core_mvp_demo=info,tower_http=debug".into()),
        )
        .init();

    info!(
        "🚀 Starting AI-CORE MVP Demo Orchestrator v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Initialize configuration
    let config = DemoConfig::default();

    // Initialize demo scenarios
    let demo_scenarios = Arc::new(initialize_demo_scenarios());

    // Create application state
    let state = AppState {
        config: config.clone(),
        workflow_store: Arc::new(RwLock::new(HashMap::new())),
        demo_scenarios,
        real_time_clients: Arc::new(RwLock::new(HashMap::new())),
    };

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("🌟 AI-CORE MVP Demo running on http://{}", addr);
    info!("📊 Demo Dashboard: http://{}/", addr);
    info!("🔌 WebSocket endpoint: ws://{}/ws", addr);
    info!("📋 Available scenarios: http://{}/api/v1/scenarios", addr);

    // Graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("✅ AI-CORE MVP Demo shut down gracefully");
    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        // Demo UI
        .route("/", get(demo_dashboard))
        .route("/demo/:workflow_id", get(workflow_viewer))
        // API endpoints
        .route("/api/v1/demo/start", post(start_demo))
        .route("/api/v1/workflows/:workflow_id", get(get_workflow))
        .route(
            "/api/v1/workflows/:workflow_id/cancel",
            post(cancel_workflow),
        )
        .route("/api/v1/scenarios", get(list_scenarios))
        .route("/api/v1/health", get(health_check))
        // WebSocket for real-time updates
        .route("/ws/:workflow_id", get(websocket_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// Demo dashboard HTML
async fn demo_dashboard() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AI-CORE MVP Demo</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { text-align: center; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 10px; margin-bottom: 30px; }
        .demo-section { background: white; padding: 30px; border-radius: 10px; margin-bottom: 20px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .input-area { margin-bottom: 20px; }
        .input-area textarea { width: 100%; height: 100px; padding: 15px; border: 2px solid #ddd; border-radius: 5px; font-size: 16px; }
        .button { background: #667eea; color: white; padding: 15px 30px; border: none; border-radius: 5px; cursor: pointer; font-size: 16px; margin: 10px 5px 0 0; }
        .button:hover { background: #5a6fd8; }
        .scenarios { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin-top: 20px; }
        .scenario { background: #f8f9ff; padding: 20px; border-radius: 8px; border-left: 4px solid #667eea; cursor: pointer; }
        .scenario:hover { background: #f0f2ff; }
        .progress-area { margin-top: 30px; padding: 20px; background: #f8f9fa; border-radius: 8px; display: none; }
        .progress-bar { width: 100%; height: 20px; background: #e9ecef; border-radius: 10px; overflow: hidden; margin: 10px 0; }
        .progress-fill { height: 100%; background: linear-gradient(90deg, #28a745, #20c997); transition: width 0.3s ease; }
        .log-area { background: #212529; color: #28a745; padding: 20px; border-radius: 8px; font-family: monospace; height: 300px; overflow-y: auto; margin-top: 20px; }
        .status { padding: 10px; border-radius: 5px; margin: 10px 0; }
        .status.success { background: #d4edda; color: #155724; border: 1px solid #c3e6cb; }
        .status.error { background: #f8d7da; color: #721c24; border: 1px solid #f5c6cb; }
        .status.info { background: #d1ecf1; color: #0c5460; border: 1px solid #bee5eb; }
        .cost-tracker { background: #fff3cd; padding: 15px; border-radius: 5px; margin: 10px 0; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 AI-CORE MVP Demo</h1>
            <p>Complete Intelligent Automation Platform Demonstration</p>
            <p><strong>Experience the full workflow:</strong> Natural Language → AI Parsing → Workflow Orchestration → Content Generation → Federation → Results</p>
        </div>

        <div class="demo-section">
            <h2>🎯 Try the Demo</h2>
            <div class="input-area">
                <label for="demoInput"><strong>Enter your automation request in natural language:</strong></label>
                <textarea id="demoInput" placeholder="Example: Create a blog post about AI automation trends and schedule it on our WordPress site and LinkedIn"></textarea>
            </div>
            <button class="button" onclick="startDemo()">🚀 Start Demo</button>
            <button class="button" onclick="loadScenarios()">📋 Load Example Scenarios</button>

            <div id="scenarios" class="scenarios"></div>

            <div id="progressArea" class="progress-area">
                <h3>🔄 Workflow Progress</h3>
                <div id="workflowStatus" class="status info">Initializing...</div>
                <div class="progress-bar">
                    <div id="progressFill" class="progress-fill" style="width: 0%"></div>
                </div>
                <div id="currentStep">Step: Preparing...</div>
                <div id="costTracker" class="cost-tracker">
                    <strong>💰 Cost Tracking:</strong> <span id="currentCost">$0.00</span>
                </div>
                <div id="logArea" class="log-area"></div>
            </div>
        </div>

        <div class="demo-section">
            <h2>📊 Demo Features</h2>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px;">
                <div style="padding: 20px; background: #f8f9ff; border-radius: 8px;">
                    <h3>🧠 Intent Parsing</h3>
                    <p>Multi-LLM intent analysis with 90%+ accuracy</p>
                </div>
                <div style="padding: 20px; background: #f8f9ff; border-radius: 8px;">
                    <h3>⚡ Real-time Progress</h3>
                    <p>Live workflow updates via WebSocket</p>
                </div>
                <div style="padding: 20px; background: #f8f9ff; border-radius: 8px;">
                    <h3>🔗 Federation</h3>
                    <p>Multi-client system coordination</p>
                </div>
                <div style="padding: 20px; background: #f8f9ff; border-radius: 8px;">
                    <h3>💰 Cost Optimization</h3>
                    <p>Real-time cost tracking and optimization</p>
                </div>
            </div>
        </div>
    </div>

    <script>
        let currentWorkflowId = null;
        let websocket = null;

        async function startDemo() {
            const input = document.getElementById('demoInput').value.trim();
            if (!input) {
                alert('Please enter a demo request');
                return;
            }

            try {
                const response = await fetch('/api/v1/demo/start', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ input: input })
                });

                if (!response.ok) throw new Error('Failed to start demo');

                const result = await response.json();
                currentWorkflowId = result.workflow_id;

                document.getElementById('progressArea').style.display = 'block';
                document.getElementById('workflowStatus').innerHTML = `✅ Demo started! Workflow ID: ${result.workflow_id}`;
                document.getElementById('workflowStatus').className = 'status success';

                connectWebSocket(result.workflow_id);
                logMessage(`🚀 Demo started: ${input}`);
            } catch (error) {
                console.error('Error starting demo:', error);
                alert('Failed to start demo: ' + error.message);
            }
        }

        function connectWebSocket(workflowId) {
            if (websocket) websocket.close();

            const wsUrl = `ws://${window.location.host}/ws/${workflowId}`;
            websocket = new WebSocket(wsUrl);

            websocket.onmessage = function(event) {
                const update = JSON.parse(event.data);
                updateProgress(update);
            };

            websocket.onclose = function() {
                logMessage('🔌 WebSocket connection closed');
            };

            websocket.onerror = function(error) {
                logMessage('❌ WebSocket error: ' + error);
            };
        }

        function updateProgress(update) {
            document.getElementById('progressFill').style.width = update.progress_percentage + '%';
            document.getElementById('currentStep').textContent = `Step: ${update.current_step}`;
            document.getElementById('currentCost').textContent = `$${update.cost_so_far.toFixed(2)}`;

            let statusClass = 'info';
            if (update.status === 'Completed') statusClass = 'success';
            if (update.status === 'Failed') statusClass = 'error';

            document.getElementById('workflowStatus').innerHTML = `${getStatusIcon(update.status)} ${update.status}: ${update.message}`;
            document.getElementById('workflowStatus').className = `status ${statusClass}`;

            logMessage(`[${update.timestamp}] ${update.message}`);
        }

        function getStatusIcon(status) {
            const icons = {
                'Pending': '⏳',
                'Parsing': '🧠',
                'Planning': '📋',
                'Executing': '⚡',
                'Completed': '✅',
                'Failed': '❌',
                'Cancelled': '⏹️'
            };
            return icons[status] || '🔄';
        }

        function logMessage(message) {
            const logArea = document.getElementById('logArea');
            const timestamp = new Date().toLocaleTimeString();
            logArea.innerHTML += `[${timestamp}] ${message}\\n`;
            logArea.scrollTop = logArea.scrollHeight;
        }

        async function loadScenarios() {
            try {
                const response = await fetch('/api/v1/scenarios');
                const scenarios = await response.json();

                const scenariosDiv = document.getElementById('scenarios');
                scenariosDiv.innerHTML = scenarios.map(scenario => `
                    <div class="scenario" onclick="selectScenario('${scenario.input}')">
                        <h4>${scenario.name}</h4>
                        <p>${scenario.description}</p>
                        <small><strong>Example:</strong> "${scenario.input}"</small>
                        <br><small><strong>Expected outcome:</strong> ${scenario.expected_outcome}</small>
                    </div>
                `).join('');
            } catch (error) {
                console.error('Error loading scenarios:', error);
            }
        }

        function selectScenario(input) {
            document.getElementById('demoInput').value = input;
        }

        // Load scenarios on page load
        window.onload = function() {
            loadScenarios();
            logMessage('🌟 AI-CORE MVP Demo ready!');
            logMessage('💡 Try typing: "Create a blog post about AI automation and publish it to WordPress"');
        };
    </script>
</body>
</html>
"#,
    )
}

// Start demo endpoint
async fn start_demo(
    State(state): State<AppState>,
    Json(request): Json<DemoRequest>,
) -> Result<Json<DemoResponse>, StatusCode> {
    let workflow_id = Uuid::new_v4();

    info!(
        "🚀 Starting demo workflow {}: {}",
        workflow_id, request.input
    );

    // Create workflow execution
    let workflow = WorkflowExecution {
        id: workflow_id,
        scenario_id: request.scenario_id,
        natural_language_input: request.input.clone(),
        parsed_intent: None,
        workflow_plan: None,
        execution_steps: initialize_demo_steps(),
        current_step: 0,
        status: WorkflowStatus::Pending,
        progress_percentage: 0.0,
        start_time: Utc::now(),
        end_time: None,
        results: Vec::new(),
        cost_tracking: CostTracking {
            total_cost_dollars: 0.0,
            breakdown: HashMap::new(),
            token_usage: HashMap::new(),
        },
        federation_info: Some(FederationInfo {
            client_id: Uuid::new_v4(),
            client_name: "Demo Client".to_string(),
            routing_decisions: Vec::new(),
        }),
    };

    // Store workflow
    state
        .workflow_store
        .write()
        .await
        .insert(workflow_id, workflow);

    // Start workflow execution in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        execute_demo_workflow(state_clone, workflow_id).await;
    });

    Ok(Json(DemoResponse {
        workflow_id,
        status: WorkflowStatus::Pending,
        message: "Demo workflow started successfully".to_string(),
        websocket_url: format!("/ws/{}", workflow_id),
    }))
}

// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(workflow_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, workflow_id, state))
}

async fn handle_websocket(socket: WebSocket, workflow_id: Uuid, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Store client connection
    state
        .real_time_clients
        .write()
        .await
        .insert(workflow_id, tx);

    // Handle incoming messages (if any)
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if msg.is_err() {
                break;
            }
        }
    });

    // Handle outgoing messages
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender
                .send(axum::extract::ws::Message::Text(msg))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = receive_task => {},
        _ = send_task => {},
    }

    // Clean up
    state.real_time_clients.write().await.remove(&workflow_id);
}

// Execute the complete demo workflow
async fn execute_demo_workflow(state: AppState, workflow_id: Uuid) {
    let steps = [
        (
            "Parsing Intent",
            "🧠 Analyzing natural language input with AI",
            15,
        ),
        (
            "Planning Workflow",
            "📋 Creating optimized execution plan",
            10,
        ),
        (
            "Content Generation",
            "✍️ Generating high-quality content",
            25,
        ),
        ("Federation Routing", "🔗 Routing to optimal providers", 8),
        (
            "Publishing Content",
            "📤 Publishing to target platforms",
            20,
        ),
        (
            "Quality Validation",
            "✅ Validating results and compliance",
            12,
        ),
        ("Cost Optimization", "💰 Finalizing cost analysis", 5),
        ("Completion", "🎉 Workflow completed successfully", 5),
    ];

    for (index, (step_name, description, duration)) in steps.iter().enumerate() {
        // Update workflow status
        {
            let mut store = state.workflow_store.write().await;
            if let Some(workflow) = store.get_mut(&workflow_id) {
                workflow.current_step = index;
                workflow.status = if index == steps.len() - 1 {
                    WorkflowStatus::Completed
                } else {
                    WorkflowStatus::Executing
                };
                workflow.progress_percentage = ((index + 1) as f32 / steps.len() as f32) * 100.0;

                // Update cost tracking
                let step_cost = rand::thread_rng().gen_range(0.05..0.50);
                workflow.cost_tracking.total_cost_dollars += step_cost;
                workflow
                    .cost_tracking
                    .breakdown
                    .insert(step_name.to_string(), step_cost);
            }
        }

        // Send real-time update
        send_progress_update(&state, workflow_id, step_name, description).await;

        // Simulate processing time
        sleep(Duration::from_secs(*duration as u64)).await;

        // Simulate step completion with results
        if index == 2 {
            // Content generation step - simulate creating content
            simulate_content_generation(&state, workflow_id).await;
        }
    }

    // Mark as completed
    {
        let mut store = state.workflow_store.write().await;
        if let Some(workflow) = store.get_mut(&workflow_id) {
            workflow.status = WorkflowStatus::Completed;
            workflow.end_time = Some(Utc::now());
            workflow.progress_percentage = 100.0;
        }
    }

    info!("✅ Demo workflow {} completed successfully", workflow_id);
}

async fn send_progress_update(
    state: &AppState,
    workflow_id: Uuid,
    step_name: &str,
    description: &str,
) {
    let (status, progress, cost) = {
        let store = state.workflow_store.read().await;
        if let Some(workflow) = store.get(&workflow_id) {
            (
                workflow.status.clone(),
                workflow.progress_percentage,
                workflow.cost_tracking.total_cost_dollars,
            )
        } else {
            return;
        }
    };

    let update = ProgressUpdate {
        workflow_id,
        status,
        progress_percentage: progress,
        current_step: step_name.to_string(),
        message: description.to_string(),
        timestamp: Utc::now(),
        cost_so_far: cost,
    };

    if let Ok(message) = serde_json::to_string(&update) {
        let clients = state.real_time_clients.read().await;
        if let Some(client) = clients.get(&workflow_id) {
            let _ = client.send(message);
        }
    }
}

async fn simulate_content_generation(state: &AppState, workflow_id: Uuid) {
    // Simulate calling the content MCP service
    let content_request = serde_json::json!({
        "content_type": "blog_post",
        "topic": "AI automation trends",
        "target_audience": "business professionals",
        "tone": "professional",
        "include_images": true,
        "seo_keywords": ["AI", "automation", "business", "technology"]
    });

    // In a real implementation, this would call the actual content MCP
    // For demo, we simulate the response
    let content_response = serde_json::json!({
        "id": Uuid::new_v4(),
        "title": "The Future of AI Automation: Transforming Business Operations in 2024",
        "content": "AI automation has revolutionized how businesses operate...",
        "word_count": 1200,
        "seo_score": 87,
        "images": [
            {
                "url": "https://demo-images.ai-core.dev/ai-automation/hero.jpg",
                "alt_text": "AI automation dashboard"
            }
        ]
    });

    // Store the result
    {
        let mut store = state.workflow_store.write().await;
        if let Some(workflow) = store.get_mut(&workflow_id) {
            workflow.results.push(StepResult {
                step_id: Uuid::new_v4(),
                success: true,
                output: content_response,
                duration_ms: 2500,
                cost_dollars: 0.25,
                metadata: [
                    ("service".to_string(), "content-mcp".to_string()),
                    ("provider".to_string(), "demo-provider".to_string()),
                ]
                .into_iter()
                .collect(),
            });
        }
    }
}

// Get workflow status
async fn get_workflow(
    Path(workflow_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<WorkflowExecution>, StatusCode> {
    let store = state.workflow_store.read().await;
    match store.get(&workflow_id) {
        Some(workflow) => Ok(Json(workflow.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// List demo scenarios
async fn list_scenarios(State(state): State<AppState>) -> Json<Vec<DemoScenario>> {
    Json((*state.demo_scenarios).clone())
}

// Health check
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "ai-core-mvp-demo",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now(),
        "uptime_seconds": 0
    }))
}

// Cancel workflow
async fn cancel_workflow(
    Path(workflow_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut store = state.workflow_store.write().await;
    match store.get_mut(&workflow_id) {
        Some(workflow) => {
            workflow.status = WorkflowStatus::Cancelled;
            workflow.end_time = Some(Utc::now());
            Ok(Json(serde_json::json!({
                "message": "Workflow cancelled successfully"
            })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// Workflow viewer page
async fn workflow_viewer(Path(workflow_id): Path<Uuid>) -> Html<String> {
    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Workflow {} - AI-CORE Demo</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 1000px; margin: 0 auto; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 10px; margin-bottom: 20px; }}
        .workflow-details {{ background: white; padding: 20px; border-radius: 10px; margin-bottom: 20px; }}
        .back-button {{ background: #6c757d; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px; display: inline-block; margin-bottom: 20px; }}
        .status {{ padding: 10px; border-radius: 5px; margin: 10px 0; }}
        .status.completed {{ background: #d4edda; color: #155724; }}
        .status.executing {{ background: #d1ecf1; color: #0c5460; }}
        .status.failed {{ background: #f8d7da; color: #721c24; }}
        pre {{ background: #f8f9fa; padding: 15px; border-radius: 5px; overflow-x: auto; }}
    </style>
</head>
<body>
    <div class="container">
        <a href="/" class="back-button">← Back to Demo</a>
        <div class="header">
            <h1>📊 Workflow Details</h1>
            <p>Workflow ID: {}</p>
        </div>
        <div class="workflow-details">
            <div id="workflowData">Loading workflow details...</div>
        </div>
    </div>

    <script>
        async function loadWorkflowDetails() {{
            try {{
                const response = await fetch('/api/v1/workflows/{}');
                const workflow = await response.json();

                document.getElementById('workflowData').innerHTML = `
                    <h3>Status</h3>
                    <div class="status ${{workflow.status.toLowerCase()}}">${{workflow.status}}</div>

                    <h3>Input</h3>
                    <p>"${{workflow.natural_language_input}}"</p>

                    <h3>Progress</h3>
                    <p>${{workflow.progress_percentage.toFixed(1)}}% complete</p>

                    <h3>Cost Tracking</h3>
                    <p>Total Cost: $$${{workflow.cost_tracking.total_cost_dollars.toFixed(2)}}</p>

                    <h3>Execution Steps</h3>
                    <pre>${{JSON.stringify(workflow.execution_steps, null, 2)}}</pre>

                    <h3>Results</h3>
                    <pre>${{JSON.stringify(workflow.results, null, 2)}}</pre>
                `;
            }} catch (error) {{
                document.getElementById('workflowData').innerHTML = `<p>Error loading workflow: ${{error.message}}</p>`;
            }}
        }}

        window.onload = loadWorkflowDetails;
    </script>
</body>
</html>
"#,
        workflow_id, workflow_id, workflow_id
    ))
}

// Initialize demo execution steps
fn initialize_demo_steps() -> VecDeque<ExecutionStep> {
    [
        ("Intent Parsing", "Parse natural language input"),
        ("Workflow Planning", "Generate execution plan"),
        ("Content Generation", "Create content using MCP"),
        ("Federation Routing", "Route to optimal providers"),
        ("Publishing", "Publish to target platforms"),
        ("Validation", "Validate results"),
        ("Cost Analysis", "Calculate final costs"),
        ("Completion", "Finalize workflow"),
    ]
    .iter()
    .map(|(name, desc)| ExecutionStep {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: desc.to_string(),
        service: "demo-orchestrator".to_string(),
        estimated_duration_seconds: 10,
        status: StepStatus::Pending,
        start_time: None,
        end_time: None,
    })
    .collect()
}

// Initialize demo scenarios
fn initialize_demo_scenarios() -> Vec<DemoScenario> {
    vec![
        DemoScenario {
            id: Uuid::new_v4(),
            name: "Blog Post Creation & Publishing".to_string(),
            description: "Create and publish a technical blog post with SEO optimization".to_string(),
            input: "Create a blog post about AI automation trends and schedule it on our WordPress site and LinkedIn".to_string(),
            expected_outcome: "High-quality blog post generated, optimized for SEO, and published to multiple platforms".to_string(),
            estimated_duration_seconds: 120,
        },
        DemoScenario {
            id: Uuid::new_v4(),
            name: "Social Media Campaign".to_string(),
            description: "Generate and schedule social media content across platforms".to_string(),
            input: "Create a social media campaign about our new product launch for Twitter, LinkedIn, and Facebook".to_string(),
            expected_outcome: "Platform-optimized social media posts with appropriate hashtags and scheduling".to_string(),
            estimated_duration_seconds: 90,
        },
        DemoScenario {
            id: Uuid::new_v4(),
            name: "Email Newsletter".to_string(),
            description: "Create and send a weekly newsletter with curated content".to_string(),
            input: "Generate our weekly tech newsletter with the latest AI developments and send it to subscribers".to_string(),
            expected_outcome: "Professional newsletter with curated content and personalized sections".to_string(),
            estimated_duration_seconds: 100,
        },
        DemoScenario {
            id: Uuid::new_v4(),
            name: "Multi-Client Federation Demo".to_string(),
            description: "Demonstrate federation across multiple client systems".to_string(),
            input: "Create marketing content for Client A and publish it using Client B's premium publishing service".to_string(),
            expected_outcome: "Content created by one client's system and published through another's infrastructure".to_string(),
            estimated_duration_seconds: 150,
        },
        DemoScenario {
            id: Uuid::new_v4(),
            name: "Cost Optimization Demo".to_string(),
            description: "Show intelligent cost optimization across providers".to_string(),
            input: "Generate a comprehensive market analysis report using the most cost-effective AI providers".to_string(),
            expected_outcome: "High-quality report generated using optimal provider selection for cost efficiency".to_string(),
            estimated_duration_seconds: 180,
        },
    ]
}
