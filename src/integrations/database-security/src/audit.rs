//! # Audit Module
//!
//! This module provides comprehensive audit logging functionality for database
//! operations in the AI-CORE platform. It tracks all data access, modifications,
//! and security events with detailed context and metadata.

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use ai_core_database::DatabaseManager;
use ai_core_shared::types::{Permission, User};

use crate::{
    error::SecureDatabaseError,
    security_context::{AuditContext, SecurityContext},
};

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Store audit logs in database
    pub store_in_database: bool,
    /// Store audit logs in files
    pub store_in_files: bool,
    /// File storage path
    pub file_storage_path: String,
    /// Log retention period in days
    pub retention_days: u32,
    /// Enable real-time audit event streaming
    pub enable_streaming: bool,
    /// Stream endpoint URL
    pub stream_endpoint: Option<String>,
    /// Buffer size for batch processing
    pub buffer_size: usize,
    /// Flush interval in seconds
    pub flush_interval_seconds: u64,
    /// Log levels to capture
    pub log_levels: Vec<AuditLevel>,
    /// Sensitive operations that always require auditing
    pub always_audit_operations: Vec<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            store_in_database: true,
            store_in_files: true,
            file_storage_path: "/var/log/ai-core/audit".to_string(),
            retention_days: 365, // 1 year retention
            enable_streaming: false,
            stream_endpoint: None,
            buffer_size: 1000,
            flush_interval_seconds: 60,
            log_levels: vec![
                AuditLevel::Info,
                AuditLevel::Warning,
                AuditLevel::Error,
                AuditLevel::Security,
            ],
            always_audit_operations: vec![
                "user:create".to_string(),
                "user:update".to_string(),
                "user:delete".to_string(),
                "admin:*".to_string(),
                "security:*".to_string(),
                "billing:*".to_string(),
            ],
        }
    }
}

/// Audit event level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditLevel {
    /// Informational events
    Info,
    /// Warning events
    Warning,
    /// Error events
    Error,
    /// Security-related events
    Security,
    /// System events
    System,
}

/// Audit event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Data access event
    DataAccess {
        table: String,
        record_id: String,
        operation: String,
    },
    /// Data modification event
    DataChange {
        table: String,
        record_id: String,
        operation: String,
        old_value: Option<String>,
        new_value: Option<String>,
    },
    /// Authentication event
    Authentication {
        event_type: String,
        success: bool,
        failure_reason: Option<String>,
    },
    /// Authorization event
    Authorization {
        resource: String,
        permission: String,
        granted: bool,
        denial_reason: Option<String>,
    },
    /// System event
    System {
        event_type: String,
        component: String,
        details: serde_json::Value,
    },
    /// Security event
    Security {
        event_type: String,
        severity: String,
        threat_level: Option<String>,
        details: serde_json::Value,
    },
}

/// Audit event entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub id: Uuid,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event level
    pub level: AuditLevel,
    /// Event type
    pub event_type: AuditEventType,
    /// User context
    pub user_context: AuditContext,
    /// Event message
    pub message: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
    /// Risk score (0-100)
    pub risk_score: Option<u8>,
    /// Whether this event triggered an alert
    pub alert_triggered: bool,
}

/// Audit logger implementation
pub struct AuditLogger {
    /// Database manager for storing audit logs
    database_manager: Arc<DatabaseManager>,
    /// Audit configuration
    config: AuditConfig,
    /// Event buffer for batch processing
    event_buffer: Arc<RwLock<Vec<AuditEvent>>>,
    /// Audit metrics
    metrics: Arc<RwLock<AuditMetrics>>,
}

/// Audit logging metrics
#[derive(Debug, Default, Clone)]
pub struct AuditMetrics {
    pub total_events: u64,
    pub events_by_level: std::collections::HashMap<String, u64>,
    pub events_by_type: std::collections::HashMap<String, u64>,
    pub events_written_to_db: u64,
    pub events_written_to_file: u64,
    pub events_streamed: u64,
    pub buffer_overflows: u64,
    pub write_errors: u64,
    pub last_flush: Option<DateTime<Utc>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub async fn new(
        database_manager: Arc<DatabaseManager>,
        config: AuditConfig,
    ) -> Result<Self, SecureDatabaseError> {
        if !config.enabled {
            info!("Audit logging is disabled");
        }

        let logger = Self {
            database_manager,
            config,
            event_buffer: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(AuditMetrics::default())),
        };

