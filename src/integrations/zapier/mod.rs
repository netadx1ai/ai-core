//! Zapier Integration Module
//!
//! This module provides functionality for integrating with Zapier. The primary
//! mechanism is a webhook receiver that can process incoming data from Zapier "Zaps."
//!
//! ## Components
//!
//! - `handlers`: Contains the Axum HTTP handlers for receiving webhook requests.
//! - `models`: Defines the data structures for Zapier webhook payloads.
//! - `security`: Implements signature validation to ensure webhooks are authentic.

// Public modules within the Zapier integration crate.
pub mod handlers;
pub mod models;
pub mod security;

// Re-export key components for easier use throughout the application.
// This allows other parts of the codebase to use `zapier::handle_webhook`
// instead of `zapier::handlers::handle_webhook`.
pub use handlers::handle_webhook;
