//! Middleware modules for the Federation Service
//!
//! This module provides HTTP middleware components for authentication,
//! rate limiting, request logging, and other cross-cutting concerns.

pub mod auth;
pub mod rate_limit;

// Re-export commonly used types
pub use auth::{AuthContext, AuthMiddleware};
pub use rate_limit::{RateLimitConfig, RateLimitMiddleware};
