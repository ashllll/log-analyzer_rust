# Task 33: Manual Functional Testing Guide

## Overview

This document provides a comprehensive manual testing checklist for verifying the complete CAS migration. All tests should be performed to ensure the system works correctly with the new CAS architecture.

**Requirements Validated**: 2.1, 2.2, 2.3, 2.4, 2.5

## Pre-Testing Setup

### 1. Build the Application

```bash
# Backend
cd log-analyzer/src-tauri
cargo build --release

# Frontend
cd ..
npm run build

# Run the application
npm run tauri dev
```

### 2. Prepare Test Data

Create the following test data structure:

```
test-data/
├── simple-folder/
│   ├── test1.log
│   ├── test2.log
│   └── subfolder/
│       └── test3.log
├── test-archive.zip
├── nested-archive.zip (contains another .zip inside)
└── large-archive.zip (>100MB with multiple files)
```

## Test Suite

---

## Test 1: Import Folder (Requirement 2.1)

### Objective
Verify that importing a folder stores all files in CAS and creates correct metadata entries.

### Steps

1. **Launch Application**
   - [ ] Application starts without errors
   - [ ] No console errors in developer tools

2. **Create New Workspace**
   - [ ] Click "New Workspace" or similar button
   - [ ] Enter workspace name: "Test Folder Import"
   - [ ] Workspace is created successfully

3. **Import Folder**
   - [ ] Click "Import Folder" button
   - [ ] Select `test-data/simple-folder/`
   - [ ] Import progress is displayed
   - [ ] Import completes successfully
   - [ ] Success message is shown

4. **Verify File Tree**
   - [ ] File tree displays all imported files
   - [ ] Folder structure is preserved
   - [ ] File names are correct
   - [ ] Subfolder files are visible

5. **Verify CAS Storage (Backend)**
   ```bash
   # Check workspace directory structure
   ls -la ~/.log-analyzer/workspaces/<workspace-id>/
   
   # Should see:
   # - metadata.db (SQLite database)
   # - objects/ (CAS storage directory)
   ```
   - [ ] `metadata.db` exists
   - [ ] `objects/` directory exists
   - [ ] Objects are stored in Git-style structure (e.g., `objects/ab/cdef123...`)

6. **Verify Metadata (Backend)**
   ```bash
   # Query metadata database
   sqlite3 ~/.log-analyzer/workspaces/<workspace-id>/metadata.db
   
   SELECT COUNT(*) FROM files;
   SELECT virtual_path, sha256_hash FROM files;
   ```
   - [ ] File count matches imported files
   - [ ] All files have SHA-256 hashes
   - [ ] Virtual paths are correct

7. **Verify Deduplication**
   - [ ] Create two identical files in test folder
   - [ ] Import again
   - [ ] Check that only one object exists in CAS for identical content
   ```bash
   # Count objects
   find ~/.log-analyzer/workspaces/<workspace-id>/objects -type f | wc -l
   ```

### Expected Results
- ✅ All files imported successfully
- ✅ CAS storage structure is correct
- ✅ Metadata database contains all file records
- ✅ Deduplication works (identical files share same hash)
- ✅ No errors in console or logs

---

## Test 2: Import Archive (Requirement 2.2)

### Objective
Verify that importing a ZIP archive extracts and stores all files in CAS.

### Steps

1. **Create New Workspace**
   - [ ] Create workspace: "Test Archive Import"

2. **Import ZIP Archive**
   - [ ] Click "Import Archive" button
   - [ ] Select `test-data/test-archive.zip`
   - [ ] Import progress is displayed
   - [ ] Import completes successfully

3. **Verify Extraction**
   - [ ] File tree shows extracted files
   - [ ] Archive structure is preserved
   - [ ] All files from archive are visible

4. **Verify CAS Storage**
   ```bash
   # Check objects directory
   find ~/.log-analyzer/workspaces/<workspace-id>/objects -type f
   ```
   - [ ] All extracted files are in CAS
   - [ ] No temporary extraction files remain

5. **Verify Archive Metadata**
   ```bash
   sqlite3 ~/.log-analyzer/workspaces/<workspace-id>/metadata.db
   
   SELECT * FROM archives;
   SELECT COUNT(*) FROM files WHERE parent_archive_id IS NOT NULL;
   ```
   - [ ] Archive record exists in `archives` table
   - [ ] Files are linked to parent archive

