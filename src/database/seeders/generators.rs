//! Data Generators for Test Data Seeding
//!
//! This module contains specialized generators for different types of test data:
//! - UserGenerator: Creates realistic user accounts with proper authentication data
//! - WorkflowGenerator: Generates workflow execution data with realistic patterns
//! - BillingGenerator: Creates subscription and billing data
//! - FederationGenerator: Generates federation client and MCP server data

use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use fake::{Fake, Faker};
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use fake::faker::company::en::*;
use fake::faker::lorem::en::*;
use fake::faker::phone_number::en::*;
use rand::{thread_rng, Rng, seq::SliceRandom};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, debug};
use uuid::Uuid;
use bcrypt::{hash, DEFAULT_COST};

use super::{SeedingConfig, utils};

/// Generator for user accounts and authentication data
pub struct UserGenerator {
    pool: Arc<PgPool>,
}

impl UserGenerator {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Generate users according to configuration
    pub async fn generate_users(&self, config: &SeedingConfig) -> Result<u32> {
        info!("Generating {} users...", config.user_count);

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        for i in 0..config.user_count {
            let user = self.generate_user_data(i, config).await?;

            // Insert user
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
            .await
            .context("Failed to insert user")?;

            // Generate API keys for this user
            let api_key_count = thread_rng().gen_range(config.api_keys_per_user.0..=config.api_keys_per_user.1);
            for _ in 0..api_key_count {
                self.generate_api_key(&mut tx, user.id).await?;
            }

            // Generate user sessions
            if user.last_login_at.is_some() {
                self.generate_user_sessions(&mut tx, user.id, &user.created_at).await?;
            }

            created_count += 1;

            if created_count % 100 == 0 {
                debug!("Generated {} users", created_count);
            }
        }

        tx.commit().await?;
        info!("Successfully generated {} users", created_count);
        Ok(created_count)
    }

    /// Generate users for specific test scenarios
    pub async fn generate_test_scenario_users(&self, config: &SeedingConfig) -> Result<u32> {
        info!("Generating test scenario users...");

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        // Generate specific test users for different scenarios
        let test_scenarios = vec![
            ("admin", "admin@test.example.com", "enterprise", true),
            ("new_user", "newuser@test.example.com", "free", false),
            ("power_user", "poweruser@test.example.com", "pro", true),
            ("suspended_user", "suspended@test.example.com", "free", false),
            ("expired_trial", "expired@test.example.com", "free", false),
        ];

        for (username, email, tier, verified) in test_scenarios {
            let user = TestUser {
                id: Uuid::new_v4(),
                email: email.to_string(),
                username: username.to_string(),
                password_hash: hash("test123", DEFAULT_COST)?,
                first_name: Some(FirstName().fake()),
                last_name: Some(LastName().fake()),
                email_verified: verified,
                status: if username == "suspended_user" { "suspended".to_string() } else { "active".to_string() },
                subscription_tier: tier.to_string(),
                created_at: if username == "new_user" {
                    Utc::now() - ChronoDuration::days(1)
                } else {
                    Utc::now() - ChronoDuration::days(thread_rng().gen_range(30..365))
                },
                updated_at: Utc::now(),
                last_login_at: if verified {
                    Some(Utc::now() - ChronoDuration::hours(thread_rng().gen_range(1..72)))
                } else {
                    None
                },
                metadata: json!({
                    "test_scenario": username,
                    "source": "test_seeder"
                }),
            };

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
            .await
            .context("Failed to insert test scenario user")?;

            created_count += 1;
        }

        // Generate additional random users to reach the target count
        let remaining = config.user_count.saturating_sub(created_count);
        for i in 0..remaining {
            let user = self.generate_user_data(i + created_count, config).await?;

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
            .await
            .context("Failed to insert user")?;

            created_count += 1;
        }

        tx.commit().await?;
        info!("Successfully generated {} test scenario users", created_count);
        Ok(created_count)
    }

