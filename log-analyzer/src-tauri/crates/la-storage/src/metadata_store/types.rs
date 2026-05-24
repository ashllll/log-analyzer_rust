//! Type definitions for the metadata store.
//!
//! Shared types used across metadata store submodules.

use serde::{Deserialize, Serialize};
use sqlx::Row;

use la_core::storage_types::AnalysisStatus;

/// Parse analysis_status from a database row
pub(crate) fn parse_analysis_status(row: &sqlx::sqlite::SqliteRow) -> AnalysisStatus {
    row.try_get::<String, _>("analysis_status")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(AnalysisStatus::Pending)
}

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
