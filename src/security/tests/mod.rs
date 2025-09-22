//! Security Test Module
//!
//! This module contains comprehensive tests for the AI-CORE security framework.

#[cfg(test)]
pub mod integration_tests;

#[cfg(test)]
pub mod vulnerability_tests;

#[cfg(test)]
pub mod basic_verification;

// Benchmarks are only available with criterion feature
#[cfg(all(test, feature = "dev-tools"))]
pub mod benchmarks;

// Test utilities and helpers
#[cfg(test)]
pub mod test_utils {
    use crate::config::SecurityConfig;
    use crate::service::SecurityService;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Create a test security service with default configuration
    pub async fn create_test_security_service() -> Arc<SecurityService> {
        let config = SecurityConfig::default();
        Arc::new(SecurityService::new(config).await.unwrap())
    }

    /// Generate a test user ID
    pub fn generate_test_user_id() -> Uuid {
        Uuid::new_v4()
    }

    /// Generate test email address
    pub fn generate_test_email(prefix: &str) -> String {
        format!("{}@test.example.com", prefix)
    }

    /// Generate test roles
    pub fn generate_test_roles() -> Vec<String> {
        vec!["user".to_string(), "test".to_string()]
    }

    /// Generate admin roles
    pub fn generate_admin_roles() -> Vec<String> {
        vec!["admin".to_string(), "superuser".to_string()]
    }

    /// Test data for various security scenarios
    pub struct TestData {
        pub user_id: Uuid,
        pub email: String,
        pub roles: Vec<String>,
        pub password: String,
    }

    impl TestData {
        pub fn new_user() -> Self {
            Self {
                user_id: generate_test_user_id(),
                email: generate_test_email("user"),
                roles: generate_test_roles(),
                password: "TestPassword123!".to_string(),
            }
        }

        pub fn new_admin() -> Self {
            Self {
                user_id: generate_test_user_id(),
                email: generate_test_email("admin"),
                roles: generate_admin_roles(),
                password: "AdminPassword456!".to_string(),
            }
        }
    }

    /// Mock HTTP request builder for testing
    pub struct MockRequestBuilder {
        method: String,
        path: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<String>,
    }

    impl MockRequestBuilder {
        pub fn new(method: &str, path: &str) -> Self {
            Self {
                method: method.to_string(),
                path: path.to_string(),
                headers: std::collections::HashMap::new(),
                body: None,
            }
        }

        pub fn header(mut self, name: &str, value: &str) -> Self {
            self.headers.insert(name.to_string(), value.to_string());
            self
        }

        pub fn auth_header(mut self, token: &str) -> Self {
            self.headers
                .insert("authorization".to_string(), format!("Bearer {}", token));
            self
        }

        pub fn body(mut self, body: &str) -> Self {
            self.body = Some(body.to_string());
            self
        }

        pub fn content_type(mut self, content_type: &str) -> Self {
            self.headers
                .insert("content-type".to_string(), content_type.to_string());
            self
        }

        pub fn user_agent(mut self, user_agent: &str) -> Self {
            self.headers
                .insert("user-agent".to_string(), user_agent.to_string());
            self
        }

        pub fn x_forwarded_for(mut self, ip: &str) -> Self {
            self.headers
                .insert("x-forwarded-for".to_string(), ip.to_string());
            self
        }
    }

    /// Security test scenarios
    pub enum SecurityTestScenario {
        ValidAuthentication,
        InvalidToken,
        MissingToken,
        ExpiredToken,
        BlacklistedToken,
        InsufficientPermissions,
        RateLimitExceeded,
        MaliciousInput,
        BruteForceAttack,
        SqlInjection,
        XssAttack,
        PathTraversal,
        CommandInjection,
    }

