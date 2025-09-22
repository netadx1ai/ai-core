//! Utility functions and helpers for the Federation Service
//!
//! This module provides common utilities, database helpers, cache management,
//! validation functions, and other shared functionality used throughout the
//! federation service.

use crate::models::FederationError;
use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::Client as RedisClient;
use serde_json;
use sqlx::PgPool;
use tracing::debug;
use uuid::Uuid;

// ================================================================================================
// Database Utilities
// ================================================================================================

/// Database manager for handling database operations
#[derive(Debug, Clone)]
pub struct DatabaseManager {
    pool: std::sync::Arc<PgPool>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new(pool: std::sync::Arc<PgPool>) -> Result<Self, FederationError> {
        Ok(Self { pool })
    }

    /// Create a client in the database
    pub async fn create_client(
        &self,
        client: &crate::models::Client,
    ) -> Result<(), FederationError> {
        // This would implement actual database insertion
        debug!("Creating client in database: {}", client.id);
        Ok(())
    }

    /// Get client by ID from database
    pub async fn get_client(
        &self,
        client_id: &Uuid,
    ) -> Result<Option<crate::models::Client>, FederationError> {
        debug!("Getting client from database: {}", client_id);
        // This would implement actual database query
        Ok(None)
    }

    /// Get client by API key from database
    pub async fn get_client_by_api_key(
        &self,
        api_key: &str,
    ) -> Result<Option<crate::models::Client>, FederationError> {
        debug!("Getting client by API key from database");
        // This would implement actual database query
        Ok(None)
    }

    /// Update client in database
    pub async fn update_client(
        &self,
        client: &crate::models::Client,
    ) -> Result<(), FederationError> {
        debug!("Updating client in database: {}", client.id);
        Ok(())
    }

    /// Delete client from database
    pub async fn delete_client(&self, client_id: &Uuid) -> Result<(), FederationError> {
        debug!("Deleting client from database: {}", client_id);
        Ok(())
    }

    /// Check if client exists by name
    pub async fn client_exists_by_name(&self, name: &str) -> Result<bool, FederationError> {
        debug!("Checking if client exists by name: {}", name);
        Ok(false)
    }

    /// List clients with filtering
    pub async fn list_clients(
        &self,
        filter: &crate::client::ClientFilter,
    ) -> Result<Vec<crate::models::Client>, FederationError> {
        debug!("Listing clients with filter");
        Ok(Vec::new())
    }

    /// Count clients with filtering
    pub async fn count_clients(
        &self,
        filter: &crate::client::ClientFilter,
    ) -> Result<u64, FederationError> {
        debug!("Counting clients with filter");
        Ok(0)
    }

    /// List all clients
    pub async fn list_all_clients(&self) -> Result<Vec<crate::models::Client>, FederationError> {
        debug!("Listing all clients");
        Ok(Vec::new())
    }

    /// Create provider in database
    pub async fn create_provider(
        &self,
        provider: &crate::models::Provider,
    ) -> Result<(), FederationError> {
        debug!("Creating provider in database: {}", provider.id);
        Ok(())
    }

    /// Get provider by ID from database
    pub async fn get_provider(
        &self,
        provider_id: &Uuid,
    ) -> Result<Option<crate::models::Provider>, FederationError> {
        debug!("Getting provider from database: {}", provider_id);
        Ok(None)
    }

    /// Update provider in database
    pub async fn update_provider(
        &self,
        provider: &crate::models::Provider,
    ) -> Result<(), FederationError> {
        debug!("Updating provider in database: {}", provider.id);
        Ok(())
    }

    /// Delete provider from database
    pub async fn delete_provider(&self, provider_id: &Uuid) -> Result<(), FederationError> {
        debug!("Deleting provider from database: {}", provider_id);
        Ok(())
    }

    /// Check if provider exists by name
    pub async fn provider_exists_by_name(&self, name: &str) -> Result<bool, FederationError> {
        debug!("Checking if provider exists by name: {}", name);
        Ok(false)
    }

    /// List all providers
    pub async fn list_all_providers(
        &self,
    ) -> Result<Vec<crate::models::Provider>, FederationError> {
        debug!("Listing all providers");
        Ok(Vec::new())
    }
}

