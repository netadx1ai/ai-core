//! Test Data Seeding System for AI-CORE
//!
//! This module provides comprehensive test data generation capabilities including:
//! - Synthetic test data generators
//! - Production data anonymization
//! - Test scenario data sets
//! - Performance test data volumes
//!
//! # Usage
//!
//! ```rust
//! use database::seeders::{SeederManager, SeedingConfig, SeedingMode};
//!
//! let config = SeedingConfig {
//!     mode: SeedingMode::TestScenarios,
//!     user_count: 100,
//!     ..Default::default()
//! };
//!
//! let seeder = SeederManager::new(db_manager).await?;
//! seeder.seed_all(&config).await?;
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use fake::{Fake, Faker};
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use fake::faker::company::en::*;
use fake::faker::lorem::en::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, debug};
use uuid::Uuid;

pub mod generators;
pub mod anonymizers;
pub mod scenarios;
pub mod performance;

use generators::*;
use anonymizers::*;
use scenarios::*;
use performance::*;

/// Configuration for the seeding process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedingConfig {
    /// Seeding mode determines the type and volume of data to generate
    pub mode: SeedingMode,
    /// Number of users to generate
    pub user_count: u32,
    /// Number of workflows per user (randomized within range)
    pub workflows_per_user: (u32, u32),
    /// Number of API keys per user (randomized within range)
    pub api_keys_per_user: (u32, u32),
    /// Percentage of users with active subscriptions
    pub subscription_rate: f32,
    /// Number of federation clients to create
    pub federation_clients_count: u32,
    /// Number of MCP servers per client
    pub mcp_servers_per_client: (u32, u32),
    /// Date range for historical data (months back from now)
    pub historical_months: u32,
    /// Whether to clean existing test data before seeding
    pub clean_before_seed: bool,
    /// Whether to use realistic timing patterns
    pub realistic_timing: bool,
    /// Custom seed for reproducible results
    pub random_seed: Option<u64>,
    /// Performance test specific settings
    pub performance_config: Option<PerformanceConfig>,
}

/// Different modes of seeding for various testing scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeedingMode {
    /// Development mode: Small dataset for local development
    Development,
    /// Test scenarios: Focused datasets for specific test cases
    TestScenarios,
    /// Integration testing: Medium dataset for integration tests
    Integration,
    /// Performance testing: Large dataset for load/stress testing
    Performance,
    /// Production-like: Large, realistic dataset mimicking production
    ProductionLike,
    /// Anonymized production: Real production data with anonymization
    AnonymizedProduction { source_config: ProductionSourceConfig },
}

/// Configuration for performance testing data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Target database size in GB
    pub target_size_gb: f32,
    /// Concurrent users simulation
    pub concurrent_users: u32,
    /// Peak usage hours simulation
    pub peak_usage_multiplier: f32,
    /// Generate time-series data for analytics
    pub generate_timeseries: bool,
}

/// Configuration for production data anonymization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionSourceConfig {
    /// Source database connection string
    pub source_url: String,
    /// Tables to anonymize (empty = all tables)
    pub tables: Vec<String>,
    /// Fields to completely remove
    pub exclude_fields: Vec<String>,
    /// Custom anonymization rules
    pub anonymization_rules: HashMap<String, AnonymizationRule>,
}

/// Rules for anonymizing specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnonymizationRule {
    /// Replace with fake data of same type
    Fake(FakeDataType),
    /// Hash the original value
    Hash,
    /// Replace with static value
    Static(String),
    /// Keep original (for non-sensitive data)
    Keep,
    /// Remove completely
    Remove,
}

/// Types of fake data that can be generated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FakeDataType {
    Name,
    Email,
    Phone,
    Address,
    Company,
    Lorem(u32), // word count
    Number(i64, i64), // min, max
    Date(i64), // days from now
    Uuid,
}

/// Main seeder manager that coordinates all seeding operations
pub struct SeederManager {
    pool: Arc<PgPool>,
    user_generator: UserGenerator,
    workflow_generator: WorkflowGenerator,
    billing_generator: BillingGenerator,
    federation_generator: FederationGenerator,
    performance_generator: PerformanceGenerator,
    anonymizer: DataAnonymizer,
}

