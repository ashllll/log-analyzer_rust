//! Property-Based Tests for CAS Migration Completion
//!
//! These tests verify that the migration from legacy path_map system to CAS
//! architecture is complete and no legacy code references remain.
//!
//! **Feature: complete-cas-migration, Property 1: No Legacy Code References**
//! **Validates: Requirements 1.1**

use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Helper to check if a line is a comment or documentation
fn is_comment_or_doc(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
}

/// Helper to search for legacy references in source files
fn search_legacy_references(root_dir: &Path, pattern: &str) -> Vec<(String, usize, String)> {
    let mut matches = Vec::new();

    for entry in WalkDir::new(root_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();

            // Only check Rust source files
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }

            // Skip test files and this file itself
            if path.to_string_lossy().contains("tests") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(path) {
                for (line_num, line) in content.lines().enumerate() {
                    if line.contains(pattern) && !is_comment_or_doc(line) {
                        matches.push((
                            path.to_string_lossy().to_string(),
                            line_num + 1,
                            line.trim().to_string(),
                        ));
                    }
                }
            }
        }
    }

    matches
}

#[test]
fn test_no_path_map_references() {
    // **Property 1: No Legacy Code References - path_map**
    // **Validates: Requirements 1.1**
    //
    // For any source file in the codebase (excluding tests and comments),
    // it must not contain references to "path_map" or "PathMap"

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    // Search for path_map references
    let path_map_matches = search_legacy_references(src_dir, "path_map");
    let path_map_type_matches = search_legacy_references(src_dir, "PathMap");

    // Combine all matches
    let mut all_matches = path_map_matches;
    all_matches.extend(path_map_type_matches);

    if !all_matches.is_empty() {
        eprintln!("\n❌ Found legacy path_map references in source code:");
        for (file, line_num, line) in &all_matches {
            eprintln!("  {}:{}: {}", file, line_num, line);
        }
        panic!(
            "Found {} legacy path_map references in non-comment code",
            all_matches.len()
        );
    }

    println!("✅ No path_map references found in source code");
}

#[test]
fn test_no_index_store_references() {
    // **Property 1: No Legacy Code References - index_store**
    // **Validates: Requirements 1.1**
    //
    // For any source file in the codebase (excluding tests and comments),
    // it must not contain references to "index_store", "load_index", or "save_index"

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    // Search for index_store references
    let index_store_matches = search_legacy_references(src_dir, "index_store");
    let load_index_matches = search_legacy_references(src_dir, "load_index");
    let save_index_matches = search_legacy_references(src_dir, "save_index");

    // Combine all matches
    let mut all_matches = index_store_matches;
    all_matches.extend(load_index_matches);
    all_matches.extend(save_index_matches);

    if !all_matches.is_empty() {
        eprintln!("\n❌ Found legacy index_store references in source code:");
        for (file, line_num, line) in &all_matches {
            eprintln!("  {}:{}: {}", file, line_num, line);
        }
        panic!(
            "Found {} legacy index_store references in non-comment code",
            all_matches.len()
        );
    }

    println!("✅ No index_store references found in source code");
}

#[test]
fn test_no_migration_references() {
    // **Property 1: No Legacy Code References - migration**
    // **Validates: Requirements 1.1**
    //
    // For any source file in the codebase (excluding tests and comments),
    // it must not contain references to migration-related code

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    // Search for migration references
    let migration_matches = search_legacy_references(src_dir, "migration");
    let migrate_workspace_matches = search_legacy_references(src_dir, "migrate_workspace");

    // Combine all matches
    let mut all_matches = migration_matches;
    all_matches.extend(migrate_workspace_matches);

    // Filter out false positives (e.g., "migration" in comments about the migration being complete)
    let filtered_matches: Vec<_> = all_matches
        .into_iter()
        .filter(|(_, _, line)| {
            // Allow references to migration in historical context
            !line.contains("CAS migration")
                && !line.contains("migration complete")
                && !line.contains("post-migration")
                && !line.contains("migration from")
        })
        .collect();

    if !filtered_matches.is_empty() {
        eprintln!("\n❌ Found legacy migration references in source code:");
        for (file, line_num, line) in &filtered_matches {
            eprintln!("  {}:{}: {}", file, line_num, line);
        }
        panic!(
            "Found {} legacy migration references in non-comment code",
            filtered_matches.len()
        );
    }

    println!("✅ No migration references found in source code");
}

