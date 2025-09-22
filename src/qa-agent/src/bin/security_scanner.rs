//! Security Scanner Binary
//!
//! Standalone binary for executing security tests, vulnerability scanning, and compliance validation
//! for the AI-CORE platform services.

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use qa_agent::config::{PenetrationTestConfig, QAConfig, SecurityConfig, VulnerabilityScanConfig};
use qa_agent::security::{SecurityTestResult, SecurityTester};
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
            EnvFilter::from_default_env().add_directive("security_scanner=info".parse()?),
        )
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::rfc_3339())
        .init();

    info!("🔒 AI-CORE Security Scanner v0.1.0");

    let matches = build_cli().get_matches();
    let config = load_configuration(&matches).await?;

    match matches.subcommand() {
        Some(("vuln-scan", sub_matches)) => {
            handle_vulnerability_scan(config, sub_matches).await?;
        }
        Some(("penetration-test", sub_matches)) => {
            handle_penetration_test(config, sub_matches).await?;
        }
        Some(("compliance-check", sub_matches)) => {
            handle_compliance_check(config, sub_matches).await?;
        }
        Some(("dependency-audit", sub_matches)) => {
            handle_dependency_audit(config, sub_matches).await?;
        }
        Some(("container-scan", sub_matches)) => {
            handle_container_scan(config, sub_matches).await?;
        }
        Some(("network-scan", sub_matches)) => {
            handle_network_scan(config, sub_matches).await?;
        }
        Some(("report", sub_matches)) => {
            handle_report_generation(config, sub_matches).await?;
        }
        Some(("continuous", sub_matches)) => {
            handle_continuous_monitoring(config, sub_matches).await?;
        }
        _ => {
            handle_interactive_mode(config).await?;
        }
    }

    Ok(())
}

fn build_cli() -> Command {
    Command::new("security-scanner")
        .version("0.1.0")
        .author("AI-CORE Team")
        .about("Comprehensive security testing for AI-CORE platform")
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
                .default_value("target/security-reports"),
        )
        .subcommand(
            Command::new("vuln-scan")
                .about("Execute vulnerability scanning")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .value_name("URL")
                        .help("Target service URL")
                        .required(true),
                )
                .arg(
                    Arg::new("scan-type")
                        .short('s')
                        .long("scan-type")
                        .value_name("TYPE")
                        .help("Type of vulnerability scan")
                        .value_parser(["web", "api", "network", "all"])
                        .default_value("all"),
                )
                .arg(
                    Arg::new("severity")
                        .long("min-severity")
                        .value_name("LEVEL")
                        .help("Minimum severity level to report")
                        .value_parser(["low", "medium", "high", "critical"])
                        .default_value("medium"),
                ),
        )
        .subcommand(
            Command::new("penetration-test")
                .about("Execute penetration testing")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .value_name("URL")
                        .help("Target service URL")
                        .required(true),
                )
                .arg(
                    Arg::new("test-suite")
                        .short('s')
                        .long("suite")
                        .value_name("SUITE")
                        .help("Penetration test suite")
                        .value_parser([
                            "owasp-top10",
                            "api-security",
                            "auth-bypass",
                            "injection",
                            "custom",
                        ])
                        .default_value("owasp-top10"),
                )
                .arg(
                    Arg::new("aggressive")
                        .long("aggressive")
                        .action(clap::ArgAction::SetTrue)
                        .help("Enable aggressive testing (may impact target)"),
                ),
        )
        .subcommand(
            Command::new("compliance-check")
                .about("Validate security compliance")
                .arg(
                    Arg::new("standard")
                        .short('s')
                        .long("standard")
                        .value_name("STANDARD")
                        .help("Compliance standard to validate")
                        .value_parser(["owasp", "pci-dss", "gdpr", "hipaa", "soc2", "iso27001"])
                        .required(true),
                )
                .arg(
                    Arg::new("scope")
                        .long("scope")
                        .value_name("SCOPE")
                        .help("Scope of compliance check")
                        .value_parser(["infrastructure", "application", "data", "all"])
                        .default_value("all"),
                ),
        )
        .subcommand(
            Command::new("dependency-audit")
                .about("Audit dependencies for vulnerabilities")
                .arg(
                    Arg::new("manifest")
                        .short('m')
                        .long("manifest")
                        .value_name("FILE")
                        .help("Package manifest file (Cargo.toml, package.json, etc.)")
                        .default_value("Cargo.toml"),
                )
                .arg(
                    Arg::new("fix")
                        .long("fix")
                        .action(clap::ArgAction::SetTrue)
                        .help("Attempt to automatically fix vulnerabilities"),
                ),
        )
        .subcommand(
            Command::new("container-scan")
                .about("Scan container images for vulnerabilities")
                .arg(
                    Arg::new("image")
                        .short('i')
                        .long("image")
                        .value_name("IMAGE")
                        .help("Container image to scan")
                        .required(true),
                )
                .arg(
                    Arg::new("registry")
                        .short('r')
                        .long("registry")
                        .value_name("REGISTRY")
                        .help("Container registry URL")
                        .default_value("docker.io"),
                ),
        )
        .subcommand(
            Command::new("network-scan")
                .about("Scan network for security issues")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .value_name("TARGET")
                        .help("Target network or host")
                        .required(true),
                )
                .arg(
                    Arg::new("ports")
                        .short('p')
                        .long("ports")
                        .value_name("PORTS")
                        .help("Port range to scan (e.g., 1-1000, 80,443,8080)")
                        .default_value("1-1000"),
                ),
        )
        .subcommand(
            Command::new("report")
                .about("Generate security reports")
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_name("FORMAT")
                        .help("Report format")
                        .value_parser(["html", "json", "xml", "sarif", "pdf"])
                        .default_value("html"),
                )
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("DIR")
                        .help("Input directory with scan results")
                        .default_value("target/security-reports"),
                ),
        )
        .subcommand(
            Command::new("continuous")
                .about("Run continuous security monitoring")
                .arg(
                    Arg::new("interval")
                        .short('i')
                        .long("interval")
                        .value_name("HOURS")
                        .help("Monitoring interval in hours")
                        .default_value("24"),
                )
                .arg(
                    Arg::new("alerts")
                        .long("enable-alerts")
                        .action(clap::ArgAction::SetTrue)
                        .help("Enable real-time alerts for critical findings"),
                ),
        )
}

