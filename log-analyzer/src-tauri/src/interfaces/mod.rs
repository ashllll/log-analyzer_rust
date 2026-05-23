//! Interfaces layer — thin Tauri command adapters.
//!
//! Commands here are thin wrappers: they extract parameters, instantiate
//! the application context, delegate to use cases, and return results.
//! No business logic lives here.

// For now, commands remain in commands/ and will migrate here gradually.
// This module exists to establish the architectural pattern.
pub mod config;
pub mod import;
pub mod log_config;
pub mod search;
pub mod state_sync;
pub mod validation;
pub mod virtual_tree;
pub mod watch;
pub mod workspace;
