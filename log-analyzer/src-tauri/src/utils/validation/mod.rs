//! Validation — domain newtypes + legacy helpers.
//!
//! Newtypes (preferred): make invalid states unrepresentable.
//! Legacy helpers: kept for backward compatibility; delegate to newtypes.

pub mod helpers; // Old validation.rs functions
pub mod query_string;
pub mod safe_path;
pub mod workspace_id;

// ── Newtypes ──
pub use query_string::{QueryString, MAX_QUERY_LENGTH};
pub use safe_path::SafePath;
pub use workspace_id::{WorkspaceId, MAX_WORKSPACE_ID_LENGTH};

// ── Re-export legacy helpers so existing callers don't break ──
pub use helpers::*;

// ── Property tests ──
#[cfg(test)]
mod validation_property_tests;