async fn load_configuration(matches: &ArgMatches) -> Result<QAConfig> {
    let config_path = matches.get_one::<String>("config").unwrap();
    info!("Loading configuration from: {}", config_path);

    let qa_config = QAConfig::from_file(&config_path)?;
    info!("Configuration loaded successfully");

    Ok(qa_config)
}

async fn handle_vulnerability_scan(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🔍 Starting vulnerability scan");

    let target_url = matches.get_one::<String>("target").unwrap();
    let scan_type = matches.get_one::<String>("scan-type").unwrap();
    let min_severity = matches.get_one::<String>("severity").unwrap();

    let security_config = SecurityConfig {
        vulnerability_scanning: VulnerabilityScanConfig {
            scan_dependencies: true,
            scan_containers: scan_type == "all" || scan_type == "container",
            scan_infrastructure: scan_type == "all" || scan_type == "network",
            scan_web_applications: scan_type == "all" || scan_type == "web",
            scan_apis: scan_type == "all" || scan_type == "api",
            severity_threshold: min_severity.to_string(),
            enable_active_scanning: true,
            custom_rules_path: None,
        },
        penetration_testing: PenetrationTestConfig::default(),
        compliance_validation: config.security.compliance_validation,
        enable_continuous_monitoring: false,
        alert_webhook_url: None,
        report_format: "json".to_string(),
        output_directory: matches
            .get_one::<String>("output")
            .unwrap_or(&"target/security-reports".to_string())
            .clone(),
    };

    let tester = SecurityTester::new(security_config).await?;
    let result = tester.run_vulnerability_scan(target_url).await?;

    info!("🎯 Vulnerability scan completed:");
    info!(
        "  - Total vulnerabilities found: {}",
        result.vulnerabilities_found
    );
    info!("  - Critical: {}", result.critical_count);
    info!("  - High: {}", result.high_count);
    info!("  - Medium: {}", result.medium_count);
    info!("  - Low: {}", result.low_count);
    info!("  - Scan duration: {} seconds", result.duration.as_secs());

    if result.critical_count > 0 {
        error!("🚨 CRITICAL vulnerabilities found! Immediate attention required.");
    } else if result.high_count > 0 {
        warn!("⚠️  HIGH severity vulnerabilities found. Review recommended.");
    } else {
        info!("✅ No critical or high severity vulnerabilities found.");
    }

    // Generate report
    let output_dir = PathBuf::from(
        matches
            .get_one::<String>("output")
            .unwrap_or(&"target/security-reports".to_string()),
    );
    tokio::fs::create_dir_all(&output_dir).await?;

    let report_file = output_dir.join("vulnerability_scan_report.json");
    let report_json = serde_json::to_string_pretty(&result)?;
    tokio::fs::write(&report_file, report_json).await?;

    info!("📊 Report saved to: {}", report_file.display());

    Ok(())
}

