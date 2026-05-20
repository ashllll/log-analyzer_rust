//! Interfaces layer — thin Tauri command adapters.
//!
//! Commands here are thin wrappers: they extract parameters, instantiate
//! the application context, delegate to use cases, and return results.
//! No business logic lives here.

// For now, commands remain in commands/ and will migrate here gradually.
// This module exists to establish the architectural pattern.
