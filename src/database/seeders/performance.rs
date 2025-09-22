//! Performance Data Generator for AI-CORE
//!
//! This module provides specialized data generation for performance testing scenarios,
//! including large-scale data volumes, realistic usage patterns, and time-series data
//! generation for analytics and load testing.
//!
//! # Features
//!
//! - High-volume data generation with batching
//! - Realistic usage patterns and temporal distributions
//! - Memory-efficient streaming data generation
//! - Performance metrics collection during generation
//! - Concurrent data generation for speed
//!
//! # Usage
//!
//! ```rust
//! use database::seeders::performance::{PerformanceGenerator, PerformanceConfig};
//!
//! let generator = PerformanceGenerator::new(pool).await?;
//! let report = generator.generate_performance_data(config, perf_config).await?;
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use futures::stream::{self, StreamExt};
use rand::{thread_rng, Rng, seq::SliceRandom};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{Duration, Instant};
use tracing::{info, debug, warn};
use uuid::Uuid;

use super::{SeedingConfig, SeedingReport, PerformanceConfig, utils};

/// Generator for large-scale performance testing data
pub struct PerformanceGenerator {
    pool: Arc<PgPool>,
    generation_stats: GenerationStats,
}

impl PerformanceGenerator {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            generation_stats: GenerationStats::new(),
        }
    }

    /// Generate performance testing data according to configuration
    pub async fn generate_performance_data(
        &self,
        config: &SeedingConfig,
        perf_config: &PerformanceConfig
    ) -> Result<SeedingReport> {
        info!("Starting performance data generation...");
        info!("Target size: {} GB, Concurrent users: {}",
              perf_config.target_size_gb, perf_config.concurrent_users);

        let start_time = Instant::now();
        let mut report = SeedingReport::new();

        // Calculate data volumes needed to reach target size
        let volumes = self.calculate_data_volumes(perf_config).await?;
        info!("Calculated volumes: {:?}", volumes);

        // Generate users first (foundation for other data)
        report.users_created = self.generate_bulk_users(&volumes).await?;
        info!("Generated {} users", report.users_created);

        // Generate workflows with realistic patterns
        report.workflows_created = self.generate_bulk_workflows(&volumes, perf_config).await?;
        info!("Generated {} workflows", report.workflows_created);

        // Generate time-series data for analytics
        if perf_config.generate_timeseries {
            report.usage_records_created = self.generate_timeseries_usage_data(&volumes, perf_config).await?;
            info!("Generated {} usage records", report.usage_records_created);
        }

        // Generate concurrent user simulation data
        report.api_keys_created = self.generate_concurrent_access_data(&volumes, perf_config).await?;
        info!("Generated {} API keys", report.api_keys_created);

        // Generate federation data for distributed system testing
        report.federation_clients_created = self.generate_federation_load_data(&volumes).await?;
        report.mcp_servers_created = volumes.mcp_servers;

        // Generate notifications for real-time system testing
        report.notifications_created = self.generate_notification_load_data(&volumes, perf_config).await?;

        let generation_time = start_time.elapsed();
        info!("Performance data generation completed in {:?}", generation_time);

        // Calculate estimated database size
        report.estimated_size_mb = Some(self.estimate_database_size(&report).await?);

        // Add performance metrics to report
        self.add_performance_metrics_to_report(&mut report, generation_time);

        Ok(report)
    }

    /// Calculate required data volumes to reach target database size
    async fn calculate_data_volumes(&self, perf_config: &PerformanceConfig) -> Result<DataVolumes> {
        let target_bytes = (perf_config.target_size_gb * 1024.0 * 1024.0 * 1024.0) as u64;

        // Estimate average row sizes (in bytes)
        let row_sizes = RowSizeEstimates {
            user: 500,           // User with metadata
            workflow: 300,       // Workflow execution data
            usage_record: 200,   // Usage tracking
            api_key: 400,        // API key with permissions
            notification: 300,   // Notification data
            session: 250,        // User sessions
            audit_log: 350,      // Audit entries
        };

        // Calculate optimal distribution based on realistic production ratios
        let distribution = DataDistribution {
            users_percent: 0.05,      // 5% - users are the foundation
            workflows_percent: 0.35,  // 35% - main business data
            usage_records_percent: 0.30, // 30% - analytics data
            api_keys_percent: 0.05,   // 5% - access tokens
            notifications_percent: 0.15, // 15% - real-time data
            sessions_percent: 0.08,   // 8% - active sessions
            audit_logs_percent: 0.02, // 2% - compliance data
        };

        let volumes = DataVolumes {
            users: ((target_bytes as f64 * distribution.users_percent) / row_sizes.user as f64) as u32,
            workflows: ((target_bytes as f64 * distribution.workflows_percent) / row_sizes.workflow as f64) as u32,
            usage_records: ((target_bytes as f64 * distribution.usage_records_percent) / row_sizes.usage_record as f64) as u32,
            api_keys: ((target_bytes as f64 * distribution.api_keys_percent) / row_sizes.api_key as f64) as u32,
            notifications: ((target_bytes as f64 * distribution.notifications_percent) / row_sizes.notification as f64) as u32,
            sessions: ((target_bytes as f64 * distribution.sessions_percent) / row_sizes.session as f64) as u32,
            audit_logs: ((target_bytes as f64 * distribution.audit_logs_percent) / row_sizes.audit_log as f64) as u32,
            federation_clients: std::cmp::max(50, volumes.users / 20), // 1 client per 20 users
            mcp_servers: std::cmp::max(200, volumes.users / 5),       // 1 server per 5 users
        };

        // Ensure minimum viable amounts
        let volumes = DataVolumes {
            users: std::cmp::max(volumes.users, perf_config.concurrent_users * 10),
            workflows: std::cmp::max(volumes.workflows, volumes.users * 5),
            usage_records: std::cmp::max(volumes.usage_records, volumes.workflows * 2),
            api_keys: std::cmp::max(volumes.api_keys, volumes.users / 2),
            notifications: std::cmp::max(volumes.notifications, volumes.workflows / 3),
            sessions: std::cmp::max(volumes.sessions, volumes.users / 4),
            audit_logs: std::cmp::max(volumes.audit_logs, volumes.workflows / 10),
            ..volumes
        };

        Ok(volumes)
    }

    /// Generate bulk users with optimized batch processing
    async fn generate_bulk_users(&self, volumes: &DataVolumes) -> Result<u32> {
        info!("Generating {} users in batches...", volumes.users);

        let batch_size = 1000;
        let batches = (volumes.users + batch_size - 1) / batch_size;
        let created_count = Arc::new(AtomicU32::new(0));

        // Process batches concurrently
        let concurrent_batches = std::cmp::min(4, batches);
        let batch_stream = stream::iter(0..batches)
            .map(|batch_index| {
                let pool = self.pool.clone();
                let created_count = created_count.clone();
                let start_index = batch_index * batch_size;
                let end_index = std::cmp::min(start_index + batch_size, volumes.users);
                let batch_count = end_index - start_index;

                async move {
                    match self.generate_user_batch(pool, start_index, batch_count).await {
                        Ok(count) => {
                            created_count.fetch_add(count, Ordering::Relaxed);
                            Ok(count)
                        }
                        Err(e) => {
                            warn!("Failed to generate user batch {}: {}", batch_index, e);
                            Err(e)
                        }
                    }
                }
            })
            .buffer_unordered(concurrent_batches);

        let results: Vec<Result<u32>> = batch_stream.collect().await;

        let successful_batches = results.iter().filter(|r| r.is_ok()).count();
        info!("Successfully generated users in {}/{} batches", successful_batches, batches);

        Ok(created_count.load(Ordering::Relaxed))
    }

    /// Generate a batch of users
    async fn generate_user_batch(&self, pool: Arc<PgPool>, start_index: u32, count: u32) -> Result<u32> {
        let mut tx = pool.begin().await?;
        let mut created = 0;

        for i in 0..count {
            let index = start_index + i;
            let user = self.generate_performance_user(index).await?;

            sqlx::query!(
                r#"
                INSERT INTO users (
                    id, email, username, password_hash, first_name, last_name,
                    email_verified, status, subscription_tier, created_at,
                    updated_at, last_login_at, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
                user.id,
                user.email,
                user.username,
                user.password_hash,
                user.first_name,
                user.last_name,
                user.email_verified,
                user.status,
                user.subscription_tier,
                user.created_at,
                user.updated_at,
                user.last_login_at,
                user.metadata
            )
            .execute(&mut *tx)
            .await?;

            created += 1;
        }

        tx.commit().await?;
        Ok(created)
    }

    /// Generate workflows with realistic usage patterns
    async fn generate_bulk_workflows(&self, volumes: &DataVolumes, perf_config: &PerformanceConfig) -> Result<u32> {
        info!("Generating {} workflows with usage patterns...", volumes.workflows);

        // Get user IDs for foreign key references
        let user_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM users WHERE email LIKE '%@perf.example.com' ORDER BY created_at"
        )
        .fetch_all(&*self.pool)
        .await?;

        if user_ids.is_empty() {
            return Err(anyhow::anyhow!("No users found for workflow generation"));
        }

        let batch_size = 2000;
        let batches = (volumes.workflows + batch_size - 1) / batch_size;
        let created_count = Arc::new(AtomicU32::new(0));

        // Generate workflows with temporal patterns
        let concurrent_batches = 6;
        let batch_stream = stream::iter(0..batches)
            .map(|batch_index| {
                let pool = self.pool.clone();
                let created_count = created_count.clone();
                let user_ids = user_ids.clone();
                let start_index = batch_index * batch_size;
                let end_index = std::cmp::min(start_index + batch_size, volumes.workflows);
                let batch_count = end_index - start_index;

                async move {
                    match self.generate_workflow_batch(
                        pool,
                        &user_ids,
                        start_index,
                        batch_count,
                        perf_config
                    ).await {
                        Ok(count) => {
                            created_count.fetch_add(count, Ordering::Relaxed);
                            Ok(count)
                        }
                        Err(e) => {
                            warn!("Failed to generate workflow batch {}: {}", batch_index, e);
                            Err(e)
                        }
                    }
                }
            })
            .buffer_unordered(concurrent_batches);

        let _results: Vec<Result<u32>> = batch_stream.collect().await;
        Ok(created_count.load(Ordering::Relaxed))
    }

    /// Generate time-series usage data for analytics testing
    async fn generate_timeseries_usage_data(&self, volumes: &DataVolumes, perf_config: &PerformanceConfig) -> Result<u32> {
        info!("Generating {} usage records for time-series analytics...", volumes.usage_records);

        // Get users and their subscriptions
        let user_subscription_pairs: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT u.id, s.id FROM users u LEFT JOIN subscriptions s ON u.id = s.user_id WHERE u.email LIKE '%@perf.example.com'"
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut created_count = 0;
        let batch_size = 5000;

        // Generate usage data across time periods
        let time_periods = self.generate_time_periods(30); // Last 30 days

        for time_period in time_periods {
            let period_records = volumes.usage_records / 30; // Distribute across days

            for batch_start in (0..period_records).step_by(batch_size) {
                let batch_end = std::cmp::min(batch_start + batch_size, period_records);
                let batch_count = batch_end - batch_start;

                let count = self.generate_usage_batch(
                    &user_subscription_pairs,
                    time_period,
                    batch_count,
                    perf_config
                ).await?;

                created_count += count;

                if created_count % 50000 == 0 {
                    debug!("Generated {} usage records", created_count);
                }
            }
        }

        Ok(created_count)
    }

    /// Generate concurrent access data for load testing
    async fn generate_concurrent_access_data(&self, volumes: &DataVolumes, perf_config: &PerformanceConfig) -> Result<u32> {
        info!("Generating concurrent access data for {} users...", perf_config.concurrent_users);

        // Get a subset of users for concurrent testing
        let concurrent_user_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM users WHERE email LIKE '%@perf.example.com' ORDER BY random() LIMIT $1"
        )
        .bind(perf_config.concurrent_users as i64)
        .fetch_all(&*self.pool)
        .await?;

        let mut created_count = 0;

        // Generate API keys for concurrent access
        for user_id in &concurrent_user_ids {
            let api_keys_per_user = thread_rng().gen_range(2..8); // 2-7 API keys per concurrent user

            for i in 0..api_keys_per_user {
                self.generate_performance_api_key(*user_id, i).await?;
                created_count += 1;
            }

            // Generate multiple active sessions for each concurrent user
            let sessions_per_user = thread_rng().gen_range(1..4); // 1-3 active sessions
            for _ in 0..sessions_per_user {
                self.generate_performance_session(*user_id).await?;
            }
        }

        Ok(created_count)
    }

    /// Generate federation data for distributed system testing
    async fn generate_federation_load_data(&self, volumes: &DataVolumes) -> Result<u32> {
        info!("Generating {} federation clients for distributed testing...", volumes.federation_clients);

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        for i in 0..volumes.federation_clients {
            let client = self.generate_performance_federation_client(i).await?;

            sqlx::query!(
                r#"
                INSERT INTO federation_clients (
                    id, client_name, client_id, api_endpoint, auth_type, auth_config,
                    webhook_url, status, rate_limit_per_minute, rate_limit_per_hour,
                    sla_uptime_percent, sla_response_time_ms, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                "#,
                client.id,
                client.client_name,
                client.client_id,
                client.api_endpoint,
                client.auth_type,
                client.auth_config,
                client.webhook_url,
                client.status,
                client.rate_limit_per_minute,
                client.rate_limit_per_hour,
                client.sla_uptime_percent,
                client.sla_response_time_ms,
                client.created_at,
                client.updated_at
            )
            .execute(&mut *tx)
            .await?;

            // Generate MCP servers for this client
            let servers_per_client = thread_rng().gen_range(3..12);
            for j in 0..servers_per_client {
                self.generate_performance_mcp_server(&mut tx, client.id, j).await?;
            }

            created_count += 1;

            // Commit in batches
            if created_count % 100 == 0 {
                tx.commit().await?;
                tx = self.pool.begin().await?;
            }
        }

        tx.commit().await?;
        Ok(created_count)
    }

    /// Generate notification load data for real-time testing
    async fn generate_notification_load_data(&self, volumes: &DataVolumes, perf_config: &PerformanceConfig) -> Result<u32> {
        info!("Generating {} notifications for real-time system testing...", volumes.notifications);

        let user_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM users WHERE email LIKE '%@perf.example.com'"
        )
        .fetch_all(&*self.pool)
        .await?;

        let workflow_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM workflows LIMIT 10000" // Limit to avoid memory issues
        )
        .fetch_all(&*self.pool)
        .await?;

        let batch_size = 3000;
        let mut created_count = 0;

        for batch_start in (0..volumes.notifications).step_by(batch_size) {
            let batch_end = std::cmp::min(batch_start + batch_size, volumes.notifications);
            let batch_count = batch_end - batch_start;

            let count = self.generate_notification_batch(
                &user_ids,
                &workflow_ids,
                batch_count,
                perf_config
            ).await?;

            created_count += count;

            if created_count % 10000 == 0 {
                debug!("Generated {} notifications", created_count);
            }
        }

        Ok(created_count)
    }

    // Helper methods for generating individual data types

    async fn generate_performance_user(&self, index: u32) -> Result<PerformanceUser> {
        let mut rng = thread_rng();

        let subscription_tiers = vec![
            ("free", 0.7),
            ("pro", 0.25),
            ("enterprise", 0.05),
        ];
        let subscription_tier = utils::weighted_choice(&subscription_tiers).unwrap();

        let created_at = utils::random_business_hours_timestamp(rng.gen_range(1..365));

        Ok(PerformanceUser {
            id: Uuid::new_v4(),
            email: format!("perfuser_{}@perf.example.com", index),
            username: format!("perfuser_{}", index),
            password_hash: "performance_test_hash".to_string(),
            first_name: Some(format!("PerfUser{}", index)),
            last_name: Some("TestAccount".to_string()),
            email_verified: true,
            status: "active".to_string(),
            subscription_tier,
            created_at,
            updated_at: created_at,
            last_login_at: Some(created_at + ChronoDuration::hours(rng.gen_range(1..24))),
            metadata: json!({
                "source": "performance_test",
                "batch_index": index,
                "test_user": true
            }),
        })
    }

    async fn generate_workflow_batch(
        &self,
        pool: Arc<PgPool>,
        user_ids: &[Uuid],
        start_index: u32,
        count: u32,
        perf_config: &PerformanceConfig
    ) -> Result<u32> {
        let mut tx = pool.begin().await?;
        let mut created = 0;

        for i in 0..count {
            let user_id = user_ids.choose(&mut thread_rng()).unwrap();
            let workflow = self.generate_performance_workflow(*user_id, start_index + i, perf_config).await?;

            sqlx::query!(
                r#"
                INSERT INTO workflows (
                    id, user_id, workflow_type, status, priority,
                    estimated_cost_cents, actual_cost_cents,
                    estimated_duration_seconds, actual_duration_seconds,
                    started_at, completed_at, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
                workflow.id,
                workflow.user_id,
                workflow.workflow_type,
                workflow.status,
                workflow.priority,
                workflow.estimated_cost_cents,
                workflow.actual_cost_cents,
                workflow.estimated_duration_seconds,
                workflow.actual_duration_seconds,
                workflow.started_at,
                workflow.completed_at,
                workflow.created_at,
                workflow.updated_at
            )
            .execute(&mut *tx)
            .await?;

            created += 1;
        }

        tx.commit().await?;
        Ok(created)
    }

    async fn generate_performance_workflow(&self, user_id: Uuid, index: u32, perf_config: &PerformanceConfig) -> Result<PerformanceWorkflow> {
        let mut rng = thread_rng();

        // Realistic workflow type distribution for performance testing
        let workflow_types = vec![
            ("data_processing", 0.35),
            ("api_integration", 0.25),
            ("report_generation", 0.15),
            ("batch_job", 0.10),
            ("real_time_analysis", 0.08),
            ("data_backup", 0.05),
            ("system_maintenance", 0.02),
        ];

        // Status distribution based on realistic completion rates
        let statuses = vec![
            ("completed", 0.75),
            ("failed", 0.15),
            ("running", 0.08),
            ("created", 0.02),
        ];

        let workflow_type = utils::weighted_choice(&workflow_types).unwrap();
        let status = utils::weighted_choice(&statuses).unwrap();

        // Generate timing with peak usage patterns
        let created_at = if rng.gen_bool(0.3) { // 30% during peak hours
            self.generate_peak_usage_timestamp(perf_config)
        } else {
            utils::random_business_hours_timestamp(rng.gen_range(1..30))
        };

        let estimated_duration = rng.gen_range(60..7200); // 1 minute to 2 hours
        let estimated_cost = rng.gen_range(10..5000);

        let (started_at, completed_at, actual_duration, actual_cost) = match status.as_str() {
            "completed" => {
                let started = created_at + ChronoDuration::seconds(rng.gen_range(1..300));
                let duration_variance = rng.gen_range(0.5..1.5);
                let actual_dur = (estimated_duration as f64 * duration_variance) as i32;
                let completed = started + ChronoDuration::seconds(actual_dur as i64);
                let cost_variance = rng.gen_range(0.8..1.2);
                let actual_cost_val = (estimated_cost as f64 * cost_variance) as i32;
                (Some(started), Some(completed), Some(actual_dur), Some(actual_cost_val))
            },
            "failed" => {
                let started = created_at + ChronoDuration::seconds(rng.gen_range(1..300));
                let failed_duration = rng.gen_range(30..estimated_duration / 3);
                let failed_at = started + ChronoDuration::seconds(failed_duration as i64);
                (Some(started), Some(failed_at), Some(failed_duration), None)
            },
            "running" => {
                let started = created_at + ChronoDuration::seconds(rng.gen_range(1..300));
                (Some(started), None, None, None)
            },
            _ => (None, None, None, None),
        };

        Ok(PerformanceWorkflow {
            id: Uuid::new_v4(),
            user_id,
            workflow_type,
            status,
            priority: if rng.gen_bool(0.1) { "high".to_string() } else { "medium".to_string() },
            estimated_cost_cents: Some(estimated_cost),
            actual_cost_cents: actual_cost,
            estimated_duration_seconds: Some(estimated_duration),
            actual_duration_seconds: actual_duration,
            started_at,
            completed_at,
            created_at,
            updated_at: completed_at.unwrap_or(started_at.unwrap_or(created_at)),
        })
    }

    fn generate_peak_usage_timestamp(&self, perf_config: &PerformanceConfig) -> DateTime<Utc> {
        let mut rng = thread_rng();

        // Peak hours: 9-11 AM and 2-4 PM
        let peak_hours = vec![9, 10, 14, 15];
        let hour = peak_hours.choose(&mut rng).unwrap();

        let days_back = rng.gen_range(1..7); // Focus on recent data for peaks
        let base_date = Utc::now() - ChronoDuration::days(days_back);

        base_date
            .date_naive()
            .and_hms_opt(*hour, rng.gen_range(0..60), rng.gen_range(0..60))
            .unwrap()
            .and_utc()
    }

    fn generate_time_periods(&self, days: u32) -> Vec<chrono::NaiveDate> {
        let mut periods = Vec::new();
        let start_date = Utc::now().date_naive();

        for i in 0..days {
            periods.push(start_date - ChronoDuration::days(i as i64));
        }

        periods
    }

    async fn generate_usage_batch(
        &self,
        user_subscription_pairs: &[(Uuid, Option<Uuid>)],
        time_period: chrono::NaiveDate,
        batch_count: u32,
        _perf_config: &PerformanceConfig
    ) -> Result<u32> {
        let mut tx = self.pool.begin().await?;
        let mut created = 0;

        for _ in 0..batch_count {
            let (user_id, subscription_id) = user_subscription_pairs.choose(&mut thread_rng()).unwrap();

            let resource_types = vec![
                ("workflow_execution", thread_rng().gen_range(1..50)),
                ("api_request", thread_rng().gen_range(100..5000)),
                ("data_storage_gb", thread_rng().gen_range(1..100)),
                ("computation_minutes", thread_rng().gen_range(10..500)),
            ];

            for (resource_type, quantity) in resource_types {
                let unit_cost = match resource_type {
                    "workflow_execution" => 25,
                    "api_request" => 1,
                    "data_storage_gb" => 50,
                    "computation_minutes" => 5,
                    _ => 1,
                };

                sqlx::query!(
                    r#"
                    INSERT INTO usage_records (
                        user_id, subscription_id, resource_type, quantity,
                        unit_cost_cents, total_cost_cents, billing_period, recorded_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    "#,
                    user_id,
                    subscription_id,
                    resource_type,
                    quantity,
                    unit_cost,
                    quantity * unit_cost,
                    time_period,
                    Utc::now()
                )
                .execute(&mut *tx)
                .await?;

                created += 1;
            }
        }

        tx.commit().await?;
        Ok(created)
    }

    async fn generate_performance_api_key(&self, user_id: Uuid, index: u32) -> Result<()> {
        let api_key = format!("perf_key_{}_{}", user_id.to_string().chars().take(8).collect::<String>(), index);
        let key_hash = format!("hash_{}", api_key);

        sqlx::query!(
            r#"
            INSERT INTO api_keys (
                user_id, key_name, key_hash, key_prefix, permissions,
                rate_limit_per_minute, rate_limit_per_hour, rate_limit_per_day,
                created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            user_id,
            format!("Performance Test Key {}", index),
            key_hash,
            api_key.chars().take(8).collect::<String>(),
            json!(["read:workflows", "write:workflows", "read:analytics"]),
            thread_rng().gen_range(1000..5000),
            thread_rng().gen_range(10000..50000),
            thread_rng().gen_range(100000..500000),
            Utc::now()
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    async fn generate_performance_session(&self, user_id: Uuid) -> Result<()> {
        let session_token = format!("perf_session_{}", Uuid::new_v4().to_string().replace("-", ""));
        let refresh_token = format!("perf_refresh_{}", Uuid::new_v4().to_string().replace("-", ""));

        sqlx::query!(
            r#"
            INSERT INTO user_sessions (
                user_id, session_token, refresh_token, expires_at,
                created_at, last_accessed_at, ip_address, user_agent, is_active
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            user_id,
            session_token,
            refresh_token,
            Utc::now() + ChronoDuration::days(30),
            Utc::now(),
            Utc::now(),
            "192.168.1.100".parse::<std::net::IpAddr>().ok(),
            Some("Performance Test Client/1.0".to_string()),
            true
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    async fn generate_performance_federation_client(&self, index: u32) -> Result<PerformanceFederationClient> {
        Ok(PerformanceFederationClient {
            id: Uuid::new_v4(),
            client_name: format!("PerfClient_{}", index),
            client_id: format!("perf_client_{}", index),
            api_endpoint: format!("https://perf-api-{}.example.com/v1", index),
            auth_type: "api_key".to_string(),
            auth_config: json!({"header_name": "X-API-Key"}),
            webhook_url: Some(format!("https://perf-webhook-{}.example.com/events", index)),
            status: "active".to_string(),
            rate_limit_per_minute: thread_rng().gen_range(1000..10000),
            rate_limit_per_hour: thread_rng().gen_range(10000..100000),
            sla_uptime_percent: rust_decimal::Decimal::from_f32_retain(99.9).unwrap(),
            sla_response_time_ms: thread_rng().gen_range(50..500),
            created_at: Utc::now() - ChronoDuration::days(thread_rng().gen_range(1..30)),
            updated_at: Utc::now(),
        })
    }

    async fn generate_performance_mcp_server(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, client_id: Uuid, index: u32) -> Result<()> {
        let server_types = vec![
            "high-throughput-processor", "real-time-analytics", "batch-data-processor",
            "stream-processor", "ml-inference-engine", "data-transformer"
        ];

        let server_type = server_types.choose(&mut thread_rng()).unwrap();
        let server_id = format!("{}-{}", server_type, index);

        sqlx::query!(
            r#"
            INSERT INTO mcp_servers (
                client_id, server_id, server_name, description, endpoint,
                version, auth_required, cost_per_request_cents, status,
                capabilities, tools, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            client_id,
            server_id,
            format!("Performance {} Server {}", server_type, index),
            Some(format!("High-performance MCP server for {}", server_type)),
            format!("https://perf-mcp.example.com/{}", server_id),
            Some("1.0.0".to_string()),
            true,
            thread_rng().gen_range(1..10),
            "active",
            json!(["high_throughput", "real_time", "batch_processing"]),
            json!([{"name": format!("{}_tool", server_type), "version": "1.0"}]),
            Utc::now(),
            Utc::now()
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn generate_notification_batch(
        &self,
        user_ids: &[Uuid],
        workflow_ids: &[Uuid],
        batch_count: u32,
        _perf_config: &PerformanceConfig
    ) -> Result<u32> {
        let mut tx = self.pool.begin().await?;
        let mut created = 0;

        let notification_types = vec![
            ("workflow_completed", 0.4),
            ("workflow_failed", 0.2),
            ("system_alert", 0.15),
            ("billing_notification", 0.1),
            ("security_alert", 0.05),
            ("maintenance_notice", 0.05),
            ("feature_announcement", 0.05),
        ];

        let priorities = vec![
            ("low", 0.4),
            ("medium", 0.4),
            ("high", 0.15),
            ("urgent", 0.05),
        ];

        for _ in 0..batch_count {
            let user_id = user_ids.choose(&mut thread_rng()).unwrap();
            let workflow_id = if thread_rng().gen_bool(0.6) {
                workflow_ids.choose(&mut thread_rng()).copied()
            } else {
                None
            };

            let notification_type = utils::weighted_choice(&notification_types).unwrap();
            let priority = utils::weighted_choice(&priorities).unwrap();

            let (title, message) = self.generate_notification_content(&notification_type);

            sqlx::query!(
                r#"
                INSERT INTO notifications (
                    user_id, notification_type, title, message, priority,
                    channel, is_read, is_sent, workflow_id, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
                user_id,
                notification_type,
                title,
                message,
                priority,
                "in_app",
                thread_rng().gen_bool(0.3), // 30% read
                thread_rng().gen_bool(0.8), // 80% sent
                workflow_id,
                Utc::now() - ChronoDuration::minutes(thread_rng().gen_range(1..10080)) // Last week
            )
            .execute(&mut *tx)
            .await?;

            created += 1;
        }

        tx.commit().await?;
        Ok(created)
    }

    fn generate_notification_content(&self, notification_type: &str) -> (String, String) {
        match notification_type {
            "workflow_completed" => (
                "Workflow Completed Successfully".to_string(),
                "Your data processing workflow has completed successfully. Results are now available.".to_string()
            ),
            "workflow_failed" => (
                "Workflow Execution Failed".to_string(),
                "Your workflow encountered an error during execution. Please check the logs for details.".to_string()
            ),
            "system_alert" => (
                "System Performance Alert".to_string(),
                "System performance metrics are showing elevated resource usage.".to_string()
            ),
            "billing_notification" => (
                "Billing Update".to_string(),
                "Your monthly usage summary is now available. Review your current billing cycle.".to_string()
            ),
            "security_alert" => (
                "Security Notice".to_string(),
                "Unusual access pattern detected on your account. Please verify recent activity.".to_string()
            ),
            _ => (
                "System Notification".to_string(),
                "You have a new system notification requiring attention.".to_string()
            )
        }
    }

    async fn estimate_database_size(&self, report: &SeedingReport) -> Result<f64> {
        // Estimate based on generated record counts and average row sizes
        let estimated_size =
            (report.users_created as f64 * 0.5) +           // 500 bytes per user
            (report.workflows_created as f64 * 0.3) +       // 300 bytes per workflow
            (report.usage_records_created as f64 * 0.2) +   // 200 bytes per usage record
            (report.api_keys_created as f64 * 0.4) +        // 400 bytes per API key
            (report.notifications_created as f64 * 0.3) +   // 300 bytes per notification
            (report.federation_clients_created as f64 * 0.6) + // 600 bytes per client
            (report.mcp_servers_created as f64 * 0.4);      // 400 bytes per server

        Ok(estimated_size / 1024.0) // Convert to MB
    }

    fn add_performance_metrics_to_report(&self, report: &mut SeedingReport, generation_time: Duration) {
        let total_records = report.users_created + report.workflows_created +
                          report.usage_records_created + report.api_keys_created +
                          report.notifications_created;

        let records_per_second = total_records as f64 / generation_time.as_secs_f64();

        report.warnings.push(format!(
            "Performance: Generated {} records in {:.2}s ({:.0} records/sec)",
            total_records,
            generation_time.as_secs_f64(),
            records_per_second
        ));
    }
}

// Data structures for performance generation

#[derive(Debug)]
struct DataVolumes {
    users: u32,
    workflows: u32,
    usage_records: u32,
    api_keys: u32,
    notifications: u32,
    sessions: u32,
    audit_logs: u32,
    federation_clients: u32,
    mcp_servers: u32,
}

#[derive(Debug)]
struct RowSizeEstimates {
    user: u32,
    workflow: u32,
    usage_record: u32,
    api_key: u32,
    notification: u32,
    session: u32,
    audit_log: u32,
}

#[derive(Debug)]
struct DataDistribution {
    users_percent: f64,
    workflows_percent: f64,
    usage_records_percent: f64,
    api_keys_percent: f64,
    notifications_percent: f64,
    sessions_percent: f64,
    audit_logs_percent: f64,
}

#[derive(Debug)]
struct GenerationStats {
    start_time: Option<Instant>,
    records_generated: AtomicU32,
    batches_processed: AtomicU32,
}

impl GenerationStats {
    fn new() -> Self {
        Self {
            start_time: None,
            records_generated: AtomicU32::new(0),
            batches_processed: AtomicU32::new(0),
        }
    }
}

// Data structures for generated records

#[derive(Debug)]
struct PerformanceUser {
    id: Uuid,
    email: String,
    username: String,
    password_hash: String,
    first_name: Option<String>,
    last_name: Option<String>,
    email_verified: bool,
    status: String,
    subscription_tier: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
}

#[derive(Debug)]
struct PerformanceWorkflow {
    id: Uuid,
    user_id: Uuid,
    workflow_type: String,
    status: String,
    priority: String,
    estimated_cost_cents: Option<i32>,
    actual_cost_cents: Option<i32>,
    estimated_duration_seconds: Option<i32>,
    actual_duration_seconds: Option<i32>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
struct PerformanceFederationClient {
    id: Uuid,
    client_name: String,
    client_id: String,
    api_endpoint: String,
    auth_type: String,
    auth_config: serde_json::Value,
    webhook_url: Option<String>,
    status: String,
    rate_limit_per_minute: i32,
    rate_limit_per_hour: i32,
    sla_uptime_percent: rust_decimal::Decimal,
    sla_response_time_ms: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_volume_calculation() {
        let perf_config = PerformanceConfig {
            target_size_gb: 1.0,
            concurrent_users: 100,
            peak_usage_multiplier: 2.0,
            generate_timeseries: true,
        };

        // This would need async test setup
        // let generator = PerformanceGenerator::new(mock_pool).await;
        // let volumes = generator.calculate_data_volumes(&perf_config).await.unwrap();
        // assert!(volumes.users >= perf_config.concurrent_users * 10);
    }

    #[test]
    fn test_time_period_generation() {
        let generator = PerformanceGenerator {
            pool: Arc::new(/* mock pool */),
            generation_stats: GenerationStats::new(),
        };

        let periods = generator.generate_time_periods(7);
        assert_eq!(periods.len(), 7);
    }
}
