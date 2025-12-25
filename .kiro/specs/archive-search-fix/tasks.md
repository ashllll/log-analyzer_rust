# Implementation Plan - Archive Search Fix

## Phase 1: Content-Addressable Storage (CAS) Foundation

- [x] 1. Set up project dependencies and structure








  - Add `sqlx` with SQLite feature to Cargo.toml
  - Add `sha2` for hashing to Cargo.toml
  - Add `async-compression` for streaming extraction
  - Create module structure: `src-tauri/src/storage/`
  - _Requirements: 2.1, 2.2_


- [x] 1.1 Implement ContentAddressableStorage core


  - Create `src-tauri/src/storage/cas.rs`
  - Implement `compute_hash()` using SHA-256
  - Implement `store_content()` with deduplication
  - Implement `get_object_path()` using Git-style 2-char prefix
  - Implement `read_content()` with error handling
  - _Requirements: 2.2, 7.1_

- [x] 1.2 Write unit tests for CAS





  - Test hash computation consistency
  - Test content storage and retrieval
  - Test deduplication (same content → same hash)
  - Test object path generation
  - _Requirements: 2.2_


- [x] 1.3 Implement incremental hashing for large files



  - Create `compute_hash_incremental()` with 8KB buffer
  - Add streaming support to avoid memory spikes
  - _Requirements: 6.2_

- [x] 1.4 Write property test for CAS






  - **Property 1: Hash idempotence**
  - **Validates: Requirements 2.2**
  - _For any_ content, computing hash twice produces same result

## Phase 2: SQLite Metadata Store

- [x] 2. Create database schema





  - Create `src-tauri/src/storage/schema.sql`
  - Define `files` table with all required fields
  - Define `archives` table for nested tracking
  - Create indexes on `virtual_path` and `parent_archive_id`
  - Create FTS5 virtual table for full-text search
  - _Requirements: 2.1, 2.3_

- [x] 2.1 Implement MetadataStore


  - Create `src-tauri/src/storage/metadata_store.rs`
  - Implement `new()` with database initialization
  - Implement `insert_file()` with transaction support
  - Implement `insert_archive()` for nested tracking
  - Implement `get_file_by_virtual_path()`
  - Implement `get_archive_children()` for hierarchy
  - _Requirements: 2.1, 2.3, 4.1_

- [x] 2.2 Write unit tests for MetadataStore






  - Test database initialization
  - Test file insertion and retrieval
  - Test archive hierarchy queries
  - Test virtual path lookups
  - _Requirements: 2.1_

- [x] 2.3 Implement search queries


  - Add `search_files()` using FTS5
  - Add `get_all_files()` for validation
  - Add `count_files()` and `count_archives()` for metrics
  - _Requirements: 1.4_

- [x] 2.4 Write property test for metadata consistency






  - **Property 2: Path Map completeness**
  - **Validates: Requirements 1.2, 1.3**
  - _For any_ extracted file, it must exist in metadata store

## Phase 3: Archive Processor Integration

- [x] 3. Refactor ArchiveProcessor to use CAS





  - Modify `src-tauri/src/archive/processor.rs`
  - Update `process_path_recursive_inner()` to use CAS
  - Replace direct file storage with `cas.store_content()`
  - Update path mapping to use SHA-256 hashes
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 3.1 Implement nested archive processing











  - Update `extract_and_process_archive()` for CAS
  - Add depth tracking for nested archives
  - Store archive metadata in MetadataStore
  - Implement recursive processing with depth limit
  - _Requirements: 4.1, 4.2, 4.3_


- [x] 3.2 Add path validation




  - Implement `validate_virtual_path()` to prevent traversal
  - Add path length checks (though CAS removes limits)
  - Validate file existence before storing
  - _Requirements: 1.5, 7.2, 8.3_

- [x] 3.3 Write integration tests for archive processing






  - Test single archive extraction
  - Test nested archive (2-3 levels)
  - Test deeply nested archive (5+ levels)
  - Test path length handling
  - _Requirements: 4.1, 4.4_


- [x] 3.4 Write property test for nested archives







  - **Property 5: Nested archive flattening**
  - **Validates: Requirements 4.1, 4.4**
  - _For any_ nested structure, all leaf files accessible via metadata

## Phase 4: Search Engine Integration

- [x] 4. Update search to use MetadataStore





  - Modify `src-tauri/src/commands/search.rs`
  - Replace HashMap lookup with SQLite query
  - Use `metadata.search_files()` for candidates
  - Read content from CAS using SHA-256 hash
  - _Requirements: 1.4_


- [x] 4.1 Implement hash-based file access





  - Update `search_single_file_with_details()` signature
  - Accept SHA-256 hash instead of real_path
  - Use `cas.read_content()` to get file data
  - Maintain virtual_path for display
  - _Requirements: 1.4, 1.5_

- [x] 4.2 Add path existence validation






  - Verify hash exists in CAS before reading
  - Log warning for missing files
  - Skip invalid entries gracefully
  - _Requirements: 8.1, 8.3_


- [x] 4.3 Write integration tests for search







  - Test search on CAS-stored files
  - Test search with nested archives
  - Test search performance (should be faster)
  - _Requirements: 1.4_

- [x] 4.4 Write property test for search access











  - **Property 4: Search file access**
  - **Validates: Requirements 1.4, 8.3**
  - _For any_ file in search results, opening must succeed

## Phase 5: Error Handling and Recovery


- [x] 5. Implement transactional processing







  - Add transaction support to archive processing
  - Use `sqlx::Transaction` for atomic operations
  - Rollback on failure to maintain consistency
  - _Requirements: 8.4_

