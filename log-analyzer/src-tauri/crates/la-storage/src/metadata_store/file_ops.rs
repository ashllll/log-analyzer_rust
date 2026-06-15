//! File metadata CRUD operations.
//!
//! These methods on `MetadataStore` handle file-related database operations:
//! insert, query, update stats, batch operations, and FTS search.

use la_core::error::{AppError, Result};
use la_core::storage_types::FileMetadata;
use sqlx::{Row, SqlitePool};
use tracing::{debug, info};

use super::types::{parse_analysis_status, MAX_BATCH_SIZE};

/// Insert file metadata with CAS deduplication (within a transaction).
pub(crate) async fn insert_file(pool: &SqlitePool, metadata: &FileMetadata) -> Result<i64> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to begin transaction: {e}")))?;

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO files (
            sha256_hash, virtual_path, original_name, size,
            modified_time, mime_type, parent_archive_id, depth_level, created_at,
            min_timestamp, max_timestamp, level_mask, analysis_status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&metadata.sha256_hash)
    .bind(&metadata.virtual_path)
    .bind(&metadata.original_name)
    .bind(metadata.size)
    .bind(metadata.modified_time)
    .bind(&metadata.mime_type)
    .bind(metadata.parent_archive_id)
    .bind(metadata.depth_level)
    .bind(chrono::Utc::now().timestamp())
    .bind(metadata.min_timestamp)
    .bind(metadata.max_timestamp)
    .bind(metadata.level_mask.map(|m| m as i64))
    .bind(metadata.analysis_status.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to insert file: {e}")))?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ? LIMIT 1")
        .bind(&metadata.sha256_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to fetch file ID: {e}")))?
        .0;

    tx.commit()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to commit transaction: {e}")))?;

    debug!(
        id = id,
        hash = %metadata.sha256_hash,
        path = %metadata.virtual_path,
        "Inserted or retrieved existing file metadata (CAS deduplication)"
    );

    Ok(id)
}

/// Get file by virtual path.
pub(crate) async fn get_file_by_virtual_path(
    pool: &SqlitePool,
    virtual_path: &str,
) -> Result<Option<FileMetadata>> {
    let row = sqlx::query("SELECT * FROM files WHERE virtual_path = ?")
        .bind(virtual_path)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to query file: {e}")))?;

    Ok(row.map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r)))
}

/// Get file by SHA-256 hash.
pub(crate) async fn get_file_by_hash(
    pool: &SqlitePool,
    hash: &str,
) -> Result<Option<FileMetadata>> {
    let row = sqlx::query("SELECT * FROM files WHERE sha256_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to query file: {e}")))?;

    Ok(row.map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r)))
}

/// Batch check which hashes have references in the files table.
pub(crate) async fn batch_check_hashes(
    pool: &SqlitePool,
    hashes: &[String],
) -> Result<std::collections::HashSet<String>> {
    use std::collections::HashSet;
    let mut referenced = HashSet::new();
    let batch_size = 1000usize;

    for chunk in hashes.chunks(batch_size) {
        if chunk.is_empty() {
            continue;
        }

        let placeholders: Vec<String> = chunk
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT DISTINCT sha256_hash FROM files WHERE sha256_hash IN ({})",
            placeholders.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for hash in chunk {
            query = query.bind(hash);
        }

        let rows = query
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to batch check hashes: {e}")))?;

        for row in rows {
            let hash: String = row.get("sha256_hash");
            referenced.insert(hash);
        }
    }

    Ok(referenced)
}

/// Get all files in an archive.
pub(crate) async fn get_archive_children(
    pool: &SqlitePool,
    archive_id: i64,
) -> Result<Vec<FileMetadata>> {
    let rows = sqlx::query("SELECT * FROM files WHERE parent_archive_id = ? ORDER BY virtual_path")
        .bind(archive_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to query archive children: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r))
        .collect())
}

/// Get all files.
pub(crate) async fn get_all_files(pool: &SqlitePool) -> Result<Vec<FileMetadata>> {
    let rows = sqlx::query("SELECT * FROM files ORDER BY virtual_path")
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to query all files: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r))
        .collect())
}