    impl SecurityTestScenario {
        pub fn description(&self) -> &'static str {
            match self {
                SecurityTestScenario::ValidAuthentication => {
                    "Valid authentication with proper token"
                }
                SecurityTestScenario::InvalidToken => "Invalid or malformed JWT token",
                SecurityTestScenario::MissingToken => "Missing authentication token",
                SecurityTestScenario::ExpiredToken => "Expired authentication token",
                SecurityTestScenario::BlacklistedToken => "Blacklisted authentication token",
                SecurityTestScenario::InsufficientPermissions => "User lacks required permissions",
                SecurityTestScenario::RateLimitExceeded => "Request rate limit exceeded",
                SecurityTestScenario::MaliciousInput => "Malicious input patterns detected",
                SecurityTestScenario::BruteForceAttack => "Brute force attack pattern",
                SecurityTestScenario::SqlInjection => "SQL injection attack attempt",
                SecurityTestScenario::XssAttack => "Cross-site scripting attack",
                SecurityTestScenario::PathTraversal => "Path traversal attack attempt",
                SecurityTestScenario::CommandInjection => "Command injection attack",
            }
        }

        pub fn expected_status_code(&self) -> u16 {
            match self {
                SecurityTestScenario::ValidAuthentication => 200,
                SecurityTestScenario::InvalidToken => 401,
                SecurityTestScenario::MissingToken => 401,
                SecurityTestScenario::ExpiredToken => 401,
                SecurityTestScenario::BlacklistedToken => 401,
                SecurityTestScenario::InsufficientPermissions => 403,
                SecurityTestScenario::RateLimitExceeded => 429,
                SecurityTestScenario::MaliciousInput => 400,
                SecurityTestScenario::BruteForceAttack => 429,
                SecurityTestScenario::SqlInjection => 400,
                SecurityTestScenario::XssAttack => 400,
                SecurityTestScenario::PathTraversal => 400,
                SecurityTestScenario::CommandInjection => 400,
            }
        }
    }

    /// Test assertion helpers
    pub fn assert_security_headers_present(headers: &std::collections::HashMap<String, String>) {
        let required_headers = [
            "x-content-type-options",
            "x-frame-options",
            "x-xss-protection",
            "strict-transport-security",
            "referrer-policy",
            "content-security-policy",
        ];

        for header in &required_headers {
            assert!(
                headers.contains_key(*header),
                "Required security header missing: {}",
                header
            );
        }
    }

    pub fn assert_rate_limit_headers_present(headers: &std::collections::HashMap<String, String>) {
        let rate_limit_headers = [
            "x-ratelimit-limit",
            "x-ratelimit-remaining",
            "x-ratelimit-reset",
        ];

        for header in &rate_limit_headers {
            if headers.contains_key(*header) {
                // At least one rate limit header should be present
                return;
            }
        }
        panic!("No rate limit headers found");
    }

    /// Performance testing utilities
    pub struct PerformanceTest {
        pub name: String,
        pub iterations: usize,
        pub max_duration_ms: u128,
        pub min_requests_per_second: f64,
    }

    impl PerformanceTest {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                iterations: 1000,
                max_duration_ms: 5000,
                min_requests_per_second: 100.0,
            }
        }

        pub fn iterations(mut self, iterations: usize) -> Self {
            self.iterations = iterations;
            self
        }

        pub fn max_duration_ms(mut self, duration: u128) -> Self {
            self.max_duration_ms = duration;
            self
        }

        pub fn min_rps(mut self, rps: f64) -> Self {
            self.min_requests_per_second = rps;
            self
        }
    }

    /// Security event logging for tests
    pub struct TestSecurityLogger {
        events: Arc<tokio::sync::Mutex<Vec<SecurityEvent>>>,
    }

    #[derive(Debug, Clone)]
    pub struct SecurityEvent {
        pub event_type: String,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub user_id: Option<Uuid>,
        pub ip_address: Option<String>,
        pub details: std::collections::HashMap<String, String>,
    }

    impl TestSecurityLogger {
        pub fn new() -> Self {
            Self {
                events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        pub async fn log_event(&self, event: SecurityEvent) {
            let mut events = self.events.lock().await;
            events.push(event);
        }

        pub async fn get_events(&self) -> Vec<SecurityEvent> {
            let events = self.events.lock().await;
            events.clone()
        }

        pub async fn clear_events(&self) {
            let mut events = self.events.lock().await;
            events.clear();
        }

        pub async fn count_events_by_type(&self, event_type: &str) -> usize {
            let events = self.events.lock().await;
            events.iter().filter(|e| e.event_type == event_type).count()
        }
    }

    /// Common test constants
    pub mod constants {
        use std::time::Duration;

        pub const TEST_JWT_SECRET: &str = "test-secret-key-for-jwt-signing-in-tests-only";
        pub const TEST_ENCRYPTION_KEY: &str = "test-encryption-key-32-bytes-long!";
        pub const TEST_DATABASE_URL: &str = "sqlite::memory:";
        pub const TEST_REDIS_URL: &str = "redis://127.0.0.1:6379";

        pub const DEFAULT_TOKEN_EXPIRY: Duration = Duration::from_secs(3600); // 1 hour
        pub const SHORT_TOKEN_EXPIRY: Duration = Duration::from_secs(60); // 1 minute
        pub const TEST_RATE_LIMIT: u32 = 100; // requests per minute
        pub const TEST_BURST_MULTIPLIER: f64 = 1.5;

        pub const BRUTE_FORCE_THRESHOLD: u32 = 5;
        pub const BLACKLIST_DURATION: Duration = Duration::from_secs(300); // 5 minutes

        pub const MAX_REQUEST_SIZE: usize = 1024 * 1024; // 1 MB
        pub const MAX_HEADER_COUNT: usize = 50;
        pub const MAX_HEADER_LENGTH: usize = 1024;
    }

    /// Test configuration builders
    pub mod config_builders {
        use super::constants::*;
        use crate::config::*;

        pub fn test_security_config() -> SecurityConfig {
            let mut config = SecurityConfig::default();
            config.jwt.secret_key = TEST_JWT_SECRET.to_string();
            config.jwt.access_token_ttl = DEFAULT_TOKEN_EXPIRY;
            config.encryption.master_key = TEST_ENCRYPTION_KEY.to_string();
            config.rate_limiting.requests_per_minute = TEST_RATE_LIMIT;
            config.threat_detection.max_login_attempts = BRUTE_FORCE_THRESHOLD;
            config
        }

        pub fn permissive_security_config() -> SecurityConfig {
            let mut config = test_security_config();
            config.rate_limiting.requests_per_minute = 10000;
            config.threat_detection.max_login_attempts = 1000;
            config.input_validation.max_request_size = 10 * 1024 * 1024; // 10 MB
            config
        }

        pub fn strict_security_config() -> SecurityConfig {
            let mut config = test_security_config();
            config.rate_limiting.requests_per_minute = 10;
            config.threat_detection.max_login_attempts = 3;
            config.input_validation.max_request_size = 1024; // 1 KB
            config.input_validation.max_header_count = 10;
            config
        }
    }
}

#[cfg(test)]
mod test_runner {
    //! Test runner utilities for coordinating security tests

    use super::test_utils::*;
    use std::time::{Duration, Instant};

    /// Test suite runner for security tests
    pub struct SecurityTestSuite {
        pub name: String,
        pub tests: Vec<SecurityTestCase>,
        pub setup_timeout: Duration,
        pub test_timeout: Duration,
    }

    pub struct SecurityTestCase {
        pub name: String,
        pub scenario: SecurityTestScenario,
        pub expected_result: TestResult,
    }

    pub enum TestResult {
        Success,
        Failure(String),
        Timeout,
    }

    impl SecurityTestSuite {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                tests: Vec::new(),
                setup_timeout: Duration::from_secs(30),
                test_timeout: Duration::from_secs(10),
            }
        }

        pub fn add_test(mut self, name: &str, scenario: SecurityTestScenario) -> Self {
            self.tests.push(SecurityTestCase {
                name: name.to_string(),
                scenario,
                expected_result: TestResult::Success,
            });
            self
        }

        pub async fn run(&self) -> SecurityTestReport {
            let start_time = Instant::now();
            let mut results = Vec::new();

            println!("Running security test suite: {}", self.name);
            println!("Total tests: {}", self.tests.len());

            for (index, test_case) in self.tests.iter().enumerate() {
                println!(
                    "Running test {}/{}: {}",
                    index + 1,
                    self.tests.len(),
                    test_case.name
                );

                let test_start = Instant::now();
                let result = self.run_single_test(test_case).await;
                let test_duration = test_start.elapsed();

                results.push(SecurityTestResult {
                    test_name: test_case.name.clone(),
                    scenario: test_case.scenario.clone(),
                    result,
                    duration: test_duration,
                });
            }

            let total_duration = start_time.elapsed();
            let passed = results
                .iter()
                .filter(|r| matches!(r.result, TestResult::Success))
                .count();
            let failed = results.len() - passed;

            SecurityTestReport {
                suite_name: self.name.clone(),
                total_tests: self.tests.len(),
                passed,
                failed,
                results,
                total_duration,
            }
        }

        async fn run_single_test(&self, _test_case: &SecurityTestCase) -> TestResult {
            // This would contain the actual test execution logic
            // For now, returning success as a placeholder
            TestResult::Success
        }
    }

    pub struct SecurityTestResult {
        pub test_name: String,
        pub scenario: SecurityTestScenario,
        pub result: TestResult,
        pub duration: Duration,
    }

    pub struct SecurityTestReport {
        pub suite_name: String,
        pub total_tests: usize,
        pub passed: usize,
        pub failed: usize,
        pub results: Vec<SecurityTestResult>,
        pub total_duration: Duration,
    }

    impl SecurityTestReport {
        pub fn print_summary(&self) {
            println!("\n{'=':<60}");
            println!("Security Test Suite Report: {}", self.suite_name);
            println!("{'=':<60}");
            println!("Total Tests: {}", self.total_tests);
            println!(
                "Passed: {} ({:.1}%)",
                self.passed,
                (self.passed as f64 / self.total_tests as f64) * 100.0
            );
            println!(
                "Failed: {} ({:.1}%)",
                self.failed,
                (self.failed as f64 / self.total_tests as f64) * 100.0
            );
            println!("Total Duration: {:.2}s", self.total_duration.as_secs_f64());
            println!("{'=':<60}");

            if self.failed > 0 {
                println!("\nFailed Tests:");
                for result in &self.results {
                    if let TestResult::Failure(reason) = &result.result {
                        println!("  ❌ {}: {}", result.test_name, reason);
                    }
                }
            }

            println!("\nTest Details:");
            for result in &self.results {
                let status = match &result.result {
                    TestResult::Success => "✅ PASS",
                    TestResult::Failure(_) => "❌ FAIL",
                    TestResult::Timeout => "⏰ TIMEOUT",
                };
                println!(
                    "  {} {} ({:.2}s) - {}",
                    status,
                    result.test_name,
                    result.duration.as_secs_f64(),
                    result.scenario.description()
                );
            }
        }

        pub fn is_successful(&self) -> bool {
            self.failed == 0
        }

        pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let json_report = serde_json::json!({
                "suite_name": self.suite_name,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "summary": {
                    "total_tests": self.total_tests,
                    "passed": self.passed,
                    "failed": self.failed,
                    "success_rate": (self.passed as f64 / self.total_tests as f64) * 100.0,
                    "total_duration_seconds": self.total_duration.as_secs_f64()
                },
                "results": self.results.iter().map(|r| {
                    serde_json::json!({
                        "test_name": r.test_name,
                        "scenario": r.scenario.description(),
                        "status": match &r.result {
                            TestResult::Success => "pass",
                            TestResult::Failure(_) => "fail",
                            TestResult::Timeout => "timeout"
                        },
                        "duration_seconds": r.duration.as_secs_f64()
                    })
                }).collect::<Vec<_>>()
            });

            std::fs::write(path, serde_json::to_string_pretty(&json_report)?)?;
            Ok(())
        }
    }
}
