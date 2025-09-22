//! Utility Functions Module
//!
//! This module provides common utility functions used throughout the MCP Manager Service,
//! including validation helpers, conversion utilities, and other shared functionality.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

/// Validation utilities
pub mod validation {
    use regex::Regex;
    use std::sync::OnceLock;

    /// Validate server name format
    pub fn is_valid_server_name(name: &str) -> bool {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = REGEX
            .get_or_init(|| Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9\-_]{0,62}[a-zA-Z0-9]$").unwrap());

        if name.len() < 2 || name.len() > 64 {
            return false;
        }

        regex.is_match(name)
    }

    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }

    /// Validate UUID format
    pub fn is_valid_uuid(uuid_str: &str) -> bool {
        uuid::Uuid::parse_str(uuid_str).is_ok()
    }

    /// Validate server type
    pub fn is_valid_server_type(server_type: &str) -> bool {
        let valid_types = [
            "filesystem",
            "database",
            "api",
            "tool",
            "resource",
            "prompt",
            "completion",
            "custom",
        ];

        valid_types.contains(&server_type)
    }

    /// Validate tag format
    pub fn is_valid_tag(tag: &str) -> bool {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = REGEX
            .get_or_init(|| Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9\-_]{0,30}[a-zA-Z0-9]$").unwrap());

        if tag.len() < 2 || tag.len() > 32 {
            return false;
        }

        regex.is_match(tag)
    }
}

/// Conversion utilities
pub mod conversion {
    use super::*;

    /// Convert timestamp to ISO 8601 string
    pub fn timestamp_to_iso8601(timestamp: DateTime<Utc>) -> String {
        timestamp.to_rfc3339()
    }

    /// Parse ISO 8601 string to timestamp
    pub fn iso8601_to_timestamp(iso_string: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        DateTime::parse_from_rfc3339(iso_string).map(|dt| dt.with_timezone(&Utc))
    }

    /// Convert HashMap to JSON Value
    pub fn hashmap_to_json(map: HashMap<String, String>) -> Value {
        let json_map: serde_json::Map<String, Value> = map
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        Value::Object(json_map)
    }

    /// Convert JSON Value to HashMap
    pub fn json_to_hashmap(value: &Value) -> HashMap<String, String> {
        match value {
            Value::Object(map) => map
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect(),
            _ => HashMap::new(),
        }
    }

    /// Convert duration to human readable string
    pub fn duration_to_human(duration_ms: u64) -> String {
        if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else if duration_ms < 60_000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else if duration_ms < 3_600_000 {
            format!("{:.1}m", duration_ms as f64 / 60_000.0)
        } else {
            format!("{:.1}h", duration_ms as f64 / 3_600_000.0)
        }
    }

    /// Convert bytes to human readable size
    pub fn bytes_to_human(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: f64 = 1024.0;

        if bytes == 0 {
            return "0 B".to_string();
        }

        let bytes_f = bytes as f64;
        let i = (bytes_f.log10() / THRESHOLD.log10()).floor() as usize;
        let i = i.min(UNITS.len() - 1);

        let size = bytes_f / THRESHOLD.powi(i as i32);

        if i == 0 {
            format!("{} {}", bytes, UNITS[i])
        } else {
            format!("{:.1} {}", size, UNITS[i])
        }
    }
}

/// String utilities
pub mod string {
    /// Truncate string to maximum length
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    /// Sanitize string for safe usage
    pub fn sanitize(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "-_.".contains(*c))
            .collect()
    }

    /// Convert string to snake_case
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_was_upper = false;

        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 && !prev_was_upper {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
                prev_was_upper = true;
            } else {
                result.push(c);
                prev_was_upper = false;
            }
        }

        result
    }

    /// Convert string to kebab-case
    pub fn to_kebab_case(s: &str) -> String {
        to_snake_case(s).replace('_', "-")
    }

    /// Check if string is empty or only whitespace
    pub fn is_blank(s: &str) -> bool {
        s.trim().is_empty()
    }
}

/// Collection utilities
pub mod collections {
    use std::collections::HashMap;

    /// Merge two HashMaps, with values from the second map taking precedence
    pub fn merge_hashmaps<K, V>(mut first: HashMap<K, V>, second: HashMap<K, V>) -> HashMap<K, V>
    where
        K: Eq + std::hash::Hash,
    {
        for (key, value) in second {
            first.insert(key, value);
        }
        first
    }

