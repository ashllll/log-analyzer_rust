# Enhanced Archive Handling - Infrastructure Setup

This document describes the infrastructure setup for the enhanced archive handling system.

## Dependencies Added

### Rust Crates

1. **sqlx v0.7** - Async SQLite database with migration support
   - Features: `runtime-tokio-rustls`, `sqlite`, `migrate`
   - Used for persistent path mapping storage

2. **sha2 v0.10** - SHA-256 hashing implementation
   - Used for content-based path shortening

3. **toml v0.8** - TOML configuration parser (already present)
   - Used for loading extraction policies

4. **proptest v1.4** - Property-based testing framework (already present)
   - Used for comprehensive correctness testing

## Database Schema

### Path Mappings Table

Location: `migrations/20231221000001_create_path_mappings.sql`

```sql
CREATE TABLE path_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    short_path TEXT NOT NULL,
    original_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    access_count INTEGER DEFAULT 0,
    UNIQUE(workspace_id, short_path)
);
```

**Indexes:**
- `idx_workspace_short`: Fast lookups by (workspace_id, short_path)
- `idx_workspace_original`: Reverse lookups by (workspace_id, original_path)
- `idx_workspace_id`: Cleanup operations by workspace_id

## Configuration

### Default Policy File

Location: `config/extraction_policy.toml`

The configuration file defines:
- **Extraction parameters**: max depth, file sizes, concurrency
- **Security settings**: compression ratio thresholds, zip bomb detection
- **Path management**: long path support, shortening thresholds
- **Performance tuning**: buffer sizes, streaming, batching
- **Audit logging**: log format, levels, retention

### Configuration Structure

The policy is loaded via `ExtractionPolicy` struct in `src/models/extraction_policy.rs`:

```rust
pub struct ExtractionPolicy {
    pub extraction: ExtractionConfig,
    pub security: SecurityConfig,
    pub paths: PathsConfig,
    pub performance: PerformanceConfig,
    pub audit: AuditConfig,
}
```

**Validation:**
- All constraints are validated before application
- Invalid configurations are rejected with detailed error messages
- Default secure values are provided

## Database Module

Location: `src/services/metadata_db.rs`

### MetadataDB API

```rust
// Create database connection
let db = MetadataDB::new("path/to/db.sqlite").await?;

// Store path mapping
db.store_mapping(workspace_id, short_path, original_path).await?;

// Retrieve original path
let original = db.get_original_path(workspace_id, short_path).await?;

// Retrieve short path
let short = db.get_short_path(workspace_id, original_path).await?;

// Cleanup workspace
let deleted = db.cleanup_workspace(workspace_id).await?;

// Track access
db.increment_access_count(workspace_id, short_path).await?;
```

### Features

- **Connection pooling**: Up to 10 concurrent connections
- **WAL mode**: Write-Ahead Logging for better concurrency
- **Automatic migrations**: Schema updates applied on startup
- **In-memory support**: Use `:memory:` for testing

## Testing

### Unit Tests

Run extraction policy tests:
```bash
cargo test --lib extraction_policy
```

Run metadata database tests:
```bash
cargo test --lib metadata_db
```

### Test Coverage

**ExtractionPolicy Tests:**
- Default policy validation
- Invalid max_depth detection
- Invalid shortening_threshold detection
- Invalid hash_length detection
- TOML parsing and validation

**MetadataDB Tests:**
- Store and retrieve mappings
- Workspace cleanup
- Access count tracking
- Mapping updates (conflict resolution)

## Migration Usage

The migrations are automatically applied when `MetadataDB::new()` is called. The migration files are located in the `migrations/` directory and are embedded into the binary using sqlx's compile-time verification.

### Manual Migration

If needed, migrations can be run manually:

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features sqlite

# Run migrations
sqlx migrate run --database-url sqlite://path/to/db.sqlite
```

## Next Steps

With the infrastructure in place, the next tasks are:

1. **Task 2**: Implement PathManager with long path support
2. **Task 3**: Implement MetadataDB integration with PathManager
3. **Task 4**: Implement SecurityDetector with zip bomb detection
4. **Task 5**: Implement ExtractionContext and ExtractionStack
5. **Task 6**: Implement ExtractionEngine with iterative traversal

## Verification

To verify the setup is complete:

```bash
# Check dependencies are installed
cargo tree | grep -E "sqlx|sha2|toml|proptest"

# Verify compilation
cargo check --lib

# Run all infrastructure tests
cargo test --lib extraction_policy metadata_db
```

All tests should pass with no errors.