6. **Test Different Archive Formats**
   - [ ] Import `.tar.gz` archive
   - [ ] Import `.tar` archive
   - [ ] Import `.gz` file
   - [ ] All formats work correctly

### Expected Results
- ✅ ZIP archive imported and extracted
- ✅ All files stored in CAS
- ✅ Archive metadata recorded
- ✅ Multiple archive formats supported
- ✅ No extraction artifacts left behind

---

## Test 3: Nested Archive (Requirement 2.2, 2.3)

### Objective
Verify that nested archives (archives within archives) are handled correctly.

### Steps

1. **Create Nested Archive**
   ```bash
   # Create nested structure
   mkdir -p nested-test/inner
   echo "Level 1" > nested-test/file1.log
   echo "Level 2" > nested-test/inner/file2.log
   cd nested-test/inner
   zip inner.zip file2.log
   cd ..
   zip -r nested.zip file1.log inner/inner.zip
   ```

2. **Import Nested Archive**
   - [ ] Create workspace: "Test Nested Archive"
   - [ ] Import `nested.zip`
   - [ ] Import completes successfully

3. **Verify Nested Structure**
   - [ ] File tree shows nested structure
   - [ ] Inner archive is visible
   - [ ] Files from both levels are accessible

4. **Verify Depth Levels**
   ```bash
   sqlite3 ~/.log-analyzer/workspaces/<workspace-id>/metadata.db
   
   SELECT virtual_path, depth_level FROM files ORDER BY depth_level;
   SELECT virtual_path, depth_level FROM archives ORDER BY depth_level;
   ```
   - [ ] Depth levels are correct (0, 1, 2, etc.)
   - [ ] Parent-child relationships are correct

5. **Test Deep Nesting**
   - [ ] Create archive with 3+ levels of nesting
   - [ ] Import successfully
   - [ ] All levels are accessible

### Expected Results
- ✅ Nested archives extracted correctly
- ✅ Depth levels tracked accurately
- ✅ Parent-child relationships maintained
- ✅ Deep nesting (3+ levels) works
- ✅ No infinite loop or stack overflow

---

## Test 4: Search Functionality (Requirement 2.3)

### Objective
Verify that search uses CAS to read file content and returns correct results.

### Steps

1. **Setup Test Workspace**
   - [ ] Import folder with files containing known text
   - [ ] Example: files with "ERROR", "WARNING", "INFO" keywords

2. **Basic Search**
   - [ ] Enter search term: "ERROR"
   - [ ] Click "Search" button
   - [ ] Results are displayed
   - [ ] Result count is shown

3. **Verify Search Results**
   - [ ] Results show correct file paths
   - [ ] Results show matching lines
   - [ ] Line numbers are displayed
   - [ ] Context around matches is shown

4. **Verify CAS Usage**
   ```bash
   # Check logs for CAS read operations
   # Should see logs like: "Reading content from CAS: hash=abc123..."
   ```
   - [ ] Search reads from CAS (not filesystem)
   - [ ] Content is retrieved by SHA-256 hash

5. **Test Search Features**
   - [ ] Case-sensitive search works
   - [ ] Case-insensitive search works
   - [ ] Regex search works (if supported)
   - [ ] Multi-keyword search works

6. **Test Search Performance**
   - [ ] Search in large workspace (1000+ files)
   - [ ] Results appear within reasonable time (<5 seconds)
   - [ ] UI remains responsive during search

7. **Test FTS5 Full-Text Search**
   ```bash
   sqlite3 ~/.log-analyzer/workspaces/<workspace-id>/metadata.db
   
   SELECT * FROM fts_files WHERE fts_files MATCH 'ERROR';
   ```
   - [ ] FTS5 index is populated
   - [ ] Full-text search returns results

### Expected Results
- ✅ Search finds correct matches
- ✅ Results display properly
- ✅ Search uses CAS for content retrieval
- ✅ Performance is acceptable
- ✅ FTS5 integration works

---

## Test 5: Workspace Management (Requirement 2.4, 2.5)

### Objective
Verify workspace creation, listing, switching, and deletion.

### Steps