    /// Filter HashMap by predicate
    pub fn filter_hashmap<K, V, F>(map: HashMap<K, V>, predicate: F) -> HashMap<K, V>
    where
        K: Eq + std::hash::Hash,
        F: Fn(&K, &V) -> bool,
    {
        map.into_iter().filter(|(k, v)| predicate(k, v)).collect()
    }

    /// Group vector items by key function
    pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> HashMap<K, Vec<T>>
    where
        K: Eq + std::hash::Hash,
        F: Fn(&T) -> K,
    {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();

        for item in items {
            let key = key_fn(&item);
            groups.entry(key).or_default().push(item);
        }

        groups
    }
}

/// Error utilities
pub mod error {
    use crate::McpError;

    /// Convert any error to McpError
    pub fn to_mcp_error<E: std::fmt::Display>(error: E, context: &str) -> McpError {
        McpError::Internal(format!("{}: {}", context, error))
    }

    /// Chain multiple errors into a single message
    pub fn chain_errors(errors: Vec<String>) -> String {
        if errors.is_empty() {
            "No errors".to_string()
        } else if errors.len() == 1 {
            errors.into_iter().next().unwrap()
        } else {
            format!("Multiple errors: {}", errors.join("; "))
        }
    }

    /// Create a validation error message
    pub fn validation_error(field: &str, reason: &str) -> String {
        format!("Validation failed for field '{}': {}", field, reason)
    }
}

/// Network utilities
pub mod network {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    /// Check if IP address is private
    pub fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
            IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
        }
    }

    /// Check if IPv4 address is private
    fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
        ip.is_private() || ip.is_loopback() || ip.is_link_local()
    }

    /// Check if IPv6 address is private
    fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
        ip.is_loopback() || ip.is_unicast_link_local() || is_ipv6_unique_local(ip)
    }

    /// Check if IPv6 address is unique local (fc00::/7)
    fn is_ipv6_unique_local(ip: &Ipv6Addr) -> bool {
        let octets = ip.octets();
        (octets[0] & 0xfe) == 0xfc
    }

    /// Parse host:port string
    pub fn parse_host_port(addr: &str) -> Result<(String, u16), String> {
        if let Some(colon_pos) = addr.rfind(':') {
            let host = &addr[..colon_pos];
            let port_str = &addr[colon_pos + 1..];

            match port_str.parse::<u16>() {
                Ok(port) => Ok((host.to_string(), port)),
                Err(_) => Err(format!("Invalid port number: {}", port_str)),
            }
        } else {
            Err("No port specified".to_string())
        }
    }
}

/// Configuration utilities
pub mod config {
    use std::env;

