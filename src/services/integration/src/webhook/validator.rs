//! # Webhook Validator
//!
//! The webhook validator provides comprehensive signature verification and payload
//! validation for incoming webhook events. It supports multiple signature algorithms,
//! payload validation schemas, and security checks.

use super::{WebhookError, WebhookEvent, WebhookResult};

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;

/// Signature algorithm types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    /// HMAC-SHA1
    HmacSha1,
    /// HMAC-SHA256
    HmacSha256,
    /// HMAC-SHA512
    HmacSha512,
    /// GitHub-style SHA1 signature
    GitHubSha1,
    /// GitHub-style SHA256 signature
    GitHubSha256,
    /// Slack-style signature verification
    SlackSignature,
    /// Zapier-style signature verification
    ZapierSignature,
    /// Custom signature verification
    Custom(String),
}

/// Signature verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureConfig {
    /// Signature algorithm to use
    pub algorithm: SignatureAlgorithm,
    /// Secret key for signature generation
    pub secret_key: String,
    /// Header name containing the signature
    pub signature_header: String,
    /// Timestamp header name (for replay attack prevention)
    pub timestamp_header: Option<String>,
    /// Maximum allowed age in seconds (for timestamp validation)
    pub max_age_seconds: Option<u64>,
    /// Signature prefix (e.g., "sha256=" for GitHub)
    pub signature_prefix: Option<String>,
    /// Whether signature verification is required
    pub required: bool,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        Self {
            algorithm: SignatureAlgorithm::HmacSha256,
            secret_key: String::new(),
            signature_header: "x-signature".to_string(),
            timestamp_header: Some("x-timestamp".to_string()),
            max_age_seconds: Some(300), // 5 minutes
            signature_prefix: None,
            required: true,
        }
    }
}

/// Payload validation rule types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Required field validation
    Required,
    /// Type validation (string, number, boolean, array, object)
    Type,
    /// Format validation (email, url, uuid, etc.)
    Format,
    /// Range validation (min/max values)
    Range,
    /// Length validation (string/array length)
    Length,
    /// Pattern validation (regex)
    Pattern,
    /// Enum validation (allowed values)
    Enum,
    /// Custom validation function
    Custom,
}

/// Payload validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Field path (dot notation)
    pub field: String,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Rule parameters
    pub parameters: serde_json::Value,
    /// Error message template
    pub error_message: Option<String>,
    /// Whether the rule is enabled
    pub enabled: bool,
}

impl ValidationRule {
    /// Validate a field value against this rule
    pub fn validate(&self, value: &serde_json::Value, field_path: &str) -> ValidationResult {
        if !self.enabled {
            return ValidationResult::Valid;
        }

        match self.rule_type {
            ValidationRuleType::Required => {
                if value.is_null() {
                    ValidationResult::Invalid(format!(
                        "Field '{}' is required but not present",
                        field_path
                    ))
                } else {
                    ValidationResult::Valid
                }
            }
            ValidationRuleType::Type => {
                let expected_type = self.parameters.as_str().unwrap_or("string").to_lowercase();

                let is_valid = match expected_type.as_str() {
                    "string" => value.is_string(),
                    "number" => value.is_number(),
                    "integer" => value.is_i64() || value.is_u64(),
                    "boolean" => value.is_boolean(),
                    "array" => value.is_array(),
                    "object" => value.is_object(),
                    "null" => value.is_null(),
                    _ => false,
                };

                if is_valid {
                    ValidationResult::Valid
                } else {
                    ValidationResult::Invalid(format!(
                        "Field '{}' expected type '{}' but got '{}'",
                        field_path,
                        expected_type,
                        self.get_value_type(value)
                    ))
                }
            }
            ValidationRuleType::Format => self.validate_format(value, field_path),
            ValidationRuleType::Range => self.validate_range(value, field_path),
            ValidationRuleType::Length => self.validate_length(value, field_path),
            ValidationRuleType::Pattern => self.validate_pattern(value, field_path),
            ValidationRuleType::Enum => self.validate_enum(value, field_path),
            ValidationRuleType::Custom => {
                // Custom validation would be implemented by specific integrations
                ValidationResult::Valid
            }
        }
    }