    async fn generate_user_data(&self, index: u32, config: &SeedingConfig) -> Result<TestUser> {
        let mut rng = thread_rng();

        // Generate realistic creation time
        let created_at = if config.realistic_timing {
            utils::random_business_hours_timestamp(rng.gen_range(1..config.historical_months as i64 * 30))
        } else {
            Utc::now() - ChronoDuration::days(rng.gen_range(1..config.historical_months as i64 * 30))
        };

        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();
        let username = format!("test_user_{}", index);
        let email = format!("{}@test.example.com", username);

        let subscription_tiers = vec![
            ("free", 0.6),
            ("pro", 0.3),
            ("enterprise", 0.1),
        ];
        let subscription_tier = utils::weighted_choice(&subscription_tiers)
            .unwrap_or("free".to_string());

        let statuses = vec![
            ("active", 0.85),
            ("suspended", 0.1),
            ("deleted", 0.05),
        ];
        let status = utils::weighted_choice(&statuses)
            .unwrap_or("active".to_string());

        let email_verified = status == "active" && rng.gen_bool(0.9);

        let last_login_at = if email_verified && status == "active" {
            Some(utils::random_timestamp_in_range(
                created_at,
                Utc::now()
            ))
        } else {
            None
        };

        Ok(TestUser {
            id: Uuid::new_v4(),
            email,
            username,
            password_hash: hash("test123", DEFAULT_COST)?,
            first_name: Some(first_name),
            last_name: Some(last_name),
            email_verified,
            status,
            subscription_tier,
            created_at,
            updated_at: created_at + ChronoDuration::seconds(rng.gen_range(1..3600)),
            last_login_at,
            metadata: json!({
                "source": "seeder",
                "index": index,
                "generated_at": Utc::now()
            }),
        })
    }

    async fn generate_api_key(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: Uuid) -> Result<()> {
        let mut rng = thread_rng();

        let key_names = vec![
            "Production API", "Development API", "Mobile App", "Web Dashboard",
            "Analytics Service", "Webhook Handler", "Backup Service", "CI/CD Pipeline"
        ];

        let key_name = key_names.choose(&mut rng).unwrap().to_string();
        let api_key = format!("ak_{}", Uuid::new_v4().to_string().replace("-", ""));
        let key_hash = hash(&api_key, DEFAULT_COST)?;
        let key_prefix = api_key.chars().take(8).collect::<String>();

        let permissions = json!([
            "read:workflows",
            "write:workflows",
            "read:users",
            if rng.gen_bool(0.3) { "admin:all" } else { "user:basic" }
        ]);

        let expires_at = if rng.gen_bool(0.3) {
            Some(Utc::now() + ChronoDuration::days(rng.gen_range(30..365)))
        } else {
            None
        };

        sqlx::query!(
            r#"
            INSERT INTO api_keys (
                user_id, key_name, key_hash, key_prefix, permissions,
                rate_limit_per_minute, rate_limit_per_hour, rate_limit_per_day,
                expires_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            user_id,
            key_name,
            key_hash,
            key_prefix,
            permissions,
            rng.gen_range(100..2000) as i32,
            rng.gen_range(1000..20000) as i32,
            rng.gen_range(10000..200000) as i32,
            expires_at,
            Utc::now()
        )
        .execute(&mut **tx)
        .await
        .context("Failed to insert API key")?;

        Ok(())
    }

    async fn generate_user_sessions(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: Uuid, user_created_at: &DateTime<Utc>) -> Result<()> {
        let mut rng = thread_rng();
        let session_count = rng.gen_range(1..5);

        for _ in 0..session_count {
            let session_token = format!("st_{}", Uuid::new_v4().to_string().replace("-", ""));
            let refresh_token = format!("rt_{}", Uuid::new_v4().to_string().replace("-", ""));

            let created_at = utils::random_timestamp_in_range(*user_created_at, Utc::now());
            let expires_at = created_at + ChronoDuration::days(30);
            let last_accessed_at = utils::random_timestamp_in_range(created_at, Utc::now());

            let user_agents = vec![
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                "Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15",
                "Mozilla/5.0 (Android 11; Mobile; rv:91.0) Gecko/91.0 Firefox/91.0",
            ];

            let ip_addresses = vec![
                "192.168.1.100", "10.0.0.50", "172.16.0.25", "203.0.113.42"
            ];

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
                expires_at,
                created_at,
                last_accessed_at,
                ip_addresses.choose(&mut rng).unwrap().parse::<std::net::IpAddr>().ok(),
                user_agents.choose(&mut rng).map(|s| s.to_string()),
                expires_at > Utc::now()
            )
            .execute(&mut **tx)
            .await
            .context("Failed to insert user session")?;
        }

        Ok(())
    }
}

