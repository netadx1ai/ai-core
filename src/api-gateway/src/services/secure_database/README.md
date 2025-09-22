# Secure Database Integration

This module provides a secure integration layer between the security and database services in the AI-PLATFORM platform. It implements secure database access patterns that enforce authentication, authorization, audit logging, and data encryption.

## Features

- **Authentication Validation**: Ensures all database operations are performed by authenticated users
- **Role-Based Access Control**: Enforces authorization for all database operations
- **Field-Level Encryption**: Automatically encrypts sensitive data fields
- **Comprehensive Audit Logging**: Tracks all database operations for security and compliance
- **Security Context Propagation**: Maintains security context across services
- **Self-Access Protection**: Allows users to access their own data even with limited permissions

## Components

### SecureRepositoryWrapper

The `SecureRepositoryWrapper` provides a secure layer around any repository that implements the `Repository` trait. It intercepts all database operations to enforce security policies:

```rust
pub struct SecureRepositoryWrapper<T> {
    repository: Arc<T>,
    security_service: Arc<SecurityService>,
    audit_logger: Arc<InMemoryAuditLogger>,
    encryption_service: Arc<EncryptionService>,
    config: SecureDatabaseConfig,
    authorization_cache: Arc<RwLock<HashMap<String, (bool, DateTime<Utc>)>>>,
}
```

### SecurityContext

The `SecurityContext` encapsulates the security information for the current operation, including:

```rust
pub struct SecurityContext {
    pub user_id: Uuid,
    pub session_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<Permission>,
    pub subscription_tier: SubscriptionTier,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### AuditTrail

The `AuditTrail` records comprehensive information about database operations:

```rust
pub struct AuditTrail {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Option<String>,
    pub operation: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub execution_time_ms: u64,
}
```

### SecureTransactionManager

The `SecureTransactionManager` provides transaction-level security controls:

```rust
pub struct SecureTransactionManager {
    database_manager: Arc<DatabaseManager>,
    security_service: Arc<SecurityService>,
    audit_logger: Arc<InMemoryAuditLogger>,
}
```

## Usage Example

```rust
use crate::services::secure_database::{SecureRepositoryWrapper, SecurityContext};
use ai_core_security::jwt::ValidationResult;

// Create security context from JWT validation
let validation_result: ValidationResult = jwt_service.validate_token(token).await?;
let security_context = SecurityContext::from_validation_result(&validation_result);

// Create secure repository wrapper
let repository = user_repository_factory.create_user_repository();
let secure_repo = SecureRepositoryWrapper::new(
    Arc::new(repository),
    security_service,
    security_service.encryption(),
);

// All operations now enforce security policies
let user = secure_repo.find_by_id((security_context, user_id)).await?;
```

## Security Flow

1. **Authentication** - Validate the user's identity through JWT tokens
2. **Authorization** - Check permissions for the requested operation using RBAC
3. **Encryption** - Automatically encrypt sensitive fields before storage
4. **Audit Logging** - Record all operations with success/failure status
5. **Decryption** - Decrypt sensitive fields when returning data to authorized users

## Configuration Options

The `SecureDatabaseConfig` provides options for customizing security behavior:

```rust
pub struct SecureDatabaseConfig {
    pub enable_audit_logging: bool,
    pub enable_field_encryption: bool,
    pub enable_authorization_cache: bool,
    pub audit_sensitive_operations_only: bool,
    pub encrypted_fields: Vec<String>,
    pub cache_ttl_seconds: u64,
    pub max_audit_batch_size: u32,
}
```

## Integration with Other Services

- **Security Service** - Provides JWT validation, RBAC, and encryption
- **Database Service** - Provides repository implementations
- **API Gateway** - Uses the secure database integration for all endpoints
- **Backend Services** - Each service uses the secure database integration for its specific domain

## Best Practices

1. **Always use security context** - Never access databases without a valid security context
2. **Use field-level encryption** - Always encrypt sensitive data fields
3. **Implement proper audit logging** - Log all security-relevant operations
4. **Cache authorization decisions** - Use authorization caching for performance
5. **Implement self-access permissions** - Allow users to access their own data with proper checks

## Performance Considerations

- **Authorization Caching** - Caches authorization decisions to reduce overhead
- **Selective Audit Logging** - Option to log only sensitive operations
- **Batched Audit Events** - Batches audit events for efficiency
- **Efficient Encryption** - Uses optimized cryptographic algorithms

## Further Resources

- [Security Service Documentation](../../security/README.md)
- [Database Service Documentation](../../database/README.md)
- [API Gateway Documentation](../../api-gateway/README.md)
- [Authentication Flow](../../security/jwt/README.md)
- [Authorization Framework](../../security/rbac/README.md)
