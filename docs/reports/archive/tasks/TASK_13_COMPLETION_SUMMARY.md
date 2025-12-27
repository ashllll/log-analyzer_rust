# Task 13 Completion Summary: Update commands/async_search.rs

## Overview
Successfully updated `commands/async_search.rs` to use CAS (Content-Addressable Storage) architecture instead of the old `path_map` system.

## Changes Made

### 1. Updated Imports
**Before:**
```rust
use std::sync::Arc;
use tokio::fs::File;
```

**After:**
```rust
use std::path::Path;
use crate::storage::{ContentAddressableStorage, MetadataStore};
```

- Removed `Arc` import (no longer needed for path_map)
- Removed `File` import (no longer reading files directly)
- Added `Path` import for workspace directory handling
- Added `ContentAddressableStorage` and `MetadataStore` imports

### 2. Updated Function Signature
**Before:**
```rust
async fn perform_async_search(
    _app: AppHandle,
    query: String,
    _workspace_id: String,
    max_results: usize,
    timeout: Duration,
    cancellation_token: CancellationToken,
    path_map: Arc<parking_lot::Mutex<std::collections::HashMap<String, String>>>,
    search_id: String,
) -> Result<usize, String>
```

**After:**
```rust
async fn perform_async_search(
    app: AppHandle,
    query: String,
    workspace_id: String,
    max_results: usize,
    timeout: Duration,
    cancellation_token: CancellationToken,
    search_id: String,
) -> Result<usize, String>
```

- ✅ Removed `path_map` parameter
- ✅ Changed `_app` to `app` (now used to get workspace directory)
- ✅ Changed `_workspace_id` to `workspace_id` (now actively used)

### 3. Updated File List Retrieval
**Before:**
```rust
// Get file list from path_map
let files: Vec<(String, String)> = {
    let guard = path_map.lock();
    guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
};
```

**After:**
```rust
// Get workspace directory
let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data dir: {}", e))?;
let workspace_dir = app_data_dir.join("extracted").join(&workspace_id);

if !workspace_dir.exists() {
    return Err(format!("Workspace not found: {}", workspace_id));
}

// Initialize MetadataStore and CAS
let metadata_store = MetadataStore::new(&workspace_dir)
    .await
    .map_err(|e| format!("Failed to open metadata store: {}", e))?;

let cas = ContentAddressableStorage::new(workspace_dir);

// Get file list from MetadataStore
let files = metadata_store
    .get_all_files()
    .await
    .map_err(|e| format!("Failed to get file list: {}", e))?;
```

- ✅ Uses `MetadataStore` to get file list
- ✅ Initializes CAS for content reading
- ✅ Proper error handling for workspace not found

### 4. Updated File Content Reading
**Before:**
```rust
// Read file directly from filesystem
match search_file_async(real_path, virtual_path, &query, results_count).await {
    // ...
}

async fn search_file_async(
    real_path: &str,
    virtual_path: &str,
    query: &str,
    global_offset: usize,
) -> Result<Vec<LogEntry>, String> {
    let file = File::open(real_path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    // ...
}
```

**After:**
```rust
// Read file content from CAS using SHA-256 hash
let content = match cas.read_content(&file.sha256_hash).await {
    Ok(bytes) => bytes,
    Err(e) => {
        tracing::warn!(
            search_id = %search_id,
            file = %file.virtual_path,
            hash = %file.sha256_hash,
            error = %e,
            "Failed to read file from CAS"
        );
        continue;
    }
};

// Convert bytes to string
let content_str = match String::from_utf8(content) {
    Ok(s) => s,
    Err(e) => {
        tracing::warn!(
            search_id = %search_id,
            file = %file.virtual_path,
            error = %e,
            "Failed to decode file content as UTF-8"
        );
        continue;
    }
};

// Search content line by line
match search_content_async(&content_str, &file.virtual_path, &query, results_count).await {
    // ...
}

async fn search_content_async(
    content: &str,
    virtual_path: &str,
    query: &str,
    global_offset: usize,
) -> Result<Vec<LogEntry>, String> {
    for line in content.lines() {
        // ...
    }
}
```

