//! # Test Orchestrator Module
//!
//! Coordinates and manages execution of all test suites across the AI-CORE platform.
//! Provides centralized test execution, result aggregation, and reporting.

use crate::config::{TestConfig, TestSuiteConfig, TestSuiteType};
use crate::testing::{TestCase, TestRunner, TestStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Test orchestrator for coordinating all test execution
#[derive(Debug, Clone)]
pub struct TestOrchestrator {
    config: TestConfig,
    test_runners: HashMap<TestSuiteType, TestRunner>,
    execution_semaphore: Arc<Semaphore>,
    results_storage: Arc<Mutex<HashMap<Uuid, TestSuiteResult>>>,
}

impl TestOrchestrator {
    /// Create a new test orchestrator
    pub async fn new(config: TestConfig) -> Result<Self> {
        let mut test_runners = HashMap::new();

        // Initialize test runners for each suite type
        for suite_config in &config.suites {
            if suite_config.enabled {
                let runner = TestRunner::new(suite_config.clone()).await?;
                test_runners.insert(suite_config.suite_type.clone(), runner);
            }
        }

        let execution_semaphore = Arc::new(Semaphore::new(config.max_workers));
        let results_storage = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            config,
            test_runners,
            execution_semaphore,
            results_storage,
        })
    }

    /// Run all enabled test suites
    pub async fn run_all_tests(&self) -> Result<TestSuiteResult> {
        info!("Starting comprehensive test execution");

        let execution_id = Uuid::new_v4();
        let start_time = Utc::now();

        // Create execution context
        let execution_context = TestExecutionContext {
            execution_id,
            start_time,
            config: self.config.clone(),
        };

        // Sort test suites by priority (highest first)
        let mut sorted_suites: Vec<_> = self
            .config
            .suites
            .iter()
            .filter(|suite| suite.enabled)
            .collect();
        sorted_suites.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut suite_results = Vec::new();
        let mut overall_status = TestStatus::Passed;

        // Execute test suites based on parallel execution setting
        if self.config.parallel_execution {
            suite_results = self
                .run_suites_parallel(&sorted_suites, &execution_context)
                .await?;
        } else {
            suite_results = self
                .run_suites_sequential(&sorted_suites, &execution_context)
                .await?;
        }

        // Determine overall status
        for result in &suite_results {
            match result.status {
                TestStatus::Failed => {
                    overall_status = TestStatus::Failed;
                    break;
                }
                TestStatus::Skipped => {
                    if overall_status == TestStatus::Passed {
                        overall_status = TestStatus::Skipped;
                    }
                }
                _ => {}
            }
        }

        let end_time = Utc::now();
        let duration = end_time - start_time;

        // Calculate aggregate metrics
        let total_tests = suite_results.iter().map(|r| r.total_tests).sum();
        let passed_tests = suite_results.iter().map(|r| r.passed_tests).sum();
        let failed_tests = suite_results.iter().map(|r| r.failed_tests).sum();
        let skipped_tests = suite_results.iter().map(|r| r.skipped_tests).sum();

        let coverage_percentage = self.calculate_overall_coverage(&suite_results).await?;

        let result = TestSuiteResult {
            execution_id,
            suite_name: "All Test Suites".to_string(),
            suite_type: TestSuiteType::Integration, // Composite type
            status: overall_status.clone(),
            start_time,
            end_time,
            duration: duration.num_seconds(),
            total_tests,
            passed_tests,
            failed_tests,
            skipped_tests,
            suite_results: Some(suite_results),
            test_cases: vec![], // Individual test cases are in suite_results
            coverage_percentage: Some(coverage_percentage),
            artifacts: TestArtifacts::default(),
            metadata: self.create_execution_metadata(&execution_context),
        };

        // Store result
        {
            let mut storage = self.results_storage.lock().await;
            storage.insert(execution_id, result.clone());
        }

        info!(
            execution_id = %execution_id,
            duration_seconds = duration.num_seconds(),
            total_tests = total_tests,
            passed_tests = passed_tests,
            failed_tests = failed_tests,
            status = ?overall_status,
            "Test execution completed"
        );

        Ok(result)
    }

    /// Run test suites in parallel
    async fn run_suites_parallel(
        &self,
        suites: &[&TestSuiteConfig],
        context: &TestExecutionContext,
    ) -> Result<Vec<TestSuiteResult>> {
        let mut handles = Vec::new();

        for suite_config in suites {
            if let Some(runner) = self.test_runners.get(&suite_config.suite_type) {
                let runner = runner.clone();
                let suite_config = suite_config.clone().clone();
                let context = context.clone();
                let semaphore = self.execution_semaphore.clone();

                let handle = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    runner.run_test_suite(&suite_config, &context).await
                });

                handles.push(handle);
            }
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    error!("Test suite execution failed: {}", e);
                    // Create a failed result for the suite
                    results.push(TestSuiteResult::failed_suite(
                        "Unknown".to_string(),
                        TestSuiteType::Unit,
                        format!("Execution error: {}", e),
                    ));
                }
                Err(e) => {
                    error!("Test suite task panicked: {}", e);
                }
            }
        }

        Ok(results)
    }

    /// Run test suites sequentially
    async fn run_suites_sequential(
        &self,
        suites: &[&TestSuiteConfig],
        context: &TestExecutionContext,
    ) -> Result<Vec<TestSuiteResult>> {
        let mut results = Vec::new();

        for suite_config in suites {
            if let Some(runner) = self.test_runners.get(&suite_config.suite_type) {
                info!("Running test suite: {}", suite_config.name);

                match runner.run_test_suite(suite_config, context).await {
                    Ok(result) => {
                        debug!(
                            suite = %suite_config.name,
                            status = ?result.status,
                            tests = result.total_tests,
                            "Test suite completed"
                        );
                        results.push(result);
                    }
                    Err(e) => {
                        error!("Test suite '{}' failed: {}", suite_config.name, e);
                        results.push(TestSuiteResult::failed_suite(
                            suite_config.name.clone(),
                            suite_config.suite_type.clone(),
                            format!("Execution error: {}", e),
                        ));
                    }
                }
            } else {
                warn!(
                    "No test runner found for suite type: {:?}",
                    suite_config.suite_type
                );
            }
        }

        Ok(results)
    }

    /// Run a specific test suite by name
    pub async fn run_test_suite(&self, suite_name: &str) -> Result<TestSuiteResult> {
        let suite_config = self
            .config
            .suites
            .iter()
            .find(|s| s.name == suite_name && s.enabled)
            .ok_or_else(|| anyhow::anyhow!("Test suite '{}' not found or disabled", suite_name))?;

        let runner = self
            .test_runners
            .get(&suite_config.suite_type)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No runner available for suite type: {:?}",
                    suite_config.suite_type
                )
            })?;

        let execution_context = TestExecutionContext {
            execution_id: Uuid::new_v4(),
            start_time: Utc::now(),
            config: self.config.clone(),
        };

        info!("Running specific test suite: {}", suite_name);
        let result = runner
            .run_test_suite(suite_config, &execution_context)
            .await?;

        // Store result
        {
            let mut storage = self.results_storage.lock().await;
            storage.insert(execution_context.execution_id, result.clone());
        }

        Ok(result)
    }

    /// Get test execution results
    pub async fn get_test_results(&self, execution_id: Uuid) -> Option<TestSuiteResult> {
        let storage = self.results_storage.lock().await;
        storage.get(&execution_id).cloned()
    }

    /// List all test execution results
    pub async fn list_test_results(&self) -> Vec<TestSuiteResult> {
        let storage = self.results_storage.lock().await;
        storage.values().cloned().collect()
    }

    /// Cancel running test execution
    pub async fn cancel_test_execution(&self, execution_id: Uuid) -> Result<()> {
        // Implementation would depend on the test runner's cancellation mechanism
        warn!(
            "Test cancellation requested for execution: {}",
            execution_id
        );
        Ok(())
    }

    /// Calculate overall test coverage
    async fn calculate_overall_coverage(&self, suite_results: &[TestSuiteResult]) -> Result<f64> {
        let mut total_lines = 0;
        let mut covered_lines = 0;

        for result in suite_results {
            if let Some(coverage) = result.coverage_percentage {
                // This is a simplified calculation
                // In a real implementation, you'd need to aggregate actual coverage data
                total_lines += 1000; // Placeholder
                covered_lines += (coverage / 100.0 * 1000.0) as u32;
            }
        }

        if total_lines > 0 {
            Ok((covered_lines as f64 / total_lines as f64) * 100.0)
        } else {
            Ok(0.0)
        }
    }

    /// Create execution metadata
    fn create_execution_metadata(&self, context: &TestExecutionContext) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("execution_id".to_string(), context.execution_id.to_string());
        metadata.insert(
            "parallel_execution".to_string(),
            self.config.parallel_execution.to_string(),
        );
        metadata.insert(
            "max_workers".to_string(),
            self.config.max_workers.to_string(),
        );
        metadata.insert(
            "timeout_seconds".to_string(),
            self.config.timeout_seconds.to_string(),
        );
        metadata.insert(
            "retry_attempts".to_string(),
            self.config.retry_attempts.to_string(),
        );
        metadata
    }

    /// Validate test environment before execution
    pub async fn validate_test_environment(&self) -> Result<TestEnvironmentValidation> {
        info!("Validating test environment");

        let mut validations = Vec::new();

        // Check if required directories exist
        for suite_config in &self.config.suites {
            if suite_config.enabled {
                for pattern in &suite_config.include_patterns {
                    let validation = self.validate_test_pattern(pattern).await?;
                    validations.push(validation);
                }
            }
        }

        // Check if fixtures directory exists
        let fixtures_validation = if self.config.fixtures_dir.exists() {
            ValidationItem {
                name: "Test Fixtures Directory".to_string(),
                status: TestStatus::Passed,
                message: Some(format!("Found at: {}", self.config.fixtures_dir.display())),
            }
        } else {
            ValidationItem {
                name: "Test Fixtures Directory".to_string(),
                status: TestStatus::Failed,
                message: Some(format!("Not found: {}", self.config.fixtures_dir.display())),
            }
        };
        validations.push(fixtures_validation);

        // Create results directory if it doesn't exist
        std::fs::create_dir_all(&self.config.results_dir)?;

        let overall_status = if validations.iter().any(|v| v.status == TestStatus::Failed) {
            TestStatus::Failed
        } else {
            TestStatus::Passed
        };

        Ok(TestEnvironmentValidation {
            overall_status,
            validations,
            timestamp: Utc::now(),
        })
    }

    /// Validate a specific test pattern
    async fn validate_test_pattern(&self, pattern: &str) -> Result<ValidationItem> {
        // Simple validation - check if pattern matches any files
        let glob_pattern = glob::glob(pattern)?;
        let mut file_count = 0;

        for entry in glob_pattern {
            if entry.is_ok() {
                file_count += 1;
            }
        }

        if file_count > 0 {
            Ok(ValidationItem {
                name: format!("Test Pattern: {}", pattern),
                status: TestStatus::Passed,
                message: Some(format!("Found {} matching files", file_count)),
            })
        } else {
            Ok(ValidationItem {
                name: format!("Test Pattern: {}", pattern),
                status: TestStatus::Failed,
                message: Some("No matching files found".to_string()),
            })
        }
    }
}

