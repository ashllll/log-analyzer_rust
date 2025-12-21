# Enhanced Archive Handling Migration Guide

## Overview

This directory contains migration tools and scripts for upgrading from the legacy archive handling system to the enhanced version.

## Migration Tools

### 1. Data Migration (`migrate_to_enhanced_archive.rs`)

Migrates existing archive extraction data to the new enhanced system.

**Features:**
- Initializes new database schema
- Migrates existing workspace data
- Scans and registers existing extracted archives
- Creates path mappings for existing files
- Validates migration integrity

**Usage:**

```bash
# Dry run (recommended first)
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces \
  --dry-run

# Actual migration
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces
```

**Options:**
- `--old-db`: Path to old database (optional)
- `--new-db`: Path to new database (required)
- `--workspace-root`: Root directory containing workspaces (required)
- `--dry-run`: Perform dry run without making changes

### 2. Configuration Migration (`config_migration.rs`)

Converts old JSON configuration to new TOML format.

**Features:**
- Loads old JSON configuration
- Converts to new TOML structure
- Validates new configuration
- Creates backup of old configuration
- Saves new configuration

**Usage:**

```bash
# With backup
cargo run --bin config_migration -- \
  --old-config config/archive.json \
  --new-config config/extraction_policy.toml \
  --backup

# Without backup
cargo run --bin config_migration -- \
  --old-config config/archive.json \
  --new-config config/extraction_policy.toml
```

**Options:**
- `--old-config`: Path to old JSON configuration
- `--new-config`: Path to new TOML configuration
- `--backup`: Create backup of old configuration

## Migration Process

### Pre-Migration Checklist

- [ ] Backup all data
  - [ ] Database: `data/archive.db`
  - [ ] Configuration: `config/archive.json`
  - [ ] Workspaces: `workspaces/` (optional, can be large)
  - [ ] Logs: `logs/`
- [ ] Verify disk space (need 2x current usage)
- [ ] Document current system state
- [ ] Schedule maintenance window
- [ ] Notify users of downtime
- [ ] Test migration on staging environment

### Step-by-Step Migration

#### Step 1: Backup

```bash
# Create backup directory
mkdir -p backups/$(date +%Y%m%d)

# Backup database
cp data/archive.db backups/$(date +%Y%m%d)/

# Backup configuration
cp config/archive.json backups/$(date +%Y%m%d)/

# Backup logs (last 30 days)
find logs/ -name "*.log" -mtime -30 -exec cp {} backups/$(date +%Y%m%d)/logs/ \;
```

#### Step 2: Stop Application

```bash
# Stop the application
systemctl stop log-analyzer

# Or if running manually
pkill -f log-analyzer
```

#### Step 3: Run Configuration Migration

```bash
# Migrate configuration
cargo run --bin config_migration -- \
  --old-config config/archive.json \
  --new-config config/extraction_policy.toml \
  --backup

# Verify new configuration
cat config/extraction_policy.toml
```

#### Step 4: Run Data Migration (Dry Run)

```bash
# Dry run first
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces \
  --dry-run

# Review output
cat migration.log
```

#### Step 5: Run Actual Data Migration

```bash
# Run actual migration
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces

# Check for errors
echo $?  # Should be 0 for success
```

#### Step 6: Verify Migration

```bash
# Check database integrity
sqlite3 data/enhanced_archive.db "PRAGMA integrity_check;"

# Verify tables exist
sqlite3 data/enhanced_archive.db ".tables"

# Check path mappings count
sqlite3 data/enhanced_archive.db "SELECT COUNT(*) FROM path_mappings;"

# Verify indexes
sqlite3 data/enhanced_archive.db ".indexes path_mappings"
```

#### Step 7: Update Application Configuration

```bash
# Edit main application config to use new database
nano config/app.toml

# Update database path
# database_path = "data/enhanced_archive.db"
```

#### Step 8: Start Application

```bash
# Start application
systemctl start log-analyzer

# Or if running manually
./target/release/log-analyzer &

# Check logs
tail -f logs/app.log
```

#### Step 9: Post-Migration Testing

```bash
# Test extraction with sample archive
./scripts/test_extraction.sh

# Verify path mappings work
./scripts/test_path_mappings.sh

# Check UI functionality
# Open browser and test:
# - Archive import
# - File browsing
# - Search functionality
# - Path resolution
```

### Rollback Procedure

If migration fails or issues are discovered:

```bash
# Stop application
systemctl stop log-analyzer

# Restore old database
cp backups/$(date +%Y%m%d)/archive.db data/

# Restore old configuration
cp backups/$(date +%Y%m%d)/archive.json config/

# Revert application configuration
nano config/app.toml
# Change database_path back to "data/archive.db"

# Start application
systemctl start log-analyzer

# Verify rollback
tail -f logs/app.log
```

## Migration Scenarios

### Scenario 1: Fresh Installation

No migration needed. The enhanced system will be used from the start.

```bash
# Just initialize the new database
cargo run --bin migrate_to_enhanced_archive -- \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces
```

### Scenario 2: Existing Installation with No Custom Configuration

Use default configuration and migrate data only.

```bash
# Copy default configuration
cp config/extraction_policy.toml.example config/extraction_policy.toml

# Migrate data
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces
```

