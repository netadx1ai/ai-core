//! # QA Agent Configuration Module
//!
//! Comprehensive configuration management for the AI-CORE Quality Assurance Agent.
//! Supports environment variables, YAML files, and programmatic configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

/// Main QA Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAConfig {
    /// Test orchestration configuration
    pub test: TestConfig,
    /// Performance testing configuration
    pub performance: PerformanceConfig,
    /// Security testing configuration
    pub security: SecurityConfig,
    /// Quality metrics configuration
    pub metrics: MetricsConfig,
    /// Dashboard configuration
    pub dashboard: DashboardConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Monitoring systems configuration
    pub monitoring: MonitoringConfig,
    /// Reporting configuration
    pub reporting: ReportingConfig,
    /// General QA settings
    pub general: GeneralConfig,
}

/// Test orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Enable parallel test execution
    pub parallel_execution: bool,
    /// Maximum number of concurrent test workers
    pub max_workers: usize,
    /// Test timeout in seconds
    pub timeout_seconds: u64,
    /// Test retry attempts for flaky tests
    pub retry_attempts: u32,
    /// Test environment configuration
    pub environment: TestEnvironmentConfig,
    /// Test suites to execute
    pub suites: Vec<TestSuiteConfig>,
    /// Test data and fixtures directory
    pub fixtures_dir: PathBuf,
    /// Test results output directory
    pub results_dir: PathBuf,
    /// Enable test coverage collection
    pub collect_coverage: bool,
    /// Minimum coverage threshold (percentage)
    pub min_coverage_threshold: f64,
}

/// Test environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEnvironmentConfig {
    /// Use Docker containers for testing
    pub use_docker: bool,
    /// Docker network for test containers
    pub docker_network: String,
    /// Environment variables for tests
    pub environment_variables: std::collections::HashMap<String, String>,
    /// Test database setup
    pub test_databases: TestDatabaseConfig,
}

/// Test database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDatabaseConfig {
    /// Use separate test databases
    pub use_separate_databases: bool,
    /// Test database prefix
    pub database_prefix: String,
    /// Auto-cleanup test data
    pub auto_cleanup: bool,
    /// Seed test data
    pub seed_test_data: bool,
}

/// Test suite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteConfig {
    /// Test suite name
    pub name: String,
    /// Test suite type
    pub suite_type: TestSuiteType,
    /// Enable this test suite
    pub enabled: bool,
    /// Test suite priority (higher runs first)
    pub priority: u32,
    /// Test patterns to include
    pub include_patterns: Vec<String>,
    /// Test patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Test suite specific configuration
    pub config: serde_json::Value,
}

/// Test suite types
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TestSuiteType {
    Unit,
    Integration,
    EndToEnd,
    Performance,
    Security,
    Load,
    Smoke,
    Regression,
}

/// Performance testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance testing
    pub enabled: bool,
    /// Performance test scenarios
    pub scenarios: Vec<PerformanceScenarioConfig>,
    /// SLA validation thresholds
    pub sla_thresholds: SLAThresholds,
    /// Load testing configuration
    pub load_testing: LoadTestingConfig,
    /// Benchmark configuration
    pub benchmarking: BenchmarkConfig,
    /// Performance monitoring during tests
    pub monitoring: PerformanceMonitoringConfig,
}

/// Performance test scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceScenarioConfig {
    /// Scenario name
    pub name: String,
    /// Target endpoint or component
    pub target: String,
    /// Test duration in seconds
    pub duration_seconds: u64,
    /// Number of virtual users
    pub virtual_users: u32,
    /// Requests per second target
    pub target_rps: u32,
    /// Ramp-up duration in seconds
    pub ramp_up_seconds: u64,
    /// Test data configuration
    pub test_data: TestDataConfig,
}

/// SLA thresholds for performance validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAThresholds {
    /// API response time P95 threshold (milliseconds)
    pub api_p95_ms: u64,
    /// API response time P99 threshold (milliseconds)
    pub api_p99_ms: u64,
    /// Database query P95 threshold (milliseconds)
    pub db_p95_ms: u64,
    /// Error rate threshold (percentage)
    pub error_rate_percent: f64,
    /// Throughput threshold (requests per second)
    pub min_throughput_rps: u32,
    /// Memory usage threshold (MB)
    pub max_memory_mb: u64,
    /// CPU usage threshold (percentage)
    pub max_cpu_percent: f64,
}

