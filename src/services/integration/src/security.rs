//! Security module for the AI-CORE Integration Service
//!
//! This module provides security utilities for webhook signature verification,
//! API key validation, and other security-related functionality.

use crate::error::{IntegrationError, IntegrationResult};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use subtle::ConstantTimeEq;
use tracing::{debug, warn};

type HmacSha256 = Hmac<Sha256>;

/// Security utilities for webhook verification
pub struct SecurityUtils;

impl SecurityUtils {
    /// Verify HMAC-SHA256 signature
    pub fn verify_hmac_sha256(
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> IntegrationResult<bool> {
        // Remove any prefix (like "sha256=")
        let signature = if signature.starts_with("sha256=") {
            &signature[7..]
        } else {
            signature
        };

        // Decode hex signature
        let provided_signature = hex::decode(signature).map_err(|_| {
            IntegrationError::signature_verification("generic", "Invalid signature format")
        })?;

        // Create HMAC with secret
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
            IntegrationError::signature_verification("generic", format!("HMAC error: {}", e))
        })?;

        mac.update(payload);
        let computed_signature = mac.finalize().into_bytes();

        // Constant-time comparison
        let is_valid: bool = computed_signature.ct_eq(&provided_signature).into();

        if !is_valid {
            warn!("HMAC signature verification failed");
        }

