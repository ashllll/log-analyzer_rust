# Task 3.1: Nested Archive Processing Implementation

## Summary

Task 3.1 has been successfully completed. The implementation adds comprehensive nested archive processing support with CAS (Content-Addressable Storage) integration, depth tracking, and metadata management.

## Implementation Details

### 1. CAS-Based Archive Processing

The core function `extract_and_process_archive_with_cas_and_checkpoints` provides:

- **Content-Addressable Storage**: Archives and their contents are stored using SHA-256 hashes
- **Streaming Support**: Large files are processed using `store_file_streaming()` to avoid memory issues
- **Deduplication**: Identical files are automatically deduplicated through hash-based storage

### 2. Depth Tracking

Nested archive processing includes robust depth tracking:

```rust
const MAX_NESTING_DEPTH: i32 = 10;

if depth_level >= MAX_NESTING_DEPTH {
    warn!(
        archive = %archive_path.display(),
        depth = depth_level,
        "Maximum nesting depth reached, skipping archive"
    );
    return Ok(());
}
```

- **Depth Limit**: Maximum nesting depth of 10 levels prevents infinite recursion
- **Depth Propagation**: Each recursive call increments `depth_level + 1`
- **Graceful Handling**: Archives exceeding the depth limit are logged and skipped

### 3. Archive Metadata Storage

Complete archive metadata tracking in MetadataStore:

```rust
// Create archive metadata
let archive_metadata = ArchiveMetadata {
    id: 0, // Auto-generated
    sha256_hash: archive_hash.clone(),
    virtual_path: normalize_path_separator(virtual_path),
    original_name: file_name.clone(),
    archive_type: archive_type.clone(),
    parent_archive_id,
    depth_level,
    extraction_status: "pending".to_string(),
};

// Insert and track status
let archive_id = context.metadata_store.insert_archive(&archive_metadata).await?;
context.metadata_store.update_archive_status(archive_id, "extracting").await?;
// ... process files ...
context.metadata_store.update_archive_status(archive_id, "completed").await?;
```

Features:
- **Archive Hierarchy**: `parent_archive_id` links nested archives to their parents
- **Status Tracking**: Extraction status progresses through "pending" → "extracting" → "completed"
- **Archive Type Detection**: Automatically detects ZIP, RAR, TAR, GZ formats
- **Hash-Based Identity**: Each archive is uniquely identified by its SHA-256 hash

### 4. Recursive Processing with Depth Limit

The implementation supports recursive processing of nested archives:

```rust
// Process extracted files recursively
for extracted_file in &extracted_files {
    // Validate path safety
    if let Err(e) = validate_path_safety(extracted_file, &extract_dir) {
        warn!(file = %extracted_file.display(), error = %e, "Skipping unsafe file");
        continue;
    }

    let new_virtual = format!("{}/{}/{}", virtual_path, file_name, relative_path);

    // Recursively process (supports nested archives)
    Box::pin(process_path_with_cas_and_checkpoints(
        extracted_file,
        &new_virtual,
        context,
        app,
        task_id,
        workspace_id,
        Some(archive_id),  // Parent archive ID
        depth_level + 1,    // Increment depth
    ))
    .await?;
}
```

Key features:
- **Path Safety Validation**: Prevents path traversal attacks
- **Virtual Path Construction**: Maintains full nested path hierarchy
- **Parent Tracking**: Each file/archive knows its parent archive
- **Async Recursion**: Uses `Box::pin` for async recursive calls

## Testing

### Integration Tests

All integration tests pass successfully:

```
test test_single_archive_extraction ... ok
test test_nested_archive_2_levels ... ok
test test_nested_archive_3_levels ... ok
test test_deeply_nested_archive_5_levels ... ok
test test_path_length_handling ... ok
test test_mixed_nested_and_regular_files ... ok
```

### Property-Based Test

The property test validates the core correctness property:

**Property 5: Nested Archive Flattening**
- *For any* nested archive structure, all leaf files must be accessible through the metadata store
- Validates: Requirements 4.1, 4.4
- Tests depths 1-3 with 1-2 files per level
- Verifies all files are indexed, retrievable, and accessible via CAS

```
test property_tests::prop_nested_archive_flattening ... ok
```

## Requirements Validation

### ✅ Requirement 4.1: Recursive Extraction
> WHEN 压缩包内包含其他压缩包时 THEN System SHALL 递归解压所有嵌套的压缩包

**Implementation**: The `extract_and_process_archive_with_cas_and_checkpoints` function recursively processes all nested archives by detecting archive files and calling itself with incremented depth.

### ✅ Requirement 4.2: Virtual Path Hierarchy
> WHEN 构建虚拟路径时 THEN System SHALL 保持嵌套结构的层次关系

**Implementation**: Virtual paths are constructed as `parent.zip/child.zip/file.log`, maintaining the full hierarchy.

### ✅ Requirement 4.3: Depth Limit
> WHEN 嵌套层级超过限制时 THEN System SHALL 停止递归并记录警告信息

**Implementation**: `MAX_NESTING_DEPTH = 10` enforces the limit, with warning logs for exceeded depth.

### ✅ Requirement 4.4: Complete Accessibility
> WHEN 所有嵌套解压完成后 THEN System SHALL 确保所有文件都可通过 Path Map 访问

**Implementation**: All files are stored in CAS and indexed in MetadataStore, ensuring complete accessibility through hash-based retrieval.

## Architecture

### Data Flow

```
Archive File
    ↓
Extract to temp directory
    ↓
Store archive in CAS (SHA-256 hash)
    ↓
Insert archive metadata (with parent_archive_id, depth_level)
    ↓
Update status: "extracting"
    ↓
For each extracted file:
    ├─ If archive → Recursive call (depth + 1)
    └─ If regular file → Store in CAS + Insert file metadata
    ↓
Update status: "completed"
```

### Key Components

1. **CasProcessingContext**: Encapsulates CAS, MetadataStore, and checkpoint support
2. **ArchiveMetadata**: Tracks archive hierarchy and extraction status
3. **FileMetadata**: Links files to parent archives with depth information
4. **Depth Tracking**: Prevents infinite recursion with configurable limit

## Backward Compatibility

The legacy `extract_and_process_archive` function (HashMap-based) has been updated with:
- Depth tracking to prevent infinite recursion
- Depth limit checking with warnings
- Maintains compatibility with existing code

New code should use `extract_and_process_archive_with_cas` for full CAS support.

## Performance Considerations

1. **Streaming Extraction**: Large files use `store_file_streaming()` to avoid memory spikes
2. **Deduplication**: Identical files are stored only once via hash-based storage
3. **Checkpoint Support**: Optional checkpoint system for resumable processing
4. **Parallel Processing**: Ready for future parallel extraction optimization

## Security

1. **Path Traversal Prevention**: `validate_path_safety()` checks all extracted paths
2. **Depth Limit**: Prevents zip bomb attacks through excessive nesting
3. **Hash Verification**: Content integrity verified through SHA-256 hashes

## Next Steps

The implementation is complete and all tests pass. The system now supports:
- ✅ Nested archive extraction up to 10 levels deep
- ✅ Complete metadata tracking with parent-child relationships
- ✅ CAS-based storage with deduplication
- ✅ Virtual path hierarchy preservation
- ✅ Graceful depth limit handling

Task 3.1 is ready for integration with the search engine (Task 4.x).