/// Load testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestingConfig {
    /// Maximum concurrent users to test
    pub max_users: u32,
    /// Load test duration in seconds
    pub duration_seconds: u64,
    /// User ramp-up pattern
    pub ramp_up_pattern: RampUpPattern,
    /// Think time between requests (milliseconds)
    pub think_time_ms: u64,
}

/// Load test ramp-up patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RampUpPattern {
    Linear,
    Exponential,
    Step,
    Spike,
    Constant,
}

/// Benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Enable micro-benchmarks
    pub enable_micro_benchmarks: bool,
    /// Number of benchmark iterations
    pub iterations: u32,
    /// Warmup iterations
    pub warmup_iterations: u32,
    /// Benchmark output format
    pub output_format: BenchmarkOutputFormat,
}

/// Benchmark output formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkOutputFormat {
    Json,
    Html,
    Csv,
    Pretty,
}

/// Performance monitoring during tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMonitoringConfig {
    /// Monitor system resources
    pub monitor_system_resources: bool,
    /// Monitor application metrics
    pub monitor_app_metrics: bool,
    /// Monitoring interval in seconds
    pub monitoring_interval_seconds: u64,
    /// Metrics collection endpoints
    pub metrics_endpoints: Vec<String>,
}

/// Test data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDataConfig {
    /// Test data generation strategy
    pub generation_strategy: TestDataStrategy,
    /// Number of records to generate
    pub record_count: u32,
    /// Test data file path
    pub data_file: Option<PathBuf>,
}

/// Test data generation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestDataStrategy {
    Generated,
    FromFile,
    Database,
    Hybrid,
}

/// Security testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security testing
    pub enabled: bool,
    /// Security scan types to run
    pub scan_types: Vec<SecurityScanType>,
    /// Vulnerability scanning configuration
    pub vulnerability_scanning: VulnerabilityScanConfig,
    /// Penetration testing configuration
    pub penetration_testing: PenetrationTestConfig,
    /// Security compliance checks
    pub compliance_checks: ComplianceCheckConfig,
    /// Security test reporting
    pub reporting: SecurityReportingConfig,
}

/// Security scan types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityScanType {
    StaticAnalysis,
    DependencyCheck,
    ContainerScan,
    InfrastructureScan,
    WebApplicationScan,
    ApiSecurityScan,
    NetworkScan,
}

/// Vulnerability scanning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityScanConfig {
    /// Enable dependency vulnerability scanning
    pub scan_dependencies: bool,
    /// Enable container image scanning
    pub scan_containers: bool,
    /// Enable infrastructure scanning
    pub scan_infrastructure: bool,
    /// Vulnerability database update frequency
    pub update_frequency_hours: u32,
    /// Severity threshold for failing tests
    pub fail_on_severity: VulnerabilitySeverity,
}

/// Vulnerability severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Penetration testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenetrationTestConfig {
    /// Enable automated penetration testing
    pub enabled: bool,
    /// Target endpoints for testing
    pub target_endpoints: Vec<String>,
    /// Authentication credentials for testing
    pub test_credentials: Option<TestCredentials>,
    /// Attack scenarios to test
    pub attack_scenarios: Vec<AttackScenario>,
}

/// Test credentials for penetration testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCredentials {
    /// Test username
    pub username: String,
    /// Test password
    pub password: String,
    /// Test API keys
    pub api_keys: std::collections::HashMap<String, String>,
}

/// Attack scenarios for penetration testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttackScenario {
    SqlInjection,
    CrossSiteScripting,
    CrossSiteRequestForgery,
    AuthenticationBypass,
    AuthorizationEscalation,
    SessionFixation,
    InsecureDirectObjectReference,
    SecurityMisconfiguration,
}

/// Security compliance check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckConfig {
    /// Enable OWASP compliance checks
    pub owasp_checks: bool,
    /// Enable GDPR compliance checks
    pub gdpr_checks: bool,
    /// Enable SOC2 compliance checks
    pub soc2_checks: bool,
    /// Custom compliance rules
    pub custom_rules: Vec<ComplianceRule>,
}

/// Custom compliance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule validation logic
    pub validation_script: String,
    /// Rule severity
    pub severity: VulnerabilitySeverity,
}

/// Security reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReportingConfig {
    /// Generate detailed security reports
    pub detailed_reports: bool,
    /// Include remediation suggestions
    pub include_remediation: bool,
    /// Export formats
    pub export_formats: Vec<SecurityReportFormat>,
}

/// Security report formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityReportFormat {
    Json,
    Html,
    Pdf,
    Sarif,
    Xml,
}

