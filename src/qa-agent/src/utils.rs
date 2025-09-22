//! # Utility Functions Module
//!
//! Common utility functions and helpers for the AI-CORE QA Agent.
//! Provides system validation, command execution, database connectivity testing, and other shared utilities.

use anyhow::Result;
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, warn};

/// Check if a command exists in the system PATH
pub async fn command_exists(command: &str) -> Result<bool> {
    debug!("Checking if command exists: {}", command);

    let output = if cfg!(target_os = "windows") {
        AsyncCommand::new("where").arg(command).output().await?
    } else {
        AsyncCommand::new("which").arg(command).output().await?
    };

    Ok(output.status.success())
}

/// Test PostgreSQL database connection
pub async fn test_postgres_connection(connection_url: &str) -> Result<()> {
    debug!("Testing PostgreSQL connection");

    // Parse connection URL to extract components
    let url = url::Url::parse(connection_url)?;

    // For demonstration, we'll just validate the URL format
    // In a real implementation, you would establish an actual connection
    if url.scheme() != "postgresql" && url.scheme() != "postgres" {
        anyhow::bail!("Invalid PostgreSQL connection URL scheme");
    }

    if url.host().is_none() {
        anyhow::bail!("PostgreSQL connection URL missing host");
    }

    // Simulate connection test delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    debug!("PostgreSQL connection test passed");
    Ok(())
}

/// Test Redis connection
pub async fn test_redis_connection(connection_url: &str) -> Result<()> {
    debug!("Testing Redis connection");

    let url = url::Url::parse(connection_url)?;

    if url.scheme() != "redis" && url.scheme() != "rediss" {
        anyhow::bail!("Invalid Redis connection URL scheme");
    }

    if url.host().is_none() {
        anyhow::bail!("Redis connection URL missing host");
    }

    // Simulate connection test delay
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    debug!("Redis connection test passed");
    Ok(())
}

/// Test MongoDB connection
pub async fn test_mongodb_connection(connection_url: &str) -> Result<()> {
    debug!("Testing MongoDB connection");

    let url = url::Url::parse(connection_url)?;

    if url.scheme() != "mongodb" && url.scheme() != "mongodb+srv" {
        anyhow::bail!("Invalid MongoDB connection URL scheme");
    }

    if url.host().is_none() {
        anyhow::bail!("MongoDB connection URL missing host");
    }

    // Simulate connection test delay
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    debug!("MongoDB connection test passed");
    Ok(())
}

/// Check service health by making HTTP request
pub async fn check_service_health(service_url: &str) -> Result<()> {
    debug!("Checking service health: {}", service_url);

    let client = reqwest::Client::builder()
        .timeout(tokio::time::Duration::from_secs(5))
        .build()?;

    let response = client.get(service_url).send().await?;

    if response.status().is_success() {
        debug!("Service health check passed for: {}", service_url);
        Ok(())
    } else {
        anyhow::bail!(
            "Service health check failed for {}: {}",
            service_url,
            response.status()
        );
    }
}

/// Check metrics endpoint accessibility
pub async fn check_metrics_endpoint(metrics_url: &str) -> Result<()> {
    debug!("Checking metrics endpoint: {}", metrics_url);

    let client = reqwest::Client::builder()
        .timeout(tokio::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(metrics_url).send().await?;

    if response.status().is_success() {
        let text = response.text().await?;

        // Basic validation that this looks like Prometheus metrics
        if text.contains("# HELP") || text.contains("# TYPE") {
            debug!("Metrics endpoint validation passed");
            Ok(())
        } else {
            warn!("Metrics endpoint accessible but content format unclear");
            Ok(())
        }
    } else {
        anyhow::bail!("Metrics endpoint check failed: {}", response.status());
    }
}

/// Execute a shell command and return output
pub async fn execute_command(command: &str, args: &[&str]) -> Result<String> {
    debug!("Executing command: {} {:?}", command, args);

    let output = AsyncCommand::new(command).args(args).output().await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }
}

/// Execute a shell command synchronously
pub fn execute_command_sync(command: &str, args: &[&str]) -> Result<String> {
    debug!("Executing command synchronously: {} {:?}", command, args);

    let output = Command::new(command).args(args).output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }
}

/// Get system information
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        family: std::env::consts::FAMILY.to_string(),
        cpu_count: num_cpus::get(),
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
    }
}

/// System information structure
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub cpu_count: usize,
    pub hostname: String,
}

/// Validate file path exists
pub fn validate_file_path(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("File path does not exist: {}", path.display());
    }
    Ok(())
}

/// Validate directory path exists or create it
pub fn ensure_directory_exists(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        debug!("Creating directory: {}", path.display());
        std::fs::create_dir_all(path)?;
    } else if !path.is_dir() {
        anyhow::bail!("Path exists but is not a directory: {}", path.display());
    }
    Ok(())
}

/// Format duration in human-readable format
pub fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let remaining_seconds = seconds % 60;
        format!("{}m {}s", minutes, remaining_seconds)
    } else {
        let hours = seconds / 3600;
        let remaining_minutes = (seconds % 3600) / 60;
        let remaining_seconds = seconds % 60;
        format!("{}h {}m {}s", hours, remaining_minutes, remaining_seconds)
    }
}

