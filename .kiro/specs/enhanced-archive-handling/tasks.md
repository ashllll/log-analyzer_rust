# Implementation Plan

- [x] 1. Set up project infrastructure and dependencies





  - Install required Rust crates: sqlx (SQLite), sha2 (hashing), toml (config), proptest (property testing)
  - Create database migration scripts for path_mappings table with indexes
  - Set up TOML configuration schema and default config file
  - _Requirements: 6.1, 6.2_

- [x] 2. Implement PathManager with long path support





  - Create PathConfig structure with OS-specific defaults (Windows: 260, Unix: 4096)
  - Implement path length prediction algorithm considering base path, archive name, internal path, and depth
  - Implement Windows UNC prefix application (\\?\) for paths exceeding 260 characters
  - Implement content-based hashing using SHA-256 truncated to 16 characters
  - Implement collision detection and counter appending (_001, _002, etc.)
  - _Requirements: 1.1, 1.2, 1.3, 4.1, 4.2, 4.3, 4.4_

- [x] 2.1 Write property test for path shortening round-trip


  - **Property 4: Path mapping round-trip**
  - **Validates: Requirements 1.4**

- [x] 2.2 Write property test for Windows UNC prefix application


  - **Property 2: Windows UNC prefix application**
  - **Validates: Requirements 1.2**

- [x] 2.3 Write property test for path shortening idempotence


  - **Property 3: Path shortening consistency**
  - **Validates: Requirements 1.3**

- [x] 3. Implement MetadataDB with SQLite persistence





  - Create SQLite connection pool with sqlx
  - Implement store_mapping method with UNIQUE constraint handling
  - Implement get_original_path and get_short_path query methods with index usage
  - Implement cleanup_workspace method for workspace deletion
  - Implement increment_access_count for usage tracking
  - _Requirements: 1.4, 4.5_

- [x] 3.1 Write property test for mapping persistence


  - **Property 19: Mapping persistence**
  - **Validates: Requirements 4.5**

- [x] 3.2 Write unit tests for database operations


  - Test concurrent access, transaction handling, constraint violations
  - _Requirements: 1.4, 4.5_

- [x] 4. Implement SecurityDetector with zip bomb detection





  - Create SecurityPolicy structure with configurable thresholds
  - Implement calculate_compression_ratio method handling zero-byte edge cases
  - Implement calculate_risk_score using exponential backoff formula (ratio^depth)
  - Implement should_halt_extraction with cumulative size tracking
  - Implement detect_suspicious_patterns for pre-extraction analysis
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4.1 Write property test for compression ratio calculation


  - **Property 10: Compression ratio calculation**
  - **Validates: Requirements 3.1**

- [x] 4.2 Write property test for exponential backoff scoring


  - **Property 13: Exponential backoff scoring**
  - **Validates: Requirements 3.4**

- [x] 4.3 Write property test for suspicious file flagging


  - **Property 11: Suspicious file flagging**
  - **Validates: Requirements 3.2**

- [x] 4.4 Write unit tests for edge cases


  - Test zero-byte files, infinite ratios, negative values
  - _Requirements: 3.1, 3.2_

- [x] 5. Implement ExtractionContext and ExtractionStack





  - Create ExtractionContext structure with workspace_id, current_depth, parent_archive, accumulated metrics
  - Create ExtractionStack with push/pop operations for iterative traversal
  - Create ExtractionItem structure with archive_path, target_dir, depth, parent_context
  - Implement stack size limit to prevent memory exhaustion
  - _Requirements: 2.4_

- [x] 5.1 Write property test for context consistency


  - **Property 8: Extraction context consistency**
  - **Validates: Requirements 2.4**

- [x] 6. Implement ExtractionEngine with iterative traversal





  - Create ExtractionEngine structure with dependencies (PathManager, SecurityDetector, ProgressTracker)
  - Implement extract_archive entry point with initial context creation
  - Implement extract_iterative using explicit stack instead of recursion
  - Implement depth limit enforcement (default 10, configurable 1-20)
  - Implement process_archive_file for single archive processing
  - Implement extract_file_streaming with 64KB buffer for memory efficiency
  - _Requirements: 2.1, 2.2, 2.3, 2.5, 8.2_

- [x] 6.1 Write property test for depth limit enforcement


  - **Property 6: Depth limit enforcement**
  - **Validates: Requirements 2.1**

- [x] 6.2 Write property test for stack safety


  - **Property 7: Iterative traversal stack safety**
  - **Validates: Requirements 2.3**

- [x] 6.3 Write property test for sibling processing independence


  - **Property 9: Sibling processing independence**
  - **Validates: Requirements 2.5**

- [x] 6.4 Write integration test for deep nesting

  - Create 15-level nested archive, verify depth limit at 10
  - _Requirements: 2.1, 2.2_

