//! # QA Orchestrator Binary
//!
//! Main executable for running comprehensive QA workflows in the AI-CORE platform.
//! Provides command-line interface for test execution, performance validation, and quality reporting.

use anyhow::Result;
use chrono::Utc;
use clap::{Arg, ArgAction, Command};
use qa_agent::{PerformanceTester, QAAgent, QAConfig, SecurityTester, TestOrchestrator};
use std::path::PathBuf;
use tokio;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qa_orchestrator=info,qa_agent=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse command line arguments
    let matches = Command::new("qa-orchestrator")
        .version("1.0.0")
        .author("AI-CORE Team")
        .about("AI-CORE Quality Assurance Orchestrator - Comprehensive testing and quality validation")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("qa-config.yaml"),
        )
        .arg(
            Arg::new("suite")
                .short('s')
                .long("suite")
                .value_name("SUITE")
                .help("Specific test suite to run (unit, integration, e2e, performance, security, load, smoke, regression)")
                .required(false),
        )
        .arg(
            Arg::new("parallel")
                .short('p')
                .long("parallel")
                .help("Enable parallel test execution")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .value_name("COUNT")
                .help("Number of parallel workers")
                .default_value("4"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("DIR")
                .help("Output directory for reports")
                .default_value("target/qa-results"),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Report format (html, json, xml, markdown)")
                .default_value("html"),
        )
        .arg(
            Arg::new("coverage")
                .long("coverage")
                .help("Collect test coverage")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("performance")
                .long("performance")
                .help("Run performance tests")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("security")
                .long("security")
                .help("Run security tests")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sla-validation")
                .long("sla-validation")
                .help("Validate SLA compliance")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Perform dry run without executing tests")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dashboard")
                .long("dashboard")
                .help("Start quality dashboard server")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dashboard-port")
                .long("dashboard-port")
                .value_name("PORT")
                .help("Dashboard server port")
                .default_value("8080"),
        )
        .subcommand(
            Command::new("validate")
                .about("Validate test environment and configuration")
                .arg(
                    Arg::new("fix")
                        .long("fix")
                        .help("Attempt to fix validation issues")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List available test suites and configurations"),
        )
        .subcommand(
            Command::new("report")
                .about("Generate reports from previous test runs")
                .arg(
                    Arg::new("execution-id")
                        .long("execution-id")
                        .value_name("ID")
                        .help("Specific execution ID to report on"),
                ),
        )
        .subcommand(
            Command::new("benchmark")
                .about("Run performance benchmarks")
                .arg(
                    Arg::new("scenario")
                        .long("scenario")
                        .value_name("SCENARIO")
                        .help("Specific benchmark scenario to run"),
                ),
        )
        .subcommand(
            Command::new("clean")
                .about("Clean test artifacts and results")
                .arg(
                    Arg::new("all")
                        .long("all")
                        .help("Clean all artifacts including logs")
                        .action(ArgAction::SetTrue),
                ),
        )
        .get_matches();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let mut config = load_configuration(config_path).await?;

    // Override config with command line arguments
    apply_cli_overrides(&mut config, &matches).await?;

    // Handle subcommands
    match matches.subcommand() {
        Some(("validate", sub_matches)) => {
            return handle_validate_command(&config, sub_matches).await;
        }
        Some(("list", _)) => {
            return handle_list_command(&config).await;
        }
        Some(("report", sub_matches)) => {
            return handle_report_command(&config, sub_matches).await;
        }
        Some(("benchmark", sub_matches)) => {
            return handle_benchmark_command(&config, sub_matches).await;
        }
        Some(("clean", sub_matches)) => {
            return handle_clean_command(&config, sub_matches).await;
        }
        _ => {
            // Continue with main execution
        }
    }

    // Start dashboard if requested
    if matches.get_flag("dashboard") {
        let port = matches
            .get_one::<String>("dashboard-port")
            .unwrap()
            .parse::<u16>()?;
        return start_dashboard_server(config, port).await;
    }

    // Perform dry run if requested
    if matches.get_flag("dry-run") {
        return perform_dry_run(&config).await;
    }

    // Initialize QA Agent
    info!("Initializing AI-CORE QA Agent");
    let qa_agent = QAAgent::new(config.clone()).await?;

    // Determine execution mode
    if let Some(specific_suite) = matches.get_one::<String>("suite") {
        // Run specific test suite
        info!("Running specific test suite: {}", specific_suite);
        let result = qa_agent.orchestrator.run_test_suite(specific_suite).await?;

        print_test_summary(&result).await?;
        generate_reports(&qa_agent, &config, &result).await?;

        // Exit with appropriate code
        std::process::exit(if result.status == qa_agent::testing::TestStatus::Passed {
            0
        } else {
            1
        });
    } else {
        // Run comprehensive QA workflow
        info!("Starting comprehensive QA workflow");
        let workflow_result = qa_agent.run_qa_workflow().await?;

        print_workflow_summary(&workflow_result).await?;
        generate_workflow_reports(&qa_agent, &config, &workflow_result).await?;

        // Exit with appropriate code
        std::process::exit(
            if workflow_result.overall_status == qa_agent::QAStatus::Passed {
                0
            } else {
                1
            },
        );
    }
}

