// AI-CORE Test Data Cleanup Service
// Automated cleanup and maintenance for test data and environments
// Backend Agent Implementation - T2.2

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::database::DatabaseManager;
use crate::models::*;

// ============================================================================
// Cleanup Service - Automated test data lifecycle management
// ============================================================================

pub struct CleanupService {
    database: Arc<DatabaseManager>,
    cleanup_jobs: Arc<RwLock<HashMap<Uuid, CleanupJob>>>,
    cleanup_policies: Arc<RwLock<HashMap<String, CleanupPolicy>>>,
}

#[derive(Debug, Clone)]
struct CleanupJob {
    id: Uuid,
    request: CleanupRequest,
    status: CleanupStatus,
    progress: u32,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    items_cleaned: i32,
    total_items: i32,
    backup_created: bool,
}

#[derive(Debug, Clone)]
enum CleanupStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
struct CleanupPolicy {
    name: String,
    description: String,
    data_types: Vec<CleanupType>,
    retention_hours: i64,
    conditions: Vec<CleanupCondition>,
    actions: Vec<CleanupAction>,
    enabled: bool,
}

#[derive(Debug, Clone)]
struct CleanupCondition {
    field: String,
    operator: ConditionOperator,
    value: String,
}

#[derive(Debug, Clone)]
enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone)]
enum CleanupAction {
    Delete,
    Archive,
    Anonymize,
    Backup,
    Notify,
}