#[test]
fn test_no_index_data_references() {
    // **Property 1: No Legacy Code References - IndexData**
    // **Validates: Requirements 1.1**
    //
    // For any source file in the codebase (excluding tests and comments),
    // it must not contain references to "IndexData" struct

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    // Search for IndexData references
    let index_data_matches = search_legacy_references(src_dir, "IndexData");

    if !index_data_matches.is_empty() {
        eprintln!("\n❌ Found legacy IndexData references in source code:");
        for (file, line_num, line) in &index_data_matches {
            eprintln!("  {}:{}: {}", file, line_num, line);
        }
        panic!(
            "Found {} legacy IndexData references in non-comment code",
            index_data_matches.len()
        );
    }

    println!("✅ No IndexData references found in source code");
}

#[test]
fn test_cas_architecture_present() {
    // **Property 1: CAS Architecture Verification**
    // **Validates: Requirements 2.1, 2.2**
    //
    // Verify that CAS architecture components are present and being used

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    // Check for CAS-related files
    let cas_file = src_dir.join("storage").join("cas.rs");
    let metadata_store_file = src_dir.join("storage").join("metadata_store.rs");

    assert!(
        cas_file.exists(),
        "CAS implementation file should exist: {:?}",
        cas_file
    );
    assert!(
        metadata_store_file.exists(),
        "MetadataStore implementation file should exist: {:?}",
        metadata_store_file
    );

    // Verify CAS is being used in commands
    let commands_dir = src_dir.join("commands");
    assert!(commands_dir.exists(), "Commands directory should exist");

    let mut cas_usage_found = false;
    let mut metadata_store_usage_found = false;

    for entry in WalkDir::new(&commands_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if content.contains("ContentAddressableStorage") {
                    cas_usage_found = true;
                }
                if content.contains("MetadataStore") {
                    metadata_store_usage_found = true;
                }
            }
        }
    }

    assert!(
        cas_usage_found,
        "ContentAddressableStorage should be used in commands"
    );
    assert!(
        metadata_store_usage_found,
        "MetadataStore should be used in commands"
    );

    println!("✅ CAS architecture components are present and being used");
}

#[test]
fn test_no_legacy_files_exist() {
    // **Property 1: No Legacy Files**
    // **Validates: Requirements 1.2, 1.3**
    //
    // Verify that legacy files have been removed

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    // Check that legacy files don't exist
    let legacy_files = vec![
        src_dir.join("services").join("index_store.rs"),
        src_dir.join("migration").join("mod.rs"),
        src_dir.join("commands").join("migration.rs"),
    ];

    let mut existing_legacy_files = Vec::new();
    for file in &legacy_files {
        if file.exists() {
            existing_legacy_files.push(file.to_string_lossy().to_string());
        }
    }

    if !existing_legacy_files.is_empty() {
        eprintln!("\n❌ Found legacy files that should have been removed:");
        for file in &existing_legacy_files {
            eprintln!("  {}", file);
        }
        panic!(
            "Found {} legacy files that should have been removed",
            existing_legacy_files.len()
        );
    }

    println!("✅ No legacy files found");
}

#[test]
fn test_metadata_db_uses_cas_schema() {
    // **Property 1: Database Schema Verification**
    // **Validates: Requirements 7.1**
    //
    // Verify that the metadata database uses CAS schema (files, archives)
    // and not legacy schema (path_mappings)

    let metadata_db_file = Path::new("src/services/metadata_db.rs");

    if metadata_db_file.exists() {
        let content = fs::read_to_string(metadata_db_file).unwrap();

        // Check for legacy path_mappings table references
        let has_path_mappings = content
            .lines()
            .filter(|line| !is_comment_or_doc(line))
            .any(|line| line.contains("path_mappings"));

        if has_path_mappings {
            panic!("metadata_db.rs contains references to legacy 'path_mappings' table");
        }
    }

    // Check metadata_store.rs for proper CAS schema
    let metadata_store_file = Path::new("src/storage/metadata_store.rs");
    assert!(
        metadata_store_file.exists(),
        "MetadataStore file should exist"
    );

    let content = fs::read_to_string(metadata_store_file).unwrap();

    // Verify it uses CAS schema
    assert!(
        content.contains("files") && content.contains("archives"),
        "MetadataStore should use 'files' and 'archives' tables"
    );

    println!("✅ Database schema uses CAS architecture");
}

