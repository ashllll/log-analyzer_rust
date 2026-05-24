//! SQLite Metadata Store
//!
//! Manages file and archive metadata using SQLite.
//! Provides fast lookups and full-text search capabilities.
//!
//! ## Module structure
//!
//! - `types` — shared type definitions (IndexState, IndexedFile)
//! - `schema` — database schema initialization and migrations
//! - `file_ops` — file metadata CRUD operations
//! - `archive_ops` — archive metadata CRUD operations
//! - `index_ops` — incremental indexing state management

mod archive_ops;
mod file_ops;
mod index_ops;
mod schema;
mod types;

use async_trait::async_trait;
use la_core::error::{AppError, Result};
pub use la_core::storage_types::{ArchiveMetadata, FileMetadata};
use la_core::traits::MetadataStorage;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;
use tracing::info;

// ── Re-exports ──
pub use la_core::storage_types::AnalysisStatus;
pub use types::{IndexState, IndexedFile};

/// SQLite metadata store manager.
///
/// Owns a connection pool and provides methods for all metadata operations.
/// For the trait-based interface, see the `MetadataStorage` impl below.
pub struct MetadataStore {
    pub(crate) pool: SqlitePool,
}

impl MetadataStore {
    /// Create a new metadata store.
    ///
    /// Initializes the SQLite database at `workspace_dir/metadata.db`,
    /// creates tables if they don't exist, and runs schema migrations.
    pub async fn new(workspace_dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(workspace_dir)
            .await
            .map_err(|e| {
                AppError::io_error(
                    format!("Failed to create workspace directory: {}", e),
                    Some(workspace_dir.to_path_buf()),
                )
            })?;

        let db_path = workspace_dir.join("metadata.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        info!(path = %db_path.display(), "Initializing metadata store");

        let pool: SqlitePool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(&db_url)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to connect to database: {}", e))
            })?;

        for pragma in &[
            "PRAGMA journal_mode = WAL",
            "PRAGMA synchronous = NORMAL",
            "PRAGMA cache_size = -8000",
            "PRAGMA busy_timeout = 5000",
            "PRAGMA mmap_size = 268435456",
            "PRAGMA temp_store = MEMORY",
        ] {
            sqlx::query(pragma).execute(&pool).await.map_err(|e| {
                AppError::database_error(format!("Failed to set PRAGMA '{}': {}", pragma, e))
            })?;
        }

        info!(path = %db_path.display(), "WAL mode enabled for better concurrency");

        schema::init_schema(&pool).await?;
        schema::migrate_schema_v2(&pool).await?;
        schema::migrate_schema_v3(&pool).await?;

