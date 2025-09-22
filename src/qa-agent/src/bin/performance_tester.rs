//! Performance Tester Binary
//!
//! Standalone binary for executing performance tests, load testing, and benchmarks
//! for the AI-CORE platform services.

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use qa_agent::config::{
    BenchmarkConfig, LoadTestingConfig, PerformanceConfig, QAConfig, RampUpPattern, SLAThresholds,
};
use qa_agent::performance::{PerformanceScenario, PerformanceTestResult, PerformanceTester};
use serde_json;
use std::path::PathBuf;
use std::time::Duration;
use tokio;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("performance_tester=info".parse()?),
        )
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::rfc_3339())
        .init();

    info!("🚀 AI-CORE Performance Tester v0.1.0");

    let matches = build_cli().get_matches();
    let config = load_configuration(&matches).await?;

    match matches.subcommand() {
        Some(("load-test", sub_matches)) => {
            handle_load_test(config, sub_matches).await?;
        }
        Some(("benchmark", sub_matches)) => {
            handle_benchmark(config, sub_matches).await?;
        }
        Some(("stress-test", sub_matches)) => {
            handle_stress_test(config, sub_matches).await?;
        }
        Some(("validate-sla", sub_matches)) => {
            handle_sla_validation(config, sub_matches).await?;
        }
        Some(("report", sub_matches)) => {
            handle_report_generation(config, sub_matches).await?;
        }
        Some(("continuous", sub_matches)) => {
            handle_continuous_testing(config, sub_matches).await?;
        }
        _ => {
            handle_interactive_mode(config).await?;
        }
    }

    Ok(())
}

fn build_cli() -> Command {
    Command::new("performance-tester")
        .version("0.1.0")
        .author("AI-CORE Team")
        .about("Comprehensive performance testing for AI-CORE platform")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/qa.toml"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity level"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("DIR")
                .help("Output directory for reports")
                .default_value("target/performance-reports"),
        )
        .subcommand(
            Command::new("load-test")
                .about("Execute load testing scenarios")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .value_name("URL")
                        .help("Target service URL")
                        .required(true),
                )
                .arg(
                    Arg::new("users")
                        .short('u')
                        .long("users")
                        .value_name("NUM")
                        .help("Number of concurrent users")
                        .default_value("10"),
                )
                .arg(
                    Arg::new("duration")
                        .short('d')
                        .long("duration")
                        .value_name("SECONDS")
                        .help("Test duration in seconds")
                        .default_value("60"),
                )
                .arg(
                    Arg::new("ramp-up")
                        .short('r')
                        .long("ramp-up")
                        .value_name("SECONDS")
                        .help("Ramp-up period in seconds")
                        .default_value("10"),
                ),
        )
        .subcommand(
            Command::new("benchmark")
                .about("Run performance benchmarks")
                .arg(
                    Arg::new("suite")
                        .short('s')
                        .long("suite")
                        .value_name("NAME")
                        .help("Benchmark suite name")
                        .value_parser(["api", "database", "full", "custom"])
                        .default_value("api"),
                )
                .arg(
                    Arg::new("iterations")
                        .short('i')
                        .long("iterations")
                        .value_name("NUM")
                        .help("Number of benchmark iterations")
                        .default_value("1000"),
                ),
        )
        .subcommand(
            Command::new("stress-test")
                .about("Execute stress testing scenarios")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .value_name("URL")
                        .help("Target service URL")
                        .required(true),
                )
                .arg(
                    Arg::new("max-users")
                        .short('m')
                        .long("max-users")
                        .value_name("NUM")
                        .help("Maximum number of users")
                        .default_value("1000"),
                ),
        )
        .subcommand(
            Command::new("validate-sla")
                .about("Validate SLA compliance")
                .arg(
                    Arg::new("sla-file")
                        .short('f')
                        .long("sla-file")
                        .value_name("FILE")
                        .help("SLA configuration file")
                        .default_value("config/sla.toml"),
                ),
        )
        .subcommand(
            Command::new("report")
                .about("Generate performance reports")
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_name("FORMAT")
                        .help("Report format")
                        .value_parser(["html", "json", "csv", "prometheus"])
                        .default_value("html"),
                )
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("DIR")
                        .help("Input directory with test results")
                        .default_value("target/performance-reports"),
                ),
        )
        .subcommand(
            Command::new("continuous")
                .about("Run continuous performance monitoring")
                .arg(
                    Arg::new("interval")
                        .short('i')
                        .long("interval")
                        .value_name("MINUTES")
                        .help("Monitoring interval in minutes")
                        .default_value("15"),
                ),
        )
}

