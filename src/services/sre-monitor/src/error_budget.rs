use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::models::{ErrorBudget, ServiceMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudgetConfig {
    pub service_name: String,
    pub slo_name: String,
    pub budget_percentage: f64,
    pub time_window: String,
    pub alert_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRateCalculation {
    pub service_name: String,
    pub slo_name: String,
    pub current_burn_rate: f64,
    pub acceptable_burn_rate: f64,
    pub time_window: String,
    pub calculated_at: DateTime<Utc>,
    pub severity: String,
}

pub struct ErrorBudgetTracker {
    db: PgPool,
    configs: tokio::sync::RwLock<HashMap<String, ErrorBudgetConfig>>,
}

impl ErrorBudgetTracker {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            configs: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_error_budget_config(&self, config: ErrorBudgetConfig) -> Result<()> {
        let key = format!("{}:{}", config.service_name, config.slo_name);
        let mut configs = self.configs.write().await;
        configs.insert(key, config);
        Ok(())
    }

    pub async fn calculate_error_budgets(&self) -> Result<()> {
        info!("Starting error budget calculation cycle");

        let configs = self.configs.read().await;

        for (key, config) in configs.iter() {
            if let Err(e) = self.calculate_service_error_budget(config).await {
                error!("Failed to calculate error budget for {}: {}", key, e);
            }
        }

        // Also calculate for services without explicit configs
        let default_services = vec![
            "api-gateway",
            "intent-parser-server",
            "mcp-manager-server",
            "federation-server",
            "test-data-api",
            "chaos-monkey"
        ];

        for service in default_services {
            if !configs.iter().any(|(_, config)| config.service_name == service) {
                let default_config = ErrorBudgetConfig {
                    service_name: service.to_string(),
                    slo_name: "availability".to_string(),
                    budget_percentage: 1.0, // 1% error budget
                    time_window: "30d".to_string(),
                    alert_threshold: 0.1, // Alert when 10% of budget consumed
                };

                if let Err(e) = self.calculate_service_error_budget(&default_config).await {
                    error!("Failed to calculate default error budget for {}: {}", service, e);
                }
            }
        }

        info!("Error budget calculation cycle completed");
        Ok(())
    }

    async fn calculate_service_error_budget(&self, config: &ErrorBudgetConfig) -> Result<()> {
        let time_window_duration = self.parse_time_window(&config.time_window)?;
        let start_time = Utc::now() - time_window_duration;
        let end_time = Utc::now();

        // Get service metrics from the database
        let metrics = self.get_service_metrics_from_db(
            &config.service_name,
            start_time,
            end_time
        ).await?;

        if metrics.is_empty() {
            warn!("No metrics found for service {} in time window {}",
                  config.service_name, config.time_window);
            return Ok(());
        }

        // Calculate error budget consumption
        let (consumed_percentage, remaining_percentage, burn_rate) =
            self.calculate_budget_consumption(&metrics, config.budget_percentage, time_window_duration)?;

        let error_budget = ErrorBudget {
            service_name: config.service_name.clone(),
            slo_name: config.slo_name.clone(),
            budget_percentage: config.budget_percentage,
            consumed_percentage,
            remaining_percentage,
            time_window: config.time_window.clone(),
            last_updated: Utc::now(),
            burn_rate,
            status: self.determine_budget_status(remaining_percentage, burn_rate),
        };

        // Store the calculated error budget
        self.store_error_budget(&error_budget).await?;

        // Check if we need to trigger alerts
        if remaining_percentage < config.alert_threshold {
            self.trigger_budget_alert(&error_budget).await?;
        }

        info!("Error budget calculated for {}: {:.2}% remaining",
              config.service_name, remaining_percentage);

        Ok(())
    }

    fn calculate_budget_consumption(
        &self,
        metrics: &[ServiceMetrics],
        budget_percentage: f64,
        time_window: ChronoDuration
    ) -> Result<(f64, f64, f64)> {
        if metrics.is_empty() {
            return Ok((0.0, budget_percentage, 0.0));
        }

        // Calculate error rate over the time window
        let total_requests: f64 = metrics.iter().map(|m| m.throughput).sum();
        let total_errors: f64 = metrics.iter()
            .map(|m| m.throughput * (m.error_rate / 100.0))
            .sum();

        let overall_error_rate = if total_requests > 0.0 {
            (total_errors / total_requests) * 100.0
        } else {
            0.0
        };

        // Calculate availability from error rate
        let availability = 100.0 - overall_error_rate;

        // SLO target (assume 99.9% availability target)
        let slo_target = 99.9;

        // Calculate budget consumption
        let unavailability = 100.0 - availability;
        let allowed_unavailability = 100.0 - slo_target;

        let consumed_percentage = if allowed_unavailability > 0.0 {
            (unavailability / allowed_unavailability) * 100.0
        } else {
            0.0
        }.min(100.0);

        let remaining_percentage = (100.0 - consumed_percentage).max(0.0);

        // Calculate burn rate (how fast we're consuming the budget)
        let hours_in_window = time_window.num_hours() as f64;
        let burn_rate = if hours_in_window > 0.0 {
            consumed_percentage / hours_in_window
        } else {
            0.0
        };

        Ok((consumed_percentage, remaining_percentage, burn_rate))
    }

    fn determine_budget_status(&self, remaining_percentage: f64, burn_rate: f64) -> String {
        if remaining_percentage <= 0.0 {
            "exhausted".to_string()
        } else if remaining_percentage < 10.0 || burn_rate > 10.0 {
            "critical".to_string()
        } else if remaining_percentage < 25.0 || burn_rate > 5.0 {
            "warning".to_string()
        } else {
            "healthy".to_string()
        }
    }

    async fn get_service_metrics_from_db(
        &self,
        service_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>
    ) -> Result<Vec<ServiceMetrics>> {
        let rows = sqlx::query!(
            r#"
            SELECT service_name, timestamp, latency_p50, latency_p95, latency_p99,
                   error_rate, throughput, availability, cpu_usage, memory_usage, disk_usage
            FROM service_metrics
            WHERE service_name = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
            service_name,
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

    async fn store_error_budget(&self, budget: &ErrorBudget) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO error_budgets
            (service_name, slo_name, budget_percentage, consumed_percentage,
             remaining_percentage, time_window, last_updated, burn_rate, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (service_name, slo_name, time_window)
            DO UPDATE SET
                budget_percentage = EXCLUDED.budget_percentage,
                consumed_percentage = EXCLUDED.consumed_percentage,
                remaining_percentage = EXCLUDED.remaining_percentage,
                last_updated = EXCLUDED.last_updated,
                burn_rate = EXCLUDED.burn_rate,
                status = EXCLUDED.status
            "#,
            budget.service_name,
            budget.slo_name,
            budget.budget_percentage,
            budget.consumed_percentage,
            budget.remaining_percentage,
            budget.time_window,
            budget.last_updated,
            budget.burn_rate,
            budget.status
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn trigger_budget_alert(&self, budget: &ErrorBudget) -> Result<()> {
        // Check if we've already alerted recently to avoid spam
        let recent_alert = sqlx::query!(
            r#"
            SELECT id FROM alerts
            WHERE service_name = $1
            AND alert_type = 'error_budget_alert'
            AND created_at > NOW() - INTERVAL '1 hour'
            AND status = 'active'
            LIMIT 1
            "#,
            budget.service_name
        )
        .fetch_optional(&self.db)
        .await?;

        if recent_alert.is_some() {
            return Ok(()); // Don't spam alerts
        }

        let severity = match budget.status.as_str() {
            "exhausted" => "critical",
            "critical" => "high",
            "warning" => "medium",
            _ => "low",
        };

        let message = format!(
            "Error budget for service '{}' SLO '{}' is {}% consumed. Status: {}. Burn rate: {:.2}%/hour",
            budget.service_name,
            budget.slo_name,
            budget.consumed_percentage,
            budget.status,
            budget.burn_rate
        );

        let alert_id = uuid::Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO alerts (id, service_name, alert_type, severity, message, status, created_at, metadata)
            VALUES ($1, $2, 'error_budget_alert', $3, $4, 'active', $5, $6)
            "#,
            alert_id,
            budget.service_name,
            severity,
            message,
            Utc::now(),
            serde_json::to_value(budget)?
        )
        .execute(&self.db)
        .await?;

        warn!("Error budget alert triggered for service {}: {}", budget.service_name, message);
        Ok(())
    }

    pub async fn get_all_error_budgets(&self, time_window: &str) -> Result<Vec<ErrorBudget>> {
        let rows = sqlx::query!(
            r#"
            SELECT service_name, slo_name, budget_percentage, consumed_percentage,
                   remaining_percentage, time_window, last_updated, burn_rate, status
            FROM error_budgets
            WHERE time_window = $1
            ORDER BY remaining_percentage ASC
            "#,
            time_window
        )
        .fetch_all(&self.db)
        .await?;

        let budgets = rows.into_iter().map(|row| ErrorBudget {
            service_name: row.service_name,
            slo_name: row.slo_name,
            budget_percentage: row.budget_percentage,
            consumed_percentage: row.consumed_percentage,
            remaining_percentage: row.remaining_percentage,
            time_window: row.time_window,
            last_updated: row.last_updated,
            burn_rate: row.burn_rate,
            status: row.status,
        }).collect();

        Ok(budgets)
    }

    pub async fn get_service_error_budget(&self, service_name: &str, time_window: &str) -> Result<ErrorBudget> {
        let row = sqlx::query!(
            r#"
            SELECT service_name, slo_name, budget_percentage, consumed_percentage,
                   remaining_percentage, time_window, last_updated, burn_rate, status
            FROM error_budgets
            WHERE service_name = $1 AND time_window = $2
            ORDER BY last_updated DESC
            LIMIT 1
            "#,
            service_name,
            time_window
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(row) => Ok(ErrorBudget {
                service_name: row.service_name,
                slo_name: row.slo_name,
                budget_percentage: row.budget_percentage,
                consumed_percentage: row.consumed_percentage,
                remaining_percentage: row.remaining_percentage,
                time_window: row.time_window,
                last_updated: row.last_updated,
                burn_rate: row.burn_rate,
                status: row.status,
            }),
            None => {
                // Return a default budget if none exists
                Ok(ErrorBudget {
                    service_name: service_name.to_string(),
                    slo_name: "availability".to_string(),
                    budget_percentage: 1.0,
                    consumed_percentage: 0.0,
                    remaining_percentage: 1.0,
                    time_window: time_window.to_string(),
                    last_updated: Utc::now(),
                    burn_rate: 0.0,
                    status: "healthy".to_string(),
                })
            }
        }
    }

    pub async fn calculate_burn_rate_alerts(&self) -> Result<Vec<BurnRateCalculation>> {
        let mut alerts = Vec::new();

        let budgets = self.get_all_error_budgets("30d").await?;

        for budget in budgets {
            let acceptable_burn_rate = self.calculate_acceptable_burn_rate(&budget.time_window)?;

            let severity = if budget.burn_rate > acceptable_burn_rate * 10.0 {
                "critical"
            } else if budget.burn_rate > acceptable_burn_rate * 5.0 {
                "high"
            } else if budget.burn_rate > acceptable_burn_rate * 2.0 {
                "medium"
            } else {
                continue; // No alert needed
            };

            alerts.push(BurnRateCalculation {
                service_name: budget.service_name,
                slo_name: budget.slo_name,
                current_burn_rate: budget.burn_rate,
                acceptable_burn_rate,
                time_window: budget.time_window,
                calculated_at: Utc::now(),
                severity: severity.to_string(),
            });
        }

        Ok(alerts)
    }

    fn calculate_acceptable_burn_rate(&self, time_window: &str) -> Result<f64> {
        let duration = self.parse_time_window(time_window)?;
        let hours = duration.num_hours() as f64;

        if hours <= 0.0 {
            return Ok(0.0);
        }

        // Acceptable burn rate is consuming the full budget evenly over the time window
        Ok(100.0 / hours)
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

    pub async fn cleanup_old_budgets(&self, retention_days: i64) -> Result<()> {
        let cutoff_date = Utc::now() - ChronoDuration::days(retention_days);

        let deleted = sqlx::query!(
            "DELETE FROM error_budgets WHERE last_updated < $1",
            cutoff_date
        )
        .execute(&self.db)
        .await?;

        info!("Cleaned up {} old error budget records", deleted.rows_affected());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn test_parse_time_window() {
        let tracker = ErrorBudgetTracker::new(PgPool::connect("").await.unwrap()); // Mock pool

        assert_eq!(tracker.parse_time_window("30d").unwrap(), ChronoDuration::days(30));
        assert_eq!(tracker.parse_time_window("24h").unwrap(), ChronoDuration::hours(24));
        assert_eq!(tracker.parse_time_window("60m").unwrap(), ChronoDuration::minutes(60));
        assert_eq!(tracker.parse_time_window("2w").unwrap(), ChronoDuration::weeks(2));

        assert!(tracker.parse_time_window("invalid").is_err());
        assert!(tracker.parse_time_window("30x").is_err());
    }

    #[test]
    fn test_determine_budget_status() {
        let tracker = ErrorBudgetTracker::new(PgPool::connect("").await.unwrap());

        assert_eq!(tracker.determine_budget_status(50.0, 1.0), "healthy");
        assert_eq!(tracker.determine_budget_status(20.0, 3.0), "warning");
        assert_eq!(tracker.determine_budget_status(5.0, 2.0), "critical");
        assert_eq!(tracker.determine_budget_status(0.0, 1.0), "exhausted");
    }

    #[test]
    fn test_calculate_acceptable_burn_rate() {
        let tracker = ErrorBudgetTracker::new(PgPool::connect("").await.unwrap());

        // For 30 days (720 hours), acceptable burn rate should be ~0.139%/hour
        let rate_30d = tracker.calculate_acceptable_burn_rate("30d").unwrap();
        assert!((rate_30d - (100.0 / 720.0)).abs() < 0.01);

        // For 24 hours, acceptable burn rate should be ~4.17%/hour
        let rate_24h = tracker.calculate_acceptable_burn_rate("24h").unwrap();
        assert!((rate_24h - (100.0 / 24.0)).abs() < 0.01);
    }

    #[test]
    fn test_calculate_budget_consumption() {
        let tracker = ErrorBudgetTracker::new(PgPool::connect("").await.unwrap());

        let metrics = vec![
            ServiceMetrics {
                service_name: "test".to_string(),
                timestamp: Utc::now(),
                latency_p50: 100.0,
                latency_p95: 200.0,
                latency_p99: 300.0,
                error_rate: 0.1, // 0.1% error rate = 99.9% availability
                throughput: 100.0,
                availability: 99.9,
                cpu_usage: 50.0,
                memory_usage: 1024.0,
                disk_usage: 20.0,
            }
        ];

        let time_window = ChronoDuration::hours(24);
        let (consumed, remaining, burn_rate) = tracker
            .calculate_budget_consumption(&metrics, 1.0, time_window)
            .unwrap();

        // With 99.9% availability and 99.9% SLO target, should consume very little budget
        assert!(consumed < 10.0);
        assert!(remaining > 90.0);
        assert!(burn_rate < 1.0);
    }
}
