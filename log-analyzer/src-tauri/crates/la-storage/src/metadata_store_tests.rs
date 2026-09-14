use super::*;
use tempfile::TempDir;

async fn create_test_store() -> (MetadataStore, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store = MetadataStore::new(temp_dir.path()).await.unwrap();
    (store, temp_dir)
}

#[tokio::test]
async fn replacing_content_invalidates_stats_in_each_upsert_path() {
    let (store, _temp) = create_test_store().await;
    for mode in 0..4 {
        let mut file = FileMetadata {
            id: 0,
            sha256_hash: "old".into(),
            virtual_path: format!("{mode}.log"),
            original_name: "file.log".into(),
            size: 10,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: Some(100),
            max_timestamp: Some(200),
            level_mask: Some(1),
            analysis_status: AnalysisStatus::Ready,
        };
        store.insert_file(&file).await.unwrap();
        file.sha256_hash = "new".into();
        file.min_timestamp = None;
        file.max_timestamp = None;
        file.level_mask = None;
        file.analysis_status = AnalysisStatus::Pending;
        match mode {
            0 => {
                store.insert_file(&file).await.unwrap();
            }
            1 => {
                store.insert_files_batch(vec![file.clone()]).await.unwrap();
            }
            2 => {
                store
                    .insert_files_batch_optimized(vec![file.clone()])
                    .await
                    .unwrap();
            }
            _ => {
                let mut tx = store.begin_transaction().await.unwrap();
                MetadataStore::insert_file_tx(&mut tx, &file).await.unwrap();
                tx.commit().await.unwrap();
            }
        }
        let actual = store
            .get_file_by_virtual_path(&file.virtual_path)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            actual.analysis_status,
            AnalysisStatus::Pending,
            "mode {mode}"
        );
        assert_eq!(
            (
                actual.min_timestamp,
                actual.max_timestamp,
                actual.level_mask
            ),
            (None, None, None)
        );
    }
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };
    store.insert_file(&metadata).await.unwrap();

    let count = store.count_files().await.unwrap();
    assert_eq!(count, 1);

    let total_size = store.sum_file_sizes().await.unwrap();
    assert_eq!(total_size, 100);
}

#[tokio::test]
async fn test_unfiltered_pruning_includes_pending_files() {
    let (store, _temp_dir) = create_test_store().await;

    let metadata = FileMetadata {
        id: 0,
        sha256_hash: "pending_hash".to_string(),
        virtual_path: "pending.log".to_string(),
        original_name: "pending.log".to_string(),
        size: 100,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };
    store.insert_file(&metadata).await.unwrap();

    let files = store
        .get_files_with_pruning(None, None, None, None)
        .await
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].virtual_path, "pending.log");
}

#[tokio::test]
async fn test_time_pruning_requires_ready_files() {
    let (store, _temp_dir) = create_test_store().await;

    let metadata = FileMetadata {
        id: 0,
        sha256_hash: "pending_hash_with_time_filter".to_string(),
        virtual_path: "pending-time.log".to_string(),
        original_name: "pending-time.log".to_string(),
        size: 100,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };
    store.insert_file(&metadata).await.unwrap();

    let files = store
        .get_files_with_pruning(Some(1), None, None, None)
        .await
        .unwrap();
    assert!(files.is_empty());
}

#[tokio::test]
async fn update_file_ready_rejects_an_unknown_virtual_path() {
    let (store, _temp_dir) = create_test_store().await;

    let result = store
        .update_file_ready("missing.log", Some(1), Some(2), Some(1))
        .await;

    assert!(
        result.is_err(),
        "marking a nonexistent file ready must not report success"
    );
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };
    let id = store.insert_file(&file).await.unwrap();
    assert!(
        id > 0,
        "Should successfully insert into initialized database"
    );
}

