//! Type definitions for the metadata store.
//!
//! Shared types used across metadata store submodules.

use serde::{Deserialize, Serialize};

/// Index state for tracking indexing progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    pub workspace_id: String,
    pub last_commit_time: i64,
    pub index_version: i32,
}

/// Indexed file tracking for incremental indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub file_path: String,
    pub workspace_id: String,
    pub last_offset: u64,
    pub file_size: i64,
    pub modified_time: i64,
    pub hash: String, // SHA-256
}

/// Maximum batch insert size to prevent SQL injection and memory overflow
pub(crate) const MAX_BATCH_SIZE: usize = 1000;