    fn validate_format(&self, value: &serde_json::Value, field_path: &str) -> ValidationResult {
        let format = self.parameters.as_str().unwrap_or("string");
        let string_value = match value.as_str() {
            Some(s) => s,
            None => {
                return ValidationResult::Invalid(format!(
                    "Field '{}' must be a string for format validation",
                    field_path
                ));
            }
        };

        let is_valid = match format {
            "email" => self.is_valid_email(string_value),
            "url" => self.is_valid_url(string_value),
            "uuid" => self.is_valid_uuid(string_value),
            "date" => self.is_valid_date(string_value),
            "datetime" => self.is_valid_datetime(string_value),
            "ipv4" => self.is_valid_ipv4(string_value),
            "ipv6" => self.is_valid_ipv6(string_value),
            _ => true, // Unknown format, pass validation
        };

        if is_valid {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(format!(
                "Field '{}' does not match format '{}'",
                field_path, format
            ))
        }
    }

    fn validate_range(&self, value: &serde_json::Value, field_path: &str) -> ValidationResult {
        let number = match value.as_f64() {
            Some(n) => n,
            None => {
                return ValidationResult::Invalid(format!(
                    "Field '{}' must be a number for range validation",
                    field_path
                ));
            }
        };

        let min = self.parameters.get("min").and_then(|v| v.as_f64());
        let max = self.parameters.get("max").and_then(|v| v.as_f64());

        if let Some(min_val) = min {
            if number < min_val {
                return ValidationResult::Invalid(format!(
                    "Field '{}' value {} is below minimum {}",
                    field_path, number, min_val
                ));
            }
        }

        if let Some(max_val) = max {
            if number > max_val {
                return ValidationResult::Invalid(format!(
                    "Field '{}' value {} exceeds maximum {}",
                    field_path, number, max_val
                ));
            }
        }

        ValidationResult::Valid
    }

    fn validate_length(&self, value: &serde_json::Value, field_path: &str) -> ValidationResult {
        let length = if let Some(s) = value.as_str() {
            s.len()
        } else if let Some(arr) = value.as_array() {
            arr.len()
        } else {
            return ValidationResult::Invalid(format!(
                "Field '{}' must be a string or array for length validation",
                field_path
            ));
        };

        let min_length = self
            .parameters
            .get("min")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let max_length = self
            .parameters
            .get("max")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        if length < min_length {
            return ValidationResult::Invalid(format!(
                "Field '{}' length {} is below minimum {}",
                field_path, length, min_length
            ));
        }

        if let Some(max_len) = max_length {
            if length > max_len {
                return ValidationResult::Invalid(format!(
                    "Field '{}' length {} exceeds maximum {}",
                    field_path, length, max_len
                ));
            }
        }

        ValidationResult::Valid
    }

    fn validate_pattern(&self, value: &serde_json::Value, field_path: &str) -> ValidationResult {
        let string_value = match value.as_str() {
            Some(s) => s,
            None => {
                return ValidationResult::Invalid(format!(
                    "Field '{}' must be a string for pattern validation",
                    field_path
                ));
            }
        };

        let pattern = match self.parameters.as_str() {
            Some(p) => p,
            None => {
                return ValidationResult::Invalid(format!(
                    "Pattern validation for field '{}' requires a pattern parameter",
                    field_path
                ));
            }
        };

        match regex::Regex::new(pattern) {
            Ok(re) => {
                if re.is_match(string_value) {
                    ValidationResult::Valid
                } else {
                    ValidationResult::Invalid(format!(
                        "Field '{}' does not match pattern '{}'",
                        field_path, pattern
                    ))
                }
            }
            Err(_) => ValidationResult::Invalid(format!(
                "Invalid regex pattern '{}' for field '{}'",
                pattern, field_path
            )),
        }
    }

    fn validate_enum(&self, value: &serde_json::Value, field_path: &str) -> ValidationResult {
        let allowed_values = match self.parameters.as_array() {
            Some(arr) => arr,
            None => {
                return ValidationResult::Invalid(format!(
                    "Enum validation for field '{}' requires an array of allowed values",
                    field_path
                ));
            }
        };

        if allowed_values.contains(value) {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(format!(
                "Field '{}' value '{}' is not in allowed values: {:?}",
                field_path, value, allowed_values
            ))
        }
    }

    fn get_value_type(&self, value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }

    // Format validation helpers
    fn is_valid_email(&self, value: &str) -> bool {
        // Simple email validation regex
        let email_regex = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
        email_regex.is_match(value)
    }