async fn load_configuration(matches: &ArgMatches) -> Result<QAConfig> {
    let config_path = matches.get_one::<String>("config").unwrap();
    info!("Loading configuration from: {}", config_path);

    let config = QAConfig::from_file(config_path)?;
    info!("Configuration loaded successfully");

    Ok(config)
}

async fn handle_load_test(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🔥 Starting load test execution");

    let target_url = matches.get_one::<String>("target").unwrap();
    let concurrent_users: u32 = matches.get_one::<String>("users").unwrap().parse()?;
    let duration_secs: u64 = matches.get_one::<String>("duration").unwrap().parse()?;
    let ramp_up_secs: u64 = matches.get_one::<String>("ramp-up").unwrap().parse()?;

    let mut performance_config = config.performance.clone();
    performance_config.load_testing.max_users = concurrent_users;
    performance_config.load_testing.duration_seconds = duration_secs;
    performance_config.load_testing.ramp_up_pattern = RampUpPattern::Linear;
    performance_config.load_testing.think_time_ms = 100;

    let tester = PerformanceTester::new(performance_config).await?;
    let result = tester.run_load_test().await?;

    info!("🎯 Load test completed:");
    info!("  - Duration: {} seconds", result.duration);
    info!("  - Status: {:?}", result.status);
    info!("  - Test ID: {}", result.test_id);
    info!("  - Start time: {}", result.start_time);
    info!("  - End time: {}", result.end_time);
    info!("  - Scenarios: {}", result.scenarios.len());
    info!(
        "  - SLA validation: {:?}",
        result.sla_validation.overall_status
    );

    // Generate report
    let output_dir = PathBuf::from(
        matches
            .get_one::<String>("output")
            .unwrap_or(&"target/performance-reports".to_string()),
    );
    tokio::fs::create_dir_all(&output_dir).await?;

    let report_file = output_dir.join("load_test_report.json");
    let report_json = serde_json::to_string_pretty(&result)?;
    tokio::fs::write(&report_file, report_json).await?;

    info!("📊 Report saved to: {}", report_file.display());

    Ok(())
}

async fn handle_benchmark(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🏁 Starting benchmark execution");

    let suite = matches.get_one::<String>("suite").unwrap();
    let iterations: u32 = matches.get_one::<String>("iterations").unwrap().parse()?;

    let benchmark_config = BenchmarkConfig {
        api_benchmarks: suite == "api" || suite == "full",
        database_benchmarks: suite == "database" || suite == "full",
        integration_benchmarks: suite == "full",
        custom_benchmarks: suite == "custom",
        benchmark_iterations: iterations,
        warmup_iterations: iterations / 10,
        measurement_time_seconds: 60,
        target_throughput: Some(1000.0),
    };

    let performance_config = PerformanceConfig {
        load_testing: LoadTestingConfig::default(),
        benchmarking: benchmark_config,
        sla_thresholds: config.performance.sla_thresholds,
        enable_real_time_metrics: true,
        output_directory: matches
            .get_one::<String>("output")
            .unwrap_or(&"target/performance-reports".to_string())
            .clone(),
    };

    let tester = PerformanceTester::new(performance_config).await?;
    let result = tester.run_benchmarks().await?;

    info!("🏆 Benchmark completed:");
    info!("  - Suite: {}", suite);
    info!("  - Iterations: {}", iterations);
    info!("  - Duration: {} seconds", result.duration.as_secs());

    for benchmark in &result.benchmark_results {
        info!(
            "  📈 {}: {:.2}ms avg, {:.2}ms p95",
            benchmark.name, benchmark.average_time_ms, benchmark.p95_time_ms
        );
    }

    Ok(())
}

async fn handle_stress_test(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("💥 Starting stress test execution");

    let target_url = matches.get_one::<String>("target").unwrap();
    let max_users: u32 = matches.get_one::<String>("max-users").unwrap().parse()?;

    // Stress test with gradually increasing load
    let mut current_users = 10;
    let step_size = max_users / 10;
    let step_duration = Duration::from_secs(30);

    while current_users <= max_users {
        info!("🔥 Stress testing with {} users", current_users);

        let mut performance_config = config.performance.clone();
        performance_config.load_testing.max_users = current_users;
        performance_config.load_testing.duration_seconds = step_duration.as_secs();
        performance_config.load_testing.ramp_up_pattern = RampUpPattern::Linear;
        performance_config.load_testing.think_time_ms = 50;

        let tester = PerformanceTester::new(performance_config).await?;
        let result = tester.run_load_test().await?;

        info!(
            "📊 Results for {} users: {:.2} RPS, {:.2}ms avg",
            current_users, result.requests_per_second, result.average_response_time_ms
        );

        // Check if system is under stress (high error rate or response time)
        if result.failed_requests as f64 / result.total_requests as f64 > 0.05
            || result.average_response_time_ms > 1000.0
        {
            warn!("⚠️  System showing stress at {} users", current_users);
        }

        current_users += step_size;
    }

    info!("✅ Stress test completed");
    Ok(())
}