/// Generator for workflow execution data
pub struct WorkflowGenerator {
    pool: Arc<PgPool>,
}

impl WorkflowGenerator {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn generate_workflows(&self, config: &SeedingConfig) -> Result<u32> {
        info!("Generating workflows...");

        // Get all users to create workflows for
        let users: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email LIKE '%@test.example.com'")
            .fetch_all(&*self.pool)
            .await?;

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        for user_id in users {
            let workflow_count = thread_rng().gen_range(config.workflows_per_user.0..=config.workflows_per_user.1);

            for _ in 0..workflow_count {
                let workflow = self.generate_workflow_data(user_id, config).await?;

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
                .await
                .context("Failed to insert workflow")?;

                created_count += 1;
            }
        }

        tx.commit().await?;
        info!("Successfully generated {} workflows", created_count);
        Ok(created_count)
    }

    pub async fn generate_test_scenario_workflows(&self, config: &SeedingConfig) -> Result<u32> {
        info!("Generating test scenario workflows...");

        let users: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email LIKE '%@test.example.com'")
            .fetch_all(&*self.pool)
            .await?;

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        // Generate specific workflow scenarios for testing
        let workflow_scenarios = vec![
            ("data_processing", "completed", "high", Some(45), Some(42)),
            ("api_integration", "failed", "medium", Some(30), None),
            ("report_generation", "running", "low", Some(120), None),
            ("data_backup", "completed", "medium", Some(60), Some(58)),
            ("system_maintenance", "cancelled", "urgent", Some(90), None),
        ];

        for user_id in users.iter().take(5) { // Only for first 5 users
            for (wf_type, status, priority, est_duration, act_duration) in &workflow_scenarios {
                let workflow = TestWorkflow {
                    id: Uuid::new_v4(),
                    user_id: *user_id,
                    workflow_type: wf_type.to_string(),
                    status: status.to_string(),
                    priority: priority.to_string(),
                    estimated_cost_cents: Some(thread_rng().gen_range(100..5000) as i32),
                    actual_cost_cents: if status == &"completed" {
                        Some(thread_rng().gen_range(90..5500) as i32)
                    } else {
                        None
                    },
                    estimated_duration_seconds: est_duration.map(|d| d * 60),
                    actual_duration_seconds: act_duration.map(|d| d * 60),
                    started_at: if status != &"created" {
                        Some(Utc::now() - ChronoDuration::hours(thread_rng().gen_range(1..24)))
                    } else {
                        None
                    },
                    completed_at: if status == &"completed" {
                        Some(Utc::now() - ChronoDuration::minutes(thread_rng().gen_range(10..1440)))
                    } else {
                        None
                    },
                    created_at: Utc::now() - ChronoDuration::days(thread_rng().gen_range(1..30)),
                    updated_at: Utc::now() - ChronoDuration::minutes(thread_rng().gen_range(1..60)),
                };

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
                .await
                .context("Failed to insert test scenario workflow")?;

                created_count += 1;
            }
        }

        tx.commit().await?;
        info!("Successfully generated {} test scenario workflows", created_count);
        Ok(created_count)
    }