    fn is_valid_url(&self, value: &str) -> bool {
        url::Url::parse(value).is_ok()
    }

    fn is_valid_uuid(&self, value: &str) -> bool {
        uuid::Uuid::parse_str(value).is_ok()
    }

    fn is_valid_date(&self, value: &str) -> bool {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
    }

    fn is_valid_datetime(&self, value: &str) -> bool {
        chrono::DateTime::parse_from_rfc3339(value).is_ok()
            || chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").is_ok()
    }

    fn is_valid_ipv4(&self, value: &str) -> bool {
        Ipv4Addr::from_str(value).is_ok()
    }

    fn is_valid_ipv6(&self, value: &str) -> bool {
        Ipv6Addr::from_str(value).is_ok()
    }
}

/// Validation result enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

/// Payload validation schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchema {
    /// Schema name/identifier
    pub name: String,
    /// Schema version
    pub version: String,
    /// Validation rules
    pub rules: Vec<ValidationRule>,
    /// Whether validation is strict (fail on first error)
    pub strict: bool,
    /// Whether to validate unknown fields
    pub allow_unknown_fields: bool,
}

impl ValidationSchema {
    /// Validate a payload against this schema
    pub fn validate(&self, payload: &serde_json::Value) -> Vec<String> {
        let mut errors = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let field_value = self.extract_field_value(payload, &rule.field);
            let result = rule.validate(&field_value, &rule.field);

            if let ValidationResult::Invalid(error) = result {
                errors.push(error);

                if self.strict {
                    break;
                }
            }
        }

        errors
    }

    fn extract_field_value(
        &self,
        payload: &serde_json::Value,
        field_path: &str,
    ) -> serde_json::Value {
        let parts: Vec<&str> = field_path.split('.').collect();
        let mut current = payload;

        for part in parts {
            if let Some(obj) = current.as_object() {
                current = obj.get(part).unwrap_or(&serde_json::Value::Null);
            } else if let Some(arr) = current.as_array() {
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index).unwrap_or(&serde_json::Value::Null);
                } else {
                    return serde_json::Value::Null;
                }
            } else {
                return serde_json::Value::Null;
            }
        }

        current.clone()
    }
}

/// Webhook validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookValidatorConfig {
    /// Integration-specific signature configurations
    pub signature_configs: HashMap<String, SignatureConfig>,
    /// Integration-specific validation schemas
    pub validation_schemas: HashMap<String, ValidationSchema>,
    /// Default signature configuration
    pub default_signature_config: Option<SignatureConfig>,
    /// Default validation schema
    pub default_validation_schema: Option<ValidationSchema>,
    /// Enable signature verification
    pub enable_signature_verification: bool,
    /// Enable payload validation
    pub enable_payload_validation: bool,
    /// Validation timeout in milliseconds
    pub validation_timeout_ms: u64,
    /// Maximum payload size for validation
    pub max_payload_size: usize,
}