async fn handle_penetration_test(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🎯 Starting penetration testing");

    let target_url = matches.get_one::<String>("target").unwrap();
    let test_suite = matches.get_one::<String>("test-suite").unwrap();
    let aggressive = matches.get_flag("aggressive");

    if aggressive {
        warn!("⚠️  Aggressive testing enabled - this may impact the target system");
    }

    let penetration_config = PenetrationTestConfig {
        test_owasp_top10: test_suite == "owasp-top10" || test_suite == "custom",
        test_api_security: test_suite == "api-security" || test_suite == "custom",
        test_authentication_bypass: test_suite == "auth-bypass" || test_suite == "custom",
        test_injection_attacks: test_suite == "injection" || test_suite == "custom",
        test_privilege_escalation: aggressive,
        test_data_exposure: true,
        enable_aggressive_testing: aggressive,
        max_request_rate: if aggressive { 100 } else { 10 },
        test_timeout_seconds: 300,
        custom_payloads_path: None,
    };

    let security_config = SecurityConfig {
        vulnerability_scanning: config.security.vulnerability_scanning,
        penetration_testing: penetration_config,
        compliance_validation: config.security.compliance_validation,
        enable_continuous_monitoring: false,
        alert_webhook_url: None,
        report_format: "json".to_string(),
        output_directory: matches
            .get_one::<String>("output")
            .unwrap_or(&"target/security-reports".to_string())
            .clone(),
    };

    let tester = SecurityTester::new(security_config).await?;
    let result = tester.run_penetration_test(target_url).await?;

    info!("🏁 Penetration testing completed:");
    info!("  - Tests executed: {}", result.tests_executed);
    info!(
        "  - Vulnerabilities found: {}",
        result.vulnerabilities_found
    );
    info!("  - Success rate: {:.2}%", result.success_rate);
    info!("  - Test duration: {} seconds", result.duration.as_secs());

    for finding in &result.security_findings {
        match finding.severity.as_str() {
            "Critical" => error!("🚨 CRITICAL: {}", finding.description),
            "High" => warn!("⚠️  HIGH: {}", finding.description),
            "Medium" => info!("ℹ️  MEDIUM: {}", finding.description),
            "Low" => debug!("💡 LOW: {}", finding.description),
            _ => info!("📝 {}: {}", finding.severity, finding.description),
        }
    }

    Ok(())
}

async fn handle_compliance_check(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("📋 Starting compliance validation");

    let standard = matches.get_one::<String>("standard").unwrap();
    let scope = matches.get_one::<String>("scope").unwrap();

    let tester = SecurityTester::new(config.security).await?;
    let result = tester.run_compliance_check(standard, scope).await?;

    info!(
        "🎯 Compliance check completed for {}:",
        standard.to_uppercase()
    );
    info!(
        "  - Overall compliance: {:.1}%",
        result.compliance_percentage
    );
    info!("  - Controls passed: {}", result.controls_passed);
    info!("  - Controls failed: {}", result.controls_failed);
    info!(
        "  - Controls not applicable: {}",
        result.controls_not_applicable
    );

    if result.compliance_percentage >= 95.0 {
        info!("✅ EXCELLENT compliance score");
    } else if result.compliance_percentage >= 80.0 {
        warn!("⚠️  GOOD compliance score - some improvements needed");
    } else {
        error!("🚨 POOR compliance score - significant improvements required");
    }

    for violation in &result.compliance_violations {
        error!("❌ Compliance Violation: {}", violation);
    }

    for recommendation in &result.recommendations {
        info!("💡 Recommendation: {}", recommendation);
    }

    Ok(())
}