    async fn generate_workflow_data(&self, user_id: Uuid, config: &SeedingConfig) -> Result<TestWorkflow> {
        let mut rng = thread_rng();

        let workflow_types = vec![
            ("data_processing", 0.3),
            ("api_integration", 0.2),
            ("report_generation", 0.15),
            ("data_backup", 0.1),
            ("email_automation", 0.1),
            ("file_conversion", 0.08),
            ("system_maintenance", 0.05),
            ("custom_script", 0.02),
        ];

        let statuses = vec![
            ("completed", 0.6),
            ("running", 0.15),
            ("failed", 0.15),
            ("created", 0.07),
            ("cancelled", 0.03),
        ];

        let priorities = vec![
            ("low", 0.4),
            ("medium", 0.4),
            ("high", 0.15),
            ("urgent", 0.05),
        ];

        let workflow_type = utils::weighted_choice(&workflow_types).unwrap();
        let status = utils::weighted_choice(&statuses).unwrap();
        let priority = utils::weighted_choice(&priorities).unwrap();

        let created_at = if config.realistic_timing {
            utils::random_business_hours_timestamp(rng.gen_range(1..config.historical_months as i64 * 30))
        } else {
            Utc::now() - ChronoDuration::days(rng.gen_range(1..config.historical_months as i64 * 30))
        };

        let estimated_duration_seconds = Some(rng.gen_range(300..7200)); // 5 minutes to 2 hours
        let estimated_cost_cents = Some(rng.gen_range(50..2000) as i32);

        let (started_at, completed_at, actual_duration_seconds, actual_cost_cents) = match status.as_str() {
            "completed" => {
                let started = created_at + ChronoDuration::seconds(rng.gen_range(1..3600));
                let duration = rng.gen_range(estimated_duration_seconds.unwrap_or(300)..=estimated_duration_seconds.unwrap_or(300) * 2);
                let completed = started + ChronoDuration::seconds(duration as i64);
                let cost_variance = rng.gen_range(0.8..1.3);
                let actual_cost = (estimated_cost_cents.unwrap_or(100) as f32 * cost_variance) as i32;
                (Some(started), Some(completed), Some(duration), Some(actual_cost))
            },
            "running" => {
                let started = created_at + ChronoDuration::seconds(rng.gen_range(1..3600));
                (Some(started), None, None, None)
            },
            "failed" => {
                let started = created_at + ChronoDuration::seconds(rng.gen_range(1..3600));
                let failed_duration = rng.gen_range(60..estimated_duration_seconds.unwrap_or(300) / 2);
                let failed_at = started + ChronoDuration::seconds(failed_duration as i64);
                (Some(started), Some(failed_at), Some(failed_duration), None)
            },
            _ => (None, None, None, None),
        };

        Ok(TestWorkflow {
            id: Uuid::new_v4(),
            user_id,
            workflow_type,
            status,
            priority,
            estimated_cost_cents,
            actual_cost_cents,
            estimated_duration_seconds,
            actual_duration_seconds,
            started_at,
            completed_at,
            created_at,
            updated_at: completed_at.unwrap_or(started_at.unwrap_or(created_at)),
        })
    }
}

/// Generator for billing and subscription data
pub struct BillingGenerator {
    pool: Arc<PgPool>,
}

impl BillingGenerator {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn generate_subscriptions(&self, config: &SeedingConfig) -> Result<u32> {
        info!("Generating subscriptions...");

        // Get users who should have subscriptions
        let users: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, subscription_tier FROM users WHERE email LIKE '%@test.example.com' AND subscription_tier != 'free'"
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        for (user_id, tier) in users {
            if thread_rng().gen_bool(config.subscription_rate as f64) {
                let subscription = self.generate_subscription_data(user_id, tier).await?;

                sqlx::query!(
                    r#"
                    INSERT INTO subscriptions (
                        id, user_id, plan_id, plan_name, status, billing_cycle,
                        amount_cents, currency, current_period_start, current_period_end,
                        trial_end, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                    "#,
                    subscription.id,
                    subscription.user_id,
                    subscription.plan_id,
                    subscription.plan_name,
                    subscription.status,
                    subscription.billing_cycle,
                    subscription.amount_cents,
                    subscription.currency,
                    subscription.current_period_start,
                    subscription.current_period_end,
                    subscription.trial_end,
                    subscription.created_at,
                    subscription.updated_at
                )
                .execute(&mut *tx)
                .await
                .context("Failed to insert subscription")?;

                // Generate usage records for this subscription
                self.generate_usage_records(&mut tx, user_id, subscription.id).await?;

                created_count += 1;
            }
        }

        tx.commit().await?;
        info!("Successfully generated {} subscriptions", created_count);
        Ok(created_count)
    }

