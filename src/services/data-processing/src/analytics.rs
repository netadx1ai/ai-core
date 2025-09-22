//! Analytics module for the Data Processing Service
//!
//! This module provides comprehensive analytics capabilities including:
//! - Real-time analytics computation
//! - Statistical analysis and aggregations
//! - Time-series analysis and forecasting
//! - Data visualization support
//! - Business intelligence metrics

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    error::{DataProcessingError, Result},
    types::DataRecord,
};

/// Analytics engine for processing data
pub struct AnalyticsEngine {
    processors: Arc<RwLock<HashMap<String, Box<dyn AnalyticsProcessor + Send + Sync>>>>,
}

/// Analytics processor trait
pub trait AnalyticsProcessor {
    fn process(&self, data: &[DataRecord]) -> Result<AnalyticsResult>;
    fn name(&self) -> &str;
}

/// Analytics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResult {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Basic aggregation processor
pub struct AggregationProcessor {
    name: String,
    operation: AggregationOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationOperation {
    Sum,
    Average,
    Count,
    Min,
    Max,
}

impl AnalyticsEngine {
    pub fn new() -> Self {
        Self {
            processors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_processor(&self, processor: Box<dyn AnalyticsProcessor + Send + Sync>) {
        let mut processors = self.processors.write().await;
        processors.insert(processor.name().to_string(), processor);
    }

    pub async fn process_data(&self, data: &[DataRecord]) -> Result<Vec<AnalyticsResult>> {
        let processors = self.processors.read().await;
        let mut results = Vec::new();

        for processor in processors.values() {
            match processor.process(data) {
                Ok(result) => results.push(result),
                Err(e) => return Err(e),
            }
        }

        Ok(results)
    }
}

impl AggregationProcessor {
    pub fn new(name: String, operation: AggregationOperation) -> Self {
        Self { name, operation }
    }
}

impl AnalyticsProcessor for AggregationProcessor {
    fn process(&self, data: &[DataRecord]) -> Result<AnalyticsResult> {
        if data.is_empty() {
            return Ok(AnalyticsResult {
                metric_name: self.name.clone(),
                value: 0.0,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            });
        }

        let value = match self.operation {
            AggregationOperation::Count => data.len() as f64,
            AggregationOperation::Sum => data.iter().filter_map(|r| r.data.as_f64()).sum(),
            AggregationOperation::Average => {
                let values: Vec<f64> = data.iter().filter_map(|r| r.data.as_f64()).collect();
                if values.is_empty() {
                    0.0
                } else {
                    values.iter().sum::<f64>() / values.len() as f64
                }
            }
            AggregationOperation::Min => data
                .iter()
                .filter_map(|r| r.data.as_f64())
                .fold(f64::INFINITY, f64::min),
            AggregationOperation::Max => data
                .iter()
                .filter_map(|r| r.data.as_f64())
                .fold(f64::NEG_INFINITY, f64::max),
        };

        Ok(AnalyticsResult {
            metric_name: self.name.clone(),
            value,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataRecord;

    #[tokio::test]
    async fn test_analytics_engine() {
        let engine = AnalyticsEngine::new();
        let processor =
            AggregationProcessor::new("test_count".to_string(), AggregationOperation::Count);
        engine.add_processor(Box::new(processor)).await;

        let data = vec![DataRecord::default()];
        let results = engine.process_data(&data).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, 1.0);
    }
}