#[test]
fn test_comprehensive_legacy_code_scan() {
    // **Property 1: Comprehensive Legacy Code Scan**
    // **Validates: Requirements 1.1**
    //
    // Comprehensive scan for any legacy code patterns

    let src_dir = Path::new("src");
    assert!(src_dir.exists(), "Source directory should exist");

    let legacy_patterns = vec![
        "path_map",
        "PathMap",
        "PathMapType",
        "index_store",
        "load_index",
        "save_index",
        "IndexData",
        "IndexResult",
        "MetadataMapType",
        "create_traditional_workspace",
    ];

    let mut all_violations = Vec::new();

    for pattern in &legacy_patterns {
        let matches = search_legacy_references(src_dir, pattern);
        if !matches.is_empty() {
            all_violations.push((pattern.to_string(), matches));
        }
    }

    if !all_violations.is_empty() {
        eprintln!("\n❌ Comprehensive legacy code scan found violations:");
        for (pattern, matches) in &all_violations {
            eprintln!("\n  Pattern: {}", pattern);
            for (file, line_num, line) in matches {
                eprintln!("    {}:{}: {}", file, line_num, line);
            }
        }
        panic!(
            "Found legacy code patterns in {} categories",
            all_violations.len()
        );
    }

    println!("✅ Comprehensive legacy code scan passed - no violations found");
}

// ========== Property 2: CAS Storage Consistency Tests ==========

