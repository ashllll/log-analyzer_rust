//! Archive metadata CRUD operations.

use la_core::error::{AppError, Result};
use la_core::storage_types::ArchiveMetadata;
use sqlx::{Row, SqlitePool};
use tracing::debug;

/// Insert archive metadata with deduplication.
pub(crate) async fn insert_archive(pool: &SqlitePool, metadata: &ArchiveMetadata) -> Result<i64> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to begin transaction: {e}")))?;

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO archives (
            sha256_hash, virtual_path, original_name, archive_type,
            parent_archive_id, depth_level, extraction_status, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&metadata.sha256_hash)
    .bind(&metadata.virtual_path)
    .bind(&metadata.original_name)
    .bind(&metadata.archive_type)
    .bind(metadata.parent_archive_id)
    .bind(metadata.depth_level)
    .bind(&metadata.extraction_status)
    .bind(chrono::Utc::now().timestamp())
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to insert archive: {e}")))?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM archives WHERE sha256_hash = ? LIMIT 1")
        .bind(&metadata.sha256_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to fetch archive ID: {e}")))?
        .0;

    tx.commit()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to commit transaction: {e}")))?;

    debug!(
        id = id,
        hash = %metadata.sha256_hash,
        path = %metadata.virtual_path,
        "Inserted or retrieved existing archive metadata (CAS deduplication)"
    );

    Ok(id)
}

/// Count total archives.
pub(crate) async fn count_archives(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM archives")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to count archives: {e}")))?;

    Ok(row.get("count"))
}

/// Update archive extraction status.
pub(crate) async fn update_archive_status(
    pool: &SqlitePool,
    archive_id: i64,
    status: &str,
) -> Result<()> {
    sqlx::query("UPDATE archives SET extraction_status = ? WHERE id = ?")
        .bind(status)
        .bind(archive_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to update archive status: {e}")))?;

    debug!(archive_id = archive_id, status = %status, "Updated archive status");
    Ok(())
}

/// Get archive by ID.
pub(crate) async fn get_archive_by_id(
    pool: &SqlitePool,
    archive_id: i64,
) -> Result<Option<ArchiveMetadata>> {
    let row = sqlx::query("SELECT * FROM archives WHERE id = ?")
        .bind(archive_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to query archive: {e}")))?;

    Ok(row.map(|r: sqlx::sqlite::SqliteRow| ArchiveMetadata {
        id: r.get("id"),
        sha256_hash: r.get("sha256_hash"),
        virtual_path: r.get("virtual_path"),
        original_name: r.get("original_name"),
        archive_type: r.get("archive_type"),
        parent_archive_id: r.get("parent_archive_id"),
        depth_level: r.get("depth_level"),
        extraction_status: r.get("extraction_status"),
    }))
}

/// Get all archives.
pub(crate) async fn get_all_archives(pool: &SqlitePool) -> Result<Vec<ArchiveMetadata>> {
    let rows = sqlx::query("SELECT * FROM archives ORDER BY virtual_path")
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to query all archives: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| ArchiveMetadata {
            id: r.get("id"),
            sha256_hash: r.get("sha256_hash"),
            virtual_path: r.get("virtual_path"),
            original_name: r.get("original_name"),
            archive_type: r.get("archive_type"),
            parent_archive_id: r.get("parent_archive_id"),
            depth_level: r.get("depth_level"),
            extraction_status: r.get("extraction_status"),
        })
        .collect())
}

/// Insert archive within a transaction.
pub(crate) async fn insert_archive_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    metadata: &ArchiveMetadata,
) -> Result<i64> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO archives (
            sha256_hash, virtual_path, original_name, archive_type,
            parent_archive_id, depth_level, extraction_status, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&metadata.sha256_hash)
    .bind(&metadata.virtual_path)
    .bind(&metadata.original_name)
    .bind(&metadata.archive_type)
    .bind(metadata.parent_archive_id)
    .bind(metadata.depth_level)
    .bind(&metadata.extraction_status)
    .bind(chrono::Utc::now().timestamp())
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        AppError::database_error(format!("Failed to insert archive in transaction: {e}"))
    })?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM archives WHERE sha256_hash = ? LIMIT 1")
        .bind(&metadata.sha256_hash)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to fetch archive ID in transaction: {e}"))
        })?
        .0;

    debug!(
        id = id,
        hash = %metadata.sha256_hash,
        path = %metadata.virtual_path,
        "Inserted or retrieved existing archive metadata in transaction (CAS deduplication)"
    );

    Ok(id)
}
