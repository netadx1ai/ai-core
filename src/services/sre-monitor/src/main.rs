use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::{interval, sleep};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{error, info, warn};
use uuid::Uuid;

mod config;
mod error_budget;
mod metrics;
mod models;
mod slo;

use config::Config;
use error_budget::ErrorBudgetTracker;
use metrics::MetricsCollector;
use models::*;
use slo::SloValidator;

#[derive(Clone)]
pub struct AppState {
    db: PgPool,
    metrics_collector: Arc<MetricsCollector>,
    error_budget_tracker: Arc<ErrorBudgetTracker>,
    slo_validator: Arc<SloValidator>,
    config: Arc<Config>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting SRE Monitor Service");

    // Load configuration
    let config = Arc::new(Config::from_env()?);

    // Initialize database connection
    let database_url = config.database_url.clone();
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&database_url)
        .await?;

    // Run database migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    // Initialize components
    let metrics_collector = Arc::new(MetricsCollector::new());
    let error_budget_tracker = Arc::new(ErrorBudgetTracker::new(db.clone()));
    let slo_validator = Arc::new(SloValidator::new(db.clone()));

    let state = AppState {
        db: db.clone(),
        metrics_collector: metrics_collector.clone(),
        error_budget_tracker: error_budget_tracker.clone(),
        slo_validator: slo_validator.clone(),
        config: config.clone(),
    };

    // Start background monitoring tasks
    tokio::spawn(run_monitoring_loop(state.clone()));
    tokio::spawn(run_slo_validation_loop(state.clone()));
    tokio::spawn(run_error_budget_calculation(state.clone()));

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .route("/slo", get(get_slos).post(create_slo))
        .route("/slo/:id", get(get_slo).put(update_slo))
        .route("/error-budget", get(get_error_budget))
        .route("/error-budget/:service", get(get_service_error_budget))
        .route("/alerts", get(get_alerts).post(create_alert))
        .route("/incidents", get(get_incidents).post(create_incident))
        .route("/incidents/:id", get(get_incident).put(update_incident))
        .route("/service-health", get(get_service_health))
        .route("/service-health/:service", get(get_service_specific_health))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(CorsLayer::permissive())
        // Body limit is handled by axum defaults
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.server.bind_address).await?;
    info!("SRE Monitor listening on {}", config.server.bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Result<Json<HealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<MetricsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.metrics_collector.get_all_metrics().await {
        Ok(metrics) => Ok(Json(MetricsResponse { metrics })),
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve metrics".to_string(),
                    code: "METRICS_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn get_slos(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SloListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service_name = params.get("service").map(|s| s.as_str());

    match state.slo_validator.get_slos(service_name).await {
        Ok(slos) => Ok(Json(SloListResponse { slos })),
        Err(e) => {
            error!("Failed to get SLOs: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve SLOs".to_string(),
                    code: "SLO_FETCH_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn create_slo(
    State(state): State<AppState>,
    Json(request): Json<CreateSloRequest>,
) -> Result<Json<SloResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.slo_validator.create_slo(request).await {
        Ok(slo) => Ok(Json(SloResponse { slo })),
        Err(e) => {
            error!("Failed to create SLO: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Failed to create SLO: {}", e),
                    code: "SLO_CREATE_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn get_slo(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SloResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.slo_validator.get_slo_by_id(id).await {
        Ok(Some(slo)) => Ok(Json(SloResponse { slo })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "SLO not found".to_string(),
                code: "SLO_NOT_FOUND".to_string(),
            }),
        )),
        Err(e) => {
            error!("Failed to get SLO: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve SLO".to_string(),
                    code: "SLO_FETCH_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn update_slo(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateSloRequest>,
) -> Result<Json<SloResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.slo_validator.update_slo(id, request).await {
        Ok(slo) => Ok(Json(SloResponse { slo })),
        Err(e) => {
            error!("Failed to update SLO: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Failed to update SLO: {}", e),
                    code: "SLO_UPDATE_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn get_error_budget(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ErrorBudgetResponse>, (StatusCode, Json<ErrorResponse>)> {
    let time_window = params.get("window").unwrap_or(&"30d".to_string()).clone();

    match state
        .error_budget_tracker
        .get_all_error_budgets(&time_window)
        .await
    {
        Ok(budgets) => Ok(Json(ErrorBudgetResponse { budgets })),
        Err(e) => {
            error!("Failed to get error budgets: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve error budgets".to_string(),
                    code: "ERROR_BUDGET_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn get_service_error_budget(
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ServiceErrorBudgetResponse>, (StatusCode, Json<ErrorResponse>)> {
    let time_window = params.get("window").unwrap_or(&"30d".to_string()).clone();

    match state
        .error_budget_tracker
        .get_service_error_budget(&service, &time_window)
        .await
    {
        Ok(budget) => Ok(Json(ServiceErrorBudgetResponse { service, budget })),
        Err(e) => {
            error!("Failed to get error budget for service {}: {}", service, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to retrieve error budget for service {}", service),
                    code: "SERVICE_ERROR_BUDGET_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn get_alerts(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlertListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let status = params.get("status");
    let service = params.get("service");

    let alerts = sqlx::query_as!(
        Alert,
        r#"
        SELECT id, service_name, alert_type, severity, message, status,
               created_at, resolved_at, metadata
        FROM alerts
        WHERE ($1::text IS NULL OR status = $1)
        AND ($2::text IS NULL OR service_name = $2)
        ORDER BY created_at DESC
        LIMIT 100
        "#,
        status,
        service
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to fetch alerts: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve alerts".to_string(),
                code: "ALERT_FETCH_ERROR".to_string(),
            }),
        )
    })?;

    Ok(Json(AlertListResponse { alerts }))
}

async fn create_alert(
    State(state): State<AppState>,
    Json(request): Json<CreateAlertRequest>,
) -> Result<Json<AlertResponse>, (StatusCode, Json<ErrorResponse>)> {
    let alert_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let alert = sqlx::query_as!(
        Alert,
        r#"
        INSERT INTO alerts (id, service_name, alert_type, severity, message, status, created_at, metadata)
        VALUES ($1, $2, $3, $4, $5, 'active', $6, $7)
        RETURNING id, service_name, alert_type, severity, message, status,
                  created_at, resolved_at, metadata
        "#,
        alert_id,
        request.service_name,
        request.alert_type,
        request.severity,
        request.message,
        now,
        request.metadata
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to create alert: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to create alert".to_string(),
                code: "ALERT_CREATE_ERROR".to_string(),
            }),
        )
    })?;

    Ok(Json(AlertResponse { alert }))
}

async fn get_incidents(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<IncidentListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let status = params.get("status");
    let service = params.get("service");

    let incidents = sqlx::query_as!(
        Incident,
        r#"
        SELECT id, title, description, service_name, status, severity,
               created_at, resolved_at, metadata
        FROM incidents
        WHERE ($1::text IS NULL OR status = $1)
        AND ($2::text IS NULL OR service_name = $2)
        ORDER BY created_at DESC
        LIMIT 50
        "#,
        status,
        service
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to fetch incidents: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve incidents".to_string(),
                code: "INCIDENT_FETCH_ERROR".to_string(),
            }),
        )
    })?;

    Ok(Json(IncidentListResponse { incidents }))
}