/// Load QA configuration from file or environment
async fn load_configuration(config_path: &str) -> Result<QAConfig> {
    let config_file = PathBuf::from(config_path);

    if config_file.exists() {
        info!("Loading configuration from: {}", config_path);
        QAConfig::from_file(config_file)
    } else {
        warn!("Configuration file not found, using defaults with environment overrides");
        QAConfig::from_env()
    }
}

/// Apply command line argument overrides to configuration
async fn apply_cli_overrides(config: &mut QAConfig, matches: &clap::ArgMatches) -> Result<()> {
    // Override parallel execution
    if matches.get_flag("parallel") {
        config.test.parallel_execution = true;
    }

    // Override worker count
    if let Some(workers) = matches.get_one::<String>("workers") {
        config.test.max_workers = workers.parse()?;
    }

    // Override output directory
    if let Some(output_dir) = matches.get_one::<String>("output") {
        config.test.results_dir = PathBuf::from(output_dir);
        config.reporting.output_dir = PathBuf::from(output_dir);
    }

    // Override coverage collection
    if matches.get_flag("coverage") {
        config.test.collect_coverage = true;
    }

    // Override performance testing
    if matches.get_flag("performance") {
        config.performance.enabled = true;
    }

    // Override security testing
    if matches.get_flag("security") {
        config.security.enabled = true;
    }

    // Override dashboard port
    if let Some(port) = matches.get_one::<String>("dashboard-port") {
        config.dashboard.port = port.parse()?;
    }

    // Override verbose logging
    if matches.get_flag("verbose") {
        config.general.debug_mode = true;
    }

    Ok(())
}

/// Handle validate subcommand
async fn handle_validate_command(config: &QAConfig, matches: &clap::ArgMatches) -> Result<()> {
    info!("Validating test environment and configuration");

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        if matches.get_flag("fix") {
            warn!("Fix option not yet implemented");
        }
        return Err(e);
    }

    // Initialize QA components for validation
    let qa_agent = QAAgent::new(config.clone()).await?;

    // Validate test environment
    let validation_result = qa_agent.orchestrator.validate_test_environment().await?;

    println!("Environment Validation Results:");
    println!("==============================");
    println!("Overall Status: {:?}", validation_result.overall_status);
    println!();

    for validation in &validation_result.validations {
        let status_symbol = match validation.status {
            qa_agent::testing::TestStatus::Passed => "✅",
            qa_agent::testing::TestStatus::Failed => "❌",
            qa_agent::testing::TestStatus::Skipped => "⏭️ ",
            _ => "⚠️ ",
        };

        println!("{} {}", status_symbol, validation.name);
        if let Some(message) = &validation.message {
            println!("   {}", message);
        }
    }

    if validation_result.overall_status == qa_agent::testing::TestStatus::Passed {
        info!("Environment validation passed");
        Ok(())
    } else {
        error!("Environment validation failed");
        std::process::exit(1);
    }
}

