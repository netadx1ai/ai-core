//! RBAC/ABAC Authorization Service
//!
//! Provides role-based access control (RBAC) and attribute-based access control (ABAC)
//! with hierarchical permissions, caching, and fine-grained authorization policies.

use crate::errors::{SecurityError, SecurityResult};
use ai_core_shared::types::Permission;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Authorization context for ABAC decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationContext {
    /// User making the request
    pub user_id: Uuid,
    /// Resource being accessed
    pub resource: String,
    /// Action being performed
    pub action: String,
    /// Additional attributes for ABAC
    pub attributes: HashMap<String, serde_json::Value>,
    /// Request metadata
    pub request_metadata: RequestMetadata,
}

/// Request metadata for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Geolocation data
    pub geolocation: Option<GeolocationData>,
}

/// Geolocation data for location-based authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationData {
    pub country: String,
    pub region: String,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Role definition with hierarchical support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub permissions: HashSet<Permission>,
    pub parent_roles: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Permission policy for ABAC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub id: Uuid,
    pub name: String,
    pub resource_pattern: String,
    pub action_pattern: String,
    pub conditions: Vec<PolicyCondition>,
    pub effect: PolicyEffect,
    pub priority: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Policy condition for ABAC evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    pub attribute: String,
    pub operator: ComparisonOperator,
    pub value: serde_json::Value,
    pub condition_type: ConditionType,
}

/// Comparison operators for policy conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    #[serde(rename = "eq")]
    Equal,
    #[serde(rename = "ne")]
    NotEqual,
    #[serde(rename = "gt")]
    GreaterThan,
    #[serde(rename = "lt")]
    LessThan,
    #[serde(rename = "gte")]
    GreaterThanOrEqual,
    #[serde(rename = "lte")]
    LessThanOrEqual,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "not_in")]
    NotIn,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "regex")]
    Regex,
}

/// Condition type for logical operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
    #[serde(rename = "not")]
    Not,
}

/// Policy effect (allow or deny)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "deny")]
    Deny,
}

/// Authorization decision result
#[derive(Debug, Clone)]
pub struct AuthorizationDecision {
    pub allowed: bool,
    pub reason: String,
    pub matched_policies: Vec<String>,
    pub evaluation_time_ms: u64,
    pub cached: bool,
}

/// Permission cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermissionCacheEntry {
    user_id: Uuid,
    resource: String,
    action: String,
    allowed: bool,
    reason: String,
    cached_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

/// Role repository trait for database operations
#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn get_user_roles(&self, user_id: Uuid) -> SecurityResult<Vec<Role>>;
    async fn get_role_by_name(&self, name: &str) -> SecurityResult<Option<Role>>;
    async fn create_role(&self, role: &Role) -> SecurityResult<()>;
    async fn update_role(&self, role: &Role) -> SecurityResult<()>;
    async fn delete_role(&self, role_id: Uuid) -> SecurityResult<()>;
    async fn get_role_hierarchy(&self, role_name: &str) -> SecurityResult<Vec<Role>>;
}

/// Permission cache trait for caching operations
#[async_trait]
pub trait PermissionCache: Send + Sync {
    async fn get(&self, key: &str) -> SecurityResult<Option<bool>>;
    async fn set(&self, key: &str, value: bool, ttl: Duration) -> SecurityResult<()>;
    async fn invalidate(&self, pattern: &str) -> SecurityResult<()>;
    async fn invalidate_user(&self, user_id: Uuid) -> SecurityResult<()>;
}

/// Redis-based permission cache implementation
pub struct RedisPermissionCache {
    client: Arc<redis::Client>,
    cache_prefix: String,
}

impl RedisPermissionCache {
    pub fn new(client: Arc<redis::Client>) -> Self {
        Self {
            client,
            cache_prefix: "auth:cache:".to_string(),
        }
    }

    fn cache_key(&self, user_id: Uuid, resource: &str, action: &str) -> String {
        format!("{}{}:{}:{}", self.cache_prefix, user_id, resource, action)
    }
}