/// Count total files.
pub(crate) async fn count_files(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM files")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to count files: {e}")))?;

    Ok(row.get("count"))
}

/// Sum of all file sizes.
pub(crate) async fn sum_file_sizes(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COALESCE(SUM(size), 0) as total FROM files")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to sum file sizes: {e}")))?;

    Ok(row.get("total"))
}

/// Get maximum nesting depth.
pub(crate) async fn get_max_depth(pool: &SqlitePool) -> Result<i32> {
    let row = sqlx::query("SELECT COALESCE(MAX(depth_level), 0) as max_depth FROM files")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to get max depth: {e}")))?;

    Ok(row.get("max_depth"))
}

/// Update file stats (min_timestamp, max_timestamp, level_mask).
pub(crate) async fn update_file_stats(
    pool: &SqlitePool,
    virtual_path: &str,
    min_timestamp: Option<i64>,
    max_timestamp: Option<i64>,
    level_mask: Option<u8>,
) -> Result<()> {
    sqlx::query(
        "UPDATE files SET min_timestamp = ?, max_timestamp = ?, level_mask = ? WHERE virtual_path = ?",
    )
    .bind(min_timestamp)
    .bind(max_timestamp)
    .bind(level_mask.map(|m| m as i64))
    .bind(virtual_path)
    .execute(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to update file stats: {e}")))?;
    Ok(())
}

/// Atomically mark file as READY with stats.
pub(crate) async fn update_file_ready(
    pool: &SqlitePool,
    virtual_path: &str,
    min_timestamp: Option<i64>,
    max_timestamp: Option<i64>,
    level_mask: Option<u8>,
) -> Result<()> {
    sqlx::query(
        "UPDATE files SET min_timestamp = ?, max_timestamp = ?, level_mask = ?, analysis_status = 'READY' WHERE virtual_path = ?",
    )
    .bind(min_timestamp)
    .bind(max_timestamp)
    .bind(level_mask.map(|m| m as i64))
    .bind(virtual_path)
    .execute(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to mark file ready: {e}")))?;
    Ok(())
}

/// Get all files with analysis_status = 'READY'.
pub(crate) async fn get_ready_files(pool: &SqlitePool) -> Result<Vec<FileMetadata>> {
    let rows =
        sqlx::query("SELECT * FROM files WHERE analysis_status = 'READY' ORDER BY virtual_path")
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to query ready files: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r))
        .collect())
}

/// Get files with pruning filters (time range, level mask, file pattern).
pub(crate) async fn get_files_with_pruning(
    pool: &SqlitePool,
    time_start: Option<i64>,
    time_end: Option<i64>,
    level_mask: Option<u8>,
    file_pattern: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let requires_stats = time_start.is_some() || time_end.is_some() || level_mask.is_some();
    let mut sql = if requires_stats {
        String::from("SELECT * FROM files WHERE analysis_status = 'READY'")
    } else {
        String::from("SELECT * FROM files WHERE 1=1")
    };

    if let (Some(_start), Some(_end)) = (time_start, time_end) {
        sql.push_str(
            " AND (min_timestamp IS NULL OR max_timestamp IS NULL OR (min_timestamp <= ? AND max_timestamp >= ?))",
        );
    } else if let Some(_start) = time_start {
        sql.push_str(" AND (max_timestamp IS NULL OR max_timestamp >= ?)");
    } else if let Some(_end) = time_end {
        sql.push_str(" AND (min_timestamp IS NULL OR min_timestamp <= ?)");
    }

    if let Some(_mask) = level_mask {
        sql.push_str(" AND (level_mask IS NULL OR (level_mask & ?) != 0)");
    }

    if file_pattern.is_some() {
        sql.push_str(" AND virtual_path GLOB ?");
    }

    sql.push_str(" ORDER BY virtual_path");

    let mut query = sqlx::query(&sql);

    if let (Some(start), Some(end)) = (time_start, time_end) {
        query = query.bind(end).bind(start);
    } else if let Some(start) = time_start {
        query = query.bind(start);
    } else if let Some(end) = time_end {
        query = query.bind(end);
    }

    if let Some(mask) = level_mask {
        query = query.bind(mask as i64);
    }

    if let Some(pattern) = file_pattern {
        query = query.bind(pattern);
    }

    let rows = query.fetch_all(pool).await.map_err(|e| {
        AppError::database_error(format!("Failed to query files with pruning: {e}"))
    })?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r))
        .collect())
}