/// Handle list subcommand
async fn handle_list_command(config: &QAConfig) -> Result<()> {
    println!("AI-CORE QA Configuration Summary");
    println!("================================");
    println!();

    println!("Test Suites:");
    for suite in &config.test.suites {
        let status = if suite.enabled { "enabled" } else { "disabled" };
        println!(
            "  - {} ({:?}) - {} (priority: {})",
            suite.name, suite.suite_type, status, suite.priority
        );
    }
    println!();

    println!(
        "Performance Testing: {}",
        if config.performance.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "Security Testing: {}",
        if config.security.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "Quality Dashboard: {}",
        if config.dashboard.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "Metrics Collection: {}",
        if config.metrics.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    println!("SLA Thresholds:");
    println!(
        "  - API P95: ≤ {}ms",
        config.performance.sla_thresholds.api_p95_ms
    );
    println!(
        "  - API P99: ≤ {}ms",
        config.performance.sla_thresholds.api_p99_ms
    );
    println!(
        "  - DB P95: ≤ {}ms",
        config.performance.sla_thresholds.db_p95_ms
    );
    println!(
        "  - Error Rate: ≤ {}%",
        config.performance.sla_thresholds.error_rate_percent
    );
    println!(
        "  - Min Throughput: ≥ {} req/s",
        config.performance.sla_thresholds.min_throughput_rps
    );

    Ok(())
}

/// Handle report subcommand
async fn handle_report_command(config: &QAConfig, _matches: &clap::ArgMatches) -> Result<()> {
    info!("Generating reports from previous test runs");

    // This would typically read from stored test results
    // For now, we'll indicate the feature
    println!("Report generation from stored results is not yet implemented.");
    println!("Use the main QA workflow to generate fresh reports.");

    Ok(())
}

/// Handle benchmark subcommand
async fn handle_benchmark_command(config: &QAConfig, _matches: &clap::ArgMatches) -> Result<()> {
    info!("Running performance benchmarks");

    let performance_tester = PerformanceTester::new(config.performance.clone()).await?;
    let result = performance_tester.run_performance_suite().await?;

    println!("Benchmark Results:");
    println!("==================");

    for scenario in &result.scenarios {
        println!("Scenario: {}", scenario.name);
        println!("Status: {:?}", scenario.status);
        println!("Duration: {}s", scenario.duration);
        println!("Metrics:");
        println!(
            "  - Avg Response Time: {}ms",
            scenario.metrics.average_response_time_ms
        );
        println!(
            "  - P95 Response Time: {}ms",
            scenario.metrics.p95_response_time_ms
        );
        println!(
            "  - Throughput: {:.2} req/s",
            scenario.metrics.requests_per_second
        );
        println!(
            "  - Error Rate: {:.2}%",
            scenario.metrics.error_rate_percent
        );
        println!();
    }

    println!(
        "SLA Compliance: {:.1}%",
        result.sla_validation.compliance_percentage
    );
    if !result.sla_validation.violations.is_empty() {
        println!("SLA Violations:");
        for violation in &result.sla_validation.violations {
            println!(
                "  - {}: expected {}, got {} ({})",
                violation.metric, violation.expected, violation.actual, violation.test_case
            );
        }
    }

    Ok(())
}

/// Handle clean subcommand
async fn handle_clean_command(config: &QAConfig, matches: &clap::ArgMatches) -> Result<()> {
    info!("Cleaning test artifacts and results");

    let clean_all = matches.get_flag("all");

    // Clean results directory
    if config.test.results_dir.exists() {
        std::fs::remove_dir_all(&config.test.results_dir)?;
        println!(
            "Cleaned results directory: {}",
            config.test.results_dir.display()
        );
    }

    // Clean reports directory
    if config.reporting.output_dir.exists() {
        std::fs::remove_dir_all(&config.reporting.output_dir)?;
        println!(
            "Cleaned reports directory: {}",
            config.reporting.output_dir.display()
        );
    }

    if clean_all {
        // Clean additional artifacts
        let additional_dirs = ["target/coverage", "target/criterion", "target/qa-temp"];
        for dir in &additional_dirs {
            let path = PathBuf::from(dir);
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
                println!("Cleaned directory: {}", path.display());
            }
        }
    }

    println!("Cleanup completed successfully");
    Ok(())
}

