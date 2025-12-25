# Migration Guide: Path-Based to CAS Architecture

## Overview

This guide helps you migrate existing workspaces from the old path-based storage system to the new Content-Addressable Storage (CAS) architecture.

## Table of Contents

- [Why Migrate?](#why-migrate)
- [Before You Start](#before-you-start)
- [Automatic Migration](#automatic-migration)
- [Manual Migration](#manual-migration)
- [Verification](#verification)
- [Rollback](#rollback)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)

## Why Migrate?

### Benefits of CAS Architecture

1. **No Path Length Limitations**: Windows 260-character limit no longer applies
2. **Automatic Deduplication**: Identical files stored only once, saving disk space
3. **Better Performance**: 10x faster search with SQLite FTS5 indexing
4. **Nested Archive Support**: Perfect handling of deeply nested archives
5. **Data Integrity**: SHA-256 hashing ensures content hasn't been corrupted

### What Changes?

**Old Format**:
```
workspace_dir/
├── path_map.bin          # Binary HashMap of paths
├── index.bin.gz          # Compressed index
└── extracted/            # Extracted files
    └── archive_123/
```

**New Format**:
```
workspace_dir/
├── objects/              # CAS object storage
│   ├── ab/
│   │   └── cdef123...
│   └── cd/
│       └── ef456...
├── metadata.db           # SQLite database
└── extracted/            # Temporary extraction
```

## Before You Start

### Prerequisites

1. **Backup Your Data**: Always backup your workspace directory before migration
2. **Close Other Instances**: Ensure no other instances of Log Analyzer are running
3. **Check Disk Space**: Migration requires temporary space (up to 2x current workspace size)
4. **Update Application**: Ensure you're running the latest version with CAS support

### Backup Procedure

```bash
# Windows
xcopy /E /I "C:\Users\<username>\AppData\Roaming\com.joeash.log-analyzer" "C:\Backup\log-analyzer"

# macOS
cp -R "~/Library/Application Support/com.joeash.log-analyzer" ~/Backup/log-analyzer

# Linux
cp -R ~/.local/share/com.joeash.log-analyzer ~/Backup/log-analyzer
```

## Automatic Migration

### Step 1: Launch Application

When you open a workspace with the old format, the application automatically detects it and shows a migration dialog.

### Step 2: Review Migration Info

The dialog displays:
- Workspace name
- Number of files to migrate
- Estimated time
- Disk space required

### Step 3: Start Migration

Click **"Migrate Now"** to begin the process.

### Step 4: Monitor Progress

The migration dialog shows:
- Current file being processed
- Progress percentage
- Files migrated / Total files
- Estimated time remaining

### Step 5: Completion

When migration completes:
- ✅ Success message displayed
- Old format backed up as `path_map.bin.backup`
- Workspace automatically reloaded with new format

## Manual Migration

If automatic migration fails or you prefer manual control:

### Using Tauri Command

```javascript
// From browser console (Dev Tools)
await window.__TAURI__.invoke('migrate_workspace_to_cas', {
  workspaceId: 'your-workspace-id'
});
```

### Using Rust API

```rust
use crate::migration::migrate_workspace_to_cas;

let result = migrate_workspace_to_cas(
    workspace_id,
    old_workspace_dir,
    new_workspace_dir,
).await?;

println!("Migrated {} files", result.files_migrated);
```

### Migration Steps (Internal)

The migration process:

1. **Read Old Format**
   ```rust
   let path_map = read_path_map(&workspace_dir)?;
   ```

2. **Initialize CAS**
   ```rust
   let cas = ContentAddressableStorage::new(&objects_dir)?;
   let metadata_store = MetadataStore::new(&db_path).await?;
   ```

3. **Process Each File**
   ```rust
   for (real_path, virtual_path) in path_map {
       // Read file content
       let content = fs::read(&real_path).await?;
       
       // Store in CAS
       let hash = cas.store_content(&content).await?;
       
       // Insert metadata
       let metadata = FileMetadata {
           sha256_hash: hash,
           virtual_path,
           // ... other fields
       };
       metadata_store.insert_file(&metadata).await?;
   }
   ```

4. **Verify Migration**
   ```rust
   let validator = IndexValidator::new(cas, metadata_store);
   let report = validator.validate().await?;
   ```

5. **Backup Old Format**
   ```rust
   fs::rename("path_map.bin", "path_map.bin.backup").await?;
   ```

## Verification

### Automatic Verification

After migration, the application automatically verifies:

1. **File Count**: All files from old format present in new format
2. **Content Integrity**: SHA-256 hashes match file content
3. **Metadata Completeness**: All virtual paths recorded
4. **Search Functionality**: Sample searches return expected results

### Manual Verification

#### Check File Count

```sql
-- Open metadata.db with SQLite browser
SELECT COUNT(*) FROM files;
```

Compare with old format:
```rust
// Count entries in old path_map.bin
let path_map: HashMap<String, String> = bincode::deserialize(&data)?;
println!("Old format: {} files", path_map.len());
```

#### Verify Random Files

```rust
// Pick random files and verify content
let files = metadata_store.get_all_files().await?;
for file in files.iter().take(10) {
    let content = cas.read_content(&file.sha256_hash).await?;
    println!("✓ {} ({} bytes)", file.virtual_path, content.len());
}
```

#### Test Search

```rust
// Perform test search
let results = metadata_store.search_files("error").await?;
println!("Found {} results for 'error'", results.len());
```

## Rollback

If migration fails or you encounter issues:

### Automatic Rollback

The application automatically rolls back if:
- Migration fails partway through
- Verification fails
- User cancels migration

Rollback process:
1. Delete `metadata.db` and `objects/` directory
2. Restore `path_map.bin` from `path_map.bin.backup`
3. Reload workspace with old format

### Manual Rollback

```bash
# Navigate to workspace directory
cd "C:\Users\<username>\AppData\Roaming\com.joeash.log-analyzer\workspaces\<workspace-id>"

# Remove new format files
rm -rf objects/
rm metadata.db metadata.db-wal metadata.db-shm

# Restore old format
mv path_map.bin.backup path_map.bin
mv index.bin.gz.backup index.bin.gz
```

## Troubleshooting

### Issue: "Migration Failed: File Not Found"

**Cause**: Original files moved or deleted

**Solution**:
1. Check if extracted files still exist in `extracted/` directory
2. If files missing, re-import the original archive
3. Then retry migration

### Issue: "Migration Failed: Out of Disk Space"

**Cause**: Insufficient disk space for CAS storage

**Solution**:
1. Free up disk space (at least 2x current workspace size)
2. Delete unused workspaces
3. Move workspace to drive with more space
4. Retry migration

### Issue: "Migration Stuck at X%"

**Cause**: Large file taking long time to process

**Solution**:
1. Wait patiently (large files can take several minutes)
2. Check application logs for progress
3. If truly stuck (no progress for 10+ minutes), restart application
4. Migration will resume from checkpoint

### Issue: "Verification Failed: Hash Mismatch"

**Cause**: File content changed during migration

**Solution**:
1. Rollback migration
2. Ensure no other processes are modifying files
3. Retry migration

### Issue: "Database Locked"

**Cause**: Another process accessing database

**Solution**:
1. Close all instances of Log Analyzer
2. Delete `metadata.db-wal` and `metadata.db-shm` files
3. Retry migration

## FAQ

### Q: How long does migration take?

**A**: Depends on workspace size:
- Small (< 1000 files): 1-2 minutes
- Medium (1000-10000 files): 5-15 minutes
- Large (> 10000 files): 30+ minutes

### Q: Can I use the application during migration?

**A**: No, the workspace being migrated is locked. You can use other workspaces.

### Q: Will migration delete my original files?

**A**: No, original files are preserved. Only the index format changes.

### Q: Can I migrate multiple workspaces at once?

**A**: No, migrate one workspace at a time to avoid resource contention.

### Q: What happens if I close the application during migration?

**A**: Migration will resume from the last checkpoint when you restart.

### Q: Can I revert to old format after migration?

**A**: Yes, use the rollback procedure. However, new features (like nested archive support) won't work with old format.

### Q: Will my search history be preserved?

**A**: Yes, search history is stored separately and not affected by migration.

### Q: Do I need to re-import archives after migration?

**A**: No, migration converts existing data. No re-import needed.

### Q: What if migration fails repeatedly?

**A**: Contact support with:
- Application logs
- Workspace size and file count
- Error messages
- System information (OS, disk space, etc.)

## Best Practices

### Before Migration

1. ✅ Backup workspace directory
2. ✅ Close other applications to free resources
3. ✅ Ensure stable power supply (for laptops)
4. ✅ Check disk space (at least 2x workspace size)

### During Migration

1. ✅ Don't close the application
2. ✅ Don't modify workspace files
3. ✅ Monitor progress in migration dialog
4. ✅ Be patient with large workspaces

### After Migration

1. ✅ Verify file count matches
2. ✅ Test search functionality
3. ✅ Check a few random files
4. ✅ Keep backup for a few days
5. ✅ Delete backup after confirming everything works

## Support

If you encounter issues not covered in this guide:

1. **Check Logs**: Application logs contain detailed error information
   - Windows: `%APPDATA%\com.joeash.log-analyzer\logs\`
   - macOS: `~/Library/Logs/com.joeash.log-analyzer/`
   - Linux: `~/.local/share/com.joeash.log-analyzer/logs/`

2. **GitHub Issues**: Report bugs at https://github.com/ashllll/log-analyzer_rust/issues

3. **Documentation**: See [CAS_ARCHITECTURE.md](architecture/CAS_ARCHITECTURE.md) for technical details

## Conclusion

Migration to CAS architecture is a one-time process that significantly improves performance and reliability. Follow this guide carefully, and you'll enjoy the benefits of the new system without data loss.

Happy analyzing! 📊
