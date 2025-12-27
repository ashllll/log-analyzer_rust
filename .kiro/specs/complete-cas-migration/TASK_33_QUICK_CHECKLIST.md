# Task 33: Quick Testing Checklist

## Quick Start

1. Build and run the application:
   ```bash
   cd log-analyzer
   npm run tauri dev
   ```

2. Prepare test data in `test-data/` folder

3. Follow the checklist below

---

## Essential Tests (Quick Validation)

### ✅ Test 1: Import Folder
- [ ] Create workspace
- [ ] Import a folder with 3-5 files
- [ ] Verify files appear in tree
- [ ] Check `~/.log-analyzer/workspaces/<id>/objects/` exists
- [ ] Check `~/.log-analyzer/workspaces/<id>/metadata.db` exists

### ✅ Test 2: Import Archive
- [ ] Create workspace
- [ ] Import a .zip file
- [ ] Verify extracted files appear
- [ ] Check CAS storage contains files

### ✅ Test 3: Nested Archive
- [ ] Create workspace
- [ ] Import archive containing another archive
- [ ] Verify both levels are visible
- [ ] Check depth levels in database

### ✅ Test 4: Search
- [ ] Import files with known content
- [ ] Search for a keyword
- [ ] Verify results are correct
- [ ] Check results show file paths and line numbers

### ✅ Test 5: Workspace Management
- [ ] Create 2-3 workspaces
- [ ] Switch between them
- [ ] Verify data isolation
- [ ] Delete one workspace
- [ ] Verify cleanup

### ✅ Test 6: File Tree
- [ ] Import complex folder structure
- [ ] Verify tree displays correctly
- [ ] Expand/collapse folders
- [ ] Click on files

---

## Database Verification Commands

```bash
# Set workspace ID
WS_ID="<your-workspace-id>"
DB_PATH="$HOME/.log-analyzer/workspaces/$WS_ID/metadata.db"

# Count files
sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM files;"

# List files
sqlite3 "$DB_PATH" "SELECT virtual_path, sha256_hash FROM files LIMIT 10;"

# Count archives
sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM archives;"

# Check depth levels
sqlite3 "$DB_PATH" "SELECT virtual_path, depth_level FROM files ORDER BY depth_level;"

# Verify FTS5
sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM fts_files;"
```

---

## CAS Verification Commands

```bash
# Set workspace ID
WS_ID="<your-workspace-id>"
OBJ_DIR="$HOME/.log-analyzer/workspaces/$WS_ID/objects"

# Count objects
find "$OBJ_DIR" -type f | wc -l

# List objects
find "$OBJ_DIR" -type f | head -10

# Verify Git-style structure (2-char prefix)
ls "$OBJ_DIR"

# Check object content (example)
HASH="abc123..."
cat "$OBJ_DIR/${HASH:0:2}/${HASH:2}"
```

---

## Performance Quick Check

```bash
# Import timing
time <import-operation>

# Search timing
time <search-operation>

# Memory usage (Linux/Mac)
ps aux | grep log-analyzer

# Memory usage (Windows)
tasklist | findstr log-analyzer
```

---

## Regression Quick Check

```bash
cd log-analyzer/src-tauri

# Should return nothing (or only comments)
rg "path_map|PathMap" --type rust
rg "index_store|save_index|load_index" --type rust
rg "migration" --type rust | grep -v "migration_guide"
```

---

## Pass Criteria

- ✅ All 6 essential tests pass
- ✅ Database contains correct data
- ✅ CAS storage structure is correct
- ✅ No old code references found
- ✅ Performance is acceptable
- ✅ No errors in console

---

## If Issues Found

1. Document in TASK_33_MANUAL_TESTING_GUIDE.md
2. Create issue tickets
3. Prioritize fixes
4. Re-test after fixes

---

## Final Sign-Off

**Tester**: _______________
**Date**: _______________
**Status**: [ ] PASS [ ] FAIL
**Notes**: _______________