async fn handle_sla_validation(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("📋 Starting SLA validation");

    let sla_file = matches.get_one::<String>("sla-file").unwrap();
    info!("Loading SLA configuration from: {}", sla_file);

    let tester = PerformanceTester::new(config.performance).await?;
    let validation_result = tester.validate_sla_simple().await?;

    info!("🎯 SLA Validation Results:");
    info!("  - Overall Status: {:?}", validation_result.overall_status);
    info!(
        "  - Response Time SLA: {}",
        if validation_result.response_time_sla_met {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );
    info!(
        "  - Throughput SLA: {}",
        if validation_result.throughput_sla_met {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );
    info!(
        "  - Error Rate SLA: {}",
        if validation_result.error_rate_sla_met {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );
    info!(
        "  - Availability SLA: {}",
        if validation_result.availability_sla_met {
            "✅ PASS"
        } else {
            "❌ FAIL"
        }
    );

    if let Some(violations) = validation_result.violations {
        warn!("⚠️  SLA Violations detected:");
        for violation in violations {
            warn!("  - {}", violation);
        }
    }

    Ok(())
}

async fn handle_report_generation(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("📊 Generating performance reports");

    let format = matches.get_one::<String>("format").unwrap();
    let input_dir = PathBuf::from(matches.get_one::<String>("input").unwrap());
    let output_dir = PathBuf::from(
        matches
            .get_one::<String>("output")
            .unwrap_or(&"target/performance-reports".to_string()),
    );

    tokio::fs::create_dir_all(&output_dir).await?;

    let tester = PerformanceTester::new(config.performance).await?;
    let report = tester.generate_comprehensive_report(&input_dir).await?;

    let output_file = match format.as_str() {
        "html" => {
            let html_report = tester.generate_html_report(&report).await?;
            let file_path = output_dir.join("performance_report.html");
            tokio::fs::write(&file_path, html_report).await?;
            file_path
        }
        "json" => {
            let json_report = serde_json::to_string_pretty(&report)?;
            let file_path = output_dir.join("performance_report.json");
            tokio::fs::write(&file_path, json_report).await?;
            file_path
        }
        "csv" => {
            let csv_report = tester.generate_csv_report(&report).await?;
            let file_path = output_dir.join("performance_report.csv");
            tokio::fs::write(&file_path, csv_report).await?;
            file_path
        }
        "prometheus" => {
            let prometheus_metrics = tester.export_prometheus_metrics(&report).await?;
            let file_path = output_dir.join("performance_metrics.prom");
            tokio::fs::write(&file_path, prometheus_metrics).await?;
            file_path
        }
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
    };

    info!("📄 Report generated: {}", output_file.display());
    Ok(())
}

async fn handle_continuous_testing(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🔄 Starting continuous performance monitoring");

    let interval_minutes: u64 = matches.get_one::<String>("interval").unwrap().parse()?;
    let interval = Duration::from_secs(interval_minutes * 60);

    let tester = PerformanceTester::new(config.performance).await?;

    loop {
        info!("🔍 Running performance monitoring cycle");

        match tester.run_monitoring_cycle().await {
            Ok(result) => {
                info!("✅ Monitoring cycle completed: {:?}", result.overall_status);

                if let Some(alerts) = result.alerts {
                    for alert in alerts {
                        warn!("🚨 Performance Alert: {}", alert);
                    }
                }
            }
            Err(e) => {
                error!("❌ Monitoring cycle failed: {}", e);
            }
        }

        info!("⏰ Waiting {} minutes until next cycle", interval_minutes);
        tokio::time::sleep(interval).await;
    }
}

async fn handle_interactive_mode(config: QAConfig) -> Result<()> {
    info!("🎮 Starting interactive performance testing mode");
    info!("Available commands: load-test, benchmark, stress-test, validate-sla, report, continuous, help, quit");

    let tester = PerformanceTester::new(config.performance).await?;

    loop {
        print!("performance-tester> ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        match input {
            "help" => {
                println!("Available commands:");
                println!("  load-test    - Execute load testing");
                println!("  benchmark    - Run performance benchmarks");
                println!("  stress-test  - Execute stress testing");
                println!("  validate-sla - Validate SLA compliance");
                println!("  report       - Generate reports");
                println!("  continuous   - Start continuous monitoring");
                println!("  status       - Show current status");
                println!("  quit         - Exit interactive mode");
            }
            "status" => {
                let status = tester.get_current_status().await?;
                println!("Current Status: {:?}", status);
            }
            "quit" | "exit" => {
                info!("👋 Exiting interactive mode");
                break;
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    input
                );
            }
        }
    }

    Ok(())
}