#[async_trait]
impl PermissionCache for RedisPermissionCache {
    async fn get(&self, key: &str) -> SecurityResult<Option<bool>> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SecurityError::CacheConnection(e.to_string()))?;

        let entry_data: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;

        if let Some(data) = entry_data {
            let entry: PermissionCacheEntry = serde_json::from_str(&data)
                .map_err(|e| SecurityError::CacheSerialization(e.to_string()))?;

            if entry.expires_at > Utc::now() {
                return Ok(Some(entry.allowed));
            } else {
                // Remove expired entry
                let _: () = conn
                    .del(key)
                    .await
                    .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;
            }
        }

        Ok(None)
    }

    async fn set(&self, key: &str, value: bool, ttl: Duration) -> SecurityResult<()> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SecurityError::CacheConnection(e.to_string()))?;

        let entry = PermissionCacheEntry {
            user_id: Uuid::new_v4(), // This would be extracted from key in practice
            resource: String::new(),
            action: String::new(),
            allowed: value,
            reason: "cached".to_string(),
            cached_at: Utc::now(),
            expires_at: Utc::now() + ttl,
        };

        let entry_json = serde_json::to_string(&entry)
            .map_err(|e| SecurityError::CacheSerialization(e.to_string()))?;

        let ttl_seconds = ttl.num_seconds().max(0) as u64;

        conn.set_ex::<_, _, ()>(key, entry_json, ttl_seconds)
            .await
            .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;

        Ok(())
    }

    async fn invalidate(&self, pattern: &str) -> SecurityResult<()> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| SecurityError::CacheConnection(e.to_string()))?;

        let keys: Vec<String> = conn
            .keys(pattern)
            .await
            .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;

        if !keys.is_empty() {
            conn.del::<_, ()>(&keys)
                .await
                .map_err(|e| SecurityError::CacheOperation(e.to_string()))?;
        }

        Ok(())
    }

    async fn invalidate_user(&self, user_id: Uuid) -> SecurityResult<()> {
        let pattern = format!("{}{}:*", self.cache_prefix, user_id);
        self.invalidate(&pattern).await
    }
}

/// Main RBAC service implementation
pub struct RbacService {
    role_repository: Arc<dyn RoleRepository>,
    permission_cache: Arc<dyn PermissionCache>,
    policies: Arc<RwLock<Vec<PermissionPolicy>>>,
    config: RbacConfig,
    role_hierarchy_cache: Arc<DashMap<String, Vec<Role>>>,
}

/// RBAC service configuration
#[derive(Debug, Clone)]
pub struct RbacConfig {
    pub enable_rbac: bool,
    pub enable_abac: bool,
    pub cache_ttl: Duration,
    pub admin_override: bool,
    pub evaluation_mode: PermissionEvaluationMode,
    pub max_policy_evaluation_time_ms: u64,
}

/// Permission evaluation mode
#[derive(Debug, Clone)]
pub enum PermissionEvaluationMode {
    Strict,     // Deny by default, explicit allow required
    Permissive, // Allow by default, explicit deny required
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            enable_rbac: true,
            enable_abac: false,
            cache_ttl: Duration::minutes(15),
            admin_override: true,
            evaluation_mode: PermissionEvaluationMode::Strict,
            max_policy_evaluation_time_ms: 100,
        }
    }
}

impl RbacService {
    /// Create a new RBAC service
    pub fn new(
        role_repository: Arc<dyn RoleRepository>,
        permission_cache: Arc<dyn PermissionCache>,
        config: RbacConfig,
    ) -> Self {
        Self {
            role_repository,
            permission_cache,
            policies: Arc::new(RwLock::new(Vec::new())),
            config,
            role_hierarchy_cache: Arc::new(DashMap::new()),
        }
    }

    /// Check if user has permission for resource and action
    pub async fn check_permission(
        &self,
        user_id: Uuid,
        resource: &str,
        action: &str,
    ) -> SecurityResult<bool> {
        let context = AuthorizationContext {
            user_id,
            resource: resource.to_string(),
            action: action.to_string(),
            attributes: HashMap::new(),
            request_metadata: RequestMetadata {
                client_ip: None,
                user_agent: None,
                timestamp: Utc::now(),
                request_id: None,
                geolocation: None,
            },
        };

        let decision = self.authorize(&context).await?;
        Ok(decision.allowed)
    }

    /// Comprehensive authorization with context
    pub async fn authorize(
        &self,
        context: &AuthorizationContext,
    ) -> SecurityResult<AuthorizationDecision> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let cache_key = format!(
            "{}:{}:{}",
            context.user_id, context.resource, context.action
        );
        if let Some(cached_result) = self.permission_cache.get(&cache_key).await? {
            debug!("Authorization cache hit for {}", cache_key);
            return Ok(AuthorizationDecision {
                allowed: cached_result,
                reason: "cached_decision".to_string(),
                matched_policies: vec![],
                evaluation_time_ms: start_time.elapsed().as_millis() as u64,
                cached: true,
            });
        }