async fn create_incident(
    State(state): State<AppState>,
    Json(request): Json<CreateIncidentRequest>,
) -> Result<Json<IncidentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let incident_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let incident = sqlx::query_as!(
        Incident,
        r#"
        INSERT INTO incidents (id, title, description, service_name, status, severity, created_at, metadata)
        VALUES ($1, $2, $3, $4, 'open', $5, $6, $7)
        RETURNING id, title, description, service_name, status, severity,
                  created_at, resolved_at, metadata
        "#,
        incident_id,
        request.title,
        request.description,
        request.service_name,
        request.severity,
        now,
        request.metadata
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to create incident: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to create incident".to_string(),
                code: "INCIDENT_CREATE_ERROR".to_string(),
            }),
        )
    })?;

    Ok(Json(IncidentResponse { incident }))
}

async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IncidentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let incident = sqlx::query_as!(
        Incident,
        r#"
        SELECT id, title, description, service_name, status, severity,
               created_at, resolved_at, metadata
        FROM incidents
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to fetch incident: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve incident".to_string(),
                code: "INCIDENT_FETCH_ERROR".to_string(),
            }),
        )
    })?;

    match incident {
        Some(incident) => Ok(Json(IncidentResponse { incident })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Incident not found".to_string(),
                code: "INCIDENT_NOT_FOUND".to_string(),
            }),
        )),
    }
}