#[tokio::test]
async fn legacy_hash_identity_schema_migrates_to_path_identity() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("metadata.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let legacy_pool = sqlx::SqlitePool::connect(&db_url).await.unwrap();

    sqlx::query(
        r#"
        CREATE TABLE files (
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
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&legacy_pool)
    .await
    .unwrap();

    for (hash, path, size, created_at) in [
        ("old_hash", "mutable.log", 100_i64, 1_i64),
        ("new_hash", "mutable.log", 200_i64, 2_i64),
        ("stable_hash", "stable.log", 300_i64, 3_i64),
    ] {
        sqlx::query(
            r#"
            INSERT INTO files (
                sha256_hash, virtual_path, original_name, size, modified_time,
                depth_level, created_at
            ) VALUES (?, ?, ?, ?, ?, 0, ?)
            "#,
        )
        .bind(hash)
        .bind(path)
        .bind(path)
        .bind(size)
        .bind(created_at)
        .bind(created_at)
        .execute(&legacy_pool)
        .await
        .unwrap();
    }
    legacy_pool.close().await;

    let store = MetadataStore::new(temp_dir.path()).await.unwrap();
    assert_eq!(
        store.count_files().await.unwrap(),
        2,
        "migration should keep only the newest record for a repeated virtual path"
    );
    let migrated = store
        .get_file_by_virtual_path("mutable.log")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrated.sha256_hash, "new_hash");
    assert_eq!(migrated.size, 200);

    store
        .insert_file(&FileMetadata {
            id: 0,
            sha256_hash: "new_hash".to_string(),
            virtual_path: "copy.log".to_string(),
            original_name: "copy.log".to_string(),
            size: 200,
            modified_time: 2,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        })
        .await
        .unwrap();
    assert_eq!(
        store.count_files().await.unwrap(),
        3,
        "migrated schema must allow one CAS hash to back multiple virtual paths"
    );

    let fts_results = store.search_files("mutable").await.unwrap();
    assert_eq!(fts_results.len(), 1, "migration must rebuild the FTS index");
}