    pub async fn generate_test_scenario_billing(&self, _config: &SeedingConfig) -> Result<u32> {
        info!("Generating test scenario billing data...");
        // Implementation for specific billing test scenarios
        Ok(0)
    }

    async fn generate_subscription_data(&self, user_id: Uuid, tier: String) -> Result<TestSubscription> {
        let mut rng = thread_rng();

        let (plan_id, plan_name, amount_cents) = match tier.as_str() {
            "pro" => ("pro_monthly", "Pro Plan", 2900),
            "enterprise" => ("enterprise_monthly", "Enterprise Plan", 9900),
            _ => ("free", "Free Plan", 0),
        };

        let billing_cycles = vec![("monthly", 0.8), ("yearly", 0.2)];
        let billing_cycle = utils::weighted_choice(&billing_cycles).unwrap();

        let amount = if billing_cycle == "yearly" {
            (amount_cents as f32 * 10.0) as i32 // 2 months free for yearly
        } else {
            amount_cents
        };

        let created_at = Utc::now() - ChronoDuration::days(rng.gen_range(1..365));
        let current_period_start = created_at;
        let current_period_end = if billing_cycle == "yearly" {
            current_period_start + ChronoDuration::days(365)
        } else {
            current_period_start + ChronoDuration::days(30)
        };

        let trial_end = if rng.gen_bool(0.3) {
            Some(created_at + ChronoDuration::days(14))
        } else {
            None
        };

        let statuses = vec![("active", 0.8), ("cancelled", 0.1), ("expired", 0.1)];
        let status = utils::weighted_choice(&statuses).unwrap();

        Ok(TestSubscription {
            id: Uuid::new_v4(),
            user_id,
            plan_id: plan_id.to_string(),
            plan_name: plan_name.to_string(),
            status,
            billing_cycle,
            amount_cents: amount,
            currency: "USD".to_string(),
            current_period_start,
            current_period_end,
            trial_end,
            created_at,
            updated_at: created_at,
        })
    }

    async fn generate_usage_records(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: Uuid, subscription_id: Uuid) -> Result<()> {
        let mut rng = thread_rng();
        let months_back = 3;

        for month in 0..months_back {
            let billing_period = (Utc::now() - ChronoDuration::days(30 * month)).date_naive();

            let resource_types = vec![
                ("workflow", rng.gen_range(10..100)),
                ("api_request", rng.gen_range(1000..10000)),
                ("storage_gb", rng.gen_range(1..50)),
            ];

            for (resource_type, quantity) in resource_types {
                let unit_cost_cents = match resource_type {
                    "workflow" => 10,
                    "api_request" => 1,
                    "storage_gb" => 100,
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
                    quantity as i32,
                    unit_cost_cents,
                    quantity as i32 * unit_cost_cents,
                    billing_period,
                    Utc::now()
                )
                .execute(&mut **tx)
                .await?;
            }
        }

        Ok(())
    }
}

/// Generator for federation clients and MCP servers
pub struct FederationGenerator {
    pool: Arc<PgPool>,
}

impl FederationGenerator {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn generate_clients(&self, config: &SeedingConfig) -> Result<u32> {
        info!("Generating {} federation clients...", config.federation_clients_count);

        let mut created_count = 0;
        let mut tx = self.pool.begin().await?;

        for i in 0..config.federation_clients_count {
            let client = self.generate_client_data(i).await?;

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
            .await
            .context("Failed to insert federation client")?;

            // Generate MCP servers for this client
            let server_count = thread_rng().gen_range(config.mcp_servers_per_client.0..=config.mcp_servers_per_client.1);
            for j in 0..server_count {
                self.generate_mcp_server(&mut tx, client.id, j).await?;
            }

            created_count += 1;
        }

