# Enhanced Archive Handling - Quick Reference

## Quick Start

### Installation

```bash
cd log-analyzer
npm install
npm run tauri build
```

### Basic Usage

```rust
use log_analyzer::archive::extract_archive_async;

let result = extract_archive_async(
    Path::new("archive.zip"),
    Path::new("./output"),
    "workspace_id"
).await?;
```

## Configuration Quick Reference

### File Location
`config/extraction_policy.toml`

### Key Settings

| Setting | Default | Range | Description |
|---------|---------|-------|-------------|
| `max_depth` | 10 | 1-20 | Maximum nesting depth |
| `max_file_size` | 100MB | >0 | Maximum single file size |
| `max_total_size` | 10GB | >0 | Maximum extraction size |
| `compression_ratio_threshold` | 100.0 | >0 | Zip bomb detection threshold |
| `concurrent_extractions` | auto | 0-32 | Concurrent operations |
| `buffer_size` | 64KB | >0 | Streaming buffer size |

### Quick Configuration Examples

**Development:**
```toml
[extraction]
max_depth = 5
max_file_size = 10485760  # 10MB
[audit]
log_level = "debug"
```

**Production:**
```toml
[extraction]
max_depth = 10
max_file_size = 104857600  # 100MB
[security]
enable_zip_bomb_detection = true
[audit]
log_level = "info"
```

**High-Security:**
```toml
[security]
compression_ratio_threshold = 50.0
enable_zip_bomb_detection = true
[audit]
enable_audit_logging = true
log_retention_days = 180
```

## API Quick Reference

### Extraction Functions

```rust
// Synchronous
extract_archive_sync(archive_path, target_dir, workspace_id)?;

// Asynchronous
extract_archive_async(archive_path, target_dir, workspace_id).await?;

// With options
extract_with_options(archive_path, target_dir, workspace_id, options).await?;
```

### Progress Tracking

```rust
let options = ExtractionOptions {
    enable_progress: true,
    progress_callback: Some(Box::new(|event| {
        println!("Progress: {} files", event.files_processed);
    })),
    ..Default::default()
};
```

### Cancellation

```rust
let token = CancellationToken::new();
let options = ExtractionOptions {
    cancellation_token: Some(token.clone()),
    ..Default::default()
};

// Cancel from another thread
token.cancel();
```

### Path Resolution

```rust
let path_manager = PathManager::new(config);

// Get original path
let original = path_manager.resolve_original_path(
    "workspace_id",
    Path::new("shortened.txt")
).await?;

// Get all mappings
let mappings = path_manager.get_workspace_mappings("workspace_id").await?;
```

## CLI Commands

### Migration

```bash
# Data migration
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces

# Config migration
cargo run --bin config_migration -- \
  --old-config config/archive.json \
  --new-config config/extraction_policy.toml
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench

# Run property tests
cargo test --test property_tests
```

### Maintenance

```bash
# Database optimization
sqlite3 data/enhanced_archive.db "VACUUM; ANALYZE;"

# Check integrity
sqlite3 data/enhanced_archive.db "PRAGMA integrity_check;"

# View path mappings
sqlite3 data/enhanced_archive.db "SELECT * FROM path_mappings LIMIT 10;"
```

## Error Codes

| Code | Error | Cause | Solution |
|------|-------|-------|----------|
| `PathTooLong` | Path exceeds OS limit | Very long filename | Automatic shortening applied |
| `UnsupportedFormat` | Unknown archive format | Unsupported file type | Check file extension |
| `CorruptedArchive` | Cannot read archive | File corruption | Re-download archive |
| `PermissionDenied` | Cannot access file | Insufficient permissions | Check file permissions |
| `ZipBombDetected` | Suspicious compression | Malicious archive | Review source, adjust threshold |
| `DepthLimitExceeded` | Too deeply nested | Exceeds max_depth | Increase max_depth or extract manually |
| `DiskSpaceExhausted` | No disk space | Disk full | Free up space |
| `CancellationRequested` | User cancelled | Manual cancellation | Resume or restart |

## Monitoring

### Metrics Endpoint

```bash
curl http://localhost:8080/api/metrics
```

### Key Metrics

- `extractions.active`: Current active extractions
- `extractions.queued`: Pending extractions
- `performance.avg_extraction_speed_mbps`: Average speed
- `security.zip_bombs_detected_today`: Security events
- `storage.total_workspace_size_gb`: Disk usage

### Log Analysis

