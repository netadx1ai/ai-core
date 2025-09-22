//! ClickHouse Analytics Module for AI-CORE Platform
//!
//! This module provides high-level analytics operations using ClickHouse
//! for real-time data processing, event tracking, and performance analytics.

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::connections::clickhouse::{ApiRequest, ClickHouseStats, SystemMetric, WorkflowEvent};
use crate::connections::ClickHouseConnection;
use crate::DatabaseError;

/// Analytics manager for ClickHouse operations
pub struct AnalyticsManager {
    connection: Arc<ClickHouseConnection>,
}

impl AnalyticsManager {
    /// Create new analytics manager
    pub fn new(connection: Arc<ClickHouseConnection>) -> Self {
        Self { connection }
    }

    /// Track workflow execution event
    pub async fn track_workflow_event(
        &self,
        workflow_id: &str,
        user_id: &str,
        service_name: &str,
        event_type: WorkflowEventType,
        duration_ms: u64,
        cost_usd: f64,
        success: bool,
        metadata: Option<WorkflowEventMetadata>,
    ) -> Result<(), DatabaseError> {
        let event = WorkflowEvent {
            event_id: Uuid::new_v4().to_string(),
            workflow_id: workflow_id.to_string(),
            user_id: user_id.to_string(),
            service_name: service_name.to_string(),
            event_type: event_type.to_string(),
            event_category: "workflow".to_string(),
            duration_ms,
            cost_usd,
            success,
            error_code: metadata
                .as_ref()
                .and_then(|m| m.error_code.clone())
                .unwrap_or_default(),
            error_message: metadata
                .as_ref()
                .and_then(|m| m.error_message.clone())
                .unwrap_or_default(),
            provider_id: metadata
                .as_ref()
                .and_then(|m| m.provider_id.clone())
                .unwrap_or_default(),
            mcp_server_id: metadata
                .as_ref()
                .and_then(|m| m.mcp_server_id.clone())
                .unwrap_or_default(),
            request_size: metadata.as_ref().map(|m| m.request_size).unwrap_or(0),
            response_size: metadata.as_ref().map(|m| m.response_size).unwrap_or(0),
            timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        self.connection
            .bulk_insert("workflow_events", vec![event])
            .await
            .map(|_| ())
    }

    /// Track API request
    pub async fn track_api_request(
        &self,
        user_id: &str,
        endpoint: &str,
        method: &str,
        status_code: u16,
        response_time_ms: u32,
        request_size: u32,
        response_size: u32,
        ip_address: &str,
        success: bool,
    ) -> Result<(), DatabaseError> {
        let request = ApiRequest {
            request_id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            api_key_prefix: "".to_string(), // Would be set from actual API key
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            status_code,
            response_time_ms,
            request_size,
            response_size,
            ip_address: ip_address.to_string(),
            user_agent: "".to_string(), // Would be set from request headers
            rate_limit_remaining: 0,    // Would be calculated from rate limiter
            success,
            error_type: if success {
                "".to_string()
            } else {
                "http_error".to_string()
            },
            timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        self.connection
            .bulk_insert("api_requests", vec![request])
            .await
            .map(|_| ())
    }

    /// Track system metric
    pub async fn track_system_metric(
        &self,
        service_name: &str,
        metric_name: &str,
        metric_type: SystemMetricType,
        value: f64,
    ) -> Result<(), DatabaseError> {
        let metric = SystemMetric {
            metric_id: Uuid::new_v4().to_string(),
            service_name: service_name.to_string(),
            metric_name: metric_name.to_string(),
            metric_type: metric_type.to_string(),
            value,
            timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        self.connection
            .bulk_insert("system_metrics", vec![metric])
            .await
            .map(|_| ())
    }

    /// Batch insert workflow events for high performance
    pub async fn batch_track_workflow_events(
        &self,
        events: Vec<WorkflowEventData>,
    ) -> Result<u64, DatabaseError> {
        let workflow_events: Vec<WorkflowEvent> = events
            .into_iter()
            .map(|data| WorkflowEvent {
                event_id: Uuid::new_v4().to_string(),
                workflow_id: data.workflow_id,
                user_id: data.user_id,
                service_name: data.service_name,
                event_type: data.event_type.to_string(),
                event_category: "workflow".to_string(),
                duration_ms: data.duration_ms,
                cost_usd: data.cost_usd,
                success: data.success,
                error_code: data
                    .metadata
                    .as_ref()
                    .and_then(|m| m.error_code.clone())
                    .unwrap_or_default(),
                error_message: data
                    .metadata
                    .as_ref()
                    .and_then(|m| m.error_message.clone())
                    .unwrap_or_default(),
                provider_id: data
                    .metadata
                    .as_ref()
                    .and_then(|m| m.provider_id.clone())
                    .unwrap_or_default(),
                mcp_server_id: data
                    .metadata
                    .as_ref()
                    .and_then(|m| m.mcp_server_id.clone())
                    .unwrap_or_default(),
                request_size: data.metadata.as_ref().map(|m| m.request_size).unwrap_or(0),
                response_size: data.metadata.as_ref().map(|m| m.response_size).unwrap_or(0),
                timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            })
            .collect();

        self.connection
            .bulk_insert("workflow_events", workflow_events)
            .await
    }

    /// Get real-time workflow metrics
    pub async fn get_workflow_metrics(
        &self,
        time_range: TimeRange,
        service_name: Option<&str>,
    ) -> Result<WorkflowMetrics, DatabaseError> {
        let service_filter = service_name
            .map(|s| format!("AND service_name = '{}'", s))
            .unwrap_or_default();

        let sql = format!(
            r#"
            SELECT
                count() as total_events,
                countIf(success = true) as success_count,
                countIf(success = false) as error_count,
                avg(duration_ms) as avg_duration_ms,
                quantile(0.5)(duration_ms) as median_duration_ms,
                quantile(0.95)(duration_ms) as p95_duration_ms,
                quantile(0.99)(duration_ms) as p99_duration_ms,
                sum(cost_usd) as total_cost_usd,
                avg(cost_usd) as avg_cost_usd
            FROM workflow_events
            WHERE timestamp >= '{}' AND timestamp <= '{}'
            {}
            "#,
            time_range.start_time.format("%Y-%m-%d %H:%M:%S"),
            time_range.end_time.format("%Y-%m-%d %H:%M:%S"),
            service_filter
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct MetricsRow {
            total_events: u64,
            success_count: u64,
            error_count: u64,
            avg_duration_ms: f64,
            median_duration_ms: f64,
            p95_duration_ms: f64,
            p99_duration_ms: f64,
            total_cost_usd: f64,
            avg_cost_usd: f64,
        }

        let result: MetricsRow = self.connection.query_one(&sql).await?;

        Ok(WorkflowMetrics {
            total_events: result.total_events,
            success_count: result.success_count,
            error_count: result.error_count,
            success_rate: if result.total_events > 0 {
                result.success_count as f64 / result.total_events as f64 * 100.0
            } else {
                0.0
            },
            avg_duration_ms: result.avg_duration_ms,
            median_duration_ms: result.median_duration_ms,
            p95_duration_ms: result.p95_duration_ms,
            p99_duration_ms: result.p99_duration_ms,
            total_cost_usd: result.total_cost_usd,
            avg_cost_usd: result.avg_cost_usd,
        })
    }

    /// Get API performance metrics
    pub async fn get_api_metrics(
        &self,
        time_range: TimeRange,
        endpoint: Option<&str>,
    ) -> Result<ApiMetrics, DatabaseError> {
        let endpoint_filter = endpoint
            .map(|e| format!("AND endpoint = '{}'", e))
            .unwrap_or_default();

        let sql = format!(
            r#"
            SELECT
                count() as total_requests,
                countIf(success = true) as success_count,
                countIf(success = false) as error_count,
                avg(response_time_ms) as avg_response_time_ms,
                quantile(0.5)(response_time_ms) as median_response_time_ms,
                quantile(0.95)(response_time_ms) as p95_response_time_ms,
                quantile(0.99)(response_time_ms) as p99_response_time_ms,
                sum(request_size + response_size) as total_bytes,
                avg(request_size + response_size) as avg_bytes_per_request
            FROM api_requests
            WHERE timestamp >= '{}' AND timestamp <= '{}'
            {}
            "#,
            time_range.start_time.format("%Y-%m-%d %H:%M:%S"),
            time_range.end_time.format("%Y-%m-%d %H:%M:%S"),
            endpoint_filter
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct ApiMetricsRow {
            total_requests: u64,
            success_count: u64,
            error_count: u64,
            avg_response_time_ms: f64,
            median_response_time_ms: f64,
            p95_response_time_ms: f64,
            p99_response_time_ms: f64,
            total_bytes: u64,
            avg_bytes_per_request: f64,
        }

        let result: ApiMetricsRow = self.connection.query_one(&sql).await?;

        Ok(ApiMetrics {
            total_requests: result.total_requests,
            success_count: result.success_count,
            error_count: result.error_count,
            success_rate: if result.total_requests > 0 {
                result.success_count as f64 / result.total_requests as f64 * 100.0
            } else {
                0.0
            },
            avg_response_time_ms: result.avg_response_time_ms,
            median_response_time_ms: result.median_response_time_ms,
            p95_response_time_ms: result.p95_response_time_ms,
            p99_response_time_ms: result.p99_response_time_ms,
            total_bytes: result.total_bytes,
            avg_bytes_per_request: result.avg_bytes_per_request,
        })
    }

    /// Get top users by activity
    pub async fn get_top_users(
        &self,
        time_range: TimeRange,
        limit: u32,
    ) -> Result<Vec<UserActivity>, DatabaseError> {
        let sql = format!(
            r#"
            SELECT
                user_id,
                count() as total_events,
                countIf(success = true) as success_count,
                countIf(success = false) as error_count,
                sum(cost_usd) as total_cost_usd,
                avg(duration_ms) as avg_duration_ms
            FROM workflow_events
            WHERE timestamp >= '{}' AND timestamp <= '{}'
            GROUP BY user_id
            ORDER BY total_events DESC
            LIMIT {}
            "#,
            time_range.start_time.format("%Y-%m-%d %H:%M:%S"),
            time_range.end_time.format("%Y-%m-%d %H:%M:%S"),
            limit
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct UserActivityRow {
            user_id: String,
            total_events: u64,
            success_count: u64,
            error_count: u64,
            total_cost_usd: f64,
            avg_duration_ms: f64,
        }

        let results: Vec<UserActivityRow> = self.connection.query(&sql).await?;

        Ok(results
            .into_iter()
            .map(|row| UserActivity {
                user_id: row.user_id,
                total_events: row.total_events,
                success_count: row.success_count,
                error_count: row.error_count,
                success_rate: if row.total_events > 0 {
                    row.success_count as f64 / row.total_events as f64 * 100.0
                } else {
                    0.0
                },
                total_cost_usd: row.total_cost_usd,
                avg_duration_ms: row.avg_duration_ms,
            })
            .collect())
    }

    /// Get system performance overview
    pub async fn get_system_overview(
        &self,
        time_range: TimeRange,
    ) -> Result<SystemOverview, DatabaseError> {
        // Get service performance
        let service_sql = format!(
            r#"
            SELECT
                service_name,
                count() as request_count,
                countIf(success = true) as success_count,
                avg(duration_ms) as avg_duration_ms,
                sum(cost_usd) as total_cost_usd
            FROM workflow_events
            WHERE timestamp >= '{}' AND timestamp <= '{}'
            GROUP BY service_name
            ORDER BY request_count DESC
            "#,
            time_range.start_time.format("%Y-%m-%d %H:%M:%S"),
            time_range.end_time.format("%Y-%m-%d %H:%M:%S")
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct ServicePerformanceRow {
            service_name: String,
            request_count: u64,
            success_count: u64,
            avg_duration_ms: f64,
            total_cost_usd: f64,
        }

        let service_results: Vec<ServicePerformanceRow> =
            self.connection.query(&service_sql).await?;

        let service_performance: Vec<ServicePerformance> = service_results
            .into_iter()
            .map(|row| ServicePerformance {
                service_name: row.service_name,
                request_count: row.request_count,
                success_count: row.success_count,
                success_rate: if row.request_count > 0 {
                    row.success_count as f64 / row.request_count as f64 * 100.0
                } else {
                    0.0
                },
                avg_duration_ms: row.avg_duration_ms,
                total_cost_usd: row.total_cost_usd,
            })
            .collect();

        // Get error summary
        let error_sql = format!(
            r#"
            SELECT
                error_code,
                count() as error_count,
                service_name
            FROM workflow_events
            WHERE timestamp >= '{}' AND timestamp <= '{}'
            AND success = false AND error_code != ''
            GROUP BY error_code, service_name
            ORDER BY error_count DESC
            LIMIT 20
            "#,
            time_range.start_time.format("%Y-%m-%d %H:%M:%S"),
            time_range.end_time.format("%Y-%m-%d %H:%M:%S")
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct ErrorSummaryRow {
            error_code: String,
            error_count: u64,
            service_name: String,
        }

        let error_results: Vec<ErrorSummaryRow> = self.connection.query(&error_sql).await?;

        let error_summary: Vec<ErrorSummary> = error_results
            .into_iter()
            .map(|row| ErrorSummary {
                error_code: row.error_code,
                error_count: row.error_count,
                service_name: row.service_name,
            })
            .collect();

        let total_services = service_performance.len() as u32;
        let total_requests = service_performance.iter().map(|s| s.request_count).sum();
        let total_cost_usd = service_performance.iter().map(|s| s.total_cost_usd).sum();

        Ok(SystemOverview {
            service_performance,
            error_summary,
            total_services,
            total_requests,
            total_cost_usd,
        })
    }

    /// Create real-time dashboard materialized view
    pub async fn create_dashboard_views(&self) -> Result<(), DatabaseError> {
        // Real-time workflow metrics (1-minute granularity)
        let workflow_view_sql = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_workflow_dashboard_1min
            ENGINE = ReplacingMergeTree()
            ORDER BY (service_name, event_type, timestamp)
            POPULATE AS
            SELECT
                service_name,
                event_type,
                toStartOfMinute(parseDateTimeBestEffort(timestamp)) as timestamp,
                count() as event_count,
                countIf(success = true) as success_count,
                countIf(success = false) as error_count,
                avg(duration_ms) as avg_duration_ms,
                quantile(0.95)(duration_ms) as p95_duration_ms,
                sum(cost_usd) as total_cost_usd
            FROM workflow_events
            WHERE timestamp >= now() - INTERVAL 1 HOUR
            GROUP BY service_name, event_type, toStartOfMinute(parseDateTimeBestEffort(timestamp))
        "#;

        self.connection.execute(workflow_view_sql).await?;

        // Real-time API metrics (1-minute granularity)
        let api_view_sql = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_api_dashboard_1min
            ENGINE = ReplacingMergeTree()
            ORDER BY (endpoint, method, timestamp)
            POPULATE AS
            SELECT
                endpoint,
                method,
                toStartOfMinute(parseDateTimeBestEffort(timestamp)) as timestamp,
                count() as request_count,
                countIf(success = true) as success_count,
                countIf(success = false) as error_count,
                avg(response_time_ms) as avg_response_time_ms,
                quantile(0.95)(response_time_ms) as p95_response_time_ms,
                sum(request_size + response_size) as total_bytes
            FROM api_requests
            WHERE timestamp >= now() - INTERVAL 1 HOUR
            GROUP BY endpoint, method, toStartOfMinute(parseDateTimeBestEffort(timestamp))
        "#;

        self.connection.execute(api_view_sql).await?;

        tracing::info!("Dashboard materialized views created successfully");
        Ok(())
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> ClickHouseStats {
        self.connection.get_stats().await
    }

    /// Optimize analytics tables for better performance
    pub async fn optimize_tables(&self) -> Result<(), DatabaseError> {
        let tables = vec![
            "workflow_events",
            "api_requests",
            "system_metrics",
            "content_events",
            "user_events",
            "error_events",
            "cost_events",
        ];

        for table in tables {
            self.connection.optimize_table(table).await?;
        }

        tracing::info!("Analytics tables optimization completed");
        Ok(())
    }
}

/// Workflow event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEventType {
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowCancelled,
    StepStarted,
    StepCompleted,
    StepFailed,
}

impl ToString for WorkflowEventType {
    fn to_string(&self) -> String {
        match self {
            WorkflowEventType::WorkflowStarted => "workflow_started".to_string(),
            WorkflowEventType::WorkflowCompleted => "workflow_completed".to_string(),
            WorkflowEventType::WorkflowFailed => "workflow_failed".to_string(),
            WorkflowEventType::WorkflowCancelled => "workflow_cancelled".to_string(),
            WorkflowEventType::StepStarted => "step_started".to_string(),
            WorkflowEventType::StepCompleted => "step_completed".to_string(),
            WorkflowEventType::StepFailed => "step_failed".to_string(),
        }
    }
}

/// System metric types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

impl ToString for SystemMetricType {
    fn to_string(&self) -> String {
        match self {
            SystemMetricType::Counter => "counter".to_string(),
            SystemMetricType::Gauge => "gauge".to_string(),
            SystemMetricType::Histogram => "histogram".to_string(),
            SystemMetricType::Summary => "summary".to_string(),
        }
    }
}

/// Workflow event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventMetadata {
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub provider_id: Option<String>,
    pub mcp_server_id: Option<String>,
    pub request_size: u32,
    pub response_size: u32,
}

/// Workflow event data for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventData {
    pub workflow_id: String,
    pub user_id: String,
    pub service_name: String,
    pub event_type: WorkflowEventType,
    pub duration_ms: u64,
    pub cost_usd: f64,
    pub success: bool,
    pub metadata: Option<WorkflowEventMetadata>,
}

/// Time range for analytics queries
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl TimeRange {
    pub fn last_hour() -> Self {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(1);
        Self {
            start_time,
            end_time,
        }
    }

    pub fn last_24_hours() -> Self {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(24);
        Self {
            start_time,
            end_time,
        }
    }

    pub fn last_7_days() -> Self {
        let end_time = Utc::now();
        let start_time = end_time - Duration::days(7);
        Self {
            start_time,
            end_time,
        }
    }

    pub fn last_30_days() -> Self {
        let end_time = Utc::now();
        let start_time = end_time - Duration::days(30);
        Self {
            start_time,
            end_time,
        }
    }

    pub fn custom(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            start_time,
            end_time,
        }
    }
}

/// Workflow metrics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub total_events: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub median_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub total_cost_usd: f64,
    pub avg_cost_usd: f64,
}

/// API metrics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub median_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub total_bytes: u64,
    pub avg_bytes_per_request: f64,
}

/// User activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    pub user_id: String,
    pub total_events: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub total_cost_usd: f64,
    pub avg_duration_ms: f64,
}

/// Service performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePerformance {
    pub service_name: String,
    pub request_count: u64,
    pub success_count: u64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub total_cost_usd: f64,
}

/// Error summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub error_code: String,
    pub error_count: u64,
    pub service_name: String,
}