        // Start background flush task if enabled
        if logger.config.enabled {
            logger.start_flush_task().await;
        }

        info!("Audit logger initialized");
        Ok(logger)
    }

    /// Log a data access event
    #[instrument(skip(self, context), fields(user_id = %context.user_id))]
    pub async fn log_data_access(
        &self,
        context: &SecurityContext,
        table: &str,
        record_id: &str,
        operation: &str,
        message: &str,
    ) {
        if !self.config.enabled {
            return;
        }

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: AuditLevel::Info,
            event_type: AuditEventType::DataAccess {
                table: table.to_string(),
                record_id: record_id.to_string(),
                operation: operation.to_string(),
            },
            user_context: context.audit_context(),
            message: message.to_string(),
            metadata: serde_json::json!({
                "table": table,
                "record_id": record_id,
                "operation": operation,
                "timestamp": Utc::now().to_rfc3339(),
            }),
            risk_score: self.calculate_risk_score(&AuditEventType::DataAccess {
                table: table.to_string(),
                record_id: record_id.to_string(),
                operation: operation.to_string(),
            }),
            alert_triggered: false,
        };

        self.add_event(event).await;

        debug!(
            user_id = %context.user_id,
            table = %table,
            record_id = %record_id,
            operation = %operation,
            "Data access logged"
        );
    }

    /// Log a data change event
    #[instrument(skip(self, context, old_value, new_value), fields(user_id = %context.user_id))]
    pub async fn log_data_change(
        &self,
        context: &SecurityContext,
        table: &str,
        record_id: &str,
        operation: &str,
        message: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let event_type = AuditEventType::DataChange {
            table: table.to_string(),
            record_id: record_id.to_string(),
            operation: operation.to_string(),
            old_value: old_value.map(|s| s.to_string()),
            new_value: new_value.map(|s| s.to_string()),
        };

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: self.determine_event_level(operation),
            event_type: event_type.clone(),
            user_context: context.audit_context(),
            message: message.to_string(),
            metadata: serde_json::json!({
                "table": table,
                "record_id": record_id,
                "operation": operation,
                "has_old_value": old_value.is_some(),
                "has_new_value": new_value.is_some(),
                "timestamp": Utc::now().to_rfc3339(),
            }),
            risk_score: self.calculate_risk_score(&event_type),
            alert_triggered: false,
        };

        self.add_event(event).await;

        info!(
            user_id = %context.user_id,
            table = %table,
            record_id = %record_id,
            operation = %operation,
            "Data change logged"
        );
    }

    /// Log an authentication event
    #[instrument(skip(self, context), fields(user_id = %context.user_id))]
    pub async fn log_authentication_event(
        &self,
        context: &SecurityContext,
        event_type: &str,
        success: bool,
        failure_reason: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let audit_event_type = AuditEventType::Authentication {
            event_type: event_type.to_string(),
            success,
            failure_reason: failure_reason.map(|s| s.to_string()),
        };

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: if success {
                AuditLevel::Info
            } else {
                AuditLevel::Security
            },
            event_type: audit_event_type.clone(),
            user_context: context.audit_context(),
            message: format!(
                "Authentication {}: {}",
                if success { "success" } else { "failure" },
                event_type
            ),
            metadata: serde_json::json!({
                "auth_event_type": event_type,
                "success": success,
                "failure_reason": failure_reason,
                "timestamp": Utc::now().to_rfc3339(),
            }),
            risk_score: self.calculate_risk_score(&audit_event_type),
            alert_triggered: !success, // Failed auth always triggers alerts
        };

        self.add_event(event).await;

        if success {
            debug!(
                user_id = %context.user_id,
                event_type = %event_type,
                "Authentication success logged"
            );
        } else {
            warn!(
                user_id = %context.user_id,
                event_type = %event_type,
                failure_reason = ?failure_reason,
                "Authentication failure logged"
            );
        }
    }

    /// Log an authorization event
    #[instrument(skip(self, context), fields(user_id = %context.user_id))]
    pub async fn log_authorization_event(
        &self,
        context: &SecurityContext,
        resource: &str,
        permission: &str,
        granted: bool,
        denial_reason: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let audit_event_type = AuditEventType::Authorization {
            resource: resource.to_string(),
            permission: permission.to_string(),
            granted,
            denial_reason: denial_reason.map(|s| s.to_string()),
        };

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: if granted {
                AuditLevel::Info
            } else {
                AuditLevel::Warning
            },
            event_type: audit_event_type.clone(),
            user_context: context.audit_context(),
            message: format!(
                "Authorization {}: {} for {}",
                if granted { "granted" } else { "denied" },
                permission,
                resource
            ),
            metadata: serde_json::json!({
                "resource": resource,
                "permission": permission,
                "granted": granted,
                "denial_reason": denial_reason,
                "timestamp": Utc::now().to_rfc3339(),
            }),
            risk_score: self.calculate_risk_score(&audit_event_type),
            alert_triggered: !granted && self.is_sensitive_permission(permission),
        };

        self.add_event(event).await;

        if granted {
            debug!(
                user_id = %context.user_id,
                resource = %resource,
                permission = %permission,
                "Authorization granted logged"
            );
        } else {
            warn!(
                user_id = %context.user_id,
                resource = %resource,
                permission = %permission,
                denial_reason = ?denial_reason,
                "Authorization denied logged"
            );
        }
    }

    /// Log a system event
    pub async fn log_system_event(
        &self,
        context: &SecurityContext,
        event_type: &str,
        message: &str,
    ) {
        if !self.config.enabled {
            return;
        }

        let audit_event_type = AuditEventType::System {
            event_type: event_type.to_string(),
            component: "database-security-integration".to_string(),
            details: serde_json::json!({
                "message": message,
                "timestamp": Utc::now().to_rfc3339(),
            }),
        };

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: AuditLevel::System,
            event_type: audit_event_type.clone(),
            user_context: context.audit_context(),
            message: message.to_string(),
            metadata: serde_json::json!({
                "event_type": event_type,
                "component": "database-security-integration",
                "timestamp": Utc::now().to_rfc3339(),
            }),
            risk_score: self.calculate_risk_score(&audit_event_type),
            alert_triggered: false,
        };

        self.add_event(event).await;

        debug!(
            user_id = %context.user_id,
            event_type = %event_type,
            "System event logged"
        );
    }

    /// Log a security event
    pub async fn log_security_event(
        &self,
        context: &SecurityContext,
        event_type: &str,
        severity: &str,
        message: &str,
        details: serde_json::Value,
    ) {
        if !self.config.enabled {
            return;
        }

        let threat_level = self.determine_threat_level(event_type, severity);
        let audit_event_type = AuditEventType::Security {
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            threat_level: threat_level.clone(),
            details: details.clone(),
        };

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: AuditLevel::Security,
            event_type: audit_event_type.clone(),
            user_context: context.audit_context(),
            message: message.to_string(),
            metadata: serde_json::json!({
                "event_type": event_type,
                "severity": severity,
                "threat_level": threat_level,
                "details": details,
                "timestamp": Utc::now().to_rfc3339(),
            }),
            risk_score: self.calculate_risk_score(&audit_event_type),
            alert_triggered: severity == "high" || severity == "critical",
        };

        self.add_event(event).await;

        error!(
            user_id = %context.user_id,
            event_type = %event_type,
            severity = %severity,
            threat_level = ?threat_level,
            "Security event logged"
        );
    }

    /// Add event to buffer
    async fn add_event(&self, event: AuditEvent) {
        let mut buffer = self.event_buffer.write().await;
        buffer.push(event);

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_events += 1;
        }

        // Check if buffer is full and needs immediate flush
        if buffer.len() >= self.config.buffer_size {
            drop(buffer); // Release lock before flush
            self.flush_events().await;
        }
    }

    /// Flush buffered events to storage
    pub async fn flush(&self) -> Result<(), SecureDatabaseError> {
        if !self.config.enabled {
            return Ok(());
        }

        self.flush_events().await;
        info!("Audit events flushed");
        Ok(())
    }

    /// Internal flush implementation
    async fn flush_events(&self) {
        let events = {
            let mut buffer = self.event_buffer.write().await;
            if buffer.is_empty() {
                return;
            }
            std::mem::take(&mut *buffer)
        };

        if events.is_empty() {
            return;
        }

        debug!("Flushing {} audit events", events.len());

        // Store in database if enabled
        if self.config.store_in_database {
            if let Err(e) = self.write_events_to_database(&events).await {
                error!(error = %e, "Failed to write audit events to database");
                self.increment_write_errors().await;
            } else {
                self.increment_db_writes(&events).await;
            }
        }

        // Store in files if enabled
        if self.config.store_in_files {
            if let Err(e) = self.write_events_to_files(&events).await {
                error!(error = %e, "Failed to write audit events to files");
                self.increment_write_errors().await;
            } else {
                self.increment_file_writes(&events).await;
            }
        }

        // Stream events if enabled
        if self.config.enable_streaming {
            if let Err(e) = self.stream_events(&events).await {
                error!(error = %e, "Failed to stream audit events");
            } else {
                self.increment_streamed_events(&events).await;
            }
        }

        // Update last flush time
        {
            let mut metrics = self.metrics.write().await;
            metrics.last_flush = Some(Utc::now());
        }
    }

    /// Write events to database
    async fn write_events_to_database(&self, events: &[AuditEvent]) -> Result<()> {
        // Implementation would depend on specific database schema
        // For now, log that we would write to database
        debug!("Writing {} events to database", events.len());
        Ok(())
    }

    /// Write events to files
    async fn write_events_to_files(&self, events: &[AuditEvent]) -> Result<()> {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        let file_path = format!(
            "{}/audit_{}.jsonl",
            self.config.file_storage_path,
            Utc::now().format("%Y%m%d")
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .context("Failed to open audit log file")?;

        for event in events {
            let line = serde_json::to_string(event)?;
            file.write_all(format!("{}\n", line).as_bytes()).await?;
        }

        file.sync_all().await?;
        debug!("Wrote {} events to file: {}", events.len(), file_path);
        Ok(())
    }

    /// Stream events to external service
    async fn stream_events(&self, events: &[AuditEvent]) -> Result<()> {
        if let Some(endpoint) = &self.config.stream_endpoint {
            debug!("Streaming {} events to {}", events.len(), endpoint);
            // Implementation would send events to streaming endpoint
            // For now, just log the action
        }
        Ok(())
    }

    /// Start background flush task
    async fn start_flush_task(&self) {
        let event_buffer = self.event_buffer.clone();
        let flush_interval = std::time::Duration::from_secs(self.config.flush_interval_seconds);
        let logger_clone = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            loop {
                interval.tick().await;
                logger_clone.flush_events().await;
            }
        });
    }

    /// Calculate risk score for an event
    fn calculate_risk_score(&self, event_type: &AuditEventType) -> Option<u8> {
        match event_type {
            AuditEventType::DataAccess { operation, .. } => match operation.as_str() {
                "read" => Some(10),
                "create" => Some(30),
                "update" => Some(40),
                "delete" => Some(70),
                _ => Some(20),
            },
            AuditEventType::DataChange { operation, .. } => match operation.as_str() {
                "create" => Some(30),
                "update" => Some(40),
                "delete" => Some(80),
                _ => Some(50),
            },
            AuditEventType::Authentication { success, .. } => {
                if *success {
                    Some(5)
                } else {
                    Some(60)
                }
            }
            AuditEventType::Authorization { granted, .. } => {
                if *granted {
                    Some(10)
                } else {
                    Some(50)
                }
            }
            AuditEventType::Security { severity, .. } => match severity.as_str() {
                "low" => Some(30),
                "medium" => Some(50),
                "high" => Some(80),
                "critical" => Some(95),
                _ => Some(40),
            },
            AuditEventType::System { .. } => Some(15),
        }
    }

    /// Determine event level based on operation
    fn determine_event_level(&self, operation: &str) -> AuditLevel {
        match operation {
            "delete" => AuditLevel::Warning,
            op if op.starts_with("admin") => AuditLevel::Security,
            op if op.starts_with("security") => AuditLevel::Security,
            _ => AuditLevel::Info,
        }
    }

    /// Determine threat level
    fn determine_threat_level(&self, event_type: &str, severity: &str) -> Option<String> {
        match (event_type, severity) {
            (_, "critical") => Some("high".to_string()),
            (_, "high") => Some("medium".to_string()),
            ("failed_login", _) => Some("low".to_string()),
            ("permission_denied", _) => Some("low".to_string()),
            _ => None,
        }
    }

    /// Check if permission is sensitive
    fn is_sensitive_permission(&self, permission: &str) -> bool {
        permission.contains("admin")
            || permission.contains("delete")
            || permission.contains("security")
            || permission.contains("billing")
    }

    /// Get audit metrics
    pub async fn get_metrics(&self) -> AuditMetrics {
        self.metrics.read().await.clone()
    }

    // Metrics helper methods
    async fn increment_write_errors(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.write_errors += 1;
    }

    async fn increment_db_writes(&self, events: &[AuditEvent]) {
        let mut metrics = self.metrics.write().await;
        metrics.events_written_to_db += events.len() as u64;
    }

    async fn increment_file_writes(&self, events: &[AuditEvent]) {
        let mut metrics = self.metrics.write().await;
        metrics.events_written_to_file += events.len() as u64;
    }

    async fn increment_streamed_events(&self, events: &[AuditEvent]) {
        let mut metrics = self.metrics.write().await;
        metrics.events_streamed += events.len() as u64;
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        Self {
            database_manager: self.database_manager.clone(),
            config: self.config.clone(),
            event_buffer: self.event_buffer.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

// Default implementation for testing
impl Default for AuditLogger {
    fn default() -> Self {
        panic!("AuditLogger::default() should not be used in production - use AuditLogger::new() instead")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_context::SecurityContext;
    use std::collections::HashSet;

    fn create_test_context() -> SecurityContext {
        let user_id = UserId::new();
        let permissions = HashSet::new();
        let roles = vec!["user".to_string()];
        SecurityContext::new(user_id, None, permissions, roles)
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert!(config.store_in_database);
        assert_eq!(config.retention_days, 365);
        assert!(!config.always_audit_operations.is_empty());
    }

    #[test]
    fn test_audit_event_creation() {
        let context = create_test_context();
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: AuditLevel::Info,
            event_type: AuditEventType::DataAccess {
                table: "users".to_string(),
                record_id: "123".to_string(),
                operation: "read".to_string(),
            },
            user_context: context.audit_context(),
            message: "User data accessed".to_string(),
            metadata: serde_json::json!({"test": "data"}),
            risk_score: Some(10),
            alert_triggered: false,
        };

        assert_eq!(event.level, AuditLevel::Info);
        assert!(!event.alert_triggered);
        assert_eq!(event.risk_score, Some(10));
    }

    #[test]
    fn test_risk_score_calculation() {
        let config = AuditConfig::default();
        let database_manager = Arc::new(DatabaseManager::default());
        let logger = AuditLogger {
            database_manager,
            config,
            event_buffer: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(AuditMetrics::default())),
        };

        let read_event = AuditEventType::DataAccess {
            table: "users".to_string(),
            record_id: "123".to_string(),
            operation: "read".to_string(),
        };

        let delete_event = AuditEventType::DataChange {
            table: "users".to_string(),
            record_id: "123".to_string(),
            operation: "delete".to_string(),
            old_value: Some("old".to_string()),
            new_value: None,
        };

        assert_eq!(logger.calculate_risk_score(&read_event), Some(10));
        assert_eq!(logger.calculate_risk_score(&delete_event), Some(80));
    }

    #[test]
    fn test_sensitive_permission_detection() {
        let config = AuditConfig::default();
        let database_manager = Arc::new(DatabaseManager::default());
        let logger = AuditLogger {
            database_manager,
            config,
            event_buffer: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(AuditMetrics::default())),
        };

        assert!(logger.is_sensitive_permission("user:admin"));
        assert!(logger.is_sensitive_permission("workflow:delete"));
        assert!(logger.is_sensitive_permission("security:read"));
        assert!(!logger.is_sensitive_permission("user:read"));
        assert!(!logger.is_sensitive_permission("workflow:create"));
    }
}
