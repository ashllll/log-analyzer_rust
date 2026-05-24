//! Index state and indexed file tracking operations.
//!
//! Manages incremental indexing state: which files have been indexed,
//! at what offset, and with what hash.

use la_core::error::{AppError, Result};
use sqlx::{Row, SqlitePool};
use tracing::debug;

use super::types::{IndexState, IndexedFile};

/// Save index state for a workspace (UPSERT).
pub(crate) async fn save_index_state(pool: &SqlitePool, state: &IndexState) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO index_state (workspace_id, last_commit_time, index_version)
        VALUES (?, ?, ?)
        ON CONFLICT(workspace_id) DO UPDATE SET
            last_commit_time = excluded.last_commit_time,
            index_version = excluded.index_version
        "#,
    )
    .bind(&state.workspace_id)
    .bind(state.last_commit_time)
    .bind(state.index_version)
    .execute(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to save index state: {}", e)))?;

    debug!(
        workspace_id = %state.workspace_id,
        last_commit_time = state.last_commit_time,
        index_version = state.index_version,
        "Saved index state"
    );

    Ok(())
}

/// Load index state for a workspace.
pub(crate) async fn load_index_state(
    pool: &SqlitePool,
    workspace_id: &str,
) -> Result<Option<IndexState>> {
    let row =
        sqlx::query("SELECT workspace_id, last_commit_time, index_version FROM index_state WHERE workspace_id = ?")
            .bind(workspace_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to load index state: {}", e)))?;

    Ok(row.map(|r: sqlx::sqlite::SqliteRow| IndexState {
        workspace_id: r.get("workspace_id"),
        last_commit_time: r.get("last_commit_time"),
        index_version: r.get("index_version"),
    }))
}

/// Save indexed file record (UPSERT).
pub(crate) async fn save_indexed_file(pool: &SqlitePool, file: &IndexedFile) -> Result<()> {
    // Ensure workspace exists in index_state before inserting indexed file
    let workspace_result = sqlx::query(
        "INSERT OR IGNORE INTO index_state (workspace_id, last_commit_time, index_version) VALUES (?, 0, 1)"
    )
    .bind(&file.workspace_id)
    .execute(pool)
    .await;

    match workspace_result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                debug!(workspace_id = %file.workspace_id, "Created new workspace entry in index_state");
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("constraint") || error_msg.contains("FOREIGN KEY") {
                return Err(AppError::database_error(format!(
                    "Foreign key constraint failed when ensuring workspace exists: {}. This may indicate database corruption.",
                    e
                )));
            }
            return Err(AppError::database_error(format!(
                "Failed to ensure workspace exists: {}",
                e
            )));
        }
    }

    let insert_result = sqlx::query(
        r#"
        INSERT INTO indexed_files (file_path, workspace_id, last_offset, file_size, modified_time, hash)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(file_path) DO UPDATE SET
            last_offset = excluded.last_offset,
            file_size = excluded.file_size,
            modified_time = excluded.modified_time,
            hash = excluded.hash
        "#,
    )
    .bind(&file.file_path)
    .bind(&file.workspace_id)
    .bind(file.last_offset as i64)
    .bind(file.file_size)
    .bind(file.modified_time)
    .bind(&file.hash)
    .execute(pool)
    .await;

    match insert_result {
        Ok(result) => {
            debug!(
                file_path = %file.file_path,
                workspace_id = %file.workspace_id,
                last_offset = file.last_offset,
                rows_affected = result.rows_affected(),
                "Saved indexed file record"
            );
            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("FOREIGN KEY") {
                Err(AppError::database_error(format!(
                    "Foreign key constraint failed when saving indexed file. File: {}, Workspace: {}. Error: {}. This may indicate the workspace was deleted.",
                    file.file_path, file.workspace_id, e
                )))
            } else if error_msg.contains("UNIQUE") {
                Err(AppError::database_error(format!(
                    "Unique constraint violation when saving indexed file. File: {}. Error: {}",
                    file.file_path, e
                )))
            } else {
                Err(AppError::database_error(format!(
                    "Failed to save indexed file: {}",
                    e
                )))
            }
        }
    }
}

/// Load all indexed files for a workspace.
pub(crate) async fn load_indexed_files(
    pool: &SqlitePool,
    workspace_id: &str,
) -> Result<Vec<IndexedFile>> {
    let rows = sqlx::query(
        "SELECT file_path, workspace_id, last_offset, file_size, modified_time, hash FROM indexed_files WHERE workspace_id = ?"
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to load indexed files: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| IndexedFile {
            file_path: r.get("file_path"),
            workspace_id: r.get("workspace_id"),
            last_offset: {
                let val: i64 = r.get("last_offset");
                val as u64
            },
            file_size: r.get("file_size"),
            modified_time: r.get("modified_time"),
            hash: r.get("hash"),
        })
        .collect())
}

/// Load a single indexed file record by path.
pub(crate) async fn load_indexed_file(
    pool: &SqlitePool,
    file_path: &str,
) -> Result<Option<IndexedFile>> {
    let row = sqlx::query(
        "SELECT file_path, workspace_id, last_offset, file_size, modified_time, hash FROM indexed_files WHERE file_path = ?"
    )
    .bind(file_path)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to load indexed file: {}", e)))?;

    Ok(row.map(|r: sqlx::sqlite::SqliteRow| IndexedFile {
        file_path: r.get("file_path"),
        workspace_id: r.get("workspace_id"),
        last_offset: {
            let val: i64 = r.get("last_offset");
            val as u64
        },
        file_size: r.get("file_size"),
        modified_time: r.get("modified_time"),
        hash: r.get("hash"),
    }))
}

/// Delete an indexed file record.
pub(crate) async fn delete_indexed_file(pool: &SqlitePool, file_path: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM indexed_files WHERE file_path = ?")
        .bind(file_path)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to delete indexed file: {}", e)))?;

    debug!(
        file_path = %file_path,
        rows_affected = result.rows_affected(),
        "Deleted indexed file record"
    );

    Ok(())
}

/// Clear all indexed files for a workspace.
pub(crate) async fn clear_indexed_files(pool: &SqlitePool, workspace_id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM indexed_files WHERE workspace_id = ?")
        .bind(workspace_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to clear indexed files: {}", e)))?;

    debug!(
        workspace_id = %workspace_id,
        rows_affected = result.rows_affected(),
        "Cleared indexed files for workspace"
    );

    Ok(())
}
