use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{
    CreateSloRequest, Slo, SloViolation, ServiceMetrics, UpdateSloRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloTarget {
    pub metric_name: String,
    pub operator: String,
    pub threshold_value: f64,
    pub target_percentage: f64,
    pub time_window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloCalculationResult {
    pub slo_id: Uuid,
    pub current_percentage: f64,
    pub target_percentage: f64,
    pub compliance: bool,
    pub data_points: u64,
    pub time_window_start: DateTime<Utc>,
    pub time_window_end: DateTime<Utc>,
    pub calculated_at: DateTime<Utc>,
}

pub struct SloValidator {
    db: PgPool,
}

impl SloValidator {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_slo(&self, request: CreateSloRequest) -> Result<Slo> {
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        let slo_id = Uuid::new_v4();
        let now = Utc::now();

        let slo = sqlx::query_as!(
            Slo,
            r#"
            INSERT INTO slos (id, name, description, service_name, metric_name, target_percentage,
                             time_window, threshold_value, operator, status, created_at, updated_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10, $11, $12)
            RETURNING id, name, description, service_name, metric_name, target_percentage,
                      time_window, threshold_value, operator, status, created_at, updated_at, metadata
            "#,
            slo_id,
            request.name,
            request.description,
            request.service_name,
            request.metric_name,
            request.target_percentage,
            request.time_window,
            request.threshold_value,
            request.operator,
            now,
            now,
            request.metadata
        )
        .fetch_one(&self.db)
        .await?;

        info!("Created new SLO: {} for service {}", slo.name, slo.service_name);
        Ok(slo)
    }

    pub async fn update_slo(&self, slo_id: Uuid, request: UpdateSloRequest) -> Result<Slo> {
        let now = Utc::now();

        let slo = sqlx::query_as!(
            Slo,
            r#"
            UPDATE slos
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                target_percentage = COALESCE($4, target_percentage),
                time_window = COALESCE($5, time_window),
                threshold_value = COALESCE($6, threshold_value),
                operator = COALESCE($7, operator),
                status = COALESCE($8, status),
                updated_at = $9,
                metadata = COALESCE($10, metadata)
            WHERE id = $1
            RETURNING id, name, description, service_name, metric_name, target_percentage,
                      time_window, threshold_value, operator, status, created_at, updated_at, metadata
            "#,
            slo_id,
            request.name,
            request.description,
            request.target_percentage,
            request.time_window,
            request.threshold_value,
            request.operator,
            request.status,
            now,
            request.metadata
        )
        .fetch_optional(&self.db)
        .await?;

        match slo {
            Some(slo) => {
                info!("Updated SLO: {} for service {}", slo.name, slo.service_name);
                Ok(slo)
            }
            None => Err(anyhow::anyhow!("SLO with id {} not found", slo_id)),
        }
    }

    pub async fn get_slo_by_id(&self, slo_id: Uuid) -> Result<Option<Slo>> {
        let slo = sqlx::query_as!(
            Slo,
            r#"
            SELECT id, name, description, service_name, metric_name, target_percentage,
                   time_window, threshold_value, operator, status, created_at, updated_at, metadata
            FROM slos
            WHERE id = $1
            "#,
            slo_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(slo)
    }

    pub async fn get_slos(&self, service_name: Option<&str>) -> Result<Vec<Slo>> {
        let slos = match service_name {
            Some(service) => {
                sqlx::query_as!(
                    Slo,
                    r#"
                    SELECT id, name, description, service_name, metric_name, target_percentage,
                           time_window, threshold_value, operator, status, created_at, updated_at, metadata
                    FROM slos
                    WHERE service_name = $1
                    ORDER BY created_at DESC
                    "#,
                    service
                )
                .fetch_all(&self.db)
                .await?
            }
            None => {
                sqlx::query_as!(
                    Slo,
                    r#"
                    SELECT id, name, description, service_name, metric_name, target_percentage,
                           time_window, threshold_value, operator, status, created_at, updated_at, metadata
                    FROM slos
                    ORDER BY service_name, created_at DESC
                    "#
                )
                .fetch_all(&self.db)
                .await?
            }
        };

        Ok(slos)
    }

    pub async fn validate_all_slos(&self) -> Result<()> {
        info!("Starting SLO validation cycle");

        let active_slos = sqlx::query_as!(
            Slo,
            r#"
            SELECT id, name, description, service_name, metric_name, target_percentage,
                   time_window, threshold_value, operator, status, created_at, updated_at, metadata
            FROM slos
            WHERE status = 'active'
            "#
        )
        .fetch_all(&self.db)
        .await?;

        let mut validation_results = Vec::new();

        for slo in active_slos {
            match self.validate_slo(&slo).await {
                Ok(result) => {
                    validation_results.push(result);

                    // Store the validation result
                    if let Err(e) = self.store_slo_calculation(&result).await {
                        error!("Failed to store SLO calculation for {}: {}", slo.name, e);
                    }
                }
                Err(e) => {
                    error!("Failed to validate SLO {} for service {}: {}",
                           slo.name, slo.service_name, e);
                }
            }
        }

        info!("SLO validation completed. Validated {} SLOs", validation_results.len());
        Ok(())
    }

    async fn validate_slo(&self, slo: &Slo) -> Result<SloCalculationResult> {
        let time_window_duration = self.parse_time_window(&slo.time_window)?;
        let end_time = Utc::now();
        let start_time = end_time - time_window_duration;

        // Get metrics for the time window
        let metrics = self.get_metrics_for_slo(slo, start_time, end_time).await?;

        if metrics.is_empty() {
            warn!("No metrics found for SLO {} in time window {}", slo.name, slo.time_window);

            return Ok(SloCalculationResult {
                slo_id: slo.id,
                current_percentage: 0.0,
                target_percentage: slo.target_percentage,
                compliance: false,
                data_points: 0,
                time_window_start: start_time,
                time_window_end: end_time,
                calculated_at: Utc::now(),
            });
        }

        // Calculate SLO compliance based on the metric and operator
        let current_percentage = self.calculate_slo_percentage(slo, &metrics)?;
        let compliance = self.check_slo_compliance(slo, current_percentage);

        Ok(SloCalculationResult {
            slo_id: slo.id,
            current_percentage,
            target_percentage: slo.target_percentage,
            compliance,
            data_points: metrics.len() as u64,
            time_window_start: start_time,
            time_window_end: end_time,
            calculated_at: Utc::now(),
        })
    }

    async fn get_metrics_for_slo(
        &self,
        slo: &Slo,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<ServiceMetrics>> {
        let rows = sqlx::query!(
            r#"
            SELECT service_name, timestamp, latency_p50, latency_p95, latency_p99,
                   error_rate, throughput, availability, cpu_usage, memory_usage, disk_usage
            FROM service_metrics
            WHERE service_name = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
            slo.service_name,
            start_time,
            end_time
        )
        .fetch_all(&self.db)
        .await?;

        let metrics = rows.into_iter().map(|row| ServiceMetrics {
            service_name: row.service_name,
            timestamp: row.timestamp,
            latency_p50: row.latency_p50,
            latency_p95: row.latency_p95,
            latency_p99: row.latency_p99,
            error_rate: row.error_rate,
            throughput: row.throughput,
            availability: row.availability,
            cpu_usage: row.cpu_usage,
            memory_usage: row.memory_usage,
            disk_usage: row.disk_usage,
        }).collect();

        Ok(metrics)
    }

    fn calculate_slo_percentage(&self, slo: &Slo, metrics: &[ServiceMetrics]) -> Result<f64> {
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let values: Vec<f64> = metrics.iter().map(|m| {
            match slo.metric_name.as_str() {
                "availability" => m.availability,
                "latency_p50" => m.latency_p50,
                "latency_p95" => m.latency_p95,
                "latency_p99" => m.latency_p99,
                "error_rate" => m.error_rate,
                "throughput" => m.throughput,
                "cpu_usage" => m.cpu_usage,
                "memory_usage" => m.memory_usage,
                "disk_usage" => m.disk_usage,
                _ => 0.0,
            }
        }).collect();

        if values.is_empty() {
            return Ok(0.0);
        }

        // Calculate the percentage based on the SLO operator and threshold
        let threshold = slo.threshold_value.unwrap_or(0.0);

        let compliant_count = values.iter().filter(|&&value| {
            match slo.operator.as_str() {
                "gte" => value >= threshold,
                "gt" => value > threshold,
                "lte" => value <= threshold,
                "lt" => value < threshold,
                "eq" => (value - threshold).abs() < 0.01,
                _ => false,
            }
        }).count();

        let percentage = (compliant_count as f64 / values.len() as f64) * 100.0;
        Ok(percentage)
    }

    fn check_slo_compliance(&self, slo: &Slo, current_percentage: f64) -> bool {
        current_percentage >= slo.target_percentage
    }

    async fn store_slo_calculation(&self, result: &SloCalculationResult) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO slo_calculations
            (slo_id, current_percentage, target_percentage, compliance, data_points,
             time_window_start, time_window_end, calculated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            result.slo_id,
            result.current_percentage,
            result.target_percentage,
            result.compliance,
            result.data_points as i64,
            result.time_window_start,
            result.time_window_end,
            result.calculated_at
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn check_violations(&self) -> Result<Vec<SloViolation>> {
        let mut violations = Vec::new();

        // Get recent SLO calculations that are not compliant
        let non_compliant = sqlx::query!(
            r#"
            SELECT sc.slo_id, sc.current_percentage, sc.target_percentage,
                   s.name, s.service_name, s.metric_name, s.threshold_value,
                   sc.calculated_at
            FROM slo_calculations sc
            JOIN slos s ON sc.slo_id = s.id
            WHERE sc.compliance = false
            AND sc.calculated_at > NOW() - INTERVAL '1 hour'
            AND s.status = 'active'
            ORDER BY sc.calculated_at DESC
            "#
        )
        .fetch_all(&self.db)
        .await?;

        for row in non_compliant {
            let severity = self.determine_violation_severity(
                row.current_percentage,
                row.target_percentage,
            );

            let description = format!(
                "SLO '{}' violation: Current {}% vs Target {}%",
                row.name,
                row.current_percentage,
                row.target_percentage
            );

            violations.push(SloViolation {
                slo_id: row.slo_id,
                slo_name: row.name,
                service_name: row.service_name,
                violation_type: "threshold_breach".to_string(),
                severity,
                description,
                current_value: row.current_percentage,
                threshold_value: row.target_percentage,
                timestamp: row.calculated_at,
            });
        }

        // Check for burn rate violations
        let burn_rate_violations = self.check_burn_rate_violations().await?;
        violations.extend(burn_rate_violations);

        Ok(violations)
    }

    async fn check_burn_rate_violations(&self) -> Result<Vec<SloViolation>> {
        let mut violations = Vec::new();

        // Get SLOs with recent rapid degradation
        let rapid_degradation = sqlx::query!(
            r#"
            WITH recent_calculations AS (
                SELECT slo_id, current_percentage, calculated_at,
                       LAG(current_percentage) OVER (PARTITION BY slo_id ORDER BY calculated_at) as prev_percentage,
                       LAG(calculated_at) OVER (PARTITION BY slo_id ORDER BY calculated_at) as prev_calculated_at
                FROM slo_calculations
                WHERE calculated_at > NOW() - INTERVAL '6 hours'
            ),
            burn_rates AS (
                SELECT rc.slo_id, rc.current_percentage, rc.calculated_at,
                       CASE
                           WHEN rc.prev_percentage IS NOT NULL AND rc.prev_calculated_at IS NOT NULL
                           THEN (rc.prev_percentage - rc.current_percentage) /
                                EXTRACT(EPOCH FROM (rc.calculated_at - rc.prev_calculated_at)) * 3600
                           ELSE 0
                       END as burn_rate_per_hour
                FROM recent_calculations rc
                WHERE rc.prev_percentage IS NOT NULL
            )
            SELECT br.slo_id, br.current_percentage, br.burn_rate_per_hour, br.calculated_at,
                   s.name, s.service_name, s.target_percentage
            FROM burn_rates br
            JOIN slos s ON br.slo_id = s.id
            WHERE br.burn_rate_per_hour > 5.0  -- More than 5% degradation per hour
            AND s.status = 'active'
            ORDER BY br.burn_rate_per_hour DESC
            "#
        )
        .fetch_all(&self.db)
        .await?;

        for row in rapid_degradation {
            let severity = if row.burn_rate_per_hour.unwrap_or(0.0) > 20.0 {
                "critical"
            } else if row.burn_rate_per_hour.unwrap_or(0.0) > 10.0 {
                "high"
            } else {
                "medium"
            };

            let description = format!(
                "High burn rate detected for SLO '{}': {:.2}% degradation per hour",
                row.name,
                row.burn_rate_per_hour.unwrap_or(0.0)
            );

            violations.push(SloViolation {
                slo_id: row.slo_id,
                slo_name: row.name,
                service_name: row.service_name,
                violation_type: "burn_rate".to_string(),
                severity: severity.to_string(),
                description,
                current_value: row.current_percentage,
                threshold_value: row.target_percentage,
                timestamp: row.calculated_at,
            });
        }

        Ok(violations)
    }

    fn determine_violation_severity(&self, current: f64, target: f64) -> String {
        let difference = target - current;

        if difference > 10.0 {
            "critical".to_string()
        } else if difference > 5.0 {
            "high".to_string()
        } else if difference > 1.0 {
            "medium".to_string()
        } else {
            "low".to_string()
        }
    }

    fn parse_time_window(&self, time_window: &str) -> Result<ChronoDuration> {
        let (num_str, unit) = if let Some(pos) = time_window.find(char::is_alphabetic) {
            time_window.split_at(pos)
        } else {
            return Err(anyhow::anyhow!("Invalid time window format: {}", time_window));
        };

        let num: i64 = num_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid number in time window: {}", num_str))?;

        match unit {
            "m" => Ok(ChronoDuration::minutes(num)),
            "h" => Ok(ChronoDuration::hours(num)),
            "d" => Ok(ChronoDuration::days(num)),
            "w" => Ok(ChronoDuration::weeks(num)),
            _ => Err(anyhow::anyhow!("Invalid time unit: {}", unit))
        }
    }

    pub async fn get_slo_history(
        &self,
        slo_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<SloCalculationResult>> {
        let rows = sqlx::query!(
            r#"
            SELECT slo_id, current_percentage, target_percentage, compliance, data_points,
                   time_window_start, time_window_end, calculated_at
            FROM slo_calculations
            WHERE slo_id = $1 AND calculated_at >= $2 AND calculated_at <= $3
            ORDER BY calculated_at ASC
            "#,
            slo_id,
            start_time,
            end_time
        )
        .fetch_all(&self.db)
        .await?;

        let history = rows.into_iter().map(|row| SloCalculationResult {
            slo_id: row.slo_id,
            current_percentage: row.current_percentage,
            target_percentage: row.target_percentage,
            compliance: row.compliance,
            data_points: row.data_points as u64,
            time_window_start: row.time_window_start,
            time_window_end: row.time_window_end,
            calculated_at: row.calculated_at,
        }).collect();

        Ok(history)
    }

    pub async fn get_slo_compliance_summary(&self, service_name: Option<&str>) -> Result<HashMap<String, f64>> {
        let mut summary = HashMap::new();

        let rows = match service_name {
            Some(service) => {
                sqlx::query!(
                    r#"
                    SELECT s.service_name, s.name as slo_name,
                           AVG(CASE WHEN sc.compliance THEN 100.0 ELSE 0.0 END) as compliance_percentage
                    FROM slos s
                    LEFT JOIN slo_calculations sc ON s.id = sc.slo_id
                        AND sc.calculated_at > NOW() - INTERVAL '7 days'
                    WHERE s.service_name = $1 AND s.status = 'active'
                    GROUP BY s.service_name, s.name
                    "#,
                    service
                )
                .fetch_all(&self.db)
                .await?
            }
            None => {
                sqlx::query!(
                    r#"
                    SELECT s.service_name, s.name as slo_name,
                           AVG(CASE WHEN sc.compliance THEN 100.0 ELSE 0.0 END) as compliance_percentage
                    FROM slos s
                    LEFT JOIN slo_calculations sc ON s.id = sc.slo_id
                        AND sc.calculated_at > NOW() - INTERVAL '7 days'
                    WHERE s.status = 'active'
                    GROUP BY s.service_name, s.name
                    "#
                )
                .fetch_all(&self.db)
                .await?
            }
        };

        for row in rows {
            let key = format!("{}:{}", row.service_name, row.slo_name);
            summary.insert(key, row.compliance_percentage.unwrap_or(0.0));
        }

        Ok(summary)
    }

    pub async fn cleanup_old_calculations(&self, retention_days: i64) -> Result<()> {
        let cutoff_date = Utc::now() - ChronoDuration::days(retention_days);

        let deleted = sqlx::query!(
            "DELETE FROM slo_calculations WHERE calculated_at < $1",
            cutoff_date
        )
        .execute(&self.db)
        .await?;

        info!("Cleaned up {} old SLO calculation records", deleted.rows_affected());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};

    // Note: These tests would require a test database setup
    // For now, they test the logic functions that don't require DB access

    #[test]
    fn test_parse_time_window() {
        let validator = SloValidator::new(PgPool::connect("").await.unwrap()); // Mock pool

        assert_eq!(validator.parse_time_window("30d").unwrap(), ChronoDuration::days(30));
        assert_eq!(validator.parse_time_window("24h").unwrap(), ChronoDuration::hours(24));
        assert_eq!(validator.parse_time_window("60m").unwrap(), ChronoDuration::minutes(60));
        assert_eq!(validator.parse_time_window("2w").unwrap(), ChronoDuration::weeks(2));

        assert!(validator.parse_time_window("invalid").is_err());
        assert!(validator.parse_time_window("30x").is_err());
    }

    #[test]
    fn test_determine_violation_severity() {
        let validator = SloValidator::new(PgPool::connect("").await.unwrap());

        assert_eq!(validator.determine_violation_severity(80.0, 95.0), "critical");
        assert_eq!(validator.determine_violation_severity(90.0, 95.0), "high");
        assert_eq!(validator.determine_violation_severity(93.0, 95.0), "medium");
        assert_eq!(validator.determine_violation_severity(94.5, 95.0), "low");
    }

    #[test]
    fn test_check_slo_compliance() {
        let validator = SloValidator::new(PgPool::connect("").await.unwrap());

        assert!(validator.check_slo_compliance(&create_test_slo(95.0), 96.0));
        assert!(!validator.check_slo_compliance(&create_test_slo(95.0), 94.0));
        assert!(validator.check_slo_compliance(&create_test_slo(95.0), 95.0));
    }

    fn create_test_slo(target_percentage: f64) -> Slo {
        Slo {
            id: Uuid::new_v4(),
            name: "Test SLO".to_string(),
            description: Some("Test SLO description".to_string()),
            service_name: "test-service".to_string(),
            metric_name: "availability".to_string(),
            target_percentage,
            time_window: "30d".to_string(),
            threshold_value: Some(99.0),
            operator: "gte".to_string(),
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: None,
        }
    }

    #[test]
    fn test_calculate_slo_percentage() {
        let validator = SloValidator::new(PgPool::connect("").await.unwrap());
        let slo = create_test_slo(99.0);

        let metrics = vec![
            ServiceMetrics {
                service_name: "test-service".to_string(),
                timestamp: Utc::now(),
                latency_p50: 100.0,
                latency_p95: 200.0,
                latency_p99: 300.0,
                error_rate: 0.5,
                throughput: 100.0,
                availability: 99.5,
                cpu_usage: 50.0,
                memory_usage: 1024.0,
                disk_usage: 20.0,
            },
            ServiceMetrics {
                service_name: "test-service".to_string(),
                timestamp: Utc::now(),
                latency_p50: 120.0,
                latency_p95: 250.0,
                latency_p99: 350.0,
                error_rate: 1.0,
                throughput: 95.0,
                availability: 99.0,
                cpu_usage: 55.0,
                memory_usage: 1100.0,
                disk_usage: 22.0,
            }
        ];

        let percentage = validator.calculate_slo_percentage(&slo, &metrics).unwrap();

        // Both metrics have availability >= 99.0 (threshold), so 100% compliance
        assert_eq!(percentage, 100.0);
    }
}
