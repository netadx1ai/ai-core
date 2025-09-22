//! Utility functions for the AI-CORE Integration Service
//!
//! This module provides common utility functions used across the integration service
//! including string manipulation, JSON processing, validation, and formatting helpers.

use crate::error::{IntegrationError, IntegrationResult};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;
use url::Url;

/// String utility functions
pub struct StringUtils;

impl StringUtils {
    /// Convert snake_case to camelCase
    pub fn snake_to_camel(snake_str: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for c in snake_str.chars() {
            if c == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Convert camelCase to snake_case
    pub fn camel_to_snake(camel_str: &str) -> String {
        let mut result = String::new();

        for (i, c) in camel_str.char_indices() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        }

        result
    }

    /// Truncate string to maximum length with ellipsis
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else if max_len <= 3 {
            "...".to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    /// Sanitize string for use as identifier
    pub fn sanitize_identifier(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_lowercase()
    }

    /// Generate a slug from a string
    pub fn slugify(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Check if string is valid email format
    pub fn is_valid_email(email: &str) -> bool {
        let email_regex =
            regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_regex.is_match(email)
    }

    /// Mask sensitive information in strings
    pub fn mask_sensitive(s: &str, visible_chars: usize) -> String {
        if s.len() <= visible_chars * 2 {
            "*".repeat(s.len().min(8))
        } else {
            let start = &s[..visible_chars];
            let end = &s[s.len() - visible_chars..];
            let masked_length = s.len() - (visible_chars * 2);
            format!("{}{}...{}", start, "*".repeat(masked_length.min(8)), end)
        }
    }
}

/// JSON utility functions
pub struct JsonUtils;

impl JsonUtils {
    /// Flatten nested JSON object with dot notation
    pub fn flatten(value: &Value) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        Self::flatten_recursive(value, String::new(), &mut result);
        result
    }

    fn flatten_recursive(value: &Value, prefix: String, result: &mut HashMap<String, Value>) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten_recursive(val, new_key, result);
                }
            }
            Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_key = format!("{}[{}]", prefix, i);
                    Self::flatten_recursive(val, new_key, result);
                }
            }
            _ => {
                result.insert(prefix, value.clone());
            }
        }
    }

    /// Extract nested value using dot notation
    pub fn get_nested<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)?;
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Set nested value using dot notation
    pub fn set_nested(value: &mut Value, path: &str, new_value: Value) -> IntegrationResult<()> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err(IntegrationError::validation("path", "Path cannot be empty"));
        }

        let mut current = value;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                match current {
                    Value::Object(obj) => {
                        obj.insert(part.to_string(), new_value);
                        return Ok(());
                    }
                    _ => {
                        return Err(IntegrationError::validation(
                            "path",
                            "Cannot set property on non-object",
                        ));
                    }
                }
            } else {
                // Navigate deeper
                match current {
                    Value::Object(obj) => {
                        current = obj
                            .entry(part.to_string())
                            .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    }
                    _ => {
                        return Err(IntegrationError::validation(
                            "path",
                            "Cannot navigate through non-object",
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove empty values from JSON
    pub fn remove_empty(value: &mut Value) {
        match value {
            Value::Object(obj) => {
                obj.retain(|_, v| !Self::is_empty(v));
                for val in obj.values_mut() {
                    Self::remove_empty(val);
                }
            }
            Value::Array(arr) => {
                arr.retain(|v| !Self::is_empty(v));
                for val in arr.iter_mut() {
                    Self::remove_empty(val);
                }
            }
            _ => {}
        }
    }

    fn is_empty(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(s) => s.is_empty(),
            Value::Array(arr) => arr.is_empty(),
            Value::Object(obj) => obj.is_empty(),
            _ => false,
        }
    }

    /// Merge two JSON objects
    pub fn merge(base: &mut Value, other: &Value) {
        match (base.clone(), other) {
            (Value::Object(mut base_obj), Value::Object(other_obj)) => {
                for (key, value) in other_obj {
                    if let Some(base_value) = base_obj.get_mut(key) {
                        Self::merge(base_value, value);
                    } else {
                        base_obj.insert(key.clone(), value.clone());
                    }
                }
                *base = Value::Object(base_obj);
            }
            _ => *base = other.clone(),
        }
    }
}

/// URL utility functions
pub struct UrlUtils;

impl UrlUtils {
    /// Parse and validate URL
    pub fn parse_url(url_str: &str) -> IntegrationResult<Url> {
        Url::parse(url_str)
            .map_err(|e| IntegrationError::validation("url", format!("Invalid URL: {}", e)))
    }

    /// Extract query parameters from URL
    pub fn extract_query_params(url_str: &str) -> IntegrationResult<HashMap<String, String>> {
        let url = Self::parse_url(url_str)?;
        let params: HashMap<String, String> = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Ok(params)
    }

    /// Build URL with query parameters
    pub fn build_url_with_params(
        base_url: &str,
        params: &HashMap<String, String>,
    ) -> IntegrationResult<String> {
        let mut url = Self::parse_url(base_url)?;

        {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in params {
                query_pairs.append_pair(key, value);
            }
        }

        Ok(url.to_string())
    }

    /// Check if URL is HTTPS
    pub fn is_https(url_str: &str) -> IntegrationResult<bool> {
        let url = Self::parse_url(url_str)?;
        Ok(url.scheme() == "https")
    }

    /// Get domain from URL
    pub fn get_domain(url_str: &str) -> IntegrationResult<String> {
        let url = Self::parse_url(url_str)?;
        url.host_str()
            .ok_or_else(|| IntegrationError::validation("url", "URL has no host"))
            .map(|s| s.to_string())
    }
}

/// Date/time utility functions
pub struct DateUtils;

impl DateUtils {
    /// Format datetime for API responses
    pub fn format_api_datetime(dt: &DateTime<Utc>) -> String {
        dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }

    /// Parse various datetime formats
    pub fn parse_flexible_datetime(s: &str) -> IntegrationResult<DateTime<Utc>> {
        // Try different formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.3fZ",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d",
        ];

        for format in &formats {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, format) {
                return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
            }
        }

        // Try parsing as timestamp
        if let Ok(timestamp) = s.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                return Ok(dt);
            }
        }

        Err(IntegrationError::validation(
            "datetime",
            format!("Unable to parse datetime: {}", s),
        ))
    }

    /// Get relative time description
    pub fn relative_time(dt: &DateTime<Utc>) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(*dt);

        if duration.num_seconds() < 60 {
            "just now".to_string()
        } else if duration.num_minutes() < 60 {
            format!("{} minutes ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{} hours ago", duration.num_hours())
        } else if duration.num_days() < 30 {
            format!("{} days ago", duration.num_days())
        } else {
            dt.format("%Y-%m-%d").to_string()
        }
    }

    /// Check if datetime is within last N minutes
    pub fn is_recent(dt: &DateTime<Utc>, minutes: i64) -> bool {
        let now = Utc::now();
        let duration = now.signed_duration_since(*dt);
        duration.num_minutes() <= minutes
    }
}

