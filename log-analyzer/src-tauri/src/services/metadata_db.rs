use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{migrate::MigrateDatabase, Row};
use std::str::FromStr;
use tracing::{debug, info};

/// Metadata database for storing path mappings
/// Provides persistent storage for shortened path to original path mappings
pub struct MetadataDB {
    pool: SqlitePool,
}

impl MetadataDB {
    /// Create a new MetadataDB instance with the given database path
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(db_path).await? {
            info!("Creating new metadata database at {}", db_path);
            sqlx::Sqlite::create_database(db_path).await?;
        }

        // Configure connection options
        let options = SqliteConnectOptions::from_str(db_path)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await?;

        // Run migrations
        info!("Running database migrations");
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Store a path mapping
    /// If a mapping with the same workspace_id and short_path exists, it will be updated
    pub async fn store_mapping(
        &self,
        workspace_id: &str,
        short_path: &str,
        original_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let created_at = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO path_mappings (workspace_id, short_path, original_path, created_at, access_count)
            VALUES (?, ?, ?, ?, 0)
            ON CONFLICT(workspace_id, short_path) 
            DO UPDATE SET original_path = excluded.original_path
            "#
        )
        .bind(workspace_id)
        .bind(short_path)
        .bind(original_path)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        debug!(
            "Stored path mapping: workspace={}, short={}, original={}",
            workspace_id, short_path, original_path
        );

