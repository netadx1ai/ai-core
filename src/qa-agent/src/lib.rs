//! # AI-CORE Quality Assurance Agent
//!
//! Comprehensive quality assurance framework providing:
//! - Testing infrastructure orchestration
//! - Automated testing pipeline coordination
//! - Performance testing and benchmarking
//! - Security testing automation
//! - Quality metrics collection and dashboard
//!
//! ## Features
//!
//! - **Test Orchestration**: Coordinates unit, integration, e2e, and performance tests
//! - **Quality Gates**: Automated BUILD/RUN/TEST/FIX validation cycles
//! - **Performance Monitoring**: SLA validation and performance regression detection
//! - **Security Testing**: Automated vulnerability scanning and penetration testing
//! - **Quality Dashboard**: Real-time quality metrics and trend analysis
//! - **CI/CD Integration**: Seamless integration with existing CI/CD pipelines
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    QA Agent Architecture                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌───────────────┐  ┌──────────────────┐  ┌─────────────────┐   │
//! │  │ Test          │  │ Performance      │  │ Security        │   │
//! │  │ Orchestrator  │  │ Testing Suite    │  │ Testing Suite   │   │
//! │  └───────────────┘  └──────────────────┘  └─────────────────┘   │
//! │           │                   │                      │          │
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │              Quality Metrics Collector                   │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! │           │                   │                      │          │
//! │  ┌───────────────────────────────────────────────────────────┐  │
//! │  │              Quality Dashboard & Reporting                │  │
//! │  └───────────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod dashboard;
pub mod metrics;
pub mod orchestrator;
pub mod performance;
pub mod reporting;
pub mod security;
pub mod testing;
pub mod utils;

// Re-export key types and traits
pub use config::{PerformanceConfig, QAConfig, SecurityConfig, TestConfig};
pub use dashboard::{DashboardService, QualityDashboard};
pub use metrics::{MetricsCollector, QualityMetricsResult, QualityScore};
pub use orchestrator::{TestOrchestrator, TestSuite, TestSuiteResult};
pub use performance::{PerformanceBenchmark, PerformanceTester};
pub use reporting::{QualityReport, ReportFormat, ReportGenerator};
pub use security::{SecurityScan, SecurityTester, VulnerabilityStatus};
pub use testing::{TestCase, TestRunner, TestStatus};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// QA Agent main coordinator
#[derive(Debug, Clone)]
pub struct QAAgent {
    pub config: QAConfig,
    pub orchestrator: TestOrchestrator,
    pub performance_tester: PerformanceTester,
    pub security_tester: SecurityTester,
    pub metrics_collector: MetricsCollector,
    pub dashboard: QualityDashboard,
}

impl QAAgent {
    /// Initialize the QA Agent with configuration
    pub async fn new(config: QAConfig) -> Result<Self> {
        let orchestrator = TestOrchestrator::new(config.test.clone()).await?;
        let performance_tester = PerformanceTester::new(config.performance.clone()).await?;
        let security_tester = SecurityTester::new(config.security.clone()).await?;
        let metrics_collector = MetricsCollector::new(config.metrics.clone()).await?;
        let dashboard = QualityDashboard::new(config.dashboard.clone()).await?;

        Ok(Self {
            config,
            orchestrator,
            performance_tester,
            security_tester,
            metrics_collector,
            dashboard,
        })
    }

    /// Run comprehensive quality assurance workflow
    pub async fn run_qa_workflow(&self) -> Result<QAWorkflowResult> {
        tracing::info!("Starting comprehensive QA workflow");

        let workflow_id = Uuid::new_v4();
        let start_time = Utc::now();

        // Phase 1: Infrastructure validation
        let infrastructure_result = self.validate_infrastructure().await?;

        // Phase 2: Test orchestration
        let test_result = self.orchestrator.run_all_tests().await?;

        // Phase 3: Performance testing
        let performance_result = self.performance_tester.run_performance_suite().await?;

        // Phase 4: Security testing
        let security_result = self.security_tester.run_security_suite().await?;

        // Phase 5: Quality metrics collection
        let metrics_result = self.metrics_collector.collect_quality_metrics().await?;

        // Phase 6: Report generation
        let report = self
            .generate_comprehensive_report(
                &test_result,
                &performance_result,
                &security_result,
                &metrics_result,
            )
            .await?;

        let end_time = Utc::now();
        let duration = end_time - start_time;

        let overall_status =
            self.calculate_overall_status(&test_result, &performance_result, &security_result);

        let workflow_result = QAWorkflowResult {
            workflow_id,
            start_time,
            end_time,
            duration: duration.num_seconds(),
            infrastructure_result,
            test_result: test_result.clone(),
            performance_result: performance_result.clone(),
            security_result: security_result.clone(),
            metrics_result,
            report,
            overall_status,
        };

        // Update dashboard
        self.dashboard
            .update_workflow_result(&workflow_result)
            .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            duration_seconds = duration.num_seconds(),
            status = ?workflow_result.overall_status,
            "QA workflow completed"
        );