/// Validation utility functions
pub struct ValidationUtils;

impl ValidationUtils {
    /// Validate webhook URL
    pub fn validate_webhook_url(url: &str) -> IntegrationResult<()> {
        let parsed_url = UrlUtils::parse_url(url)?;

        // Must be HTTPS for security
        if parsed_url.scheme() != "https" {
            return Err(IntegrationError::validation(
                "webhook_url",
                "Webhook URL must use HTTPS",
            ));
        }

        // Must have a host
        if parsed_url.host_str().is_none() {
            return Err(IntegrationError::validation(
                "webhook_url",
                "Webhook URL must have a valid host",
            ));
        }

        // Check for localhost/private IPs in production
        if let Some(host) = parsed_url.host_str() {
            if host == "localhost" || host == "127.0.0.1" {
                debug!("Warning: Webhook URL points to localhost");
            }
        }

        Ok(())
    }

    /// Validate event name
    pub fn validate_event_name(name: &str) -> IntegrationResult<()> {
        if name.is_empty() {
            return Err(IntegrationError::validation(
                "event_name",
                "Event name cannot be empty",
            ));
        }

        if name.len() > 100 {
            return Err(IntegrationError::validation(
                "event_name",
                "Event name too long (max 100 characters)",
            ));
        }

        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(IntegrationError::validation(
                "event_name",
                "Event name contains invalid characters",
            ));
        }