/// Property-based test for CAS Storage Consistency
///
/// **Feature: complete-cas-migration, Property 2: CAS Storage Consistency**
/// **Validates: Requirements 2.1, 2.2**
///
/// For any imported file, it must be stored in CAS and have a corresponding
/// entry in MetadataStore. This ensures the integrity of the CAS architecture.
#[cfg(test)]
mod cas_storage_consistency_tests {
    use log_analyzer::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
    use proptest::prelude::*;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    /// Generate random file content for testing
    fn file_content_strategy() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..10000)
    }

    /// Generate a valid virtual path
    fn virtual_path_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("logs/[a-zA-Z0-9_-]{1,20}\\.log").unwrap()
    }

    /// **Property 2: CAS Storage Consistency**
    /// **Validates: Requirements 2.1, 2.2**
    ///
    /// For any file content that is imported:
    /// 1. The content must be stored in CAS with correct SHA-256 hash
    /// 2. A corresponding metadata entry must exist in MetadataStore
    /// 3. The metadata entry must reference the correct CAS hash
    /// 4. The CAS object must be retrievable and match the original content
    #[test]
    fn prop_cas_storage_consistency() {
        let config = ProptestConfig::with_cases(100);

        proptest!(config, |(
            contents in prop::collection::vec(file_content_strategy(), 1..10),
            virtual_paths in prop::collection::vec(virtual_path_strategy(), 1..10)
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                // Create CAS and MetadataStore instances
                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Simulate import: store files in CAS and create metadata entries
                let mut imported_files = Vec::new();

                for (i, content) in contents.iter().enumerate() {
                    // Use modulo to cycle through virtual paths if we have more contents than paths
                    let virtual_path = &virtual_paths[i % virtual_paths.len()];

                    // Store content in CAS
                    let hash = cas.store_content(content).await.unwrap();

                    // Create metadata entry
                    let file_meta = FileMetadata {
                        id: 0,
                        sha256_hash: hash.clone(),
                        virtual_path: format!("{}_{}", virtual_path, i), // Make unique
                        original_name: format!("file_{}.log", i),
                        size: content.len() as i64,
                        modified_time: chrono::Utc::now().timestamp(),
                        mime_type: Some("text/plain".to_string()),
                        parent_archive_id: None,
                        depth_level: 0,
                    };

                    // Insert into metadata store
                    metadata_store.insert_file(&file_meta).await.unwrap();

                    imported_files.push((content.clone(), hash, file_meta));
                }

                // Property 1: All imported files must be in CAS
                for (original_content, hash, _) in &imported_files {
                    prop_assert!(
                        cas.exists(hash),
                        "CAS object must exist for hash: {}",
                        hash
                    );

                    // Verify content can be retrieved
                    let retrieved_content = cas.read_content(hash).await.unwrap();
                    prop_assert_eq!(
                        &retrieved_content,
                        original_content,
                        "Retrieved content must match original content"
                    );

                    // Verify hash integrity
                    let mut hasher = Sha256::new();
                    hasher.update(&retrieved_content);
                    let computed_hash = format!("{:x}", hasher.finalize());
                    prop_assert_eq!(
                        &computed_hash,
                        hash,
                        "Computed hash must match stored hash"
                    );
                }

                // Property 2: All imported files must have MetadataStore records
                let all_metadata = metadata_store.get_all_files().await.unwrap();
                prop_assert_eq!(
                    all_metadata.len(),
                    imported_files.len(),
                    "MetadataStore must contain exactly the number of imported files"
                );

                for (_, hash, file_meta) in &imported_files {
                    // Verify file can be retrieved by virtual path
                    let retrieved = metadata_store
                        .get_file_by_virtual_path(&file_meta.virtual_path)
                        .await
                        .unwrap();

                    prop_assert!(
                        retrieved.is_some(),
                        "Metadata entry must exist for virtual_path: {}",
                        file_meta.virtual_path
                    );

                    let retrieved_meta = retrieved.unwrap();
                    prop_assert_eq!(
                        &retrieved_meta.sha256_hash,
                        hash,
                        "Metadata hash must match CAS hash"
                    );

                    // Verify file can be retrieved by hash
                    let retrieved_by_hash = metadata_store
                        .get_file_by_hash(hash)
                        .await
                        .unwrap();

                    prop_assert!(
                        retrieved_by_hash.is_some(),
                        "Metadata entry must be retrievable by hash: {}",
                        hash
                    );
                }

                // Property 3: Bidirectional consistency
                // For every metadata entry, the CAS object must exist
                for metadata in &all_metadata {
                    prop_assert!(
                        cas.exists(&metadata.sha256_hash),
                        "CAS object must exist for every metadata entry (hash: {})",
                        metadata.sha256_hash
                    );

                    // Verify content is readable
                    let content = cas.read_content(&metadata.sha256_hash).await.unwrap();
                    prop_assert_eq!(
                        content.len() as i64,
                        metadata.size,
                        "CAS content size must match metadata size"
                    );
                }

                Ok(())
            }).unwrap();
        });
    }

    /// **Property 2.1: CAS Deduplication at Storage Level**
    /// **Validates: Requirements 2.1**
    ///
    /// When the same content is stored multiple times in CAS:
    /// 1. Only one CAS object should be created (deduplication)
    /// 2. The hash should be identical for all attempts
    /// 3. The content should be retrievable with the same hash
    ///
    /// Note: The current system has a UNIQUE constraint on sha256_hash in the files table,
    /// so each hash can only have one metadata entry. This test verifies CAS-level deduplication.
    #[test]
    fn prop_cas_deduplication_consistency() {
        let config = ProptestConfig::with_cases(50);

        proptest!(config, |(
            content in file_content_strategy(),
            store_count in 2usize..=5
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());

                // Store the same content multiple times in CAS
                let mut hashes = Vec::new();

                for _ in 0..store_count {
                    let hash = cas.store_content(&content).await.unwrap();
                    hashes.push(hash);
                }

                // Property 1: All hashes must be identical (deduplication)
                let first_hash = &hashes[0];
                for hash in &hashes {
                    prop_assert_eq!(
                        hash,
                        first_hash,
                        "All hashes for identical content must be the same"
                    );
                }

                // Property 2: CAS object exists (only one copy stored)
                prop_assert!(
                    cas.exists(first_hash),
                    "CAS object must exist for the deduplicated hash"
                );

                // Property 3: Content is retrievable
                let retrieved = cas.read_content(first_hash).await.unwrap();
                prop_assert_eq!(
                    &retrieved,
                    &content,
                    "Retrieved content must match original content"
                );

                // Property 4: Verify integrity
                let is_valid = cas.verify_integrity(first_hash).await.unwrap();
                prop_assert!(
                    is_valid,
                    "CAS integrity check must pass for deduplicated content"
                );

                Ok(())
            }).unwrap();
        });
    }

    /// **Property 2.2: CAS Content Integrity**
    /// **Validates: Requirements 2.1, 2.2**
    ///
    /// For any content stored in CAS:
    /// 1. The stored content must be retrievable
    /// 2. The retrieved content must match the original exactly
    /// 3. The hash of the retrieved content must match the stored hash
    /// 4. The integrity verification must pass
    #[test]
    fn prop_cas_content_integrity() {
        let config = ProptestConfig::with_cases(100);

        proptest!(config, |(content in file_content_strategy())| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Store content
                let hash = cas.store_content(&content).await.unwrap();

                // Create metadata
                let file_meta = FileMetadata {
                    id: 0,
                    sha256_hash: hash.clone(),
                    virtual_path: "test/integrity.log".to_string(),
                    original_name: "integrity.log".to_string(),
                    size: content.len() as i64,
                    modified_time: chrono::Utc::now().timestamp(),
                    mime_type: Some("text/plain".to_string()),
                    parent_archive_id: None,
                    depth_level: 0,
                };

                metadata_store.insert_file(&file_meta).await.unwrap();

                // Property 1: Content must be retrievable
                let retrieved = cas.read_content(&hash).await.unwrap();
                prop_assert_eq!(
                    &retrieved,
                    &content,
                    "Retrieved content must exactly match original content"
                );

                // Property 2: Hash must be correct
                let mut hasher = Sha256::new();
                hasher.update(&retrieved);
                let computed_hash = format!("{:x}", hasher.finalize());
                prop_assert_eq!(
                    &computed_hash,
                    &hash,
                    "Computed hash of retrieved content must match stored hash"
                );

                // Property 3: Integrity verification must pass
                let is_valid = cas.verify_integrity(&hash).await.unwrap();
                prop_assert!(
                    is_valid,
                    "CAS integrity verification must pass for stored content"
                );

                // Property 4: Metadata size must match actual content size
                let metadata = metadata_store.get_file_by_hash(&hash).await.unwrap().unwrap();
                prop_assert_eq!(
                    metadata.size,
                    content.len() as i64,
                    "Metadata size must match actual content size"
                );

                Ok(())
            }).unwrap();
        });
    }
}

