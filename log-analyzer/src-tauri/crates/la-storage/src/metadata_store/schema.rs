//! Database schema initialization and migrations.
//!
//! These are associated functions on `MetadataStore` that set up
//! the SQLite tables, indexes, triggers, and perform schema migrations.

use la_core::error::{AppError, Result};
use sqlx::{Connection, Row, SqlitePool};
use tracing::info;

const CREATE_FILES_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS files (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sha256_hash TEXT NOT NULL,
        virtual_path TEXT NOT NULL UNIQUE,
        original_name TEXT NOT NULL,
        size INTEGER NOT NULL,
        modified_time INTEGER NOT NULL,
        mime_type TEXT,
        parent_archive_id INTEGER,
        depth_level INTEGER NOT NULL DEFAULT 0,
        min_timestamp INTEGER,
        max_timestamp INTEGER,
        level_mask INTEGER,
        analysis_status TEXT NOT NULL DEFAULT 'PENDING',
        created_at INTEGER NOT NULL,
        FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
    )
"#;

const CREATE_ARCHIVES_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS archives (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sha256_hash TEXT NOT NULL,
        virtual_path TEXT NOT NULL UNIQUE,
        original_name TEXT NOT NULL,
        archive_type TEXT NOT NULL,
        parent_archive_id INTEGER,
        depth_level INTEGER NOT NULL DEFAULT 0,
        extraction_status TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
    )
"#;

const INDEXES: [&str; 11] = [
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

const FILE_STATUS_INDEX_SQL: &str =
    "CREATE INDEX IF NOT EXISTS idx_files_status ON files(analysis_status)";

const FTS_TRIGGERS: [(&str, &str); 3] = [
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

pub(crate) async fn init_schema(pool: &SqlitePool) -> Result<()> {
    // Create files table
    sqlx::query(CREATE_FILES_TABLE_SQL)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create files table: {e}")))?;

    // Create archives table
    sqlx::query(CREATE_ARCHIVES_TABLE_SQL)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create archives table: {e}")))?;

    // Create indexes
    for idx_sql in &INDEXES {
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
    for (name, sql) in &FTS_TRIGGERS {
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
    sqlx::query(FILE_STATUS_INDEX_SQL)
        .execute(pool)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create status index: {e}")))?;

    Ok(())
}

async fn has_single_column_unique_index(
    pool: &SqlitePool,
    table: &str,
    column: &str,
) -> Result<bool> {
    let escaped_table = table.replace('\'', "''");
    let indexes = sqlx::query(&format!("PRAGMA index_list('{escaped_table}')"))
        .fetch_all(pool)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to inspect indexes for '{table}': {e}"))
        })?;

    for index in indexes {
        let is_unique: i64 = index
            .try_get("unique")
            .map_err(|e| AppError::database_error(format!("Invalid index metadata: {e}")))?;
        if is_unique == 0 {
            continue;
        }

        let index_name: String = index
            .try_get("name")
            .map_err(|e| AppError::database_error(format!("Invalid index metadata: {e}")))?;
        let escaped_name = index_name.replace('"', "\"\"");
        let columns = sqlx::query(&format!(r#"PRAGMA index_info("{escaped_name}")"#))
            .fetch_all(pool)
            .await
            .map_err(|e| {
                AppError::database_error(format!(
                    "Failed to inspect unique index '{index_name}': {e}"
                ))
            })?;

        if columns.len() == 1 {
            let indexed_column: String = columns[0]
                .try_get("name")
                .map_err(|e| AppError::database_error(format!("Invalid index metadata: {e}")))?;
            if indexed_column == column {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Move metadata identity from content hash to virtual path.
///
/// CAS objects remain hash-deduplicated. The metadata table must retain one row
/// per visible virtual path, so identical content can appear at multiple paths
/// while a re-import of the same path replaces its content reference.
pub(crate) async fn migrate_schema_v4(pool: &SqlitePool) -> Result<()> {
    let hash_is_unique = has_single_column_unique_index(pool, "files", "sha256_hash").await?;
    let virtual_path_is_unique =
        has_single_column_unique_index(pool, "files", "virtual_path").await?;
    if !hash_is_unique && virtual_path_is_unique {
        return Ok(());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to begin v4 migration: {e}")))?;

    for (name, _) in &FTS_TRIGGERS {
        sqlx::query(&format!("DROP TRIGGER IF EXISTS {name}"))
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::database_error(format!(
                    "Failed to drop FTS trigger '{name}' during v4 migration: {e}"
                ))
            })?;
    }

    sqlx::query("ALTER TABLE files RENAME TO files_before_path_identity")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!(
                "Failed to rename files table for v4 migration: {e}"
            ))
        })?;

    sqlx::query(CREATE_FILES_TABLE_SQL)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create v4 files table: {e}")))?;

    sqlx::query(
        r#"
        INSERT INTO files (
            id, sha256_hash, virtual_path, original_name, size, modified_time,
            mime_type, parent_archive_id, depth_level, min_timestamp,
            max_timestamp, level_mask, analysis_status, created_at
        )
        SELECT
            old.id, old.sha256_hash, old.virtual_path, old.original_name,
            old.size, old.modified_time, old.mime_type, old.parent_archive_id,
            old.depth_level, old.min_timestamp, old.max_timestamp,
            old.level_mask, old.analysis_status, old.created_at
        FROM files_before_path_identity AS old
        WHERE old.id = (
            SELECT newest.id
            FROM files_before_path_identity AS newest
            WHERE newest.virtual_path = old.virtual_path
            ORDER BY newest.id DESC
            LIMIT 1
        )
        ORDER BY old.id
        "#,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        AppError::database_error(format!("Failed to copy file metadata in v4 migration: {e}"))
    })?;

    sqlx::query("DROP TABLE files_before_path_identity")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to drop legacy files table: {e}")))?;

    for idx_sql in &INDEXES {
        sqlx::query(idx_sql).execute(&mut *tx).await.map_err(|e| {
            AppError::database_error(format!("Failed to create index during v4 migration: {e}"))
        })?;
    }

    sqlx::query(FILE_STATUS_INDEX_SQL)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!(
                "Failed to create status index during v4 migration: {e}"
            ))
        })?;

    for (name, sql) in &FTS_TRIGGERS {
        sqlx::query(sql).execute(&mut *tx).await.map_err(|e| {
            AppError::database_error(format!(
                "Failed to recreate FTS trigger '{name}' during v4 migration: {e}"
            ))
        })?;
    }

    sqlx::query("INSERT INTO files_fts(files_fts) VALUES('rebuild')")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to rebuild FTS index: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| AppError::database_error(format!("Failed to commit v4 migration: {e}")))?;

    info!("Migrated file metadata identity from hash to virtual path");
    Ok(())
}