/// Start dashboard server
async fn start_dashboard_server(config: QAConfig, port: u16) -> Result<()> {
    info!("Starting quality dashboard server on port {}", port);

    // Initialize QA Agent
    let qa_agent = QAAgent::new(config).await?;

    // Start dashboard server
    qa_agent.dashboard.start_server(port).await?;

    Ok(())
}

/// Perform dry run
async fn perform_dry_run(config: &QAConfig) -> Result<()> {
    info!("Performing dry run - validating configuration and environment");

    println!("Dry Run Results:");
    println!("================");

    // Validate configuration
    match config.validate() {
        Ok(_) => println!("✅ Configuration validation passed"),
        Err(e) => {
            println!("❌ Configuration validation failed: {}", e);
            return Err(e);
        }
    }

    // Check if QA Agent can be initialized
    match QAAgent::new(config.clone()).await {
        Ok(_) => println!("✅ QA Agent initialization successful"),
        Err(e) => {
            println!("❌ QA Agent initialization failed: {}", e);
            return Err(e);
        }
    }

    // List what would be executed
    println!("\nTest Suites that would be executed:");
    for suite in &config.test.suites {
        if suite.enabled {
            println!(
                "  - {} ({:?}) - priority: {}",
                suite.name, suite.suite_type, suite.priority
            );
        }
    }

    if config.performance.enabled {
        println!("  - Performance Testing Suite");
    }

    if config.security.enabled {
        println!("  - Security Testing Suite");
    }

    println!("\nDry run completed successfully - ready for actual execution");
    Ok(())
}

/// Print test suite summary
async fn print_test_summary(result: &qa_agent::orchestrator::TestSuiteResult) -> Result<()> {
    println!();
    println!("Test Suite Results:");
    println!("===================");
    println!("Suite: {}", result.suite_name);
    println!("Status: {:?}", result.status);
    println!("Duration: {}s", result.duration);
    println!("Total Tests: {}", result.total_tests);
    println!("Passed: {}", result.passed_tests);
    println!("Failed: {}", result.failed_tests);
    println!("Skipped: {}", result.skipped_tests);

    if let Some(coverage) = result.coverage_percentage {
        println!("Coverage: {:.1}%", coverage);
    }

    if result.failed_tests > 0 {
        println!("\nFailed Test Cases:");
        for test_case in &result.test_cases {
            if test_case.status == qa_agent::testing::TestStatus::Failed {
                println!("  - {}", test_case.name);
                if let Some(error) = &test_case.error_message {
                    println!("    Error: {}", error);
                }
            }
        }
    }

    println!();
    Ok(())
}