        Ok(Self { pool })
    }

    /// Close the database and perform WAL checkpoint.
    pub async fn close(&self) {
        let _ = sqlx::query("PRAGMA wal_checkpoint(RESTART)")
            .execute(&self.pool)
            .await;
        self.pool.close().await;
    }

    // ── File operations (delegated to file_ops) ──

    pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
        file_ops::insert_file(&self.pool, metadata).await
    }

    pub async fn get_file_by_virtual_path(
        &self,
        virtual_path: &str,
    ) -> Result<Option<FileMetadata>> {
        file_ops::get_file_by_virtual_path(&self.pool, virtual_path).await
    }

    pub async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>> {
        file_ops::get_file_by_hash(&self.pool, hash).await
    }

    pub async fn batch_check_hashes(
        &self,
        hashes: &[String],
    ) -> Result<std::collections::HashSet<String>> {
        file_ops::batch_check_hashes(&self.pool, hashes).await
    }

    pub async fn get_archive_children(&self, archive_id: i64) -> Result<Vec<FileMetadata>> {
        file_ops::get_archive_children(&self.pool, archive_id).await
    }

    pub async fn get_all_files(&self) -> Result<Vec<FileMetadata>> {
        file_ops::get_all_files(&self.pool).await
    }

    pub async fn count_files(&self) -> Result<i64> {
        file_ops::count_files(&self.pool).await
    }

    pub async fn sum_file_sizes(&self) -> Result<i64> {
        file_ops::sum_file_sizes(&self.pool).await
    }

    pub async fn get_max_depth(&self) -> Result<i32> {
        file_ops::get_max_depth(&self.pool).await
    }

    pub async fn update_file_stats(
        &self,
        virtual_path: &str,
        min_timestamp: Option<i64>,
        max_timestamp: Option<i64>,
        level_mask: Option<u8>,
    ) -> Result<()> {
        file_ops::update_file_stats(&self.pool, virtual_path, min_timestamp, max_timestamp, level_mask).await
    }

    pub async fn update_file_ready(
        &self,
        virtual_path: &str,
        min_timestamp: Option<i64>,
        max_timestamp: Option<i64>,
        level_mask: Option<u8>,
    ) -> Result<()> {
        file_ops::update_file_ready(&self.pool, virtual_path, min_timestamp, max_timestamp, level_mask).await
    }

    pub async fn get_ready_files(&self) -> Result<Vec<FileMetadata>> {
        file_ops::get_ready_files(&self.pool).await
    }

    pub async fn get_files_with_pruning(
        &self,
        time_start: Option<i64>,
        time_end: Option<i64>,
        level_mask: Option<u8>,
        file_pattern: Option<&str>,
    ) -> Result<Vec<FileMetadata>> {
        file_ops::get_files_with_pruning(&self.pool, time_start, time_end, level_mask, file_pattern).await
    }

    pub async fn search_files(&self, query: &str) -> Result<Vec<FileMetadata>> {
        file_ops::search_files(&self.pool, query).await
    }

    pub async fn insert_files_batch(&self, files: Vec<FileMetadata>) -> Result<Vec<i64>> {
        file_ops::insert_files_batch(&self.pool, files).await
    }

    pub async fn supports_returning_clause(&self) -> bool {
        file_ops::supports_returning_clause(&self.pool).await
    }

    pub async fn insert_files_batch_optimized(&self, files: Vec<FileMetadata>) -> Result<Vec<i64>> {
        file_ops::insert_files_batch_optimized(&self.pool, files).await
    }

    pub async fn insert_files_batch_smart(&self, files: Vec<FileMetadata>) -> Result<Vec<i64>> {
        file_ops::insert_files_batch_smart(&self.pool, files).await
    }

    pub async fn clear_all(&self) -> Result<()> {
        file_ops::clear_all_files(&self.pool).await
    }

    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>> {
        file_ops::begin_transaction(&self.pool).await
    }

    // ── Archive operations (delegated to archive_ops) ──

    pub async fn insert_archive(&self, metadata: &ArchiveMetadata) -> Result<i64> {
        archive_ops::insert_archive(&self.pool, metadata).await
    }

    pub async fn count_archives(&self) -> Result<i64> {
        archive_ops::count_archives(&self.pool).await
    }

    pub async fn update_archive_status(&self, archive_id: i64, status: &str) -> Result<()> {
        archive_ops::update_archive_status(&self.pool, archive_id, status).await
    }

    pub async fn get_archive_by_id(&self, archive_id: i64) -> Result<Option<ArchiveMetadata>> {
        archive_ops::get_archive_by_id(&self.pool, archive_id).await
    }

    pub async fn get_all_archives(&self) -> Result<Vec<ArchiveMetadata>> {
        archive_ops::get_all_archives(&self.pool).await
    }

    // ── Index state operations (delegated to index_ops) ──

    pub async fn save_index_state(&self, state: &IndexState) -> Result<()> {
        index_ops::save_index_state(&self.pool, state).await
    }

    pub async fn load_index_state(&self, workspace_id: &str) -> Result<Option<IndexState>> {
        index_ops::load_index_state(&self.pool, workspace_id).await
    }

    pub async fn save_indexed_file(&self, file: &IndexedFile) -> Result<()> {
        index_ops::save_indexed_file(&self.pool, file).await
    }

    pub async fn load_indexed_files(&self, workspace_id: &str) -> Result<Vec<IndexedFile>> {
        index_ops::load_indexed_files(&self.pool, workspace_id).await
    }

    pub async fn load_indexed_file(&self, file_path: &str) -> Result<Option<IndexedFile>> {
        index_ops::load_indexed_file(&self.pool, file_path).await
    }

    pub async fn delete_indexed_file(&self, file_path: &str) -> Result<()> {
        index_ops::delete_indexed_file(&self.pool, file_path).await
    }

    pub async fn clear_indexed_files(&self, workspace_id: &str) -> Result<()> {
        index_ops::clear_indexed_files(&self.pool, workspace_id).await
    }
}

// ── Static transaction helpers ──

impl MetadataStore {
    /// Insert file within a transaction.
    pub async fn insert_file_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        metadata: &FileMetadata,
    ) -> Result<i64> {
        file_ops::insert_file_tx(tx, metadata).await
    }

    /// Insert archive within a transaction.
    pub async fn insert_archive_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        metadata: &ArchiveMetadata,
    ) -> Result<i64> {
        archive_ops::insert_archive_tx(tx, metadata).await
    }
}

// ── MetadataStorage trait impl ──

#[async_trait]
impl MetadataStorage for MetadataStore {
    async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
        self.insert_file(metadata).await
    }

    async fn get_all_files(&self) -> Result<Vec<FileMetadata>> {
        self.get_all_files().await
    }

    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>> {
        self.get_file_by_hash(hash).await
    }
}

#[cfg(test)]
#[path = "../metadata_store_tests.rs"]
mod tests;