// ================================================================================================
// Cache Management
// ================================================================================================

/// Cache manager for Redis operations
#[derive(Debug, Clone)]
pub struct CacheManager {
    client: std::sync::Arc<RedisClient>,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new(client: std::sync::Arc<RedisClient>) -> Result<Self, FederationError> {
        Ok(Self { client })
    }

    /// Cache a client
    pub async fn cache_client(
        &self,
        client: &crate::models::Client,
    ) -> Result<(), FederationError> {
        debug!("Caching client: {}", client.id);
        Ok(())
    }

    /// Get client from cache
    pub async fn get_client(
        &self,
        client_id: &Uuid,
    ) -> Result<Option<crate::models::Client>, FederationError> {
        debug!("Getting client from cache: {}", client_id);
        Ok(None)
    }

    /// Get client by API key hash
    pub async fn get_client_by_api_key_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<Uuid>, FederationError> {
        debug!("Getting client by API key hash from cache");
        Ok(None)
    }

    /// Remove client from cache
    pub async fn remove_client(&self, client_id: &Uuid) -> Result<(), FederationError> {
        debug!("Removing client from cache: {}", client_id);
        Ok(())
    }

    /// Cache a provider
    pub async fn cache_provider(
        &self,
        provider: &crate::models::Provider,
    ) -> Result<(), FederationError> {
        debug!("Caching provider: {}", provider.id);
        Ok(())
    }

    /// Get provider from cache
    pub async fn get_provider(
        &self,
        provider_id: &Uuid,
    ) -> Result<Option<crate::models::Provider>, FederationError> {
        debug!("Getting provider from cache: {}", provider_id);
        Ok(None)
    }

    /// Remove provider from cache
    pub async fn remove_provider(&self, provider_id: &Uuid) -> Result<(), FederationError> {
        debug!("Removing provider from cache: {}", provider_id);
        Ok(())
    }
}

// ================================================================================================
// Database Connection Utilities
// ================================================================================================

pub mod database {
    pub use super::DatabaseManager;
    use super::*;
    use crate::config::DatabaseConfig;

    /// Create a database connection pool
    pub async fn create_connection_pool(
        config: &DatabaseConfig,
    ) -> Result<PgPool, FederationError> {
        debug!("Creating database connection pool");

        let pool =
            PgPool::connect(&config.url)
                .await
                .map_err(|e| FederationError::DatabaseError {
                    message: format!("Failed to connect to database: {}", e),
                })?;

        debug!("Database connection pool created successfully");
        Ok(pool)
    }

    /// Test database connection
    pub async fn test_connection(pool: &PgPool) -> Result<(), FederationError> {
        sqlx::query("SELECT 1").execute(pool).await.map_err(|e| {
            FederationError::DatabaseError {
                message: format!("Database connection test failed: {}", e),
            }
        })?;

        Ok(())
    }
}

// ================================================================================================
// Cache Connection Utilities
// ================================================================================================

pub mod cache {
    pub use super::CacheManager;
    use super::*;
    use crate::config::RedisConfig;

    /// Create a Redis client
    pub async fn create_redis_client(config: &RedisConfig) -> Result<RedisClient, FederationError> {
        debug!("Creating Redis client");

        let client =
            RedisClient::open(config.url.as_str()).map_err(|e| FederationError::CacheError {
                message: format!("Failed to create Redis client: {}", e),
            })?;

        debug!("Redis client created successfully");
        Ok(client)
    }

    /// Test Redis connection
    pub async fn test_connection(client: &RedisClient) -> Result<(), FederationError> {
        let mut conn =
            client
                .get_async_connection()
                .await
                .map_err(|e| FederationError::CacheError {
                    message: format!("Failed to connect to Redis: {}", e),
                })?;

        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| FederationError::CacheError {
                message: format!("Redis ping failed: {}", e),
            })?;

        Ok(())
    }
}

// ================================================================================================
// Validation Utilities
// ================================================================================================

pub mod validation {
    use super::*;
    use regex::Regex;
    use std::collections::HashSet;
    use url::Url;

