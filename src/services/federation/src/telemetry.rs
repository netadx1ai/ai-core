//! Telemetry and observability for the Federation Service
//!
//! This module provides comprehensive telemetry capabilities including structured logging,
//! metrics collection, distributed tracing, and observability integrations for the
//! federation service.

use crate::config::TelemetryConfig;
use crate::models::FederationError;
use anyhow::Result;
use serde_json;
use std::collections::HashMap;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize telemetry system
pub fn init_tracing(config: &TelemetryConfig) -> Result<(), FederationError> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    if config.tracing.enabled {
        match config.logging.format.as_str() {
            "json" => {
                let subscriber = tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer().json().with_target(true).with_thread_ids(true));

                tracing::subscriber::set_global_default(subscriber).map_err(|e| {
                    FederationError::ConfigurationError {
                        message: format!("Failed to set tracing subscriber: {}", e),
                    }
                })?;
            }
            _ => {
                let subscriber = tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer().with_target(true).with_thread_ids(true));

                tracing::subscriber::set_global_default(subscriber).map_err(|e| {
                    FederationError::ConfigurationError {
                        message: format!("Failed to set tracing subscriber: {}", e),
                    }
                })?;
            }
        }
    }

    Ok(())
}

/// Telemetry manager for metrics and tracing
#[derive(Debug, Clone)]
pub struct TelemetryManager {
    config: TelemetryConfig,
}

impl TelemetryManager {
    pub async fn new(config: TelemetryConfig) -> Result<Self, FederationError> {
        Ok(Self { config })
    }

    pub async fn record_metric(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        tracing::info!(
            metric_name = %name,
            metric_value = %value,
            labels = ?labels,
            "Recording metric"
        );
    }

    pub async fn start_span(&self, name: &str) -> tracing::Span {
        tracing::info_span!("federation_span", span_name = %name)
    }

    pub async fn health(&self) -> Result<serde_json::Value, FederationError> {
        Ok(serde_json::json!({
            "status": "healthy",
            "logging_level": self.config.logging.level,
            "tracing_enabled": self.config.tracing.enabled,
            "metrics_enabled": self.config.metrics.enabled
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_manager_creation() {
        let config = TelemetryConfig::default();
        let manager = TelemetryManager::new(config).await.unwrap();
        assert!(manager.config.logging.level.len() > 0);
    }
}