- [x] 5.1 Implement checkpoint system







  - Create `ProcessingCheckpoint` struct
  - Implement `save_checkpoint()` every 100 files
  - Implement `resume_from_checkpoint()`
  - _Requirements: 8.4_


- [x] 5.2 Add integrity verification






  - Implement `verify_file_integrity()` using hash
  - Add verification step after import
  - Generate validation report
  - _Requirements: 2.4_



- [x] 5.3 Write tests for error recovery






  - Test transaction rollback on failure
  - Test checkpoint save and resume
  - Test integrity verification
  - _Requirements: 8.1, 8.4_


- [x] 5.4 Write property test for error isolation





  - **Property 7: Error recovery isolation**
  - **Validates: Requirements 8.1, 8.4**
  - _For any_ single file failure, remaining files process successfully

## Phase 6: Workspace Management Updates


- [x] 6. Update workspace deletion




  - Modify `src-tauri/src/commands/workspace.rs`
  - Delete SQLite database file
  - Delete CAS objects directory
  - Update cleanup logic for new structure
  - _Requirements: 5.1_

- [x] 6.1 Implement workspace validation







  - Create `IndexValidator` in `src-tauri/src/services/index_validator.rs`
  - Implement `validate_path_map()` → `validate_metadata()`
  - Check all hashes exist in CAS
  - Generate validation report
  - _Requirements: 2.4_


- [x] 6.2 Add workspace metrics






  - Implement `collect_metrics()` for CAS
  - Track deduplication ratio
  - Track storage efficiency
  - Track max nesting depth
  - _Requirements: 3.1_


- [x] 6.3 Write tests for workspace management







  - Test workspace creation with CAS
  - Test workspace deletion (cleanup)
  - Test validation report generation
  - _Requirements: 5.1, 5.2_

## Phase 7: Frontend Integration

- [x] 7. Create virtual file tree API





  - Add Tauri command `get_virtual_file_tree`
  - Query metadata store for file hierarchy
  - Build tree structure from flat data
  - Return JSON with virtual paths and hashes
  - _Requirements: 4.2_


- [x] 7.1 Add file content retrieval by hash





  - Add Tauri command `read_file_by_hash`
  - Accept SHA-256 hash parameter
  - Read from CAS and return content
  - Handle errors gracefully
  - _Requirements: 1.4_

- [x] 7.2 Implement VirtualFileTree React component






  - Create `src/components/VirtualFileTree.tsx`
  - Display nested archive structure
  - Support expand/collapse for archives
  - Handle file click to show content
  - _Requirements: 4.2_

- [x] 7.3 Update search results display


  - Modify search results to use virtual paths
  - Display full nested path in results
  - Update file opening to use hash-based retrieval
  - _Requirements: 1.4_


- [x] 7.4 Write E2E tests for frontend





  - Test file tree rendering
  - Test nested archive navigation
  - Test file content display
  - Test search with virtual paths
  - _Requirements: 4.2_

## Phase 8: Migration and Compatibility


- [x] 8. Implement data migration tool









  - Create `src-tauri/src/migration/mod.rs`
  - Implement `migrate_workspace_to_cas()`
  - Read old path_map format
  - Convert to CAS + metadata store
  - Verify migration completeness
  - _Requirements: 8.4_


- [x] 8.1 Add backward compatibility layer





  - Detect old vs new workspace format
  - Support loading old workspaces (read-only)
  - Prompt user to migrate
  - _Requirements: 8.4_


- [x] 8.2 Write migration tests






  - Test migration from old format
  - Test data integrity after migration
  - Test backward compatibility
  - _Requirements: 8.4_

## Phase 9: Performance Optimization


- [x] 9. Implement parallel processing




  - Use `rayon` for parallel file processing
  - Process multiple archives concurrently
  - Batch database insertions
  - _Requirements: 6.2_


- [x] 9.1 Add streaming extraction






  - Implement `stream_extract_gz()` for large files
  - Use `async-compression` for streaming
  - Avoid loading entire archive into memory
  - _Requirements: 6.1, 6.2_


- [x] 9.2 Write performance tests







  - Benchmark CAS vs old approach
  - Test with large archives (1GB+)
  - Test with deeply nested archives (10+ levels)
  - Verify memory usage stays reasonable
  - _Requirements: 6.1, 6.2_

## Phase 10: Documentation and Cleanup


- [x] 10. Update documentation




  - Document CAS architecture in README
  - Add migration guide for users
  - Update API documentation
  - Add troubleshooting section
  - _Requirements: 3.1_


- [x] 10.1 Remove temporary debug code

  - Remove debug logging added during development
  - Clean up commented code
  - Update error messages for clarity
  - _Requirements: 3.3_

- [x] 10.2 Final validation


  - Run all tests (unit, integration, property, E2E)
  - Verify all requirements are met
  - Test on Windows, Linux, macOS
  - Performance regression testing
  - _Requirements: All_

## Checkpoint Tasks

- [x] Checkpoint 1: After Phase 2





  - Ensure all CAS and MetadataStore tests pass
  - Verify database schema is correct
  - Ask user if questions arise

- [x] Checkpoint 2: After Phase 4





  - Ensure search integration works
  - Verify nested archives are processed correctly
  - Test end-to-end import and search flow
  - Ask user if questions arise

- [x] Checkpoint 3: After Phase 7





  - Ensure frontend displays virtual file tree
  - Verify user can navigate nested archives
  - Test complete user workflow
  - Ask user if questions arise


- [x] Final Checkpoint: After Phase 10




  - Ensure all tests pass
  - Verify all requirements are met
  - Performance meets expectations
  - Ready for release
