use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

mod config;
mod error;
mod llm;
mod parser;
mod types;

use config::Config;
use error::{AppError, Result};
use llm::LLMClient;
use parser::IntentParser;
use types::*;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub intent_parser: Arc<IntentParser>,
    pub llm_client: Arc<LLMClient>,
    pub health_status: Arc<tokio::sync::RwLock<HealthStatus>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub llm_status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "intent_parser=info,tower_http=debug".into()),
        )
        .init();

    info!("Starting Intent Parser Service");

    // Load configuration
    let config = Arc::new(Config::from_env()?);
    info!("Configuration loaded successfully");

    // Initialize LLM client
    let llm_client = Arc::new(LLMClient::new(&config).await?);
    info!("LLM client initialized");

    // Initialize intent parser
    let intent_parser = Arc::new(IntentParser::new(llm_client.clone()));
    info!("Intent parser initialized");

    // Initialize health status
    let start_time = std::time::Instant::now();
    let health_status = Arc::new(tokio::sync::RwLock::new(HealthStatus {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        llm_status: "connected".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,
    }));

    // Create application state
    let state = AppState {
        config: config.clone(),
        intent_parser,
        llm_client,
        health_status: health_status.clone(),
    };

    // Start health monitoring task
    let health_monitor_state = state.clone();
    let health_start_time = start_time;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let uptime = health_start_time.elapsed().as_secs();
            let llm_status = match health_monitor_state.llm_client.health_check().await {
                Ok(_) => "connected".to_string(),
                Err(_) => "disconnected".to_string(),
            };

            let mut health = health_monitor_state.health_status.write().await;
            health.timestamp = chrono::Utc::now();
            health.uptime_seconds = uptime;
            health.llm_status = llm_status;

            if health.llm_status == "disconnected" {
                health.status = "degraded".to_string();
                warn!("LLM client disconnected, service running in degraded mode");
            } else {
                health.status = "healthy".to_string();
            }
        }
    });

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Intent Parser Service listening on {}", addr);

    // Graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("Intent Parser Service shut down gracefully");
    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/parse", post(parse_intent))
        .route("/v1/parse/batch", post(parse_batch_intents))
        .route("/v1/parse/validate", post(validate_intent))
        .route("/v1/capabilities", get(get_capabilities))
        .route("/v1/functions", get(list_functions))
        .route("/v1/functions/:function_id", get(get_function_details))
        .route("/v1/context/:user_id", get(get_user_context))
        .route("/v1/context/:user_id", post(update_user_context))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    request_logging_middleware,
                )),
        )
        .with_state(state)
}

// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Result<Json<HealthStatus>> {
    let health = state.health_status.read().await;
    let health_status = (*health).clone();
    Ok(Json(health_status))
}

// Parse natural language intent into structured workflow
async fn parse_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ParseIntentRequest>,
) -> Result<Json<ParsedIntent>> {
    info!("Parsing intent for user: {}", request.user_id);

    // Extract user context from headers if available
    let user_context = extract_user_context(&headers)?;

    // Parse the intent
    let parsed_intent = state
        .intent_parser
        .parse_request(&request, user_context)
        .await
        .map_err(|e| {
            error!("Failed to parse intent: {:?}", e);
            AppError::InternalServerError(format!("Intent parsing failed: {}", e))
        })?;

    info!(
        "Successfully parsed intent for user {}: {} functions generated",
        request.user_id,
        parsed_intent.functions.len()
    );

    Ok(Json(parsed_intent))
}