/// Search files using FTS5.
pub(crate) async fn search_files(pool: &SqlitePool, query: &str) -> Result<Vec<FileMetadata>> {
    let rows = sqlx::query(
        r#"
        SELECT f.* FROM files f
        JOIN files_fts fts ON f.id = fts.rowid
        WHERE files_fts MATCH ?
        ORDER BY f.virtual_path
        "#,
    )
    .bind(query)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to search files: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| row_to_file_metadata(&r))
        .collect())
}

/// Insert multiple files in a single transaction.
pub(crate) async fn insert_files_batch(
    pool: &SqlitePool,
    files: Vec<FileMetadata>,
) -> Result<Vec<i64>> {
    if files.len() > MAX_BATCH_SIZE {
        return Err(AppError::database_error(format!(
            "Batch size {} exceeds maximum {}",
            files.len(),
            MAX_BATCH_SIZE
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to begin transaction: {e}")))?;

    let mut ids = Vec::with_capacity(files.len());

    for metadata in files {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO files (
                sha256_hash, virtual_path, original_name, size,
                modified_time, mime_type, parent_archive_id, depth_level, created_at,
                min_timestamp, max_timestamp, level_mask, analysis_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&metadata.sha256_hash)
        .bind(&metadata.virtual_path)
        .bind(&metadata.original_name)
        .bind(metadata.size)
        .bind(metadata.modified_time)
        .bind(&metadata.mime_type)
        .bind(metadata.parent_archive_id)
        .bind(metadata.depth_level)
        .bind(chrono::Utc::now().timestamp())
        .bind(metadata.min_timestamp)
        .bind(metadata.max_timestamp)
        .bind(metadata.level_mask.map(|m| m as i64))
        .bind(metadata.analysis_status.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to insert file: {e}")))?;

        let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ? LIMIT 1")
            .bind(&metadata.sha256_hash)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to fetch file ID: {e}")))?
            .0;

        ids.push(id);
    }

    tx.commit()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to commit transaction: {e}")))?;

    info!(
        count = ids.len(),
        "Inserted files in batch transaction (with CAS deduplication)"
    );
    Ok(ids)
}

/// Check if SQLite supports RETURNING clause.
pub(crate) async fn supports_returning_clause(pool: &SqlitePool) -> bool {
    let version: String = match sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(pool)
        .await
    {
        Ok(v) => v,
        Err(_) => return false,
    };

    let parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .take(3)
        .collect();

    if parts.len() < 3 {
        return false;
    }

    (parts[0], parts[1], parts[2]) >= (3, 35, 0)
}

/// Batch insert using RETURNING clause (SQLite 3.35.0+).
pub(crate) async fn insert_files_batch_optimized(
    pool: &SqlitePool,
    files: Vec<FileMetadata>,
) -> Result<Vec<i64>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    if files.len() > MAX_BATCH_SIZE {
        return Err(AppError::database_error(format!(
            "Batch size {} exceeds maximum {}",
            files.len(),
            MAX_BATCH_SIZE
        )));
    }

    if !supports_returning_clause(pool).await {
        return insert_files_batch(pool, files).await;
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to begin transaction: {e}")))?;

    let mut query = String::from(
        "INSERT OR IGNORE INTO files (sha256_hash, virtual_path, original_name, size, modified_time, mime_type, parent_archive_id, depth_level, created_at, min_timestamp, max_timestamp, level_mask, analysis_status) VALUES ",
    );

    let placeholders: Vec<String> = files
        .iter()
        .map(|_| "(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)".to_string())
        .collect();
    query.push_str(&placeholders.join(", "));
    query.push_str(" RETURNING id");

    let mut query_builder = sqlx::query(&query);
    let now = chrono::Utc::now().timestamp();

    for metadata in &files {
        query_builder = query_builder
            .bind(&metadata.sha256_hash)
            .bind(&metadata.virtual_path)
            .bind(&metadata.original_name)
            .bind(metadata.size)
            .bind(metadata.modified_time)
            .bind(&metadata.mime_type)
            .bind(metadata.parent_archive_id)
            .bind(metadata.depth_level)
            .bind(now)
            .bind(metadata.min_timestamp)
            .bind(metadata.max_timestamp)
            .bind(metadata.level_mask.map(|m| m as i64))
            .bind(metadata.analysis_status.as_str());
    }

    let rows = query_builder
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to execute batch insert: {e}")))?;

    let ids: Vec<i64> = rows.iter().map(|row| row.get("id")).collect();

    tx.commit()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to commit transaction: {e}")))?;

    debug!(
        count = ids.len(),
        "Batch insert completed using RETURNING clause"
    );

    Ok(ids)
}

/// Smart batch insert — chooses best implementation by batch size.
pub(crate) async fn insert_files_batch_smart(
    pool: &SqlitePool,
    files: Vec<FileMetadata>,
) -> Result<Vec<i64>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    if files.len() > 10 && supports_returning_clause(pool).await {
        return insert_files_batch_optimized(pool, files).await;
    }

    insert_files_batch(pool, files).await
}

/// Delete all files (for workspace cleanup).
pub(crate) async fn clear_all_files(pool: &SqlitePool) -> Result<()> {
    match sqlx::query("DELETE FROM files WHERE 1=1")
        .execute(pool)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            let err_str = e.to_string();
            if !err_str.contains("no such table") {
                return Err(AppError::database_error(format!(
                    "Failed to delete files: {e}"
                )));
            }
        }
    }

    match sqlx::query("DELETE FROM archives WHERE 1=1")
        .execute(pool)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            let err_str = e.to_string();
            if !err_str.contains("no such table") {
                return Err(AppError::database_error(format!(
                    "Failed to delete archives: {e}"
                )));
            }
        }
    }

    info!("Cleared all files and archives");
    Ok(())
}

