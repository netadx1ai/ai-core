//! Security Utilities Module
//!
//! Provides common utility functions for security operations.

use crate::errors::{SecurityError, SecurityResult};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

/// Generate a secure random string
pub fn generate_random_string(length: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Generate cryptographically secure random bytes
pub fn generate_secure_random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Generate a secure random token
pub fn generate_secure_token(length: usize) -> String {
    let bytes = generate_secure_random_bytes(length);
    BASE64_STANDARD.encode(&bytes)
}

/// Generate a UUID v4
pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Constant-time string comparison
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes()
        .zip(b.bytes())
        .fold(0, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Constant-time byte slice comparison
pub fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.iter().zip(b.iter()).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Hash a string using SHA-256
pub fn sha256_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    BASE64_STANDARD.encode(&result)
}

/// Encode data to base64
pub fn base64_encode(data: &[u8]) -> String {
    BASE64_STANDARD.encode(data)
}

/// Decode data from base64
pub fn base64_decode(data: &str) -> SecurityResult<Vec<u8>> {
    BASE64_STANDARD
        .decode(data)
        .map_err(|e| SecurityError::InvalidInputFormat(e.to_string()))
}

/// URL-safe base64 encoding
pub fn base64_url_encode(data: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    URL_SAFE_NO_PAD.encode(data)
}

/// URL-safe base64 decoding
pub fn base64_url_decode(data: &str) -> SecurityResult<Vec<u8>> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| SecurityError::InvalidInputFormat(e.to_string()))
}

/// Sanitize string for logging (remove sensitive patterns)
pub fn sanitize_for_logging(input: &str) -> String {
    let sensitive_patterns = [
        "password",
        "token",
        "secret",
        "key",
        "authorization",
        "bearer",
    ];

    let mut sanitized = input.to_string();
    for pattern in &sensitive_patterns {
        if sanitized.to_lowercase().contains(pattern) {
            sanitized = "[REDACTED]".to_string();
            break;
        }
    }

    // Truncate if too long
    if sanitized.len() > 100 {
        sanitized = format!("{}...", &sanitized[..97]);
    }

    sanitized
}

/// Format duration in human-readable format
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Time utilities
pub struct TimeUtils;

impl TimeUtils {
    /// Get current UTC timestamp
    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// Create expiry timestamp
    pub fn expires_at(duration: Duration) -> DateTime<Utc> {
        Utc::now() + chrono::Duration::from_std(duration).unwrap_or_default()
    }

    /// Check if timestamp is expired
    pub fn is_expired(timestamp: DateTime<Utc>, grace_period: Duration) -> bool {
        let now = Utc::now();
        let grace = chrono::Duration::from_std(grace_period).unwrap_or_default();
        timestamp + grace < now
    }

    /// Calculate time until expiry
    pub fn time_until_expiry(expires_at: DateTime<Utc>) -> Option<Duration> {
        let now = Utc::now();
        if expires_at <= now {
            None
        } else {
            Some(Duration::from_secs(
                (expires_at.timestamp() - now.timestamp()) as u64,
            ))
        }
    }

    /// Check if timestamp is within acceptable window
    pub fn is_within_window(timestamp: DateTime<Utc>, window: Duration) -> bool {
        let now = Utc::now();
        let window_duration = chrono::Duration::from_std(window).unwrap_or_default();
        let diff = (now - timestamp).abs();
        diff <= window_duration
    }
}

/// Cryptographic utilities
pub struct CryptoUtils;

impl CryptoUtils {
    /// Generate cryptographically secure random bytes
    pub fn generate_random_bytes(length: usize) -> Vec<u8> {
        generate_secure_random_bytes(length)
    }

    /// Generate random string for tokens
    pub fn generate_random_string(length: usize) -> String {
        generate_random_string(length)
    }

    /// Generate secure token
    pub fn generate_token() -> String {
        generate_secure_token(32)
    }

    /// Constant-time comparison for security
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        constant_time_eq_bytes(a, b)
    }

    /// Generate salt for hashing
    pub fn generate_salt() -> String {
        generate_secure_token(16)
    }
}

/// String utilities for security
pub struct StringUtils;

impl StringUtils {
    /// Remove null bytes from string
    pub fn sanitize_string(input: &str) -> String {
        input.replace('\0', "")
    }