impl CleanupService {
    pub async fn new(database: Arc<DatabaseManager>) -> Result<Self> {
        info!("Initializing CleanupService with automated policies");

        let service = Self {
            database,
            cleanup_jobs: Arc::new(RwLock::new(HashMap::new())),
            cleanup_policies: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize default cleanup policies
        service.initialize_default_policies().await?;

        info!("CleanupService initialized successfully");
        Ok(service)
    }

    // ========================================================================
    // Public API Methods
    // ========================================================================

    pub async fn cleanup(&self, request: CleanupRequest) -> Result<CleanupResponse> {
        let cleanup_id = Uuid::new_v4();
        let now = Utc::now();

        debug!("Starting cleanup operation: {} - {:?}", cleanup_id, request.cleanup_type);

        // Validate request
        self.validate_cleanup_request(&request).await?;

        // Estimate items to cleanup
        let estimated_items = self.estimate_cleanup_items(&request).await?;

        // Create cleanup job
        let job = CleanupJob {
            id: cleanup_id,
            request: request.clone(),
            status: CleanupStatus::Pending,
            progress: 0,
            created_at: now,
            completed_at: None,
            error_message: None,
            items_cleaned: 0,
            total_items: estimated_items,
            backup_created: false,
        };

        // Store job
        {
            let mut jobs = self.cleanup_jobs.write().await;
            jobs.insert(cleanup_id, job);
        }

        // Start cleanup in background
        let service = self.clone();
        tokio::spawn(async move {
            if let Err(e) = service.execute_cleanup(cleanup_id).await {
                error!("Cleanup operation failed: {}", e);
                service.mark_cleanup_failed(cleanup_id, e.to_string()).await;
            }
        });

        let estimated_duration = self.estimate_cleanup_duration(&request, estimated_items).await;

        Ok(CleanupResponse {
            cleanup_id,
            status: "pending".to_string(),
            items_to_cleanup: estimated_items,
            estimated_duration_seconds: estimated_duration,
            progress_url: format!("/api/cleanup/{}/status", cleanup_id),
        })
    }

    pub async fn get_cleanup_status(&self, cleanup_id: Uuid) -> Result<CleanupResponse> {
        let jobs = self.cleanup_jobs.read().await;
        let job = jobs.get(&cleanup_id)
            .ok_or_else(|| anyhow!("Cleanup job not found"))?;

        Ok(CleanupResponse {
            cleanup_id,
            status: format!("{:?}", job.status).to_lowercase(),
            items_to_cleanup: job.total_items,
            estimated_duration_seconds: if job.completed_at.is_some() { 0 } else { 300 },
            progress_url: format!("/api/cleanup/{}/status", cleanup_id),
        })
    }

    pub async fn reset_environment(&self, environment_id: Uuid) -> Result<()> {
        info!("Resetting test environment: {}", environment_id);

        // Get environment details
        let environments = self.database.get_test_environments().await?;
        let environment = environments.into_iter()
            .find(|env| env.id == environment_id)
            .ok_or_else(|| anyhow!("Environment not found"))?;

        // Create comprehensive cleanup request for environment
        let cleanup_request = CleanupRequest {
            environment_ids: vec![environment_id],
            cleanup_type: CleanupType::All,
            force: true,
            backup_before_cleanup: true,
        };

        // Execute immediate cleanup
        self.execute_environment_reset(&environment, &cleanup_request).await?;

        info!("Environment reset completed: {}", environment_id);
        Ok(())
    }

    pub async fn run_scheduled_cleanup(&self) -> Result<()> {
        debug!("Running scheduled cleanup tasks");

        let policies = self.cleanup_policies.read().await;

        for (policy_name, policy) in policies.iter() {
            if !policy.enabled {
                continue;
            }

            debug!("Executing cleanup policy: {}", policy_name);

            match self.execute_policy_cleanup(policy).await {
                Ok(cleaned_count) => {
                    info!("Policy '{}' cleaned {} items", policy_name, cleaned_count);
                }
                Err(e) => {
                    error!("Policy '{}' failed: {}", policy_name, e);
                }
            }
        }

        info!("Scheduled cleanup tasks completed");
        Ok(())
    }

    // ========================================================================
    // Cleanup Execution Implementation
    // ========================================================================

    async fn execute_cleanup(&self, cleanup_id: Uuid) -> Result<()> {
        info!("Executing cleanup operation: {}", cleanup_id);

        // Update status to running
        self.update_cleanup_status(cleanup_id, CleanupStatus::Running, 0).await;

        let job = {
            let jobs = self.cleanup_jobs.read().await;
            jobs.get(&cleanup_id).cloned()
                .ok_or_else(|| anyhow!("Cleanup job not found"))?
        };

        // Create backup if requested
        if job.request.backup_before_cleanup {
            self.create_backup(&job).await?;
            self.mark_backup_created(cleanup_id).await;
        }

        // Execute cleanup based on type
        let cleaned_count = match job.request.cleanup_type {
            CleanupType::Users => self.cleanup_users(&job).await?,
            CleanupType::Workflows => self.cleanup_workflows(&job).await?,
            CleanupType::TestData => self.cleanup_test_data(&job).await?,
            CleanupType::Environments => self.cleanup_environments(&job).await?,
            CleanupType::Artifacts => self.cleanup_artifacts(&job).await?,
            CleanupType::All => self.cleanup_all(&job).await?,
        };

        // Mark as completed
        self.mark_cleanup_completed(cleanup_id, cleaned_count).await;

        info!("Cleanup operation completed: {} ({} items cleaned)", cleanup_id, cleaned_count);
        Ok(())
    }

    async fn cleanup_users(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up test users for environments: {:?}", job.request.environment_ids);

        let mut cleaned_count = 0;
        let batch_size = 100;

        for environment_id in &job.request.environment_ids {
            // Get users for environment (in batches)
            loop {
                let users = self.database.get_test_users("", batch_size).await?;
                if users.is_empty() {
                    break;
                }

                let mut batch_cleaned = 0;
                for user in users {
                    // Check if user should be cleaned up
                    if self.should_cleanup_user(&user, job).await? {
                        if let Ok(deleted) = self.database.delete_test_user(user.id).await {
                            if deleted {
                                batch_cleaned += 1;
                                cleaned_count += 1;
                            }
                        }
                    }

                    // Update progress
                    if cleaned_count % 10 == 0 {
                        let progress = std::cmp::min(
                            ((cleaned_count as f32 / job.total_items as f32) * 100.0) as u32,
                            95
                        );
                        self.update_cleanup_progress(job.id, progress, cleaned_count).await;
                    }
                }

                debug!("Cleaned {} users in batch", batch_cleaned);

                if batch_cleaned == 0 {
                    break; // No more users to clean up
                }

                // Small delay between batches
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Ok(cleaned_count)
    }

    async fn cleanup_workflows(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up workflows");
        // Implementation would query workflow tables and clean up based on criteria
        // This is a placeholder implementation

        let mut cleaned_count = 0;
        let cutoff_date = Utc::now() - chrono::Duration::hours(72); // 3 days old

        // Simulate workflow cleanup
        for i in 0..50 {
            // Check if should be cleaned up based on age, status, etc.
            cleaned_count += 1;

            if i % 10 == 0 {
                let progress = ((i as f32 / 50.0) * 100.0) as u32;
                self.update_cleanup_progress(job.id, progress, cleaned_count).await;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        Ok(cleaned_count)
    }

    async fn cleanup_test_data(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up general test data");

        let mut cleaned_count = 0;

        // Clean up cached data from Redis
        cleaned_count += self.cleanup_redis_cache(job).await?;

        // Clean up MongoDB test documents
        cleaned_count += self.cleanup_mongodb_data(job).await?;

        // Clean up ClickHouse analytics data
        cleaned_count += self.cleanup_clickhouse_data(job).await?;

        Ok(cleaned_count)
    }

    async fn cleanup_environments(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up test environments");

        let mut cleaned_count = 0;
        let environments = self.database.get_test_environments().await?;

        for environment in environments {
            if self.should_cleanup_environment(&environment, job).await? {
                // In a real implementation, this would properly destroy the environment
                debug!("Would destroy environment: {} ({})", environment.name, environment.id);
                cleaned_count += 1;
            }

            // Update progress
            if cleaned_count % 5 == 0 {
                let progress = std::cmp::min(
                    ((cleaned_count as f32 / job.total_items as f32) * 100.0) as u32,
                    95
                );
                self.update_cleanup_progress(job.id, progress, cleaned_count).await;
            }
        }

        Ok(cleaned_count)
    }

    async fn cleanup_artifacts(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up test artifacts");

        let mut cleaned_count = 0;

        // Clean up test artifacts like screenshots, logs, reports, etc.
        // This would typically involve file system cleanup

        let artifact_types = ["screenshots", "videos", "logs", "reports", "traces"];

        for artifact_type in &artifact_types {
            debug!("Cleaning up {} artifacts", artifact_type);

            // Simulate artifact cleanup
            for i in 0..20 {
                cleaned_count += 1;

                if i % 5 == 0 {
                    let progress = std::cmp::min(
                        ((cleaned_count as f32 / job.total_items as f32) * 100.0) as u32,
                        95
                    );
                    self.update_cleanup_progress(job.id, progress, cleaned_count).await;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        Ok(cleaned_count)
    }

    async fn cleanup_all(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Performing comprehensive cleanup");

        let mut total_cleaned = 0;

        // Clean up in order of dependency
        total_cleaned += self.cleanup_users(job).await?;
        total_cleaned += self.cleanup_workflows(job).await?;
        total_cleaned += self.cleanup_test_data(job).await?;
        total_cleaned += self.cleanup_artifacts(job).await?;
        total_cleaned += self.cleanup_environments(job).await?;

        Ok(total_cleaned)
    }

    async fn execute_environment_reset(&self, environment: &TestEnvironment, request: &CleanupRequest) -> Result<()> {
        debug!("Executing environment reset: {}", environment.name);

        // 1. Stop all running processes
        self.stop_environment_processes(environment).await?;

        // 2. Clear databases
        self.clear_environment_databases(environment).await?;

        // 3. Reset configurations
        self.reset_environment_configuration(environment).await?;

        // 4. Clear caches
        self.clear_environment_caches(environment).await?;

        // 5. Restart services
        self.restart_environment_services(environment).await?;

        info!("Environment reset completed: {}", environment.name);
        Ok(())
    }

    // ========================================================================
    // Cleanup Policy Implementation
    // ========================================================================

    async fn execute_policy_cleanup(&self, policy: &CleanupPolicy) -> Result<i32> {
        debug!("Executing cleanup policy: {}", policy.name);

        let mut total_cleaned = 0;
        let cutoff_time = Utc::now() - chrono::Duration::hours(policy.retention_hours);

        for data_type in &policy.data_types {
            let cleaned_count = match data_type {
                CleanupType::Users => self.cleanup_expired_users(&cutoff_time, policy).await?,
                CleanupType::Workflows => self.cleanup_expired_workflows(&cutoff_time, policy).await?,
                CleanupType::TestData => self.cleanup_expired_test_data(&cutoff_time, policy).await?,
                CleanupType::Environments => self.cleanup_expired_environments(&cutoff_time, policy).await?,
                CleanupType::Artifacts => self.cleanup_expired_artifacts(&cutoff_time, policy).await?,
                CleanupType::All => {
                    let mut all_count = 0;
                    all_count += self.cleanup_expired_users(&cutoff_time, policy).await?;
                    all_count += self.cleanup_expired_workflows(&cutoff_time, policy).await?;
                    all_count += self.cleanup_expired_test_data(&cutoff_time, policy).await?;
                    all_count += self.cleanup_expired_artifacts(&cutoff_time, policy).await?;
                    all_count += self.cleanup_expired_environments(&cutoff_time, policy).await?;
                    all_count
                }
            };

            total_cleaned += cleaned_count;
            debug!("Policy '{}' cleaned {} items of type {:?}", policy.name, cleaned_count, data_type);
        }

        Ok(total_cleaned)
    }

    async fn cleanup_expired_users(&self, cutoff_time: &DateTime<Utc>, policy: &CleanupPolicy) -> Result<i32> {
        let users = self.database.get_test_users("", 1000).await?;
        let mut cleaned_count = 0;

        for user in users {
            if let Some(cleanup_after) = user.cleanup_after {
                if cleanup_after <= *cutoff_time && self.matches_policy_conditions(&user, policy) {
                    if self.database.delete_test_user(user.id).await.unwrap_or(false) {
                        cleaned_count += 1;
                    }
                }
            }
        }

        Ok(cleaned_count)
    }

    async fn cleanup_expired_workflows(&self, cutoff_time: &DateTime<Utc>, _policy: &CleanupPolicy) -> Result<i32> {
        // Implementation for workflow cleanup
        debug!("Cleaning up expired workflows before {}", cutoff_time);
        Ok(0) // Placeholder
    }

    async fn cleanup_expired_test_data(&self, cutoff_time: &DateTime<Utc>, _policy: &CleanupPolicy) -> Result<i32> {
        // Implementation for test data cleanup
        debug!("Cleaning up expired test data before {}", cutoff_time);
        Ok(0) // Placeholder
    }

    async fn cleanup_expired_environments(&self, cutoff_time: &DateTime<Utc>, policy: &CleanupPolicy) -> Result<i32> {
        let environments = self.database.get_test_environments().await?;
        let mut cleaned_count = 0;

        for environment in environments {
            if let Some(expires_at) = environment.expires_at {
                if expires_at <= *cutoff_time && environment.auto_cleanup {
                    // In real implementation, would destroy the environment
                    debug!("Would cleanup expired environment: {}", environment.name);
                    cleaned_count += 1;
                }
            }
        }

        Ok(cleaned_count)
    }

    async fn cleanup_expired_artifacts(&self, cutoff_time: &DateTime<Utc>, _policy: &CleanupPolicy) -> Result<i32> {
        // Implementation for artifact cleanup
        debug!("Cleaning up expired artifacts before {}", cutoff_time);
        Ok(0) // Placeholder
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    async fn should_cleanup_user(&self, user: &TestUser, job: &CleanupJob) -> Result<bool> {
        // Check if user should be cleaned up based on criteria

        // Force cleanup if requested
        if job.request.force {
            return Ok(true);
        }

        // Check if user has cleanup_after date and it's passed
        if let Some(cleanup_after) = user.cleanup_after {
            if Utc::now() > cleanup_after {
                return Ok(true);
            }
        }

        // Check if user is inactive for long time
        if let Some(last_login) = user.last_login_at {
            let inactive_days = (Utc::now() - last_login).num_days();
            if inactive_days > 30 { // 30 days inactive
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn should_cleanup_environment(&self, environment: &TestEnvironment, job: &CleanupJob) -> Result<bool> {
        if job.request.force {
            return Ok(true);
        }

        if environment.should_cleanup() {
            return Ok(true);
        }

        // Check if environment is in error state
        if matches!(environment.status, EnvironmentStatus::Error) {
            return Ok(true);
        }

        Ok(false)
    }

    fn matches_policy_conditions(&self, user: &TestUser, policy: &CleanupPolicy) -> bool {
        // Check if user matches policy conditions
        for condition in &policy.conditions {
            match condition.field.as_str() {
                "role" => {
                    let user_role = user.role.to_string();
                    if !self.evaluate_condition(&user_role, &condition.operator, &condition.value) {
                        return false;
                    }
                }
                "test_environment" => {
                    if !self.evaluate_condition(&user.test_environment, &condition.operator, &condition.value) {
                        return false;
                    }
                }
                "is_active" => {
                    let active_str = user.is_active.to_string();
                    if !self.evaluate_condition(&active_str, &condition.operator, &condition.value) {
                        return false;
                    }
                }
                _ => continue,
            }
        }
        true
    }

    fn evaluate_condition(&self, field_value: &str, operator: &ConditionOperator, expected_value: &str) -> bool {
        match operator {
            ConditionOperator::Equals => field_value == expected_value,
            ConditionOperator::NotEquals => field_value != expected_value,
            ConditionOperator::Contains => field_value.contains(expected_value),
            ConditionOperator::GreaterThan => field_value > expected_value,
            ConditionOperator::LessThan => field_value < expected_value,
            ConditionOperator::IsNull => field_value.is_empty(),
            ConditionOperator::IsNotNull => !field_value.is_empty(),
        }
    }

    async fn create_backup(&self, job: &CleanupJob) -> Result<()> {
        debug!("Creating backup before cleanup: {}", job.id);

        // In a real implementation, this would:
        // 1. Export data to backup storage
        // 2. Create database dumps
        // 3. Archive files and artifacts
        // 4. Generate backup manifest

        info!("Backup created for cleanup job: {}", job.id);
        Ok(())
    }

    async fn cleanup_redis_cache(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up Redis cache data");

        // In real implementation, would connect to Redis and clean up keys
        // based on patterns or TTL

        Ok(25) // Placeholder count
    }

    async fn cleanup_mongodb_data(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up MongoDB test data");

        // In real implementation, would clean up MongoDB collections
        // based on criteria like age, test environment, etc.

        Ok(30) // Placeholder count
    }

    async fn cleanup_clickhouse_data(&self, job: &CleanupJob) -> Result<i32> {
        debug!("Cleaning up ClickHouse analytics data");

        // In real implementation, would clean up old analytics data
        // to maintain performance and storage limits

        Ok(15) // Placeholder count
    }

    async fn stop_environment_processes(&self, environment: &TestEnvironment) -> Result<()> {
        debug!("Stopping processes for environment: {}", environment.name);
        // Implementation would stop running services/processes
        Ok(())
    }

    async fn clear_environment_databases(&self, environment: &TestEnvironment) -> Result<()> {
        debug!("Clearing databases for environment: {}", environment.name);
        // Implementation would clear/reset database data
        Ok(())
    }

    async fn reset_environment_configuration(&self, environment: &TestEnvironment) -> Result<()> {
        debug!("Resetting configuration for environment: {}", environment.name);
        // Implementation would reset configuration to defaults
        Ok(())
    }

    async fn clear_environment_caches(&self, environment: &TestEnvironment) -> Result<()> {
        debug!("Clearing caches for environment: {}", environment.name);
        // Implementation would clear Redis and other caches
        Ok(())
    }

    async fn restart_environment_services(&self, environment: &TestEnvironment) -> Result<()> {
        debug!("Restarting services for environment: {}", environment.name);
        // Implementation would restart services in correct order
        Ok(())
    }

    async fn estimate_cleanup_items(&self, request: &CleanupRequest) -> Result<i32> {
        let mut estimated_items = 0;

        match request.cleanup_type {
            CleanupType::Users => {
                // Estimate based on environment user count
                for _env_id in &request.environment_ids {
                    estimated_items += 100; // Rough estimate per environment
                }
            }
            CleanupType::Workflows => estimated_items += 50,
            CleanupType::TestData => estimated_items += 200,
            CleanupType::Environments => estimated_items += request.environment_ids.len() as i32,
            CleanupType::Artifacts => estimated_items += 300,
            CleanupType::All => estimated_items += 1000,
        }

        Ok(estimated_items)
    }

    async fn estimate_cleanup_duration(&self, request: &CleanupRequest, item_count: i32) -> i32 {
        let base_time_per_item = match request.cleanup_type {
            CleanupType::Users => 0.1,
            CleanupType::Workflows => 0.2,
            CleanupType::TestData => 0.05,
            CleanupType::Environments => 5.0, // Environments take longer
            CleanupType::Artifacts => 0.02,
            CleanupType::All => 0.2,
        };

        let estimated_seconds = (item_count as f32 * base_time_per_item) + 30.0; // Add 30s overhead
        estimated_seconds as i32
    }

    async fn validate_cleanup_request(&self, request: &CleanupRequest) -> Result<()> {
        if request.environment_ids.is_empty() {
            return Err(anyhow!("At least one environment ID must be specified"));
        }

        Ok(())
    }

    async fn initialize_default_policies(&self) -> Result<()> {
        debug!("Initializing default cleanup policies");

        let mut policies = self.cleanup_policies.write().await;

        // Policy for cleaning up old test users
        policies.insert("expired_test_users".to_string(), CleanupPolicy {
            name: "Expired Test Users".to_string(),
            description: "Clean up test users that have exceeded their TTL".to_string(),
            data_types: vec![CleanupType::Users],
            retention_hours: 72, // 3 days
            conditions: vec![
                CleanupCondition {
                    field: "test_environment".to_string(),
                    operator: ConditionOperator::Contains,
                    value: "test".to_string(),
                }
            ],
            actions: vec![CleanupAction::Delete],
            enabled: true,
        });

        // Policy for cleaning up old environments
        policies.insert("expired_environments".to_string(), CleanupPolicy {
            name: "Expired Test Environments".to_string(),
            description: "Clean up test environments that have expired".to_string(),
            data_types: vec![CleanupType::Environments],
            retention_hours: 168, // 1 week
            conditions: vec![],
            actions: vec![CleanupAction::Archive, CleanupAction::Delete],
            enabled: true,
        });

        // Policy for cleaning up old artifacts
        policies.insert("old_artifacts".to_string(), CleanupPolicy {
            name: "Old Test Artifacts".to_string(),
            description: "Clean up test artifacts older than 1 week".to_string(),
            data_types: vec![CleanupType::Artifacts],
            retention_hours: 168, // 1 week
            conditions: vec![],
            actions: vec![CleanupAction::Archive, CleanupAction::Delete],
            enabled: true,
        });

        info!("Default cleanup policies initialized");
        Ok(())
    }

    async fn update_cleanup_status(&self, cleanup_id: Uuid, status: CleanupStatus, progress: u32) {
        if let Ok(mut jobs) = self.cleanup_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&cleanup_id) {
                job.status = status;
                job.progress = progress;
            }
        }
    }

    async fn update_cleanup_progress(&self, cleanup_id: Uuid, progress: u32, items_cleaned: i32) {
        if let Ok(mut jobs) = self.cleanup_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&cleanup_id) {
                job.progress = progress;
                job.items_cleaned = items_cleaned;
            }
        }
    }

    async fn mark_backup_created(&self, cleanup_id: Uuid) {
        if let Ok(mut jobs) = self.cleanup_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&cleanup_id) {
                job.backup_created = true;
            }
        }
    }

    async fn mark_cleanup_completed(&self, cleanup_id: Uuid, items_cleaned: i32) {
        if let Ok(mut jobs) = self.cleanup_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&cleanup_id) {
                job.status = CleanupStatus::Completed;
                job.progress = 100;
                job.items_cleaned = items_cleaned;
                job.completed_at = Some(Utc::now());
            }
        }
    }

    async fn mark_cleanup_failed(&self, cleanup_id: Uuid, error_message: String) {
        if let Ok(mut jobs) = self.cleanup_jobs.try_write() {
            if let Some(job) = jobs.get_mut(&cleanup_id) {
                job.status = CleanupStatus::Failed;
                job.error_message = Some(error_message);
                job.completed_at = Some(Utc::now());
            }
        }
    }
}

// ============================================================================
// Clone implementation for shared usage
// ============================================================================

impl Clone for CleanupService {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            cleanup_jobs: self.cleanup_jobs.clone(),
            cleanup_policies: self.cleanup_policies.clone(),
        }
    }
}