```bash
# Count extractions by status
cat logs/app.log | jq -r 'select(.event=="extraction_complete") | .status' | sort | uniq -c

# Average extraction time
cat logs/app.log | jq -r 'select(.event=="extraction_complete") | .duration_seconds' | awk '{sum+=$1; count++} END {print sum/count}'

# Security events
cat logs/app.log | jq -r 'select(.level=="WARN" and .category=="security")'
```

## Troubleshooting

### Common Issues

| Problem | Quick Fix |
|---------|-----------|
| High memory usage | Reduce `concurrent_extractions` and `buffer_size` |
| Slow extraction | Increase `buffer_size`, use SSD storage |
| Database locked | Stop application, check for zombie processes |
| Path too long error | Enable `enable_long_paths = true` |
| Zip bomb warning | Review archive source, adjust `compression_ratio_threshold` |

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/log-analyzer

# Or in config
[audit]
log_level = "debug"
```

### Support Bundle

```bash
./scripts/collect_support_info.sh
```

## Performance Tuning

### CPU-Bound Workloads

```toml
[extraction]
concurrent_extractions = 2  # Reduce concurrency
buffer_size = 131072  # Increase buffer (128KB)
```

### I/O-Bound Workloads

```toml
[extraction]
concurrent_extractions = 8  # Increase concurrency
buffer_size = 65536  # Standard buffer (64KB)
```

### Memory-Constrained Systems

```toml
[extraction]
concurrent_extractions = 2
buffer_size = 32768  # Reduce buffer (32KB)
```

### Network Storage

```toml
[extraction]
concurrent_extractions = 2  # Reduce to avoid network saturation
buffer_size = 262144  # Increase buffer (256KB)
```

## Security

### Enable All Security Features

```toml
[security]
compression_ratio_threshold = 100.0
exponential_backoff_threshold = 1000000.0
enable_zip_bomb_detection = true

[audit]
enable_audit_logging = true
log_format = "json"
log_level = "info"
```

### Security Event Types

- `ZipBombDetected`: High compression ratio
- `PathTraversalAttempt`: `../` in paths
- `ForbiddenExtension`: Executable files
- `ExcessiveFileCount`: Too many files
- `SymlinkCycle`: Circular symlinks

### Audit Log Location

`logs/security_audit.log`

## Limits and Defaults

### Default Limits

| Limit | Default Value |
|-------|---------------|
| Max nesting depth | 10 levels |
| Max file size | 100 MB |
| Max total size | 10 GB |
| Max workspace size | 50 GB |
| Compression ratio threshold | 100:1 |
| Concurrent extractions | CPU cores / 2 |
| Buffer size | 64 KB |
| Temp file TTL | 24 hours |
| Log retention | 90 days |

### OS-Specific Limits

| OS | Max Path Length | Max Filename Length |
|----|-----------------|---------------------|
| Windows | 32,767 (with UNC) | 255 |
| Linux | 4,096 | 255 |
| macOS | 1,024 | 255 |

## Extension Points

### Custom Archive Handler

```rust
#[async_trait]
impl ArchiveHandler for MyHandler {
    fn can_handle(&self, path: &Path) -> bool { /* ... */ }
    async fn extract_with_limits(/* ... */) -> Result<ExtractionSummary> { /* ... */ }
    fn file_extensions(&self) -> Vec<&str> { /* ... */ }
}
```

### Custom Security Validator

```rust
impl SecurityValidator for MyValidator {
    fn validate_file(&self, path: &Path, metadata: &FileMetadata) -> Result<()> { /* ... */ }
    fn validate_archive(&self, path: &Path, entries: &[ArchiveEntry]) -> Result<()> { /* ... */ }
}
```

### Custom Progress Reporter

```rust
impl ProgressReporter for MyReporter {
    fn report_progress(&self, event: ProgressEvent) { /* ... */ }
    fn report_error(&self, error: &ExtractionError) { /* ... */ }
}
```

## Resources

- **User Guide**: `docs/ENHANCED_ARCHIVE_USER_GUIDE.md`
- **Operator Guide**: `docs/ENHANCED_ARCHIVE_OPERATOR_GUIDE.md`
- **Developer Guide**: `docs/ENHANCED_ARCHIVE_DEVELOPER_GUIDE.md`
- **Migration Guide**: `src-tauri/migrations/README.md`
- **API Docs**: https://api-docs.example.com
- **GitHub**: https://github.com/your-org/log-analyzer

## Version

Current Version: **2.0.0**

## License

MIT License - See LICENSE file for details
