//! Middleware Module
//!
//! This module provides HTTP middleware for the MCP Manager Service,
//! including authentication, rate limiting, request logging, and other
//! cross-cutting concerns.

pub mod auth;
pub mod rate_limit;
pub mod request_logging;

// Re-exports for convenience
pub use auth::*;
pub use rate_limit::*;
pub use request_logging::*;