/// --- Transaction helpers ---
/// Begin a transaction.
pub(crate) async fn begin_transaction(
    pool: &SqlitePool,
) -> Result<sqlx::Transaction<'static, sqlx::Sqlite>> {
    pool.begin()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to begin transaction: {e}")))
}

/// Insert file within a transaction.
pub(crate) async fn insert_file_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    metadata: &FileMetadata,
) -> Result<i64> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO files (
            sha256_hash, virtual_path, original_name, size,
            modified_time, mime_type, parent_archive_id, depth_level, created_at,
            min_timestamp, max_timestamp, level_mask, analysis_status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&metadata.sha256_hash)
    .bind(&metadata.virtual_path)
    .bind(&metadata.original_name)
    .bind(metadata.size)
    .bind(metadata.modified_time)
    .bind(&metadata.mime_type)
    .bind(metadata.parent_archive_id)
    .bind(metadata.depth_level)
    .bind(chrono::Utc::now().timestamp())
    .bind(metadata.min_timestamp)
    .bind(metadata.max_timestamp)
    .bind(metadata.level_mask.map(|m| m as i64))
    .bind(metadata.analysis_status.as_str())
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to insert file in transaction: {e}")))?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ? LIMIT 1")
        .bind(&metadata.sha256_hash)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to fetch file ID in transaction: {e}"))
        })?
        .0;

    debug!(
        id = id,
        hash = %metadata.sha256_hash,
        path = %metadata.virtual_path,
        "Inserted or retrieved existing file metadata in transaction (CAS deduplication)"
    );

    Ok(id)
}

/// Convert a database row to FileMetadata.
fn row_to_file_metadata(r: &sqlx::sqlite::SqliteRow) -> FileMetadata {
    FileMetadata {
        id: r.get("id"),
        sha256_hash: r.get("sha256_hash"),
        virtual_path: r.get("virtual_path"),
        original_name: r.get("original_name"),
        size: r.get("size"),
        modified_time: r.get("modified_time"),
        mime_type: r.get("mime_type"),
        parent_archive_id: r.get("parent_archive_id"),
        depth_level: r.get("depth_level"),
        min_timestamp: r.try_get("min_timestamp").ok(),
        max_timestamp: r.try_get("max_timestamp").ok(),
        level_mask: r.try_get("level_mask").ok(),
        analysis_status: parse_analysis_status(r),
    }
}
