//! HTTP Request Handlers Module
//!
//! This module contains all HTTP request handlers for the MCP Manager Service,
//! organized by functionality area. Each handler is responsible for processing
//! specific API endpoints and coordinating with service components.

pub mod health;
pub mod load_balancer;
pub mod metrics;
pub mod protocol;
pub mod registry;
pub mod servers;
pub mod status;

// Re-exports for convenience.
// Most modules are glob-exported. Modules with naming conflicts are handled explicitly
// to prevent ambiguity in the public API.

pub use health::*;
pub use metrics::*;
pub use protocol::*;
pub use servers::*;
pub use status::*;

// Explicitly re-export from `load_balancer` to resolve conflicts.
// The conflicting `get_statistics` function is renamed to `get_load_balancer_statistics`.
pub use load_balancer::{
    get_statistics as get_load_balancer_statistics, select_server, update_weights,
};

// Explicitly re-export from `registry` to resolve conflicts.
// The conflicting `get_statistics` function is renamed to `get_registry_statistics`.
pub use registry::{cleanup_stale, get_statistics as get_registry_statistics};
