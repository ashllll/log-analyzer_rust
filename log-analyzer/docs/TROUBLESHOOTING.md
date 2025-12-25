# Troubleshooting Guide

## Table of Contents

- [Common Issues](#common-issues)
  - [Import Issues](#import-issues)
  - [Search Issues](#search-issues)
  - [Performance Issues](#performance-issues)
  - [Database Issues](#database-issues)
  - [Archive Extraction Issues](#archive-extraction-issues)
- [Error Messages](#error-messages)
- [Diagnostic Tools](#diagnostic-tools)
- [Log Files](#log-files)
- [Getting Help](#getting-help)

## Common Issues

### Import Issues

#### Issue: "Failed to import archive"

**Symptoms**:
- Import fails with error message
- Progress bar stops
- Task shows as failed

**Possible Causes**:
1. Corrupted archive file
2. Unsupported archive format
3. Insufficient disk space
4. Permission issues

**Solutions**:

1. **Verify archive integrity**:
   ```bash
   # Test ZIP file
   unzip -t archive.zip
   
   # Test TAR file
   tar -tzf archive.tar.gz
   ```

2. **Check supported formats**:
   - Supported: `.zip`, `.tar`, `.tar.gz`, `.tgz`, `.gz`, `.rar`
   - If unsupported, extract manually first

3. **Check disk space**:
   - Ensure at least 2x archive size available
   - Clean up old workspaces if needed

4. **Check permissions**:
   - Ensure read access to archive file
   - Ensure write access to workspace directory

#### Issue: "Import stuck at X%"

**Symptoms**:
- Progress bar not moving
- Application appears frozen
- No error message

**Possible Causes**:
1. Large file being processed
2. Deeply nested archive
3. Application hang

**Solutions**:

1. **Wait patiently**:
   - Large files (>100MB) can take several minutes
   - Check CPU usage - if high, import is still running

2. **Check logs**:
   - Look for recent log entries
   - If no new logs for 10+ minutes, application may be hung

3. **Restart application**:
   - Close and reopen application
   - Import will resume from checkpoint

#### Issue: "Path too long" error

**Symptoms**:
- Import fails with path length error
- Nested archives fail to extract
- Windows-specific issue

**Possible Causes**:
- Windows 260-character path limit
- Deeply nested archives
- Long file names

**Solutions**:

1. **Use CAS architecture** (automatic in new version):
   - CAS eliminates path length limitations
   - Migrate old workspaces to CAS

2. **Enable long paths on Windows** (if using old format):
   ```powershell
   # Run as Administrator
   New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
     -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
   ```

3. **Extract to shorter path**:
   - Move archive to root directory (e.g., `C:\temp\`)
   - Import from there

### Search Issues

#### Issue: "No search results found"

**Symptoms**:
- Search returns 0 results
- Expected files not appearing
- Search completes instantly

**Possible Causes**:
1. Incorrect search query
2. Files not indexed
3. Case sensitivity
4. Database corruption

**Solutions**:

1. **Verify query syntax**:
   - Use `|` for OR logic: `error|warning`
   - Check for typos
   - Try simpler query first

2. **Check file count**:
   ```sql
   -- Open metadata.db with SQLite browser
   SELECT COUNT(*) FROM files;
   ```
   - If 0, workspace not properly imported

3. **Rebuild index**:
   - Delete workspace and re-import
   - Or use index validator (see Diagnostic Tools)

4. **Check case sensitivity**:
   - Search is case-insensitive by default
   - Verify query matches actual content

#### Issue: "Search is very slow"

**Symptoms**:
- Search takes 10+ seconds
- Application becomes unresponsive
- High CPU usage

**Possible Causes**:
1. Large workspace (100,000+ files)
2. Complex regex query
3. FTS5 index not optimized
4. Insufficient RAM

**Solutions**:

1. **Optimize database**:
   ```sql
   -- Open metadata.db with SQLite browser
   VACUUM;
   ANALYZE;
   INSERT INTO files_fts(files_fts) VALUES('optimize');
   ```

2. **Simplify query**:
   - Avoid complex regex patterns
   - Use specific keywords instead of wildcards
   - Add filters (date range, file pattern)

3. **Increase memory**:
   - Close other applications
   - Restart application to clear cache

4. **Split large workspaces**:
   - Create separate workspaces for different time periods
   - Or different log sources

#### Issue: "Search results incomplete"

**Symptoms**:
- Some expected files missing from results
- Result count lower than expected
- Inconsistent results

**Possible Causes**:
1. Result limit reached (default: 50,000)
2. Files filtered out
3. Database inconsistency

**Solutions**:

1. **Check result limit**:
   - Default limit: 50,000 results
   - Add more specific filters to reduce results
   - Or increase limit in settings

2. **Verify filters**:
   - Check date range filter
   - Check log level filter
   - Check file pattern filter

3. **Validate database**:
   ```rust
   // Use index validator
   let validator = IndexValidator::new(cas, metadata_store);
   let report = validator.validate().await?;
   ```

### Performance Issues

#### Issue: "Application slow to start"

**Symptoms**:
- Long startup time (30+ seconds)
- Splash screen shows for extended period
- Application unresponsive after launch

**Possible Causes**:
1. Large number of workspaces
2. Database corruption
3. Disk I/O bottleneck

**Solutions**:

1. **Clean up workspaces**:
   - Delete unused workspaces
   - Archive old workspaces

2. **Check disk health**:
   - Run disk check utility
   - Ensure SSD TRIM enabled
   - Check for disk errors

3. **Optimize databases**:
   ```bash
   # For each workspace
   sqlite3 metadata.db "VACUUM; ANALYZE;"
   ```

#### Issue: "High memory usage"

**Symptoms**:
- Application uses >2GB RAM
- System becomes slow
- Out of memory errors

**Possible Causes**:
1. Large search results cached
2. Memory leak
3. Too many workspaces open

**Solutions**:

1. **Clear cache**:
   - Restart application
   - Cache automatically cleared

2. **Reduce result limit**:
   - Lower max results in settings
   - Use more specific queries

3. **Close unused workspaces**:
   - Only keep active workspaces open
   - Delete or archive old workspaces

#### Issue: "High CPU usage"

**Symptoms**:
- CPU usage 100%
- Fan running at full speed
- Application slow to respond

**Possible Causes**:
1. Large import in progress
2. Complex search running
3. Background indexing

**Solutions**:

1. **Wait for operations to complete**:
   - Check background tasks panel
   - Import/search will complete eventually

2. **Cancel long-running operations**:
   - Click cancel button in search
   - Or close application

3. **Reduce parallelism**:
   - Close other applications
   - Reduce number of concurrent operations

### Database Issues

#### Issue: "Database is locked"

**Symptoms**:
- Error: "database is locked"
- Operations fail
- Application hangs

**Possible Causes**:
1. Multiple instances accessing same workspace
2. Incomplete transaction
3. Crashed process holding lock

**Solutions**:

1. **Close other instances**:
   - Ensure only one instance of application running
   - Check task manager for orphaned processes

2. **Delete WAL files**:
   ```bash
   # Navigate to workspace directory
   rm metadata.db-wal metadata.db-shm
   ```

3. **Restart application**:
   - Close and reopen application
   - Database lock will be released

#### Issue: "Database corrupted"

**Symptoms**:
- Error: "database disk image is malformed"
- Workspace fails to load
- Search returns errors

**Possible Causes**:
1. Unexpected shutdown
2. Disk errors
3. Incomplete write

**Solutions**:

1. **Attempt recovery**:
   ```bash
   # Backup first
   cp metadata.db metadata.db.backup
   
   # Try to recover
   sqlite3 metadata.db ".recover" | sqlite3 metadata_recovered.db
   mv metadata_recovered.db metadata.db
   ```

2. **Rebuild from CAS**:
   - If recovery fails, rebuild metadata from CAS objects
   - Use diagnostic tool (see below)

3. **Re-import**:
   - Last resort: delete workspace and re-import
   - Original archives should still be available

### Archive Extraction Issues

#### Issue: "Failed to extract nested archive"

**Symptoms**:
- Nested archive not processed
- Some files missing from workspace
- Warning in logs

**Possible Causes**:
1. Maximum nesting depth reached (default: 10)
2. Corrupted nested archive
3. Unsupported format in nested archive

**Solutions**:

1. **Check nesting depth**:
   ```sql
   SELECT MAX(depth_level) FROM files;
   ```
   - If >= 10, increase limit or extract manually

2. **Verify nested archive**:
   - Extract outer archive manually
   - Test nested archive separately

3. **Extract manually**:
   - Extract nested archives manually
   - Import extracted files directly

#### Issue: "RAR extraction failed"

**Symptoms**:
- RAR files not extracting
- Error: "unrar not found"
- RAR files skipped

**Possible Causes**:
1. unrar binary missing
2. Corrupted RAR file
3. Password-protected RAR

**Solutions**:

1. **Verify unrar binary**:
   - Check `src-tauri/binaries/` directory
   - Ensure correct binary for your platform

2. **Test RAR file**:
   ```bash
   unrar t archive.rar
   ```

3. **Extract manually**:
   - Use WinRAR or 7-Zip to extract
   - Import extracted files

## Error Messages

### "AppError::ArchiveError"

**Meaning**: Archive processing failed

**Common Causes**:
- Corrupted archive
- Unsupported format
- Extraction failure

**Solution**: Check archive integrity, verify format support

### "AppError::ValidationError"

**Meaning**: Input validation failed

**Common Causes**:
- Invalid path
- Path traversal attempt
- Invalid file name

**Solution**: Check file paths, ensure no special characters

### "AppError::IoError"

**Meaning**: File system operation failed

**Common Causes**:
- Permission denied
- File not found
- Disk full

**Solution**: Check permissions, disk space, file existence

### "AppError::DatabaseError"

**Meaning**: Database operation failed

**Common Causes**:
- Database locked
- Database corrupted
- SQL syntax error

**Solution**: See Database Issues section

## Diagnostic Tools

### Index Validator

Validates CAS and metadata consistency:

```rust
use crate::services::index_validator::IndexValidator;

let validator = IndexValidator::new(cas, metadata_store);
let report = validator.validate().await?;

println!("Total files: {}", report.total_files);
println!("Valid files: {}", report.valid_files);
println!("Invalid files: {}", report.invalid_files.len());

for invalid in &report.invalid_files {
    println!("  {} - {}", invalid.path, invalid.reason);
}
```

### Workspace Metrics

Collect workspace statistics:

```rust
use crate::services::workspace_metrics::WorkspaceMetrics;

let metrics = WorkspaceMetrics::collect(&workspace_dir).await?;

println!("Total files: {}", metrics.total_files);
println!("Total size: {} bytes", metrics.total_size);
println!("Deduplication ratio: {:.2}%", metrics.dedup_ratio * 100.0);
println!("Max nesting depth: {}", metrics.max_depth);
```

### Database Integrity Check

Check SQLite database integrity:

```sql
-- Open metadata.db with SQLite browser
PRAGMA integrity_check;
PRAGMA foreign_key_check;
```

### CAS Integrity Check

Verify all CAS objects:

```rust
use crate::storage::ContentAddressableStorage;

let cas = ContentAddressableStorage::new(&objects_dir)?;

// Get all hashes from database
let hashes = metadata_store.get_all_hashes().await?;

// Verify each hash exists in CAS
for hash in hashes {
    if !cas.exists(&hash) {
        println!("Missing: {}", hash);
    }
}
```

## Log Files

### Location

**Windows**:
```
%APPDATA%\com.joeash.log-analyzer\logs\
```

**macOS**:
```
~/Library/Logs/com.joeash.log-analyzer/
```

**Linux**:
```
~/.local/share/com.joeash.log-analyzer/logs/
```

### Log Levels

- **ERROR**: Critical errors requiring attention
- **WARN**: Warnings that don't stop operation
- **INFO**: Important events (import start/complete)
- **DEBUG**: Detailed information for debugging
- **TRACE**: Very detailed information (rarely needed)

### Enabling Debug Logs

Set environment variable before starting application:

```bash
# Windows (PowerShell)
$env:RUST_LOG="debug"
.\log-analyzer.exe

# macOS/Linux
RUST_LOG=debug ./log-analyzer
```

### Reading Logs

```bash
# View recent logs
tail -f app.log

# Search for errors
grep ERROR app.log

# Search for specific operation
grep "import" app.log
```

## Getting Help

### Before Asking for Help

1. ✅ Check this troubleshooting guide
2. ✅ Search existing GitHub issues
3. ✅ Check application logs
4. ✅ Try basic solutions (restart, clear cache)

### When Reporting Issues

Include:

1. **System Information**:
   - OS and version
   - Application version
   - Available disk space
   - RAM

2. **Problem Description**:
   - What you were trying to do
   - What happened instead
   - Error messages (exact text)

3. **Steps to Reproduce**:
   - Detailed steps to reproduce issue
   - Sample files (if possible)

4. **Logs**:
   - Relevant log excerpts
   - Full logs if possible (attach as file)

5. **Screenshots**:
   - Error dialogs
   - Application state

### Support Channels

1. **GitHub Issues**: https://github.com/ashllll/log-analyzer_rust/issues
2. **Documentation**: See `docs/` directory
3. **Email**: [Your support email]

## Conclusion

Most issues can be resolved by:
- Restarting the application
- Checking disk space and permissions
- Validating database integrity
- Re-importing problematic workspaces

If issues persist, don't hesitate to ask for help with detailed information about your problem.

Happy troubleshooting! 🔧