- ✅ Uses `CAS::read_content()` with SHA-256 hash
- ✅ Proper error handling for CAS read failures
- ✅ Proper error handling for UTF-8 decoding
- ✅ Changed from async file reading to in-memory string processing

### 5. Updated Function Call
**Before:**
```rust
let path_map = Arc::clone(&state.path_map);

tauri::async_runtime::spawn(async move {
    let result = perform_async_search(
        app_handle,
        query_clone,
        workspace_id,
        max_results,
        timeout,
        cancellation_token,
        path_map,
        search_id_clone.clone(),
    )
    .await;
    // ...
});
```

**After:**
```rust
let workspace_id_clone = workspace_id.clone();

tauri::async_runtime::spawn(async move {
    let result = perform_async_search(
        app_handle,
        query_clone,
        workspace_id_clone,
        max_results,
        timeout,
        cancellation_token,
        search_id_clone.clone(),
    )
    .await;
    // ...
});
```

- ✅ Removed `path_map` cloning
- ✅ Added `workspace_id` cloning for async task

### 6. Updated Tests
**Before:**
```rust
#[tokio::test]
async fn test_search_file_async() {
    let mut temp_file = NamedTempFile::new().unwrap();
    let content = "2023-01-01 10:00:00 INFO Test log entry\n...";
    temp_file.write_all(content.as_bytes()).unwrap();
    
    let file_path = temp_file.path().to_str().unwrap();
    let results = search_file_async(file_path, "test.log", "Test", 0)
        .await
        .unwrap();
    // ...
}
```

**After:**
```rust
#[tokio::test]
async fn test_search_content_async() {
    let content = "2023-01-01 10:00:00 INFO Test log entry\n...";
    
    let results = search_content_async(content, "test.log", "Test", 0)
        .await
        .unwrap();
    // ...
}
```

- ✅ Updated test to use in-memory content instead of file I/O
- ✅ Simplified test by removing temporary file creation

## Requirements Validated

### Requirement 2.3
✅ **WHEN executing search THEN System SHALL query MetadataStore and read from CAS**
- Search now uses `MetadataStore::get_all_files()` to get file list
- Search now uses `CAS::read_content()` to read file content by hash

### Requirement 8.1
✅ **WHEN calling Tauri commands THEN System SHALL use CAS architecture**
- `async_search_logs` command now uses CAS architecture
- All file access goes through CAS using SHA-256 hashes
- No direct filesystem access to log files

## Verification

### Compilation Status
- ✅ No compilation errors in `async_search.rs`
- ✅ All changes follow the CAS architecture pattern
- ⚠️ Other files still have compilation errors (covered by other tasks)

### Code Quality
- ✅ Proper error handling with descriptive messages
- ✅ Structured logging with tracing
- ✅ Consistent with other CAS-based commands (workspace.rs, import.rs)
- ✅ No references to old `path_map` system

### Architecture Compliance
- ✅ Uses `MetadataStore` for file metadata queries
- ✅ Uses `CAS` for content retrieval by hash
- ✅ Uses workspace directory pattern from AppState
- ✅ Follows async/await patterns consistently

## Impact Analysis

### Files Modified
1. `log-analyzer/src-tauri/src/commands/async_search.rs` - Complete rewrite to use CAS

### Files NOT Modified (as expected)
- No other files call `perform_async_search` directly (it's internal)
- `async_search_logs` command signature unchanged (public API stable)

### Breaking Changes
- None - the public API (`async_search_logs` command) remains unchanged
- Internal implementation changed from path_map to CAS (transparent to callers)

## Next Steps

This task is complete. The next tasks in the migration plan are:

1. **Task 14**: Verify `commands/search.rs` uses CAS
2. **Task 15**: Verify `archive/processor.rs` uses CAS
3. **Task 16**: Compilation verification

## Notes

- The implementation follows the exact same pattern as `commands/workspace.rs`
- Error handling is comprehensive with proper logging
- The change is backward compatible at the API level
- Performance should be similar or better (CAS caching benefits)
