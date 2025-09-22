use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

// Core data models

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Slo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub service_name: String,
    pub metric_name: String,
    pub target_percentage: f64,
    pub time_window: String,
    pub threshold_value: Option<f64>,
    pub operator: String, // "lt", "lte", "gt", "gte", "eq"
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Alert {
    pub id: Uuid,
    pub service_name: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Incident {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub service_name: String,
    pub status: String,
    pub severity: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub timestamp: DateTime<Utc>,
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub error_rate: f64,
    pub throughput: f64,
    pub availability: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudget {
    pub service_name: String,
    pub slo_name: String,
    pub budget_percentage: f64,
    pub consumed_percentage: f64,
    pub remaining_percentage: f64,
    pub time_window: String,
    pub last_updated: DateTime<Utc>,
    pub burn_rate: f64,
    pub status: String, // "healthy", "warning", "critical", "exhausted"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloViolation {
    pub slo_id: Uuid,
    pub slo_name: String,
    pub service_name: String,
    pub violation_type: String,
    pub severity: String,
    pub description: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub service_name: String,
    pub status: String, // "healthy", "warning", "critical", "down"
    pub last_check: DateTime<Utc>,
    pub uptime_percentage: f64,
    pub response_time: f64,
    pub error_count: u64,
    pub health_score: f64,
    pub dependencies: Vec<ServiceDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependency {
    pub name: String,
    pub status: String,
    pub latency: f64,
    pub last_check: DateTime<Utc>,
}

// Request/Response models

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub metrics: HashMap<String, ServiceMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SloListResponse {
    pub slos: Vec<Slo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SloResponse {
    pub slo: Slo,
}

#[derive(Debug, Deserialize)]
pub struct CreateSloRequest {
    pub name: String,
    pub description: Option<String>,
    pub service_name: String,
    pub metric_name: String,
    pub target_percentage: f64,
    pub time_window: String,
    pub threshold_value: Option<f64>,
    pub operator: String,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSloRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_percentage: Option<f64>,
    pub time_window: Option<String>,
    pub threshold_value: Option<f64>,
    pub operator: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBudgetResponse {
    pub budgets: Vec<ErrorBudget>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceErrorBudgetResponse {
    pub service: String,
    pub budget: ErrorBudget,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertListResponse {
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertResponse {
    pub alert: Alert,
}

#[derive(Debug, Deserialize)]
pub struct CreateAlertRequest {
    pub service_name: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IncidentListResponse {
    pub incidents: Vec<Incident>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IncidentResponse {
    pub incident: Incident,
}

#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub title: String,
    pub description: Option<String>,
    pub service_name: String,
    pub severity: String,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub severity: Option<String>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealthResponse {
    pub services: HashMap<String, ServiceHealth>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceSpecificHealthResponse {
    pub service: String,
    pub health: ServiceHealth,
}

// Configuration models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloTarget {
    pub metric: String,
    pub operator: String,
    pub threshold: f64,
    pub time_window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub condition: String,
    pub severity: String,
    pub cooldown: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Duration {
    pub seconds: u64,
}

// Metrics collection models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeries {
    pub name: String,
    pub help: String,
    pub metric_type: String,
    pub points: Vec<MetricPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusMetric {
    pub name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: HashMap<String, String>,
}

// SLO calculation models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloCalculation {
    pub slo_id: Uuid,
    pub current_percentage: f64,
    pub target_percentage: f64,
    pub compliance: bool,
    pub calculated_at: DateTime<Utc>,
    pub data_points: u64,
    pub time_window_start: DateTime<Utc>,
    pub time_window_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRateAlert {
    pub service_name: String,
    pub slo_name: String,
    pub burn_rate: f64,
    pub threshold: f64,
    pub severity: String,
    pub time_window: String,
    pub detected_at: DateTime<Utc>,
}

// Query parameters

#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub duration: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceQuery {
    pub service: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub service: Option<String>,
    pub metric: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub step: Option<String>,
}

// Validation helpers

impl CreateSloRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("SLO name cannot be empty".to_string());
        }

        if self.service_name.is_empty() {
            return Err("Service name cannot be empty".to_string());
        }

        if self.metric_name.is_empty() {
            return Err("Metric name cannot be empty".to_string());
        }

        if !(0.0..=100.0).contains(&self.target_percentage) {
            return Err("Target percentage must be between 0 and 100".to_string());
        }

        if !["lt", "lte", "gt", "gte", "eq"].contains(&self.operator.as_str()) {
            return Err("Invalid operator. Must be one of: lt, lte, gt, gte, eq".to_string());
        }

        // Validate time window format (e.g., "30d", "7d", "24h", "1h")
        if !self.time_window.ends_with('d')
            && !self.time_window.ends_with('h')
            && !self.time_window.ends_with('m')
        {
            return Err(
                "Invalid time window format. Use format like '30d', '24h', '60m'".to_string(),
            );
        }

        Ok(())
    }
}

impl CreateAlertRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.service_name.is_empty() {
            return Err("Service name cannot be empty".to_string());
        }

        if self.alert_type.is_empty() {
            return Err("Alert type cannot be empty".to_string());
        }

        if !["low", "medium", "high", "critical"].contains(&self.severity.as_str()) {
            return Err(
                "Invalid severity. Must be one of: low, medium, high, critical".to_string(),
            );
        }

        if self.message.is_empty() {
            return Err("Alert message cannot be empty".to_string());
        }

        Ok(())
    }
}

impl CreateIncidentRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() {
            return Err("Incident title cannot be empty".to_string());
        }

        if self.service_name.is_empty() {
            return Err("Service name cannot be empty".to_string());
        }

        if !["low", "medium", "high", "critical"].contains(&self.severity.as_str()) {
            return Err(
                "Invalid severity. Must be one of: low, medium, high, critical".to_string(),
            );
        }

        Ok(())
    }
}

// Helper functions for status determination

impl ServiceHealth {
    pub fn determine_status(&self) -> String {
        if self.uptime_percentage < 95.0 {
            "critical".to_string()
        } else if self.uptime_percentage < 99.0 || self.response_time > 2000.0 {
            "warning".to_string()
        } else if self.uptime_percentage >= 99.9 {
            "healthy".to_string()
        } else {
            "warning".to_string()
        }
    }

    pub fn calculate_health_score(&self) -> f64 {
        let uptime_score = self.uptime_percentage / 100.0;
        let response_time_score = (2000.0 - self.response_time.min(2000.0)) / 2000.0;
        let error_score = if self.error_count == 0 {
            1.0
        } else {
            1.0 / (1.0 + self.error_count as f64 / 100.0)
        };

        (uptime_score * 0.4 + response_time_score * 0.4 + error_score * 0.2) * 100.0
    }
}

impl ErrorBudget {
    pub fn determine_status(&self) -> String {
        if self.remaining_percentage <= 0.0 {
            "exhausted".to_string()
        } else if self.remaining_percentage < 10.0 {
            "critical".to_string()
        } else if self.remaining_percentage < 25.0 {
            "warning".to_string()
        } else {
            "healthy".to_string()
        }
    }

    pub fn calculate_burn_rate(&self, time_window_hours: f64) -> f64 {
        if time_window_hours == 0.0 {
            return 0.0;
        }

        let consumed_in_window = self.consumed_percentage;
        let burn_rate = consumed_in_window / time_window_hours;

        burn_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_slo_request_validation() {
        let valid_request = CreateSloRequest {
            name: "API Availability".to_string(),
            description: Some("API should be available 99.9% of the time".to_string()),
            service_name: "api-gateway".to_string(),
            metric_name: "availability".to_string(),
            target_percentage: 99.9,
            time_window: "30d".to_string(),
            threshold_value: None,
            operator: "gte".to_string(),
            metadata: None,
        };

        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateSloRequest {
            name: "".to_string(),
            description: None,
            service_name: "api-gateway".to_string(),
            metric_name: "availability".to_string(),
            target_percentage: 150.0,
            time_window: "invalid".to_string(),
            threshold_value: None,
            operator: "invalid_op".to_string(),
            metadata: None,
        };

        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_service_health_status_determination() {
        let mut health = ServiceHealth {
            service_name: "test-service".to_string(),
            status: "".to_string(),
            last_check: Utc::now(),
            uptime_percentage: 99.95,
            response_time: 150.0,
            error_count: 5,
            health_score: 0.0,
            dependencies: vec![],
        };

        assert_eq!(health.determine_status(), "healthy");

        health.uptime_percentage = 98.0;
        assert_eq!(health.determine_status(), "warning");

        health.uptime_percentage = 90.0;
        assert_eq!(health.determine_status(), "critical");
    }

    #[test]
    fn test_error_budget_status_determination() {
        let mut budget = ErrorBudget {
            service_name: "test-service".to_string(),
            slo_name: "availability".to_string(),
            budget_percentage: 1.0,
            consumed_percentage: 0.5,
            remaining_percentage: 0.5,
            time_window: "30d".to_string(),
            last_updated: Utc::now(),
            burn_rate: 0.01,
            status: "".to_string(),
        };

        assert_eq!(budget.determine_status(), "healthy");

        budget.remaining_percentage = 15.0;
        assert_eq!(budget.determine_status(), "warning");

        budget.remaining_percentage = 5.0;
        assert_eq!(budget.determine_status(), "critical");

        budget.remaining_percentage = 0.0;
        assert_eq!(budget.determine_status(), "exhausted");
    }
}