- [x] 7. Implement ProgressTracker with hierarchical reporting





  - Create ProgressTracker structure with event emitter and metrics
  - Create ProgressMetrics with atomic counters for thread-safe updates
  - Implement emit_progress with hierarchical_path construction
  - Implement record_error with categorization (PathTooLong, ZipBombDetected, etc.)
  - Implement generate_summary with all required fields
  - Implement estimate_remaining_time based on current progress rate
  - _Requirements: 5.1, 5.2, 5.3, 5.5_

- [x] 7.1 Write property test for progress event completeness


  - **Property 20: Progress event completeness**
  - **Validates: Requirements 5.1**

- [x] 7.2 Write property test for hierarchical progress structure


  - **Property 21: Hierarchical progress structure**
  - **Validates: Requirements 5.2**

- [x] 7.3 Write property test for error categorization


  - **Property 22: Error categorization and resilience**
  - **Validates: Requirements 5.3**

- [x] 8. Implement PolicyManager with configuration validation





  - Create PolicyManager structure with config_path and current_policy
  - Implement load_policy from TOML file with error handling
  - Implement validate_policy checking all constraints (max_depth 1-20, positive sizes, etc.)
  - Implement update_policy with hot reload support using RwLock
  - Implement get_policy for thread-safe access
  - Create default policy with secure values
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 8.1 Write property test for configuration validation

  - **Property 26: Configuration validation enforcement**
  - **Validates: Requirements 6.3**

- [x] 8.2 Write property test for invalid configuration rejection

  - **Property 27: Invalid configuration rejection**
  - **Validates: Requirements 6.4**

- [x] 8.3 Write unit tests for TOML parsing

  - Test valid configs, invalid formats, missing fields, type mismatches
  - _Requirements: 6.1, 6.3_

- [x] 9. Implement edge case handlers





  - Implement Unicode normalization to NFC form using unicode-normalization crate
  - Implement duplicate filename detection with case-insensitive comparison on Windows
  - Implement incomplete extraction detection using checkpoint files
  - Implement disk space pre-flight check using filesystem stats
  - Implement circular reference detection using visited set with canonical paths
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 9.1 Write property test for Unicode normalization idempotence


  - **Property 29: Unicode normalization consistency**
  - **Validates: Requirements 7.1**

- [x] 9.2 Write property test for duplicate filename uniqueness


  - **Property 30: Duplicate filename uniqueness**
  - **Validates: Requirements 7.2**

- [x] 9.3 Write property test for circular reference detection


  - **Property 33: Circular reference detection**
  - **Validates: Requirements 7.5**

- [x] 9.4 Write integration test for interruption recovery


  - Kill process mid-extraction, restart, verify detection and resume
  - _Requirements: 7.3_

- [x] 10. Implement ExtractionOrchestrator with concurrency control





  - Create ExtractionOrchestrator structure with semaphore for concurrency limiting
  - Implement request deduplication using DashMap<archive_path, Arc<Mutex<Future>>>
  - Implement cancellation support using tokio::sync::CancellationToken
  - Implement concurrent extraction limiting (default: CPU cores / 2)
  - Implement graceful cleanup on cancellation
  - _Requirements: 8.1, 10.2, 10.3_

- [x] 10.1 Write property test for concurrency limit enforcement


  - **Property 34: Concurrency limit enforcement**
  - **Validates: Requirements 8.1**

- [x] 10.2 Write property test for request deduplication


  - **Property 45: Request deduplication**
  - **Validates: Requirements 10.3**

- [x] 10.3 Write property test for cancellation responsiveness


  - **Property 44: Cancellation responsiveness**
  - **Validates: Requirements 10.2**

- [x] 10.4 Write integration test for concurrent extractions


  - Submit 20 concurrent requests, verify limit enforcement and throughput
  - _Requirements: 8.1, 10.3_

- [x] 11. Implement audit logging with structured format





  - Integrate tracing crate with JSON formatter
  - Implement extraction start logging with all required fields
  - Implement security event logging at WARN level
  - Implement completion/failure logging with metrics
  - Implement log rotation based on size (100MB) and time (24 hours)
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 11.1 Write property test for audit log completeness

  - **Property 39: Audit log completeness**
  - **Validates: Requirements 9.1**

- [x] 11.2 Write property test for structured log format

  - **Property 42: Structured log format**
  - **Validates: Requirements 9.4**

- [x] 11.3 Write property test for security event logging

  - **Property 40: Security event logging**
  - **Validates: Requirements 9.2**