    /// Escape HTML entities
    pub fn escape_html(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// URL encode string
    pub fn url_encode(input: &str) -> String {
        urlencoding::encode(input).to_string()
    }

    /// Normalize whitespace
    pub fn normalize_whitespace(input: &str) -> String {
        input.split_whitespace().collect::<Vec<&str>>().join(" ")
    }

    /// Truncate string to max length
    pub fn truncate(input: &str, max_length: usize) -> String {
        if input.len() <= max_length {
            input.to_string()
        } else {
            format!("{}...", &input[..max_length.saturating_sub(3)])
        }
    }

    /// Check if string is safe (basic validation)
    pub fn is_safe_string(input: &str) -> bool {
        !input.contains('<') && !input.contains('>') && !input.contains('\0')
    }
}

/// IP address utilities
pub struct IpUtils;

impl IpUtils {
    /// Check if IP is localhost
    pub fn is_localhost(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        }
    }

    /// Check if IP is private
    pub fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_private() || ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        }
    }

    /// Parse IP address from string
    pub fn parse_ip(ip_str: &str) -> SecurityResult<IpAddr> {
        IpAddr::from_str(ip_str)
            .map_err(|e| SecurityError::InvalidInputFormat(format!("Invalid IP address: {}", e)))
    }

    /// Extract IP from X-Forwarded-For header
    pub fn extract_real_ip(forwarded_for: &str) -> Option<IpAddr> {
        forwarded_for
            .split(',')
            .map(|ip| ip.trim())
            .find_map(|ip| IpAddr::from_str(ip).ok())
    }
}

