//! Windowing module for the Data Processing Service
//!
//! This module provides comprehensive windowing capabilities for stream processing including:
//! - Tumbling windows (non-overlapping fixed-size windows)
//! - Sliding windows (overlapping fixed-size windows)
//! - Session windows (dynamic windows based on inactivity gaps)
//! - Global windows (single window for all data)
//! - Custom windowing strategies

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, DurationRound, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    error::{DataProcessingError, Result, StreamProcessingError},
    types::{DataRecord, WindowType},
};

/// Window manager for handling different window types and operations
pub struct WindowManager {
    windows: Arc<DashMap<String, Window>>,
    window_configs: Arc<RwLock<HashMap<String, WindowConfig>>>,
    watermark_manager: Arc<WatermarkManager>,
}

/// Individual window containing records
#[derive(Debug, Clone)]
pub struct Window {
    pub id: String,
    pub window_type: WindowType,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub records: Vec<DataRecord>,
    pub is_complete: bool,
    pub is_fired: bool,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub name: String,
    pub window_type: WindowType,
    pub key_extractor: KeyExtractor,
    pub timestamp_extractor: TimestampExtractor,
    pub allowed_lateness: Duration,
    pub trigger: WindowTrigger,
    pub evictor: Option<WindowEvictor>,
}

/// Key extraction strategy for partitioning data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyExtractor {
    /// Extract key from a specific field
    Field(String),
    /// Extract key from multiple fields (composite key)
    CompositeFields(Vec<String>),
    /// Use a custom extraction function
    Custom(String),
    /// No key extraction (all records go to same partition)
    None,
}

/// Timestamp extraction strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimestampExtractor {
    /// Use event time from a specific field
    EventTime(String),
    /// Use processing time (when record is processed)
    ProcessingTime,
    /// Use ingestion time (when record was ingested)
    IngestionTime,
}

/// Window trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowTrigger {
    /// Fire when window is complete (event time past window end + allowed lateness)
    EventTime,
    /// Fire when processing time reaches window end
    ProcessingTime,
    /// Fire when a certain number of elements is reached
    Count(usize),
    /// Fire after a processing time delay
    ProcessingTimeDelay(Duration),
    /// Fire on every element
    Continually,
    /// Composite trigger combining multiple conditions
    Composite {
        triggers: Vec<WindowTrigger>,
        logic: TriggerLogic,
    },
}

/// Logic for combining multiple triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerLogic {
    And,
    Or,
}

/// Window evictor for removing elements from windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowEvictor {
    /// Keep only the last N elements
    Count(usize),
    /// Keep only elements within a time range
    Time(Duration),
    /// Custom evictor logic
    Custom(String),
}

/// Window assignment result
#[derive(Debug, Clone)]
pub struct WindowAssignment {
    pub window_id: String,
    pub window_key: String,
    pub assigned_windows: Vec<Window>,
}

/// Watermark manager for handling event-time processing
pub struct WatermarkManager {
    watermarks: DashMap<String, DateTime<Utc>>,
    watermark_strategy: WatermarkStrategy,
}

/// Watermark generation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatermarkStrategy {
    /// Fixed delay from the maximum timestamp seen
    BoundedOutOfOrderness(Duration),
    /// Watermark based on percentile of timestamps
    Percentile {
        percentile: f64,
        window_size: Duration,
    },
    /// Custom watermark generation
    Custom(String),
}

/// Session window state for tracking sessions
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub key: String,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub gap_duration: Duration,
    pub is_active: bool,
}

impl WindowManager {
    /// Create a new window manager
    pub fn new() -> Self {
        Self {
            windows: Arc::new(DashMap::new()),
            window_configs: Arc::new(RwLock::new(HashMap::new())),
            watermark_manager: Arc::new(WatermarkManager::new(
                WatermarkStrategy::BoundedOutOfOrderness(Duration::from_secs(10)),
            )),
        }
    }

    /// Add a window configuration
    pub async fn add_window_config(&self, config: WindowConfig) -> Result<()> {
        let mut configs = self.window_configs.write().await;
        configs.insert(config.name.clone(), config);
        Ok(())
    }