1. **Create Multiple Workspaces**
   - [ ] Create workspace: "Workspace 1"
   - [ ] Create workspace: "Workspace 2"
   - [ ] Create workspace: "Workspace 3"
   - [ ] All workspaces appear in list

2. **List Workspaces**
   - [ ] Workspace list shows all workspaces
   - [ ] Workspace names are correct
   - [ ] Workspace IDs are unique
   - [ ] Creation dates are shown

3. **Switch Between Workspaces**
   - [ ] Click on "Workspace 1"
   - [ ] File tree updates to show Workspace 1 files
   - [ ] Click on "Workspace 2"
   - [ ] File tree updates to show Workspace 2 files
   - [ ] No data mixing between workspaces

4. **Verify Workspace Isolation**
   - [ ] Import files in Workspace 1
   - [ ] Switch to Workspace 2
   - [ ] Workspace 2 is empty (no files from Workspace 1)
   - [ ] Switch back to Workspace 1
   - [ ] Files are still there

5. **Delete Workspace**
   - [ ] Select "Workspace 3"
   - [ ] Click "Delete Workspace"
   - [ ] Confirmation dialog appears
   - [ ] Confirm deletion
   - [ ] Workspace is removed from list

6. **Verify Cleanup**
   ```bash
   # Check that workspace directory is deleted
   ls ~/.log-analyzer/workspaces/
   ```
   - [ ] Workspace directory is removed
   - [ ] CAS objects are cleaned up
   - [ ] Metadata database is deleted

7. **Test Workspace Persistence**
   - [ ] Close application
   - [ ] Reopen application
   - [ ] Workspaces are still listed
   - [ ] Files are still accessible

### Expected Results
- ✅ Multiple workspaces can be created
- ✅ Workspaces are isolated from each other
- ✅ Switching between workspaces works
- ✅ Deletion removes all workspace data
- ✅ Workspaces persist across app restarts

---

## Test 6: File Tree Display (Requirement 2.4)

### Objective
Verify that the virtual file tree is constructed correctly from MetadataStore.

### Steps

1. **Import Complex Structure**
   ```
   complex-folder/
   ├── logs/
   │   ├── app.log
   │   └── error.log
   ├── data/
   │   ├── config.json
   │   └── cache/
   │       └── temp.dat
   └── README.md
   ```
   - [ ] Import this structure

2. **Verify Tree Structure**
   - [ ] Root level shows correct folders
   - [ ] Folders can be expanded/collapsed
   - [ ] Files are shown under correct folders
   - [ ] Nested folders display correctly

3. **Verify Tree Icons**
   - [ ] Folders have folder icons
   - [ ] Files have file icons
   - [ ] Archives have archive icons
   - [ ] Different file types have appropriate icons

4. **Test Tree Interactions**
   - [ ] Click on file to view content
   - [ ] Double-click to open file
   - [ ] Right-click for context menu (if supported)
   - [ ] Drag-and-drop (if supported)

5. **Test Large Trees**
   - [ ] Import folder with 1000+ files
   - [ ] Tree renders without lag
   - [ ] Scrolling is smooth
   - [ ] Virtual scrolling works (if implemented)

6. **Verify Virtual Paths**
   ```bash
   sqlite3 ~/.log-analyzer/workspaces/<workspace-id>/metadata.db
   
   SELECT virtual_path FROM files ORDER BY virtual_path;
   ```
   - [ ] Virtual paths match tree display
   - [ ] Paths use correct separators (/)
   - [ ] No duplicate paths

### Expected Results
- ✅ File tree displays correctly
- ✅ Complex structures render properly
- ✅ Tree interactions work
- ✅ Performance is good with large trees
- ✅ Virtual paths are consistent

---

## Test 7: Error Handling

### Objective
Verify that errors are handled gracefully.

### Steps

1. **Test Invalid Archive**
   - [ ] Try to import corrupted ZIP file
   - [ ] Error message is displayed
   - [ ] Application doesn't crash

2. **Test Missing Files**
   - [ ] Try to import non-existent folder
   - [ ] Appropriate error message shown

3. **Test Disk Space**
   - [ ] (Optional) Fill disk to near capacity
   - [ ] Try to import large archive
   - [ ] Disk space error is handled