        Ok(())
    }

    /// Retrieve the original path for a given shortened path
    pub async fn get_original_path(
        &self,
        workspace_id: &str,
        short_path: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let result = sqlx::query(
            r#"
            SELECT original_path FROM path_mappings
            WHERE workspace_id = ? AND short_path = ?
            "#,
        )
        .bind(workspace_id)
        .bind(short_path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| row.get("original_path")))
    }

    /// Retrieve the shortened path for a given original path
    pub async fn get_short_path(
        &self,
        workspace_id: &str,
        original_path: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let result = sqlx::query(
            r#"
            SELECT short_path FROM path_mappings
            WHERE workspace_id = ? AND original_path = ?
            "#,
        )
        .bind(workspace_id)
        .bind(original_path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| row.get("short_path")))
    }

    /// Cleanup all mappings for a deleted workspace
    /// Returns the number of mappings deleted
    pub async fn cleanup_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let result = sqlx::query(
            r#"
            DELETE FROM path_mappings
            WHERE workspace_id = ?
            "#,
        )
        .bind(workspace_id)
        .execute(&self.pool)
        .await?;

        let rows_affected = result.rows_affected() as usize;
        info!(
            "Cleaned up {} path mappings for workspace {}",
            rows_affected, workspace_id
        );

        Ok(rows_affected)
    }

    /// Increment the access counter for a path mapping
    pub async fn increment_access_count(
        &self,
        workspace_id: &str,
        short_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            r#"
            UPDATE path_mappings
            SET access_count = access_count + 1
            WHERE workspace_id = ? AND short_path = ?
            "#,
        )
        .bind(workspace_id)
        .bind(short_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all mappings for a workspace (useful for debugging/admin)
    pub async fn get_workspace_mappings(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PathMapping>, Box<dyn std::error::Error>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, short_path, original_path, created_at, access_count
            FROM path_mappings
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        let mappings = rows
            .into_iter()
            .map(|row| PathMapping {
                id: row.get("id"),
                workspace_id: row.get("workspace_id"),
                short_path: row.get("short_path"),
                original_path: row.get("original_path"),
                created_at: row.get("created_at"),
                access_count: row.get("access_count"),
            })
            .collect();

        Ok(mappings)
    }

    /// Close the database connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Path mapping record
#[derive(Debug, Clone)]
pub struct PathMapping {
    pub id: i64,
    pub workspace_id: String,
    pub short_path: String,
    pub original_path: String,
    pub created_at: i64,
    pub access_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_store_and_retrieve_mapping() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        let workspace_id = "test_workspace";
        let short_path = "short/path.txt";
        let original_path = "very/long/original/path/that/exceeds/limits.txt";

        // Store mapping
        db.store_mapping(workspace_id, short_path, original_path)
            .await
            .unwrap();

        // Retrieve original path
        let retrieved = db
            .get_original_path(workspace_id, short_path)
            .await
            .unwrap();
        assert_eq!(retrieved, Some(original_path.to_string()));

        // Retrieve short path
        let retrieved_short = db
            .get_short_path(workspace_id, original_path)
            .await
            .unwrap();
        assert_eq!(retrieved_short, Some(short_path.to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_workspace() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        let workspace_id = "test_workspace";

        // Store multiple mappings
        db.store_mapping(workspace_id, "short1", "original1")
            .await
            .unwrap();
        db.store_mapping(workspace_id, "short2", "original2")
            .await
            .unwrap();
        db.store_mapping("other_workspace", "short3", "original3")
            .await
            .unwrap();

        // Cleanup workspace
        let deleted = db.cleanup_workspace(workspace_id).await.unwrap();
        assert_eq!(deleted, 2);

        // Verify mappings are deleted
        let result = db.get_original_path(workspace_id, "short1").await.unwrap();
        assert_eq!(result, None);

        // Verify other workspace is unaffected
        let result = db
            .get_original_path("other_workspace", "short3")
            .await
            .unwrap();
        assert_eq!(result, Some("original3".to_string()));
    }

    #[tokio::test]
    async fn test_increment_access_count() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        let workspace_id = "test_workspace";
        let short_path = "short/path.txt";

        // Store mapping
        db.store_mapping(workspace_id, short_path, "original")
            .await
            .unwrap();

        // Increment access count multiple times
        db.increment_access_count(workspace_id, short_path)
            .await
            .unwrap();
        db.increment_access_count(workspace_id, short_path)
            .await
            .unwrap();

        // Verify access count
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].access_count, 2);
    }

    #[tokio::test]
    async fn test_update_existing_mapping() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        let workspace_id = "test_workspace";
        let short_path = "short/path.txt";

        // Store initial mapping
        db.store_mapping(workspace_id, short_path, "original1")
            .await
            .unwrap();

        // Update with new original path
        db.store_mapping(workspace_id, short_path, "original2")
            .await
            .unwrap();

        // Verify updated mapping
        let retrieved = db
            .get_original_path(workspace_id, short_path)
            .await
            .unwrap();
        assert_eq!(retrieved, Some("original2".to_string()));

        // Verify only one mapping exists
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 1);
    }

    /// Test concurrent access to the database
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_concurrent_access() {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        let workspace_id = "concurrent_workspace";

        // Spawn multiple concurrent tasks that write to the database
        let mut tasks = JoinSet::new();

        for i in 0..20 {
            let db_clone = Arc::clone(&db);
            let ws_id = workspace_id.to_string();
            tasks.spawn(async move {
                let short_path = format!("short_{}", i);
                let original_path = format!("original_{}", i);
                db_clone
                    .store_mapping(&ws_id, &short_path, &original_path)
                    .await
                    .unwrap();
            });
        }

        // Wait for all tasks to complete
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }

        // Verify all mappings were stored correctly
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 20);

        // Verify each mapping is retrievable
        for i in 0..20 {
            let short_path = format!("short_{}", i);
            let original_path = format!("original_{}", i);
            let retrieved = db
                .get_original_path(workspace_id, &short_path)
                .await
                .unwrap();
            assert_eq!(retrieved, Some(original_path));
        }
    }

    /// Test concurrent reads and writes
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_concurrent_reads_and_writes() {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        let workspace_id = "rw_workspace";

        // Pre-populate some data
        for i in 0..10 {
            db.store_mapping(
                workspace_id,
                &format!("short_{}", i),
                &format!("original_{}", i),
            )
            .await
            .unwrap();
        }

        let mut tasks = JoinSet::new();

        // Spawn reader tasks
        for i in 0..10 {
            let db_clone = Arc::clone(&db);
            let ws_id = workspace_id.to_string();
            tasks.spawn(async move {
                let short_path = format!("short_{}", i);
                let result = db_clone
                    .get_original_path(&ws_id, &short_path)
                    .await
                    .unwrap();
                assert_eq!(result, Some(format!("original_{}", i)));
            });
        }

        // Spawn writer tasks
        for i in 10..20 {
            let db_clone = Arc::clone(&db);
            let ws_id = workspace_id.to_string();
            tasks.spawn(async move {
                let short_path = format!("short_{}", i);
                let original_path = format!("original_{}", i);
                db_clone
                    .store_mapping(&ws_id, &short_path, &original_path)
                    .await
                    .unwrap();
            });
        }

        // Wait for all tasks
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }

        // Verify final state
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 20);
    }

    /// Test UNIQUE constraint handling
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_unique_constraint_handling() {
        let db = MetadataDB::new(":memory:").await.unwrap();
        let workspace_id = "constraint_workspace";
        let short_path = "short/path.txt";

        // Store initial mapping
        db.store_mapping(workspace_id, short_path, "original1")
            .await
            .unwrap();

        // Store same short_path with different original_path (should update)
        db.store_mapping(workspace_id, short_path, "original2")
            .await
            .unwrap();

        // Verify the mapping was updated, not duplicated
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].original_path, "original2");
    }

    /// Test concurrent updates to the same mapping
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_concurrent_updates_same_mapping() {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        let workspace_id = "update_workspace";
        let short_path = "short/path.txt";

        // Store initial mapping
        db.store_mapping(workspace_id, short_path, "original_0")
            .await
            .unwrap();

        let mut tasks = JoinSet::new();

        // Spawn multiple tasks that update the same mapping
        for i in 1..=10 {
            let db_clone = Arc::clone(&db);
            let ws_id = workspace_id.to_string();
            let sp = short_path.to_string();
            tasks.spawn(async move {
                let original_path = format!("original_{}", i);
                db_clone
                    .store_mapping(&ws_id, &sp, &original_path)
                    .await
                    .unwrap();
            });
        }

        // Wait for all tasks
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }

        // Verify only one mapping exists (no duplicates)
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 1);

        // The final value should be one of the updates
        let retrieved = db
            .get_original_path(workspace_id, short_path)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().starts_with("original_"));
    }

    /// Test concurrent access count increments
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_concurrent_access_count_increments() {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        let workspace_id = "access_count_workspace";
        let short_path = "short/path.txt";

        // Store initial mapping
        db.store_mapping(workspace_id, short_path, "original")
            .await
            .unwrap();

        let mut tasks = JoinSet::new();
        let increment_count = 50;

        // Spawn multiple tasks that increment access count
        for _ in 0..increment_count {
            let db_clone = Arc::clone(&db);
            let ws_id = workspace_id.to_string();
            let sp = short_path.to_string();
            tasks.spawn(async move {
                db_clone.increment_access_count(&ws_id, &sp).await.unwrap();
            });
        }

        // Wait for all tasks
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }

        // Verify access count is correct
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].access_count, increment_count as i64);
    }

    /// Test transaction-like behavior with multiple operations
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_multiple_operations_consistency() {
        let db = MetadataDB::new(":memory:").await.unwrap();
        let workspace_id = "transaction_workspace";

        // Perform multiple operations
        db.store_mapping(workspace_id, "short1", "original1")
            .await
            .unwrap();
        db.store_mapping(workspace_id, "short2", "original2")
            .await
            .unwrap();
        db.increment_access_count(workspace_id, "short1")
            .await
            .unwrap();

        // Verify all operations succeeded
        let mappings = db.get_workspace_mappings(workspace_id).await.unwrap();
        assert_eq!(mappings.len(), 2);

        let mapping1 = mappings.iter().find(|m| m.short_path == "short1").unwrap();
        assert_eq!(mapping1.access_count, 1);

        let mapping2 = mappings.iter().find(|m| m.short_path == "short2").unwrap();
        assert_eq!(mapping2.access_count, 0);
    }

    /// Test workspace isolation
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_workspace_isolation() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        // Store mappings in different workspaces with same short_path
        db.store_mapping("workspace1", "short/path.txt", "original1")
            .await
            .unwrap();
        db.store_mapping("workspace2", "short/path.txt", "original2")
            .await
            .unwrap();

        // Verify each workspace has its own mapping
        let result1 = db
            .get_original_path("workspace1", "short/path.txt")
            .await
            .unwrap();
        assert_eq!(result1, Some("original1".to_string()));

        let result2 = db
            .get_original_path("workspace2", "short/path.txt")
            .await
            .unwrap();
        assert_eq!(result2, Some("original2".to_string()));

        // Cleanup one workspace shouldn't affect the other
        db.cleanup_workspace("workspace1").await.unwrap();

        let result1_after = db
            .get_original_path("workspace1", "short/path.txt")
            .await
            .unwrap();
        assert_eq!(result1_after, None);

        let result2_after = db
            .get_original_path("workspace2", "short/path.txt")
            .await
            .unwrap();
        assert_eq!(result2_after, Some("original2".to_string()));
    }

    /// Test empty workspace cleanup
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_empty_workspace_cleanup() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        // Cleanup non-existent workspace should return 0
        let deleted = db.cleanup_workspace("nonexistent").await.unwrap();
        assert_eq!(deleted, 0);
    }

    /// Test increment access count on non-existent mapping
    /// Validates: Requirements 1.4, 4.5
    #[tokio::test]
    async fn test_increment_nonexistent_mapping() {
        let db = MetadataDB::new(":memory:").await.unwrap();

        // Incrementing access count on non-existent mapping should not error
        // but also should not create a mapping
        let result = db.increment_access_count("workspace", "nonexistent").await;
        assert!(result.is_ok());

        // Verify no mapping was created
        let mappings = db.get_workspace_mappings("workspace").await.unwrap();
        assert_eq!(mappings.len(), 0);
    }
}