/// Test execution context
#[derive(Debug, Clone)]
pub struct TestExecutionContext {
    pub execution_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub config: TestConfig,
}

/// Test suite execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteResult {
    pub execution_id: Uuid,
    pub suite_name: String,
    pub suite_type: TestSuiteType,
    pub status: TestStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: i64, // seconds
    pub total_tests: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub skipped_tests: u32,
    pub suite_results: Option<Vec<TestSuiteResult>>, // For composite results
    pub test_cases: Vec<TestCaseResult>,
    pub coverage_percentage: Option<f64>,
    pub artifacts: TestArtifacts,
    pub metadata: HashMap<String, String>,
}

impl TestSuiteResult {
    /// Create a failed test suite result
    pub fn failed_suite(name: String, suite_type: TestSuiteType, error_message: String) -> Self {
        let now = Utc::now();
        Self {
            execution_id: Uuid::new_v4(),
            suite_name: name,
            suite_type,
            status: TestStatus::Failed,
            start_time: now,
            end_time: now,
            duration: 0,
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 1,
            skipped_tests: 0,
            suite_results: None,
            test_cases: vec![TestCaseResult {
                name: "Suite Execution".to_string(),
                status: TestStatus::Failed,
                duration: 0,
                error_message: Some(error_message),
                assertions: 0,
                output: None,
            }],
            coverage_percentage: None,
            artifacts: TestArtifacts::default(),
            metadata: HashMap::new(),
        }
    }
}