/// Move archive metadata identity from content hash to virtual path while
/// preserving all parent-child references.
pub(crate) async fn migrate_schema_v5(pool: &SqlitePool) -> Result<()> {
    let hash_is_unique = has_single_column_unique_index(pool, "archives", "sha256_hash").await?;
    let virtual_path_is_unique =
        has_single_column_unique_index(pool, "archives", "virtual_path").await?;
    if !hash_is_unique && virtual_path_is_unique {
        return Ok(());
    }

    let mut connection = pool.acquire().await.map_err(|e| {
        AppError::database_error(format!(
            "Failed to acquire connection for v5 migration: {e}"
        ))
    })?;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *connection)
        .await
        .map_err(|e| {
            AppError::database_error(format!(
                "Failed to disable foreign keys for v5 migration: {e}"
            ))
        })?;

    let migration_result: Result<()> = async {
        let mut tx = connection
            .begin()
            .await
            .map_err(|e| AppError::database_error(format!("Failed to begin v5 migration: {e}")))?;

        for (name, _) in &FTS_TRIGGERS {
            sqlx::query(&format!("DROP TRIGGER IF EXISTS {name}"))
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    AppError::database_error(format!(
                        "Failed to drop FTS trigger '{name}' during v5 migration: {e}"
                    ))
                })?;
        }

        sqlx::query(
            r#"
            CREATE TEMP TABLE archive_path_identity_map (
                old_id INTEGER PRIMARY KEY,
                retained_id INTEGER NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to create archive ID map: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO archive_path_identity_map (old_id, retained_id)
            SELECT
                old.id,
                (
                    SELECT newest.id
                    FROM archives AS newest
                    WHERE newest.virtual_path = old.virtual_path
                    ORDER BY newest.id DESC
                    LIMIT 1
                )
            FROM archives AS old
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to populate archive ID map: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE archives_path_identity (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sha256_hash TEXT NOT NULL,
                virtual_path TEXT NOT NULL UNIQUE,
                original_name TEXT NOT NULL,
                archive_type TEXT NOT NULL,
                parent_archive_id INTEGER,
                depth_level INTEGER NOT NULL DEFAULT 0,
                extraction_status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (parent_archive_id)
                    REFERENCES archives_path_identity(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to create v5 archives table: {e}"))
        })?;

        sqlx::query(
            r#"
            INSERT INTO archives_path_identity (
                id, sha256_hash, virtual_path, original_name, archive_type,
                parent_archive_id, depth_level, extraction_status, created_at
            )
            SELECT
                old.id,
                old.sha256_hash,
                old.virtual_path,
                old.original_name,
                old.archive_type,
                parent_map.retained_id,
                old.depth_level,
                old.extraction_status,
                old.created_at
            FROM archives AS old
            JOIN archive_path_identity_map AS own_map
                ON own_map.old_id = old.id AND own_map.retained_id = old.id
            LEFT JOIN archive_path_identity_map AS parent_map
                ON parent_map.old_id = old.parent_archive_id
            ORDER BY old.id
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database_error(format!("Failed to copy archive metadata: {e}")))?;

        sqlx::query(
            r#"
            UPDATE files
            SET parent_archive_id = (
                SELECT retained_id
                FROM archive_path_identity_map
                WHERE old_id = files.parent_archive_id
            )
            WHERE parent_archive_id IS NOT NULL
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::database_error(format!("Failed to remap file archive parents: {e}"))
        })?;

        sqlx::query("DROP TABLE archives")
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to drop legacy archives table: {e}"))
            })?;
        sqlx::query("ALTER TABLE archives_path_identity RENAME TO archives")
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::database_error(format!("Failed to rename v5 archives table: {e}"))
            })?;

        for idx_sql in &INDEXES {
            sqlx::query(idx_sql).execute(&mut *tx).await.map_err(|e| {
                AppError::database_error(format!(
                    "Failed to recreate index during v5 migration: {e}"
                ))
            })?;
        }

        for (name, sql) in &FTS_TRIGGERS {
            sqlx::query(sql).execute(&mut *tx).await.map_err(|e| {
                AppError::database_error(format!(
                    "Failed to recreate FTS trigger '{name}' during v5 migration: {e}"
                ))
            })?;
        }
        sqlx::query("INSERT INTO files_fts(files_fts) VALUES('rebuild')")
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::database_error(format!(
                    "Failed to rebuild FTS index during v5 migration: {e}"
                ))
            })?;

        let violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| {
                AppError::database_error(format!(
                    "Failed to validate foreign keys after v5 migration: {e}"
                ))
            })?;
        if !violations.is_empty() {
            return Err(AppError::database_error(format!(
                "Archive path migration produced {} foreign key violation(s)",
                violations.len()
            )));
        }

        sqlx::query("DROP TABLE archive_path_identity_map")
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database_error(format!("Failed to drop archive ID map: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| AppError::database_error(format!("Failed to commit v5 migration: {e}")))?;
        Ok(())
    }
    .await;

    let restore_result = sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *connection)
        .await
        .map_err(|e| {
            AppError::database_error(format!(
                "Failed to restore foreign key enforcement after v5 migration: {e}"
            ))
        });

    if let Err(error) = restore_result {
        connection.close_on_drop();
        return Err(error);
    }
    let foreign_keys_enabled: i64 = match sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&mut *connection)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            connection.close_on_drop();
            return Err(AppError::database_error(format!(
                "Failed to verify foreign key enforcement after v5 migration: {error}"
            )));
        }
    };
    if foreign_keys_enabled != 1 {
        connection.close_on_drop();
        return Err(AppError::database_error(
            "Foreign key enforcement remained disabled after v5 migration",
        ));
    }

    migration_result?;
    info!("Migrated archive metadata identity from hash to virtual path");
    Ok(())
}