        Ok(workflow_result)
    }

    /// Validate testing infrastructure
    async fn validate_infrastructure(&self) -> Result<InfrastructureValidationResult> {
        tracing::info!("Validating testing infrastructure");

        let mut validations = HashMap::new();

        // Validate test environment setup
        validations.insert(
            "test_environment".to_string(),
            self.validate_test_environment().await?,
        );

        // Validate database connections
        validations.insert(
            "database_connectivity".to_string(),
            self.validate_database_connectivity().await?,
        );

        // Validate external dependencies
        validations.insert(
            "external_dependencies".to_string(),
            self.validate_external_dependencies().await?,
        );

        // Validate monitoring systems
        validations.insert(
            "monitoring_systems".to_string(),
            self.validate_monitoring_systems().await?,
        );

        let all_passed = validations
            .values()
            .all(|v| v.status == ValidationStatus::Passed);
        let overall_status = if all_passed {
            ValidationStatus::Passed
        } else {
            ValidationStatus::Failed
        };

        Ok(InfrastructureValidationResult {
            overall_status,
            validations,
            timestamp: Utc::now(),
        })
    }

    /// Calculate overall QA workflow status
    fn calculate_overall_status(
        &self,
        test_result: &orchestrator::TestSuiteResult,
        performance_result: &performance::PerformanceTestResult,
        security_result: &security::SecurityTestResult,
    ) -> QAStatus {
        let test_passed = test_result.status == testing::TestStatus::Passed;
        let performance_passed =
            performance_result.status == performance::PerformanceStatus::Passed;
        let security_passed = security_result.status == security::SecurityStatus::Passed;

        if test_passed && performance_passed && security_passed {
            QAStatus::Passed
        } else if !test_passed {
            QAStatus::TestsFailed
        } else if !performance_passed {
            QAStatus::PerformanceFailed
        } else if !security_passed {
            QAStatus::SecurityFailed
        } else {
            QAStatus::Unknown
        }
    }

    /// Generate comprehensive QA report
    async fn generate_comprehensive_report(
        &self,
        test_result: &orchestrator::TestSuiteResult,
        performance_result: &performance::PerformanceTestResult,
        security_result: &security::SecurityTestResult,
        metrics_result: &metrics::QualityMetricsResult,
    ) -> Result<QualityReport> {
        let report_generator = ReportGenerator::new(self.config.reporting.clone());

        report_generator
            .generate_comprehensive_report(
                test_result,
                performance_result,
                security_result,
                metrics_result,
            )
            .await
    }

    // Infrastructure validation methods
    async fn validate_test_environment(&self) -> Result<ValidationResult> {
        // Validate Rust toolchain
        if !utils::command_exists("cargo").await? {
            return Ok(ValidationResult {
                status: ValidationStatus::Failed,
                message: "Cargo not found - Rust toolchain required".to_string(),
                details: None,
            });
        }

        // Validate Node.js for frontend testing
        if !utils::command_exists("node").await? {
            return Ok(ValidationResult {
                status: ValidationStatus::Failed,
                message: "Node.js not found - required for frontend testing".to_string(),
                details: None,
            });
        }

        // Validate Docker for containerized testing
        if !utils::command_exists("docker").await? {
            return Ok(ValidationResult {
                status: ValidationStatus::Warning,
                message: "Docker not found - containerized testing disabled".to_string(),
                details: None,
            });
        }

        Ok(ValidationResult {
            status: ValidationStatus::Passed,
            message: "Test environment validation passed".to_string(),
            details: Some("All required tools are available".to_string()),
        })
    }

    async fn validate_database_connectivity(&self) -> Result<ValidationResult> {
        let mut issues = Vec::new();

        // Test PostgreSQL connection
        if let Err(e) = utils::test_postgres_connection(&self.config.database.postgres_url).await {
            issues.push(format!("PostgreSQL connection failed: {}", e));
        }

        // Test Redis connection
        if let Err(e) = utils::test_redis_connection(&self.config.database.redis_url).await {
            issues.push(format!("Redis connection failed: {}", e));
        }

        // Test MongoDB connection
        if let Err(e) = utils::test_mongodb_connection(&self.config.database.mongodb_url).await {
            issues.push(format!("MongoDB connection failed: {}", e));
        }

        if issues.is_empty() {
            Ok(ValidationResult {
                status: ValidationStatus::Passed,
                message: "All database connections successful".to_string(),
                details: None,
            })
        } else {
            Ok(ValidationResult {
                status: ValidationStatus::Failed,
                message: "Database connectivity issues detected".to_string(),
                details: Some(issues.join("; ")),
            })
        }
    }

    async fn validate_external_dependencies(&self) -> Result<ValidationResult> {
        // Validate external services availability
        let services_to_check = [
            ("Prometheus", &self.config.monitoring.prometheus_url),
            ("Grafana", &self.config.monitoring.grafana_url),
            ("Jaeger", &self.config.monitoring.jaeger_url),
        ];

        let mut failed_services = Vec::new();

        for (service_name, url) in &services_to_check {
            if let Err(e) = utils::check_service_health(url).await {
                failed_services.push(format!("{}: {}", service_name, e));
            }
        }

        if failed_services.is_empty() {
            Ok(ValidationResult {
                status: ValidationStatus::Passed,
                message: "All external dependencies available".to_string(),
                details: None,
            })
        } else {
            Ok(ValidationResult {
                status: ValidationStatus::Warning,
                message: "Some external dependencies unavailable".to_string(),
                details: Some(failed_services.join("; ")),
            })
        }
    }

    async fn validate_monitoring_systems(&self) -> Result<ValidationResult> {
        // Validate metrics collection endpoint
        if let Err(e) =
            utils::check_metrics_endpoint(&self.config.monitoring.metrics_endpoint).await
        {
            return Ok(ValidationResult {
                status: ValidationStatus::Failed,
                message: "Metrics endpoint validation failed".to_string(),
                details: Some(e.to_string()),
            });
        }

        Ok(ValidationResult {
            status: ValidationStatus::Passed,
            message: "Monitoring systems validation passed".to_string(),
            details: None,
        })
    }
}

