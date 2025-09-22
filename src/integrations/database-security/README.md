# Database-Security Integration

A comprehensive Rust crate that provides secure database access patterns by integrating AI-CORE's security-agent services with database-agent repositories. This integration ensures all database operations are authenticated, authorized, encrypted, and audited.

## 🎯 Overview

The Database-Security Integration bridges the gap between the AI-CORE security framework and database operations, providing:

- **Secure Authentication**: JWT-based authentication for all database access
- **Fine-grained Authorization**: RBAC/ABAC permission checking for database operations
- **Data Encryption**: Transparent encryption/decryption of sensitive data at rest
- **Comprehensive Audit Logging**: Complete audit trail of all database activities
- **Performance Monitoring**: Real-time metrics and performance tracking
- **Multi-Database Support**: PostgreSQL, ClickHouse, MongoDB, and Redis

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  SecureDatabaseManager                     │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │ SecurityContext │  │ AccessControl   │  │ AuditLogger  │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │ DataEncryption  │  │ Metrics         │  │ Config       │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                 Secure Repository Layer                    │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────┐ │
│  │ PostgreSQL  │ │ ClickHouse  │ │ MongoDB     │ │ Redis  │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Basic Usage

```rust
use database_security_integration::{SecureDatabaseManager, SecurityContext};
use shared_types::auth::UserId;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize secure database manager
    let db_manager = SecureDatabaseManager::new().await?;

    // Create security context for authenticated user
    let user_id = UserId::new();
    let permissions = HashSet::from([
        "user:read".to_string(),
        "workflow:create".to_string(),
    ]);
    let roles = vec!["user".to_string()];

    let security_context = SecurityContext::new(
        user_id,
        None,
        permissions,
        roles
    );

    // Perform secure database operations
    let user_data = db_manager
        .secure_postgres()
        .get_user_with_permissions(&security_context, &user_id)
        .await?;

    println!("User data retrieved: {:?}", user_data);
    Ok(())
}
```

### Token-Based Authentication

```rust
use database_security_integration::SecureDatabaseManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_manager = SecureDatabaseManager::new().await?;

    // Authenticate with JWT token
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
    let security_context = db_manager.authenticate_token(token).await?;

    // Now perform authenticated operations
    let workflows = db_manager
        .secure_postgres()
        .get_workflow_secure(&security_context, &workflow_id)
        .await?;

    Ok(())
}
```

## 🔧 Configuration

### YAML Configuration

```yaml
database:
  postgres:
    host: "localhost"
    port: 5432
    database: "ai_core"
    username: "ai_core"
    password: "secure_password"
    ssl_mode: "require"
    connect_timeout: 30
    query_timeout: 60

  redis:
    host: "localhost"
    port: 6379
    database: 0
    password: "redis_password"
    pool_size: 10

security:
  jwt:
    secret: "your-jwt-secret-key"
    expiration: 3600
    algorithm: "HS256"

access_control:
  strict_mode: true
  enable_permission_caching: true
  cache_ttl_seconds: 300

audit:
  enabled: true
  store_in_database: true
  store_in_files: true
  retention_days: 365

encryption:
  enabled: true
  field_level_encryption: true
  always_encrypt_fields:
    - "email"
    - "phone"
    - "ssn"
    - "credit_card"
```

### Environment Variables

```bash
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_DATABASE=ai_core
export POSTGRES_USERNAME=ai_core
export POSTGRES_PASSWORD=secure_password

export REDIS_HOST=localhost
export REDIS_PORT=6379
export REDIS_PASSWORD=redis_password

export JWT_SECRET=your-jwt-secret-key
export DEBUG_MODE=false
```

### Programmatic Configuration

```rust
use database_security_integration::{SecureDatabaseManager, SecureDatabaseConfig};

let mut config = SecureDatabaseConfig::default();
config.database.postgres.host = "production-db.example.com".to_string();
config.security.jwt.secret = "production-secret".to_string();
config.audit.retention_days = 2555; // 7 years

let db_manager = SecureDatabaseManager::with_config(config).await?;
```

## 🔐 Security Features

### Authentication

```rust
// JWT token validation
let context = db_manager.authenticate_token(&jwt_token).await?;

// Session-based authentication
let context = db_manager.create_security_context(&user_id, Some(session_id)).await?;

// System-level authentication for service accounts
let context = SecurityContext::system_context("backup-service", permissions);
```

### Authorization

```rust
// Role-based access control
if context.has_role("admin") {
    // Admin operations
}

// Permission-based access control
if context.has_permission("user:delete") {
    // Delete operations
}

// Multi-permission checks
if context.has_all_permissions(&["workflow:read", "workflow:execute"]) {
    // Workflow execution
}
```