        let mut decision = AuthorizationDecision {
            allowed: false,
            reason: "default_deny".to_string(),
            matched_policies: vec![],
            evaluation_time_ms: 0,
            cached: false,
        };

        // RBAC evaluation
        if self.config.enable_rbac {
            let rbac_result = self.evaluate_rbac(context).await?;
            if rbac_result.allowed {
                decision.allowed = true;
                decision.reason = rbac_result.reason;
                decision
                    .matched_policies
                    .extend(rbac_result.matched_policies);
            }
        }

        // ABAC evaluation (if enabled and RBAC didn't grant access)
        if self.config.enable_abac && !decision.allowed {
            let abac_result = self.evaluate_abac(context).await?;
            if abac_result.allowed {
                decision.allowed = true;
                decision.reason = abac_result.reason;
                decision
                    .matched_policies
                    .extend(abac_result.matched_policies);
            }
        }

        // Admin override check
        if self.config.admin_override && !decision.allowed {
            if self.is_admin_user(context.user_id).await? {
                decision.allowed = true;
                decision.reason = "admin_override".to_string();
                decision.matched_policies.push("admin_override".to_string());
            }
        }

        decision.evaluation_time_ms = start_time.elapsed().as_millis() as u64;

        // Cache the decision
        self.permission_cache
            .set(&cache_key, decision.allowed, self.config.cache_ttl)
            .await?;

        info!(
            "Authorization decision: user={}, resource={}, action={}, allowed={}, reason={}, time={}ms",
            context.user_id, context.resource, context.action,
            decision.allowed, decision.reason, decision.evaluation_time_ms
        );