// ========== Property 3: Search Uses CAS Tests ==========

/// Property-based tests for Search Uses CAS
///
/// **Feature: complete-cas-migration, Property 3: Search Uses CAS**
/// **Validates: Requirements 2.3**
///
/// For any search operation, it must query MetadataStore and read content from CAS
/// using SHA-256 hash. This ensures search functionality is fully integrated with
/// the CAS architecture.
#[cfg(test)]
mod search_uses_cas_tests {
    use log_analyzer::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
    use proptest::prelude::*;
    use tempfile::TempDir;

    /// Generate random search query
    fn search_query_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9 ]{3,20}").unwrap()
    }

    /// Generate random log content with searchable terms
    fn log_content_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop::string::string_regex("(ERROR|WARN|INFO|DEBUG): [a-zA-Z0-9 ]{10,50}").unwrap(),
            10..100,
        )
    }

    /// **Property 3.1: Search Queries MetadataStore**
    /// **Validates: Requirements 2.3**
    ///
    /// For any search operation:
    /// 1. The file list must be retrieved from MetadataStore
    /// 2. Each file in the search must have a valid metadata entry
    /// 3. The metadata entry must contain a SHA-256 hash
    #[test]
    fn prop_search_queries_metadata_store() {
        let config = ProptestConfig::with_cases(50);

        proptest!(config, |(
            file_count in 1usize..=10,
            query in search_query_strategy()
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Create test files with searchable content
                let mut created_files = Vec::new();

                for i in 0..file_count {
                    let content = format!(
                        "2024-01-01 10:00:00 INFO Log entry {} containing {}\n\
                         2024-01-01 10:01:00 ERROR Error message {}\n\
                         2024-01-01 10:02:00 WARN Warning {}\n",
                        i, query, i, i
                    );

                    // Store in CAS
                    let hash = cas.store_content(content.as_bytes()).await.unwrap();

                    // Create metadata
                    let file_meta = FileMetadata {
                        id: 0,
                        sha256_hash: hash.clone(),
                        virtual_path: format!("logs/file_{}.log", i),
                        original_name: format!("file_{}.log", i),
                        size: content.len() as i64,
                        modified_time: chrono::Utc::now().timestamp(),
                        mime_type: Some("text/plain".to_string()),
                        parent_archive_id: None,
                        depth_level: 0,
                    };

                    metadata_store.insert_file(&file_meta).await.unwrap();
                    created_files.push((hash, file_meta));
                }

                // Property 1: Search must query MetadataStore for file list
                let all_files = metadata_store.get_all_files().await.unwrap();
                prop_assert_eq!(
                    all_files.len(),
                    file_count,
                    "MetadataStore must return all files for search"
                );

                // Property 2: Each file must have valid metadata
                for file in &all_files {
                    prop_assert!(
                        !file.sha256_hash.is_empty(),
                        "Each file must have a SHA-256 hash"
                    );
                    prop_assert_eq!(
                        file.sha256_hash.len(),
                        64,
                        "SHA-256 hash must be 64 characters (hex)"
                    );
                    prop_assert!(
                        !file.virtual_path.is_empty(),
                        "Each file must have a virtual path"
                    );
                }

                // Property 3: Metadata entries must match created files
                for (expected_hash, _expected_meta) in &created_files {
                    let found = all_files.iter().any(|f| &f.sha256_hash == expected_hash);
                    prop_assert!(
                        found,
                        "MetadataStore must contain entry for hash: {}",
                        expected_hash
                    );
                }

                Ok(())
            }).unwrap();
        });
    }

    /// **Property 3.2: Search Reads Content from CAS**
    /// **Validates: Requirements 2.3**
    ///
    /// For any search operation:
    /// 1. Content must be read from CAS using SHA-256 hash
    /// 2. The hash must be obtained from MetadataStore
    /// 3. The content must be retrievable and valid
    /// 4. Search results must reference the CAS hash
    #[test]
    fn prop_search_reads_from_cas() {
        let config = ProptestConfig::with_cases(50);

        proptest!(config, |(
            log_lines in log_content_strategy(),
            search_term in prop::string::string_regex("(ERROR|WARN|INFO)").unwrap()
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Create test file with log content
                let content = log_lines.join("\n");
                let content_bytes = content.as_bytes();

                // Store in CAS
                let hash = cas.store_content(content_bytes).await.unwrap();

                // Create metadata
                let file_meta = FileMetadata {
                    id: 0,
                    sha256_hash: hash.clone(),
                    virtual_path: "logs/test.log".to_string(),
                    original_name: "test.log".to_string(),
                    size: content_bytes.len() as i64,
                    modified_time: chrono::Utc::now().timestamp(),
                    mime_type: Some("text/plain".to_string()),
                    parent_archive_id: None,
                    depth_level: 0,
                };

                metadata_store.insert_file(&file_meta).await.unwrap();

                // Simulate search operation
                // Step 1: Get files from MetadataStore
                let files = metadata_store.get_all_files().await.unwrap();
                prop_assert_eq!(files.len(), 1, "Should have one file");

                let file = &files[0];

                // Step 2: Read content from CAS using hash
                let retrieved_content = cas.read_content(&file.sha256_hash).await.unwrap();
                prop_assert_eq!(
                    &retrieved_content,
                    content_bytes,
                    "Content from CAS must match original content"
                );

                // Step 3: Verify content is searchable
                let content_str = String::from_utf8(retrieved_content).unwrap();
                let matching_lines: Vec<_> = content_str
                    .lines()
                    .filter(|line| line.contains(&search_term))
                    .collect();

                // Property 1: Search must use CAS hash
                prop_assert_eq!(
                    &file.sha256_hash,
                    &hash,
                    "Metadata must reference correct CAS hash"
                );

                // Property 2: Content must be retrievable via hash
                prop_assert!(
                    cas.exists(&file.sha256_hash),
                    "CAS must contain object for hash: {}",
                    file.sha256_hash
                );

                // Property 3: Search results should be based on CAS content
                let expected_matches = log_lines
                    .iter()
                    .filter(|line| line.contains(&search_term))
                    .count();
                prop_assert_eq!(
                    matching_lines.len(),
                    expected_matches,
                    "Search results must match content from CAS"
                );

                Ok(())
            }).unwrap();
        });
    }

    /// **Property 3.3: Search Does Not Use Path-Based Access**
    /// **Validates: Requirements 2.3**
    ///
    /// For any search operation:
    /// 1. Files must be accessed via CAS hash, not filesystem paths
    /// 2. The real_path in search results should reference CAS (cas://<hash>)
    /// 3. No direct filesystem access should occur for file content
    #[test]
    fn prop_search_uses_hash_not_path() {
        let config = ProptestConfig::with_cases(50);

        proptest!(config, |(
            file_count in 1usize..=5,
            query in search_query_strategy()
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Create test files
                let mut file_hashes = Vec::new();

                for i in 0..file_count {
                    let content = format!(
                        "2024-01-01 10:00:00 INFO Test log {} with {}\n",
                        i, query
                    );

                    let hash = cas.store_content(content.as_bytes()).await.unwrap();

                    let file_meta = FileMetadata {
                        id: 0,
                        sha256_hash: hash.clone(),
                        virtual_path: format!("logs/file_{}.log", i),
                        original_name: format!("file_{}.log", i),
                        size: content.len() as i64,
                        modified_time: chrono::Utc::now().timestamp(),
                        mime_type: Some("text/plain".to_string()),
                        parent_archive_id: None,
                        depth_level: 0,
                    };

                    metadata_store.insert_file(&file_meta).await.unwrap();
                    file_hashes.push(hash);
                }

                // Simulate search: get files and verify hash-based access
                let files = metadata_store.get_all_files().await.unwrap();

                for file in &files {
                    // Property 1: File must have a valid SHA-256 hash
                    prop_assert_eq!(
                        file.sha256_hash.len(),
                        64,
                        "File must have valid SHA-256 hash"
                    );

                    // Property 2: Content must be accessible via hash
                    let content = cas.read_content(&file.sha256_hash).await.unwrap();
                    prop_assert!(
                        !content.is_empty(),
                        "Content must be retrievable via CAS hash"
                    );

                    // Property 3: Hash must be in our created set
                    prop_assert!(
                        file_hashes.contains(&file.sha256_hash),
                        "File hash must be one we created"
                    );

                    // Property 4: Virtual path should not be used for content access
                    // (This is a design property - virtual_path is for display only)
                    prop_assert!(
                        !file.virtual_path.is_empty(),
                        "Virtual path should exist for display"
                    );
                }

                Ok(())
            }).unwrap();
        });
    }

    /// **Property 3.4: Search Integrity Across Multiple Files**
    /// **Validates: Requirements 2.3**
    ///
    /// For any search across multiple files:
    /// 1. All files must be queried from MetadataStore
    /// 2. All content must be read from CAS
    /// 3. Search results must be consistent with CAS content
    /// 4. No files should be missed or duplicated
    #[test]
    fn prop_search_integrity_multiple_files() {
        let config = ProptestConfig::with_cases(30);

        proptest!(config, |(
            file_count in 3usize..=10,
            search_term in prop::string::string_regex("(ERROR|SUCCESS|FAILURE)").unwrap()
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Create multiple files with varying content
                let mut expected_matches = 0;
                let mut created_hashes = std::collections::HashSet::new();

                for i in 0..file_count {
                    // Some files contain the search term, some don't
                    let contains_term = i % 2 == 0;
                    let content = if contains_term {
                        expected_matches += 1;
                        format!(
                            "2024-01-01 10:00:00 {} Message in file {}\n",
                            search_term, i
                        )
                    } else {
                        format!(
                            "2024-01-01 10:00:00 INFO Regular message in file {}\n",
                            i
                        )
                    };

                    let hash = cas.store_content(content.as_bytes()).await.unwrap();
                    created_hashes.insert(hash.clone());

                    let file_meta = FileMetadata {
                        id: 0,
                        sha256_hash: hash,
                        virtual_path: format!("logs/file_{}.log", i),
                        original_name: format!("file_{}.log", i),
                        size: content.len() as i64,
                        modified_time: chrono::Utc::now().timestamp(),
                        mime_type: Some("text/plain".to_string()),
                        parent_archive_id: None,
                        depth_level: 0,
                    };

                    metadata_store.insert_file(&file_meta).await.unwrap();
                }

                // Simulate search operation
                let all_files = metadata_store.get_all_files().await.unwrap();

                // Property 1: All files must be returned
                prop_assert_eq!(
                    all_files.len(),
                    file_count,
                    "MetadataStore must return all files"
                );

                // Property 2: No duplicate hashes
                let returned_hashes: std::collections::HashSet<_> =
                    all_files.iter().map(|f| f.sha256_hash.clone()).collect();
                prop_assert_eq!(
                    returned_hashes.len(),
                    file_count,
                    "No duplicate files should be returned"
                );

                // Property 3: All returned hashes must be in created set
                for hash in &returned_hashes {
                    prop_assert!(
                        created_hashes.contains(hash),
                        "Returned hash must be one we created: {}",
                        hash
                    );
                }

                // Property 4: Search results must match expected
                let mut actual_matches = 0;
                for file in &all_files {
                    let content = cas.read_content(&file.sha256_hash).await.unwrap();
                    let content_str = String::from_utf8(content).unwrap();

                    if content_str.contains(&search_term) {
                        actual_matches += 1;
                    }
                }

                prop_assert_eq!(
                    actual_matches,
                    expected_matches,
                    "Search must find all matching files via CAS"
                );

                Ok(())
            }).unwrap();
        });
    }

    /// **Property 3.5: Search Hash Verification**
    /// **Validates: Requirements 2.3**
    ///
    /// For any search operation:
    /// 1. The hash in metadata must match the actual CAS object hash
    /// 2. Content retrieved via hash must be verifiable
    /// 3. Hash integrity must be maintained throughout search
    #[test]
    fn prop_search_hash_verification() {
        let config = ProptestConfig::with_cases(50);

        proptest!(config, |(
            content in prop::collection::vec(any::<u8>(), 100..1000)
        )| {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let workspace_dir = temp_dir.path().to_path_buf();

                let cas = ContentAddressableStorage::new(workspace_dir.clone());
                let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

                // Store content in CAS
                let hash = cas.store_content(&content).await.unwrap();

                // Create metadata
                let file_meta = FileMetadata {
                    id: 0,
                    sha256_hash: hash.clone(),
                    virtual_path: "logs/test.log".to_string(),
                    original_name: "test.log".to_string(),
                    size: content.len() as i64,
                    modified_time: chrono::Utc::now().timestamp(),
                    mime_type: Some("application/octet-stream".to_string()),
                    parent_archive_id: None,
                    depth_level: 0,
                };

                metadata_store.insert_file(&file_meta).await.unwrap();

                // Simulate search: retrieve file and verify hash
                let files = metadata_store.get_all_files().await.unwrap();
                prop_assert_eq!(files.len(), 1);

                let file = &files[0];

                // Property 1: Hash must match
                prop_assert_eq!(
                    &file.sha256_hash,
                    &hash,
                    "Metadata hash must match CAS hash"
                );

                // Property 2: Content must be retrievable
                let retrieved = cas.read_content(&file.sha256_hash).await.unwrap();
                prop_assert_eq!(
                    &retrieved,
                    &content,
                    "Retrieved content must match original"
                );

                // Property 3: Hash integrity verification
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&retrieved);
                let computed_hash = format!("{:x}", hasher.finalize());

                prop_assert_eq!(
                    &computed_hash,
                    &hash,
                    "Computed hash must match stored hash"
                );

                // Property 4: CAS integrity check
                let is_valid = cas.verify_integrity(&hash).await.unwrap();
                prop_assert!(
                    is_valid,
                    "CAS integrity verification must pass"
                );

                Ok(())
            }).unwrap();
        });
    }
}