impl SeederManager {
    /// Create a new seeder manager
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        Ok(Self {
            pool: pool.clone(),
            user_generator: UserGenerator::new(pool.clone()),
            workflow_generator: WorkflowGenerator::new(pool.clone()),
            billing_generator: BillingGenerator::new(pool.clone()),
            federation_generator: FederationGenerator::new(pool.clone()),
            performance_generator: PerformanceGenerator::new(pool.clone()),
            anonymizer: DataAnonymizer::new(pool.clone()),
        })
    }

    /// Seed all data according to the configuration
    pub async fn seed_all(&self, config: &SeedingConfig) -> Result<SeedingReport> {
        info!("Starting data seeding with config: {:?}", config);
        let start_time = Utc::now();

        // Set random seed for reproducible results
        if let Some(seed) = config.random_seed {
            // Note: This would need a global RNG state management
            info!("Using random seed: {}", seed);
        }

        // Clean existing test data if requested
        if config.clean_before_seed {
            self.clean_test_data().await?;
        }

        let mut report = SeedingReport::new();

        match &config.mode {
            SeedingMode::Development => {
                report = self.seed_development_data(config).await?;
            }
            SeedingMode::TestScenarios => {
                report = self.seed_test_scenarios(config).await?;
            }
            SeedingMode::Integration => {
                report = self.seed_integration_data(config).await?;
            }
            SeedingMode::Performance => {
                report = self.seed_performance_data(config).await?;
            }
            SeedingMode::ProductionLike => {
                report = self.seed_production_like_data(config).await?;
            }
            SeedingMode::AnonymizedProduction { source_config } => {
                report = self.seed_anonymized_production_data(config, source_config).await?;
            }
        }

        report.duration = Utc::now() - start_time;
        info!("Data seeding completed in {:?}", report.duration);

        // Generate summary report
        self.generate_seeding_summary(&report).await?;

        Ok(report)
    }

    /// Clean all test data from the database
    pub async fn clean_test_data(&self) -> Result<()> {
        info!("Cleaning existing test data...");

        let mut tx = self.pool.begin().await?;

        // Delete in reverse dependency order to avoid FK constraints
        sqlx::query("DELETE FROM audit_logs WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM notifications WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM scheduled_tasks WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM usage_records WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM invoices WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM subscriptions WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM workflows WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM user_sessions WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM api_keys WHERE user_id IN (SELECT id FROM users WHERE email LIKE '%@test.example.com')")
            .execute(&mut *tx).await?;

        // Delete test federation data
        sqlx::query("DELETE FROM mcp_servers WHERE client_id IN (SELECT id FROM federation_clients WHERE client_name LIKE 'Test_%')")
            .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM federation_clients WHERE client_name LIKE 'Test_%'")
            .execute(&mut *tx).await?;

        // Finally delete test users
        sqlx::query("DELETE FROM users WHERE email LIKE '%@test.example.com' OR username LIKE 'test_%'")
            .execute(&mut *tx).await?;

        tx.commit().await?;
        info!("Test data cleaned successfully");
        Ok(())
    }

    /// Seed data for development environment
    async fn seed_development_data(&self, config: &SeedingConfig) -> Result<SeedingReport> {
        let mut report = SeedingReport::new();

        // Generate small dataset for development
        let dev_config = SeedingConfig {
            user_count: 10,
            workflows_per_user: (1, 5),
            api_keys_per_user: (0, 2),
            subscription_rate: 0.5,
            federation_clients_count: 3,
            mcp_servers_per_client: (1, 3),
            historical_months: 1,
            ..config.clone()
        };

        report.users_created = self.user_generator.generate_users(&dev_config).await?;
        report.workflows_created = self.workflow_generator.generate_workflows(&dev_config).await?;
        report.subscriptions_created = self.billing_generator.generate_subscriptions(&dev_config).await?;
        report.federation_clients_created = self.federation_generator.generate_clients(&dev_config).await?;

        Ok(report)
    }

    /// Seed data for test scenarios
    async fn seed_test_scenarios(&self, config: &SeedingConfig) -> Result<SeedingReport> {
        let mut report = SeedingReport::new();

        // Generate specific test scenarios
        report.users_created = self.user_generator.generate_test_scenario_users(config).await?;
        report.workflows_created = self.workflow_generator.generate_test_scenario_workflows(config).await?;
        report.subscriptions_created = self.billing_generator.generate_test_scenario_billing(config).await?;

        Ok(report)
    }

    /// Seed data for integration testing
    async fn seed_integration_data(&self, config: &SeedingConfig) -> Result<SeedingReport> {
        let mut report = SeedingReport::new();

        let integration_config = SeedingConfig {
            user_count: 50,
            workflows_per_user: (5, 20),
            api_keys_per_user: (1, 3),
            subscription_rate: 0.7,
            federation_clients_count: 10,
            mcp_servers_per_client: (2, 8),
            historical_months: 3,
            ..config.clone()
        };

        report.users_created = self.user_generator.generate_users(&integration_config).await?;
        report.workflows_created = self.workflow_generator.generate_workflows(&integration_config).await?;
        report.subscriptions_created = self.billing_generator.generate_subscriptions(&integration_config).await?;
        report.federation_clients_created = self.federation_generator.generate_clients(&integration_config).await?;

        Ok(report)
    }

    /// Seed data for performance testing
    async fn seed_performance_data(&self, config: &SeedingConfig) -> Result<SeedingReport> {
        let mut report = SeedingReport::new();

        if let Some(perf_config) = &config.performance_config {
            report = self.performance_generator.generate_performance_data(config, perf_config).await?;
        } else {
            warn!("Performance config not provided, using defaults");
            let default_perf_config = PerformanceConfig {
                target_size_gb: 1.0,
                concurrent_users: 100,
                peak_usage_multiplier: 3.0,
                generate_timeseries: true,
            };
            report = self.performance_generator.generate_performance_data(config, &default_perf_config).await?;
        }

        Ok(report)
    }

    /// Seed production-like data
    async fn seed_production_like_data(&self, config: &SeedingConfig) -> Result<SeedingReport> {
        let mut report = SeedingReport::new();

        let prod_config = SeedingConfig {
            user_count: 1000,
            workflows_per_user: (10, 100),
            api_keys_per_user: (0, 5),
            subscription_rate: 0.3,
            federation_clients_count: 50,
            mcp_servers_per_client: (5, 20),
            historical_months: 12,
            realistic_timing: true,
            ..config.clone()
        };

        report.users_created = self.user_generator.generate_users(&prod_config).await?;
        report.workflows_created = self.workflow_generator.generate_workflows(&prod_config).await?;
        report.subscriptions_created = self.billing_generator.generate_subscriptions(&prod_config).await?;
        report.federation_clients_created = self.federation_generator.generate_clients(&prod_config).await?;

        Ok(report)
    }

    /// Seed anonymized production data
    async fn seed_anonymized_production_data(
        &self,
        config: &SeedingConfig,
        source_config: &ProductionSourceConfig
    ) -> Result<SeedingReport> {
        let mut report = SeedingReport::new();

        info!("Starting anonymized production data import...");
        report = self.anonymizer.anonymize_and_import(config, source_config).await?;

        Ok(report)
    }

    /// Generate a comprehensive seeding summary report
    async fn generate_seeding_summary(&self, report: &SeedingReport) -> Result<()> {
        info!("=== Data Seeding Summary ===");
        info!("Duration: {:?}", report.duration);
        info!("Users created: {}", report.users_created);
        info!("Workflows created: {}", report.workflows_created);
        info!("Subscriptions created: {}", report.subscriptions_created);
        info!("Federation clients created: {}", report.federation_clients_created);
        info!("MCP servers created: {}", report.mcp_servers_created);
        info!("API keys created: {}", report.api_keys_created);
        info!("Usage records created: {}", report.usage_records_created);
        info!("Notifications created: {}", report.notifications_created);

        if let Some(size_mb) = report.estimated_size_mb {
            info!("Estimated database size: {:.2} MB", size_mb);
        }

        if !report.errors.is_empty() {
            warn!("Errors encountered during seeding:");
            for error in &report.errors {
                warn!("  - {}", error);
            }
        }

        // Save detailed report to file
        let report_json = serde_json::to_string_pretty(report)?;
        tokio::fs::write(
            format!("database/seeders/reports/seeding_report_{}.json", Utc::now().format("%Y%m%d_%H%M%S")),
            report_json
        ).await.context("Failed to write seeding report")?;

        Ok(())
    }

    /// Get current database statistics after seeding
    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let mut stats = DatabaseStats::default();

        // Count records in each table
        let row = sqlx::query("SELECT COUNT(*) FROM users").fetch_one(&*self.pool).await?;
        stats.user_count = row.try_get::<i64, _>(0)? as u32;

        let row = sqlx::query("SELECT COUNT(*) FROM workflows").fetch_one(&*self.pool).await?;
        stats.workflow_count = row.try_get::<i64, _>(0)? as u32;

        let row = sqlx::query("SELECT COUNT(*) FROM subscriptions").fetch_one(&*self.pool).await?;
        stats.subscription_count = row.try_get::<i64, _>(0)? as u32;

        let row = sqlx::query("SELECT COUNT(*) FROM federation_clients").fetch_one(&*self.pool).await?;
        stats.federation_client_count = row.try_get::<i64, _>(0)? as u32;

        // Calculate database size (approximate)
        let size_query = r#"
            SELECT pg_size_pretty(pg_database_size(current_database())) as size,
                   pg_database_size(current_database()) as size_bytes
        "#;
        let row = sqlx::query(size_query).fetch_one(&*self.pool).await?;
        stats.database_size_pretty = row.try_get("size")?;
        stats.database_size_bytes = row.try_get::<i64, _>("size_bytes")? as u64;

        Ok(stats)
    }
}

