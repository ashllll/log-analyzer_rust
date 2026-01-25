// Archive integration tests

use log_analyzer::archive::ArchiveManager;
use log_analyzer::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

// Common helper functions - consolidated from all archive test files

/// Create a temporary workspace with CAS and metadata store
async fn create_test_workspace() -> (TempDir, PathBuf, ContentAddressableStorage, MetadataStore) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let cas = ContentAddressableStorage::new(workspace_path.clone());
    let metadata = MetadataStore::new(&workspace_path).await.unwrap();

    (temp_dir, workspace_path, cas, metadata)
}

/// Create a simple ZIP archive with specified files
fn create_zip_with_files(dir: &Path, name: &str, files: Vec<(&str, &[u8])>) -> PathBuf {
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (filename, content) in files {
        zip.start_file(filename, options).unwrap();
        zip.write_all(content).unwrap();
    }

    zip.finish().unwrap();
    zip_path
}

/// Create a nested ZIP archive (archive containing another archive)
fn create_nested_zip(dir: &Path, outer_name: &str, inner_name: &str) -> PathBuf {
    // Create inner archive
    let _inner_path = dir.join(inner_name);
    let inner_zip = create_zip_with_files(dir, inner_name, vec![("file.txt", b"nested content")]);

    // Create outer archive containing inner archive
    let outer_path = dir.join(outer_name);
    let file = fs::File::create(&outer_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Read inner archive content and add to outer
    let inner_content = fs::read(&inner_zip).unwrap();
    zip.start_file(inner_name, options).unwrap();
    zip.write_all(&inner_content).unwrap();

    // Add some regular files
    zip.start_file("outer_file.log", options).unwrap();
    zip.write_all(b"outer level content").unwrap();

    zip.finish().unwrap();
    outer_path
}

/// Helper to extract and index files from archive
async fn extract_and_index_archive(
    archive_path: &Path,
    extract_dir: &Path,
    cas: &ContentAddressableStorage,
    metadata: &MetadataStore,
) -> usize {
    let manager = ArchiveManager::new();
    let _summary = manager
        .extract_archive(archive_path, extract_dir)
        .await
        .unwrap();

    let mut indexed_files = 0;
    for entry in WalkDir::new(extract_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();
            let content = fs::read(path).unwrap();
            let hash = cas.store_content(&content).await.unwrap();

            let metadata_entry = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: format!("archive/{}", path.file_name().unwrap().to_string_lossy()),
                original_name: path.file_name().unwrap().to_str().unwrap().to_string(),
                size: content.len() as i64,
                modified_time: 0,
                mime_type: Some("text/plain".to_string()),
                parent_archive_id: None,
                depth_level: 1,
            };

            metadata.insert_file(&metadata_entry).await.unwrap();
            indexed_files += 1;
        }
    }

    indexed_files
}

// Test cases migrated from individual files

#[tokio::test]
async fn test_archive_extraction_single_level() {
    let (temp_dir, _workspace_path, cas, metadata) = create_test_workspace().await;

    let test_files = vec![
        ("app.log", b"application log content" as &[u8]),
        ("error.log", b"error log content" as &[u8]),
        ("debug.log", b"debug log content" as &[u8]),
    ];

    let zip_path = create_zip_with_files(temp_dir.path(), "logs.zip", test_files);
    let extract_dir = temp_dir.path().join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Extract and verify
    let indexed = extract_and_index_archive(&zip_path, &extract_dir, &cas, &metadata).await;
    assert_eq!(indexed, 3, "Should index all extracted files");

    // Verify search works
    let results = metadata.search_files("error").await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].original_name, "error.log");
}

#[tokio::test]
async fn test_archive_extraction_with_nested_archive() {
    let (temp_dir, _workspace_path, _cas, _metadata) = create_test_workspace().await;

    let outer_zip = create_nested_zip(temp_dir.path(), "outer.zip", "inner.zip");

    let extract_dir = temp_dir.path().join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    let manager = ArchiveManager::new();
    let summary = manager
        .extract_archive(&outer_zip, &extract_dir)
        .await
        .unwrap();

    assert_eq!(summary.files_extracted, 2, "Should extract outer archive");
    assert!(summary
        .extracted_files
        .iter()
        .any(|p| p.ends_with("inner.zip")));
}

#[tokio::test]
async fn test_archive_extraction_with_cas_integration() {
    let (temp_dir, _workspace_path, cas, _metadata) = create_test_workspace().await;

    let zip_path = create_zip_with_files(
        temp_dir.path(),
        "test.zip",
        vec![
            ("file1.txt", b"content1" as &[u8]),
            ("file2.txt", b"content2" as &[u8]),
        ],
    );

    let extract_dir = temp_dir.path().join("extracted");
    let manager = ArchiveManager::new();

    let summary = manager
        .extract_archive(&zip_path, &extract_dir)
        .await
        .unwrap();

    assert_eq!(summary.files_extracted, 2);
    assert!(summary.total_size > 0);

    // Use WalkDir to verify files are actually extracted
    let mut found_files = 0;
    for entry in WalkDir::new(&extract_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            found_files += 1;
            let path = entry.path();
            let content = fs::read(path).unwrap();
            let hash = cas.store_content(&content).await.unwrap();

            // Verify round-trip
            let retrieved = cas.read_content(&hash).await.unwrap();
            assert_eq!(retrieved, content);
        }
    }
    assert_eq!(found_files, 2, "Should find exactly 2 files");
}

#[tokio::test]
async fn test_archive_extraction_with_deep_nesting() {
    let (temp_dir, _workspace_path, _cas, _metadata) = create_test_workspace().await;

    // Create 3-level nesting: outer → middle → inner
    let inner_zip = create_zip_with_files(
        temp_dir.path(),
        "inner.zip",
        vec![("deep_file.log", b"deep content")],
    );

    let middle_zip = create_zip_with_files(
        temp_dir.path(),
        "middle.zip",
        vec![("inner.zip", &fs::read(&inner_zip).unwrap())],
    );

    let outer_zip = create_zip_with_files(
        temp_dir.path(),
        "outer.zip",
        vec![("middle.zip", &fs::read(&middle_zip).unwrap())],
    );

    let extract_dir = temp_dir.path().join("extracted");
    let manager = ArchiveManager::new();

    let summary = manager
        .extract_archive(&outer_zip, &extract_dir)
        .await
        .unwrap();

    // Should extract outer and detect middle (but not extract inner automatically)
    assert!(
        summary.files_extracted >= 1,
        "Should extract at least outer archive"
    );
    assert!(summary
        .extracted_files
        .iter()
        .any(|p| p.ends_with("middle.zip")));
}

#[tokio::test]
async fn test_archive_extraction_cross_platform_paths() {
    let (temp_dir, _workspace_path, _cas, _metadata) = create_test_workspace().await;

    // Test long filename handling (Windows path limit)
    let long_name = "a".repeat(300) + ".txt";
    let zip_path = create_zip_with_files(
        temp_dir.path(),
        "long.zip",
        vec![(&long_name, b"long content")],
    );

    let extract_dir = temp_dir.path().join("extracted");
    let manager = ArchiveManager::new();

    let result = manager.extract_archive(&zip_path, &extract_dir).await;
    assert!(result.is_ok(), "Should handle long filenames");
}