async fn handle_dependency_audit(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("📦 Starting dependency audit");

    let manifest_file = matches.get_one::<String>("manifest").unwrap();
    let auto_fix = matches.get_flag("fix");

    let tester = SecurityTester::new(config.security).await?;
    let result = tester.run_dependency_audit(manifest_file).await?;

    info!("🔍 Dependency audit completed:");
    info!("  - Dependencies scanned: {}", result.dependencies_scanned);
    info!(
        "  - Vulnerable dependencies: {}",
        result.vulnerable_dependencies
    );
    info!(
        "  - Critical vulnerabilities: {}",
        result.critical_vulnerabilities
    );
    info!("  - High vulnerabilities: {}", result.high_vulnerabilities);

    if result.vulnerable_dependencies > 0 {
        warn!(
            "⚠️  {} vulnerable dependencies found",
            result.vulnerable_dependencies
        );

        for vuln in &result.vulnerability_details {
            error!(
                "🚨 {}: {} ({})",
                vuln.package_name, vuln.vulnerability_id, vuln.severity
            );
            info!("   📝 {}", vuln.description);
            if let Some(fix) = &vuln.fixed_version {
                info!("   🔧 Fix available: Update to version {}", fix);
            }
        }

        if auto_fix {
            info!("🔧 Attempting automatic fixes...");
            let fix_result = tester.attempt_dependency_fixes(manifest_file).await?;
            info!(
                "   ✅ {} vulnerabilities fixed automatically",
                fix_result.fixes_applied
            );
            if fix_result.fixes_failed > 0 {
                warn!(
                    "   ⚠️  {} vulnerabilities require manual intervention",
                    fix_result.fixes_failed
                );
            }
        }
    } else {
        info!("✅ No vulnerable dependencies found");
    }

    Ok(())
}

async fn handle_container_scan(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🐳 Starting container security scan");

    let image = matches.get_one::<String>("image").unwrap();
    let registry = matches.get_one::<String>("registry").unwrap();

    let tester = SecurityTester::new(config.security).await?;
    let result = tester.run_container_scan(image, registry).await?;

    info!("📦 Container scan completed for {}:", image);
    info!("  - Image layers scanned: {}", result.layers_scanned);
    info!(
        "  - Vulnerabilities found: {}",
        result.vulnerabilities_found
    );
    info!("  - Critical: {}", result.critical_count);
    info!("  - High: {}", result.high_count);
    info!("  - Medium: {}", result.medium_count);
    info!("  - Low: {}", result.low_count);

    if result.critical_count > 0 || result.high_count > 0 {
        error!(
            "🚨 Container has {} critical and {} high severity vulnerabilities",
            result.critical_count, result.high_count
        );
        error!("   🚫 This container should NOT be deployed to production");
    } else if result.medium_count > 5 {
        warn!(
            "⚠️  Container has {} medium severity vulnerabilities",
            result.medium_count
        );
        warn!("   📋 Review and remediation recommended before production deployment");
    } else {
        info!("✅ Container passes security baseline");
    }

    // Show configuration issues
    for config_issue in &result.configuration_issues {
        warn!("⚙️  Configuration Issue: {}", config_issue);
    }

    // Show secrets detection results
    if result.secrets_found > 0 {
        error!(
            "🔐 {} potential secrets found in container image!",
            result.secrets_found
        );
        for secret in &result.secret_details {
            error!("   🚨 {}: {}", secret.secret_type, secret.location);
        }
    }

    Ok(())
}

async fn handle_network_scan(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🌐 Starting network security scan");

    let target = matches.get_one::<String>("target").unwrap();
    let ports = matches.get_one::<String>("ports").unwrap();

    let tester = SecurityTester::new(config.security).await?;
    let result = tester.run_network_scan(target, ports).await?;

    info!("🔍 Network scan completed for {}:", target);
    info!("  - Ports scanned: {}", result.ports_scanned);
    info!("  - Open ports: {}", result.open_ports.len());
    info!("  - Services identified: {}", result.services_identified);
    info!("  - Security issues: {}", result.security_issues.len());

    info!("📊 Open ports:");
    for port in &result.open_ports {
        info!(
            "  🔓 Port {}/{}: {}",
            port.port,
            port.protocol,
            port.service.as_deref().unwrap_or("Unknown")
        );
        if let Some(version) = &port.version {
            info!("     📝 Version: {}", version);
        }
    }

    if !result.security_issues.is_empty() {
        warn!("⚠️  Security issues found:");
        for issue in &result.security_issues {
            match issue.severity.as_str() {
                "Critical" => error!("  🚨 CRITICAL: {}", issue.description),
                "High" => warn!("  ⚠️  HIGH: {}", issue.description),
                "Medium" => info!("  ℹ️  MEDIUM: {}", issue.description),
                "Low" => debug!("  💡 LOW: {}", issue.description),
                _ => info!("  📝 {}: {}", issue.severity, issue.description),
            }
        }
    } else {
        info!("✅ No security issues found in network scan");
    }

    Ok(())
}