    /// Get environment variable with default value
    pub fn env_var_or_default(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get environment variable as integer with default
    pub fn env_var_as_int_or_default(key: &str, default: i32) -> i32 {
        env::var(key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Get environment variable as boolean with default
    pub fn env_var_as_bool_or_default(key: &str, default: bool) -> bool {
        env::var(key)
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
            .unwrap_or(default)
    }
}

/// Retry utilities
pub mod retry {
    use std::time::Duration;
    use tokio::time::sleep;

    /// Retry configuration
    #[derive(Debug, Clone)]
    pub struct RetryConfig {
        pub max_attempts: u32,
        pub initial_delay: Duration,
        pub max_delay: Duration,
        pub backoff_multiplier: f64,
    }

    impl Default for RetryConfig {
        fn default() -> Self {
            Self {
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(10),
                backoff_multiplier: 2.0,
            }
        }
    }

    /// Retry an async operation with exponential backoff
    pub async fn retry_with_backoff<F, T, E>(operation: F, config: RetryConfig) -> Result<T, E>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>> + Send,
        E: std::fmt::Debug,
    {
        let mut delay = config.initial_delay;
        let mut last_error = None;

        for attempt in 1..=config.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);

                    if attempt < config.max_attempts {
                        sleep(delay).await;
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64 * config.backoff_multiplier) as u64,
                            ),
                            config.max_delay,
                        );
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validation_tests {
        use super::validation::*;

        #[test]
        fn test_server_name_validation() {
            assert!(is_valid_server_name("my-server"));
            assert!(is_valid_server_name("server123"));
            assert!(is_valid_server_name("my_server_01"));

            assert!(!is_valid_server_name(""));
            assert!(!is_valid_server_name("a"));
            assert!(!is_valid_server_name("-invalid"));
            assert!(!is_valid_server_name("invalid-"));
            assert!(!is_valid_server_name("server with spaces"));
        }

        #[test]
        fn test_url_validation() {
            assert!(is_valid_url("http://example.com"));
            assert!(is_valid_url("https://example.com/path"));
            assert!(is_valid_url("ws://localhost:9000"));

            assert!(!is_valid_url("not-a-url"));
            // localhost:8080 is actually a valid URL - it's parsed as scheme:path
            // where scheme="localhost" and path="8080"
            // Let's test with a truly invalid URL instead
            assert!(!is_valid_url("://invalid-url"));
        }

        #[test]
        fn test_uuid_validation() {
            assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
            assert!(!is_valid_uuid("not-a-uuid"));
            assert!(!is_valid_uuid("550e8400-e29b-41d4-a716"));
        }

        #[test]
        fn test_server_type_validation() {
            assert!(is_valid_server_type("filesystem"));
            assert!(is_valid_server_type("database"));
            assert!(is_valid_server_type("api"));

            assert!(!is_valid_server_type("invalid"));
            assert!(!is_valid_server_type(""));
        }
    }

    mod conversion_tests {
        use super::conversion::*;
        use chrono::Utc;

        #[test]
        fn test_duration_to_human() {
            assert_eq!(duration_to_human(500), "500ms");
            assert_eq!(duration_to_human(1500), "1.5s");
            assert_eq!(duration_to_human(90000), "1.5m");
            assert_eq!(duration_to_human(7200000), "2.0h");
        }

        #[test]
        fn test_bytes_to_human() {
            assert_eq!(bytes_to_human(0), "0 B");
            assert_eq!(bytes_to_human(512), "512 B");
            assert_eq!(bytes_to_human(1536), "1.5 KB");
            assert_eq!(bytes_to_human(2097152), "2.0 MB");
        }

        #[test]
        fn test_timestamp_conversion() {
            let now = Utc::now();
            let iso_string = timestamp_to_iso8601(now);
            let parsed = iso8601_to_timestamp(&iso_string).unwrap();

            // Allow for small differences due to precision
            assert!((now.timestamp_millis() - parsed.timestamp_millis()).abs() < 1000);
        }
    }

    mod string_tests {
        use super::string::*;

        #[test]
        fn test_truncate() {
            assert_eq!(truncate("hello", 10), "hello");
            assert_eq!(truncate("hello world", 8), "hello...");
            assert_eq!(truncate("hi", 5), "hi");
        }

        #[test]
        fn test_snake_case() {
            assert_eq!(to_snake_case("CamelCase"), "camel_case");
            assert_eq!(to_snake_case("XMLHttpRequest"), "xmlhttp_request");
            assert_eq!(to_snake_case("already_snake"), "already_snake");
        }

        #[test]
        fn test_kebab_case() {
            assert_eq!(to_kebab_case("CamelCase"), "camel-case");
            assert_eq!(to_kebab_case("snake_case"), "snake-case");
        }

        #[test]
        fn test_is_blank() {
            assert!(is_blank(""));
            assert!(is_blank("   "));
            assert!(is_blank("\t\n"));
            assert!(!is_blank("hello"));
            assert!(!is_blank(" hello "));
        }
    }

    mod network_tests {
        use super::network::*;
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

        #[test]
        fn test_private_ip_detection() {
            // Private IPv4 addresses
            assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
            assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
            assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

            // Public IPv4 addresses
            assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
            assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        }

        #[test]
        fn test_parse_host_port() {
            assert_eq!(
                parse_host_port("localhost:8080"),
                Ok(("localhost".to_string(), 8080))
            );
            assert_eq!(
                parse_host_port("192.168.1.1:3000"),
                Ok(("192.168.1.1".to_string(), 3000))
            );

            assert!(parse_host_port("localhost").is_err());
            assert!(parse_host_port("localhost:abc").is_err());
        }
    }
}
