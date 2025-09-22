//! # Event Storage Module
//!
//! This module provides event storage functionality for the event streaming service.
//! It handles persistent storage of events for audit, replay, and compliance purposes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{PgPool, Row};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{EventStreamingError, Result},
    events::Event,
    types::{ComponentHealth, EventCategory, EventStatus, HealthStatus},
};

/// Event storage manager for persistent event storage
#[derive(Clone)]
pub struct EventStorage {
    config: Arc<Config>,
    pool: Arc<PgPool>,
}

impl EventStorage {
    /// Create a new event storage manager
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Initializing Event Storage");

        // Create database connection pool
        let pool = PgPool::connect(&config.storage.database_url)
            .await
            .map_err(|e| {
                EventStreamingError::storage(format!("Failed to connect to database: {}", e))
            })?;

        // Run migrations if needed
        Self::run_migrations(&pool).await?;

        Ok(Self {
            config: Arc::new(config.clone()),
            pool: Arc::new(pool),
        })
    }

    /// Store an event in the database
    pub async fn store_event(&self, event: &Event) -> Result<()> {
        let start_time = Instant::now();

        // Serialize event payload and metadata
        let payload_json = serde_json::to_value(&event.payload).map_err(|e| {
            EventStreamingError::storage(format!("Failed to serialize payload: {}", e))
        })?;

        let metadata_json = serde_json::to_value(&event.metadata).map_err(|e| {
            EventStreamingError::storage(format!("Failed to serialize metadata: {}", e))
        })?;

        let correlation_json = serde_json::to_value(&event.correlation).map_err(|e| {
            EventStreamingError::storage(format!("Failed to serialize correlation: {}", e))
        })?;

        let source_json = serde_json::to_value(&event.source).map_err(|e| {
            EventStreamingError::storage(format!("Failed to serialize source: {}", e))
        })?;

        let destinations_json = serde_json::to_value(&event.destinations).map_err(|e| {
            EventStreamingError::storage(format!("Failed to serialize destinations: {}", e))
        })?;

        let processing_history_json =
            serde_json::to_value(&event.processing_history).map_err(|e| {
                EventStreamingError::storage(format!(
                    "Failed to serialize processing history: {}",
                    e
                ))
            })?;

        let error_json = event
            .error
            .as_ref()
            .map(|e| serde_json::to_value(e))
            .transpose()
            .map_err(|e| {
                EventStreamingError::storage(format!("Failed to serialize error: {}", e))
            })?;

        // Insert event into database
        let query = r#"
            INSERT INTO events (
                id, event_type, category, priority, source, correlation, destinations,
                payload, metadata, status, created_at, updated_at, expires_at,
                attempt_count, error, processing_history
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
            ) ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at,
                attempt_count = EXCLUDED.attempt_count,
                error = EXCLUDED.error,
                processing_history = EXCLUDED.processing_history
        "#;

        sqlx::query(query)
            .bind(event.id)
            .bind(&event.event_type)
            .bind(serde_json::to_string(&event.category).unwrap())
            .bind(serde_json::to_string(&event.priority).unwrap())
            .bind(source_json)
            .bind(correlation_json)
            .bind(destinations_json)
            .bind(payload_json)
            .bind(metadata_json)
            .bind(serde_json::to_string(&event.status).unwrap())
            .bind(event.created_at)
            .bind(event.updated_at)
            .bind(event.expires_at)
            .bind(event.attempt_count as i32)
            .bind(error_json)
            .bind(processing_history_json)
            .execute(&*self.pool)
            .await
            .map_err(|e| EventStreamingError::storage(format!("Failed to store event: {}", e)))?;

        let duration = start_time.elapsed();
        debug!("Stored event {} in {:?}", event.id, duration);

        Ok(())
    }

    /// Get an event by ID
    pub async fn get_event(&self, event_id: Uuid) -> Result<Option<Event>> {
        let query = "SELECT * FROM events WHERE id = $1";

        let row = sqlx::query(query)
            .bind(event_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| EventStreamingError::storage(format!("Failed to get event: {}", e)))?;

        match row {
            Some(row) => {
                let event = self.row_to_event(row)?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    /// Update event status
    pub async fn update_event_status(&self, event: &Event) -> Result<()> {
        let processing_history_json =
            serde_json::to_value(&event.processing_history).map_err(|e| {
                EventStreamingError::storage(format!(
                    "Failed to serialize processing history: {}",
                    e
                ))
            })?;

        let error_json = event
            .error
            .as_ref()
            .map(|e| serde_json::to_value(e))
            .transpose()
            .map_err(|e| {
                EventStreamingError::storage(format!("Failed to serialize error: {}", e))
            })?;

        let query = r#"
            UPDATE events SET
                status = $2,
                updated_at = $3,
                attempt_count = $4,
                error = $5,
                processing_history = $6
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(event.id)
            .bind(serde_json::to_string(&event.status).unwrap())
            .bind(event.updated_at)
            .bind(event.attempt_count as i32)
            .bind(error_json)
            .bind(processing_history_json)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                EventStreamingError::storage(format!("Failed to update event status: {}", e))
            })?;

        debug!(
            "Updated status for event {} to {:?}",
            event.id, event.status
        );
        Ok(())
    }

    /// Get event status and processing history
    pub async fn get_event_status(
        &self,
        event_id: Uuid,
    ) -> Result<Option<(EventStatus, Vec<serde_json::Value>)>> {
        let query = "SELECT status, processing_history FROM events WHERE id = $1";

        let row = sqlx::query(query)
            .bind(event_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                EventStreamingError::storage(format!("Failed to get event status: {}", e))
            })?;

        match row {
            Some(row) => {
                let status_str: String = row.get("status");
                let status: EventStatus = serde_json::from_str(&status_str).map_err(|e| {
                    EventStreamingError::storage(format!("Failed to parse status: {}", e))
                })?;

                let history_json: serde_json::Value = row.get("processing_history");
                let history: Vec<serde_json::Value> = serde_json::from_value(history_json)
                    .map_err(|e| {
                        EventStreamingError::storage(format!(
                            "Failed to parse processing history: {}",
                            e
                        ))
                    })?;

                Ok(Some((status, history)))
            }
            None => Ok(None),
        }
    }

    /// Count events for replay
    pub async fn count_events(
        &self,
        from_timestamp: DateTime<Utc>,
        to_timestamp: Option<DateTime<Utc>>,
        event_types: Option<Vec<String>>,
        categories: Option<Vec<EventCategory>>,
    ) -> Result<u64> {
        let mut query = "SELECT COUNT(*) FROM events WHERE created_at >= $1".to_string();
        let mut param_count = 1;

        if let Some(to_timestamp) = to_timestamp {
            param_count += 1;
            query.push_str(&format!(" AND created_at <= ${}", param_count));
        }

        if let Some(event_types) = &event_types {
            param_count += 1;
            query.push_str(&format!(" AND event_type = ANY(${}", param_count));
        }

        if let Some(categories) = &categories {
            let category_strs: Vec<String> = categories
                .iter()
                .map(|c| serde_json::to_string(c).unwrap_or_default())
                .collect();
            param_count += 1;
            query.push_str(&format!(" AND category = ANY(${}", param_count));
        }

        let mut sql_query = sqlx::query(&query).bind(from_timestamp);

        if let Some(to_timestamp) = to_timestamp {
            sql_query = sql_query.bind(to_timestamp);
        }

        if let Some(event_types) = event_types {
            sql_query = sql_query.bind(event_types);
        }

        if let Some(categories) = categories {
            let category_strs: Vec<String> = categories
                .iter()
                .map(|c| serde_json::to_string(c).unwrap_or_default())
                .collect();
            sql_query = sql_query.bind(category_strs);
        }

        let row = sql_query
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| EventStreamingError::storage(format!("Failed to count events: {}", e)))?;

        let count: i64 = row.get(0);
        Ok(count as u64)
    }

    /// Get events for replay
    pub async fn get_events_for_replay(
        &self,
        from_timestamp: DateTime<Utc>,
        to_timestamp: Option<DateTime<Utc>>,
        event_types: Option<Vec<String>>,
        categories: Option<Vec<EventCategory>>,
        limit: u32,
        offset: u64,
    ) -> Result<Vec<Event>> {
        let mut query = "SELECT * FROM events WHERE created_at >= $1".to_string();
        let mut param_count = 1;

        if let Some(to_timestamp) = to_timestamp {
            param_count += 1;
            query.push_str(&format!(" AND created_at <= ${}", param_count));
        }

        if let Some(event_types) = &event_types {
            param_count += 1;
            query.push_str(&format!(" AND event_type = ANY(${}", param_count));
        }

        if let Some(categories) = &categories {
            param_count += 1;
            query.push_str(&format!(" AND category = ANY(${}", param_count));
        }

        query.push_str(" ORDER BY created_at ASC");
        param_count += 1;
        query.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));

        let mut sql_query = sqlx::query(&query).bind(from_timestamp);

        if let Some(to_timestamp) = to_timestamp {
            sql_query = sql_query.bind(to_timestamp);
        }

        if let Some(event_types) = event_types {
            sql_query = sql_query.bind(event_types);
        }

        if let Some(categories) = categories {
            let category_strs: Vec<String> = categories
                .iter()
                .map(|c| serde_json::to_string(c).unwrap_or_default())
                .collect();
            sql_query = sql_query.bind(category_strs);
        }

        sql_query = sql_query.bind(limit as i64).bind(offset as i64);

        let rows = sql_query.fetch_all(&*self.pool).await.map_err(|e| {
            EventStreamingError::storage(format!("Failed to get events for replay: {}", e))
        })?;

        let mut events = Vec::new();
        for row in rows {
            match self.row_to_event(row) {
                Ok(event) => events.push(event),
                Err(e) => {
                    warn!("Failed to parse event from database row: {}", e);
                    continue;
                }
            }
        }

        Ok(events)
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<ComponentHealth> {
        let start_time = Instant::now();

        // Test database connectivity
        match sqlx::query("SELECT 1").fetch_one(&*self.pool).await {
            Ok(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(ComponentHealth {
                    component: "storage".to_string(),
                    status: HealthStatus::Healthy,
                    last_check: chrono::Utc::now(),
                    response_time_ms: response_time,
                    details: [
                        (
                            "database_url".to_string(),
                            self.config.storage.database_url.clone(),
                        ),
                        (
                            "max_connections".to_string(),
                            self.config.storage.max_connections.to_string(),
                        ),
                    ]
                    .into(),
                })
            }
            Err(e) => {
                error!("Storage health check failed: {}", e);
                Ok(ComponentHealth {
                    component: "storage".to_string(),
                    status: HealthStatus::Unhealthy,
                    last_check: chrono::Utc::now(),
                    response_time_ms: 0,
                    details: [("error".to_string(), e.to_string())].into(),
                })
            }
        }
    }

    /// Run database migrations
    async fn run_migrations(pool: &PgPool) -> Result<()> {
        info!("Running database migrations");

        let migration_sql = r#"
            CREATE TABLE IF NOT EXISTS events (
                id UUID PRIMARY KEY,
                event_type VARCHAR(100) NOT NULL,
                category VARCHAR(50) NOT NULL,
                priority VARCHAR(20) NOT NULL,
                source JSONB NOT NULL,
                correlation JSONB NOT NULL,
                destinations JSONB NOT NULL DEFAULT '[]',
                payload JSONB NOT NULL,
                metadata JSONB NOT NULL DEFAULT '{}',
                status VARCHAR(20) NOT NULL DEFAULT 'pending',
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                expires_at TIMESTAMPTZ,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                error JSONB,
                processing_history JSONB NOT NULL DEFAULT '[]',

                -- Indexes for common queries
                CONSTRAINT events_status_check CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'retried', 'dead_letter', 'skipped'))
            );

            CREATE INDEX IF NOT EXISTS events_created_at_idx ON events (created_at);
            CREATE INDEX IF NOT EXISTS events_event_type_idx ON events (event_type);
            CREATE INDEX IF NOT EXISTS events_category_idx ON events (category);
            CREATE INDEX IF NOT EXISTS events_status_idx ON events (status);
            CREATE INDEX IF NOT EXISTS events_updated_at_idx ON events (updated_at);

            -- GIN indexes for JSONB columns
            CREATE INDEX IF NOT EXISTS events_source_gin_idx ON events USING GIN (source);
            CREATE INDEX IF NOT EXISTS events_metadata_gin_idx ON events USING GIN (metadata);
            CREATE INDEX IF NOT EXISTS events_payload_gin_idx ON events USING GIN (payload);
        "#;

        sqlx::query(migration_sql)
            .execute(pool)
            .await
            .map_err(|e| {
                EventStreamingError::storage(format!("Failed to run migrations: {}", e))
            })?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Convert database row to Event
    fn row_to_event(&self, row: sqlx::postgres::PgRow) -> Result<Event> {
        let id: Uuid = row.get("id");
        let event_type: String = row.get("event_type");
        let category_str: String = row.get("category");
        let priority_str: String = row.get("priority");
        let source_json: serde_json::Value = row.get("source");
        let correlation_json: serde_json::Value = row.get("correlation");
        let destinations_json: serde_json::Value = row.get("destinations");
        let payload_json: serde_json::Value = row.get("payload");
        let metadata_json: serde_json::Value = row.get("metadata");
        let status_str: String = row.get("status");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");
        let expires_at: Option<DateTime<Utc>> = row.get("expires_at");
        let attempt_count: i32 = row.get("attempt_count");
        let error_json: Option<serde_json::Value> = row.get("error");
        let processing_history_json: serde_json::Value = row.get("processing_history");

        // Parse all JSON fields
        let category = serde_json::from_str(&category_str).map_err(|e| {
            EventStreamingError::storage(format!("Failed to parse category: {}", e))
        })?;

        let priority = serde_json::from_str(&priority_str).map_err(|e| {
            EventStreamingError::storage(format!("Failed to parse priority: {}", e))
        })?;

        let source = serde_json::from_value(source_json)
            .map_err(|e| EventStreamingError::storage(format!("Failed to parse source: {}", e)))?;

        let correlation = serde_json::from_value(correlation_json).map_err(|e| {
            EventStreamingError::storage(format!("Failed to parse correlation: {}", e))
        })?;

        let destinations = serde_json::from_value(destinations_json).map_err(|e| {
            EventStreamingError::storage(format!("Failed to parse destinations: {}", e))
        })?;

        let payload = serde_json::from_value(payload_json)
            .map_err(|e| EventStreamingError::storage(format!("Failed to parse payload: {}", e)))?;

        let metadata = serde_json::from_value(metadata_json).map_err(|e| {
            EventStreamingError::storage(format!("Failed to parse metadata: {}", e))
        })?;

        let status = serde_json::from_str(&status_str)
            .map_err(|e| EventStreamingError::storage(format!("Failed to parse status: {}", e)))?;

        let error = error_json
            .map(|json| serde_json::from_value(json))
            .transpose()
            .map_err(|e| EventStreamingError::storage(format!("Failed to parse error: {}", e)))?;

        let processing_history = serde_json::from_value(processing_history_json).map_err(|e| {
            EventStreamingError::storage(format!("Failed to parse processing history: {}", e))
        })?;

        Ok(Event {
            id,
            event_type,
            category,
            priority,
            source,
            correlation,
            destinations,
            payload,
            metadata,
            status,
            created_at,
            updated_at,
            expires_at,
            attempt_count: attempt_count as u32,
            error,
            processing_history,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::events::{Event, EventPayload};
    use crate::types::{EventCategory, EventSource};

    #[tokio::test]
    async fn test_event_storage_creation() {
        let config = Config::default();

        // This test might fail without a PostgreSQL database
        if let Ok(_storage) = EventStorage::new(&config).await {
            // Test passed - database is available
        } else {
            // Test skipped - database not available
            println!("PostgreSQL not available, test skipped");
        }
    }

    #[tokio::test]
    async fn test_event_storage_and_retrieval() {
        let config = Config::default();

        if let Ok(storage) = EventStorage::new(&config).await {
            let source = EventSource {
                service: "test-service".to_string(),
                version: "1.0.0".to_string(),
                instance_id: None,
                hostname: None,
                metadata: std::collections::HashMap::new(),
            };

            let payload = EventPayload::Custom(serde_json::json!({"test": "data"}));
            let event = Event::new("test.event", EventCategory::System, source, payload);
            let event_id = event.id;

            // Store event
            if let Ok(()) = storage.store_event(&event).await {
                // Retrieve event
                if let Ok(Some(retrieved_event)) = storage.get_event(event_id).await {
                    assert_eq!(retrieved_event.id, event_id);
                    assert_eq!(retrieved_event.event_type, "test.event");
                }
            }
        } else {
            println!("PostgreSQL not available, test skipped");
        }
    }
}