/// Quality metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics collection interval in seconds
    pub collection_interval_seconds: u64,
    /// Metrics retention period in days
    pub retention_days: u32,
    /// Quality score calculation configuration
    pub quality_score: QualityScoreConfig,
    /// Metrics storage configuration
    pub storage: MetricsStorageConfig,
}

/// Quality score calculation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScoreConfig {
    /// Test coverage weight in quality score
    pub test_coverage_weight: f64,
    /// Performance score weight
    pub performance_weight: f64,
    /// Security score weight
    pub security_weight: f64,
    /// Code quality weight
    pub code_quality_weight: f64,
    /// Documentation weight
    pub documentation_weight: f64,
}

/// Metrics storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStorageConfig {
    /// Metrics storage backend
    pub backend: MetricsBackend,
    /// Storage connection URL
    pub connection_url: String,
    /// Batch size for metrics ingestion
    pub batch_size: u32,
}

/// Metrics storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricsBackend {
    Prometheus,
    InfluxDB,
    TimescaleDB,
    ClickHouse,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Enable quality dashboard
    pub enabled: bool,
    /// Dashboard server port
    pub port: u16,
    /// Dashboard host address
    pub host: String,
    /// Dashboard refresh interval in seconds
    pub refresh_interval_seconds: u64,
    /// Dashboard authentication
    pub authentication: DashboardAuthConfig,
    /// Dashboard features
    pub features: DashboardFeatures,
}

/// Dashboard authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAuthConfig {
    /// Enable authentication
    pub enabled: bool,
    /// Authentication provider
    pub provider: AuthProvider,
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
}

/// Authentication providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthProvider {
    Local,
    OAuth2,
    LDAP,
    SAML,
}

/// Dashboard features configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardFeatures {
    /// Enable real-time updates
    pub real_time_updates: bool,
    /// Enable test execution from dashboard
    pub test_execution: bool,
    /// Enable report generation
    pub report_generation: bool,
    /// Enable alert management
    pub alert_management: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub postgres_url: String,
    /// Redis connection URL
    pub redis_url: String,
    /// MongoDB connection URL
    pub mongodb_url: String,
    /// ClickHouse connection URL
    pub clickhouse_url: String,
    /// Database connection pool configuration
    pub pool: DatabasePoolConfig,
}

/// Database connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabasePoolConfig {
    /// Maximum number of connections
    pub max_connections: u32,
    /// Minimum number of connections
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Idle timeout in seconds
    pub idle_timeout_seconds: u64,
}

/// Monitoring systems configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Prometheus server URL
    pub prometheus_url: String,
    /// Grafana server URL
    pub grafana_url: String,
    /// Jaeger server URL
    pub jaeger_url: String,
    /// Metrics endpoint URL
    pub metrics_endpoint: String,
    /// Health check endpoints
    pub health_endpoints: Vec<String>,
}

/// Reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    /// Enable report generation
    pub enabled: bool,
    /// Report output directory
    pub output_dir: PathBuf,
    /// Report formats to generate
    pub formats: Vec<ReportFormat>,
    /// Report templates directory
    pub templates_dir: PathBuf,
    /// Email notification configuration
    pub email_notifications: EmailNotificationConfig,
}

/// Report formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Html,
    Pdf,
    Json,
    Xml,
    Markdown,
}

/// Email notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotificationConfig {
    /// Enable email notifications
    pub enabled: bool,
    /// SMTP server configuration
    pub smtp_server: String,
    /// SMTP port
    pub smtp_port: u16,
    /// SMTP username
    pub smtp_username: String,
    /// SMTP password
    pub smtp_password: String,
    /// Notification recipients
    pub recipients: Vec<String>,
}