    /// Validate email address
    pub fn is_valid_email(email: &str) -> bool {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_regex.is_match(email)
    }

    /// Validate URL
    pub fn is_valid_url(url: &str) -> bool {
        Url::parse(url).is_ok()
    }

    /// Validate UUID
    pub fn is_valid_uuid(uuid_str: &str) -> bool {
        Uuid::parse_str(uuid_str).is_ok()
    }

    /// Validate client name
    pub fn is_valid_client_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Client name cannot be empty".to_string());
        }

        if name.len() > 100 {
            return Err("Client name cannot be longer than 100 characters".to_string());
        }

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || " -_".contains(c))
        {
            return Err("Client name can only contain alphanumeric characters, spaces, hyphens, and underscores".to_string());
        }

        Ok(())
    }

    /// Validate provider name
    pub fn is_valid_provider_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }

        if name.len() > 100 {
            return Err("Provider name cannot be longer than 100 characters".to_string());
        }

        Ok(())
    }

    /// Validate API key format
    pub fn is_valid_api_key(api_key: &str) -> bool {
        api_key.starts_with("fed_") && api_key.len() == 68
    }

    /// Validate capabilities list
    pub fn validate_capabilities(capabilities: &[String]) -> Result<(), String> {
        if capabilities.is_empty() {
            return Err("At least one capability must be specified".to_string());
        }

        let mut seen = HashSet::new();
        for capability in capabilities {
            if capability.is_empty() {
                return Err("Capability cannot be empty".to_string());
            }

            if !seen.insert(capability) {
                return Err(format!("Duplicate capability: {}", capability));
            }
        }

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_email_validation() {
            assert!(is_valid_email("test@example.com"));
            assert!(is_valid_email("user.name+tag@domain.co.uk"));
            assert!(!is_valid_email("invalid-email"));
            assert!(!is_valid_email("@example.com"));
            assert!(!is_valid_email("test@"));
        }

        #[test]
        fn test_url_validation() {
            assert!(is_valid_url("https://example.com"));
            assert!(is_valid_url("http://localhost:8080"));
            assert!(is_valid_url("ftp://files.example.com"));
            assert!(!is_valid_url("not-a-url"));
            assert!(!is_valid_url(""));
        }

        #[test]
        fn test_uuid_validation() {
            assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
            assert!(!is_valid_uuid("invalid-uuid"));
            assert!(!is_valid_uuid(""));
        }

        #[test]
        fn test_client_name_validation() {
            assert!(is_valid_client_name("Valid Client").is_ok());
            assert!(is_valid_client_name("Client-123").is_ok());
            assert!(is_valid_client_name("Client_Name").is_ok());

            assert!(is_valid_client_name("").is_err());
            assert!(is_valid_client_name(&"a".repeat(101)).is_err());
            assert!(is_valid_client_name("Client@Name").is_err());
        }

        #[test]
        fn test_api_key_validation() {
            assert!(is_valid_api_key(
                "fed_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ));
            assert!(!is_valid_api_key("invalid_key"));
            assert!(!is_valid_api_key("fed_short"));
            assert!(!is_valid_api_key(
                "wrong_prefix_1234567890abcdef1234567890abcdef1234567890abcdef123456"
            ));
        }

        #[test]
        fn test_capabilities_validation() {
            assert!(validate_capabilities(&["llm".to_string(), "storage".to_string()]).is_ok());
            assert!(validate_capabilities(&[]).is_err());
            assert!(validate_capabilities(&["".to_string()]).is_err());
            assert!(validate_capabilities(&["llm".to_string(), "llm".to_string()]).is_err());
        }
    }
}

// ================================================================================================
// String Utilities
// ================================================================================================

pub mod string {

    /// Convert string to kebab-case
    pub fn to_kebab_case(input: &str) -> String {
        input
            .chars()
            .map(|c| {
                if c.is_uppercase() {
                    format!("-{}", c.to_lowercase())
                } else if c.is_whitespace() {
                    "-".to_string()
                } else {
                    c.to_string()
                }
            })
            .collect::<String>()
            .trim_start_matches('-')
            .to_string()
    }

    /// Convert string to snake_case
    pub fn to_snake_case(input: &str) -> String {
        input
            .chars()
            .map(|c| {
                if c.is_uppercase() {
                    format!("_{}", c.to_lowercase())
                } else if c.is_whitespace() {
                    "_".to_string()
                } else {
                    c.to_string()
                }
            })
            .collect::<String>()
            .trim_start_matches('_')
            .to_string()
    }

    /// Truncate string to specified length
    pub fn truncate(input: &str, max_length: usize) -> String {
        if input.len() <= max_length {
            input.to_string()
        } else {
            format!("{}...", &input[..max_length.saturating_sub(3)])
        }
    }

    /// Check if string is blank (empty or only whitespace)
    pub fn is_blank(input: &str) -> bool {
        input.trim().is_empty()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_to_kebab_case() {
            assert_eq!(to_kebab_case("CamelCase"), "camel-case");
            assert_eq!(to_kebab_case("Already kebab-case"), "already-kebab-case");
            assert_eq!(to_kebab_case("With Spaces"), "with-spaces");
            assert_eq!(to_kebab_case("lowercase"), "lowercase");
        }

        #[test]
        fn test_to_snake_case() {
            assert_eq!(to_snake_case("CamelCase"), "camel_case");
            assert_eq!(to_snake_case("Already snake_case"), "already_snake_case");
            assert_eq!(to_snake_case("With Spaces"), "with_spaces");
            assert_eq!(to_snake_case("lowercase"), "lowercase");
        }

        #[test]
        fn test_truncate() {
            assert_eq!(truncate("Short", 10), "Short");
            assert_eq!(truncate("This is a long string", 10), "This is...");
            assert_eq!(truncate("Exactly10!", 10), "Exactly10!");
            assert_eq!(truncate("", 5), "");
        }

        #[test]
        fn test_is_blank() {
            assert!(is_blank(""));
            assert!(is_blank("   "));
            assert!(is_blank("\t\n"));
            assert!(!is_blank("text"));
            assert!(!is_blank(" text "));
        }
    }
}

// ================================================================================================
// Time Utilities
// ================================================================================================

pub mod time {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Get current Unix timestamp
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Convert DateTime to Unix timestamp
    pub fn datetime_to_timestamp(dt: DateTime<Utc>) -> u64 {
        dt.timestamp() as u64
    }

    /// Convert Unix timestamp to DateTime
    pub fn timestamp_to_datetime(timestamp: u64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(|| Utc::now())
    }

    /// Format duration in human-readable format
    pub fn format_duration(seconds: u64) -> String {
        match seconds {
            s if s < 60 => format!("{}s", s),
            s if s < 3600 => format!("{}m {}s", s / 60, s % 60),
            s if s < 86400 => format!("{}h {}m", s / 3600, (s % 3600) / 60),
            s => format!("{}d {}h", s / 86400, (s % 86400) / 3600),
        }
    }

    /// Get time ago string
    pub fn time_ago(dt: DateTime<Utc>) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(dt);

        if let Ok(std_duration) = duration.to_std() {
            let seconds = std_duration.as_secs();
            match seconds {
                s if s < 60 => "just now".to_string(),
                s if s < 3600 => format!("{} minutes ago", s / 60),
                s if s < 86400 => format!("{} hours ago", s / 3600),
                s if s < 2592000 => format!("{} days ago", s / 86400),
                s => format!("{} months ago", s / 2592000),
            }
        } else {
            "unknown".to_string()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::Duration;

        #[test]
        fn test_current_timestamp() {
            let timestamp = current_timestamp();
            assert!(timestamp > 1_600_000_000); // Should be after 2020
        }

        #[test]
        fn test_datetime_conversion() {
            let now = Utc::now();
            let timestamp = datetime_to_timestamp(now);
            let converted_back = timestamp_to_datetime(timestamp);

            // Allow for small differences due to precision
            assert!((now.timestamp() - converted_back.timestamp()).abs() <= 1);
        }

        #[test]
        fn test_format_duration() {
            assert_eq!(format_duration(30), "30s");
            assert_eq!(format_duration(90), "1m 30s");
            assert_eq!(format_duration(3661), "1h 1m");
            assert_eq!(format_duration(90061), "1d 1h");
        }

        #[test]
        fn test_time_ago() {
            let now = Utc::now();

            assert_eq!(time_ago(now - Duration::seconds(30)), "just now");
            assert_eq!(time_ago(now - Duration::minutes(5)), "5 minutes ago");
            assert_eq!(time_ago(now - Duration::hours(2)), "2 hours ago");
            assert_eq!(time_ago(now - Duration::days(3)), "3 days ago");
        }
    }
}

// ================================================================================================
// Encoding Utilities
// ================================================================================================

pub mod encoding {
    use super::*;
    use base64::prelude::*;

    /// Encode bytes to base64
    pub fn encode_base64(data: &[u8]) -> String {
        BASE64_STANDARD.encode(data)
    }

    /// Decode base64 to bytes
    pub fn decode_base64(encoded: &str) -> Result<Vec<u8>, String> {
        BASE64_STANDARD.decode(encoded).map_err(|e| e.to_string())
    }

    /// Encode string to hex
    pub fn encode_hex(data: &[u8]) -> String {
        hex::encode(data)
    }

    /// Decode hex to bytes
    pub fn decode_hex(encoded: &str) -> Result<Vec<u8>, String> {
        hex::decode(encoded).map_err(|e| e.to_string())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_base64_encoding() {
            let data = b"Hello, World!";
            let encoded = encode_base64(data);
            let decoded = decode_base64(&encoded).unwrap();
            assert_eq!(data.to_vec(), decoded);
        }

        #[test]
        fn test_hex_encoding() {
            let data = b"Hello, World!";
            let encoded = encode_hex(data);
            let decoded = decode_hex(&encoded).unwrap();
            assert_eq!(data.to_vec(), decoded);
        }
    }
}

// ================================================================================================
// Error Utilities
// ================================================================================================

pub mod error {
    use super::*;

    /// Create a validation error
    pub fn validation_error(field: &str, message: &str) -> FederationError {
        FederationError::ValidationError {
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a configuration error
    pub fn config_error(message: &str) -> FederationError {
        FederationError::ConfigurationError {
            message: message.to_string(),
        }
    }

    /// Create a database error
    pub fn database_error(message: &str) -> FederationError {
        FederationError::DatabaseError {
            message: message.to_string(),
        }
    }

    /// Create a cache error
    pub fn cache_error(message: &str) -> FederationError {
        FederationError::CacheError {
            message: message.to_string(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: &str) -> FederationError {
        FederationError::InternalError {
            message: message.to_string(),
        }
    }
}

// ================================================================================================
// JSON Utilities
// ================================================================================================

pub mod json {
    use super::*;

    /// Pretty print JSON
    pub fn pretty_print(value: &serde_json::Value) -> String {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    }

    /// Merge two JSON objects
    pub fn merge_objects(base: &mut serde_json::Value, other: serde_json::Value) {
        match (base, other) {
            (serde_json::Value::Object(base_map), serde_json::Value::Object(other_map)) => {
                for (key, value) in other_map {
                    base_map.insert(key, value);
                }
            }
            (base_value, other_value) => {
                *base_value = other_value;
            }
        }
    }

    /// Extract string field from JSON object
    pub fn get_string_field(obj: &serde_json::Value, field: &str) -> Option<String> {
        obj.get(field)?.as_str().map(|s| s.to_string())
    }

    /// Extract number field from JSON object
    pub fn get_number_field(obj: &serde_json::Value, field: &str) -> Option<f64> {
        obj.get(field)?.as_f64()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn test_merge_objects() {
            let mut base = json!({"a": 1, "b": 2});
            let other = json!({"b": 3, "c": 4});

            merge_objects(&mut base, other);

            assert_eq!(base, json!({"a": 1, "b": 3, "c": 4}));
        }

        #[test]
        fn test_get_fields() {
            let obj = json!({"name": "test", "value": 42.5});

            assert_eq!(get_string_field(&obj, "name"), Some("test".to_string()));
            assert_eq!(get_number_field(&obj, "value"), Some(42.5));
            assert_eq!(get_string_field(&obj, "missing"), None);
        }
    }
}
