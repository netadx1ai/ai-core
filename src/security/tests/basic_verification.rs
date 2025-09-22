//! Basic verification tests for the security framework
//!
//! Simple tests to verify core functionality works

use crate::config::SecurityConfig;
use crate::encryption::PasswordService;
use crate::utils::{generate_random_string, PasswordStrength};

#[test]
fn test_random_string_generation() {
    let s1 = generate_random_string(16);
    let s2 = generate_random_string(16);

    assert_eq!(s1.len(), 16);
    assert_eq!(s2.len(), 16);
    assert_ne!(s1, s2); // Should be different
}

#[test]
fn test_password_strength_basic() {
    let weak = PasswordStrength::analyze("123");
    let strong = PasswordStrength::analyze("MyStr0ng!P@ssw0rd");

    assert!(weak.score < strong.score);
    assert!(strong.has_uppercase);
    assert!(strong.has_lowercase);
    assert!(strong.has_numbers);
    assert!(strong.has_symbols);
}

#[test]
fn test_security_config_default() {
    let config = SecurityConfig::default();
    assert!(config.validate().is_ok());
    assert!(config.jwt.access_token_ttl.as_secs() > 0);
    assert!(!config.jwt.secret_key.is_empty());
    assert!(config.rate_limiting.requests_per_minute > 0);
}

#[tokio::test]
async fn test_password_service_basic() {
    let config = SecurityConfig::default();
    let password_service = PasswordService::new(config.encryption.password);

    let password = "TestPassword123!";
    let hash_result = password_service.hash_password(password).await.unwrap();

    assert!(!hash_result.hash.is_empty());
    assert!(hash_result.hash.starts_with("$argon2id$"));

    // Verify password
    let is_valid = password_service
        .verify_password(password, &hash_result.hash)
        .await
        .unwrap();
    assert!(is_valid);

    // Wrong password should fail
    let is_invalid = password_service
        .verify_password("WrongPassword", &hash_result.hash)
        .await
        .unwrap();
    assert!(!is_invalid);
}

#[test]
fn test_basic_functionality_works() {
    // This test ensures the basic security components can be instantiated
    let config = SecurityConfig::default();

    // Test configuration is valid
    assert!(config.validate().is_ok());

    // Test utilities work
    let random_str = generate_random_string(32);
    assert_eq!(random_str.len(), 32);

    // Test password analysis
    let analysis = PasswordStrength::analyze("TestPassword123!");
    assert!(analysis.score > 50);

    println!("✅ Basic security functionality verification passed");
}
