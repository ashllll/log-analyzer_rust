# Task 33: Manual Functional Testing - Instructions

## 🎯 Quick Start

You are now ready to perform manual functional testing of the CAS migration!

## 📋 What You Need to Do

### 1. Generate Test Data (5 minutes)

Choose your platform and run the appropriate script:

**Linux/Mac**:
```bash
cd .kiro/specs/complete-cas-migration
chmod +x generate_test_data.sh
./generate_test_data.sh
```

**Windows**:
```powershell
cd .kiro\specs\complete-cas-migration
.\generate_test_data.ps1
```

This creates a `test-data/` folder with all necessary test files.

### 2. Build and Launch Application (5 minutes)

```bash
cd log-analyzer
npm run tauri dev
```

Wait for the application to start.

### 3. Perform Testing (2-4 hours)

Open one of these guides:

**For comprehensive testing**:
- Open: `TASK_33_MANUAL_TESTING_GUIDE.md`
- Follow all 8 tests sequentially
- Document results in the checklist

**For quick validation**:
- Open: `TASK_33_QUICK_CHECKLIST.md`
- Complete the essential tests
- Use the quick verification commands

### 4. Document Results

As you test, fill in the checklists:
- ✅ Mark tests that pass
- ❌ Mark tests that fail
- 📝 Document any issues found

## 📚 Available Resources

| File | Purpose |
|------|---------|
| `TASK_33_MANUAL_TESTING_GUIDE.md` | Complete testing guide with 8 detailed tests |
| `TASK_33_QUICK_CHECKLIST.md` | Quick reference for essential tests |
| `TASK_33_SUMMARY.md` | Overview and context |
| `generate_test_data.sh` | Test data generator (Linux/Mac) |
| `generate_test_data.ps1` | Test data generator (Windows) |
| `TASK_33_INSTRUCTIONS.md` | This file |

## 🧪 Test Coverage

You will be testing:

1. **Import Folder** - Verify CAS storage of folder contents
2. **Import Archive** - Verify extraction and CAS storage
3. **Nested Archive** - Verify multi-level archive handling
4. **Search** - Verify search uses CAS and returns correct results
5. **Workspace Management** - Verify creation, switching, deletion
6. **File Tree** - Verify virtual file tree display
7. **Error Handling** - Verify graceful error handling
8. **Legacy Detection** - Verify old format rejection

## ✅ Success Criteria

Testing is complete when:
- All functional tests pass
- Performance meets benchmarks
- No old code references found
- No console errors
- Documentation is accurate

## 🔍 Key Verification Points

### CAS Storage
```bash
# Check workspace structure
ls -la ~/.log-analyzer/workspaces/<workspace-id>/

# Should see:
# - metadata.db (SQLite database)
# - objects/ (CAS storage with 2-char prefix dirs)
```

### Metadata Database
```bash
# Query database
sqlite3 ~/.log-analyzer/workspaces/<workspace-id>/metadata.db

SELECT COUNT(*) FROM files;
SELECT virtual_path, sha256_hash FROM files LIMIT 5;
```

### No Old Code
```bash
cd log-analyzer/src-tauri
rg "path_map|PathMap|index_store" --type rust
# Should return nothing or only comments
```

## 📊 Performance Expectations

| Operation | Expected Time |
|-----------|---------------|
| Import 10 files | < 2 seconds |
| Import 100 files | < 10 seconds |
| Search 100 files | < 3 seconds |
| Search 1000 files | < 10 seconds |

## 🐛 If You Find Issues

1. **Document the issue** in the testing guide
2. **Include details**:
   - What you were doing
   - What you expected
   - What actually happened
   - Error messages (if any)
   - Screenshots (if helpful)
3. **Continue testing** other areas
4. **Report at the end** with all findings

## 💡 Tips

- **Take your time** - Thoroughness is more important than speed
- **Follow the order** - Tests build on each other
- **Use verification commands** - Don't just trust the UI
- **Check the database** - Verify data is stored correctly
- **Monitor console** - Watch for errors or warnings
- **Test edge cases** - Try unusual inputs
- **Document everything** - Good notes help with debugging

## 🎬 Example Test Flow

Here's what a typical test looks like:

1. **Start**: Open application
2. **Action**: Create workspace "Test 1"
3. **Action**: Import `test-data/simple-folder/`
4. **Verify**: Files appear in tree
5. **Verify**: Check database has entries
6. **Verify**: Check CAS has objects
7. **Result**: ✅ Pass or ❌ Fail
8. **Document**: Note any issues

## 📞 Need Help?

If you're stuck:
1. Check the troubleshooting section in the testing guide
2. Review the design document for expected behavior
3. Look at previous task summaries for context
4. Check the requirements document for specifications

## 🚀 After Testing

Once all tests are complete:
1. Fill in the sign-off section
2. Calculate pass/fail rate
3. Summarize findings
4. Proceed to Task 34 (Code Review)

---

## Ready? Let's Go! 🎉

1. Generate test data
2. Launch application
3. Open testing guide
4. Start testing!

**Good luck with the testing!** 🧪✨