    /// Assign a record to appropriate windows
    pub async fn assign_to_windows(&self, record: &DataRecord) -> Result<Vec<WindowAssignment>> {
        let configs = self.window_configs.read().await;
        let mut assignments = Vec::new();

        for (name, config) in configs.iter() {
            let key = self.extract_key(record, &config.key_extractor)?;
            let timestamp = self.extract_timestamp(record, &config.timestamp_extractor)?;

            let windows = self.create_windows_for_record(config, &key, timestamp)?;

            for mut window in windows {
                // Add record to window
                window.records.push(record.clone());
                window.last_updated = Utc::now();

                // Store or update window
                self.windows.insert(window.id.clone(), window.clone());

                assignments.push(WindowAssignment {
                    window_id: window.id.clone(),
                    window_key: key.clone(),
                    assigned_windows: vec![window],
                });
            }
        }

        Ok(assignments)
    }

    /// Get windows that are ready to fire
    pub async fn get_ready_windows(&self) -> Vec<Window> {
        let mut ready_windows = Vec::new();

        for entry in self.windows.iter() {
            let window = entry.value();
            if !window.is_fired && self.should_fire_window(window).await {
                ready_windows.push(window.clone());
            }
        }

        ready_windows
    }

    /// Mark a window as fired
    pub async fn mark_window_fired(&self, window_id: &str) -> Result<()> {
        if let Some(mut window_entry) = self.windows.get_mut(window_id) {
            window_entry.is_fired = true;
            debug!("Window {} marked as fired", window_id);
        } else {
            warn!(
                "Window {} not found when trying to mark as fired",
                window_id
            );
        }
        Ok(())
    }

    /// Clean up expired windows
    pub async fn cleanup_expired_windows(&self) -> Result<usize> {
        let now = Utc::now();
        let mut removed_count = 0;

        // Remove windows that are old and have been fired
        let expired_windows: Vec<String> = self
            .windows
            .iter()
            .filter(|entry| {
                let window = entry.value();
                window.is_fired && (now - window.end_time).num_hours() > 24 // Keep for 24 hours after end
            })
            .map(|entry| entry.key().clone())
            .collect();

        for window_id in expired_windows {
            self.windows.remove(&window_id);
            removed_count += 1;
        }

        debug!("Cleaned up {} expired windows", removed_count);
        Ok(removed_count)
    }

    /// Extract key from record based on key extractor
    fn extract_key(&self, record: &DataRecord, extractor: &KeyExtractor) -> Result<String> {
        match extractor {
            KeyExtractor::Field(field_name) => {
                if let Some(value) = record.data.get(field_name) {
                    Ok(value.to_string())
                } else {
                    Ok("default".to_string())
                }
            }
            KeyExtractor::CompositeFields(fields) => {
                let mut key_parts = Vec::new();
                for field in fields {
                    if let Some(value) = record.data.get(field) {
                        key_parts.push(value.to_string());
                    } else {
                        key_parts.push("null".to_string());
                    }
                }
                Ok(key_parts.join("|"))
            }
            KeyExtractor::Custom(_function) => {
                // In a real implementation, this would execute custom key extraction logic
                Ok("custom_key".to_string())
            }
            KeyExtractor::None => Ok("global".to_string()),
        }
    }