async fn update_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateIncidentRequest>,
) -> Result<Json<IncidentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let resolved_at = if request.status.as_deref() == Some("resolved") {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let incident = sqlx::query_as!(
        Incident,
        r#"
        UPDATE incidents
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            status = COALESCE($4, status),
            severity = COALESCE($5, severity),
            resolved_at = CASE WHEN $6::timestamptz IS NOT NULL THEN $6 ELSE resolved_at END,
            metadata = COALESCE($7, metadata)
        WHERE id = $1
        RETURNING id, title, description, service_name, status, severity,
                  created_at, resolved_at, metadata
        "#,
        id,
        request.title,
        request.description,
        request.status,
        request.severity,
        resolved_at,
        request.metadata
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to update incident: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update incident".to_string(),
                code: "INCIDENT_UPDATE_ERROR".to_string(),
            }),
        )
    })?;

    match incident {
        Some(incident) => Ok(Json(IncidentResponse { incident })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Incident not found".to_string(),
                code: "INCIDENT_NOT_FOUND".to_string(),
            }),
        )),
    }
}

async fn get_service_health(
    State(state): State<AppState>,
) -> Result<Json<ServiceHealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.metrics_collector.get_service_health_summary().await {
        Ok(health_summary) => Ok(Json(ServiceHealthResponse {
            services: health_summary,
        })),
        Err(e) => {
            error!("Failed to get service health: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve service health".to_string(),
                    code: "SERVICE_HEALTH_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn get_service_specific_health(
    State(state): State<AppState>,
    Path(service): Path<String>,
) -> Result<Json<ServiceSpecificHealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .metrics_collector
        .get_service_specific_health(&service)
        .await
    {
        Ok(health) => Ok(Json(ServiceSpecificHealthResponse {
            service: service.clone(),
            health,
        })),
        Err(e) => {
            error!("Failed to get health for service {}: {}", service, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to retrieve health for service {}", service),
                    code: "SERVICE_SPECIFIC_HEALTH_ERROR".to_string(),
                }),
            ))
        }
    }
}

async fn run_monitoring_loop(state: AppState) {
    let mut interval = interval(Duration::from_secs(
        state.config.monitoring.collection_interval,
    ));

    info!(
        "Starting monitoring loop with interval: {}s",
        state.config.monitoring.collection_interval
    );

    loop {
        interval.tick().await;

        if let Err(e) = state.metrics_collector.collect_metrics().await {
            error!("Error collecting metrics: {}", e);
        }

        // Check for SLO violations and trigger alerts
        if let Err(e) = check_slo_violations(&state).await {
            error!("Error checking SLO violations: {}", e);
        }
    }
}

async fn run_slo_validation_loop(state: AppState) {
    let mut interval = interval(Duration::from_secs(state.config.slo.validation_interval));

    info!(
        "Starting SLO validation loop with interval: {}s",
        state.config.slo.validation_interval
    );

    loop {
        interval.tick().await;

        if let Err(e) = state.slo_validator.validate_all_slos().await {
            error!("Error validating SLOs: {}", e);
        }
    }
}

async fn run_error_budget_calculation(state: AppState) {
    let mut interval = interval(Duration::from_secs(
        state.config.error_budget.calculation_interval,
    ));

    info!(
        "Starting error budget calculation loop with interval: {}s",
        state.config.error_budget.calculation_interval
    );

    loop {
        interval.tick().await;

        if let Err(e) = state.error_budget_tracker.calculate_error_budgets().await {
            error!("Error calculating error budgets: {}", e);
        }
    }
}

async fn check_slo_violations(state: &AppState) -> Result<(), anyhow::Error> {
    let violations = state.slo_validator.check_violations().await?;

    for violation in violations {
        warn!("SLO violation detected: {:?}", violation);

        // Create alert for SLO violation
        let alert_request = CreateAlertRequest {
            service_name: violation.service_name,
            alert_type: "slo_violation".to_string(),
            severity: violation.severity.clone(),
            message: format!(
                "SLO '{}' violated: {}",
                violation.slo_name, violation.description
            ),
            metadata: Some(serde_json::to_value(&violation)?),
        };

        if let Err(e) = create_alert_internal(state, alert_request).await {
            error!("Failed to create alert for SLO violation: {}", e);
        }
    }

    Ok(())
}

async fn create_alert_internal(
    state: &AppState,
    request: CreateAlertRequest,
) -> Result<Alert, sqlx::Error> {
    let alert_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query_as!(
        Alert,
        r#"
        INSERT INTO alerts (id, service_name, alert_type, severity, message, status, created_at, metadata)
        VALUES ($1, $2, $3, $4, $5, 'active', $6, $7)
        RETURNING id, service_name, alert_type, severity, message, status,
                  created_at, resolved_at, metadata
        "#,
        alert_id,
        request.service_name,
        request.alert_type,
        request.severity,
        request.message,
        now,
        request.metadata
    )
    .fetch_one(&state.db)
    .await
}