/// General QA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// QA agent name
    pub agent_name: String,
    /// QA agent version
    pub agent_version: String,
    /// Log level
    pub log_level: LogLevel,
    /// Working directory
    pub working_dir: PathBuf,
    /// Temporary directory
    pub temp_dir: PathBuf,
    /// Enable debug mode
    pub debug_mode: bool,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for QAConfig {
    fn default() -> Self {
        Self {
            test: TestConfig::default(),
            performance: PerformanceConfig::default(),
            security: SecurityConfig::default(),
            metrics: MetricsConfig::default(),
            dashboard: DashboardConfig::default(),
            database: DatabaseConfig::default(),
            monitoring: MonitoringConfig::default(),
            reporting: ReportingConfig::default(),
            general: GeneralConfig::default(),
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            max_workers: num_cpus::get(),
            timeout_seconds: 300,
            retry_attempts: 3,
            environment: TestEnvironmentConfig::default(),
            suites: vec![
                TestSuiteConfig {
                    name: "unit".to_string(),
                    suite_type: TestSuiteType::Unit,
                    enabled: true,
                    priority: 100,
                    include_patterns: vec!["**/*test*.rs".to_string()],
                    exclude_patterns: vec![],
                    config: serde_json::json!({}),
                },
                TestSuiteConfig {
                    name: "integration".to_string(),
                    suite_type: TestSuiteType::Integration,
                    enabled: true,
                    priority: 90,
                    include_patterns: vec!["**/integration/**/*.rs".to_string()],
                    exclude_patterns: vec![],
                    config: serde_json::json!({}),
                },
            ],
            fixtures_dir: PathBuf::from("tests/fixtures"),
            results_dir: PathBuf::from("target/qa-results"),
            collect_coverage: true,
            min_coverage_threshold: 80.0,
        }
    }
}

impl Default for TestEnvironmentConfig {
    fn default() -> Self {
        Self {
            use_docker: true,
            docker_network: "ai-core-test".to_string(),
            environment_variables: std::collections::HashMap::new(),
            test_databases: TestDatabaseConfig::default(),
        }
    }
}

impl Default for TestDatabaseConfig {
    fn default() -> Self {
        Self {
            use_separate_databases: true,
            database_prefix: "test_".to_string(),
            auto_cleanup: true,
            seed_test_data: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scenarios: vec![],
            sla_thresholds: SLAThresholds::default(),
            load_testing: LoadTestingConfig::default(),
            benchmarking: BenchmarkConfig::default(),
            monitoring: PerformanceMonitoringConfig::default(),
        }
    }
}

impl Default for SLAThresholds {
    fn default() -> Self {
        Self {
            api_p95_ms: 50,
            api_p99_ms: 100,
            db_p95_ms: 10,
            error_rate_percent: 1.0,
            min_throughput_rps: 1000,
            max_memory_mb: 512,
            max_cpu_percent: 80.0,
        }
    }
}

impl Default for LoadTestingConfig {
    fn default() -> Self {
        Self {
            max_users: 1000,
            duration_seconds: 300,
            ramp_up_pattern: RampUpPattern::Linear,
            think_time_ms: 1000,
        }
    }
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            enable_micro_benchmarks: true,
            iterations: 100,
            warmup_iterations: 10,
            output_format: BenchmarkOutputFormat::Html,
        }
    }
}

impl Default for PerformanceMonitoringConfig {
    fn default() -> Self {
        Self {
            monitor_system_resources: true,
            monitor_app_metrics: true,
            monitoring_interval_seconds: 5,
            metrics_endpoints: vec!["http://localhost:9090/metrics".to_string()],
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scan_types: vec![
                SecurityScanType::StaticAnalysis,
                SecurityScanType::DependencyCheck,
                SecurityScanType::WebApplicationScan,
            ],
            vulnerability_scanning: VulnerabilityScanConfig::default(),
            penetration_testing: PenetrationTestConfig::default(),
            compliance_checks: ComplianceCheckConfig::default(),
            reporting: SecurityReportingConfig::default(),
        }
    }
}

impl Default for VulnerabilityScanConfig {
    fn default() -> Self {
        Self {
            scan_dependencies: true,
            scan_containers: true,
            scan_infrastructure: true,
            update_frequency_hours: 24,
            fail_on_severity: VulnerabilitySeverity::High,
        }
    }
}

impl Default for PenetrationTestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_endpoints: vec![],
            test_credentials: None,
            attack_scenarios: vec![
                AttackScenario::SqlInjection,
                AttackScenario::CrossSiteScripting,
                AttackScenario::AuthenticationBypass,
            ],
        }
    }
}

impl Default for ComplianceCheckConfig {
    fn default() -> Self {
        Self {
            owasp_checks: true,
            gdpr_checks: true,
            soc2_checks: false,
            custom_rules: vec![],
        }
    }
}

impl Default for SecurityReportingConfig {
    fn default() -> Self {
        Self {
            detailed_reports: true,
            include_remediation: true,
            export_formats: vec![SecurityReportFormat::Html, SecurityReportFormat::Json],
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_seconds: 60,
            retention_days: 30,
            quality_score: QualityScoreConfig::default(),
            storage: MetricsStorageConfig::default(),
        }
    }
}

impl Default for QualityScoreConfig {
    fn default() -> Self {
        Self {
            test_coverage_weight: 0.25,
            performance_weight: 0.25,
            security_weight: 0.25,
            code_quality_weight: 0.15,
            documentation_weight: 0.10,
        }
    }
}