/// Individual test case result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub name: String,
    pub status: TestStatus,
    pub duration: i64, // milliseconds
    pub error_message: Option<String>,
    pub assertions: u32,
    pub output: Option<String>,
}

/// Test artifacts (logs, screenshots, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestArtifacts {
    pub log_files: Vec<PathBuf>,
    pub screenshots: Vec<PathBuf>,
    pub coverage_reports: Vec<PathBuf>,
    pub performance_reports: Vec<PathBuf>,
    pub other_files: Vec<PathBuf>,
}

/// Test environment validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEnvironmentValidation {
    pub overall_status: TestStatus,
    pub validations: Vec<ValidationItem>,
    pub timestamp: DateTime<Utc>,
}

/// Individual validation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationItem {
    pub name: String,
    pub status: TestStatus,
    pub message: Option<String>,
}

/// Test suite trait for implementing custom test suites
#[async_trait::async_trait]
pub trait TestSuite {
    /// Get the name of this test suite
    fn name(&self) -> &str;

    /// Get the type of this test suite
    fn suite_type(&self) -> TestSuiteType;

    /// Run the test suite
    async fn run(&self, context: &TestExecutionContext) -> Result<TestSuiteResult>;

    /// Validate the test suite can run
    async fn validate(&self) -> Result<()>;

    /// Get test suite configuration
    fn config(&self) -> &TestSuiteConfig;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TestConfig;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = TestConfig::default();
        let orchestrator = TestOrchestrator::new(config).await;
        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_execution_context() {
        let context = TestExecutionContext {
            execution_id: Uuid::new_v4(),
            start_time: Utc::now(),
            config: TestConfig::default(),
        };

        assert!(!context.execution_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_failed_suite_creation() {
        let result = TestSuiteResult::failed_suite(
            "Test Suite".to_string(),
            TestSuiteType::Unit,
            "Test error".to_string(),
        );

        assert_eq!(result.status, TestStatus::Failed);
        assert_eq!(result.failed_tests, 1);
        assert_eq!(result.test_cases.len(), 1);
    }

    #[tokio::test]
    async fn test_test_artifacts_default() {
        let artifacts = TestArtifacts::default();
        assert!(artifacts.log_files.is_empty());
        assert!(artifacts.screenshots.is_empty());
        assert!(artifacts.coverage_reports.is_empty());
    }
}