4. **Test Concurrent Operations**
   - [ ] Start import operation
   - [ ] Start another import immediately
   - [ ] Both complete successfully or queue properly

5. **Test Database Corruption**
   - [ ] (Backup first!) Corrupt metadata.db
   - [ ] Try to open workspace
   - [ ] Error is detected and reported

### Expected Results
- ✅ Errors are caught and reported
- ✅ Application remains stable
- ✅ User-friendly error messages
- ✅ No data corruption

---

## Test 8: Legacy Format Detection

### Objective
Verify that old format workspaces are detected and rejected.

### Steps

1. **Create Mock Legacy Workspace**
   ```bash
   mkdir -p ~/.log-analyzer/workspaces/legacy-test
   touch ~/.log-analyzer/workspaces/legacy-test/index.idx.gz
   ```

2. **Try to Open Legacy Workspace**
   - [ ] Application detects legacy format
   - [ ] Warning message is displayed
   - [ ] User is directed to migration guide

3. **Verify Detection API**
   ```bash
   # If there's a detection command
   # Test it returns correct format
   ```

### Expected Results
- ✅ Legacy format detected
- ✅ Clear warning message
- ✅ Migration guide link provided
- ✅ No attempt to load legacy data

---

## Performance Benchmarks

### Import Performance

| Test Case | File Count | Total Size | Expected Time | Actual Time | Pass/Fail |
|-----------|------------|------------|---------------|-------------|-----------|
| Small folder | 10 | 1 MB | < 2s | | |
| Medium folder | 100 | 10 MB | < 10s | | |
| Large folder | 1000 | 100 MB | < 60s | | |
| Small archive | 10 | 1 MB | < 3s | | |
| Large archive | 1000 | 100 MB | < 90s | | |

### Search Performance

| Test Case | File Count | Query | Expected Time | Actual Time | Pass/Fail |
|-----------|------------|-------|---------------|-------------|-----------|
| Small workspace | 10 | "ERROR" | < 1s | | |
| Medium workspace | 100 | "ERROR" | < 3s | | |
| Large workspace | 1000 | "ERROR" | < 10s | | |

### Memory Usage

| Test Case | Expected Memory | Actual Memory | Pass/Fail |
|-----------|-----------------|---------------|-----------|
| Idle | < 100 MB | | |
| After import (100 files) | < 200 MB | | |
| After import (1000 files) | < 500 MB | | |
| During search | < 300 MB | | |

---

## Regression Checks

### Verify No Old Code Usage

1. **Check for path_map references**
   ```bash
   cd log-analyzer/src-tauri
   rg "path_map|PathMap" --type rust
   ```
   - [ ] No references found (except in docs/comments)

2. **Check for index_store references**
   ```bash
   rg "index_store|save_index|load_index" --type rust
   ```
   - [ ] No references found

3. **Check for migration references**
   ```bash
   rg "migration|migrate_workspace" --type rust
   ```
   - [ ] No references found (except in docs)

---

## Sign-Off Checklist

### Functional Tests
- [ ] Test 1: Import Folder - PASSED
- [ ] Test 2: Import Archive - PASSED
- [ ] Test 3: Nested Archive - PASSED
- [ ] Test 4: Search Functionality - PASSED
- [ ] Test 5: Workspace Management - PASSED
- [ ] Test 6: File Tree Display - PASSED
- [ ] Test 7: Error Handling - PASSED
- [ ] Test 8: Legacy Format Detection - PASSED

### Performance Tests
- [ ] Import performance meets benchmarks
- [ ] Search performance meets benchmarks
- [ ] Memory usage is acceptable

### Regression Tests
- [ ] No old code references found
- [ ] All automated tests pass
- [ ] No console errors

### Documentation
- [ ] User guide is accurate
- [ ] API documentation is updated
- [ ] Migration guide is complete

---

## Issues Found

| Issue # | Description | Severity | Status | Notes |
|---------|-------------|----------|--------|-------|
| | | | | |

---

## Test Environment

- **OS**: _______________
- **Application Version**: _______________
- **Test Date**: _______________
- **Tester**: _______________

---

## Conclusion

**Overall Status**: [ ] PASSED / [ ] FAILED / [ ] PARTIAL

**Summary**:


**Recommendations**:


**Sign-off**: _______________  Date: _______________