#[tokio::test]
async fn legacy_archive_identity_migration_remaps_hierarchy() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("metadata.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let legacy_pool = sqlx::SqlitePool::connect(&db_url).await.unwrap();

    sqlx::query(
        r#"
        CREATE TABLE archives (
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
    .execute(&legacy_pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        CREATE TABLE files (
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
        "#,
    )
    .execute(&legacy_pool)
    .await
    .unwrap();

    for (hash, path, parent_id, depth, created_at) in [
        ("old_parent_hash", "parent.zip", None, 0_i32, 1_i64),
        ("new_parent_hash", "parent.zip", None, 0_i32, 2_i64),
        (
            "child_archive_hash",
            "parent.zip/child.zip",
            Some(1_i64),
            1_i32,
            3_i64,
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO archives (
                sha256_hash, virtual_path, original_name, archive_type,
                parent_archive_id, depth_level, extraction_status, created_at
            ) VALUES (?, ?, ?, 'zip', ?, ?, 'completed', ?)
            "#,
        )
        .bind(hash)
        .bind(path)
        .bind(path)
        .bind(parent_id)
        .bind(depth)
        .bind(created_at)
        .execute(&legacy_pool)
        .await
        .unwrap();
    }

    sqlx::query(
        r#"
        INSERT INTO files (
            sha256_hash, virtual_path, original_name, size, modified_time,
            parent_archive_id, depth_level, analysis_status, created_at
        ) VALUES (
            'child_file_hash', 'parent.zip/child.log', 'child.log', 10, 0,
            1, 1, 'PENDING', 3
        )
        "#,
    )
    .execute(&legacy_pool)
    .await
    .unwrap();
    legacy_pool.close().await;

    let store = MetadataStore::new(temp_dir.path()).await.unwrap();
    let archives = store.get_all_archives().await.unwrap();
    assert_eq!(archives.len(), 2);
    let parent = archives
        .iter()
        .find(|archive| archive.virtual_path == "parent.zip")
        .unwrap();
    assert_eq!(parent.sha256_hash, "new_parent_hash");
    let child = archives
        .iter()
        .find(|archive| archive.virtual_path == "parent.zip/child.zip")
        .unwrap();
    assert_eq!(child.parent_archive_id, Some(parent.id));

    let child_file = store
        .get_file_by_virtual_path("parent.zip/child.log")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(child_file.parent_archive_id, Some(parent.id));

    store
        .insert_archive(&ArchiveMetadata {
            id: 0,
            sha256_hash: "new_parent_hash".to_string(),
            virtual_path: "copy.zip".to_string(),
            original_name: "copy.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(store.count_archives().await.unwrap(), 3);

    sqlx::query("DELETE FROM archives WHERE id = ?")
        .bind(parent.id)
        .execute(&store.pool)
        .await
        .unwrap();
    assert!(
        store
            .get_file_by_virtual_path("parent.zip/child.log")
            .await
            .unwrap()
            .is_none(),
        "migrated child file must still cascade with its retained parent"
    );
    assert!(
        store
            .get_all_archives()
            .await
            .unwrap()
            .iter()
            .all(|archive| archive.virtual_path != "parent.zip/child.zip"),
        "migrated nested archive must still cascade with its retained parent"
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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

#[tokio::test]
async fn identical_archive_content_retains_each_virtual_path() {
    let (store, _temp_dir) = create_test_store().await;
    let first = ArchiveMetadata {
        id: 0,
        sha256_hash: "shared_archive_hash".to_string(),
        virtual_path: "first.zip".to_string(),
        original_name: "first.zip".to_string(),
        archive_type: "zip".to_string(),
        parent_archive_id: None,
        depth_level: 0,
        extraction_status: "completed".to_string(),
    };
    let second = ArchiveMetadata {
        virtual_path: "second.zip".to_string(),
        original_name: "second.zip".to_string(),
        ..first.clone()
    };

    let first_id = store.insert_archive(&first).await.unwrap();
    let second_id = store.insert_archive(&second).await.unwrap();

    assert_ne!(
        first_id, second_id,
        "each archive path must have its own metadata identity"
    );
    let archives = store.get_all_archives().await.unwrap();
    assert_eq!(archives.len(), 2);
    assert!(archives
        .iter()
        .any(|archive| archive.virtual_path == "first.zip"));
    assert!(archives
        .iter()
        .any(|archive| archive.virtual_path == "second.zip"));
}

#[tokio::test]
async fn reinserting_archive_path_replaces_its_content_reference() {
    let (store, _temp_dir) = create_test_store().await;
    let original = ArchiveMetadata {
        id: 0,
        sha256_hash: "old_archive_hash".to_string(),
        virtual_path: "mutable.zip".to_string(),
        original_name: "mutable.zip".to_string(),
        archive_type: "zip".to_string(),
        parent_archive_id: None,
        depth_level: 0,
        extraction_status: "pending".to_string(),
    };
    let replacement = ArchiveMetadata {
        sha256_hash: "new_archive_hash".to_string(),
        extraction_status: "completed".to_string(),
        ..original.clone()
    };

    let original_id = store.insert_archive(&original).await.unwrap();
    let replacement_id = store.insert_archive(&replacement).await.unwrap();

    assert_eq!(
        original_id, replacement_id,
        "an archive path should keep one stable metadata identity"
    );
    let archives = store.get_all_archives().await.unwrap();
    assert_eq!(archives.len(), 1);
    assert_eq!(archives[0].sha256_hash, "new_archive_hash");
    assert_eq!(archives[0].extraction_status, "completed");
}

#[tokio::test]
async fn deleting_archive_cascades_to_child_file_metadata() {
    let (store, _temp_dir) = create_test_store().await;

    let archive_id = store
        .insert_archive(&ArchiveMetadata {
            id: 0,
            sha256_hash: "cascade_archive_hash".to_string(),
            virtual_path: "cascade.zip".to_string(),
            original_name: "cascade.zip".to_string(),
            archive_type: "zip".to_string(),
            parent_archive_id: None,
            depth_level: 0,
            extraction_status: "completed".to_string(),
        })
        .await
        .unwrap();

    store
        .insert_file(&FileMetadata {
            id: 0,
            sha256_hash: "cascade_child_hash".to_string(),
            virtual_path: "cascade.zip/child.log".to_string(),
            original_name: "child.log".to_string(),
            size: 12,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: Some(archive_id),
            depth_level: 1,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        })
        .await
        .unwrap();

    sqlx::query("DELETE FROM archives WHERE id = ?")
        .bind(archive_id)
        .execute(&store.pool)
        .await
        .unwrap();

    assert!(
        store
            .get_file_by_virtual_path("cascade.zip/child.log")
            .await
            .unwrap()
            .is_none(),
        "deleting an archive must not leave child file metadata behind"
    );
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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

#[tokio::test]
async fn invalid_analysis_status_is_reported_as_database_corruption() {
    let (store, _temp_dir) = create_test_store().await;
    store
        .insert_file(&FileMetadata {
            id: 0,
            sha256_hash: "invalid_status_hash".to_string(),
            virtual_path: "invalid-status.log".to_string(),
            original_name: "invalid-status.log".to_string(),
            size: 1,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        })
        .await
        .unwrap();

    sqlx::query("UPDATE files SET analysis_status = 'CORRUPT' WHERE virtual_path = ?")
        .bind("invalid-status.log")
        .execute(&store.pool)
        .await
        .unwrap();

    let error = store
        .get_file_by_virtual_path("invalid-status.log")
        .await
        .expect_err("an invalid persisted status must not be silently treated as pending");
    assert!(
        error.to_string().contains("Unknown analysis status"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn nullable_analysis_fields_round_trip_as_none() {
    let (store, _temp_dir) = create_test_store().await;
    store
        .insert_file(&FileMetadata {
            id: 0,
            sha256_hash: "nullable_stats_hash".to_string(),
            virtual_path: "nullable-stats.log".to_string(),
            original_name: "nullable-stats.log".to_string(),
            size: 1,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        })
        .await
        .unwrap();

    let stored = store
        .get_file_by_virtual_path("nullable-stats.log")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.min_timestamp, None);
    assert_eq!(stored.max_timestamp, None);
    assert_eq!(stored.level_mask, None);
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
            sha256_hash: format!("hash_{i}"),
            virtual_path: path.to_string(),
            original_name: path.split('/').next_back().unwrap().to_string(),
            size: 100 * (i as i64 + 1),
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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
            sha256_hash: format!("hash_{i}"),
            virtual_path: format!("file_{i}.log"),
            original_name: format!("file_{i}.log"),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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
            sha256_hash: format!("archive_hash_{i}"),
            virtual_path: format!("archive_{i}.zip"),
            original_name: format!("archive_{i}.zip"),
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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
            sha256_hash: format!("hash_{name}"),
            virtual_path: format!("logs/{name}"),
            original_name: name.to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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
            sha256_hash: format!("hash_{size}_{depth}"),
            virtual_path: format!("file_{size}_{depth}.log"),
            original_name: "file.log".to_string(),
            size,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: depth,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
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

/// CAS deduplicates content, while metadata retains every virtual path.
#[tokio::test]
async fn identical_content_retains_each_virtual_path() {
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };

    let file2 = FileMetadata {
        id: 0,
        sha256_hash: "duplicate_hash".to_string(),
        virtual_path: "file2.log".to_string(),
        original_name: "file2.log".to_string(),
        size: 100,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };

    let first_id = store.insert_file(&file1).await.unwrap();
    let second_id = store.insert_file(&file2).await.unwrap();
    assert_ne!(
        first_id, second_id,
        "each virtual path must have its own metadata identity"
    );

    let retrieved1 = store
        .get_file_by_virtual_path("file1.log")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved1.sha256_hash, "duplicate_hash");

    let retrieved2 = store
        .get_file_by_virtual_path("file2.log")
        .await
        .unwrap()
        .expect("the second virtual path must not be discarded");
    assert_eq!(retrieved2.sha256_hash, "duplicate_hash");
    assert_eq!(store.count_files().await.unwrap(), 2);
}

#[tokio::test]
async fn reinserting_virtual_path_replaces_its_content_reference() {
    let (store, _temp_dir) = create_test_store().await;

    let original = FileMetadata {
        id: 0,
        sha256_hash: "original_hash".to_string(),
        virtual_path: "mutable.log".to_string(),
        original_name: "mutable.log".to_string(),
        size: 100,
        modified_time: 10,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
    };
    let replacement = FileMetadata {
        sha256_hash: "replacement_hash".to_string(),
        size: 200,
        modified_time: 20,
        ..original.clone()
    };

    let original_id = store.insert_file(&original).await.unwrap();
    let replacement_id = store.insert_file(&replacement).await.unwrap();

    assert_eq!(
        original_id, replacement_id,
        "a virtual path should keep one stable metadata identity"
    );
    assert_eq!(store.count_files().await.unwrap(), 1);

    let stored = store
        .get_file_by_virtual_path("mutable.log")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.sha256_hash, "replacement_hash");
    assert_eq!(stored.size, 200);
    assert_eq!(stored.modified_time, 20);
}
// Reimporting identical content can reuse already-computed statistics.

#[tokio::test]
async fn upsert_with_null_stats_preserves_existing_analysis() {
    let (store, _temp_dir) = create_test_store().await;

    let analyzed = FileMetadata {
        id: 0,
        sha256_hash: "hash-1".to_string(),
        virtual_path: "watched.log".to_string(),
        original_name: "watched.log".to_string(),
        size: 100,
        modified_time: 1,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: Some(1_700_000_000),
        max_timestamp: Some(1_700_000_100),
        level_mask: Some(0b1010),
        analysis_status: AnalysisStatus::Ready,
    };
    store.insert_file(&analyzed).await.unwrap();

    let snapshot = FileMetadata {
        sha256_hash: "hash-1".to_string(),
        size: 100,
        modified_time: 2,
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
        ..analyzed.clone()
    };
    store.insert_file(&snapshot).await.unwrap();

    let stored = store
        .get_file_by_virtual_path("watched.log")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.min_timestamp,
        Some(1_700_000_000),
        "stats must survive NULL upserts"
    );
    assert_eq!(stored.max_timestamp, Some(1_700_000_100));
    assert_eq!(stored.level_mask, Some(0b1010));
    assert_eq!(
        stored.analysis_status,
        AnalysisStatus::Ready,
        "Ready must not regress to permanent Pending"
    );
    assert_eq!(
        stored.sha256_hash, "hash-1",
        "identical content keeps its statistics"
    );
    assert_eq!(stored.size, 100);
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
            file_path: format!("/path/file{i}.log"),
            workspace_id: workspace_id.to_string(),
            last_offset: i * 100,
            file_size: (i * 200) as i64,
            modified_time: (i * 1000) as i64,
            hash: format!("hash{i}"),
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
            prop::collection::vec(0u8..=255, 32..=32)
                .prop_map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()),
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
                        min_timestamp: None,
                        max_timestamp: None,
                        level_mask: None,
                        analysis_status: AnalysisStatus::Pending,
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

                // Simulate extraction: every path is retained, and reinserting the
                // same path replaces that path's content reference.
                let mut expected_by_path = std::collections::HashMap::new();
                for file in files {
                    store.insert_file(&file).await.unwrap();
                    expected_by_path.insert(file.virtual_path.clone(), file);
                }

                // Property: For any extracted file, it must exist in metadata store
                for file in expected_by_path.values() {
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
                    expected_by_path.len(),
                    "Metadata store should contain exactly one row per virtual path"
                );

                // Verify each inserted file is in the complete list
                for file in expected_by_path.values() {
                    prop_assert!(
                        all_files.iter().any(|stored| {
                            stored.virtual_path == file.virtual_path
                                && stored.sha256_hash == file.sha256_hash
                        }),
                        "File path '{}' with hash '{}' must be in the complete file list",
                        file.virtual_path,
                        file.sha256_hash,
                    );
                }

                Ok(())
            }).unwrap();
        });
    }
}

#[tokio::test]
async fn legacy_sampled_large_file_stats_never_exclude_search_candidates() {
    let (store, _temp) = create_test_store().await;
    let mut file = FileMetadata {
        id: 0,
        sha256_hash: "legacy".into(),
        virtual_path: "logs/large.log".into(),
        original_name: "large.log".into(),
        size: la_core::utils::file_stats::MAX_FULL_PARSE_BYTES as i64 + 1,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp: Some(100),
        max_timestamp: Some(200),
        level_mask: Some(4),
        analysis_status: AnalysisStatus::Ready,
    };
    store.insert_file(&file).await.unwrap();
    file.virtual_path = "logs/small.log".into();
    file.size = 100;
    store.insert_file(&file).await.unwrap();
    for (start, end, mask) in [
        (None, None, Some(1)),
        (Some(300), None, None),
        (None, Some(50), None),
        (Some(300), Some(400), Some(1)),
    ] {
        let files = store
            .get_files_with_pruning(start, end, mask, None)
            .await
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].virtual_path, "logs/large.log");
    }
    assert!(store
        .get_files_with_pruning(None, None, Some(1), Some("other/*"))
        .await
        .unwrap()
        .is_empty());
}