/// Report generated after seeding operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedingReport {
    pub duration: ChronoDuration,
    pub users_created: u32,
    pub workflows_created: u32,
    pub subscriptions_created: u32,
    pub federation_clients_created: u32,
    pub mcp_servers_created: u32,
    pub api_keys_created: u32,
    pub usage_records_created: u32,
    pub notifications_created: u32,
    pub estimated_size_mb: Option<f64>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SeedingReport {
    pub fn new() -> Self {
        Self {
            duration: ChronoDuration::zero(),
            users_created: 0,
            workflows_created: 0,
            subscriptions_created: 0,
            federation_clients_created: 0,
            mcp_servers_created: 0,
            api_keys_created: 0,
            usage_records_created: 0,
            notifications_created: 0,
            estimated_size_mb: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Current database statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub user_count: u32,
    pub workflow_count: u32,
    pub subscription_count: u32,
    pub federation_client_count: u32,
    pub mcp_server_count: u32,
    pub database_size_pretty: String,
    pub database_size_bytes: u64,
}

/// Default configuration for different environments
impl Default for SeedingConfig {
    fn default() -> Self {
        Self {
            mode: SeedingMode::Development,
            user_count: 10,
            workflows_per_user: (1, 5),
            api_keys_per_user: (0, 2),
            subscription_rate: 0.5,
            federation_clients_count: 3,
            mcp_servers_per_client: (1, 3),
            historical_months: 1,
            clean_before_seed: true,
            realistic_timing: false,
            random_seed: None,
            performance_config: None,
        }
    }
}

/// Utility functions for seeding operations
pub mod utils {
    use super::*;

    /// Generate a realistic timestamp within a given range
    pub fn random_timestamp_in_range(start: DateTime<Utc>, end: DateTime<Utc>) -> DateTime<Utc> {
        let mut rng = thread_rng();
        let start_timestamp = start.timestamp();
        let end_timestamp = end.timestamp();
        let random_timestamp = rng.gen_range(start_timestamp..=end_timestamp);
        DateTime::from_timestamp(random_timestamp, 0).unwrap_or(Utc::now())
    }

    /// Generate weighted random choice
    pub fn weighted_choice<T: Clone>(choices: &[(T, f32)]) -> Option<T> {
        let mut rng = thread_rng();
        let total_weight: f32 = choices.iter().map(|(_, weight)| weight).sum();
        let mut random_weight = rng.gen_range(0.0..total_weight);

        for (choice, weight) in choices {
            random_weight -= weight;
            if random_weight <= 0.0 {
                return Some(choice.clone());
            }
        }
        None
    }

    /// Generate realistic business hours timestamp
    pub fn random_business_hours_timestamp(days_back: i64) -> DateTime<Utc> {
        let mut rng = thread_rng();
        let base_date = Utc::now() - ChronoDuration::days(days_back);

        // Business hours: 9 AM to 6 PM
        let hour = rng.gen_range(9..18);
        let minute = rng.gen_range(0..60);

        base_date
            .date_naive()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
            .and_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn test_seeder_manager_creation(pool: PgPool) {
        let pool = Arc::new(pool);
        let seeder = SeederManager::new(pool).await;
        assert!(seeder.is_ok());
    }

    #[test]
    fn test_seeding_config_defaults() {
        let config = SeedingConfig::default();
        assert_eq!(config.user_count, 10);
        assert_eq!(config.workflows_per_user, (1, 5));
        assert!(matches!(config.mode, SeedingMode::Development));
    }

    #[test]
    fn test_weighted_choice() {
        let choices = vec![
            ("A".to_string(), 0.5),
            ("B".to_string(), 0.3),
            ("C".to_string(), 0.2),
        ];

        let result = utils::weighted_choice(&choices);
        assert!(result.is_some());
        assert!(["A", "B", "C"].contains(&result.unwrap().as_str()));
    }
}
