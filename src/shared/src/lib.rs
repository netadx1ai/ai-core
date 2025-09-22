//! Shared types and utilities for the AI-CORE Platform

pub mod config;
pub mod types;

// Export config types with different names to avoid conflicts
pub use config::{
    AuthConfig as ConfigAuthConfig, DatabaseConfig, ExternalServiceConfig, ObservabilityConfig,
    RateLimitConfig as ConfigRateLimitConfig, RedisConfig, RoutingConfig, SecurityConfig,
    ServerConfig, ServiceConfig, TemporalConfig,
};

// Export all types from types module
pub use types::*;