### Data Encryption

```rust
// Field-level encryption
let encrypted_email = data_encryption.encrypt_field("email", &user_email).await?;
let decrypted_email = data_encryption.decrypt_field("email", &encrypted_email).await?;

// Record-level encryption
let mut user_record = serde_json::json!({
    "name": "John Doe",
    "email": "john@example.com",
    "ssn": "123-45-6789"
});

data_encryption.encrypt_record("users", &mut user_record).await?;
```

### Audit Logging

```rust
// Data access logging
audit_logger.log_data_access(
    &context,
    "users",
    &user_id.to_string(),
    "read",
    "User profile accessed"
).await;

// Data modification logging
audit_logger.log_data_change(
    &context,
    "users",
    &user_id.to_string(),
    "update",
    "User profile updated",
    Some(&old_value),
    Some(&new_value)
).await;

// Security event logging
audit_logger.log_security_event(
    &context,
    "suspicious_login",
    "high",
    "Multiple failed login attempts detected",
    serde_json::json!({
        "attempts": 5,
        "time_window": "5 minutes"
    })
).await;
```

## 📊 Monitoring & Metrics

### Health Checks

```rust
// Database health check
let health_status = db_manager.health_check(&context).await?;
println!("PostgreSQL: {}", health_status.postgres);
println!("Redis: {}", health_status.redis);
```

### Performance Metrics

```rust
// Get security metrics
let metrics = db_manager.get_security_metrics(&context).await?;
println!("Total operations: {}", metrics.total_operations);
println!("Failed authentications: {}", metrics.failed_authentications);
println!("Average response time: {:.2}ms", metrics.avg_response_time_ms);
```

### Prometheus Integration

```rust
let metrics_collector = SecureDatabaseMetrics::new()?;
let prometheus_output = metrics_collector.export_prometheus().await?;

// Export metrics for monitoring systems
println!("{}", prometheus_output);
```

## 🔧 Advanced Usage

### Custom Security Context

```rust
use database_security_integration::{SecurityContext, SecurityContextMetadata, SecurityLevel};

let mut context = SecurityContext::new(user_id, None, permissions, roles);

// Add request metadata
context.set_request_id("req-12345".to_string());
context.set_organization_id(org_id);

// Elevate security context for sensitive operations
context.elevate()?;

// Validate MFA for critical operations
context.validate_mfa_for_operation("user:delete")?;
```

### Batch Operations

```rust
// Batch user creation with encryption
let users = vec![user1, user2, user3];
let mut encrypted_users = Vec::new();

for user in users {
    let encrypted_user = db_manager
        .secure_postgres()
        .create_user_secure(&context, user)
        .await?;
    encrypted_users.push(encrypted_user);
}
```

### Cross-Database Transactions

```rust
// Coordinated operations across multiple databases
let workflow_data = db_manager
    .secure_postgres()
    .get_workflow_secure(&context, &workflow_id)
    .await?;

let analytics_data = db_manager
    .secure_clickhouse()
    .get_user_analytics(&context, &user_id, date_range)
    .await?;

let cached_result = db_manager
    .secure_redis()
    .get_cached_data(&context, &cache_key)
    .await?;
```

## 🧪 Testing

### Unit Tests

```bash
cargo test --lib
```

### Integration Tests

```bash
# Start test databases with Docker
docker-compose -f docker/test-compose.yml up -d

# Run integration tests
cargo test --test integration_tests

# Cleanup
docker-compose -f docker/test-compose.yml down
```

### Test Configuration

```rust
use database_security_integration::SecureDatabaseConfig;

let test_config = SecureDatabaseConfig::test_config();
let db_manager = SecureDatabaseManager::with_config(test_config).await?;
```

## 📈 Performance

### Optimization Tips

1. **Enable Connection Pooling**:
   ```rust
   config.performance.enable_connection_pooling = true;
   config.database.pool.max_connections = 50;
   ```

2. **Use Permission Caching**:
   ```rust
   config.access_control.enable_permission_caching = true;
   config.access_control.cache_ttl_seconds = 300;
   ```

3. **Optimize Encryption**:
   ```rust
   config.encryption.enable_caching = true;
   config.encryption.max_cache_size = 10000;
   ```

4. **Batch Audit Events**:
   ```rust
   config.audit.buffer_size = 1000;
   config.audit.flush_interval_seconds = 60;
   ```

### Benchmarks

Run performance benchmarks:

```bash
cargo bench
```

