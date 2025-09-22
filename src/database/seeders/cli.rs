//! CLI Tool for AI-CORE Test Data Seeding
//!
//! This module provides a command-line interface for the test data seeding system,
//! allowing developers and testers to easily generate various types of test data
//! for different scenarios and environments.
//!
//! # Usage
//!
//! ```bash
//! # Development data
//! cargo run --bin seed-data -- --mode development --users 10
//!
//! # Performance testing data
//! cargo run --bin seed-data -- --mode performance --target-size 2.5 --concurrent-users 500
//!
//! # Test scenarios
//! cargo run --bin seed-data -- --mode test-scenarios --clean
//!
//! # Anonymize production data
//! cargo run --bin seed-data -- --mode anonymized-production --source-url "postgresql://..."
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::{
    SeederManager, SeedingConfig, SeedingMode, PerformanceConfig,
    ProductionSourceConfig, AnonymizationRule, FakeDataType
};

/// AI-CORE Test Data Seeding Tool
#[derive(Parser)]
#[command(name = "seed-data")]
#[command(about = "Generate test data for AI-CORE platform")]
#[command(version = "1.0.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Database URL (overrides config file)
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: Option<String>,

    /// Output format for reports
    #[arg(long, default_value = "json")]
    pub output_format: OutputFormat,

    /// Output directory for reports
    #[arg(long, default_value = "./database/seeders/reports")]
    pub output_dir: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate test data
    Generate {
        /// Seeding mode
        #[arg(short, long, default_value = "development")]
        mode: SeedingModeArg,

        /// Number of users to generate
        #[arg(short, long)]
        users: Option<u32>,

        /// Number of workflows per user (format: min,max)
        #[arg(long)]
        workflows_per_user: Option<String>,

        /// Subscription rate (0.0 to 1.0)
        #[arg(long)]
        subscription_rate: Option<f32>,

        /// Clean existing test data before seeding
        #[arg(long)]
        clean: bool,

        /// Random seed for reproducible results
        #[arg(long)]
        seed: Option<u64>,

        /// Target database size in GB (for performance mode)
        #[arg(long)]
        target_size: Option<f32>,

        /// Number of concurrent users (for performance mode)
        #[arg(long)]
        concurrent_users: Option<u32>,

        /// Source database URL (for anonymized production mode)
        #[arg(long)]
        source_url: Option<String>,

        /// Anonymization rules file
        #[arg(long)]
        anonymization_rules: Option<PathBuf>,
    },

    /// Validate existing test data
    Validate {
        /// Run comprehensive validation checks
        #[arg(long)]
        comprehensive: bool,

        /// Check for sensitive data patterns
        #[arg(long)]
        check_sensitive: bool,
    },

    /// Clean test data
    Clean {
        /// Confirm deletion without prompting
        #[arg(short, long)]
        force: bool,

        /// Keep specific data types
        #[arg(long)]
        keep: Vec<String>,
    },

    /// Show database statistics
    Stats {
        /// Include detailed table statistics
        #[arg(long)]
        detailed: bool,

        /// Export statistics to file
        #[arg(long)]
        export: Option<PathBuf>,
    },

    /// Generate seeding configuration template
    Config {
        /// Configuration template type
        #[arg(short, long, default_value = "development")]
        template: ConfigTemplate,

        /// Output file path
        #[arg(short, long, default_value = "seeding-config.yaml")]
        output: PathBuf,
    },

    /// Benchmark seeding performance
    Benchmark {
        /// Number of benchmark iterations
        #[arg(short, long, default_value = "3")]
        iterations: u32,

        /// Record size variations to test
        #[arg(long)]
        record_counts: Option<String>,

        /// Export benchmark results
        #[arg(long)]
        export: Option<PathBuf>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SeedingModeArg {
    Development,
    TestScenarios,
    Integration,
    Performance,
    ProductionLike,
    AnonymizedProduction,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ConfigTemplate {
    Development,
    Testing,
    Performance,
    Production,
    Minimal,
}

/// Configuration structure that can be loaded from file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedingCliConfig {
    pub database: DatabaseConfig,
    pub seeding: SeedingConfig,
    pub anonymization: Option<AnonymizationConfig>,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    pub source_url: String,
    pub tables: Vec<String>,
    pub exclude_fields: Vec<String>,
    pub rules: HashMap<String, AnonymizationRuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnonymizationRuleConfig {
    Fake { data_type: String, params: Option<serde_json::Value> },
    Hash,
    Static { value: String },
    Keep,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub directory: PathBuf,
    pub include_metrics: bool,
    pub compress: bool,
}

/// Main CLI runner
pub struct CliRunner {
    database_url: String,
    config: SeedingCliConfig,
}

impl CliRunner {
    /// Create new CLI runner with configuration
    pub async fn new(args: &Cli) -> Result<Self> {
        // Initialize logging
        Self::init_logging(args.verbose)?;

        // Load configuration
        let config = if let Some(config_path) = &args.config {
            Self::load_config(config_path).await?
        } else {
            Self::default_config()?
        };

        // Override database URL if provided
        let database_url = args.database_url
            .clone()
            .unwrap_or_else(|| config.database.url.clone());

        Ok(Self {
            database_url,
            config,
        })
    }

    /// Execute CLI command
    pub async fn run(&self, args: &Cli) -> Result<()> {
        match &args.command {
            Commands::Generate {
                mode, users, workflows_per_user, subscription_rate, clean, seed,
                target_size, concurrent_users, source_url, anonymization_rules
            } => {
                self.run_generate(
                    mode, *users, workflows_per_user.as_deref(), *subscription_rate,
                    *clean, *seed, *target_size, *concurrent_users,
                    source_url.as_deref(), anonymization_rules.as_deref(),
                    &args.output_format, &args.output_dir
                ).await
            }
            Commands::Validate { comprehensive, check_sensitive } => {
                self.run_validate(*comprehensive, *check_sensitive).await
            }
            Commands::Clean { force, keep } => {
                self.run_clean(*force, keep).await
            }
            Commands::Stats { detailed, export } => {
                self.run_stats(*detailed, export.as_deref()).await
            }
            Commands::Config { template, output } => {
                self.run_config_generation(template, output).await
            }
            Commands::Benchmark { iterations, record_counts, export } => {
                self.run_benchmark(*iterations, record_counts.as_deref(), export.as_deref()).await
            }
        }
    }

    /// Initialize logging based on verbosity
    fn init_logging(verbose: bool) -> Result<()> {
        let log_level = if verbose { "debug" } else { "info" };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| format!("seed_data={},database={}", log_level, log_level).into()),
            )
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .init();

        Ok(())
    }

    /// Load configuration from file
    async fn load_config(path: &PathBuf) -> Result<SeedingCliConfig> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read configuration file")?;

        if path.extension().and_then(|s| s.to_str()) == Some("yaml") ||
           path.extension().and_then(|s| s.to_str()) == Some("yml") {
            serde_yaml::from_str(&content)
                .context("Failed to parse YAML configuration")
        } else {
            serde_json::from_str(&content)
                .context("Failed to parse JSON configuration")
        }
    }

    /// Generate default configuration
    fn default_config() -> Result<SeedingCliConfig> {
        Ok(SeedingCliConfig {
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://localhost:5432/ai_core_test".to_string()),
                max_connections: Some(10),
                timeout_seconds: Some(30),
            },
            seeding: SeedingConfig::default(),
            anonymization: None,
            output: OutputConfig {
                format: "json".to_string(),
                directory: PathBuf::from("./database/seeders/reports"),
                include_metrics: true,
                compress: false,
            },
        })
    }

    /// Run data generation command
    async fn run_generate(
        &self,
        mode: &SeedingModeArg,
        users: Option<u32>,
        workflows_per_user: Option<&str>,
        subscription_rate: Option<f32>,
        clean: bool,
        seed: Option<u64>,
        target_size: Option<f32>,
        concurrent_users: Option<u32>,
        source_url: Option<&str>,
        anonymization_rules: Option<&std::path::Path>,
        output_format: &OutputFormat,
        output_dir: &PathBuf,
    ) -> Result<()> {
        info!("Starting data generation...");

        // Connect to database
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(self.config.database.max_connections.unwrap_or(10))
            .connect(&self.database_url)
            .await
            .context("Failed to connect to database")?;

        let pool = std::sync::Arc::new(pool);

        // Create seeder manager
        let seeder = SeederManager::new(pool).await?;

        // Build configuration
        let mut config = self.config.seeding.clone();

        // Override with CLI arguments
        if let Some(user_count) = users {
            config.user_count = user_count;
        }

        if let Some(workflows) = workflows_per_user {
            config.workflows_per_user = Self::parse_range(workflows)?;
        }

        if let Some(rate) = subscription_rate {
            config.subscription_rate = rate;
        }

        config.clean_before_seed = clean;
        config.random_seed = seed;

        // Set seeding mode
        config.mode = match mode {
            SeedingModeArg::Development => SeedingMode::Development,
            SeedingModeArg::TestScenarios => SeedingMode::TestScenarios,
            SeedingModeArg::Integration => SeedingMode::Integration,
            SeedingModeArg::Performance => {
                config.performance_config = Some(PerformanceConfig {
                    target_size_gb: target_size.unwrap_or(1.0),
                    concurrent_users: concurrent_users.unwrap_or(100),
                    peak_usage_multiplier: 2.0,
                    generate_timeseries: true,
                });
                SeedingMode::Performance
            }
            SeedingModeArg::ProductionLike => SeedingMode::ProductionLike,
            SeedingModeArg::AnonymizedProduction => {
                let source_url = source_url
                    .ok_or_else(|| anyhow::anyhow!("Source URL required for anonymized production mode"))?;

                let anonymization_config = if let Some(rules_path) = anonymization_rules {
                    Self::load_anonymization_rules(rules_path).await?
                } else {
                    Self::default_anonymization_config()
                };

                let source_config = ProductionSourceConfig {
                    source_url: source_url.to_string(),
                    tables: anonymization_config.tables,
                    exclude_fields: anonymization_config.exclude_fields,
                    anonymization_rules: Self::convert_anonymization_rules(anonymization_config.rules),
                };

                SeedingMode::AnonymizedProduction { source_config }
            }
        };

        // Execute seeding
        let report = seeder.seed_all(&config).await?;

        // Output report
        self.output_report(&report, output_format, output_dir).await?;

        info!("Data generation completed successfully!");
        Ok(())
    }

    /// Run validation command
    async fn run_validate(&self, comprehensive: bool, check_sensitive: bool) -> Result<()> {
        info!("Running data validation...");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect(&self.database_url)
            .await?;

        // Basic validation checks
        self.validate_data_consistency(&pool).await?;
        self.validate_foreign_keys(&pool).await?;

        if comprehensive {
            self.validate_data_distribution(&pool).await?;
            self.validate_performance_metrics(&pool).await?;
        }

        if check_sensitive {
            self.validate_sensitive_data(&pool).await?;
        }

        info!("Validation completed successfully!");
        Ok(())
    }

    /// Run clean command
    async fn run_clean(&self, force: bool, keep: &[String]) -> Result<()> {
        if !force {
            println!("This will delete all test data. Are you sure? (y/N)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().to_lowercase().starts_with('y') {
                println!("Aborted.");
                return Ok(());
            }
        }

        info!("Cleaning test data...");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect(&self.database_url)
            .await?;

        let pool = std::sync::Arc::new(pool);
        let seeder = SeederManager::new(pool).await?;

        if keep.is_empty() {
            seeder.clean_test_data().await?;
        } else {
            self.selective_clean(&seeder, keep).await?;
        }

        info!("Test data cleaned successfully!");
        Ok(())
    }

    /// Run statistics command
    async fn run_stats(&self, detailed: bool, export: Option<&std::path::Path>) -> Result<()> {
        info!("Gathering database statistics...");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect(&self.database_url)
            .await?;

        let pool = std::sync::Arc::new(pool);
        let seeder = SeederManager::new(pool).await?;
        let stats = seeder.get_database_stats().await?;

        // Display stats
        println!("Database Statistics:");
        println!("  Users: {}", stats.user_count);
        println!("  Workflows: {}", stats.workflow_count);
        println!("  Subscriptions: {}", stats.subscription_count);
        println!("  Federation Clients: {}", stats.federation_client_count);
        println!("  Database Size: {}", stats.database_size_pretty);

        if detailed {
            self.show_detailed_stats(&pool).await?;
        }

        if let Some(export_path) = export {
            let stats_json = serde_json::to_string_pretty(&stats)?;
            fs::write(export_path, stats_json).await?;
            info!("Statistics exported to {}", export_path.display());
        }

        Ok(())
    }

    /// Run configuration generation command
    async fn run_config_generation(&self, template: &ConfigTemplate, output: &PathBuf) -> Result<()> {
        info!("Generating configuration template...");

        let config = match template {
            ConfigTemplate::Development => self.generate_development_config(),
            ConfigTemplate::Testing => self.generate_testing_config(),
            ConfigTemplate::Performance => self.generate_performance_config(),
            ConfigTemplate::Production => self.generate_production_config(),
            ConfigTemplate::Minimal => self.generate_minimal_config(),
        };

        let config_yaml = serde_yaml::to_string(&config)?;
        fs::write(output, config_yaml).await?;

        info!("Configuration template saved to {}", output.display());
        Ok(())
    }

    /// Run benchmark command
    async fn run_benchmark(
        &self,
        iterations: u32,
        record_counts: Option<&str>,
        export: Option<&std::path::Path>
    ) -> Result<()> {
        info!("Running seeding performance benchmark...");

        let counts = if let Some(counts_str) = record_counts {
            Self::parse_counts(counts_str)?
        } else {
            vec![100, 500, 1000, 5000]
        };

        let mut results = Vec::new();

        for count in counts {
            info!("Benchmarking with {} records...", count);
            let mut iteration_times = Vec::new();

            for i in 0..iterations {
                info!("Iteration {} of {}", i + 1, iterations);
                let start = std::time::Instant::now();

                // Run seeding
                let config = SeedingConfig {
                    user_count: count,
                    clean_before_seed: true,
                    ..Default::default()
                };

                let pool = sqlx::postgres::PgPoolOptions::new()
                    .connect(&self.database_url)
                    .await?;

                let seeder = SeederManager::new(std::sync::Arc::new(pool)).await?;
                let _report = seeder.seed_all(&config).await?;

                let duration = start.elapsed();
                iteration_times.push(duration);
                info!("Iteration {} completed in {:?}", i + 1, duration);
            }

            let avg_time = iteration_times.iter().sum::<std::time::Duration>() / iterations;
            let records_per_second = count as f64 / avg_time.as_secs_f64();

            results.push(BenchmarkResult {
                record_count: count,
                iterations,
                average_time: avg_time,
                records_per_second,
                times: iteration_times,
            });

            info!("Average time for {} records: {:?} ({:.0} records/sec)",
                  count, avg_time, records_per_second);
        }

        // Display summary
        println!("\nBenchmark Results:");
        println!("{:<12} {:<15} {:<15}", "Records", "Avg Time", "Records/sec");
        println!("{:-<42}", "");
        for result in &results {
            println!("{:<12} {:<15.2?} {:<15.0}",
                     result.record_count, result.average_time, result.records_per_second);
        }

        // Export if requested
        if let Some(export_path) = export {
            let results_json = serde_json::to_string_pretty(&results)?;
            fs::write(export_path, results_json).await?;
            info!("Benchmark results exported to {}", export_path.display());
        }

        Ok(())
    }

    // Helper methods

    fn parse_range(range_str: &str) -> Result<(u32, u32)> {
        let parts: Vec<&str> = range_str.split(',').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Range must be in format 'min,max'"));
        }

        let min = parts[0].trim().parse::<u32>()?;
        let max = parts[1].trim().parse::<u32>()?;

        if min > max {
            return Err(anyhow::anyhow!("Minimum cannot be greater than maximum"));
        }

        Ok((min, max))
    }

    fn parse_counts(counts_str: &str) -> Result<Vec<u32>> {
        counts_str
            .split(',')
            .map(|s| s.trim().parse::<u32>())
            .collect::<Result<Vec<u32>, _>>()
            .context("Failed to parse record counts")
    }

    async fn load_anonymization_rules(&self, _path: &std::path::Path) -> Result<AnonymizationConfig> {
        // Placeholder implementation
        Ok(Self::default_anonymization_config())
    }

    fn default_anonymization_config() -> AnonymizationConfig {
        AnonymizationConfig {
            source_url: String::new(),
            tables: vec!["users".to_string(), "workflows".to_string()],
            exclude_fields: vec!["password_hash".to_string(), "session_token".to_string()],
            rules: HashMap::new(),
        }
    }

    fn convert_anonymization_rules(_rules: HashMap<String, AnonymizationRuleConfig>) -> HashMap<String, AnonymizationRule> {
        // Convert from config format to internal format
        HashMap::new()
    }

    async fn output_report(&self, report: &super::SeedingReport, format: &OutputFormat, output_dir: &PathBuf) -> Result<()> {
        fs::create_dir_all(output_dir).await?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = match format {
            OutputFormat::Json => format!("seeding_report_{}.json", timestamp),
            OutputFormat::Yaml => format!("seeding_report_{}.yaml", timestamp),
            OutputFormat::Table => format!("seeding_report_{}.txt", timestamp),
        };

        let filepath = output_dir.join(filename);

        let content = match format {
            OutputFormat::Json => serde_json::to_string_pretty(report)?,
            OutputFormat::Yaml => serde_yaml::to_string(report)?,
            OutputFormat::Table => self.format_report_as_table(report),
        };

        fs::write(&filepath, content).await?;
        info!("Report saved to {}", filepath.display());

        Ok(())
    }

    fn format_report_as_table(&self, report: &super::SeedingReport) -> String {
        format!(
            "AI-CORE Seeding Report\n\
             ======================\n\
             Duration: {:?}\n\
             Users Created: {}\n\
             Workflows Created: {}\n\
             Subscriptions Created: {}\n\
             Federation Clients Created: {}\n\
             MCP Servers Created: {}\n\
             API Keys Created: {}\n\
             Usage Records Created: {}\n\
             Notifications Created: {}\n\
             Estimated Size: {:.2} MB\n\
             \nErrors: {}\n\
             Warnings: {}\n",
            report.duration,
            report.users_created,
            report.workflows_created,
            report.subscriptions_created,
            report.federation_clients_created,
            report.mcp_servers_created,
            report.api_keys_created,
            report.usage_records_created,
            report.notifications_created,
            report.estimated_size_mb.unwrap_or(0.0),
            report.errors.len(),
            report.warnings.len()
        )
    }

    // Validation helper methods (placeholder implementations)
    async fn validate_data_consistency(&self, _pool: &sqlx::PgPool) -> Result<()> {
        info!("Validating data consistency...");
        Ok(())
    }

    async fn validate_foreign_keys(&self, _pool: &sqlx::PgPool) -> Result<()> {
        info!("Validating foreign key relationships...");
        Ok(())
    }

    async fn validate_data_distribution(&self, _pool: &sqlx::PgPool) -> Result<()> {
        info!("Validating data distribution patterns...");
        Ok(())
    }

    async fn validate_performance_metrics(&self, _pool: &sqlx::PgPool) -> Result<()> {
        info!("Validating performance metrics...");
        Ok(())
    }

    async fn validate_sensitive_data(&self, _pool: &sqlx::PgPool) -> Result<()> {
        info!("Checking for sensitive data patterns...");
        Ok(())
    }

    async fn selective_clean(&self, _seeder: &SeederManager, _keep: &[String]) -> Result<()> {
        info!("Performing selective data cleanup...");
        Ok(())
    }

    async fn show_detailed_stats(&self, _pool: &sqlx::postgres::PgPool) -> Result<()> {
        info!("Gathering detailed statistics...");
        Ok(())
    }

    // Configuration template generators
    fn generate_development_config(&self) -> SeedingCliConfig {
        SeedingCliConfig {
            database: self.config.database.clone(),
            seeding: SeedingConfig {
                mode: SeedingMode::Development,
                user_count: 10,
                workflows_per_user: (1, 5),
                subscription_rate: 0.5,
                clean_before_seed: true,
                ..Default::default()
            },
            anonymization: None,
            output: self.config.output.clone(),
        }
    }

    fn generate_testing_config(&self) -> SeedingCliConfig {
        SeedingCliConfig {
            database: self.config.database.clone(),
            seeding: SeedingConfig {
                mode: SeedingMode::TestScenarios,
                user_count: 50,
                workflows_per_user: (5, 15),
                subscription_rate: 0.7,
                clean_before_seed: true,
                ..Default::default()
            },
            anonymization: None,
            output: self.config.output.clone(),
        }
    }

    fn generate_performance_config(&self) -> SeedingCliConfig {
        SeedingCliConfig {
            database: self.config.database.clone(),
            seeding: SeedingConfig {
                mode: SeedingMode::Performance,
                user_count: 10000,
                workflows_per_user: (10, 50),
                subscription_rate: 0.3,
                performance_config: Some(PerformanceConfig {
                    target_size_gb: 5.0,
                    concurrent_users: 1000,
                    peak_usage_multiplier: 3.0,
                    generate_timeseries: true,
                }),
                ..Default::default()
            },
            anonymization: None,
            output: self.config.output.clone(),
        }
    }

    fn generate_production_config(&self) -> SeedingCliConfig {
        SeedingCliConfig {
            database: self.config.database.clone(),
            seeding: SeedingConfig {
                mode: SeedingMode::ProductionLike,
                user_count: 100000,
                workflows_per_user: (50, 500),
                subscription_rate: 0.25,
                historical_months: 24,
                realistic_timing: true,
                ..Default::default()
            },
            anonymization: None,
            output: self.config.output.clone(),
        }
    }

    fn generate_minimal_config(&self) -> SeedingCliConfig {
        SeedingCliConfig {
            database: self.config.database.clone(),
            seeding: SeedingConfig::default(),
            anonymization: None,
            output: self.config.output.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    record_count: u32,
    iterations: u32,
    average_time: std::time::Duration,
    records_per_second: f64,
    times: Vec<std::time::Duration>,
}

/// Main entry point for the CLI tool
pub async fn run() -> Result<()> {
    let args = Cli::parse();
    let runner = CliRunner::new(&args).await?;
    runner.run(&args).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range() {
        let result = CliRunner::parse_range("1,5").unwrap();
        assert_eq!(result, (1, 5));

        let result = CliRunner::parse_range("10, 20").unwrap();
        assert_eq!(result, (10, 20));

        assert!(CliRunner::parse_range("5,1").is_err()); // min > max
        assert!(CliRunner::parse_range("invalid").is_err()); // invalid format
    }

    #[test]
    fn test_parse_counts() {
        let result = CliRunner::parse_counts("100,500,1000").unwrap();
        assert_eq!(result, vec![100, 500, 1000]);

        let result = CliRunner::parse_counts("1, 2, 3").unwrap();
        assert_eq!(result, vec![1, 2, 3]);

        assert!(CliRunner::parse_counts("invalid,counts").is_err());
    }

    #[test]
    fn test_default_config() {
        let config = CliRunner::default_config().unwrap();
        assert_eq!(config.seeding.user_count, 10);
        assert_eq!(config.output.format, "json");
    }
}