        Ok(())
    }

    /// Validate integration name
    pub fn validate_integration_name(name: &str) -> IntegrationResult<()> {
        let valid_names = ["zapier", "slack", "github"];

        if !valid_names.contains(&name) {
            return Err(IntegrationError::validation(
                "integration",
                format!("Unknown integration: {}", name),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_snake_to_camel() {
        assert_eq!(StringUtils::snake_to_camel("hello_world"), "helloWorld");
        assert_eq!(
            StringUtils::snake_to_camel("test_case_name"),
            "testCaseName"
        );
        assert_eq!(StringUtils::snake_to_camel("simple"), "simple");
        assert_eq!(StringUtils::snake_to_camel(""), "");
    }

    #[test]
    fn test_camel_to_snake() {
        assert_eq!(StringUtils::camel_to_snake("helloWorld"), "hello_world");
        assert_eq!(
            StringUtils::camel_to_snake("testCaseName"),
            "test_case_name"
        );
        assert_eq!(StringUtils::camel_to_snake("simple"), "simple");
        assert_eq!(StringUtils::camel_to_snake(""), "");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(StringUtils::truncate("hello", 10), "hello");
        assert_eq!(StringUtils::truncate("hello world", 8), "hello...");
        assert_eq!(StringUtils::truncate("hi", 2), "hi");
        assert_eq!(StringUtils::truncate("test", 3), "...");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(StringUtils::slugify("Hello World"), "hello-world");
        assert_eq!(StringUtils::slugify("Test@#$%Case"), "test-case");
        assert_eq!(StringUtils::slugify("multiple---dashes"), "multiple-dashes");
    }

    #[test]
    fn test_is_valid_email() {
        assert!(StringUtils::is_valid_email("test@example.com"));
        assert!(StringUtils::is_valid_email("user+tag@domain.co.uk"));
        assert!(!StringUtils::is_valid_email("invalid-email"));
        assert!(!StringUtils::is_valid_email("@example.com"));
        assert!(!StringUtils::is_valid_email("test@"));
    }

    #[test]
    fn test_mask_sensitive() {
        assert_eq!(
            StringUtils::mask_sensitive("password123", 2),
            "pa********...23"
        );
        assert_eq!(StringUtils::mask_sensitive("short", 1), "********");
        assert_eq!(StringUtils::mask_sensitive("a", 1), "*");
    }

    #[test]
    fn test_json_flatten() {
        let json = json!({
            "user": {
                "name": "John",
                "details": {
                    "age": 30,
                    "city": "NYC"
                }
            },
            "items": [1, 2, 3]
        });

        let flattened = JsonUtils::flatten(&json);
        assert_eq!(flattened.get("user.name"), Some(&json!("John")));
        assert_eq!(flattened.get("user.details.age"), Some(&json!(30)));
        assert_eq!(flattened.get("items[0]"), Some(&json!(1)));
    }

    #[test]
    fn test_json_get_nested() {
        let json = json!({
            "user": {
                "profile": {
                    "name": "John"
                }
            }
        });

        assert_eq!(
            JsonUtils::get_nested(&json, "user.profile.name"),
            Some(&json!("John"))
        );
        assert_eq!(JsonUtils::get_nested(&json, "user.nonexistent"), None);
    }

    #[test]
    fn test_json_set_nested() {
        let mut json = json!({});
        JsonUtils::set_nested(&mut json, "user.name", json!("John")).unwrap();
        assert_eq!(json["user"]["name"], "John");
    }

    #[test]
    fn test_json_remove_empty() {
        let mut json = json!({
            "name": "John",
            "empty_string": "",
            "null_value": null,
            "empty_array": [],
            "empty_object": {},
            "valid_array": [1, 2],
            "nested": {
                "empty": "",
                "valid": "data"
            }
        });

        JsonUtils::remove_empty(&mut json);
        assert!(!json.as_object().unwrap().contains_key("empty_string"));
        assert!(!json.as_object().unwrap().contains_key("null_value"));
        assert!(!json.as_object().unwrap().contains_key("empty_array"));
        assert!(json.as_object().unwrap().contains_key("name"));
        assert!(json.as_object().unwrap().contains_key("valid_array"));
    }

    #[test]
    fn test_url_parse() {
        assert!(UrlUtils::parse_url("https://example.com").is_ok());
        assert!(UrlUtils::parse_url("invalid-url").is_err());
    }

    #[test]
    fn test_url_extract_query_params() {
        let params = UrlUtils::extract_query_params("https://example.com?a=1&b=2").unwrap();
        assert_eq!(params.get("a"), Some(&"1".to_string()));
        assert_eq!(params.get("b"), Some(&"2".to_string()));
    }

    #[test]
    fn test_url_is_https() {
        assert!(UrlUtils::is_https("https://example.com").unwrap());
        assert!(!UrlUtils::is_https("http://example.com").unwrap());
    }

    #[test]
    fn test_date_format_api_datetime() {
        let dt = DateTime::from_timestamp(1609459200, 0).unwrap(); // 2021-01-01 00:00:00 UTC
        let formatted = DateUtils::format_api_datetime(&dt);
        assert!(formatted.starts_with("2021-01-01T00:00:00"));
        assert!(formatted.ends_with("Z"));
    }

    #[test]
    fn test_date_is_recent() {
        let now = Utc::now();
        let five_min_ago = now - chrono::Duration::minutes(5);
        let hour_ago = now - chrono::Duration::hours(1);

        assert!(DateUtils::is_recent(&five_min_ago, 10));
        assert!(!DateUtils::is_recent(&hour_ago, 10));
    }

    #[test]
    fn test_validate_webhook_url() {
        assert!(ValidationUtils::validate_webhook_url("https://example.com/webhook").is_ok());
        assert!(ValidationUtils::validate_webhook_url("http://example.com/webhook").is_err());
        assert!(ValidationUtils::validate_webhook_url("invalid-url").is_err());
    }

    #[test]
    fn test_validate_event_name() {
        assert!(ValidationUtils::validate_event_name("user_created").is_ok());
        assert!(ValidationUtils::validate_event_name("order.completed").is_ok());
        assert!(ValidationUtils::validate_event_name("").is_err());
        assert!(ValidationUtils::validate_event_name(&"x".repeat(101)).is_err());
        assert!(ValidationUtils::validate_event_name("invalid@event").is_err());
    }

    #[test]
    fn test_validate_integration_name() {
        assert!(ValidationUtils::validate_integration_name("zapier").is_ok());
        assert!(ValidationUtils::validate_integration_name("slack").is_ok());
        assert!(ValidationUtils::validate_integration_name("github").is_ok());
        assert!(ValidationUtils::validate_integration_name("unknown").is_err());
    }
}