Expected performance characteristics:
- **Authentication**: <5ms per request
- **Permission Checks**: <1ms per check (cached)
- **Encryption/Decryption**: <10ms per field
- **Database Operations**: <50ms per query (depends on complexity)

## 🔒 Security Considerations

### Production Deployment

1. **Use Strong JWT Secrets**: Minimum 32 characters, cryptographically secure
2. **Enable TLS**: All database connections should use TLS/SSL
3. **Regular Key Rotation**: Implement automatic key rotation (90-day cycles)
4. **Audit Log Protection**: Store audit logs in append-only, tamper-evident storage
5. **Network Security**: Use VPC/private networks for database connections

### Security Best Practices

```rust
// Always validate input
if !is_valid_user_id(&user_id) {
    return Err(SecureDatabaseError::validation_error("Invalid user ID"));
}

// Use parameterized queries (handled automatically)
let user = repo.get_user_with_permissions(&context, &user_id).await?;

// Implement rate limiting
if !rate_limiter.check_rate(&context.user_id).await? {
    return Err(SecureDatabaseError::rate_limit_exceeded("Too many requests"));
}

// Log security-relevant events
audit_logger.log_security_event(&context, "admin_access", "medium", "Admin panel accessed", details).await;
```

## 🐛 Troubleshooting

### Common Issues

**Connection Errors**:
```rust
// Enable connection validation
config.database.pool.test_on_borrow = true;
config.database.pool.validation_query = "SELECT 1".to_string();
```

**Permission Denied**:
```rust
// Check security context
println!("User permissions: {:?}", context.permissions);
println!("Required permission: user:read");

// Clear permission cache if needed
access_control.clear_user_cache(&context.user_id).await;
```

**Encryption Failures**:
```rust
// Check encryption configuration
if !config.encryption.enabled {
    println!("Warning: Encryption is disabled");
}

// Verify key rotation status
if data_encryption.needs_key_rotation().await {
    data_encryption.rotate_keys().await?;
}
```

### Debug Mode

```rust
// Enable debug mode for detailed logging
config.features.debug_mode = true;
config.monitoring.enable_tracing = true;
config.monitoring.tracing_sample_rate = 1.0;
```

## 📚 API Reference

### Core Types

- [`SecureDatabaseManager`](src/lib.rs) - Main entry point for secure database operations
- [`SecurityContext`](src/security_context.rs) - Authentication and authorization context
- [`SecurePostgresRepository`](src/secure_repositories.rs) - Secure PostgreSQL operations
- [`DataEncryption`](src/encryption_integration.rs) - Data encryption/decryption
- [`AuditLogger`](src/audit.rs) - Comprehensive audit logging
- [`DatabaseAccessControl`](src/access_control.rs) - Permission and role management

### Error Handling

All operations return `Result<T, SecureDatabaseError>` with comprehensive error types:

```rust
match result {
    Err(SecureDatabaseError::AccessDenied(msg)) => {
        // Handle access denied
    }
    Err(SecureDatabaseError::MfaRequired(msg)) => {
        // Prompt for MFA
    }
    Err(SecureDatabaseError::DatabaseOperation(msg)) => {
        // Handle database error
    }
    Ok(data) => {
        // Process successful result
    }
}
```

## 🤝 Contributing

### Development Setup

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/ai-core/ai-core
   cd AI-CORE/integrations/database-security
   ```

2. **Install Dependencies**:
   ```bash
   cargo build
   ```

3. **Start Test Environment**:
   ```bash
   docker-compose -f docker/dev-compose.yml up -d
   ```

4. **Run Tests**:
   ```bash
   cargo test
   ```

### Code Style

Follow Rust standard formatting:
```bash
cargo fmt
cargo clippy
```

### Pull Request Process

1. Create a feature branch from `main`
2. Write comprehensive tests for new functionality
3. Update documentation and examples
4. Ensure all tests pass and code is formatted
5. Submit pull request with detailed description

## 📄 License

This project is licensed under the MIT OR Apache-2.0 License - see the [LICENSE](LICENSE) files for details.

## 🙏 Acknowledgments

- [Axum](https://github.com/tokio-rs/axum) for the web framework
- [SQLx](https://github.com/launchbadge/sqlx) for async SQL toolkit
- [MongoDB Rust Driver](https://github.com/mongodb/mongo-rust-driver) for MongoDB support
- [Redis-rs](https://github.com/redis-rs/redis-rs) for Redis integration
- [Ring](https://github.com/briansmith/ring) for cryptographic operations

---

**🔐 Secure by Design | 🚀 Production Ready | 📊 Observable | 🧪 Well Tested**