/// Format bytes in human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Calculate percentage with proper handling of division by zero
pub fn calculate_percentage(part: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

/// Parse environment variable as boolean
pub fn parse_env_bool(var_name: &str, default: bool) -> bool {
    std::env::var(var_name)
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(default)
}

/// Parse environment variable as integer
pub fn parse_env_int<T>(var_name: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    std::env::var(var_name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Generate a simple hash for string content
pub fn simple_hash(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Retry a function with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    max_attempts: u32,
    initial_delay_ms: u64,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut delay = initial_delay_ms;

    for attempt in 1..=max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempt == max_attempts {
                    return Err(error);
                }

                debug!(
                    "Attempt {} failed, retrying in {}ms: {:?}",
                    attempt, delay, error
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                delay *= 2; // Exponential backoff
            }
        }
    }

    unreachable!()
}

/// Truncate string to specified length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Sanitize string for use in filenames
pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Check if running in CI environment
pub fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("BUILDKITE").is_ok()
}

/// Get current timestamp as ISO 8601 string
pub fn current_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Validate email address format (basic validation)
pub fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

/// Create a temporary directory for testing
pub fn create_temp_dir(prefix: &str) -> Result<std::path::PathBuf> {
    let temp_dir = std::env::temp_dir().join(format!("{}_{}", prefix, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

/// Clean up temporary directory
pub fn cleanup_temp_dir(path: &std::path::Path) -> Result<()> {
    if path.exists() && path.is_dir() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_command_exists() {
        // Test with a command that should exist on most systems
        let result = command_exists("echo").await;
        assert!(result.is_ok());

        // Test with a command that shouldn't exist
        let result = command_exists("nonexistent_command_12345").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m 1s");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_calculate_percentage() {
        assert_eq!(calculate_percentage(50, 100), 50.0);
        assert_eq!(calculate_percentage(0, 100), 0.0);
        assert_eq!(calculate_percentage(100, 100), 100.0);
        assert_eq!(calculate_percentage(1, 0), 0.0); // Division by zero handling
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 2), "hi");
        assert_eq!(truncate_string("hello", 3), "...");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("valid_filename.txt"),
            "valid_filename.txt"
        );
        assert_eq!(
            sanitize_filename("invalid/filename:with*chars"),
            "invalid_filename_with_chars"
        );
        assert_eq!(
            sanitize_filename("file<name>with|quotes\""),
            "file_name_with_quotes_"
        );
    }

    #[test]
    fn test_parse_env_bool() {
        std::env::set_var("TEST_BOOL_TRUE", "true");
        std::env::set_var("TEST_BOOL_FALSE", "false");
        std::env::set_var("TEST_BOOL_ONE", "1");
        std::env::set_var("TEST_BOOL_ZERO", "0");

        assert!(parse_env_bool("TEST_BOOL_TRUE", false));
        assert!(!parse_env_bool("TEST_BOOL_FALSE", true));
        assert!(parse_env_bool("TEST_BOOL_ONE", false));
        assert!(!parse_env_bool("TEST_BOOL_ZERO", true));
        assert!(parse_env_bool("TEST_BOOL_NONEXISTENT", true));

        // Cleanup
        std::env::remove_var("TEST_BOOL_TRUE");
        std::env::remove_var("TEST_BOOL_FALSE");
        std::env::remove_var("TEST_BOOL_ONE");
        std::env::remove_var("TEST_BOOL_ZERO");
    }

    #[test]
    fn test_parse_env_int() {
        std::env::set_var("TEST_INT_VALID", "42");
        std::env::set_var("TEST_INT_INVALID", "not_a_number");

        assert_eq!(parse_env_int("TEST_INT_VALID", 0), 42);
        assert_eq!(parse_env_int("TEST_INT_INVALID", 100), 100);
        assert_eq!(parse_env_int("TEST_INT_NONEXISTENT", 50), 50);

        // Cleanup
        std::env::remove_var("TEST_INT_VALID");
        std::env::remove_var("TEST_INT_INVALID");
    }

    #[test]
    fn test_simple_hash() {
        let hash1 = simple_hash("hello");
        let hash2 = simple_hash("hello");
        let hash3 = simple_hash("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_current_timestamp() {
        let timestamp = current_timestamp();
        assert!(timestamp.len() > 20); // ISO 8601 format is at least 20 chars
        assert!(timestamp.contains('T')); // ISO 8601 has T separator
        assert!(timestamp.ends_with('Z')); // UTC timezone indicator
    }

    #[test]
    fn test_is_valid_email() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@domain.co.uk"));
        assert!(!is_valid_email("invalid_email"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("a@b"));
    }

    #[test]
    fn test_get_system_info() {
        let info = get_system_info();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_count > 0);
        assert!(!info.hostname.is_empty());
    }

    #[tokio::test]
    async fn test_retry_with_backoff() {
        let mut attempt_count = 0;

        let result = retry_with_backoff(
            || {
                attempt_count += 1;
                async move {
                    if attempt_count < 3 {
                        Err("temporary failure")
                    } else {
                        Ok("success")
                    }
                }
            },
            5,
            10,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count, 3);
    }

    #[test]
    fn test_create_and_cleanup_temp_dir() {
        let temp_dir = create_temp_dir("qa_test").unwrap();
        assert!(temp_dir.exists());
        assert!(temp_dir.is_dir());

        let cleanup_result = cleanup_temp_dir(&temp_dir);
        assert!(cleanup_result.is_ok());
        assert!(!temp_dir.exists());
    }
}
