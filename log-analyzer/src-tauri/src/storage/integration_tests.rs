//! Integration tests for CAS + MetadataStore
//!
//! These tests verify that the Content-Addressable Storage and
//! SQLite Metadata Store work together correctly.

#[cfg(test)]
mod tests {
    use crate::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
    use tempfile::TempDir;

    async fn create_test_workspace() -> (ContentAddressableStorage, MetadataStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let cas = ContentAddressableStorage::new(workspace_path.clone());
        let metadata = MetadataStore::new(&workspace_path).await.unwrap();

        (cas, metadata, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_index_file() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        // Store content in CAS
        let content = b"test log content";
        let hash = cas.store_content(content).await.unwrap();

        // Store metadata in database
        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash.clone(),
            virtual_path: "logs/app.log".to_string(),
            original_name: "app.log".to_string(),
            size: content.len() as i64,
            modified_time: 1234567890,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        let file_id = metadata.insert_file(&file_meta).await.unwrap();
        assert!(file_id > 0);

        // Retrieve metadata
        let retrieved_meta = metadata
            .get_file_by_virtual_path("logs/app.log")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved_meta.sha256_hash, hash);

        // Retrieve content using hash from metadata
        let retrieved_content = cas.read_content(&retrieved_meta.sha256_hash).await.unwrap();
        assert_eq!(retrieved_content, content);
    }

    #[tokio::test]
    async fn test_deduplication_with_metadata() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        let content = b"duplicate content";

        // Store same content twice in CAS
        let hash1 = cas.store_content(content).await.unwrap();
        let hash2 = cas.store_content(content).await.unwrap();

        assert_eq!(hash1, hash2, "Same content should produce same hash");

        // Create two different file entries with same content
        // Note: In a real scenario, these would be different files with identical content
        // The database has a UNIQUE constraint on sha256_hash, so we can only insert once
        let file1 = FileMetadata {
            id: 0,
            sha256_hash: hash1.clone(),
            virtual_path: "logs/file1.log".to_string(),
            original_name: "file1.log".to_string(),
            size: content.len() as i64,
            modified_time: 1234567890,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        // Insert first file
        metadata.insert_file(&file1).await.unwrap();

        // Try to insert second file with same hash - should fail due to UNIQUE constraint
        let file2 = FileMetadata {
            id: 0,
            sha256_hash: hash2.clone(),
            virtual_path: "logs/file2.log".to_string(),
            original_name: "file2.log".to_string(),
            size: content.len() as i64,
            modified_time: 1234567890,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        let result = metadata.insert_file(&file2).await;
        assert!(
            result.is_err(),
            "Should not be able to insert duplicate hash"
        );

        // Verify the first file is accessible
        let retrieved = metadata
            .get_file_by_virtual_path("logs/file1.log")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.sha256_hash, hash1);

        // Content should be stored only once in CAS
        let content_retrieved = cas.read_content(&retrieved.sha256_hash).await.unwrap();
        assert_eq!(content_retrieved, content);

        // Verify CAS deduplication worked
        assert!(cas.exists(&hash1));
        assert!(cas.exists(&hash2)); // Same hash, so should exist
    }

    #[tokio::test]
    async fn test_batch_insert_with_cas() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        let mut files = Vec::new();

        // Store multiple files
        for i in 0..10 {
            let content = format!("log content {}", i);
            let hash = cas.store_content(content.as_bytes()).await.unwrap();

            files.push(FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: format!("logs/file{}.log", i),
                original_name: format!("file{}.log", i),
                size: content.len() as i64,
                modified_time: 1234567890,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            });
        }

        // Batch insert
        let ids = metadata.insert_files_batch(files).await.unwrap();
        assert_eq!(ids.len(), 10);

        // Verify all files are indexed
        let count = metadata.count_files().await.unwrap();
        assert_eq!(count, 10);

        // Verify all files are accessible
        let all_files = metadata.get_all_files().await.unwrap();
        assert_eq!(all_files.len(), 10);

        for file in all_files {
            let content = cas.read_content(&file.sha256_hash).await.unwrap();
            assert!(!content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_search_and_retrieve() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        // Store files with searchable names
        let files = vec![
            ("error.log", b"error message" as &[u8]),
            ("debug.log", b"debug info"),
            ("access.log", b"access log"),
        ];

        for (name, content) in files {
            let hash = cas.store_content(content).await.unwrap();
            let file_meta = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: format!("logs/{}", name),
                original_name: name.to_string(),
                size: content.len() as i64,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            metadata.insert_file(&file_meta).await.unwrap();
        }

        // Search using FTS5
        let results = metadata.search_files("error").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].original_name, "error.log");

        // Retrieve content
        let content = cas.read_content(&results[0].sha256_hash).await.unwrap();
        assert_eq!(content, b"error message");
    }

    #[tokio::test]
    async fn test_integrity_verification() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        let content = b"integrity test content";
        let hash = cas.store_content(content).await.unwrap();

        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash.clone(),
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata.insert_file(&file_meta).await.unwrap();

        // Verify integrity
        let is_valid = cas.verify_integrity(&hash).await.unwrap();
        assert!(is_valid, "Content integrity should be valid");

        // Verify all indexed files have valid content
        let all_files = metadata.get_all_files().await.unwrap();
        for file in all_files {
            assert!(
                cas.exists(&file.sha256_hash),
                "File {} should exist in CAS",
                file.virtual_path
            );
        }
    }

    #[tokio::test]
    async fn test_workspace_cleanup() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        // Add some files
        let content = b"cleanup test";
        let hash = cas.store_content(content).await.unwrap();

        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata.insert_file(&file_meta).await.unwrap();

        let count_before = metadata.count_files().await.unwrap();
        assert_eq!(count_before, 1);

        // Clear all metadata
        metadata.clear_all().await.unwrap();

        let count_after = metadata.count_files().await.unwrap();
        assert_eq!(count_after, 0);

        // CAS content should still exist (orphaned)
        assert!(cas.exists(&file_meta.sha256_hash));
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let (cas, metadata, _temp_dir) = create_test_workspace().await;

        // Add multiple files
        for i in 0..5 {
            let content = format!("content {}", i);
            let hash = cas.store_content(content.as_bytes()).await.unwrap();

            let file_meta = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: format!("file{}.log", i),
                original_name: format!("file{}.log", i),
                size: content.len() as i64,
                modified_time: 0,
                mime_type: None,
                parent_archive_id: None,
                depth_level: i as i32,
            };

            metadata.insert_file(&file_meta).await.unwrap();
        }

        // Collect metrics
        let file_count = metadata.count_files().await.unwrap();
        let total_size = metadata.sum_file_sizes().await.unwrap();
        let max_depth = metadata.get_max_depth().await.unwrap();
        let storage_size = cas.get_storage_size().await.unwrap();

        assert_eq!(file_count, 5);
        assert!(total_size > 0);
        assert_eq!(max_depth, 4);
        assert!(storage_size > 0);
    }
}