- [x] 12. Implement public API with sync and async interfaces





  - Create synchronous extract_archive_sync wrapper using block_on
  - Create asynchronous extract_archive_async with tokio runtime
  - Implement ExtractionResult structure with all required fields
  - Implement ExtractionError structure with error_code, message, path, remediation
  - Implement error conversion from internal errors to public API errors
  - _Requirements: 10.1, 10.4, 10.5_

- [x] 12.1 Write property test for error structure completeness


  - **Property 46: Error structure completeness**
  - **Validates: Requirements 10.4**

- [x] 12.2 Write property test for result structure completeness


  - **Property 47: Result structure completeness**
  - **Validates: Requirements 10.5**

- [x] 12.3 Write unit tests for API interfaces


  - Test sync/async equivalence, error propagation, result consistency
  - _Requirements: 10.1, 10.4, 10.5_

- [x] 13. Integrate with existing ArchiveManager





  - Update ArchiveManager to use new ExtractionEngine for secure extraction
  - Add feature flag for gradual rollout (use_enhanced_extraction)
  - Maintain backward compatibility with existing extraction path
  - Update import commands to pass workspace_id to extraction engine
  - Add configuration UI for policy management in settings
  - _Requirements: All_

- [x] 13.1 Write integration tests for ArchiveManager compatibility


  - Test existing archives work with new engine, path mappings are accessible
  - _Requirements: All_

- [x] 14. Implement resource management and cleanup





  - Implement temporary directory cleanup with TTL (default 24 hours)
  - Implement file handle release within 5 seconds of completion
  - Implement memory buffer cleanup using Drop trait
  - Implement workspace cleanup on deletion (remove path mappings, temp files)
  - _Requirements: 8.4, 8.5_

- [x] 14.1 Write property test for resource release timing


  - **Property 38: Resource release timing**
  - **Validates: Requirements 8.5**

- [x] 14.2 Write property test for temporary file cleanup


  - **Property 37: Temporary file cleanup**
  - **Validates: Requirements 8.4**

- [x] 15. Implement resumption and checkpoint support





  - Create checkpoint file format with last_extracted_file, accumulated_metrics
  - Implement checkpoint writing at regular intervals (every 100 files or 1GB)
  - Implement checkpoint reading on extraction start
  - Implement resume logic skipping already extracted files
  - Implement checkpoint cleanup on successful completion
  - _Requirements: 5.4_

- [x] 15.1 Write property test for resumption checkpoint consistency

  - **Property 23: Resumption checkpoint consistency**
  - **Validates: Requirements 5.4**

- [x] 15.2 Write integration test for pause and resume

  - Pause extraction mid-way, resume, verify no duplicate extractions
  - _Requirements: 5.4_

- [x] 16. Checkpoint - Ensure all tests pass





  - Run cargo test to verify all unit tests pass
  - Run property tests with 1000 iterations each
  - Run integration tests with real-world archive samples
  - Ensure all tests pass, ask the user if questions arise.

- [x] 17. Implement performance optimizations





  - Implement directory creation batching (batch size: 10)
  - Implement streaming extraction with configurable buffer (default 64KB)
  - Implement parallel file extraction within single archive (up to 4 files)
  - Implement path mapping cache using DashMap for fast lookups
  - Profile and optimize hot paths (hashing, database queries)
  - _Requirements: 8.2, 8.3_

- [x] 17.1 Write property test for streaming memory bounds


  - **Property 35: Streaming memory bounds**
  - **Validates: Requirements 8.2**

- [x] 17.2 Write property test for directory creation batching


  - **Property 36: Directory creation batching**
  - **Validates: Requirements 8.3**

- [x] 17.3 Write performance benchmarks


  - Benchmark extraction speed (MB/s), memory usage, concurrency scaling
  - _Requirements: 8.1, 8.2, 8.3_

- [x] 18. Implement security testing and hardening





  - Create test archives for zip bomb detection (42.zip style)
  - Create test archives with path traversal attempts (../../../etc/passwd)
  - Create test archives with circular symlinks
  - Create test archives with 1 million tiny files
  - Create test archives with filenames containing null bytes and control characters
  - Verify all security tests pass and attacks are detected/blocked
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 18.1 Write security integration tests


  - Test zip bomb detection, path traversal rejection, symlink cycle detection
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 19. Create migration tooling and documentation





  - Create migration script for existing deployments
  - Create configuration migration tool (old format → new TOML)
  - Write user documentation for new features (long paths, deep nesting, security)
  - Write operator documentation for configuration and monitoring
  - Write developer documentation for API usage and extension points
  - _Requirements: All_

- [x] 20. Final checkpoint - Comprehensive validation





  - Run full test suite (unit + property + integration + security)
  - Run performance benchmarks and verify targets met
  - Run security tests and verify all attacks blocked
  - Verify backward compatibility with existing archives
  - Verify configuration hot reload works correctly
  - Ensure all tests pass, ask the user if questions arise.
