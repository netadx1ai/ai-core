//! Input Validation Module
//!
//! Provides input sanitization and validation capabilities for security.

use crate::errors::{SecurityError, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationConfig {
    /// Maximum input length
    pub max_input_length: usize,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum number of headers
    pub max_headers: usize,
    /// Maximum header value length
    pub max_header_length: usize,
    /// Blocked user agents
    pub blocked_user_agents: Vec<String>,
    /// Allowed file types
    pub allowed_file_types: Vec<String>,
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            max_input_length: 10_000,
            max_request_size: 10 * 1024 * 1024, // 10 MB
            max_headers: 100,
            max_header_length: 8192,
            blocked_user_agents: vec![
                "bot".to_string(),
                "crawler".to_string(),
                "spider".to_string(),
            ],
            allowed_file_types: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "pdf".to_string(),
                "txt".to_string(),
            ],
        }
    }
}

/// Input validator service
pub struct InputValidator {
    config: SanitizationConfig,
}

impl InputValidator {
    /// Create new input validator
    pub fn new(config: SanitizationConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SanitizationConfig::default())
    }

    /// Validate and sanitize text input
    pub fn sanitize_text(&self, input: &str) -> SecurityResult<String> {
        // Check length
        if input.len() > self.config.max_input_length {
            return Err(SecurityError::InputTooLong {
                max: self.config.max_input_length,
                actual: input.len(),
            });
        }

        // Basic HTML escaping
        let sanitized = input
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('&', "&amp;");

        Ok(sanitized)
    }

    /// Validate email format
    pub fn validate_email(&self, email: &str) -> SecurityResult<bool> {
        if email.len() > 254 {
            return Ok(false);
        }

        // Basic email validation
        let email_regex = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").map_err(|e| {
            SecurityError::InputValidation {
                field: "email".to_string(),
                message: e.to_string(),
            }
        })?;

        Ok(email_regex.is_match(email))
    }

    /// Validate username format
    pub fn validate_username(&self, username: &str) -> SecurityResult<bool> {
        if username.is_empty() || username.len() > 64 {
            return Ok(false);
        }

        // Alphanumeric plus underscore and hyphen
        let username_regex =
            regex::Regex::new(r"^[a-zA-Z0-9_-]+$").map_err(|e| SecurityError::InputValidation {
                field: "username".to_string(),
                message: e.to_string(),
            })?;

        Ok(username_regex.is_match(username))
    }

    /// Check for potentially malicious input patterns
    pub fn check_malicious_patterns(&self, input: &str) -> SecurityResult<Vec<String>> {
        let mut threats = Vec::new();

        // SQL injection patterns
        let sql_patterns = [
            "union", "select", "insert", "delete", "update", "drop", "exec", "execute", "--", "/*",
            "*/", "'", "\"", ";",
        ];

        let input_lower = input.to_lowercase();
        for pattern in &sql_patterns {
            if input_lower.contains(pattern) {
                threats.push(format!("Potential SQL injection: {}", pattern));
            }
        }

        // XSS patterns
        let xss_patterns = ["<script", "javascript:", "onload=", "onerror=", "onclick="];
        for pattern in &xss_patterns {
            if input_lower.contains(pattern) {
                threats.push(format!("Potential XSS: {}", pattern));
            }
        }

        // Path traversal patterns
        let path_patterns = ["../", "..\\", "/etc/", "/proc/", "c:\\"];
        for pattern in &path_patterns {
            if input_lower.contains(pattern) {
                threats.push(format!("Potential path traversal: {}", pattern));
            }
        }

        Ok(threats)
    }

    /// Validate file upload
    pub fn validate_file_upload(
        &self,
        filename: &str,
        content_type: &str,
        size: usize,
    ) -> SecurityResult<()> {
        // Check file size
        if size > self.config.max_request_size {
            return Err(SecurityError::InputTooLong {
                max: self.config.max_request_size,
                actual: size,
            });
        }

        // Check file extension
        let extension = filename.split('.').last().unwrap_or("").to_lowercase();

        if !self.config.allowed_file_types.contains(&extension) {
            return Err(SecurityError::UnsupportedFileType(extension));
        }

        // Validate content type matches extension
        let expected_content_types: HashMap<&str, Vec<&str>> = [
            ("jpg", vec!["image/jpeg"]),
            ("jpeg", vec!["image/jpeg"]),
            ("png", vec!["image/png"]),
            ("gif", vec!["image/gif"]),
            ("pdf", vec!["application/pdf"]),
            ("txt", vec!["text/plain"]),
        ]
        .iter()
        .cloned()
        .collect();

        if let Some(expected_types) = expected_content_types.get(extension.as_str()) {
            if !expected_types.contains(&content_type) {
                return Err(SecurityError::FileValidation(format!(
                    "Content type '{}' does not match file extension '{}'",
                    content_type, extension
                )));
            }
        }

        Ok(())
    }

    /// Sanitize headers
    pub fn sanitize_headers(
        &self,
        headers: &HashMap<String, String>,
    ) -> SecurityResult<HashMap<String, String>> {
        if headers.len() > self.config.max_headers {
            return Err(SecurityError::InputValidation {
                field: "headers".to_string(),
                message: format!("Too many headers: {}", headers.len()),
            });
        }

        let mut sanitized = HashMap::new();

        for (name, value) in headers {
            // Check header name
            if name.is_empty() || name.len() > 256 {
                continue; // Skip invalid header names
            }

            // Check header value length
            if value.len() > self.config.max_header_length {
                return Err(SecurityError::InputTooLong {
                    max: self.config.max_header_length,
                    actual: value.len(),
                });
            }

            // Basic sanitization
            let sanitized_value = value
                .chars()
                .filter(|c| c.is_ascii() && !c.is_control() || *c == '\t')
                .collect::<String>();

            sanitized.insert(name.clone(), sanitized_value);
        }

        Ok(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_text() {
        let validator = InputValidator::with_defaults();

        let input = "<script>alert('xss')</script>";
        let result = validator.sanitize_text(input).unwrap();
        assert_eq!(
            result,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_validate_email() {
        let validator = InputValidator::with_defaults();

        assert!(validator.validate_email("test@example.com").unwrap());
        assert!(!validator.validate_email("invalid-email").unwrap());
        assert!(!validator.validate_email("@example.com").unwrap());
        assert!(!validator.validate_email("test@").unwrap());
    }

    #[test]
    fn test_validate_username() {
        let validator = InputValidator::with_defaults();

        assert!(validator.validate_username("valid_user-123").unwrap());
        assert!(!validator.validate_username("invalid user").unwrap());
        assert!(!validator.validate_username("user@domain").unwrap());
        assert!(!validator.validate_username("").unwrap());
    }

    #[test]
    fn test_check_malicious_patterns() {
        let validator = InputValidator::with_defaults();

        let malicious_input = "'; DROP TABLE users; --";
        let threats = validator.check_malicious_patterns(malicious_input).unwrap();
        assert!(!threats.is_empty());

        let safe_input = "Hello, world!";
        let threats = validator.check_malicious_patterns(safe_input).unwrap();
        assert!(threats.is_empty());
    }

    #[test]
    fn test_validate_file_upload() {
        let validator = InputValidator::with_defaults();

        // Valid file
        assert!(validator
            .validate_file_upload("image.jpg", "image/jpeg", 1024)
            .is_ok());

        // Invalid extension
        assert!(validator
            .validate_file_upload("malware.exe", "application/octet-stream", 1024)
            .is_err());

        // Content type mismatch
        assert!(validator
            .validate_file_upload("image.jpg", "text/plain", 1024)
            .is_err());
    }
}