/// Security context utilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Request identifier
    pub request_id: String,
    /// Client IP address
    pub client_ip: Option<IpAddr>,
    /// User agent string
    pub user_agent: Option<String>,
    /// User identifier if authenticated
    pub user_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            client_ip: None,
            user_agent: None,
            user_id: None,
            metadata: HashMap::new(),
            timestamp: TimeUtils::now(),
        }
    }

    /// Set client IP address
    pub fn with_client_ip(mut self, ip: IpAddr) -> Self {
        self.client_ip = Some(ip);
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if request is from a trusted IP
    pub fn is_trusted_ip(&self) -> bool {
        self.client_ip
            .map(|ip| IpUtils::is_localhost(&ip) || IpUtils::is_private_ip(&ip))
            .unwrap_or(false)
    }

    /// Get a summary of the security context
    pub fn summary(&self) -> String {
        format!(
            "Request {} from {} at {}",
            self.request_id,
            self.client_ip
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

/// Password strength analyzer
pub struct PasswordStrength;

impl PasswordStrength {
    /// Analyze password strength
    pub fn analyze(password: &str) -> PasswordAnalysis {
        let mut analysis = PasswordAnalysis {
            score: 0,
            length_score: 0,
            complexity_score: 0,
            entropy_score: 0,
            has_uppercase: false,
            has_lowercase: false,
            has_numbers: false,
            has_symbols: false,
            common_patterns: Vec::new(),
        };

        // Length scoring
        analysis.length_score = match password.len() {
            0..=5 => 0,
            6..=7 => 10,
            8..=11 => 20,
            12..=15 => 30,
            _ => 40,
        };

        // Character type analysis
        analysis.has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
        analysis.has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
        analysis.has_numbers = password.chars().any(|c| c.is_ascii_digit());
        analysis.has_symbols = password.chars().any(|c| c.is_ascii_punctuation());

        // Complexity scoring
        let mut complexity = 0;
        if analysis.has_uppercase {
            complexity += 10;
        }
        if analysis.has_lowercase {
            complexity += 10;
        }
        if analysis.has_numbers {
            complexity += 10;
        }
        if analysis.has_symbols {
            complexity += 15;
        }
        analysis.complexity_score = complexity;

        // Check for common patterns
        if password.to_lowercase().contains("password") {
            analysis
                .common_patterns
                .push("Contains 'password'".to_string());
        }
        if password
            .chars()
            .collect::<Vec<_>>()
            .windows(3)
            .any(|w| w.iter().collect::<String>() == "123" || w.iter().collect::<String>() == "abc")
        {
            analysis
                .common_patterns
                .push("Contains sequential characters".to_string());
        }

        // Entropy calculation (simplified)
        let charset_size: f64 = if analysis.has_symbols {
            95.0
        } else if analysis.has_numbers {
            62.0
        } else if analysis.has_uppercase && analysis.has_lowercase {
            52.0
        } else if analysis.has_lowercase || analysis.has_uppercase {
            26.0
        } else {
            10.0
        };

        let entropy = (password.len() as f64) * charset_size.log2();
        analysis.entropy_score = (entropy / 10.0).min(40.0) as u32;

        // Deduct points for common patterns
        let pattern_penalty = analysis.common_patterns.len() as u32 * 10;

        // Calculate final score
        analysis.score =
            (analysis.length_score + analysis.complexity_score + analysis.entropy_score)
                .saturating_sub(pattern_penalty);

        analysis
    }

    /// Check if password meets minimum requirements
    pub fn meets_requirements(
        password: &str,
        min_length: usize,
        require_mixed_case: bool,
        require_numbers: bool,
        require_symbols: bool,
    ) -> bool {
        if password.len() < min_length {
            return false;
        }

        let analysis = Self::analyze(password);

        if require_mixed_case && (!analysis.has_uppercase || !analysis.has_lowercase) {
            return false;
        }

        if require_numbers && !analysis.has_numbers {
            return false;
        }

        if require_symbols && !analysis.has_symbols {
            return false;
        }

        true
    }
}

/// Password analysis result
#[derive(Debug, Clone)]
pub struct PasswordAnalysis {
    pub score: u32,
    pub length_score: u32,
    pub complexity_score: u32,
    pub entropy_score: u32,
    pub has_uppercase: bool,
    pub has_lowercase: bool,
    pub has_numbers: bool,
    pub has_symbols: bool,
    pub common_patterns: Vec<String>,
}

impl PasswordAnalysis {
    /// Get password strength level
    pub fn strength_level(&self) -> PasswordStrengthLevel {
        match self.score {
            0..=30 => PasswordStrengthLevel::Weak,
            31..=60 => PasswordStrengthLevel::Fair,
            61..=80 => PasswordStrengthLevel::Good,
            81..=95 => PasswordStrengthLevel::Strong,
            _ => PasswordStrengthLevel::VeryStrong,
        }
    }

    /// Get recommendations for improving password
    pub fn recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if self.length_score < 20 {
            recommendations.push("Use at least 8 characters".to_string());
        }
        if !self.has_uppercase {
            recommendations.push("Include uppercase letters".to_string());
        }
        if !self.has_lowercase {
            recommendations.push("Include lowercase letters".to_string());
        }
        if !self.has_numbers {
            recommendations.push("Include numbers".to_string());
        }
        if !self.has_symbols {
            recommendations.push("Include special characters".to_string());
        }
        if !self.common_patterns.is_empty() {
            recommendations.push("Avoid common patterns and dictionary words".to_string());
        }

        recommendations
    }
}

/// Password strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrengthLevel {
    Weak,
    Fair,
    Good,
    Strong,
    VeryStrong,
}

impl std::fmt::Display for PasswordStrengthLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordStrengthLevel::Weak => write!(f, "Weak"),
            PasswordStrengthLevel::Fair => write!(f, "Fair"),
            PasswordStrengthLevel::Good => write!(f, "Good"),
            PasswordStrengthLevel::Strong => write!(f, "Strong"),
            PasswordStrengthLevel::VeryStrong => write!(f, "Very Strong"),
        }
    }
}

/// Input sanitization utilities
pub struct SanitizationUtils;

impl SanitizationUtils {
    /// Sanitize HTML input
    pub fn sanitize_html(input: &str) -> String {
        input
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('&', "&amp;")
    }

    /// Sanitize SQL input (basic)
    pub fn sanitize_sql_basic(input: &str) -> String {
        input.replace('\'', "''").replace(';', "")
    }

    /// Remove null bytes
    pub fn remove_null_bytes(input: &str) -> String {
        input.replace('\0', "")
    }