impl Default for MetricsStorageConfig {
    fn default() -> Self {
        Self {
            backend: MetricsBackend::Prometheus,
            connection_url: "http://localhost:9090".to_string(),
            batch_size: 1000,
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8080,
            host: "0.0.0.0".to_string(),
            refresh_interval_seconds: 30,
            authentication: DashboardAuthConfig::default(),
            features: DashboardFeatures::default(),
        }
    }
}

impl Default for DashboardAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: AuthProvider::Local,
            session_timeout_minutes: 60,
        }
    }
}

impl Default for DashboardFeatures {
    fn default() -> Self {
        Self {
            real_time_updates: true,
            test_execution: true,
            report_generation: true,
            alert_management: true,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: "postgresql://localhost:5432/ai_core_test".to_string(),
            redis_url: "redis://localhost:6379/1".to_string(),
            mongodb_url: "mongodb://localhost:27017/ai_core_test".to_string(),
            clickhouse_url: "http://localhost:8123/ai_core_test".to_string(),
            pool: DatabasePoolConfig::default(),
        }
    }
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 600,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            prometheus_url: "http://localhost:9090".to_string(),
            grafana_url: "http://localhost:3000".to_string(),
            jaeger_url: "http://localhost:14268".to_string(),
            metrics_endpoint: "http://localhost:9090/metrics".to_string(),
            health_endpoints: vec![
                "http://localhost:8000/health".to_string(),
                "http://localhost:8001/health".to_string(),
            ],
        }
    }
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: PathBuf::from("target/qa-reports"),
            formats: vec![ReportFormat::Html, ReportFormat::Json],
            templates_dir: PathBuf::from("src/qa-agent/templates"),
            email_notifications: EmailNotificationConfig::default(),
        }
    }
}

impl Default for EmailNotificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_server: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            recipients: vec![],
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            agent_name: "AI-CORE QA Agent".to_string(),
            agent_version: "1.0.0".to_string(),
            log_level: LogLevel::Info,
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            temp_dir: std::env::temp_dir(),
            debug_mode: false,
        }
    }
}

impl QAConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: QAConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = QAConfig::default();

        // Override with environment variables
        if let Ok(postgres_url) = std::env::var("QA_POSTGRES_URL") {
            config.database.postgres_url = postgres_url;
        }
        if let Ok(redis_url) = std::env::var("QA_REDIS_URL") {
            config.database.redis_url = redis_url;
        }
        if let Ok(mongodb_url) = std::env::var("QA_MONGODB_URL") {
            config.database.mongodb_url = mongodb_url;
        }
        if let Ok(dashboard_port) = std::env::var("QA_DASHBOARD_PORT") {
            config.dashboard.port = dashboard_port.parse()?;
        }

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate URLs
        Url::parse(&self.database.postgres_url)?;
        Url::parse(&self.database.redis_url)?;
        Url::parse(&self.monitoring.prometheus_url)?;

        // Validate directories exist or can be created
        std::fs::create_dir_all(&self.test.fixtures_dir)?;
        std::fs::create_dir_all(&self.test.results_dir)?;
        std::fs::create_dir_all(&self.reporting.output_dir)?;

        // Validate thresholds
        if self.test.min_coverage_threshold < 0.0 || self.test.min_coverage_threshold > 100.0 {
            anyhow::bail!("Coverage threshold must be between 0 and 100");
        }

        if self.performance.sla_thresholds.error_rate_percent < 0.0
            || self.performance.sla_thresholds.error_rate_percent > 100.0
        {
            anyhow::bail!("Error rate threshold must be between 0 and 100");
        }

        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = QAConfig::default();
        assert!(config.test.parallel_execution);
        assert!(config.performance.enabled);
        assert!(config.security.enabled);
        assert!(config.metrics.enabled);
        assert!(config.dashboard.enabled);
    }

    #[test]
    fn test_config_validation() {
        let config = QAConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = QAConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: QAConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.general.agent_name, deserialized.general.agent_name);
        assert_eq!(config.dashboard.port, deserialized.dashboard.port);
    }

    #[test]
    fn test_config_file_operations() {
        let config = QAConfig::default();
        let temp_file = NamedTempFile::new().unwrap();

        // Save to file
        config.save_to_file(&temp_file.path()).unwrap();

        // Load from file
        let loaded_config = QAConfig::from_file(&temp_file.path()).unwrap();
        assert_eq!(config.general.agent_name, loaded_config.general.agent_name);
    }
}
