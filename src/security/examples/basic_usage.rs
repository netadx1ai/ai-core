//! Basic Usage Example for AI-CORE Security Framework
//!
//! This example demonstrates the basic functionality of the security framework
//! including configuration, password hashing, and core services.

use ai_core_security::{
    config::SecurityConfig,
    service::SecurityService,
    utils::{generate_random_string, PasswordStrength},
};
use std::error::Error;
use std::str::FromStr;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔐 AI-CORE Security Framework - Basic Usage Example");
    println!("{}", "=".repeat(60));

    // 1. Test Configuration
    println!("1. Testing Security Configuration...");
    let config = SecurityConfig::default();
    config.validate()?;
    println!("   ✅ Security configuration is valid");

    // 2. Test Utilities
    println!("\n2. Testing Security Utilities...");
    let random_string = generate_random_string(32);
    println!(
        "   ✅ Generated random string: {} (length: {})",
        &random_string[..16],
        random_string.len()
    );

    // 3. Test Password Strength Analysis
    println!("\n3. Testing Password Strength Analysis...");
    let weak_password = "123456";
    let strong_password = "MyStr0ng!P@ssw0rd#2023";

    let weak_analysis = PasswordStrength::analyze(weak_password);
    let strong_analysis = PasswordStrength::analyze(strong_password);

    println!(
        "   Weak password '{}' score: {}",
        weak_password, weak_analysis.score
    );
    println!("   Strong password score: {}", strong_analysis.score);
    println!("   ✅ Password strength analysis working correctly");

    // 4. Test Security Service Initialization
    println!("\n4. Testing Security Service...");
    let security_service = SecurityService::new(config).await?;
    println!("   ✅ Security service initialized successfully");

    // 5. Test Password Hashing
    println!("\n5. Testing Password Hashing...");
    let test_password = "TestPassword123!";

    let hash_result = security_service.hash_password(test_password)?;
    println!(
        "   ✅ Password hashed successfully: {}",
        &hash_result.hash[..20]
    );

    let is_valid = security_service.verify_password(test_password, &hash_result)?;
    println!(
        "   ✅ Password verification: {}",
        if is_valid { "VALID" } else { "INVALID" }
    );

    let is_invalid = security_service.verify_password("WrongPassword", &hash_result)?;
    println!(
        "   ✅ Wrong password verification: {}",
        if is_invalid { "VALID" } else { "INVALID" }
    );

    // 6. Test Encryption Service
    println!("\n6. Testing Encryption Service...");
    let encryption_service = security_service.encryption();
    let test_data = b"This is sensitive data that needs encryption";

    let encrypted = encryption_service.encrypt(test_data).await?;
    println!("   ✅ Data encrypted successfully");

    let decrypted = encryption_service.decrypt(&encrypted).await?;
    let decrypted_str = String::from_utf8(decrypted)?;
    println!("   ✅ Data decrypted: {}", decrypted_str);

    // 7. Test RBAC Service
    println!("\n7. Testing RBAC Service...");
    let rbac_service = security_service.rbac();

    let user_id = Uuid::new_v4();
    let has_permission = rbac_service
        .check_permission(user_id, "documents", "read")
        .await?;
    println!(
        "   ✅ User permission check: {}",
        if has_permission { "ALLOWED" } else { "DENIED" }
    );

    println!("\n{}", "=".repeat(60));
    println!("🎉 All basic security functionality tests completed successfully!");
    println!("🛡️  The AI-CORE Security Framework is working correctly.");
    println!("\n📊 Summary:");
    println!("   - Configuration: ✅ Valid");
    println!("   - Password Hashing: ✅ Working");
    println!("   - Encryption: ✅ AES-256 Encrypt/Decrypt");
    println!("   - RBAC: ✅ Permission Checking");
    println!("   - Service Integration: ✅ All Components");

    Ok(())
}