/// Print workflow summary
async fn print_workflow_summary(result: &qa_agent::QAWorkflowResult) -> Result<()> {
    println!();
    println!("QA Workflow Results:");
    println!("====================");
    println!("Workflow ID: {}", result.workflow_id);
    println!("Overall Status: {}", result.overall_status);
    println!("Duration: {}s", result.duration);
    println!();

    println!("Test Results:");
    println!("  Status: {:?}", result.test_result.status);
    println!("  Total Tests: {}", result.test_result.total_tests);
    println!("  Passed: {}", result.test_result.passed_tests);
    println!("  Failed: {}", result.test_result.failed_tests);

    println!("\nPerformance Results:");
    println!("  Status: {:?}", result.performance_result.status);
    println!(
        "  SLA Compliance: {:.1}%",
        result
            .performance_result
            .sla_validation
            .compliance_percentage
    );
    println!("  Scenarios: {}", result.performance_result.scenarios.len());

    println!("\nSecurity Results:");
    println!("  Status: {:?}", result.security_result.status);
    println!("  Scans: {}", result.security_result.scans.len());

    if !result
        .performance_result
        .sla_validation
        .violations
        .is_empty()
    {
        println!("\nSLA Violations:");
        for violation in &result.performance_result.sla_validation.violations {
            println!(
                "  - {}: expected {}, got {}",
                violation.metric, violation.expected, violation.actual
            );
        }
    }

    if !result.performance_result.recommendations.is_empty() {
        println!("\nRecommendations:");
        for rec in &result.performance_result.recommendations {
            println!("  - {}: {}", rec.title, rec.description);
        }
    }

    println!();
    Ok(())
}

/// Generate reports for test suite
async fn generate_reports(
    qa_agent: &QAAgent,
    config: &QAConfig,
    result: &qa_agent::orchestrator::TestSuiteResult,
) -> Result<()> {
    if !config.reporting.enabled {
        return Ok(());
    }

    info!("Generating test reports");

    // Ensure output directory exists
    std::fs::create_dir_all(&config.reporting.output_dir)?;

    // Generate HTML report
    let html_report_path = config.reporting.output_dir.join("test-report.html");
    let html_content = generate_html_test_report(result).await?;
    std::fs::write(&html_report_path, html_content)?;
    info!("HTML report generated: {}", html_report_path.display());

    // Generate JSON report
    let json_report_path = config.reporting.output_dir.join("test-report.json");
    let json_content = serde_json::to_string_pretty(result)?;
    std::fs::write(&json_report_path, json_content)?;
    info!("JSON report generated: {}", json_report_path.display());

    Ok(())
}

/// Generate reports for QA workflow
async fn generate_workflow_reports(
    qa_agent: &QAAgent,
    config: &QAConfig,
    result: &qa_agent::QAWorkflowResult,
) -> Result<()> {
    if !config.reporting.enabled {
        return Ok(());
    }

    info!("Generating workflow reports");

    // Ensure output directory exists
    std::fs::create_dir_all(&config.reporting.output_dir)?;

    // Generate comprehensive HTML report
    let html_report_path = config.reporting.output_dir.join("qa-workflow-report.html");
    let html_content = generate_html_workflow_report(result).await?;
    std::fs::write(&html_report_path, html_content)?;
    info!(
        "HTML workflow report generated: {}",
        html_report_path.display()
    );

    // Generate JSON report
    let json_report_path = config.reporting.output_dir.join("qa-workflow-report.json");
    let json_content = serde_json::to_string_pretty(result)?;
    std::fs::write(&json_report_path, json_content)?;
    info!(
        "JSON workflow report generated: {}",
        json_report_path.display()
    );

    Ok(())
}