/// System overview result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOverview {
    pub service_performance: Vec<ServicePerformance>,
    pub error_summary: Vec<ErrorSummary>,
    pub total_services: u32,
    pub total_requests: u64,
    pub total_cost_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_event_type_to_string() {
        assert_eq!(
            WorkflowEventType::WorkflowStarted.to_string(),
            "workflow_started"
        );
        assert_eq!(
            WorkflowEventType::WorkflowCompleted.to_string(),
            "workflow_completed"
        );
        assert_eq!(WorkflowEventType::StepFailed.to_string(), "step_failed");
    }

    #[test]
    fn test_system_metric_type_to_string() {
        assert_eq!(SystemMetricType::Counter.to_string(), "counter");
        assert_eq!(SystemMetricType::Gauge.to_string(), "gauge");
        assert_eq!(SystemMetricType::Histogram.to_string(), "histogram");
    }

    #[test]
    fn test_time_range_creation() {
        let range = TimeRange::last_hour();
        assert!(range.end_time > range.start_time);

        let duration = range.end_time - range.start_time;
        assert!(duration.num_hours() <= 1);
    }

    #[test]
    fn test_workflow_event_metadata() {
        let metadata = WorkflowEventMetadata {
            error_code: Some("RATE_LIMIT".to_string()),
            error_message: Some("Rate limit exceeded".to_string()),
            provider_id: Some("openai".to_string()),
            mcp_server_id: Some("mcp-1".to_string()),
            request_size: 1024,
            response_size: 2048,
        };

        assert_eq!(metadata.error_code.unwrap(), "RATE_LIMIT");
        assert_eq!(metadata.request_size, 1024);
    }
}
