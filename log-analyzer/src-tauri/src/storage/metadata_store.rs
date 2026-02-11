//! SQLite Metadata Store
//!
//! Manages file and archive metadata using SQLite.
//! Provides fast lookups and full-text search capabilities.
//!
//! ## Database Schema
//!
//! - `files`: Individual file metadata with SHA-256 hashes
//! - `archives`: Archive metadata for nested tracking
//! - `files_fts`: Full-text search index (FTS5)
//!
//! ## Features
//!
//! - Async SQLite operations using sqlx
//! - Transaction support for atomic operations
//! - Full-text search with FTS5
//! - Hierarchical archive tracking

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info};

/// File metadata stored in SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub size: i64,
    pub modified_time: i64,
    pub mime_type: Option<String>,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
}

/// Archive metadata for nested tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub archive_type: String,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
    pub extraction_status: String,
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

/// SQLite metadata store manager
pub struct MetadataStore {
    pool: SqlitePool,
}

impl MetadataStore {
    /// Create a new metadata store
    ///
    /// Initializes the SQLite database and creates tables if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `workspace_dir` - Workspace directory (database will be at `workspace_dir/metadata.db`)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Failed to connect to database
    /// - Failed to create tables
    pub async fn new(workspace_dir: &Path) -> Result<Self> {
        // Create workspace directory if it doesn't exist
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

        // 老王备注：使用业内成熟的SQLite连接池配置
        // 连接池大小和超时配置基于SQLite最佳实践
        let pool = SqlitePoolOptions::new()
            .min_connections(1) // 最小连接数：桌面应用通常1个足够
            .max_connections(10) // 最大连接数：WAL模式支持更多并发
            .acquire_timeout(Duration::from_secs(30)) // 获取连接超时
            .idle_timeout(Duration::from_secs(600)) // 空闲连接超时：10分钟
            .max_lifetime(Duration::from_secs(1800)) // 连接最大生命周期：30分钟
            .connect(&db_url)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to connect to database: {}", e))
            })?;

        // Enable WAL mode for better concurrent read performance
        // WAL (Write-Ahead Logging) allows concurrent reads while writing
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to enable WAL mode: {}", e)))?;

        // Optimize for performance
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to set synchronous mode: {}", e))
            })?;

        // Increase cache size for better performance (default is -2000, we use -8000 for ~8MB)
        sqlx::query("PRAGMA cache_size = -8000")
            .execute(&pool)
            .await
            .ok(); // Ignore errors for cache size (may not be supported on all platforms)

        info!(path = %db_path.display(), "WAL mode enabled for better concurrency");

        // Initialize schema
        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    /// Initialize database schema
    async fn init_schema(pool: &SqlitePool) -> Result<()> {
        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sha256_hash TEXT NOT NULL UNIQUE,
                virtual_path TEXT NOT NULL,
                original_name TEXT NOT NULL,
                size INTEGER NOT NULL,
                modified_time INTEGER NOT NULL,
                mime_type TEXT,
                parent_archive_id INTEGER,
                depth_level INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create files table: {}", e)))?;

        // Create archives table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS archives (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sha256_hash TEXT NOT NULL UNIQUE,
                virtual_path TEXT NOT NULL,
                original_name TEXT NOT NULL,
                archive_type TEXT NOT NULL,
                parent_archive_id INTEGER,
                depth_level INTEGER NOT NULL DEFAULT 0,
                extraction_status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create archives table: {}", e)))?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_virtual_path ON files(virtual_path)")
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_files_parent_archive ON files(parent_archive_id)",
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_hash ON files(sha256_hash)")
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_archives_virtual_path ON archives(virtual_path)",
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_archives_parent ON archives(parent_archive_id)",
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_archives_hash ON archives(sha256_hash)")
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_depth ON files(depth_level)")
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_archives_depth ON archives(depth_level)")
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        // Create FTS5 virtual table for full-text search
        sqlx::query(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                virtual_path,
                original_name,
                content='files',
                content_rowid='id'
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create FTS table: {}", e)))?;

        // Create triggers to keep FTS index in sync
        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS files_fts_insert AFTER INSERT ON files BEGIN
                INSERT INTO files_fts(rowid, virtual_path, original_name)
                VALUES (new.id, new.virtual_path, new.original_name);
            END
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create FTS insert trigger: {}", e))
        })?;

        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS files_fts_delete AFTER DELETE ON files BEGIN
                DELETE FROM files_fts WHERE rowid = old.id;
            END
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create FTS delete trigger: {}", e))
        })?;

        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS files_fts_update AFTER UPDATE ON files BEGIN
                DELETE FROM files_fts WHERE rowid = old.id;
                INSERT INTO files_fts(rowid, virtual_path, original_name)
                VALUES (new.id, new.virtual_path, new.original_name);
            END
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create FTS update trigger: {}", e))
        })?;

        // Create index_state table for tracking indexing progress
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS index_state (
                workspace_id TEXT PRIMARY KEY,
                last_commit_time INTEGER NOT NULL,
                index_version INTEGER NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create index_state table: {}", e))
        })?;

        // Create indexed_files table for incremental indexing
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS indexed_files (
                file_path TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                last_offset INTEGER NOT NULL DEFAULT 0,
                file_size INTEGER NOT NULL,
                modified_time INTEGER NOT NULL,
                hash TEXT NOT NULL,
                FOREIGN KEY (workspace_id) REFERENCES index_state(workspace_id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create indexed_files table: {}", e))
        })?;

        // Create index on workspace_id for faster queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_indexed_files_workspace ON indexed_files(workspace_id)",
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create index: {}", e)))?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Insert file metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - File metadata to insert (id will be ignored and auto-generated)
    ///
    /// # Returns
    ///
    /// The auto-generated file ID
    pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
        // 老王备注：使用 INSERT OR IGNORE 处理 UNIQUE 约束冲突
        // 如果 sha256_hash 已存在，跳过插入（CAS 去重设计）
        // 然后查询已存在的记录 ID
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO files (
                sha256_hash, virtual_path, original_name, size,
                modified_time, mime_type, parent_archive_id, depth_level, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to insert file: {}", e)))?;

        // 老王备注：查询插入的记录或已存在的记录 ID
        let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ? LIMIT 1")
            .bind(&metadata.sha256_hash)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to fetch file ID: {}", e)))?
            .0;

        debug!(
            id = id,
            hash = %metadata.sha256_hash,
            path = %metadata.virtual_path,
            "Inserted or retrieved existing file metadata (CAS deduplication)"
        );

        Ok(id)
    }

    /// Insert archive metadata
    pub async fn insert_archive(&self, metadata: &ArchiveMetadata) -> Result<i64> {
        // 老王备注：使用 INSERT OR IGNORE 处理 UNIQUE 约束冲突（CAS 去重设计）
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
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to insert archive: {}", e)))?;

        // 老王备注：查询插入的记录或已存在的记录 ID
        let id =
            sqlx::query_as::<_, (i64,)>("SELECT id FROM archives WHERE sha256_hash = ? LIMIT 1")
                .bind(&metadata.sha256_hash)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database_error(format!("Failed to fetch archive ID: {}", e))
                })?
                .0;

        debug!(
            id = id,
            hash = %metadata.sha256_hash,
            path = %metadata.virtual_path,
            "Inserted or retrieved existing archive metadata (CAS deduplication)"
        );

        Ok(id)
    }

    /// Get file by virtual path
    pub async fn get_file_by_virtual_path(
        &self,
        virtual_path: &str,
    ) -> Result<Option<FileMetadata>> {
        let row = sqlx::query("SELECT * FROM files WHERE virtual_path = ?")
            .bind(virtual_path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to query file: {}", e)))?;

        Ok(row.map(|r| FileMetadata {
            id: r.get("id"),
            sha256_hash: r.get("sha256_hash"),
            virtual_path: r.get("virtual_path"),
            original_name: r.get("original_name"),
            size: r.get("size"),
            modified_time: r.get("modified_time"),
            mime_type: r.get("mime_type"),
            parent_archive_id: r.get("parent_archive_id"),
            depth_level: r.get("depth_level"),
        }))
    }

    /// Get file by SHA-256 hash
    pub async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>> {
        let row = sqlx::query("SELECT * FROM files WHERE sha256_hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to query file: {}", e)))?;

        Ok(row.map(|r| FileMetadata {
            id: r.get("id"),
            sha256_hash: r.get("sha256_hash"),
            virtual_path: r.get("virtual_path"),
            original_name: r.get("original_name"),
            size: r.get("size"),
            modified_time: r.get("modified_time"),
            mime_type: r.get("mime_type"),
            parent_archive_id: r.get("parent_archive_id"),
            depth_level: r.get("depth_level"),
        }))
    }

    /// Get all files in an archive
    pub async fn get_archive_children(&self, archive_id: i64) -> Result<Vec<FileMetadata>> {
        let rows =
            sqlx::query("SELECT * FROM files WHERE parent_archive_id = ? ORDER BY virtual_path")
                .bind(archive_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    AppError::database_error(format!("Failed to query archive children: {}", e))
                })?;

        Ok(rows
            .into_iter()
            .map(|r| FileMetadata {
                id: r.get("id"),
                sha256_hash: r.get("sha256_hash"),
                virtual_path: r.get("virtual_path"),
                original_name: r.get("original_name"),
                size: r.get("size"),
                modified_time: r.get("modified_time"),
                mime_type: r.get("mime_type"),
                parent_archive_id: r.get("parent_archive_id"),
                depth_level: r.get("depth_level"),
            })
            .collect())
    }

    /// Get all files (for validation)
    pub async fn get_all_files(&self) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query("SELECT * FROM files ORDER BY virtual_path")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to query all files: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| FileMetadata {
                id: r.get("id"),
                sha256_hash: r.get("sha256_hash"),
                virtual_path: r.get("virtual_path"),
                original_name: r.get("original_name"),
                size: r.get("size"),
                modified_time: r.get("modified_time"),
                mime_type: r.get("mime_type"),
                parent_archive_id: r.get("parent_archive_id"),
                depth_level: r.get("depth_level"),
            })
            .collect())
    }

    /// Count total files
    pub async fn count_files(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM files")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to count files: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Count total archives
    pub async fn count_archives(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM archives")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to count archives: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Sum of all file sizes
    pub async fn sum_file_sizes(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COALESCE(SUM(size), 0) as total FROM files")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to sum file sizes: {}", e)))?;

        Ok(row.get("total"))
    }

    /// Get maximum nesting depth
    pub async fn get_max_depth(&self) -> Result<i32> {
        let row = sqlx::query("SELECT COALESCE(MAX(depth_level), 0) as max_depth FROM files")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to get max depth: {}", e)))?;

        Ok(row.get("max_depth"))
    }

    /// Update archive extraction status
    pub async fn update_archive_status(&self, archive_id: i64, status: &str) -> Result<()> {
        sqlx::query("UPDATE archives SET extraction_status = ? WHERE id = ?")
            .bind(status)
            .bind(archive_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to update archive status: {}", e))
            })?;

        debug!(archive_id = archive_id, status = %status, "Updated archive status");
        Ok(())
    }

    /// Search files using FTS5
    pub async fn search_files(&self, query: &str) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query(
            r#"
            SELECT f.* FROM files f
            JOIN files_fts fts ON f.id = fts.rowid
            WHERE files_fts MATCH ?
            ORDER BY f.virtual_path
            "#,
        )
        .bind(query)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to search files: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| FileMetadata {
                id: r.get("id"),
                sha256_hash: r.get("sha256_hash"),
                virtual_path: r.get("virtual_path"),
                original_name: r.get("original_name"),
                size: r.get("size"),
                modified_time: r.get("modified_time"),
                mime_type: r.get("mime_type"),
                parent_archive_id: r.get("parent_archive_id"),
                depth_level: r.get("depth_level"),
            })
            .collect())
    }

    /// Insert multiple files in a single transaction
    ///
    /// This is more efficient than inserting files one by one
    /// and ensures atomicity - either all files are inserted or none.
    ///
    /// # Arguments
    ///
    /// * `files` - Vector of file metadata to insert
    ///
    /// # Returns
    ///
    /// Vector of auto-generated file IDs in the same order as input
    pub async fn insert_files_batch(&self, files: Vec<FileMetadata>) -> Result<Vec<i64>> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                AppError::database_error(format!("Failed to begin transaction: {}", e))
            })?;

        let mut ids = Vec::with_capacity(files.len());

        for metadata in files {
            // 老王备注：使用 INSERT OR IGNORE 处理 UNIQUE 约束冲突（批量插入版本）
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO files (
                    sha256_hash, virtual_path, original_name, size,
                    modified_time, mime_type, parent_archive_id, depth_level, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to insert file: {}", e)))?;

            // 老王备注：查询插入的记录或已存在的记录 ID
            let id =
                sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ? LIMIT 1")
                    .bind(&metadata.sha256_hash)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| {
                        AppError::database_error(format!("Failed to fetch file ID: {}", e))
                    })?
                    .0;

            ids.push(id);
        }

        tx.commit().await.map_err(|e| {
            AppError::database_error(format!("Failed to commit transaction: {}", e))
        })?;

        info!(
            count = ids.len(),
            "Inserted files in batch transaction (with CAS deduplication)"
        );
        Ok(ids)
    }

    /// Delete all files and archives (for workspace cleanup)
    pub async fn clear_all(&self) -> Result<()> {
        let mut tx =
            self.pool.begin().await.map_err(|e| {
                AppError::database_error(format!("Failed to begin transaction: {}", e))
            })?;

        sqlx::query("DELETE FROM files")
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to delete files: {}", e)))?;

        sqlx::query("DELETE FROM archives")
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to delete archives: {}", e)))?;

        tx.commit().await.map_err(|e| {
            AppError::database_error(format!("Failed to commit transaction: {}", e))
        })?;

        info!("Cleared all files and archives");
        Ok(())
    }

    /// Begin a transaction for atomic operations
    ///
    /// This allows multiple operations to be performed atomically.
    /// If any operation fails, the entire transaction can be rolled back.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut tx = metadata_store.begin_transaction().await?;
    /// // Perform operations...
    /// tx.commit().await?;
    /// ```
    ///
    /// # Requirements
    ///
    /// Validates: Requirements 8.4
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>> {
        self.pool
            .begin()
            .await
            .map_err(|e| AppError::database_error(format!("Failed to begin transaction: {}", e)))
    }

    /// Insert file metadata within a transaction
    ///
    /// This is similar to `insert_file` but operates within an existing transaction.
    /// Useful for atomic multi-file operations.
    ///
    /// # Arguments
    ///
    /// * `tx` - Active transaction
    /// * `metadata` - File metadata to insert
    ///
    /// # Returns
    ///
    /// The auto-generated file ID
    ///
    /// # Requirements
    ///
    /// Validates: Requirements 8.4
    pub async fn insert_file_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        metadata: &FileMetadata,
    ) -> Result<i64> {
        // 老王备注：使用 INSERT OR IGNORE 处理 UNIQUE 约束冲突（事务版本）
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO files (
                sha256_hash, virtual_path, original_name, size,
                modified_time, mime_type, parent_archive_id, depth_level, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to insert file in transaction: {}", e))
        })?;

        // 老王备注：查询插入的记录或已存在的记录 ID
        let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ? LIMIT 1")
            .bind(&metadata.sha256_hash)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to fetch file ID in transaction: {}", e))
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

    /// Insert archive metadata within a transaction
    ///
    /// # Arguments
    ///
    /// * `tx` - Active transaction
    /// * `metadata` - Archive metadata to insert
    ///
    /// # Returns
    ///
    /// The auto-generated archive ID
    ///
    /// # Requirements
    ///
    /// Validates: Requirements 8.4
    pub async fn insert_archive_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        metadata: &ArchiveMetadata,
    ) -> Result<i64> {
        // 老王备注：使用 INSERT OR IGNORE 处理 UNIQUE 约束冲突（事务版本）
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
            AppError::database_error(format!("Failed to insert archive in transaction: {}", e))
        })?;

        // 老王备注：查询插入的记录或已存在的记录 ID
        let id =
            sqlx::query_as::<_, (i64,)>("SELECT id FROM archives WHERE sha256_hash = ? LIMIT 1")
                .bind(&metadata.sha256_hash)
                .fetch_one(&mut **tx)
                .await
                .map_err(|e| {
                    AppError::database_error(format!(
                        "Failed to fetch archive ID in transaction: {}",
                        e
                    ))
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

    /// Get archive by ID
    pub async fn get_archive_by_id(&self, archive_id: i64) -> Result<Option<ArchiveMetadata>> {
        let row = sqlx::query("SELECT * FROM archives WHERE id = ?")
            .bind(archive_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to query archive: {}", e)))?;

        Ok(row.map(|r| ArchiveMetadata {
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

    /// Get all archives (for validation)
    pub async fn get_all_archives(&self) -> Result<Vec<ArchiveMetadata>> {
        let rows = sqlx::query("SELECT * FROM archives ORDER BY virtual_path")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to query all archives: {}", e))
            })?;

        Ok(rows
            .into_iter()
            .map(|r| ArchiveMetadata {
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

    // ========== Index State Management for Incremental Indexing ==========

    /// Save index state for a workspace
    ///
    /// # Arguments
    ///
    /// * `state` - Index state to save
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn save_index_state(&self, state: &IndexState) -> Result<()> {
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
        .execute(&self.pool)
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

    /// Load index state for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace ID to load state for
    ///
    /// # Returns
    ///
    /// - `Some(IndexState)` if state exists
    /// - `None` if no state exists for this workspace
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn load_index_state(&self, workspace_id: &str) -> Result<Option<IndexState>> {
        let row = sqlx::query("SELECT workspace_id, last_commit_time, index_version FROM index_state WHERE workspace_id = ?")
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to load index state: {}", e)))?;

        Ok(row.map(|r| IndexState {
            workspace_id: r.get("workspace_id"),
            last_commit_time: r.get("last_commit_time"),
            index_version: r.get("index_version"),
        }))
    }

    /// Save indexed file record (UPSERT)
    ///
    /// # Arguments
    ///
    /// * `file` - Indexed file record to save
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn save_indexed_file(&self, file: &IndexedFile) -> Result<()> {
        // Ensure workspace exists in index_state before inserting indexed file
        // This prevents FOREIGN KEY constraint failures
        sqlx::query(
            "INSERT OR IGNORE INTO index_state (workspace_id, last_commit_time, index_version) VALUES (?, 0, 1)"
        )
        .bind(&file.workspace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to ensure workspace exists: {}", e)))?;

        sqlx::query(
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
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to save indexed file: {}", e)))?;

        debug!(
            file_path = %file.file_path,
            workspace_id = %file.workspace_id,
            last_offset = file.last_offset,
            "Saved indexed file record"
        );

        Ok(())
    }

    /// Load all indexed files for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace ID to load files for
    ///
    /// # Returns
    ///
    /// Vector of indexed file records
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn load_indexed_files(&self, workspace_id: &str) -> Result<Vec<IndexedFile>> {
        let rows = sqlx::query(
            "SELECT file_path, workspace_id, last_offset, file_size, modified_time, hash FROM indexed_files WHERE workspace_id = ?"
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to load indexed files: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| IndexedFile {
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

    /// Load indexed file record by file path
    ///
    /// # Arguments
    ///
    /// * `file_path` - File path to load record for
    ///
    /// # Returns
    ///
    /// - `Some(IndexedFile)` if record exists
    /// - `None` if no record exists for this file
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn load_indexed_file(&self, file_path: &str) -> Result<Option<IndexedFile>> {
        let row = sqlx::query(
            "SELECT file_path, workspace_id, last_offset, file_size, modified_time, hash FROM indexed_files WHERE file_path = ?"
        )
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to load indexed file: {}", e)))?;

        Ok(row.map(|r| IndexedFile {
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

    /// Delete indexed file record
    ///
    /// # Arguments
    ///
    /// * `file_path` - File path to delete record for
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn delete_indexed_file(&self, file_path: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM indexed_files WHERE file_path = ?")
            .bind(file_path)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to delete indexed file: {}", e))
            })?;

        debug!(
            file_path = %file_path,
            rows_affected = result.rows_affected(),
            "Deleted indexed file record"
        );

        Ok(())
    }

    /// Clear all indexed files for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace ID to clear files for
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn clear_indexed_files(&self, workspace_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM indexed_files WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to clear indexed files: {}", e))
            })?;

        debug!(
            workspace_id = %workspace_id,
            rows_affected = result.rows_affected(),
            "Cleared indexed files for workspace"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_store() -> (MetadataStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = MetadataStore::new(temp_dir.path()).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_create_metadata_store() {
        let (store, _temp_dir) = create_test_store().await;

        let count = store.count_files().await.unwrap();
        assert_eq!(count, 0, "New store should have no files");
    }

    #[tokio::test]
    async fn test_insert_and_retrieve_file() {
        let (store, _temp_dir) = create_test_store().await;

        let metadata = FileMetadata {
            id: 0,
            sha256_hash: "abc123".to_string(),
            virtual_path: "test/file.log".to_string(),
            original_name: "file.log".to_string(),
            size: 1024,
            modified_time: 1234567890,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        let id = store.insert_file(&metadata).await.unwrap();
        assert!(id > 0, "Should return valid ID");

        let retrieved = store
            .get_file_by_virtual_path("test/file.log")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.sha256_hash, "abc123");
        assert_eq!(retrieved.size, 1024);
    }

    #[tokio::test]
    async fn test_count_operations() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert test file
        let metadata = FileMetadata {
            id: 0,
            sha256_hash: "hash1".to_string(),
            virtual_path: "file1.log".to_string(),
            original_name: "file1.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };
        store.insert_file(&metadata).await.unwrap();

        let count = store.count_files().await.unwrap();
        assert_eq!(count, 1);

        let total_size = store.sum_file_sizes().await.unwrap();
        assert_eq!(total_size, 100);
    }

    // ========== Additional Unit Tests for Task 2.2 ==========

    /// Test database initialization creates all required tables and indexes
    #[tokio::test]
    async fn test_database_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let store = MetadataStore::new(temp_dir.path()).await.unwrap();

        // Verify tables exist by querying them
        let files_count = store.count_files().await.unwrap();
        assert_eq!(files_count, 0, "Files table should exist and be empty");

        let archives_count = store.count_archives().await.unwrap();
        assert_eq!(
            archives_count, 0,
            "Archives table should exist and be empty"
        );

        // Verify we can insert data (tests that schema is correct)
        let file = FileMetadata {
            id: 0,
            sha256_hash: "test_hash".to_string(),
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };
        let id = store.insert_file(&file).await.unwrap();
        assert!(
            id > 0,
            "Should successfully insert into initialized database"
        );
    }

    /// Test file insertion with all fields
    #[tokio::test]
    async fn test_insert_file_with_all_fields() {
        let (store, _temp_dir) = create_test_store().await;

        // First create a parent archive
        let parent_archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "parent_archive_hash".to_string(),
            virtual_path: "archive.zip".to_string(),
            original_name: "archive.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        };
        let parent_id = store.insert_archive(&parent_archive).await.unwrap();

        let metadata = FileMetadata {
            id: 0,
            sha256_hash: "abc123def456".to_string(),
            virtual_path: "archive.zip/logs/app.log".to_string(),
            original_name: "app.log".to_string(),
            size: 2048,
            modified_time: 1234567890,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: Some(parent_id),
            depth_level: 2,
        };

        let id = store.insert_file(&metadata).await.unwrap();
        assert!(id > 0);

        // Retrieve and verify all fields
        let retrieved = store
            .get_file_by_virtual_path("archive.zip/logs/app.log")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.sha256_hash, "abc123def456");
        assert_eq!(retrieved.virtual_path, "archive.zip/logs/app.log");
        assert_eq!(retrieved.original_name, "app.log");
        assert_eq!(retrieved.size, 2048);
        assert_eq!(retrieved.modified_time, 1234567890);
        assert_eq!(retrieved.mime_type, Some("text/plain".to_string()));
        assert_eq!(retrieved.parent_archive_id, Some(parent_id));
        assert_eq!(retrieved.depth_level, 2);
    }

    /// Test file retrieval by hash
    #[tokio::test]
    async fn test_get_file_by_hash() {
        let (store, _temp_dir) = create_test_store().await;

        let metadata = FileMetadata {
            id: 0,
            sha256_hash: "unique_hash_123".to_string(),
            virtual_path: "test/file.log".to_string(),
            original_name: "file.log".to_string(),
            size: 512,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        store.insert_file(&metadata).await.unwrap();

        let retrieved = store
            .get_file_by_hash("unique_hash_123")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.virtual_path, "test/file.log");
        assert_eq!(retrieved.size, 512);
    }

    /// Test retrieving non-existent file returns None
    #[tokio::test]
    async fn test_get_nonexistent_file() {
        let (store, _temp_dir) = create_test_store().await;

        let result = store
            .get_file_by_virtual_path("nonexistent/file.log")
            .await
            .unwrap();

        assert!(result.is_none(), "Should return None for non-existent file");

        let result_by_hash = store.get_file_by_hash("nonexistent_hash").await.unwrap();
        assert!(
            result_by_hash.is_none(),
            "Should return None for non-existent hash"
        );
    }

    /// Test archive insertion and retrieval
    #[tokio::test]
    async fn test_insert_and_retrieve_archive() {
        let (store, _temp_dir) = create_test_store().await;

        let archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "archive_hash_123".to_string(),
            virtual_path: "logs.zip".to_string(),
            original_name: "logs.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        };

        let id = store.insert_archive(&archive).await.unwrap();
        assert!(id > 0);

        let retrieved = store.get_archive_by_id(id).await.unwrap().unwrap();
        assert_eq!(retrieved.sha256_hash, "archive_hash_123");
        assert_eq!(retrieved.virtual_path, "logs.zip");
        assert_eq!(retrieved.archive_type, "zip");
        assert_eq!(retrieved.extraction_status, "completed");
    }

    /// Test archive hierarchy queries - get children of an archive
    #[tokio::test]
    async fn test_archive_hierarchy_queries() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert parent archive
        let parent_archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "parent_archive".to_string(),
            virtual_path: "parent.zip".to_string(),
            original_name: "parent.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        };
        let parent_id = store.insert_archive(&parent_archive).await.unwrap();

        // Insert files belonging to this archive
        let file1 = FileMetadata {
            id: 0,
            sha256_hash: "file1_hash".to_string(),
            virtual_path: "parent.zip/file1.log".to_string(),
            original_name: "file1.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: Some(parent_id),
            depth_level: 1,
        };

        let file2 = FileMetadata {
            id: 0,
            sha256_hash: "file2_hash".to_string(),
            virtual_path: "parent.zip/file2.log".to_string(),
            original_name: "file2.log".to_string(),
            size: 200,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: Some(parent_id),
            depth_level: 1,
        };

        store.insert_file(&file1).await.unwrap();
        store.insert_file(&file2).await.unwrap();

        // Query children
        let children = store.get_archive_children(parent_id).await.unwrap();
        assert_eq!(children.len(), 2, "Should have 2 children");
        assert_eq!(children[0].original_name, "file1.log");
        assert_eq!(children[1].original_name, "file2.log");
    }

    /// Test nested archive hierarchy (multi-level)
    #[tokio::test]
    async fn test_nested_archive_hierarchy() {
        let (store, _temp_dir) = create_test_store().await;

        // Level 0: Root archive
        let root_archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "root_hash".to_string(),
            virtual_path: "root.zip".to_string(),
            original_name: "root.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        };
        let root_id = store.insert_archive(&root_archive).await.unwrap();

        // Level 1: Nested archive inside root
        let nested_archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "nested_hash".to_string(),
            virtual_path: "root.zip/nested.zip".to_string(),
            original_name: "nested.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: Some(root_id),
            depth_level: 1,
            extraction_status: "completed".to_string(),
        };
        let nested_id = store.insert_archive(&nested_archive).await.unwrap();

        // Level 2: File inside nested archive
        let deep_file = FileMetadata {
            id: 0,
            sha256_hash: "deep_file_hash".to_string(),
            virtual_path: "root.zip/nested.zip/deep.log".to_string(),
            original_name: "deep.log".to_string(),
            size: 300,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: Some(nested_id),
            depth_level: 2,
        };
        store.insert_file(&deep_file).await.unwrap();

        // Verify hierarchy
        let nested_children = store.get_archive_children(nested_id).await.unwrap();
        assert_eq!(nested_children.len(), 1);
        assert_eq!(nested_children[0].depth_level, 2);
        assert_eq!(nested_children[0].original_name, "deep.log");

        // Verify max depth
        let max_depth = store.get_max_depth().await.unwrap();
        assert_eq!(max_depth, 2);
    }

    /// Test virtual path lookups
    #[tokio::test]
    async fn test_virtual_path_lookups() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert files with different virtual paths
        let paths = [
            "logs/app.log",
            "logs/error.log",
            "archive.zip/logs/nested.log",
            "data/metrics.log",
        ];

        for (i, path) in paths.iter().enumerate() {
            let file = FileMetadata {
                id: 0,
                sha256_hash: format!("hash_{}", i),
                virtual_path: path.to_string(),
                original_name: path.split('/').next_back().unwrap().to_string(),
                size: 100 * (i as i64 + 1),
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            store.insert_file(&file).await.unwrap();
        }

        // Test exact path lookup
        let result = store
            .get_file_by_virtual_path("logs/app.log")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.original_name, "app.log");

        let result = store
            .get_file_by_virtual_path("archive.zip/logs/nested.log")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.original_name, "nested.log");

        // Test non-existent path
        let result = store
            .get_file_by_virtual_path("nonexistent/path.log")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    /// Test batch file insertion
    #[tokio::test]
    async fn test_batch_file_insertion() {
        let (store, _temp_dir) = create_test_store().await;

        let files = vec![
            FileMetadata {
                id: 0,
                sha256_hash: "batch_hash_1".to_string(),
                virtual_path: "batch/file1.log".to_string(),
                original_name: "file1.log".to_string(),
                size: 100,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            },
            FileMetadata {
                id: 0,
                sha256_hash: "batch_hash_2".to_string(),
                virtual_path: "batch/file2.log".to_string(),
                original_name: "file2.log".to_string(),
                size: 200,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            },
            FileMetadata {
                id: 0,
                sha256_hash: "batch_hash_3".to_string(),
                virtual_path: "batch/file3.log".to_string(),
                original_name: "file3.log".to_string(),
                size: 300,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            },
        ];

        let ids = store.insert_files_batch(files).await.unwrap();
        assert_eq!(ids.len(), 3, "Should return 3 IDs");

        // Verify all files were inserted
        let count = store.count_files().await.unwrap();
        assert_eq!(count, 3);

        let total_size = store.sum_file_sizes().await.unwrap();
        assert_eq!(total_size, 600);
    }

    /// Test get all files
    #[tokio::test]
    async fn test_get_all_files() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert multiple files
        for i in 0..5 {
            let file = FileMetadata {
                id: 0,
                sha256_hash: format!("hash_{}", i),
                virtual_path: format!("file_{}.log", i),
                original_name: format!("file_{}.log", i),
                size: 100,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            store.insert_file(&file).await.unwrap();
        }

        let all_files = store.get_all_files().await.unwrap();
        assert_eq!(all_files.len(), 5);

        // Verify they're sorted by virtual_path
        for i in 0..4 {
            assert!(all_files[i].virtual_path <= all_files[i + 1].virtual_path);
        }
    }

    /// Test get all archives
    #[tokio::test]
    async fn test_get_all_archives() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert multiple archives
        for i in 0..3 {
            let archive = ArchiveMetadata {
                id: 0,
                sha256_hash: format!("archive_hash_{}", i),
                virtual_path: format!("archive_{}.zip", i),
                original_name: format!("archive_{}.zip", i),
                archive_type: "zip".to_string(),
                parent_archive_id: None,
                depth_level: 0,
                extraction_status: "completed".to_string(),
            };
            store.insert_archive(&archive).await.unwrap();
        }

        let all_archives = store.get_all_archives().await.unwrap();
        assert_eq!(all_archives.len(), 3);
    }

    /// Test update archive status
    #[tokio::test]
    async fn test_update_archive_status() {
        let (store, _temp_dir) = create_test_store().await;

        let archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "status_test_hash".to_string(),
            virtual_path: "test.zip".to_string(),
            original_name: "test.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "pending".to_string(),
        };

        let id = store.insert_archive(&archive).await.unwrap();

        // Update status
        store.update_archive_status(id, "completed").await.unwrap();

        // Verify update
        let updated = store.get_archive_by_id(id).await.unwrap().unwrap();
        assert_eq!(updated.extraction_status, "completed");
    }

    /// Test clear all data
    #[tokio::test]
    async fn test_clear_all() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert some data
        let file = FileMetadata {
            id: 0,
            sha256_hash: "clear_test_hash".to_string(),
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };
        store.insert_file(&file).await.unwrap();

        let archive = ArchiveMetadata {
            id: 0,
            sha256_hash: "clear_archive_hash".to_string(),
            virtual_path: "test.zip".to_string(),
            original_name: "test.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        };
        store.insert_archive(&archive).await.unwrap();

        // Verify data exists
        assert_eq!(store.count_files().await.unwrap(), 1);
        assert_eq!(store.count_archives().await.unwrap(), 1);

        // Clear all
        store.clear_all().await.unwrap();

        // Verify everything is cleared
        assert_eq!(store.count_files().await.unwrap(), 0);
        assert_eq!(store.count_archives().await.unwrap(), 0);
    }

    /// Test FTS search functionality
    #[tokio::test]
    async fn test_fts_search() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert files with searchable content
        let files = vec![
            ("error.log", "error"),
            ("application.log", "application"),
            ("system_error.log", "system"),
            ("debug.log", "debug"),
        ];

        for (name, _keyword) in files {
            let file = FileMetadata {
                id: 0,
                sha256_hash: format!("hash_{}", name),
                virtual_path: format!("logs/{}", name),
                original_name: name.to_string(),
                size: 100,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            store.insert_file(&file).await.unwrap();
        }

        // Search for "error" - should match error.log and system_error.log
        let results = store.search_files("error").await.unwrap();
        assert_eq!(results.len(), 2, "Should find 2 files with 'error'");

        // Search for "application"
        let results = store.search_files("application").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].original_name, "application.log");

        // Search for non-existent term
        let results = store.search_files("nonexistent").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    /// Test metrics operations
    #[tokio::test]
    async fn test_metrics_operations() {
        let (store, _temp_dir) = create_test_store().await;

        // Insert files with varying sizes and depths
        let files = vec![(100, 0), (200, 1), (300, 2), (400, 1), (500, 3)];

        for (size, depth) in files {
            let file = FileMetadata {
                id: 0,
                sha256_hash: format!("hash_{}_{}", size, depth),
                virtual_path: format!("file_{}_{}.log", size, depth),
                original_name: "file.log".to_string(),
                size,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: depth,
            };
            store.insert_file(&file).await.unwrap();
        }

        // Test count
        let count = store.count_files().await.unwrap();
        assert_eq!(count, 5);

        // Test sum of sizes
        let total_size = store.sum_file_sizes().await.unwrap();
        assert_eq!(total_size, 1500); // 100+200+300+400+500

        // Test max depth
        let max_depth = store.get_max_depth().await.unwrap();
        assert_eq!(max_depth, 3);
    }

    /// Test unique constraint on hash
    #[tokio::test]
    async fn test_unique_hash_constraint() {
        let (store, _temp_dir) = create_test_store().await;

        let file1 = FileMetadata {
            id: 0,
            sha256_hash: "duplicate_hash".to_string(),
            virtual_path: "file1.log".to_string(),
            original_name: "file1.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        store.insert_file(&file1).await.unwrap();

        // Try to insert another file with same hash - CAS deduplication should return existing ID
        let file2 = FileMetadata {
            id: 0,
            sha256_hash: "duplicate_hash".to_string(), // 相同的哈希
            virtual_path: "file2.log".to_string(),
            original_name: "file2.log".to_string(),
            size: 200, // 大小不同，但哈希相同
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        // CAS 去重设计：相同哈希应该成功插入（返回已存在记录的 ID）
        // 但由于 UNIQUE 约束在 sha256_hash 上，第二个文件不会创建新的虚拟路径记录
        let result = store.insert_file(&file2).await;
        assert!(
            result.is_ok(),
            "Should successfully insert duplicate hash (returns existing ID due to UNIQUE constraint)"
        );

        // 验证第一个文件仍然存在
        let retrieved1 = store
            .get_file_by_virtual_path("file1.log")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved1.sha256_hash, "duplicate_hash");

        // 验证第二个虚拟路径不存在（被 INSERT OR IGNORE 忽略）
        let retrieved2 = store.get_file_by_virtual_path("file2.log").await.unwrap();
        assert!(
            retrieved2.is_none(),
            "Second virtual path should not exist (ignored by UNIQUE constraint)"
        );
    }

    // ========== Index State Management Tests ==========

    /// Test save and load index state
    #[tokio::test]
    async fn test_save_and_load_index_state() {
        let (store, _temp_dir) = create_test_store().await;

        let workspace_id = "test_workspace";
        let state = IndexState {
            workspace_id: workspace_id.to_string(),
            last_commit_time: 1234567890,
            index_version: 1,
        };

        // Save state
        store.save_index_state(&state).await.unwrap();

        // Load state
        let loaded = store.load_index_state(workspace_id).await.unwrap();
        assert!(loaded.is_some(), "Should load saved state");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.workspace_id, workspace_id);
        assert_eq!(loaded.last_commit_time, 1234567890);
        assert_eq!(loaded.index_version, 1);
    }

    /// Test load non-existent index state returns None
    #[tokio::test]
    async fn test_load_nonexistent_index_state() {
        let (store, _temp_dir) = create_test_store().await;

        let loaded = store
            .load_index_state("nonexistent_workspace")
            .await
            .unwrap();
        assert!(
            loaded.is_none(),
            "Should return None for non-existent workspace"
        );
    }

    /// Test update existing index state
    #[tokio::test]
    async fn test_update_index_state() {
        let (store, _temp_dir) = create_test_store().await;

        let workspace_id = "test_workspace";
        let state1 = IndexState {
            workspace_id: workspace_id.to_string(),
            last_commit_time: 1000,
            index_version: 1,
        };

        // Save initial state
        store.save_index_state(&state1).await.unwrap();

        // Update with new values
        let state2 = IndexState {
            workspace_id: workspace_id.to_string(),
            last_commit_time: 2000,
            index_version: 2,
        };
        store.save_index_state(&state2).await.unwrap();

        // Load and verify updated state
        let loaded = store.load_index_state(workspace_id).await.unwrap().unwrap();
        assert_eq!(loaded.last_commit_time, 2000);
        assert_eq!(loaded.index_version, 2);
    }

    /// Test save and load indexed file
    #[tokio::test]
    async fn test_save_and_load_indexed_file() {
        let (store, _temp_dir) = create_test_store().await;

        let workspace_id = "test_workspace";
        let file_path = "/path/to/file.log";

        let indexed_file = IndexedFile {
            file_path: file_path.to_string(),
            workspace_id: workspace_id.to_string(),
            last_offset: 1024,
            file_size: 2048,
            modified_time: 1234567890,
            hash: "abc123def456".to_string(),
        };

        // Save indexed file
        store.save_indexed_file(&indexed_file).await.unwrap();

        // Load indexed file
        let loaded = store.load_indexed_file(file_path).await.unwrap();
        assert!(loaded.is_some(), "Should load saved indexed file");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.file_path, file_path);
        assert_eq!(loaded.workspace_id, workspace_id);
        assert_eq!(loaded.last_offset, 1024);
        assert_eq!(loaded.file_size, 2048);
        assert_eq!(loaded.modified_time, 1234567890);
        assert_eq!(loaded.hash, "abc123def456");
    }

    /// Test upsert indexed file (update existing)
    #[tokio::test]
    async fn test_upsert_indexed_file() {
        let (store, _temp_dir) = create_test_store().await;

        let file_path = "/path/to/file.log";

        // Insert initial record
        let file1 = IndexedFile {
            file_path: file_path.to_string(),
            workspace_id: "workspace1".to_string(),
            last_offset: 100,
            file_size: 200,
            modified_time: 1000,
            hash: "hash1".to_string(),
        };
        store.save_indexed_file(&file1).await.unwrap();

        // Update with new values
        let file2 = IndexedFile {
            file_path: file_path.to_string(),
            workspace_id: "workspace1".to_string(),
            last_offset: 500,    // Updated offset
            file_size: 600,      // Updated size
            modified_time: 2000, // Updated time
            hash: "hash2".to_string(),
        };
        store.save_indexed_file(&file2).await.unwrap();

        // Load and verify updated values
        let loaded = store.load_indexed_file(file_path).await.unwrap().unwrap();
        assert_eq!(loaded.last_offset, 500);
        assert_eq!(loaded.file_size, 600);
        assert_eq!(loaded.modified_time, 2000);
        assert_eq!(loaded.hash, "hash2");
    }

    /// Test load indexed files for workspace
    #[tokio::test]
    async fn test_load_indexed_files_for_workspace() {
        let (store, _temp_dir) = create_test_store().await;

        let workspace_id = "test_workspace";

        // Insert multiple files for the same workspace
        let files = vec![
            IndexedFile {
                file_path: "/path/file1.log".to_string(),
                workspace_id: workspace_id.to_string(),
                last_offset: 100,
                file_size: 200,
                modified_time: 1000,
                hash: "hash1".to_string(),
            },
            IndexedFile {
                file_path: "/path/file2.log".to_string(),
                workspace_id: workspace_id.to_string(),
                last_offset: 300,
                file_size: 400,
                modified_time: 2000,
                hash: "hash2".to_string(),
            },
            IndexedFile {
                file_path: "/path/file3.log".to_string(),
                workspace_id: workspace_id.to_string(),
                last_offset: 500,
                file_size: 600,
                modified_time: 3000,
                hash: "hash3".to_string(),
            },
        ];

        for file in files {
            store.save_indexed_file(&file).await.unwrap();
        }

        // Load all files for workspace
        let loaded = store.load_indexed_files(workspace_id).await.unwrap();
        assert_eq!(loaded.len(), 3);

        // Verify files are loaded correctly
        let file_paths: Vec<_> = loaded.iter().map(|f| f.file_path.as_str()).collect();
        assert!(file_paths.contains(&"/path/file1.log"));
        assert!(file_paths.contains(&"/path/file2.log"));
        assert!(file_paths.contains(&"/path/file3.log"));
    }

    /// Test delete indexed file
    #[tokio::test]
    async fn test_delete_indexed_file() {
        let (store, _temp_dir) = create_test_store().await;

        let file_path = "/path/to/file.log";

        // Insert indexed file
        let indexed_file = IndexedFile {
            file_path: file_path.to_string(),
            workspace_id: "workspace1".to_string(),
            last_offset: 100,
            file_size: 200,
            modified_time: 1000,
            hash: "hash1".to_string(),
        };
        store.save_indexed_file(&indexed_file).await.unwrap();

        // Verify it exists
        let loaded = store.load_indexed_file(file_path).await.unwrap();
        assert!(loaded.is_some(), "File should exist before deletion");

        // Delete file
        store.delete_indexed_file(file_path).await.unwrap();

        // Verify it's deleted
        let loaded = store.load_indexed_file(file_path).await.unwrap();
        assert!(loaded.is_none(), "File should not exist after deletion");
    }

    /// Test clear indexed files for workspace
    #[tokio::test]
    async fn test_clear_indexed_files_for_workspace() {
        let (store, _temp_dir) = create_test_store().await;

        let workspace_id = "test_workspace";

        // Insert multiple files
        for i in 1..=3 {
            let file = IndexedFile {
                file_path: format!("/path/file{}.log", i),
                workspace_id: workspace_id.to_string(),
                last_offset: i * 100,
                file_size: (i * 200) as i64,
                modified_time: (i * 1000) as i64,
                hash: format!("hash{}", i),
            };
            store.save_indexed_file(&file).await.unwrap();
        }

        // Insert file for different workspace
        let other_file = IndexedFile {
            file_path: "/path/other.log".to_string(),
            workspace_id: "other_workspace".to_string(),
            last_offset: 999,
            file_size: 888i64,
            modified_time: 777i64,
            hash: "other_hash".to_string(),
        };
        store.save_indexed_file(&other_file).await.unwrap();

        // Clear files for workspace
        store.clear_indexed_files(workspace_id).await.unwrap();

        // Verify workspace files are cleared
        let loaded = store.load_indexed_files(workspace_id).await.unwrap();
        assert_eq!(loaded.len(), 0, "All files for workspace should be cleared");

        // Verify other workspace files are not affected
        let other_loaded = store.load_indexed_file("/path/other.log").await.unwrap();
        assert!(
            other_loaded.is_some(),
            "Other workspace files should not be affected"
        );
    }

    /// Test load non-existent indexed file returns None
    #[tokio::test]
    async fn test_load_nonexistent_indexed_file() {
        let (store, _temp_dir) = create_test_store().await;

        let loaded = store
            .load_indexed_file("/nonexistent/file.log")
            .await
            .unwrap();
        assert!(loaded.is_none(), "Should return None for non-existent file");
    }

    // ========== Property-Based Tests ==========

    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        /// Generate a valid file metadata for testing
        fn file_metadata_strategy() -> impl Strategy<Value = FileMetadata> {
            (
                // sha256_hash: 64 hex characters
                prop::collection::vec(0u8..=255, 32..=32).prop_map(|bytes| {
                    bytes
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                }),
                // virtual_path: reasonable path string
                prop::string::string_regex("[a-zA-Z0-9_/.-]{1,200}").unwrap(),
                // original_name: file name
                prop::string::string_regex("[a-zA-Z0-9_.-]{1,50}").unwrap(),
                // size: reasonable file size (0 to 100MB)
                0i64..=100_000_000i64,
                // modified_time: unix timestamp
                0i64..=2_000_000_000i64,
                // mime_type: optional
                prop::option::of(Just("text/plain".to_string())),
                // depth_level: 0 to 10
                0i32..=10i32,
            )
                .prop_map(
                    |(
                        hash,
                        virtual_path,
                        original_name,
                        size,
                        modified_time,
                        mime_type,
                        depth_level,
                    )| {
                        FileMetadata {
                            id: 0,
                            sha256_hash: hash,
                            virtual_path,
                            original_name,
                            size,
                            modified_time,
                            mime_type,
                            parent_archive_id: None,
                            depth_level,
                        }
                    },
                )
        }

        /// **Feature: archive-search-fix, Property 2: Path Map completeness**
        /// **Validates: Requirements 1.2, 1.3**
        ///
        /// For any extracted file, if extraction succeeds, then that file's path
        /// must exist in the metadata store.
        ///
        /// This property ensures that all successfully extracted files are properly
        /// indexed and can be found via the metadata store. This is critical for
        /// the search functionality to work correctly.
        #[test]
        fn prop_extracted_files_in_metadata_store() {
            // Use a smaller number of cases for async tests
            let config = ProptestConfig::with_cases(50);

            proptest!(config, |(files in prop::collection::vec(file_metadata_strategy(), 1..20))| {
                // Use tokio-test to run async code in property tests
                tokio_test::block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let store = MetadataStore::new(temp_dir.path()).await.unwrap();

                    // Simulate extraction: insert all files into metadata store
                    let mut inserted_files = Vec::new();
                    for file in files {
                        // Skip files with duplicate hashes (database constraint)
                        if inserted_files.iter().any(|f: &FileMetadata| f.sha256_hash == file.sha256_hash) {
                            continue;
                        }

                        match store.insert_file(&file).await {
                            Ok(_) => {
                                inserted_files.push(file.clone());
                            }
                            Err(_) => {
                                // Skip files that fail to insert (e.g., constraint violations)
                                continue;
                            }
                        }
                    }

                    // Property: For any extracted file, it must exist in metadata store
                    for file in &inserted_files {
                        // Verify file can be retrieved by virtual path
                        let retrieved = store
                            .get_file_by_virtual_path(&file.virtual_path)
                            .await
                            .unwrap();

                        prop_assert!(
                            retrieved.is_some(),
                            "Extracted file with virtual_path '{}' must exist in metadata store",
                            file.virtual_path
                        );

                        let retrieved_file = retrieved.unwrap();

                        // Verify the retrieved file matches what we inserted
                        prop_assert_eq!(
                            &retrieved_file.sha256_hash,
                            &file.sha256_hash,
                            "Retrieved file hash must match inserted file hash"
                        );

                        prop_assert_eq!(
                            &retrieved_file.virtual_path,
                            &file.virtual_path,
                            "Retrieved file virtual_path must match inserted file virtual_path"
                        );

                        // Also verify file can be retrieved by hash
                        let retrieved_by_hash = store
                            .get_file_by_hash(&file.sha256_hash)
                            .await
                            .unwrap();

                        prop_assert!(
                            retrieved_by_hash.is_some(),
                            "Extracted file with hash '{}' must be retrievable by hash",
                            file.sha256_hash
                        );
                    }

                    // Verify completeness: all inserted files should be in get_all_files()
                    let all_files = store.get_all_files().await.unwrap();
                    prop_assert_eq!(
                        all_files.len(),
                        inserted_files.len(),
                        "Metadata store should contain exactly the number of files we inserted"
                    );

                    // Verify each inserted file is in the complete list
                    for file in &inserted_files {
                        prop_assert!(
                            all_files.iter().any(|f| f.sha256_hash == file.sha256_hash),
                            "File with hash '{}' must be in the complete file list",
                            file.sha256_hash
                        );
                    }

                    Ok(())
                }).unwrap();
            });
        }
    }
}
