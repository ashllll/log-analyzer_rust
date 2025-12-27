# ⚠️ Legacy Format No Longer Supported

## Quick Notice

**Log Analyzer 2.0 no longer supports the legacy path-based storage format.**

If you see this message, your workspace was created with an older version and needs to be recreated.

## What You Need to Do

### Option 1: Re-import from Original Archives (Recommended)

1. **Create a new workspace**
   - Click "New Workspace"
   - Give it a descriptive name

2. **Import your archives**
   - Click "Import Archive"
   - Select your original .zip, .tar.gz, or other archive files
   - Wait for import to complete

3. **Done!**
   - Your data is now in the new CAS format
   - Enjoy 10x faster search and better reliability

### Option 2: Export and Re-import

If you don't have the original archives:

1. **Keep the old version** (if you still have it)
2. **Export your data** from the legacy workspace
3. **Create an archive** from the exported files
4. **Import into new workspace** in Log Analyzer 2.0

### Option 3: Start Fresh

- Create new workspaces for new data
- Keep old version for accessing legacy data (read-only)

## Why This Change?

The new **Content-Addressable Storage (CAS)** format provides:

✅ **10x faster search** with SQLite FTS5  
✅ **Automatic deduplication** saves disk space  
✅ **No path length limits** (Windows 260-char limit gone!)  
✅ **Perfect nested archive support** (unlimited depth)  
✅ **Data integrity** with SHA-256 verification  
✅ **Industry-standard** technology (Git-style storage)  

## Need Help?

- **Full Guide**: See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) for detailed instructions
- **User Guide**: See [ENHANCED_ARCHIVE_USER_GUIDE.md](ENHANCED_ARCHIVE_USER_GUIDE.md)
- **Troubleshooting**: See [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- **Report Issues**: https://github.com/ashllll/log-analyzer_rust/issues

## FAQ

**Q: Will I lose my data?**  
A: No! Your original archive files are safe. Just re-import them.

**Q: How long does re-importing take?**  
A: About the same as the original import (5-30 minutes for typical archives).

**Q: Can I still access my old workspaces?**  
A: Yes, by keeping an older version of Log Analyzer installed.

**Q: Is this a one-time change?**  
A: Yes. CAS format is stable and based on industry standards (Git, SQLite, SHA-256).

---

**Thank you for your understanding!** The new format provides significant improvements that benefit all users. 🚀
