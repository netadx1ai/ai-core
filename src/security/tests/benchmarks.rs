//! Security Benchmarks
//!
//! Performance benchmarks for security components.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use uuid::Uuid;

use ai_core_security::{
    config::SecurityConfig,
    encryption::{EncryptionService, PasswordService},
    jwt::{JwtService, JwtServiceTrait},
    rbac::{RbacService, RbacServiceTrait},
    service::SecurityService,
    utils::{generate_secure_random_bytes, PasswordStrength},
};

/// Benchmark JWT token generation
fn bench_jwt_generation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let security_service = rt.block_on(async {
        let config = SecurityConfig::default();
        SecurityService::new(config).await.unwrap()
    });

    let jwt_service = security_service.jwt_service().clone();

    c.bench_function("jwt_token_generation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let user_id = Uuid::new_v4();
                let email = "benchmark@example.com";
                let roles = vec!["user".to_string()];

                jwt_service
                    .generate_access_token(black_box(user_id), black_box(email), black_box(roles))
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark JWT token validation
fn bench_jwt_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (jwt_service, token) = rt.block_on(async {
        let config = SecurityConfig::default();
        let security_service = SecurityService::new(config).await.unwrap();
        let jwt_service = security_service.jwt_service().clone();

        let user_id = Uuid::new_v4();
        let token = jwt_service
            .generate_access_token(user_id, "benchmark@example.com", vec!["user".to_string()])
            .await
            .unwrap();

        (jwt_service, token)
    });

    c.bench_function("jwt_token_validation", |b| {
        b.iter(|| {
            rt.block_on(async {
                jwt_service
                    .validate_access_token(black_box(&token.token))
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark password hashing
fn bench_password_hashing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let password_service = rt.block_on(async {
        let config = SecurityConfig::default();
        PasswordService::new(config.encryption.password)
    });

    c.bench_function("password_hashing", |b| {
        b.iter(|| {
            rt.block_on(async {
                password_service
                    .hash_password(black_box("benchmark_password_123!"))
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark password verification
fn bench_password_verification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (password_service, hash) = rt.block_on(async {
        let config = SecurityConfig::default();
        let password_service = PasswordService::new(config.encryption.password);
        let hash = password_service
            .hash_password("benchmark_password_123!")
            .await
            .unwrap();
        (password_service, hash)
    });

    c.bench_function("password_verification", |b| {
        b.iter(|| {
            rt.block_on(async {
                password_service
                    .verify_password(black_box("benchmark_password_123!"), black_box(&hash.hash))
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark AES encryption
fn bench_aes_encryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let encryption_service = rt.block_on(async {
        let config = SecurityConfig::default();
        EncryptionService::new(config.encryption).await.unwrap()
    });

    let data = b"This is benchmark data for AES encryption performance testing.";

    c.bench_function("aes_encryption", |b| {
        b.iter(|| {
            rt.block_on(async {
                encryption_service
                    .encrypt_aes_256(black_box(data), None)
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark AES decryption
fn bench_aes_decryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (encryption_service, encrypted_data) = rt.block_on(async {
        let config = SecurityConfig::default();
        let encryption_service = EncryptionService::new(config.encryption).await.unwrap();
        let data = b"This is benchmark data for AES encryption performance testing.";
        let encrypted = encryption_service
            .encrypt_aes_256(data, None)
            .await
            .unwrap();
        (encryption_service, encrypted)
    });

    c.bench_function("aes_decryption", |b| {
        b.iter(|| {
            rt.block_on(async {
                encryption_service
                    .decrypt_aes_256(black_box(&encrypted_data), None)
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark ChaCha20-Poly1305 encryption
fn bench_chacha20_encryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let encryption_service = rt.block_on(async {
        let config = SecurityConfig::default();
        EncryptionService::new(config.encryption).await.unwrap()
    });

    let data = b"This is benchmark data for ChaCha20-Poly1305 encryption performance testing.";

    c.bench_function("chacha20_encryption", |b| {
        b.iter(|| {
            rt.block_on(async {
                encryption_service
                    .encrypt_chacha20_poly1305(black_box(data), None)
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark ChaCha20-Poly1305 decryption
fn bench_chacha20_decryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (encryption_service, encrypted_data) = rt.block_on(async {
        let config = SecurityConfig::default();
        let encryption_service = EncryptionService::new(config.encryption).await.unwrap();
        let data = b"This is benchmark data for ChaCha20-Poly1305 encryption performance testing.";
        let encrypted = encryption_service
            .encrypt_chacha20_poly1305(data, None)
            .await
            .unwrap();
        (encryption_service, encrypted)
    });

    c.bench_function("chacha20_decryption", |b| {
        b.iter(|| {
            rt.block_on(async {
                encryption_service
                    .decrypt_chacha20_poly1305(black_box(&encrypted_data), None)
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark RBAC permission checking
fn bench_rbac_permission_check(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (rbac_service, user_id) = rt.block_on(async {
        let config = SecurityConfig::default();
        let security_service = SecurityService::new(config).await.unwrap();
        let rbac_service = security_service.rbac_service().clone();

        let user_id = Uuid::new_v4();
        rbac_service
            .assign_role_to_user(user_id, "benchmark_role".to_string())
            .await
            .unwrap();
        rbac_service
            .grant_permission_to_role("benchmark_role", "benchmark_resource", "read")
            .await
            .unwrap();

        (rbac_service, user_id)
    });

    c.bench_function("rbac_permission_check", |b| {
        b.iter(|| {
            rt.block_on(async {
                rbac_service
                    .check_permission(
                        black_box(user_id),
                        black_box("benchmark_resource"),
                        black_box("read"),
                    )
                    .await
                    .unwrap()
            })
        })
    });
}

/// Benchmark password strength analysis
fn bench_password_strength_analysis(c: &mut Criterion) {
    let passwords = [
        "weak",
        "better123",
        "StrongPassword123!",
        "VeryStr0ng&C0mpl3x!P@ssw0rd#2023",
    ];

    c.bench_function("password_strength_analysis", |b| {
        b.iter(|| {
            for password in &passwords {
                PasswordStrength::analyze(black_box(password));
            }
        })
    });
}

/// Benchmark secure random bytes generation
fn bench_secure_random_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("secure_random_generation");

    for size in [16, 32, 64, 128, 256].iter() {
        group.bench_with_input(format!("{}_bytes", size), size, |b, &size| {
            b.iter(|| generate_secure_random_bytes(black_box(size)))
        });
    }

    group.finish();
}

/// Benchmark concurrent JWT operations
fn bench_concurrent_jwt_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let jwt_service = rt.block_on(async {
        let config = SecurityConfig::default();
        let security_service = SecurityService::new(config).await.unwrap();
        security_service.jwt_service().clone()
    });

    c.bench_function("concurrent_jwt_operations", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];

                for i in 0..10 {
                    let jwt_service = jwt_service.clone();
                    let handle = tokio::spawn(async move {
                        let user_id = Uuid::new_v4();
                        let email = format!("user{}@benchmark.com", i);
                        let token = jwt_service
                            .generate_access_token(user_id, &email, vec!["user".to_string()])
                            .await
                            .unwrap();

                        jwt_service
                            .validate_access_token(&token.token)
                            .await
                            .unwrap()
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });
}

/// Benchmark full security middleware pipeline
fn bench_security_middleware_pipeline(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let security_service = rt.block_on(async {
        let config = SecurityConfig::default();
        SecurityService::new(config).await.unwrap()
    });

    let jwt_service = security_service.jwt_service().clone();
    let rbac_service = security_service.rbac_service().clone();

    c.bench_function("security_middleware_pipeline", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate full security pipeline
                let user_id = Uuid::new_v4();

                // 1. Generate token
                let token = jwt_service
                    .generate_access_token(
                        user_id,
                        "pipeline@benchmark.com",
                        vec!["user".to_string()],
                    )
                    .await
                    .unwrap();

                // 2. Validate token
                let validation_result = jwt_service
                    .validate_access_token(&token.token)
                    .await
                    .unwrap();

                // 3. Check permissions
                rbac_service
                    .assign_role_to_user(validation_result.user_id, "user".to_string())
                    .await
                    .unwrap();

                let has_permission = rbac_service
                    .check_permission(validation_result.user_id, "resource", "read")
                    .await
                    .unwrap();

                black_box(has_permission)
            })
        })
    });
}

/// Benchmark memory usage of security operations
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("memory_intensive_security_ops", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = SecurityConfig::default();
                let security_service = SecurityService::new(config).await.unwrap();
                let jwt_service = security_service.jwt_service().clone();
                let encryption_service = security_service.encryption_service().clone();

                // Generate multiple tokens
                let mut tokens = vec![];
                for i in 0..100 {
                    let user_id = Uuid::new_v4();
                    let token = jwt_service
                        .generate_access_token(
                            user_id,
                            &format!("user{}@memory.test", i),
                            vec!["user".to_string()],
                        )
                        .await
                        .unwrap();
                    tokens.push(token);
                }

                // Encrypt multiple data blocks
                let mut encrypted_blocks = vec![];
                for i in 0..50 {
                    let data = format!("Memory test data block {}", i).into_bytes();
                    let encrypted = encryption_service
                        .encrypt_aes_256(&data, None)
                        .await
                        .unwrap();
                    encrypted_blocks.push(encrypted);
                }

                // Validate all tokens
                for token in &tokens {
                    jwt_service
                        .validate_access_token(&token.token)
                        .await
                        .unwrap();
                }

                // Decrypt all blocks
                for encrypted in &encrypted_blocks {
                    encryption_service
                        .decrypt_aes_256(encrypted, None)
                        .await
                        .unwrap();
                }

                black_box((tokens, encrypted_blocks))
            })
        })
    });
}

criterion_group!(
    security_benches,
    bench_jwt_generation,
    bench_jwt_validation,
    bench_password_hashing,
    bench_password_verification,
    bench_aes_encryption,
    bench_aes_decryption,
    bench_chacha20_encryption,
    bench_chacha20_decryption,
    bench_rbac_permission_check,
    bench_password_strength_analysis,
    bench_secure_random_generation,
    bench_concurrent_jwt_operations,
    bench_security_middleware_pipeline,
    bench_memory_usage
);

criterion_main!(security_benches);
