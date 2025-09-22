//! Audit Module
//!
//! Provides comprehensive security audit logging and monitoring capabilities.

use crate::errors::SecurityResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditLevel {
    /// Informational events
    Info,
    /// Warning events
    Warn,
    /// Error events
    Error,
    /// Critical security events
    Critical,
}

/// Security event types for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEvent {
    /// Authentication events
    Authentication {
        user_id: Option<String>,
        success: bool,
        method: String,
        client_ip: Option<String>,
    },
    /// Authorization events
    Authorization {
        user_id: String,
        resource: String,
        action: String,
        granted: bool,
        reason: Option<String>,
    },
    /// Encryption/Decryption events
    Encryption {
        operation: String,
        algorithm: String,
        key_id: String,
        success: bool,
    },
    /// Key management events
    KeyManagement {
        operation: String,
        key_id: String,
        algorithm: Option<String>,
        success: bool,
    },
    /// Security policy violations
    PolicyViolation {
        policy: String,
        details: String,
        severity: AuditLevel,
    },
    /// Rate limiting events
    RateLimit {
        client_ip: String,
        endpoint: String,
        limit_exceeded: bool,
    },
    /// System events
    System {
        event: String,
        details: HashMap<String, String>,
    },
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique event ID
    pub id: String,
    /// Timestamp when event occurred
    pub timestamp: DateTime<Utc>,
    /// Event severity level
    pub level: AuditLevel,
    /// Security event details
    pub event: SecurityEvent,
    /// Request ID for correlation
    pub request_id: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Additional context information
    pub context: HashMap<String, String>,
}

impl AuditLogEntry {
    /// Create a new audit log entry
    pub fn new(level: AuditLevel, event: SecurityEvent) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            event,
            request_id: None,
            user_agent: None,
            context: HashMap::new(),
        }
    }

    /// Add request ID for correlation
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Add user agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Add context information
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Audit logger trait for dependency injection
#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log an audit event
    async fn log(&self, entry: AuditLogEntry) -> SecurityResult<()>;

    /// Retrieve audit logs with optional filters
    async fn get_logs(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        level: Option<AuditLevel>,
        limit: Option<usize>,
    ) -> SecurityResult<Vec<AuditLogEntry>>;

    /// Count audit logs matching criteria
    async fn count_logs(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        level: Option<AuditLevel>,
    ) -> SecurityResult<u64>;

    /// Clean up old audit logs
    async fn cleanup_old_logs(&self, older_than: DateTime<Utc>) -> SecurityResult<u64>;
}

/// In-memory audit logger implementation
pub struct InMemoryAuditLogger {
    logs: std::sync::Arc<tokio::sync::RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
}

impl InMemoryAuditLogger {
    /// Create a new in-memory audit logger
    pub fn new(max_entries: usize) -> Self {
        Self {
            logs: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            max_entries,
        }
    }
}

#[async_trait::async_trait]
impl AuditLogger for InMemoryAuditLogger {
    async fn log(&self, entry: AuditLogEntry) -> SecurityResult<()> {
        let mut logs = self.logs.write().await;

        // Add new entry
        logs.push(entry);

        // Trim if exceeding max entries
        if logs.len() > self.max_entries {
            let to_remove = logs.len() - self.max_entries;
            logs.drain(0..to_remove);
        }

        Ok(())
    }