        Ok(decision)
    }

    /// Evaluate RBAC permissions
    async fn evaluate_rbac(
        &self,
        context: &AuthorizationContext,
    ) -> SecurityResult<AuthorizationDecision> {
        let user_roles = self.role_repository.get_user_roles(context.user_id).await?;

        if user_roles.is_empty() {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: "no_roles_assigned".to_string(),
                matched_policies: vec![],
                evaluation_time_ms: 0,
                cached: false,
            });
        }

        // Collect all permissions from roles and their hierarchies
        let mut all_permissions = HashSet::new();
        let mut matched_roles = Vec::new();

        for role in &user_roles {
            if !role.is_active {
                continue;
            }

            // Add direct permissions
            all_permissions.extend(role.permissions.iter().cloned());
            matched_roles.push(role.name.clone());

            // Add permissions from parent roles
            let hierarchy = self.get_role_hierarchy(&role.name).await?;
            for parent_role in hierarchy {
                if parent_role.is_active {
                    all_permissions.extend(parent_role.permissions.iter().cloned());
                    matched_roles.push(parent_role.name.clone());
                }
            }
        }

        // Check if required permission exists
        let required_permission = self.construct_permission(&context.resource, &context.action);
        if let Some(permission) = required_permission {
            if all_permissions.contains(&permission) {
                return Ok(AuthorizationDecision {
                    allowed: true,
                    reason: format!("rbac_permission_granted_{:?}", permission),
                    matched_policies: matched_roles,
                    evaluation_time_ms: 0,
                    cached: false,
                });
            }
        }

        // Check wildcard permissions
        if self.check_wildcard_permissions(&all_permissions, &context.resource, &context.action) {
            return Ok(AuthorizationDecision {
                allowed: true,
                reason: "rbac_wildcard_permission".to_string(),
                matched_policies: matched_roles,
                evaluation_time_ms: 0,
                cached: false,
            });
        }

        Ok(AuthorizationDecision {
            allowed: false,
            reason: "rbac_permission_denied".to_string(),
            matched_policies: vec![],
            evaluation_time_ms: 0,
            cached: false,
        })
    }

    /// Evaluate ABAC policies
    async fn evaluate_abac(
        &self,
        context: &AuthorizationContext,
    ) -> SecurityResult<AuthorizationDecision> {
        let policies = self.policies.read().await;
        let mut matched_policies = Vec::new();
        let mut final_decision = match self.config.evaluation_mode {
            PermissionEvaluationMode::Strict => false,
            PermissionEvaluationMode::Permissive => true,
        };

        let mut policy_matches: Vec<(&PermissionPolicy, bool)> = Vec::new();

        for policy in policies.iter() {
            if !policy.is_active {
                continue;
            }

            if self.policy_matches_request(policy, context).await? {
                policy_matches.push((policy, matches!(policy.effect, PolicyEffect::Allow)));
                matched_policies.push(policy.name.clone());
            }
        }

        // Sort by priority (higher priority first)
        policy_matches.sort_by(|a, b| b.0.priority.cmp(&a.0.priority));

        // Apply policies in priority order
        for (policy, is_allow) in policy_matches {
            match policy.effect {
                PolicyEffect::Allow if is_allow => {
                    final_decision = true;
                    break; // First allow wins
                }
                PolicyEffect::Deny if is_allow => {
                    final_decision = false;
                    break; // First deny wins
                }
                _ => continue,
            }
        }

        let reason = if final_decision {
            "abac_policy_allow".to_string()
        } else {
            "abac_policy_deny".to_string()
        };

        Ok(AuthorizationDecision {
            allowed: final_decision,
            reason,
            matched_policies,
            evaluation_time_ms: 0,
            cached: false,
        })
    }

    /// Check if policy matches the request
    async fn policy_matches_request(
        &self,
        policy: &PermissionPolicy,
        context: &AuthorizationContext,
    ) -> SecurityResult<bool> {
        // Check resource pattern
        if !self.matches_pattern(&policy.resource_pattern, &context.resource) {
            return Ok(false);
        }

        // Check action pattern
        if !self.matches_pattern(&policy.action_pattern, &context.action) {
            return Ok(false);
        }

        // Evaluate conditions
        for condition in &policy.conditions {
            if !self.evaluate_condition(condition, context).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Evaluate a single policy condition
    async fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        context: &AuthorizationContext,
    ) -> SecurityResult<bool> {
        let attribute_value = self
            .get_attribute_value(&condition.attribute, context)
            .await?;

        let result = match condition.operator {
            ComparisonOperator::Equal => attribute_value == condition.value,
            ComparisonOperator::NotEqual => attribute_value != condition.value,
            ComparisonOperator::GreaterThan => {
                self.compare_numeric(&attribute_value, &condition.value, |a, b| a > b)?
            }
            ComparisonOperator::LessThan => {
                self.compare_numeric(&attribute_value, &condition.value, |a, b| a < b)?
            }
            ComparisonOperator::GreaterThanOrEqual => {
                self.compare_numeric(&attribute_value, &condition.value, |a, b| a >= b)?
            }
            ComparisonOperator::LessThanOrEqual => {
                self.compare_numeric(&attribute_value, &condition.value, |a, b| a <= b)?
            }
            ComparisonOperator::In => {
                if let serde_json::Value::Array(values) = &condition.value {
                    values.contains(&attribute_value)
                } else {
                    false
                }
            }
            ComparisonOperator::NotIn => {
                if let serde_json::Value::Array(values) = &condition.value {
                    !values.contains(&attribute_value)
                } else {
                    true
                }
            }
            ComparisonOperator::Contains => {
                if let (serde_json::Value::String(haystack), serde_json::Value::String(needle)) =
                    (&attribute_value, &condition.value)
                {
                    haystack.contains(needle)
                } else {
                    false
                }
            }
            ComparisonOperator::Regex => {
                if let (serde_json::Value::String(text), serde_json::Value::String(pattern)) =
                    (&attribute_value, &condition.value)
                {
                    regex::Regex::new(pattern)
                        .map_err(|e| SecurityError::Configuration(format!("Invalid regex: {}", e)))?
                        .is_match(text)
                } else {
                    false
                }
            }
        };

        Ok(result)
    }

    /// Get attribute value from context
    async fn get_attribute_value(
        &self,
        attribute: &str,
        context: &AuthorizationContext,
    ) -> SecurityResult<serde_json::Value> {
        match attribute {
            "user_id" => Ok(serde_json::Value::String(context.user_id.to_string())),
            "resource" => Ok(serde_json::Value::String(context.resource.clone())),
            "action" => Ok(serde_json::Value::String(context.action.clone())),
            "timestamp" => Ok(serde_json::Value::String(
                context.request_metadata.timestamp.to_rfc3339(),
            )),
            "client_ip" => Ok(context
                .request_metadata
                .client_ip
                .as_ref()
                .map(|ip| serde_json::Value::String(ip.clone()))
                .unwrap_or(serde_json::Value::Null)),
            _ => {
                // Check custom attributes
                Ok(context
                    .attributes
                    .get(attribute)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null))
            }
        }
    }

    /// Compare numeric values
    fn compare_numeric<F>(
        &self,
        a: &serde_json::Value,
        b: &serde_json::Value,
        op: F,
    ) -> SecurityResult<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        let a_num = a.as_f64().ok_or_else(|| {
            SecurityError::InvalidInputFormat("Expected numeric value".to_string())
        })?;
        let b_num = b.as_f64().ok_or_else(|| {
            SecurityError::InvalidInputFormat("Expected numeric value".to_string())
        })?;

        Ok(op(a_num, b_num))
    }

    /// Check if pattern matches value (supports wildcards)
    fn matches_pattern(&self, pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            let regex_pattern = pattern.replace('*', ".*");
            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return regex.is_match(value);
            }
        }

        pattern == value
    }

    /// Get role hierarchy for a role
    async fn get_role_hierarchy(&self, role_name: &str) -> SecurityResult<Vec<Role>> {
        // Check cache first
        if let Some(cached_hierarchy) = self.role_hierarchy_cache.get(role_name) {
            return Ok(cached_hierarchy.clone());
        }

        // Fetch from repository
        let hierarchy = self.role_repository.get_role_hierarchy(role_name).await?;

        // Cache the result
        self.role_hierarchy_cache
            .insert(role_name.to_string(), hierarchy.clone());

        Ok(hierarchy)
    }

    /// Check if user is admin
    async fn is_admin_user(&self, user_id: Uuid) -> SecurityResult<bool> {
        let user_roles = self.role_repository.get_user_roles(user_id).await?;
        Ok(user_roles
            .iter()
            .any(|role| role.name == "admin" || role.name == "super_admin"))
    }

    /// Construct permission from resource and action
    fn construct_permission(&self, resource: &str, action: &str) -> Option<Permission> {
        let permission_str = format!("{}_{}", resource, action);
        permission_str.parse().ok()
    }

    /// Check wildcard permissions
    fn check_wildcard_permissions(
        &self,
        permissions: &HashSet<Permission>,
        _resource: &str,
        _action: &str,
    ) -> bool {
        // Check for admin permissions that grant access to everything
        permissions.contains(&Permission::AdminSystem)
    }

    /// Add ABAC policy
    pub async fn add_policy(&self, policy: PermissionPolicy) -> SecurityResult<()> {
        let mut policies = self.policies.write().await;
        policies.push(policy);
        Ok(())
    }

    /// Remove ABAC policy
    pub async fn remove_policy(&self, policy_id: Uuid) -> SecurityResult<()> {
        let mut policies = self.policies.write().await;
        policies.retain(|p| p.id != policy_id);
        Ok(())
    }

    /// Invalidate cache for user
    pub async fn invalidate_user_cache(&self, user_id: Uuid) -> SecurityResult<()> {
        self.permission_cache.invalidate_user(user_id).await?;

        // Also clear role hierarchy cache if user roles changed
        // In practice, you might want to be more selective about this
        self.role_hierarchy_cache.clear();

        Ok(())
    }

    /// Get authorization statistics
    pub async fn get_authorization_stats(&self) -> SecurityResult<HashMap<String, u64>> {
        let mut stats = HashMap::new();
        stats.insert("cached_permissions".to_string(), 0); // Would query cache size
        stats.insert(
            "active_policies".to_string(),
            self.policies.read().await.len() as u64,
        );
        stats.insert(
            "cached_hierarchies".to_string(),
            self.role_hierarchy_cache.len() as u64,
        );
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mock repository for testing
    struct MockRoleRepository {
        roles: Arc<Mutex<HashMap<Uuid, Vec<Role>>>>,
    }

    impl MockRoleRepository {
        fn new() -> Self {
            Self {
                roles: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn add_user_role(&self, user_id: Uuid, role: Role) {
            let mut roles = self.roles.lock().unwrap();
            roles.entry(user_id).or_insert_with(Vec::new).push(role);
        }
    }

    #[async_trait]
    impl RoleRepository for MockRoleRepository {
        async fn get_user_roles(&self, user_id: Uuid) -> SecurityResult<Vec<Role>> {
            let roles = self.roles.lock().unwrap();
            Ok(roles.get(&user_id).cloned().unwrap_or_default())
        }

        async fn get_role_by_name(&self, _name: &str) -> SecurityResult<Option<Role>> {
            Ok(None)
        }

        async fn create_role(&self, _role: &Role) -> SecurityResult<()> {
            Ok(())
        }

        async fn update_role(&self, _role: &Role) -> SecurityResult<()> {
            Ok(())
        }

        async fn delete_role(&self, _role_id: Uuid) -> SecurityResult<()> {
            Ok(())
        }

        async fn get_role_hierarchy(&self, _role_name: &str) -> SecurityResult<Vec<Role>> {
            Ok(Vec::new())
        }
    }

    // Mock cache for testing
    struct MockPermissionCache {
        cache: Arc<Mutex<HashMap<String, bool>>>,
    }

    impl MockPermissionCache {
        fn new() -> Self {
            Self {
                cache: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl PermissionCache for MockPermissionCache {
        async fn get(&self, key: &str) -> SecurityResult<Option<bool>> {
            let cache = self.cache.lock().unwrap();
            Ok(cache.get(key).copied())
        }

        async fn set(&self, key: &str, value: bool, _ttl: Duration) -> SecurityResult<()> {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(key.to_string(), value);
            Ok(())
        }

        async fn invalidate(&self, _pattern: &str) -> SecurityResult<()> {
            let mut cache = self.cache.lock().unwrap();
            cache.clear();
            Ok(())
        }

        async fn invalidate_user(&self, _user_id: Uuid) -> SecurityResult<()> {
            let mut cache = self.cache.lock().unwrap();
            cache.clear();
            Ok(())
        }
    }

    fn create_test_role() -> Role {
        Role {
            id: Uuid::new_v4(),
            name: "test_role".to_string(),
            description: "Test role".to_string(),
            permissions: [Permission::WorkflowsRead, Permission::ContentRead]
                .iter()
                .cloned()
                .collect(),
            parent_roles: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_active: true,
        }
    }

    #[tokio::test]
    async fn test_rbac_permission_check() {
        let repository = Arc::new(MockRoleRepository::new());
        let cache = Arc::new(MockPermissionCache::new());
        let config = RbacConfig::default();
        let rbac = RbacService::new(repository.clone(), cache, config);

        let user_id = Uuid::new_v4();
        let role = create_test_role();
        repository.add_user_role(user_id, role);

        let result = rbac
            .check_permission(user_id, "workflows", "read")
            .await
            .unwrap();
        assert!(result);

        let result = rbac
            .check_permission(user_id, "workflows", "delete")
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_authorization_context() {
        let context = AuthorizationContext {
            user_id: Uuid::new_v4(),
            resource: "workflows".to_string(),
            action: "read".to_string(),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert(
                    "department".to_string(),
                    serde_json::Value::String("engineering".to_string()),
                );
                attrs
            },
            request_metadata: RequestMetadata {
                client_ip: Some("127.0.0.1".to_string()),
                user_agent: Some("test-agent".to_string()),
                timestamp: Utc::now(),
                request_id: Some("req-123".to_string()),
                geolocation: None,
            },
        };

        assert_eq!(context.resource, "workflows");
        assert_eq!(context.action, "read");
        assert!(context.attributes.contains_key("department"));
    }

    #[test]
    fn test_pattern_matching() {
        let repository = Arc::new(MockRoleRepository::new());
        let cache = Arc::new(MockPermissionCache::new());
        let config = RbacConfig::default();
        let rbac = RbacService::new(repository, cache, config);

        assert!(rbac.matches_pattern("*", "anything"));
        assert!(rbac.matches_pattern("workflows", "workflows"));
        assert!(rbac.matches_pattern("work*", "workflows"));
        assert!(!rbac.matches_pattern("content", "workflows"));
    }

    #[tokio::test]
    async fn test_policy_conditions() {
        let repository = Arc::new(MockRoleRepository::new());
        let cache = Arc::new(MockPermissionCache::new());
        let config = RbacConfig::default();
        let rbac = RbacService::new(repository, cache, config);

        let context = AuthorizationContext {
            user_id: Uuid::new_v4(),
            resource: "workflows".to_string(),
            action: "read".to_string(),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert(
                    "score".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(85)),
                );
                attrs
            },
            request_metadata: RequestMetadata {
                client_ip: None,
                user_agent: None,
                timestamp: Utc::now(),
                request_id: None,
                geolocation: None,
            },
        };

        let condition = PolicyCondition {
            attribute: "score".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: serde_json::Value::Number(serde_json::Number::from(80)),
            condition_type: ConditionType::And,
        };

        let result = rbac.evaluate_condition(&condition, &context).await.unwrap();
        assert!(result);
    }
}
