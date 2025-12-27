# CAS Architecture - Migration Information

## Overview

This application now uses a pure Content-Addressable Storage (CAS) architecture with SQLite metadata storage. 

**Legacy migration tools have been removed** as the system no longer supports backward compatibility with old formats.

## Current Architecture

The system uses:
- **CAS (Content-Addressable Storage)**: SHA-256 based object storage (similar to Git)
- **MetadataStore**: SQLite database with `files` and `archives` tables
- **FTS5**: Full-text search for efficient log searching

## For New Installations

No migration is needed. The CAS system will be initialized automatically when you create your first workspace.

## For Existing Users

**Important**: Old workspace formats (using `path_mappings` table or `.idx.gz` files) are no longer supported.

If you have workspaces from an older version:
1. Create a new workspace
2. Re-import your log files or archives
3. The CAS system will automatically deduplicate content and provide better performance


## Database Schema

The CAS architecture uses the following tables:

### Files Table
```sql
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash TEXT NOT NULL UNIQUE,
    virtual_path TEXT NOT NULL,
    original_name TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_time INTEGER NOT NULL,
    mime_type TEXT,
    parent_archive_id INTEGER,
    depth_level INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);
```

### Archives Table
```sql
CREATE TABLE archives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash TEXT NOT NULL UNIQUE,
    virtual_path TEXT NOT NULL,
    archive_type TEXT NOT NULL,
    parent_archive_id INTEGER,
    depth_level INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);
```

### Full-Text Search
The system uses SQLite FTS5 for efficient full-text search across all files.

## Benefits of CAS Architecture

1. **Automatic Deduplication**: Identical files are stored only once
2. **Data Integrity**: SHA-256 hashing ensures content integrity
3. **Efficient Storage**: Similar to Git's object storage
4. **Fast Search**: SQLite FTS5 provides high-performance full-text search
5. **Reliable**: ACID transactions and foreign key constraints

## Additional Resources

- **User Guide**: `docs/ENHANCED_ARCHIVE_USER_GUIDE.md`
- **Operator Guide**: `docs/ENHANCED_ARCHIVE_OPERATOR_GUIDE.md`
- **Developer Guide**: `docs/ENHANCED_ARCHIVE_DEVELOPER_GUIDE.md`
- **API Documentation**: `docs/API.md`
- **Configuration Reference**: `config/extraction_policy.toml.example`