### Scenario 3: Existing Installation with Custom Configuration

Migrate both configuration and data.

```bash
# Migrate configuration
cargo run --bin config_migration -- \
  --old-config config/archive.json \
  --new-config config/extraction_policy.toml \
  --backup

# Migrate data
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces
```

### Scenario 4: Large Installation (>100GB workspaces)

For large installations, consider:

1. **Incremental Migration**: Migrate workspaces in batches
2. **Parallel Processing**: Use multiple migration processes
3. **Extended Maintenance Window**: Allow more time

```bash
# Migrate specific workspaces
for workspace in workspace1 workspace2 workspace3; do
  cargo run --bin migrate_to_enhanced_archive -- \
    --new-db data/enhanced_archive.db \
    --workspace-root ./workspaces/$workspace
done
```

## Troubleshooting

### Issue: Migration Script Fails

**Symptoms**: Migration exits with error code

**Solutions**:
1. Check error message in output
2. Verify disk space: `df -h`
3. Check database permissions: `ls -l data/`
4. Review migration.log for details
5. Try dry run first to identify issues

### Issue: Database Locked

**Symptoms**: "database is locked" error

**Solutions**:
1. Ensure application is stopped
2. Check for zombie processes: `ps aux | grep log-analyzer`
3. Kill any remaining processes: `pkill -9 log-analyzer`
4. Wait a few seconds and retry

### Issue: Path Mappings Not Created

**Symptoms**: No path mappings in new database

**Solutions**:
1. Check if old system used path shortening
2. Verify workspace directory structure
3. Check migration log for skipped files
4. Manually create mappings if needed

### Issue: Configuration Validation Fails

**Symptoms**: New configuration rejected

**Solutions**:
1. Review validation error message
2. Check value ranges (e.g., max_depth 1-20)
3. Verify TOML syntax
4. Compare with example configuration
5. Use default configuration as starting point

### Issue: Performance Degradation After Migration

**Symptoms**: Slower extraction after migration

**Solutions**:
1. Run database optimization: `sqlite3 data/enhanced_archive.db "VACUUM; ANALYZE;"`
2. Check concurrent_extractions setting
3. Verify disk I/O performance
4. Review buffer_size setting
5. Check for resource contention

## Validation Scripts

### Database Integrity Check

```bash
#!/bin/bash
# validate_database.sh

DB_PATH="data/enhanced_archive.db"

echo "Checking database integrity..."
sqlite3 $DB_PATH "PRAGMA integrity_check;"

echo "Checking foreign keys..."
sqlite3 $DB_PATH "PRAGMA foreign_key_check;"

echo "Checking table structure..."
sqlite3 $DB_PATH ".schema path_mappings"

echo "Checking indexes..."
sqlite3 $DB_PATH ".indexes path_mappings"

echo "Checking record counts..."
sqlite3 $DB_PATH "SELECT COUNT(*) as total_mappings FROM path_mappings;"
sqlite3 $DB_PATH "SELECT workspace_id, COUNT(*) as count FROM path_mappings GROUP BY workspace_id;"
```

### Configuration Validation

```bash
#!/bin/bash
# validate_config.sh

CONFIG_PATH="config/extraction_policy.toml"

echo "Validating configuration..."
cargo run --bin validate_config -- $CONFIG_PATH

if [ $? -eq 0 ]; then
    echo "Configuration is valid"
else
    echo "Configuration validation failed"
    exit 1
fi
```

### End-to-End Test

```bash
#!/bin/bash
# test_migration.sh

echo "Testing extraction with sample archive..."

# Create test archive
echo "Creating test archive..."
mkdir -p test_data
echo "test content" > test_data/test.txt
zip test_data/test.zip test_data/test.txt

# Extract using new system
echo "Extracting archive..."
cargo run --bin test_extraction -- \
  --archive test_data/test.zip \
  --output test_output \
  --workspace test_workspace

# Verify extraction
if [ -f test_output/test.txt ]; then
    echo "✓ Extraction successful"
else
    echo "✗ Extraction failed"
    exit 1
fi

# Check path mappings
echo "Checking path mappings..."
sqlite3 data/enhanced_archive.db \
  "SELECT * FROM path_mappings WHERE workspace_id='test_workspace';"

# Cleanup
rm -rf test_data test_output

echo "Migration test completed successfully"
```

## Support

If you encounter issues during migration:

1. Check the troubleshooting section above
2. Review migration logs: `cat migration.log`
3. Check application logs: `tail -f logs/app.log`
4. Consult the operator guide: `docs/ENHANCED_ARCHIVE_OPERATOR_GUIDE.md`
5. Contact support with:
   - Migration command used
   - Error messages
   - Migration log file
   - System information (OS, disk space, etc.)

## Additional Resources

- **User Guide**: `docs/ENHANCED_ARCHIVE_USER_GUIDE.md`
- **Operator Guide**: `docs/ENHANCED_ARCHIVE_OPERATOR_GUIDE.md`
- **Developer Guide**: `docs/ENHANCED_ARCHIVE_DEVELOPER_GUIDE.md`
- **API Documentation**: `docs/API.md`
- **Configuration Reference**: `config/extraction_policy.toml.example`