// Parse multiple intents in batch
async fn parse_batch_intents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BatchParseRequest>,
) -> Result<Json<BatchParseResponse>> {
    info!("Parsing {} intents in batch", request.requests.len());

    if request.requests.len() > state.config.max_batch_size {
        return Err(AppError::BadRequest(format!(
            "Batch size {} exceeds maximum of {}",
            request.requests.len(),
            state.config.max_batch_size
        )));
    }

    let user_context = extract_user_context(&headers)?;
    let mut results = Vec::new();
    let mut errors = Vec::new();

    // Process each request
    for (index, parse_request) in request.requests.into_iter().enumerate() {
        match state
            .intent_parser
            .parse_request(&parse_request, user_context.clone())
            .await
        {
            Ok(parsed_intent) => {
                results.push(BatchParseResult {
                    index,
                    success: true,
                    intent: Some(parsed_intent),
                    error: None,
                });
            }
            Err(error) => {
                warn!("Failed to parse intent at index {}: {:?}", index, error);
                errors.push(format!("Index {}: {}", index, error));
                results.push(BatchParseResult {
                    index,
                    success: false,
                    intent: None,
                    error: Some(error.to_string()),
                });
            }
        }
    }

    info!(
        "Batch parsing completed: {} successful, {} errors",
        results.iter().filter(|r| r.success).count(),
        errors.len()
    );

    Ok(Json(BatchParseResponse {
        total_processed: results.len(),
        successful: results.iter().filter(|r| r.success).count(),
        failed: errors.len(),
        results,
        errors,
    }))
}

// Validate parsed intent structure
async fn validate_intent(
    State(state): State<AppState>,
    Json(intent): Json<ParsedIntent>,
) -> Result<Json<ValidationResponse>> {
    info!(
        "Validating parsed intent with {} functions",
        intent.functions.len()
    );

    let validation_result = state.intent_parser.validate_intent(&intent).await?;

    Ok(Json(ValidationResponse {
        valid: validation_result.is_valid,
        confidence_score: validation_result.confidence_score,
        warnings: validation_result.warnings,
        suggestions: validation_result.suggestions,
        estimated_execution_time: validation_result.estimated_execution_time,
        estimated_cost: validation_result.estimated_cost,
    }))
}

// Get available capabilities and functions
async fn get_capabilities(State(state): State<AppState>) -> Result<Json<CapabilitiesResponse>> {
    let capabilities = state.intent_parser.get_available_capabilities().await?;

    Ok(Json(CapabilitiesResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        supported_functions: capabilities.functions,
        supported_domains: capabilities.domains,
        supported_integrations: capabilities.integrations,
        max_complexity_score: capabilities.max_complexity_score,
        max_functions_per_workflow: capabilities.max_functions_per_workflow,
    }))
}

// List all available functions
async fn list_functions(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<FunctionListResponse>> {
    let domain_filter = params.get("domain");
    let functions = state
        .intent_parser
        .list_available_functions(domain_filter)
        .await?;

    Ok(Json(FunctionListResponse {
        total_count: functions.len(),
        filtered_by_domain: domain_filter.cloned(),
        functions,
    }))
}

// Get detailed information about a specific function
async fn get_function_details(
    State(state): State<AppState>,
    Path(function_id): Path<String>,
) -> Result<Json<FunctionDetails>> {
    let details = state
        .intent_parser
        .get_function_details(&function_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Function not found: {}", function_id)))?;

    Ok(Json(details))
}

// Get user context for personalized parsing
async fn get_user_context(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserContext>> {
    let context = state
        .intent_parser
        .get_user_context(user_id)
        .await?
        .unwrap_or_default();

    Ok(Json(context))
}

// Update user context
async fn update_user_context(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(context): Json<UserContext>,
) -> Result<Json<UserContext>> {
    let updated_context = state
        .intent_parser
        .update_user_context(user_id, context)
        .await?;

    Ok(Json(updated_context))
}

// Extract user context from request headers
fn extract_user_context(headers: &HeaderMap) -> Result<Option<UserContext>> {
    if let Some(context_header) = headers.get("x-user-context") {
        let context_str = context_header
            .to_str()
            .map_err(|_| AppError::BadRequest("Invalid user context header".to_string()))?;

        let context: UserContext = serde_json::from_str(context_str)
            .map_err(|_| AppError::BadRequest("Invalid user context JSON".to_string()))?;

        Ok(Some(context))
    } else {
        Ok(None)
    }
}

// Request logging middleware
async fn request_logging_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start_time = std::time::Instant::now();

    let response = next.run(req).await;

    let duration = start_time.elapsed();
    info!(
        "{} {} - {:?} - {}ms",
        method,
        uri,
        response.status(),
        duration.as_millis()
    );

    response
}