    /// Extract timestamp from record based on timestamp extractor
    fn extract_timestamp(
        &self,
        record: &DataRecord,
        extractor: &TimestampExtractor,
    ) -> Result<DateTime<Utc>> {
        match extractor {
            TimestampExtractor::EventTime(field_name) => {
                if let Some(value) = record.data.get(field_name) {
                    if let Some(timestamp_str) = value.as_str() {
                        chrono::DateTime::parse_from_rfc3339(timestamp_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .map_err(|e| {
                                DataProcessingError::validation(
                                    field_name,
                                    format!("Invalid timestamp: {}", e),
                                )
                            })
                    } else {
                        Ok(record.timestamp)
                    }
                } else {
                    Ok(record.timestamp)
                }
            }
            TimestampExtractor::ProcessingTime => Ok(Utc::now()),
            TimestampExtractor::IngestionTime => Ok(record.timestamp),
        }
    }

    /// Create windows for a record based on window type
    fn create_windows_for_record(
        &self,
        config: &WindowConfig,
        key: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<Vec<Window>> {
        match &config.window_type {
            WindowType::Tumbling { size_secs } => {
                let size = Duration::from_secs(*size_secs);
                let window_start = timestamp
                    .duration_trunc(chrono::Duration::from_std(size).unwrap())
                    .unwrap();
                let window_end = window_start + chrono::Duration::from_std(size).unwrap();

                let window = Window {
                    id: format!(
                        "tumbling-{}-{}-{}",
                        key,
                        window_start.timestamp(),
                        size_secs
                    ),
                    window_type: config.window_type.clone(),
                    start_time: window_start,
                    end_time: window_end,
                    records: Vec::new(),
                    is_complete: false,
                    is_fired: false,
                    created_at: Utc::now(),
                    last_updated: Utc::now(),
                };

                Ok(vec![window])
            }
            WindowType::Sliding {
                size_secs,
                slide_secs,
            } => {
                let size = Duration::from_secs(*size_secs);
                let slide = Duration::from_secs(*slide_secs);
                let mut windows = Vec::new();

                // Calculate how many sliding windows this record belongs to
                let slide_duration = chrono::Duration::from_std(slide).unwrap();
                let size_duration = chrono::Duration::from_std(size).unwrap();

                let mut window_start = timestamp.duration_trunc(slide_duration).unwrap();

                // Go back to find the earliest window that includes this timestamp
                while window_start + size_duration > timestamp {
                    window_start = window_start - slide_duration;
                }

                // Create all windows that include this timestamp
                while window_start <= timestamp {
                    let window_end = window_start + size_duration;

                    if timestamp >= window_start && timestamp < window_end {
                        let window = Window {
                            id: format!(
                                "sliding-{}-{}-{}-{}",
                                key,
                                window_start.timestamp(),
                                size_secs,
                                slide_secs
                            ),
                            window_type: config.window_type.clone(),
                            start_time: window_start,
                            end_time: window_end,
                            records: Vec::new(),
                            is_complete: false,
                            is_fired: false,
                            created_at: Utc::now(),
                            last_updated: Utc::now(),
                        };
                        windows.push(window);
                    }

                    window_start = window_start + slide_duration;
                }

                Ok(windows)
            }
            WindowType::Session { gap_secs } => {
                // Session windows require more complex logic to merge sessions
                // For now, create a simple session window
                let gap = Duration::from_secs(*gap_secs);

                let window = Window {
                    id: format!("session-{}-{}", key, Uuid::new_v4()),
                    window_type: config.window_type.clone(),
                    start_time: timestamp,
                    end_time: timestamp + chrono::Duration::from_std(gap).unwrap(),
                    records: Vec::new(),
                    is_complete: false,
                    is_fired: false,
                    created_at: Utc::now(),
                    last_updated: Utc::now(),
                };

                Ok(vec![window])
            }
            WindowType::Global => {
                let window = Window {
                    id: format!("global-{}", key),
                    window_type: config.window_type.clone(),
                    start_time: DateTime::<Utc>::MIN_UTC,
                    end_time: DateTime::<Utc>::MAX_UTC,
                    records: Vec::new(),
                    is_complete: false,
                    is_fired: false,
                    created_at: Utc::now(),
                    last_updated: Utc::now(),
                };

                Ok(vec![window])
            }
        }
    }

    /// Check if a window should fire based on its trigger conditions
    async fn should_fire_window(&self, window: &Window) -> bool {
        let now = Utc::now();

        // Simple firing logic - in practice this would be more sophisticated
        match &window.window_type {
            WindowType::Tumbling { .. } | WindowType::Sliding { .. } => {
                // Fire if current time is past window end time
                now >= window.end_time
            }
            WindowType::Session { gap_secs } => {
                // Fire if no activity for the gap duration
                let gap = chrono::Duration::seconds(*gap_secs as i64);
                (now - window.last_updated) >= gap
            }
            WindowType::Global => {
                // Global windows typically fire on external triggers
                false
            }
        }
    }
}

impl WatermarkManager {
    fn new(strategy: WatermarkStrategy) -> Self {
        Self {
            watermarks: DashMap::new(),
            watermark_strategy: strategy,
        }
    }

    /// Update watermark for a source
    pub fn update_watermark(&self, source: &str, timestamp: DateTime<Utc>) {
        self.watermarks.insert(source.to_string(), timestamp);
    }

    /// Get current watermark for a source
    pub fn get_watermark(&self, source: &str) -> Option<DateTime<Utc>> {
        self.watermarks.get(source).map(|entry| *entry.value())
    }

    /// Get global watermark (minimum of all source watermarks)
    pub fn get_global_watermark(&self) -> Option<DateTime<Utc>> {
        self.watermarks.iter().map(|entry| *entry.value()).min()
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataRecord;
    use serde_json::json;

    #[tokio::test]
    async fn test_tumbling_window_assignment() {
        let manager = WindowManager::new();

        let config = WindowConfig {
            name: "test_tumbling".to_string(),
            window_type: WindowType::Tumbling { size_secs: 60 },
            key_extractor: KeyExtractor::Field("user_id".to_string()),
            timestamp_extractor: TimestampExtractor::EventTime("timestamp".to_string()),
            allowed_lateness: Duration::from_secs(10),
            trigger: WindowTrigger::EventTime,
            evictor: None,
        };

        manager.add_window_config(config).await.unwrap();

        let mut record = DataRecord::default();
        record.data = json!({
            "user_id": "user123",
            "timestamp": "2023-01-01T10:00:00Z",
            "value": 42
        });

        let assignments = manager.assign_to_windows(&record).await.unwrap();

        assert_eq!(assignments.len(), 1);
        assert!(assignments[0].window_id.contains("tumbling"));
        assert_eq!(assignments[0].window_key, "user123");
    }

    #[tokio::test]
    async fn test_sliding_window_assignment() {
        let manager = WindowManager::new();

        let config = WindowConfig {
            name: "test_sliding".to_string(),
            window_type: WindowType::Sliding {
                size_secs: 120,
                slide_secs: 60,
            },
            key_extractor: KeyExtractor::None,
            timestamp_extractor: TimestampExtractor::ProcessingTime,
            allowed_lateness: Duration::from_secs(10),
            trigger: WindowTrigger::EventTime,
            evictor: None,
        };

        manager.add_window_config(config).await.unwrap();

        let record = DataRecord::default();
        let assignments = manager.assign_to_windows(&record).await.unwrap();

        // Sliding windows should create multiple overlapping windows
        assert!(!assignments.is_empty());
    }

    #[test]
    fn test_key_extraction() {
        let manager = WindowManager::new();

        let mut record = DataRecord::default();
        record.data = json!({
            "user_id": "user123",
            "session_id": "session456"
        });

        // Test field key extraction
        let key = manager
            .extract_key(&record, &KeyExtractor::Field("user_id".to_string()))
            .unwrap();
        assert_eq!(key, "\"user123\"");

        // Test composite key extraction
        let composite_key = manager
            .extract_key(
                &record,
                &KeyExtractor::CompositeFields(vec![
                    "user_id".to_string(),
                    "session_id".to_string(),
                ]),
            )
            .unwrap();
        assert!(composite_key.contains("user123"));
        assert!(composite_key.contains("session456"));
    }

    #[tokio::test]
    async fn test_window_cleanup() {
        let manager = WindowManager::new();

        // Create an expired, fired window
        let window = Window {
            id: "test_window".to_string(),
            window_type: WindowType::Tumbling { size_secs: 60 },
            start_time: Utc::now() - chrono::Duration::days(2),
            end_time: Utc::now() - chrono::Duration::days(2) + chrono::Duration::hours(1),
            records: Vec::new(),
            is_complete: true,
            is_fired: true,
            created_at: Utc::now() - chrono::Duration::days(2),
            last_updated: Utc::now() - chrono::Duration::days(2),
        };

        manager.windows.insert(window.id.clone(), window);

        let cleaned_count = manager.cleanup_expired_windows().await.unwrap();
        assert_eq!(cleaned_count, 1);
    }
}
