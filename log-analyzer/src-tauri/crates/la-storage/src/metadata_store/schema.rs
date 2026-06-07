//! Database schema initialization and migrations.
//!
//! These are associated functions on `MetadataStore` that set up
//! the SQLite tables, indexes, triggers, and perform schema migrations.

use la_core::error::{AppError, Result};
use sqlx::SqlitePool;
use tracing::info;

pub(crate) async fn init_schema(pool: &SqlitePool) -> Result<()> {
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
            min_timestamp INTEGER,
            max_timestamp INTEGER,
            level_mask INTEGER,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to create files table: {e}")))?;

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
    .map_err(|e| AppError::database_error(format!("Failed to create archives table: {e}")))?;

    // Create indexes
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_files_virtual_path ON files(virtual_path)",
        "CREATE INDEX IF NOT EXISTS idx_files_parent_archive ON files(parent_archive_id)",
        "CREATE INDEX IF NOT EXISTS idx_files_hash ON files(sha256_hash)",
        "CREATE INDEX IF NOT EXISTS idx_archives_virtual_path ON archives(virtual_path)",
        "CREATE INDEX IF NOT EXISTS idx_archives_parent ON archives(parent_archive_id)",
        "CREATE INDEX IF NOT EXISTS idx_archives_hash ON archives(sha256_hash)",
        "CREATE INDEX IF NOT EXISTS idx_files_depth ON files(depth_level)",
        "CREATE INDEX IF NOT EXISTS idx_files_min_ts ON files(min_timestamp)",
        "CREATE INDEX IF NOT EXISTS idx_files_max_ts ON files(max_timestamp)",
        "CREATE INDEX IF NOT EXISTS idx_files_level_mask ON files(level_mask)",
        "CREATE INDEX IF NOT EXISTS idx_archives_depth ON archives(depth_level)",
    ];

    for idx_sql in &indexes {
        sqlx::query(idx_sql)
            .execute(pool)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to create index: {e}")))?;
    }

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
    .map_err(|e| AppError::database_error(format!("Failed to create FTS table: {e}")))?;

    // Create triggers to keep FTS index in sync
    let fts_triggers = [
        (
            "files_fts_insert",
            r#"
            CREATE TRIGGER IF NOT EXISTS files_fts_insert AFTER INSERT ON files BEGIN
                INSERT INTO files_fts(rowid, virtual_path, original_name)
                VALUES (new.id, new.virtual_path, new.original_name);
            END
            "#,
        ),
        (
            "files_fts_delete",
            r#"
            CREATE TRIGGER IF NOT EXISTS files_fts_delete AFTER DELETE ON files BEGIN
                DELETE FROM files_fts WHERE rowid = old.id;
            END
            "#,
        ),
        (
            "files_fts_update",
            r#"
            CREATE TRIGGER IF NOT EXISTS files_fts_update AFTER UPDATE ON files BEGIN
                DELETE FROM files_fts WHERE rowid = old.id;
                INSERT INTO files_fts(rowid, virtual_path, original_name)
                VALUES (new.id, new.virtual_path, new.original_name);
            END
            "#,
        ),
    ];

    for (name, sql) in &fts_triggers {
        sqlx::query(sql).execute(pool).await.map_err(|e| {
            AppError::database_error(format!("Failed to create FTS trigger '{name}': {e}"))
        })?;
    }

    // Create index_state table
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
    .map_err(|e| AppError::database_error(format!("Failed to create index_state table: {e}")))?;

    // Create indexed_files table
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
    .map_err(|e| AppError::database_error(format!("Failed to create indexed_files table: {e}")))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_indexed_files_workspace ON indexed_files(workspace_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to create index: {e}")))?;

    info!("Database schema initialized successfully");
    Ok(())
}

pub(crate) async fn migrate_schema_v2(pool: &SqlitePool) -> Result<()> {
    for (col, typ) in [
        ("min_timestamp", "INTEGER"),
        ("max_timestamp", "INTEGER"),
        ("level_mask", "INTEGER"),
    ] {
        let sql = format!("ALTER TABLE files ADD COLUMN {col} {typ}");
        if let Err(e) = sqlx::query(&sql).execute(pool).await {
            let msg = e.to_string().to_lowercase();
            if !msg.contains("duplicate column") {
                return Err(AppError::database_error(format!(
                    "Failed to add column {col}: {e}"
                )));
            }
        }
    }
    Ok(())
}

pub(crate) async fn migrate_schema_v3(pool: &SqlitePool) -> Result<()> {
    let sql = "ALTER TABLE files ADD COLUMN analysis_status TEXT NOT NULL DEFAULT 'PENDING'";
    if let Err(e) = sqlx::query(sql).execute(pool).await {
        let msg = e.to_string().to_lowercase();
        if !msg.contains("duplicate column") {
            return Err(AppError::database_error(format!(
                "Failed to add analysis_status column: {e}"
            )));
        }
    }

    // Mark existing files with stats as READY
    sqlx::query(
        "UPDATE files SET analysis_status = 'READY' WHERE min_timestamp IS NOT NULL OR max_timestamp IS NOT NULL OR level_mask IS NOT NULL",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to update analysis_status: {e}")))?;

    // Create index for status-filtered queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_status ON files(analysis_status)")
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create status index: {e}")))?;

    Ok(())
}