    /// Normalize whitespace
    pub fn normalize_whitespace(input: &str) -> String {
        input.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}

/// Validation utilities
pub struct ValidationUtils;

impl ValidationUtils {
    /// Check if string contains only allowed characters
    pub fn is_alphanumeric_with_underscore(input: &str) -> bool {
        input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Check if string is a valid identifier
    pub fn is_valid_identifier(input: &str) -> bool {
        !input.is_empty()
            && input.len() <= 64
            && input.chars().next().unwrap().is_ascii_alphabetic()
            && Self::is_alphanumeric_with_underscore(input)
    }

    /// Check if email format is valid (basic)
    pub fn is_valid_email_basic(email: &str) -> bool {
        email.contains('@')
            && email.len() <= 254
            && !email.starts_with('@')
            && !email.ends_with('@')
    }

    /// Check if URL is safe
    pub fn is_safe_url(url: &str) -> bool {
        url.starts_with("https://") || url.starts_with("http://")
    }
}

/// Redaction utilities for sensitive data
pub struct RedactionUtils;

impl RedactionUtils {
    /// Redact credit card numbers
    pub fn redact_credit_card(input: &str) -> String {
        use regex::Regex;
        let re = Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").unwrap();
        re.replace_all(input, "[REDACTED-CC]").to_string()
    }

    /// Redact email addresses
    pub fn redact_emails(input: &str) -> String {
        use regex::Regex;
        let re = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
        re.replace_all(input, "[REDACTED-EMAIL]").to_string()
    }

    /// Redact phone numbers
    pub fn redact_phone_numbers(input: &str) -> String {
        use regex::Regex;
        let re = Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();
        re.replace_all(input, "[REDACTED-PHONE]").to_string()
    }

    /// Generic PII redaction
    pub fn redact_pii(input: &str) -> String {
        let mut result = Self::redact_credit_card(input);
        result = Self::redact_emails(&result);
        result = Self::redact_phone_numbers(&result);
        result
    }
}

/// Security headers helper
pub struct SecurityHeaders;

impl SecurityHeaders {
    /// Get standard security headers
    pub fn get_standard_headers() -> Vec<(String, String)> {
        vec![
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            ("X-Frame-Options".to_string(), "DENY".to_string()),
            ("X-XSS-Protection".to_string(), "1; mode=block".to_string()),
            (
                "Strict-Transport-Security".to_string(),
                "max-age=31536000; includeSubDomains".to_string(),
            ),
            (
                "Referrer-Policy".to_string(),
                "strict-origin-when-cross-origin".to_string(),
            ),
            (
                "Content-Security-Policy".to_string(),
                "default-src 'self'".to_string(),
            ),
        ]
    }

    /// Check if header value is safe
    pub fn is_header_value_safe(value: &str) -> bool {
        // Check for control characters and common injection patterns
        !value.chars().any(|c| c.is_control()) && !value.contains('\n') && !value.contains('\r')
    }
}

/// Error context helper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub client_ip: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            timestamp: Utc::now(),
            request_id: None,
            user_id: None,
            client_ip: None,
        }
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_client_ip(mut self, client_ip: impl Into<String>) -> Self {
        self.client_ip = Some(client_ip.into());
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operation={}, timestamp={}, request_id={:?}, user_id={:?}, client_ip={:?}",
            self.operation, self.timestamp, self.request_id, self.user_id, self.client_ip
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_string() {
        let s1 = generate_random_string(32);
        let s2 = generate_random_string(32);

        assert_eq!(s1.len(), 32);
        assert_eq!(s2.len(), 32);
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("hello", "hello"));
        assert!(!constant_time_eq("hello", "world"));
        assert!(!constant_time_eq("hello", "hello2"));
    }

