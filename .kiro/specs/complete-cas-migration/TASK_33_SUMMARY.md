# Task 33: Manual Functional Testing - Summary

## Overview

Task 33 involves comprehensive manual functional testing of the CAS migration to ensure all features work correctly with the new architecture.

**Status**: ✅ **READY FOR TESTING**

**Requirements Validated**: 2.1, 2.2, 2.3, 2.4, 2.5

## What Was Created

### 1. Comprehensive Testing Guide
**File**: `TASK_33_MANUAL_TESTING_GUIDE.md`

A detailed 8-test suite covering:
- ✅ Test 1: Import Folder (Requirement 2.1)
- ✅ Test 2: Import Archive (Requirement 2.2)
- ✅ Test 3: Nested Archive (Requirement 2.2, 2.3)
- ✅ Test 4: Search Functionality (Requirement 2.3)
- ✅ Test 5: Workspace Management (Requirement 2.4, 2.5)
- ✅ Test 6: File Tree Display (Requirement 2.4)
- ✅ Test 7: Error Handling
- ✅ Test 8: Legacy Format Detection

Each test includes:
- Clear objectives
- Step-by-step instructions
- Verification commands
- Expected results
- Pass/fail criteria

### 2. Quick Reference Checklist
**File**: `TASK_33_QUICK_CHECKLIST.md`

A condensed version for rapid testing with:
- Essential tests only
- Quick verification commands
- Database and CAS verification scripts
- Performance quick checks
- Regression checks

### 3. Test Data Generator (Bash)
**File**: `generate_test_data.sh`

Automated script that creates:
- Simple folder structures
- Complex nested folders
- Duplicate content (for deduplication testing)
- Various archive formats (.zip, .tar.gz, .tar, .gz)
- Nested archives (2-3 levels deep)
- Large datasets (100+ files)
- Search-optimized test data

### 4. Test Data Generator (PowerShell)
**File**: `generate_test_data.ps1`

Windows-compatible version of the test data generator with identical functionality.

## How to Use

### Step 1: Generate Test Data

**On Linux/Mac**:
```bash
cd .kiro/specs/complete-cas-migration
chmod +x generate_test_data.sh
./generate_test_data.sh
```

**On Windows**:
```powershell
cd .kiro\specs\complete-cas-migration
.\generate_test_data.ps1
```

This creates a `test-data/` directory with all necessary test files.

### Step 2: Build and Run Application

```bash
cd log-analyzer
npm run tauri dev
```

### Step 3: Follow Testing Guide

Open `TASK_33_MANUAL_TESTING_GUIDE.md` and follow each test sequentially.

For quick validation, use `TASK_33_QUICK_CHECKLIST.md`.

### Step 4: Document Results

Fill in the checklists and document any issues found in the "Issues Found" section of the testing guide.

## Key Testing Areas

### 1. CAS Storage Verification
- Files stored with SHA-256 hashes
- Git-style object storage (2-char prefix directories)
- Deduplication working correctly
- No temporary files left behind

### 2. Metadata Store Verification
- SQLite database created correctly
- All files have metadata entries
- Virtual paths are correct
- Depth levels tracked for nested archives
- FTS5 full-text search index populated

### 3. Functional Testing
- Import operations work
- Search returns correct results
- Workspace isolation maintained
- File tree displays correctly
- Deletion cleans up properly

### 4. Performance Testing
- Import times are acceptable
- Search is fast (<10s for 1000 files)
- Memory usage is reasonable
- No memory leaks

### 5. Regression Testing
- No old code references (path_map, index_store, migration)
- All automated tests pass
- No console errors

## Verification Commands

### Database Inspection
```bash
# Set your workspace ID
WS_ID="<workspace-id>"
DB_PATH="$HOME/.log-analyzer/workspaces/$WS_ID/metadata.db"

# Count files
sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM files;"

# List files with hashes
sqlite3 "$DB_PATH" "SELECT virtual_path, sha256_hash FROM files LIMIT 10;"

# Check depth levels
sqlite3 "$DB_PATH" "SELECT virtual_path, depth_level FROM files ORDER BY depth_level;"

# Verify FTS5
sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM fts_files;"
```