        tx.commit().await?;
        info!("Successfully generated {} federation clients", created_count);
        Ok(created_count)
    }

    async fn generate_client_data(&self, index: u32) -> Result<TestFederationClient> {
        let mut rng = thread_rng();

        let company_name: String = CompanyName().fake();
        let client_name = format!("Test_{}", company_name.replace(" ", "_"));
        let client_id = format!("client_{}", index);

        let auth_types = vec![
            ("api_key", 0.4),
            ("oauth2", 0.3),
            ("jwt", 0.2),
            ("basic_auth", 0.1),
        ];
        let auth_type = utils::weighted_choice(&auth_types).unwrap();

        let auth_config = match auth_type.as_str() {
            "api_key" => json!({"header_name": "X-API-Key"}),
            "oauth2" => json!({"client_id": format!("oauth_{}", index), "scope": "read write"}),
            "jwt" => json!({"issuer": "https://auth.example.com", "audience": "api"}),
            "basic_auth" => json!({"username": format!("user_{}", index)}),
            _ => json!({}),
        };

        Ok(TestFederationClient {
            id: Uuid::new_v4(),
            client_name,
            client_id,
            api_endpoint: format!("https://api.{}.example.com/v1", index),
            auth_type,
            auth_config,
            webhook_url: Some(format!("https://webhook.{}.example.com/events", index)),
            status: "active".to_string(),
            rate_limit_per_minute: rng.gen_range(100..1000) as i32,
            rate_limit_per_hour: rng.gen_range(1000..10000) as i32,
            sla_uptime_percent: rust_decimal::Decimal::from_f32_retain(rng.gen_range(95.0..99.9)).unwrap(),
            sla_response_time_ms: rng.gen_range(100..3000) as i32,
            created_at: Utc::now() - ChronoDuration::days(rng.gen_range(1..365)),
            updated_at: Utc::now(),
        })
    }

    async fn generate_mcp_server(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, client_id: Uuid, index: u32) -> Result<()> {
        let mut rng = thread_rng();

        let server_types = vec![
            "data-processor", "file-converter", "email-service", "notification-hub",
            "analytics-engine", "backup-service", "webhook-relay", "task-scheduler"
        ];

        let server_type = server_types.choose(&mut rng).unwrap();
        let server_id = format!("{}-{}", server_type, index);
        let server_name = format!("{} Server {}", server_type.replace("-", " ").to_title_case(), index);

        let capabilities = json!([
            "data_processing",
            "real_time_updates",
            if rng.gen_bool(0.3) { "batch_processing" } else { "streaming" }
        ]);

        let tools = json!([
            {"name": format!("{}_tool", server_type), "version": "1.0"},
            {"name": "health_check", "version": "1.0"}
        ]);

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
            server_name,
            Some(format!("Test MCP server for {}", server_type)),
            format!("https://mcp.example.com/{}", server_id),
            Some("1.0.0".to_string()),
            rng.gen_bool(0.7),
            rng.gen_range(1..50) as i32,
            "active",
            capabilities,
            tools,
            Utc::now(),
            Utc::now()
        )
        .execute(&mut **tx)
        .await
        .context("Failed to insert MCP server")?;

        Ok(())
    }
}

// Helper structs for generated data
#[derive(Debug)]
struct TestUser {
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
struct TestWorkflow {
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
struct TestSubscription {
    id: Uuid,
    user_id: Uuid,
    plan_id: String,
    plan_name: String,
    status: String,
    billing_cycle: String,
    amount_cents: i32,
    currency: String,
    current_period_start: DateTime<Utc>,
    current_period_end: DateTime<Utc>,
    trial_end: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
struct TestFederationClient {
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

// Extension trait for string title case
trait ToTitleCase {
    fn to_title_case(&self) -> String;
}

impl ToTitleCase for str {
    fn to_title_case(&self) -> String {
        self.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