async fn handle_report_generation(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("📊 Generating security reports");

    let format = matches.get_one::<String>("format").unwrap();
    let input_dir = PathBuf::from(matches.get_one::<String>("input").unwrap());
    let output_dir = PathBuf::from(
        matches
            .get_one::<String>("output")
            .unwrap_or(&"target/security-reports".to_string()),
    );

    tokio::fs::create_dir_all(&output_dir).await?;

    let tester = SecurityTester::new(config.security).await?;
    let report = tester.generate_comprehensive_report(&input_dir).await?;

    let output_file = match format.as_str() {
        "html" => {
            let html_report = tester.generate_html_report(&report).await?;
            let file_path = output_dir.join("security_report.html");
            tokio::fs::write(&file_path, html_report).await?;
            file_path
        }
        "json" => {
            let json_report = serde_json::to_string_pretty(&report)?;
            let file_path = output_dir.join("security_report.json");
            tokio::fs::write(&file_path, json_report).await?;
            file_path
        }
        "xml" => {
            let xml_report = tester.generate_xml_report(&report).await?;
            let file_path = output_dir.join("security_report.xml");
            tokio::fs::write(&file_path, xml_report).await?;
            file_path
        }
        "sarif" => {
            let sarif_report = tester.generate_sarif_report(&report).await?;
            let file_path = output_dir.join("security_report.sarif");
            tokio::fs::write(&file_path, sarif_report).await?;
            file_path
        }
        "pdf" => {
            let pdf_report = tester.generate_pdf_report(&report).await?;
            let file_path = output_dir.join("security_report.pdf");
            tokio::fs::write(&file_path, pdf_report).await?;
            file_path
        }
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
    };

    info!("📄 Report generated: {}", output_file.display());
    Ok(())
}

async fn handle_continuous_monitoring(config: QAConfig, matches: &ArgMatches) -> Result<()> {
    info!("🔄 Starting continuous security monitoring");

    let interval_hours: u64 = matches.get_one::<String>("interval").unwrap().parse()?;
    let enable_alerts = matches.get_flag("alerts");
    let interval = Duration::from_secs(interval_hours * 3600);

    let tester = SecurityTester::new(config.security).await?;

    loop {
        info!("🔍 Running security monitoring cycle");

        match tester.run_monitoring_cycle().await {
            Ok(result) => {
                info!(
                    "✅ Security monitoring cycle completed: {:?}",
                    result.overall_status
                );

                if enable_alerts && !result.security_alerts.is_empty() {
                    for alert in &result.security_alerts {
                        error!("🚨 Security Alert: {}", alert);
                    }

                    // Send alerts to webhook if configured
                    if let Some(webhook_url) = &config.security.alert_webhook_url {
                        tester
                            .send_security_alerts(&result.security_alerts, webhook_url)
                            .await?;
                    }
                }
            }
            Err(e) => {
                error!("❌ Security monitoring cycle failed: {}", e);
            }
        }

        info!(
            "⏰ Waiting {} hours until next security scan",
            interval_hours
        );
        tokio::time::sleep(interval).await;
    }
}

async fn handle_interactive_mode(config: QAConfig) -> Result<()> {
    info!("🎮 Starting interactive security testing mode");
    info!("Available commands: vuln-scan, penetration-test, compliance-check, dependency-audit, container-scan, network-scan, report, continuous, help, quit");

    let tester = SecurityTester::new(config.security).await?;

    loop {
        print!("security-scanner> ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        match input {
            "help" => {
                println!("Available commands:");
                println!("  vuln-scan         - Execute vulnerability scanning");
                println!("  penetration-test  - Run penetration testing");
                println!("  compliance-check  - Validate security compliance");
                println!("  dependency-audit  - Audit dependencies for vulnerabilities");
                println!("  container-scan    - Scan container images");
                println!("  network-scan      - Scan network for security issues");
                println!("  report            - Generate security reports");
                println!("  continuous        - Start continuous monitoring");
                println!("  status            - Show current security status");
                println!("  quit              - Exit interactive mode");
            }
            "status" => {
                let status = tester.get_current_security_status().await?;
                println!("Current Security Status: {:?}", status);
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