### CAS Storage Inspection
```bash
# Set your workspace ID
WS_ID="<workspace-id>"
OBJ_DIR="$HOME/.log-analyzer/workspaces/$WS_ID/objects"

# Count objects
find "$OBJ_DIR" -type f | wc -l

# List object directories (should be 2-char hex)
ls "$OBJ_DIR"

# Verify object content
HASH="<some-hash>"
cat "$OBJ_DIR/${HASH:0:2}/${HASH:2}"
```

### Code Regression Check
```bash
cd log-analyzer/src-tauri

# Should return nothing (or only comments/docs)
rg "path_map|PathMap" --type rust
rg "index_store|save_index|load_index" --type rust
rg "migration" --type rust | grep -v "migration_guide"
```

## Success Criteria

The testing is considered successful when:

- ✅ All 8 functional tests pass
- ✅ Database contains correct data structure
- ✅ CAS storage follows Git-style organization
- ✅ Deduplication works correctly
- ✅ Search returns accurate results
- ✅ Workspace isolation is maintained
- ✅ Performance meets benchmarks
- ✅ No old code references found
- ✅ No errors in console or logs
- ✅ Legacy format detection works

## Performance Benchmarks

### Import Performance
| Test Case | File Count | Total Size | Expected Time |
|-----------|------------|------------|---------------|
| Small folder | 10 | 1 MB | < 2s |
| Medium folder | 100 | 10 MB | < 10s |
| Large folder | 1000 | 100 MB | < 60s |
| Small archive | 10 | 1 MB | < 3s |
| Large archive | 1000 | 100 MB | < 90s |

### Search Performance
| Test Case | File Count | Expected Time |
|-----------|------------|---------------|
| Small workspace | 10 | < 1s |
| Medium workspace | 100 | < 3s |
| Large workspace | 1000 | < 10s |

### Memory Usage
| Test Case | Expected Memory |
|-----------|-----------------|
| Idle | < 100 MB |
| After import (100 files) | < 200 MB |
| After import (1000 files) | < 500 MB |
| During search | < 300 MB |

## Next Steps

1. **Execute Tests**: Follow the testing guide and complete all tests
2. **Document Issues**: Record any problems found in the testing guide
3. **Verify Fixes**: Re-test after any fixes are applied
4. **Sign Off**: Complete the sign-off checklist when all tests pass
5. **Proceed to Task 34**: Move to code review once testing is complete

## Notes

- This is a **manual testing task** - it requires human interaction with the application
- Automated tests (unit, integration, property-based) have already been completed in previous tasks
- Manual testing validates the end-to-end user experience
- Take your time and be thorough - this is the final validation before release

## Files Created

1. `TASK_33_MANUAL_TESTING_GUIDE.md` - Comprehensive testing guide (8 tests)
2. `TASK_33_QUICK_CHECKLIST.md` - Quick reference for essential tests
3. `generate_test_data.sh` - Bash script to generate test data
4. `generate_test_data.ps1` - PowerShell script to generate test data
5. `TASK_33_SUMMARY.md` - This summary document

## Testing Timeline

Estimated time to complete all tests: **2-4 hours**

- Test 1-3 (Import): 30-45 minutes
- Test 4 (Search): 20-30 minutes
- Test 5-6 (Workspace/Tree): 30-45 minutes
- Test 7-8 (Error/Legacy): 20-30 minutes
- Performance testing: 20-30 minutes
- Documentation: 20-30 minutes

## Contact

If you encounter any issues during testing:
1. Document them in the "Issues Found" section
2. Check the troubleshooting section in the guide
3. Review previous task summaries for context
4. Consult the design document for expected behavior

---

**Ready to Test**: All materials are prepared. You can now begin manual functional testing.

**Status**: ✅ Task 33 preparation complete - Ready for execution