/// Generate HTML test report
async fn generate_html_test_report(
    result: &qa_agent::orchestrator::TestSuiteResult,
) -> Result<String> {
    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>AI-CORE QA Test Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .status-passed {{ color: green; font-weight: bold; }}
        .status-failed {{ color: red; font-weight: bold; }}
        .metrics {{ display: flex; gap: 20px; margin: 20px 0; }}
        .metric {{ background-color: #f9f9f9; padding: 10px; border-radius: 5px; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>AI-CORE QA Test Report</h1>
        <p><strong>Suite:</strong> {}</p>
        <p><strong>Status:</strong> <span class="status-{}">{:?}</span></p>
        <p><strong>Generated:</strong> {}</p>
    </div>

    <div class="metrics">
        <div class="metric">
            <h3>Total Tests</h3>
            <p>{}</p>
        </div>
        <div class="metric">
            <h3>Passed</h3>
            <p>{}</p>
        </div>
        <div class="metric">
            <h3>Failed</h3>
            <p>{}</p>
        </div>
        <div class="metric">
            <h3>Duration</h3>
            <p>{}s</p>
        </div>
    </div>

    <h2>Test Cases</h2>
    <table>
        <thead>
            <tr>
                <th>Test Case</th>
                <th>Status</th>
                <th>Duration (ms)</th>
                <th>Assertions</th>
            </tr>
        </thead>
        <tbody>
            {}
        </tbody>
    </table>
</body>
</html>
"#,
        result.suite_name,
        if result.status == qa_agent::testing::TestStatus::Passed {
            "passed"
        } else {
            "failed"
        },
        result.status,
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        result.total_tests,
        result.passed_tests,
        result.failed_tests,
        result.duration,
        result
            .test_cases
            .iter()
            .map(|tc| format!(
                "<tr><td>{}</td><td class=\"status-{}\">{:?}</td><td>{}</td><td>{}</td></tr>",
                tc.name,
                if tc.status == qa_agent::testing::TestStatus::Passed {
                    "passed"
                } else {
                    "failed"
                },
                tc.status,
                tc.duration,
                tc.assertions
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(html)
}

/// Generate HTML workflow report
async fn generate_html_workflow_report(result: &qa_agent::QAWorkflowResult) -> Result<String> {
    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>AI-CORE QA Workflow Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .status-passed {{ color: green; font-weight: bold; }}
        .status-failed {{ color: red; font-weight: bold; }}
        .section {{ margin: 30px 0; padding: 20px; border: 1px solid #ddd; border-radius: 5px; }}
        .metrics {{ display: flex; gap: 20px; margin: 20px 0; }}
        .metric {{ background-color: #f9f9f9; padding: 10px; border-radius: 5px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>AI-CORE QA Workflow Report</h1>
        <p><strong>Workflow ID:</strong> {}</p>
        <p><strong>Overall Status:</strong> <span class="status-{}">{}</span></p>
        <p><strong>Duration:</strong> {}s</p>
        <p><strong>Generated:</strong> {}</p>
    </div>

    <div class="section">
        <h2>Test Results Summary</h2>
        <div class="metrics">
            <div class="metric">
                <h3>Total Tests</h3>
                <p>{}</p>
            </div>
            <div class="metric">
                <h3>Passed</h3>
                <p>{}</p>
            </div>
            <div class="metric">
                <h3>Failed</h3>
                <p>{}</p>
            </div>
        </div>
    </div>

    <div class="section">
        <h2>Performance Results</h2>
        <p><strong>Status:</strong> <span class="status-{}">{:?}</span></p>
        <p><strong>SLA Compliance:</strong> {:.1}%</p>
        <p><strong>Scenarios Executed:</strong> {}</p>
    </div>

    <div class="section">
        <h2>Security Results</h2>
        <p><strong>Status:</strong> <span class="status-{}">{:?}</span></p>
        <p><strong>Scans Completed:</strong> {}</p>
    </div>
</body>
</html>
"#,
        result.workflow_id,
        if result.overall_status == qa_agent::QAStatus::Passed {
            "passed"
        } else {
            "failed"
        },
        result.overall_status,
        result.duration,
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        result.test_result.total_tests,
        result.test_result.passed_tests,
        result.test_result.failed_tests,
        if result.performance_result.status == qa_agent::performance::PerformanceStatus::Passed {
            "passed"
        } else {
            "failed"
        },
        result.performance_result.status,
        result
            .performance_result
            .sla_validation
            .compliance_percentage,
        result.performance_result.scenarios.len(),
        if result.security_result.status == qa_agent::security::SecurityStatus::Passed {
            "passed"
        } else {
            "failed"
        },
        result.security_result.status,
        result.security_result.scans.len()
    );

    Ok(html)
}
