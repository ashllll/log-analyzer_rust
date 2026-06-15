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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: AnalysisStatus::Pending,
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

                // Simulate extraction: insert all files into metadata store
                let mut inserted_files = Vec::new();
                for file in files {
                    // Skip files with duplicate hashes (database constraint)
                    if inserted_files.iter().any(|f: &FileMetadata| f.sha256_hash == file.sha256_hash) {
                        continue;
                    }
                    // Skip files with duplicate virtual_path (database UNIQUE constraint)
                    if inserted_files.iter().any(|f: &FileMetadata| f.virtual_path == file.virtual_path) {
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