/// QA workflow execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAWorkflowResult {
    pub workflow_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: i64, // seconds
    pub infrastructure_result: InfrastructureValidationResult,
    pub test_result: orchestrator::TestSuiteResult,
    pub performance_result: performance::PerformanceTestResult,
    pub security_result: security::SecurityTestResult,
    pub metrics_result: metrics::QualityMetricsResult,
    pub report: QualityReport,
    pub overall_status: QAStatus,
}

/// Infrastructure validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureValidationResult {
    pub overall_status: ValidationStatus,
    pub validations: HashMap<String, ValidationResult>,
    pub timestamp: DateTime<Utc>,
}

/// Individual validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub status: ValidationStatus,
    pub message: String,
    pub details: Option<String>,
}

/// Validation status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Passed,
    Warning,
    Failed,
}

/// Overall QA status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QAStatus {
    Passed,
    TestsFailed,
    PerformanceFailed,
    SecurityFailed,
    InfrastructureFailed,
    Unknown,
}

impl std::fmt::Display for QAStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QAStatus::Passed => write!(f, "PASSED"),
            QAStatus::TestsFailed => write!(f, "TESTS_FAILED"),
            QAStatus::PerformanceFailed => write!(f, "PERFORMANCE_FAILED"),
            QAStatus::SecurityFailed => write!(f, "SECURITY_FAILED"),
            QAStatus::InfrastructureFailed => write!(f, "INFRASTRUCTURE_FAILED"),
            QAStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_qa_agent_initialization() {
        let config = QAConfig::default();
        let qa_agent = QAAgent::new(config).await;
        assert!(qa_agent.is_ok());
    }

    #[tokio::test]
    async fn test_qa_status_display() {
        assert_eq!(QAStatus::Passed.to_string(), "PASSED");
        assert_eq!(QAStatus::TestsFailed.to_string(), "TESTS_FAILED");
        assert_eq!(
            QAStatus::PerformanceFailed.to_string(),
            "PERFORMANCE_FAILED"
        );
        assert_eq!(QAStatus::SecurityFailed.to_string(), "SECURITY_FAILED");
    }

    #[tokio::test]
    async fn test_validation_status_equality() {
        assert_eq!(ValidationStatus::Passed, ValidationStatus::Passed);
        assert_ne!(ValidationStatus::Passed, ValidationStatus::Failed);
    }
}