    async fn get_logs(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        level: Option<AuditLevel>,
        limit: Option<usize>,
    ) -> SecurityResult<Vec<AuditLogEntry>> {
        let logs = self.logs.read().await;

        let filtered: Vec<AuditLogEntry> = logs
            .iter()
            .filter(|entry| {
                // Time range filter
                if let Some(start) = start_time {
                    if entry.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = end_time {
                    if entry.timestamp > end {
                        return false;
                    }
                }

                // Level filter
                if let Some(filter_level) = level {
                    if entry.level != filter_level {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        let result = if let Some(limit) = limit {
            filtered.into_iter().take(limit).collect()
        } else {
            filtered
        };

        Ok(result)
    }

    async fn count_logs(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        level: Option<AuditLevel>,
    ) -> SecurityResult<u64> {
        let logs = self.get_logs(start_time, end_time, level, None).await?;
        Ok(logs.len() as u64)
    }

    async fn cleanup_old_logs(&self, older_than: DateTime<Utc>) -> SecurityResult<u64> {
        let mut logs = self.logs.write().await;
        let initial_count = logs.len();

        logs.retain(|entry| entry.timestamp >= older_than);

        let removed_count = initial_count - logs.len();
        Ok(removed_count as u64)
    }
}

/// Convenience functions for creating common audit events
pub mod events {
    use super::*;

    /// Create authentication success event
    pub fn auth_success(
        user_id: String,
        method: String,
        client_ip: Option<String>,
    ) -> AuditLogEntry {
        AuditLogEntry::new(
            AuditLevel::Info,
            SecurityEvent::Authentication {
                user_id: Some(user_id),
                success: true,
                method,
                client_ip,
            },
        )
    }

    /// Create authentication failure event
    pub fn auth_failure(method: String, client_ip: Option<String>) -> AuditLogEntry {
        AuditLogEntry::new(
            AuditLevel::Warn,
            SecurityEvent::Authentication {
                user_id: None,
                success: false,
                method,
                client_ip,
            },
        )
    }

    /// Create authorization granted event
    pub fn authz_granted(user_id: String, resource: String, action: String) -> AuditLogEntry {
        AuditLogEntry::new(
            AuditLevel::Info,
            SecurityEvent::Authorization {
                user_id,
                resource,
                action,
                granted: true,
                reason: None,
            },
        )
    }

    /// Create authorization denied event
    pub fn authz_denied(
        user_id: String,
        resource: String,
        action: String,
        reason: Option<String>,
    ) -> AuditLogEntry {
        AuditLogEntry::new(
            AuditLevel::Warn,
            SecurityEvent::Authorization {
                user_id,
                resource,
                action,
                granted: false,
                reason,
            },
        )
    }

    /// Create key rotation event
    pub fn key_rotated(key_id: String, algorithm: String) -> AuditLogEntry {
        AuditLogEntry::new(
            AuditLevel::Info,
            SecurityEvent::KeyManagement {
                operation: "rotate".to_string(),
                key_id,
                algorithm: Some(algorithm),
                success: true,
            },
        )
    }

    /// Create policy violation event
    pub fn policy_violation(
        policy: String,
        details: String,
        severity: AuditLevel,
    ) -> AuditLogEntry {
        AuditLogEntry::new(
            severity,
            SecurityEvent::PolicyViolation {
                policy,
                details,
                severity,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_in_memory_audit_logger() {
        let logger = InMemoryAuditLogger::new(100);

        // Log some events
        let entry1 = events::auth_success(
            "user1".to_string(),
            "password".to_string(),
            Some("192.168.1.1".to_string()),
        );
        let entry2 = events::auth_failure("password".to_string(), Some("192.168.1.2".to_string()));

        logger.log(entry1).await.unwrap();
        logger.log(entry2).await.unwrap();

        // Retrieve logs
        let logs = logger.get_logs(None, None, None, None).await.unwrap();
        assert_eq!(logs.len(), 2);

        // Filter by level
        let warn_logs = logger
            .get_logs(None, None, Some(AuditLevel::Warn), None)
            .await
            .unwrap();
        assert_eq!(warn_logs.len(), 1);

        // Count logs
        let count = logger.count_logs(None, None, None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_audit_log_entry_builder() {
        let entry = AuditLogEntry::new(
            AuditLevel::Info,
            SecurityEvent::System {
                event: "test".to_string(),
                details: HashMap::new(),
            },
        )
        .with_request_id("req123")
        .with_user_agent("test-agent")
        .with_context("key", "value");

        assert_eq!(entry.request_id, Some("req123".to_string()));
        assert_eq!(entry.user_agent, Some("test-agent".to_string()));
        assert_eq!(entry.context.get("key"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_old_logs() {
        let logger = InMemoryAuditLogger::new(100);

        // Add some logs
        logger
            .log(events::auth_success(
                "user1".to_string(),
                "password".to_string(),
                None,
            ))
            .await
            .unwrap();

        // Cleanup logs older than now (should remove all)
        let removed = logger
            .cleanup_old_logs(Utc::now() + Duration::minutes(1))
            .await
            .unwrap();
        assert_eq!(removed, 1);

        let count = logger.count_logs(None, None, None).await.unwrap();
        assert_eq!(count, 0);
    }
}