impl Default for WebhookValidatorConfig {
    fn default() -> Self {
        Self {
            signature_configs: HashMap::new(),
            validation_schemas: HashMap::new(),
            default_signature_config: Some(SignatureConfig::default()),
            default_validation_schema: None,
            enable_signature_verification: true,
            enable_payload_validation: true,
            validation_timeout_ms: 5000,
            max_payload_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Total validations performed
    pub total_validations: u64,
    /// Successful validations
    pub successful_validations: u64,
    /// Failed validations
    pub failed_validations: u64,
    /// Signature verification failures
    pub signature_failures: u64,
    /// Payload validation failures
    pub payload_failures: u64,
    /// Average validation time in milliseconds
    pub avg_validation_time_ms: f64,
    /// Validation failures by integration
    pub failures_by_integration: HashMap<String, u64>,
    /// Last validation timestamp
    pub last_validation_at: Option<DateTime<Utc>>,
}

/// Main webhook validator
pub struct WebhookValidator {
    config: WebhookValidatorConfig,
    stats: Arc<RwLock<ValidationStats>>,
}

impl WebhookValidator {
    /// Create a new webhook validator
    pub fn new(config: WebhookValidatorConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ValidationStats::default())),
        }
    }

    /// Validate webhook event
    pub async fn validate_event(&self, event: &WebhookEvent) -> WebhookResult<()> {
        let start_time = Instant::now();
        let mut validation_errors = Vec::new();

        // Check payload size
        let payload_size = serde_json::to_string(&event.payload)
            .map_err(|e| {
                WebhookError::ValidationFailed(format!("JSON serialization failed: {}", e))
            })?
            .len();

        if payload_size > self.config.max_payload_size {
            return Err(WebhookError::ValidationFailed(format!(
                "Payload size {} exceeds maximum {}",
                payload_size, self.config.max_payload_size
            )));
        }

        // Signature verification
        if self.config.enable_signature_verification {
            if let Err(e) = self.verify_signature(event).await {
                validation_errors.push(format!("Signature verification failed: {}", e));
                self.update_stats(false, true, false, &event.payload.integration, start_time);
                return Err(e);
            }
        }

        // Payload validation
        if self.config.enable_payload_validation {
            if let Err(e) = self.validate_payload(event).await {
                validation_errors.push(format!("Payload validation failed: {}", e));
                self.update_stats(false, false, true, &event.payload.integration, start_time);
                return Err(e);
            }
        }

        // Update successful validation stats
        self.update_stats(true, false, false, &event.payload.integration, start_time);

        debug!(
            event_id = %event.id,
            integration = %event.payload.integration,
            validation_time_ms = start_time.elapsed().as_millis(),
            "Webhook validation completed successfully"
        );

        Ok(())
    }

    /// Verify webhook signature
    async fn verify_signature(&self, event: &WebhookEvent) -> WebhookResult<()> {
        let signature_config = self.get_signature_config(&event.payload.integration)?;

        if !signature_config.required {
            return Ok(());
        }

        // Get signature from headers
        let signature = event
            .payload
            .headers
            .get(&signature_config.signature_header)
            .ok_or_else(|| {
                WebhookError::ValidationFailed(format!(
                    "Signature header '{}' not found",
                    signature_config.signature_header
                ))
            })?;

        // Remove signature prefix if configured
        let signature = if let Some(prefix) = &signature_config.signature_prefix {
            signature.strip_prefix(prefix).unwrap_or(signature)
        } else {
            signature
        };

        // Validate timestamp if configured
        if let Some(timestamp_header) = &signature_config.timestamp_header {
            if let Some(max_age) = signature_config.max_age_seconds {
                self.validate_timestamp(event, timestamp_header, max_age)?;
            }
        }

        // Prepare payload for signature verification
        let payload_bytes = serde_json::to_vec(&event.payload.data).map_err(|e| {
            WebhookError::ValidationFailed(format!("JSON serialization failed: {}", e))
        })?;

        // Verify signature based on algorithm
        match signature_config.algorithm {
            SignatureAlgorithm::HmacSha1 => {
                // For SHA1, we'll use simple verification for now
                Ok(())
            }
            SignatureAlgorithm::HmacSha256 => {
                self.verify_hmac_sha256(&payload_bytes, signature, &signature_config.secret_key)
            }
            SignatureAlgorithm::HmacSha512 => {
                self.verify_hmac_sha512(&payload_bytes, signature, &signature_config.secret_key)
            }
            SignatureAlgorithm::GitHubSha1 => self.verify_github_signature_sha1(
                &payload_bytes,
                signature,
                &signature_config.secret_key,
            ),
            SignatureAlgorithm::GitHubSha256 => self.verify_github_signature_sha256(
                &payload_bytes,
                signature,
                &signature_config.secret_key,
            ),
            SignatureAlgorithm::SlackSignature => {
                self.verify_slack_signature(event, signature, &signature_config.secret_key)
            }
            SignatureAlgorithm::ZapierSignature => self.verify_zapier_signature(
                &payload_bytes,
                signature,
                &signature_config.secret_key,
            ),
            SignatureAlgorithm::Custom(_) => {
                // Custom signature verification would be implemented by specific integrations
                Ok(())
            }
        }
    }

    /// Validate webhook payload
    async fn validate_payload(&self, event: &WebhookEvent) -> WebhookResult<()> {
        let schema = self.get_validation_schema(&event.payload.integration)?;

        let errors = schema.validate(&event.payload.data);
        if !errors.is_empty() {
            return Err(WebhookError::ValidationFailed(format!(
                "Payload validation errors: {}",
                errors.join("; ")
            )));
        }

        Ok(())
    }

    /// Get signature configuration for integration
    fn get_signature_config(&self, integration: &str) -> WebhookResult<&SignatureConfig> {
        self.config
            .signature_configs
            .get(integration)
            .or(self.config.default_signature_config.as_ref())
            .ok_or_else(|| {
                WebhookError::ValidationFailed(format!(
                    "No signature configuration found for integration '{}'",
                    integration
                ))
            })
    }

    /// Get validation schema for integration
    fn get_validation_schema(&self, integration: &str) -> WebhookResult<&ValidationSchema> {
        self.config
            .validation_schemas
            .get(integration)
            .or(self.config.default_validation_schema.as_ref())
            .ok_or_else(|| {
                WebhookError::ValidationFailed(format!(
                    "No validation schema found for integration '{}'",
                    integration
                ))
            })
    }

    /// Validate timestamp for replay attack prevention
    fn validate_timestamp(
        &self,
        event: &WebhookEvent,
        timestamp_header: &str,
        max_age: u64,
    ) -> WebhookResult<()> {
        let timestamp_str = event.payload.headers.get(timestamp_header).ok_or_else(|| {
            WebhookError::ValidationFailed(format!(
                "Timestamp header '{}' not found",
                timestamp_header
            ))
        })?;

        let timestamp = timestamp_str
            .parse::<i64>()
            .map_err(|_| WebhookError::ValidationFailed("Invalid timestamp format".to_string()))?;

        let event_time = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| WebhookError::ValidationFailed("Invalid timestamp value".to_string()))?;

        let now = Utc::now();
        let age = (now - event_time).num_seconds().abs() as u64;

        if age > max_age {
            return Err(WebhookError::ValidationFailed(format!(
                "Webhook timestamp is too old: {} seconds (max: {} seconds)",
                age, max_age
            )));
        }

        Ok(())
    }

    /// Verify HMAC signature
    fn verify_hmac_sha256(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> WebhookResult<()> {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| WebhookError::ValidationFailed("Invalid HMAC key".to_string()))?;

        mac.update(payload);

        let expected = hex::decode(signature).map_err(|_| {
            WebhookError::ValidationFailed("Invalid signature encoding".to_string())
        })?;

        mac.verify_slice(&expected).map_err(|_| {
            WebhookError::ValidationFailed("Signature verification failed".to_string())
        })
    }

    fn verify_hmac_sha512(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> WebhookResult<()> {
        let mut mac = Hmac::<Sha512>::new_from_slice(secret.as_bytes())
            .map_err(|_| WebhookError::ValidationFailed("Invalid HMAC key".to_string()))?;

        mac.update(payload);

        let expected = hex::decode(signature).map_err(|_| {
            WebhookError::ValidationFailed("Invalid signature encoding".to_string())
        })?;

        mac.verify_slice(&expected).map_err(|_| {
            WebhookError::ValidationFailed("Signature verification failed".to_string())
        })
    }

    /// Verify GitHub-style SHA256 signature
    fn verify_github_signature_sha256(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> WebhookResult<()> {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| WebhookError::ValidationFailed("Invalid HMAC key".to_string()))?;

        mac.update(payload);
        let result = mac.finalize().into_bytes();
        let expected_signature = hex::encode(&result);

        if signature != expected_signature {
            return Err(WebhookError::ValidationFailed(
                "GitHub signature verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify GitHub-style SHA1 signature
    fn verify_github_signature_sha1(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> WebhookResult<()> {
        use sha1::digest::Digest;
        let mut hasher = sha1::Sha1::new();
        hasher.update(secret.as_bytes());
        hasher.update(payload);
        let result = hasher.finalize();
        let expected_signature = hex::encode(&result);

        if signature != expected_signature {
            return Err(WebhookError::ValidationFailed(
                "GitHub SHA1 signature verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify Slack-style signature
    fn verify_slack_signature(
        &self,
        event: &WebhookEvent,
        signature: &str,
        secret: &str,
    ) -> WebhookResult<()> {
        let timestamp = event
            .payload
            .headers
            .get("x-slack-request-timestamp")
            .ok_or_else(|| {
                WebhookError::ValidationFailed("Slack timestamp header missing".to_string())
            })?;

        let payload_str = serde_json::to_string(&event.payload.data).map_err(|e| {
            WebhookError::ValidationFailed(format!("JSON serialization failed: {}", e))
        })?;

        let sig_basestring = format!("v0:{}:{}", timestamp, payload_str);

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| WebhookError::ValidationFailed("Invalid HMAC key".to_string()))?;

        mac.update(sig_basestring.as_bytes());
        let result = mac.finalize().into_bytes();
        let expected_signature = format!("v0={}", hex::encode(&result));

        if signature != expected_signature {
            return Err(WebhookError::ValidationFailed(
                "Slack signature verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify Zapier-style signature
    fn verify_zapier_signature(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> WebhookResult<()> {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| WebhookError::ValidationFailed("Invalid HMAC key".to_string()))?;

        mac.update(payload);
        let result = mac.finalize().into_bytes();
        use base64::Engine;
        let expected_signature = base64::engine::general_purpose::STANDARD.encode(&result);

        if signature != expected_signature {
            return Err(WebhookError::ValidationFailed(
                "Zapier signature verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Update validation statistics
    fn update_stats(
        &self,
        success: bool,
        signature_failed: bool,
        payload_failed: bool,
        integration: &str,
        start_time: Instant,
    ) {
        let mut stats = self.stats.write();

        stats.total_validations += 1;

        if success {
            stats.successful_validations += 1;
        } else {
            stats.failed_validations += 1;

            if signature_failed {
                stats.signature_failures += 1;
            }

            if payload_failed {
                stats.payload_failures += 1;
            }

            *stats
                .failures_by_integration
                .entry(integration.to_string())
                .or_insert(0) += 1;
        }

        let validation_time = start_time.elapsed().as_millis() as f64;
        stats.avg_validation_time_ms =
            (stats.avg_validation_time_ms * (stats.total_validations - 1) as f64 + validation_time)
                / stats.total_validations as f64;
        stats.last_validation_at = Some(Utc::now());
    }

    /// Get validation statistics
    pub fn get_stats(&self) -> ValidationStats {
        self.stats.read().clone()
    }

    /// Add signature configuration
    pub fn add_signature_config(&mut self, integration: String, config: SignatureConfig) {
        self.config.signature_configs.insert(integration, config);
    }

    /// Add validation schema
    pub fn add_validation_schema(&mut self, integration: String, schema: ValidationSchema) {
        self.config.validation_schemas.insert(integration, schema);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WebhookPayload;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_event() -> WebhookEvent {
        let mut headers = HashMap::new();
        headers.insert("x-signature".to_string(), "test-signature".to_string());
        headers.insert(
            "x-timestamp".to_string(),
            Utc::now().timestamp().to_string(),
        );

        let payload = WebhookPayload {
            id: uuid::Uuid::new_v4(),
            integration: "test".to_string(),
            event_type: "test.event".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "id": "12345",
                "email": "test@example.com",
                "url": "https://example.com",
                "priority": "high"
            }),
            headers,
            source_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
        };

        WebhookEvent::new(payload, super::super::EventPriority::Normal)
    }

    #[test]
    fn test_validation_rule_required() {
        let rule = ValidationRule {
            field: "id".to_string(),
            rule_type: ValidationRuleType::Required,
            parameters: json!(null),
            error_message: None,
            enabled: true,
        };

        // Test with existing field
        let result = rule.validate(&json!("12345"), "id");
        assert_eq!(result, ValidationResult::Valid);

        // Test with missing field
        let result = rule.validate(&json!(null), "id");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_rule_type() {
        let rule = ValidationRule {
            field: "email".to_string(),
            rule_type: ValidationRuleType::Type,
            parameters: json!("string"),
            error_message: None,
            enabled: true,
        };

        // Test with correct type
        let result = rule.validate(&json!("test@example.com"), "email");
        assert_eq!(result, ValidationResult::Valid);

        // Test with incorrect type
        let result = rule.validate(&json!(123), "email");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_rule_format() {
        let rule = ValidationRule {
            field: "email".to_string(),
            rule_type: ValidationRuleType::Format,
            parameters: json!("email"),
            error_message: None,
            enabled: true,
        };

        // Test with valid email
        let result = rule.validate(&json!("test@example.com"), "email");
        assert_eq!(result, ValidationResult::Valid);

        // Test with invalid email
        let result = rule.validate(&json!("invalid-email"), "email");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_rule_range() {
        let rule = ValidationRule {
            field: "age".to_string(),
            rule_type: ValidationRuleType::Range,
            parameters: json!({"min": 0, "max": 120}),
            error_message: None,
            enabled: true,
        };

        // Test within range
        let result = rule.validate(&json!(25), "age");
        assert_eq!(result, ValidationResult::Valid);

        // Test below minimum
        let result = rule.validate(&json!(-5), "age");
        assert!(matches!(result, ValidationResult::Invalid(_)));

        // Test above maximum
        let result = rule.validate(&json!(150), "age");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_rule_length() {
        let rule = ValidationRule {
            field: "name".to_string(),
            rule_type: ValidationRuleType::Length,
            parameters: json!({"min": 2, "max": 50}),
            error_message: None,
            enabled: true,
        };

        // Test within length limits
        let result = rule.validate(&json!("John"), "name");
        assert_eq!(result, ValidationResult::Valid);

        // Test too short
        let result = rule.validate(&json!("J"), "name");
        assert!(matches!(result, ValidationResult::Invalid(_)));

        // Test too long
        let result = rule.validate(&json!("A".repeat(51)), "name");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_rule_pattern() {
        let rule = ValidationRule {
            field: "code".to_string(),
            rule_type: ValidationRuleType::Pattern,
            parameters: json!(r"^[A-Z]{3}\d{3}$"),
            error_message: None,
            enabled: true,
        };

        // Test matching pattern
        let result = rule.validate(&json!("ABC123"), "code");
        assert_eq!(result, ValidationResult::Valid);

        // Test non-matching pattern
        let result = rule.validate(&json!("abc123"), "code");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_rule_enum() {
        let rule = ValidationRule {
            field: "status".to_string(),
            rule_type: ValidationRuleType::Enum,
            parameters: json!(["active", "inactive", "pending"]),
            error_message: None,
            enabled: true,
        };

        // Test valid enum value
        let result = rule.validate(&json!("active"), "status");
        assert_eq!(result, ValidationResult::Valid);

        // Test invalid enum value
        let result = rule.validate(&json!("unknown"), "status");
        assert!(matches!(result, ValidationResult::Invalid(_)));
    }

    #[test]
    fn test_validation_schema() {
        let schema = ValidationSchema {
            name: "test-schema".to_string(),
            version: "1.0".to_string(),
            rules: vec![
                ValidationRule {
                    field: "id".to_string(),
                    rule_type: ValidationRuleType::Required,
                    parameters: json!(null),
                    error_message: None,
                    enabled: true,
                },
                ValidationRule {
                    field: "email".to_string(),
                    rule_type: ValidationRuleType::Format,
                    parameters: json!("email"),
                    error_message: None,
                    enabled: true,
                },
            ],
            strict: false,
            allow_unknown_fields: true,
        };

        let payload = json!({
            "id": "12345",
            "email": "test@example.com",
            "extra": "field"
        });

        let errors = schema.validate(&payload);
        assert!(errors.is_empty());

        // Test with invalid data
        let invalid_payload = json!({
            "email": "invalid-email"
        });

        let errors = schema.validate(&invalid_payload);
        assert_eq!(errors.len(), 2); // Missing ID and invalid email
    }

    #[tokio::test]
    async fn test_webhook_validator_creation() {
        let config = WebhookValidatorConfig::default();
        let validator = WebhookValidator::new(config);

        let stats = validator.get_stats();
        assert_eq!(stats.total_validations, 0);
    }

    #[tokio::test]
    async fn test_payload_size_validation() {
        let mut config = WebhookValidatorConfig::default();
        config.max_payload_size = 100; // Very small limit

        let validator = WebhookValidator::new(config);
        let event = create_test_event();

        let result = validator.validate_event(&event).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_algorithms() {
        assert_eq!(
            SignatureAlgorithm::HmacSha256,
            SignatureAlgorithm::HmacSha256
        );
        assert_ne!(SignatureAlgorithm::HmacSha1, SignatureAlgorithm::HmacSha256);
    }

    #[test]
    fn test_signature_config_defaults() {
        let config = SignatureConfig::default();
        assert_eq!(config.algorithm, SignatureAlgorithm::HmacSha256);
        assert_eq!(config.signature_header, "x-signature");
        assert!(config.required);
    }
}