    #[test]
    fn test_base64_operations() {
        let data = b"hello world";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_sanitize_for_logging() {
        let sensitive = "password=secret123";
        let sanitized = sanitize_for_logging(sensitive);
        assert_eq!(sanitized, "[REDACTED]");

        let safe = "username=john";
        let sanitized = sanitize_for_logging(safe);
        assert_eq!(sanitized, safe);
    }

    #[test]
    fn test_format_duration() {
        let duration = std::time::Duration::from_secs(3661);
        let formatted = format_duration(duration);
        assert_eq!(formatted, "1h 1m 1s");
    }

    #[test]
    fn test_time_utils() {
        let now = TimeUtils::now();
        let future = TimeUtils::expires_at(Duration::from_secs(300));

        assert!(future > now);
        assert!(!TimeUtils::is_expired(now, Duration::from_secs(300)));
        assert!(TimeUtils::time_until_expiry(future).is_some());
        assert!(TimeUtils::is_within_window(now, Duration::from_secs(60)));
    }

    #[test]
    fn test_crypto_utils() {
        let bytes = CryptoUtils::generate_random_bytes(32);
        assert_eq!(bytes.len(), 32);

        let string = CryptoUtils::generate_random_string(16);
        assert_eq!(string.len(), 16);

        let token = CryptoUtils::generate_token();
        assert!(!token.is_empty());

        // Test constant time comparison
        assert!(CryptoUtils::constant_time_eq(b"hello", b"hello"));
        assert!(!CryptoUtils::constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn test_string_utils() {
        assert_eq!(StringUtils::sanitize_string("hello\0world"), "helloworld");
        assert_eq!(StringUtils::escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(StringUtils::url_encode("hello world"), "hello%20world");
        assert_eq!(
            StringUtils::normalize_whitespace("  hello   world  "),
            "hello world"
        );
        assert_eq!(StringUtils::truncate("hello world", 5), "he...");
        assert!(StringUtils::is_safe_string("hello_world123"));
        assert!(!StringUtils::is_safe_string("hello<script>"));
    }

    #[test]
    fn test_ip_utils() {
        let localhost = "127.0.0.1".parse::<IpAddr>().unwrap();
        assert!(IpUtils::is_localhost(&localhost));
        assert!(IpUtils::is_private_ip(&localhost));

        let private_ip = "192.168.1.1".parse::<IpAddr>().unwrap();
        assert!(IpUtils::is_private_ip(&private_ip));
        assert!(!IpUtils::is_localhost(&private_ip));

        assert!(IpUtils::parse_ip("127.0.0.1").is_ok());
        assert!(IpUtils::parse_ip("invalid").is_err());
    }

    #[test]
    fn test_password_strength() {
        let weak = PasswordStrength::analyze("123");
        assert_eq!(weak.strength_level(), PasswordStrengthLevel::Weak);

        let strong = PasswordStrength::analyze("MyStr0ngP@ssw0rd!");
        assert!(matches!(
            strong.strength_level(),
            PasswordStrengthLevel::Strong | PasswordStrengthLevel::VeryStrong
        ));
        assert!(strong.has_uppercase);
        assert!(strong.has_lowercase);
        assert!(strong.has_numbers);
        assert!(strong.has_symbols);

        assert!(!PasswordStrength::meets_requirements(
            "short", 8, true, true, true
        ));
        assert!(PasswordStrength::meets_requirements(
            "MyStr0ngP@ss!",
            8,
            true,
            true,
            true
        ));
    }

    #[test]
    fn test_html_sanitization() {
        let malicious = "<script>alert('xss')</script>";
        let sanitized = SanitizationUtils::sanitize_html(malicious);
        assert_eq!(
            sanitized,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_validation_utils() {
        assert!(ValidationUtils::is_valid_identifier("valid_name"));
        assert!(!ValidationUtils::is_valid_identifier("123invalid"));
        assert!(!ValidationUtils::is_valid_identifier(""));

        assert!(ValidationUtils::is_valid_email_basic("test@example.com"));
        assert!(!ValidationUtils::is_valid_email_basic("invalid"));
    }

    #[test]
    fn test_redaction_utils() {
        let text = "My card is 1234 5678 9012 3456 and email is test@example.com";
        let redacted = RedactionUtils::redact_pii(text);
        assert!(redacted.contains("[REDACTED-CC]"));
        assert!(redacted.contains("[REDACTED-EMAIL]"));
    }

    #[test]
    fn test_security_headers() {
        let headers = SecurityHeaders::get_standard_headers();
        assert!(!headers.is_empty());

        assert!(SecurityHeaders::is_header_value_safe("safe value"));
        assert!(!SecurityHeaders::is_header_value_safe("unsafe\nvalue"));
    }

    #[test]
    fn test_security_context() {
        let ctx = SecurityContext::new("req-123".to_string())
            .with_client_ip("127.0.0.1".parse().unwrap())
            .with_user_agent("test-agent".to_string())
            .with_user_id("user-456".to_string())
            .with_metadata("key".to_string(), "value".to_string());

        assert_eq!(ctx.request_id, "req-123");
        assert!(ctx.client_ip.is_some());
        assert_eq!(ctx.user_agent.as_ref().unwrap(), "test-agent");
        assert_eq!(ctx.user_id.as_ref().unwrap(), "user-456");
        assert_eq!(ctx.get_metadata("key").unwrap(), "value");
        assert!(ctx.is_trusted_ip());
    }
}