        Ok(is_valid)
    }

    /// Verify Zapier webhook signature
    pub fn verify_zapier_signature(
        payload: &[u8],
        headers: &HashMap<String, String>,
        secret: &str,
    ) -> IntegrationResult<bool> {
        debug!("Verifying Zapier webhook signature");

        let signature = headers
            .get("x-zapier-signature")
            .or_else(|| headers.get("X-Zapier-Signature"))
            .ok_or_else(|| {
                IntegrationError::signature_verification("zapier", "Missing signature header")
            })?;

        Self::verify_hmac_sha256(payload, signature, secret)
    }

    /// Verify Slack webhook signature
    pub fn verify_slack_signature(
        payload: &[u8],
        headers: &HashMap<String, String>,
        signing_secret: &str,
    ) -> IntegrationResult<bool> {
        debug!("Verifying Slack webhook signature");

        let signature = headers
            .get("x-slack-signature")
            .or_else(|| headers.get("X-Slack-Signature"))
            .ok_or_else(|| {
                IntegrationError::signature_verification("slack", "Missing signature header")
            })?;

        let timestamp = headers
            .get("x-slack-request-timestamp")
            .or_else(|| headers.get("X-Slack-Request-Timestamp"))
            .ok_or_else(|| {
                IntegrationError::signature_verification("slack", "Missing timestamp header")
            })?;

        // Check timestamp to prevent replay attacks (within 5 minutes)
        let current_time = chrono::Utc::now().timestamp();
        let request_time = timestamp.parse::<i64>().map_err(|_| {
            IntegrationError::signature_verification("slack", "Invalid timestamp format")
        })?;

        if (current_time - request_time).abs() > 300 {
            return Err(IntegrationError::signature_verification(
                "slack",
                "Request timestamp too old",
            ));
        }

        // Create Slack signature base string
        let sig_basestring = format!("v0:{}:", timestamp);
        let sig_basestring = sig_basestring
            .as_bytes()
            .iter()
            .chain(payload.iter())
            .copied()
            .collect::<Vec<u8>>();

        // Verify signature
        Self::verify_hmac_sha256(&sig_basestring, signature, signing_secret)
    }

    /// Verify GitHub webhook signature
    pub fn verify_github_signature(
        payload: &[u8],
        headers: &HashMap<String, String>,
        secret: &str,
    ) -> IntegrationResult<bool> {
        debug!("Verifying GitHub webhook signature");

        let signature = headers
            .get("x-hub-signature-256")
            .or_else(|| headers.get("X-Hub-Signature-256"))
            .ok_or_else(|| {
                IntegrationError::signature_verification("github", "Missing signature header")
            })?;

        Self::verify_hmac_sha256(payload, signature, secret)
    }

    /// Validate API key
    pub fn validate_api_key(
        headers: &HashMap<String, String>,
        valid_keys: &[String],
    ) -> IntegrationResult<bool> {
        let api_key = headers
            .get("authorization")
            .or_else(|| headers.get("Authorization"))
            .or_else(|| headers.get("x-api-key"))
            .or_else(|| headers.get("X-API-Key"))
            .ok_or_else(|| IntegrationError::authentication("Missing API key"))?;

        // Handle Bearer token format
        let api_key = if api_key.starts_with("Bearer ") {
            &api_key[7..]
        } else if api_key.starts_with("ApiKey ") {
            &api_key[7..]
        } else {
            api_key
        };

        let is_valid = valid_keys.iter().any(|key| key == api_key);

        if !is_valid {
            warn!("Invalid API key provided");
            return Err(IntegrationError::authentication("Invalid API key"));
        }

        Ok(true)
    }

    /// Generate a secure random string for secrets
    pub fn generate_secret(length: usize) -> String {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    /// Constant-time string comparison
    pub fn constant_time_eq(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let a_bytes = a.as_bytes();
        let b_bytes = b.as_bytes();
        let mut result = 0u8;

        for i in 0..a_bytes.len() {
            result |= a_bytes[i] ^ b_bytes[i];
        }

        result == 0
    }

    /// Extract IP address from headers (considering proxies)
    pub fn extract_real_ip(
        headers: &HashMap<String, String>,
        connect_ip: &str,
        trusted_proxies: &[String],
    ) -> String {
        // Check X-Forwarded-For header
        if let Some(forwarded_for) = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("X-Forwarded-For"))
        {
            if let Some(first_ip) = forwarded_for.split(',').next() {
                let first_ip = first_ip.trim();
                // Only trust if the connection comes from a trusted proxy
                if trusted_proxies.iter().any(|proxy| proxy == connect_ip) {
                    return first_ip.to_string();
                }
            }
        }

        // Check X-Real-IP header
        if let Some(real_ip) = headers
            .get("x-real-ip")
            .or_else(|| headers.get("X-Real-IP"))
        {
            if trusted_proxies.iter().any(|proxy| proxy == connect_ip) {
                return real_ip.trim().to_string();
            }
        }

        // Fall back to connection IP
        connect_ip.to_string()
    }

    /// Sanitize webhook payload for logging
    pub fn sanitize_for_logging(payload: &serde_json::Value) -> serde_json::Value {
        use serde_json::{Map, Value};

        match payload {
            Value::Object(obj) => {
                let mut sanitized = Map::new();
                for (key, value) in obj {
                    let key_lower = key.to_lowercase();
                    if key_lower.contains("token")
                        || key_lower.contains("secret")
                        || key_lower.contains("password")
                        || key_lower.contains("key")
                        || key_lower.contains("auth")
                    {
                        sanitized.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                    } else {
                        sanitized.insert(key.clone(), Self::sanitize_for_logging(value));
                    }
                }
                Value::Object(sanitized)
            }
            Value::Array(arr) => Value::Array(arr.iter().map(Self::sanitize_for_logging).collect()),
            _ => payload.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_hmac_verification() {
        let payload = b"test payload";
        let secret = "test-secret";

        // Generate valid signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let result = SecurityUtils::verify_hmac_sha256(payload, &signature, secret);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with sha256= prefix
        let prefixed_signature = format!("sha256={}", signature);
        let result = SecurityUtils::verify_hmac_sha256(payload, &prefixed_signature, secret);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with invalid signature
        let result = SecurityUtils::verify_hmac_sha256(payload, "invalid", secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_zapier_signature_verification() {
        let payload = b"test payload";
        let secret = "test-secret";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let mut headers = HashMap::new();
        headers.insert("x-zapier-signature".to_string(), signature);

        let result = SecurityUtils::verify_zapier_signature(payload, &headers, secret);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test missing header
        let empty_headers = HashMap::new();
        let result = SecurityUtils::verify_zapier_signature(payload, &empty_headers, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_github_signature_verification() {
        let payload = b"test payload";
        let secret = "test-secret";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let mut headers = HashMap::new();
        headers.insert("x-hub-signature-256".to_string(), signature);

        let result = SecurityUtils::verify_github_signature(payload, &headers, secret);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_api_key_validation() {
        let valid_keys = vec!["key1".to_string(), "key2".to_string()];

        // Test valid API key
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "key1".to_string());
        let result = SecurityUtils::validate_api_key(&headers, &valid_keys);
        assert!(result.is_ok());

        // Test Bearer token format
        headers.insert("authorization".to_string(), "Bearer key2".to_string());
        let result = SecurityUtils::validate_api_key(&headers, &valid_keys);
        assert!(result.is_ok());

        // Test invalid key
        headers.insert("x-api-key".to_string(), "invalid".to_string());
        let result = SecurityUtils::validate_api_key(&headers, &valid_keys);
        assert!(result.is_err());

        // Test missing key
        let empty_headers = HashMap::new();
        let result = SecurityUtils::validate_api_key(&empty_headers, &valid_keys);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_secret() {
        let secret = SecurityUtils::generate_secret(32);
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()));

        // Test that generated secrets are different
        let secret2 = SecurityUtils::generate_secret(32);
        assert_ne!(secret, secret2);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(SecurityUtils::constant_time_eq("test", "test"));
        assert!(!SecurityUtils::constant_time_eq("test", "different"));
        assert!(!SecurityUtils::constant_time_eq("test", "longer_string"));
        assert!(!SecurityUtils::constant_time_eq("", "test"));
    }

    #[test]
    fn test_extract_real_ip() {
        let trusted_proxies = vec!["192.168.1.1".to_string(), "10.0.0.1".to_string()];

        // Test X-Forwarded-For from trusted proxy
        let mut headers = HashMap::new();
        headers.insert(
            "x-forwarded-for".to_string(),
            "203.0.113.1, 192.168.1.1".to_string(),
        );
        let ip = SecurityUtils::extract_real_ip(&headers, "192.168.1.1", &trusted_proxies);
        assert_eq!(ip, "203.0.113.1");

        // Test X-Real-IP from trusted proxy
        headers.clear();
        headers.insert("x-real-ip".to_string(), "203.0.113.2".to_string());
        let ip = SecurityUtils::extract_real_ip(&headers, "10.0.0.1", &trusted_proxies);
        assert_eq!(ip, "203.0.113.2");

        // Test from untrusted proxy (should return connection IP)
        let ip = SecurityUtils::extract_real_ip(&headers, "203.0.113.100", &trusted_proxies);
        assert_eq!(ip, "203.0.113.100");

        // Test no proxy headers
        let empty_headers = HashMap::new();
        let ip = SecurityUtils::extract_real_ip(&empty_headers, "203.0.113.3", &trusted_proxies);
        assert_eq!(ip, "203.0.113.3");
    }

    #[test]
    fn test_sanitize_for_logging() {
        let payload = json!({
            "user": "john_doe",
            "api_token": "secret123",
            "password": "mysecret",
            "data": {
                "name": "John",
                "auth_key": "another_secret",
                "public_info": "visible"
            },
            "items": [
                {"id": 1, "secret_value": "hidden"},
                {"id": 2, "normal_value": "visible"}
            ]
        });

        let sanitized = SecurityUtils::sanitize_for_logging(&payload);

        assert_eq!(sanitized["user"], "john_doe");
        assert_eq!(sanitized["api_token"], "[REDACTED]");
        assert_eq!(sanitized["password"], "[REDACTED]");
        assert_eq!(sanitized["data"]["name"], "John");
        assert_eq!(sanitized["data"]["auth_key"], "[REDACTED]");
        assert_eq!(sanitized["data"]["public_info"], "visible");
        assert_eq!(sanitized["items"][0]["secret_value"], "[REDACTED]");
        assert_eq!(sanitized["items"][1]["normal_value"], "visible");
    }

    #[test]
    fn test_slack_signature_verification() {
        let payload = b"test payload";
        let signing_secret = "test-secret";
        let timestamp = chrono::Utc::now().timestamp().to_string();

        // Create Slack signature
        let sig_basestring = format!("v0:{}:", timestamp);
        let sig_basestring = sig_basestring
            .as_bytes()
            .iter()
            .chain(payload.iter())
            .copied()
            .collect::<Vec<u8>>();

        let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).unwrap();
        mac.update(&sig_basestring);
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        let mut headers = HashMap::new();
        headers.insert("x-slack-signature".to_string(), signature);
        headers.insert("x-slack-request-timestamp".to_string(), timestamp);

        let result = SecurityUtils::verify_slack_signature(payload, &headers, signing_secret);
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test old timestamp (should fail)
        let old_timestamp = (chrono::Utc::now().timestamp() - 400).to_string();
        headers.insert("x-slack-request-timestamp".to_string(), old_timestamp);
        let result = SecurityUtils::verify_slack_signature(payload, &headers, signing_secret);
        assert!(result.is_err());
    }
}
