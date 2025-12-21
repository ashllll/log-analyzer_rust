# Enhanced Archive Handling - User Guide

## Overview

The Enhanced Archive Handling system provides robust support for extracting complex archive structures with advanced features including:

- **Long Path Support**: Handle filenames exceeding 255 characters
- **Deep Nesting**: Safely extract archives nested up to 10 levels deep
- **Security Protection**: Automatic detection and prevention of zip bombs
- **Progress Tracking**: Real-time progress updates during extraction
- **Error Recovery**: Resume interrupted extractions

## Key Features

### 1. Long Filename Support

The system automatically handles long filenames that exceed operating system limits:

**Windows**: Supports paths up to 32,767 characters using UNC prefix (`\\?\`)
**Unix/Linux**: Supports 255 bytes per path component

#### How It Works

When a filename exceeds the OS limit, the system:
1. Automatically shortens the path using content-based hashing
2. Maintains a mapping between original and shortened names
3. Displays original names in the UI transparently

#### Example

```
Original: very_long_filename_with_many_characters_that_exceeds_limit_2024_01_15_log_data.txt
Shortened: a3f5b2c8d1e4f6a9.txt
```

You'll always see the original name in the interface, but the file is stored with the shortened name on disk.

### 2. Deep Nesting Support

Extract archives containing other archives up to 10 levels deep (configurable).

#### Nesting Levels Explained

```
Level 0: archive.zip
Level 1: ├── logs.tar.gz (inside archive.zip)
Level 2:     ├── daily.zip (inside logs.tar.gz)
Level 3:         └── data.rar (inside daily.zip)
```

#### What Happens at the Limit

When the nesting depth limit is reached:
- Extraction continues for sibling archives at the same level
- A warning is logged indicating the depth limit was reached
- The archive path is recorded for manual inspection if needed

### 3. Security Features

#### Zip Bomb Detection

The system automatically detects and prevents zip bomb attacks:

**Compression Ratio Check**: Files with compression ratios exceeding 100:1 are flagged
**Cumulative Size Limit**: Extraction stops if total extracted size exceeds 10GB (default)
**Exponential Backoff**: Nested archives with high compression ratios are scored and rejected

#### Example Warning

```
⚠️ Warning: Suspicious compression detected
File: malicious.zip
Compression Ratio: 1000:1
Action: Extraction halted, user confirmation required
```

#### Safe Defaults

- Maximum file size: 100MB
- Maximum total extraction size: 10GB per archive
- Maximum workspace size: 50GB
- Compression ratio threshold: 100:1

### 4. Progress Tracking

Monitor extraction progress in real-time:

```
Extracting: archive.zip
├── Files processed: 150/500
├── Bytes extracted: 2.5 GB / 5.0 GB
├── Current depth: 3
├── Estimated time remaining: 2 minutes
└── Status: Processing nested archive...
```

#### Progress Information

- **Current file**: The file currently being extracted
- **Files processed**: Number of files extracted so far
- **Bytes extracted**: Total data extracted
- **Current depth**: Current nesting level
- **Estimated time**: Remaining time based on current speed

### 5. Error Recovery

If extraction is interrupted (power loss, crash), the system can resume:

1. **Automatic Detection**: On restart, the system detects incomplete extractions
2. **Resume Option**: Choose to resume from the last checkpoint
3. **Cleanup Option**: Or clean up and start fresh

#### Checkpoint System

Checkpoints are created:
- Every 100 files extracted
- Every 1GB of data extracted
- Before processing each nested archive

## Common Use Cases

### Extracting Large Log Archives

```
Scenario: You have a 5GB archive containing nested daily log archives

Steps:
1. Import the archive into your workspace
2. The system automatically:
   - Detects nested structure
   - Applies security checks
   - Extracts with progress tracking
3. View extracted logs in the file browser
4. Search across all extracted files
```

### Handling Archives with Long Filenames

```
Scenario: Archive contains log files with timestamps and long descriptive names

What happens:
1. System detects filenames exceeding 255 characters
2. Automatically applies path shortening
3. Creates mapping in database
4. You see original names in the UI
5. Search works with original names
```

### Dealing with Suspicious Archives

```
Scenario: You receive an archive from an untrusted source

Protection:
1. System calculates compression ratio
2. Detects if ratio exceeds 100:1
3. Halts extraction and shows warning
4. Requires explicit confirmation to continue
5. Logs security event for audit
```

## Configuration

### Adjusting Extraction Limits

You can customize extraction behavior through the settings panel:

#### Maximum Nesting Depth
- **Default**: 10 levels
- **Range**: 1-20 levels
- **Recommendation**: Keep at 10 unless you have specific needs

#### File Size Limits
- **Max file size**: 100MB (default)
- **Max total size**: 10GB (default)
- **Max workspace size**: 50GB (default)

#### Security Settings
- **Compression ratio threshold**: 100:1 (default)
- **Enable zip bomb detection**: Yes (recommended)

### Configuration File

Advanced users can edit the configuration file directly:

**Location**: `config/extraction_policy.toml`

```toml
[extraction]
max_depth = 10
max_file_size = 104857600  # 100MB
max_total_size = 10737418240  # 10GB

[security]
compression_ratio_threshold = 100.0
enable_zip_bomb_detection = true

[paths]
enable_long_paths = true
shortening_threshold = 0.8
```

## Troubleshooting

### Problem: "Path too long" error

**Solution**: 
- On Windows, ensure long path support is enabled in the system
- The application should handle this automatically
- Check that `enable_long_paths = true` in configuration

### Problem: Extraction stops at depth limit

**Solution**:
- This is expected behavior for deeply nested archives
- Increase `max_depth` in configuration if needed
- Check warnings to see which archives were skipped

### Problem: "Zip bomb detected" warning

**Solution**:
- This is a security feature protecting your system
- Review the archive source before proceeding
- If the archive is trusted, you can manually extract specific files
- Consider increasing `compression_ratio_threshold` if you regularly work with highly compressed data

### Problem: Extraction is slow

**Possible causes**:
- Large archive size
- Many small files
- Deep nesting structure
- Disk I/O limitations

**Solutions**:
- Extraction speed depends on archive structure
- Expected speed: 50+ MB/s for uncompressed archives
- Check disk space and I/O performance
- Consider extracting to a faster drive (SSD)

### Problem: Cannot resume interrupted extraction

**Solution**:
- Ensure checkpoint files are not deleted
- Check logs for specific error messages
- If resume fails, use the cleanup option and re-extract

## Best Practices

### 1. Workspace Organization

- Create separate workspaces for different projects
- Regularly clean up old extracted archives
- Monitor workspace size limits

### 2. Security

- Always enable zip bomb detection
- Review warnings before proceeding with suspicious archives
- Keep audit logs enabled for compliance

### 3. Performance

- Extract large archives during off-peak hours
- Ensure sufficient disk space (2x archive size recommended)
- Use SSD storage for better performance

### 4. Maintenance

- Regularly review and clean up old path mappings
- Monitor disk space usage
- Check audit logs for security events

## FAQ

**Q: What happens to shortened filenames when I search?**
A: Search works with original filenames. The shortening is transparent to you.

**Q: Can I extract archives larger than 10GB?**
A: Yes, adjust `max_total_size` in the configuration. Be aware of disk space limits.

**Q: How do I know if an archive is safe?**
A: The system checks compression ratios and file counts. Always review warnings for untrusted sources.

**Q: Can I extract password-protected archives?**
A: Currently, password-protected archives are not supported. Extract them manually first.

**Q: What archive formats are supported?**
A: ZIP, RAR, TAR, GZ, and combinations (e.g., .tar.gz)

**Q: How much disk space do I need?**
A: At least 2x the compressed archive size, plus overhead for path mappings and checkpoints.

## Getting Help

If you encounter issues:

1. Check the logs: `logs/app.log`
2. Review audit logs for security events
3. Consult the troubleshooting section above
4. Contact support with:
   - Archive size and format
   - Error messages from logs
   - Configuration settings
   - Steps to reproduce the issue

## Version History

### Version 2.0 (Enhanced Archive Handling)
- Added long path support
- Implemented deep nesting extraction
- Added zip bomb detection
- Improved progress tracking
- Added checkpoint/resume functionality

### Version 1.0 (Legacy)
- Basic archive extraction
- Limited to 5 nesting levels
- 255 character filename limit
